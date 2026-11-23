use std::collections::VecDeque;

use sundaydeck_core::Slide;

use crate::config::{VoiceConfig, VoiceSyncMode};

const MAX_TRANSCRIPT_WINDOW: usize = 24;
const FILLER_WORDS: &[&str] = &[
    "oh",
    "ooh",
    "yeah",
    "yes",
    "sing",
    "come",
    "on",
    "everybody",
    "church",
    "amen",
    "uh",
    "um",
];

#[derive(Debug, Clone, PartialEq)]
pub enum VoiceDecision {
    Hold,
    Suggest {
        target_index: usize,
        confidence: f32,
        reason: String,
    },
    Advance {
        target_index: usize,
        confidence: f32,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlignmentUpdate {
    pub slide_progress: f32,
    pub confidence: f32,
    pub decision: VoiceDecision,
}

#[derive(Debug, Clone)]
struct IndexedWord {
    norm: String,
    phonetic: String,
}

#[derive(Debug, Clone)]
struct SlideVoiceIndex {
    label: String,
    words: Vec<IndexedWord>,
}

/// Real-time offline phonetic & anchor-based slide aligner (`<1 ms` per transcript chunk).
///
/// Tracks spoken/sung word progression across the active live deck, highlights upcoming cues in
/// `Suggest` mode, auto-advances on end-of-slide anchors or next-slide openings in `Auto` mode,
/// and detects spontaneous worship-leader repeats (e.g. jumping back to Chorus or Bridge).
pub struct DeckVoiceAligner {
    slides: Vec<SlideVoiceIndex>,
    active_index: usize,
    progress_word_idx: usize,
    recent_words: VecDeque<IndexedWord>,
    last_transition_ms: i64,
    locked_by_override: bool,
}

impl Default for DeckVoiceAligner {
    fn default() -> Self {
        Self::new()
    }
}

impl DeckVoiceAligner {
    pub fn new() -> Self {
        Self {
            slides: Vec::new(),
            active_index: 0,
            progress_word_idx: 0,
            recent_words: VecDeque::with_capacity(MAX_TRANSCRIPT_WINDOW),
            last_transition_ms: 0,
            locked_by_override: false,
        }
    }

    pub fn update_deck(&mut self, slides: &[Slide], active_index: usize, now_ms: i64) {
        self.slides = slides
            .iter()
            .map(|slide| SlideVoiceIndex {
                label: slide.label.clone(),
                words: tokenize_indexed(slide.content.primary_text()),
            })
            .collect();
        self.active_index = active_index.min(self.slides.len().saturating_sub(1));
        self.progress_word_idx = 0;
        self.recent_words.clear();
        self.last_transition_ms = now_ms;
    }

    pub fn note_slide_changed(&mut self, new_active_index: usize, now_ms: i64) {
        self.active_index = new_active_index.min(self.slides.len().saturating_sub(1));
        self.progress_word_idx = 0;
        self.recent_words.clear();
        self.last_transition_ms = now_ms;
    }

    pub fn set_override_locked(&mut self, locked: bool) {
        self.locked_by_override = locked;
    }

    pub fn active_index(&self) -> usize {
        self.active_index
    }

    pub fn active_slide_progress(&self) -> f32 {
        let Some(cur) = self.slides.get(self.active_index) else {
            return 0.0;
        };
        if cur.words.is_empty() {
            return 0.0;
        }
        (self.progress_word_idx as f32 / cur.words.len() as f32).clamp(0.0, 1.0)
    }

    /// Processes a new partial or final transcript chunk and returns live progress + decision.
    pub fn ingest_transcript(
        &mut self,
        transcript: &str,
        now_ms: i64,
        config: &VoiceConfig,
    ) -> AlignmentUpdate {
        if config.mode == VoiceSyncMode::Off || self.slides.is_empty() {
            return AlignmentUpdate {
                slide_progress: self.active_slide_progress(),
                confidence: 0.0,
                decision: VoiceDecision::Hold,
            };
        }

        let incoming = tokenize_indexed(transcript);
        if incoming.is_empty() {
            return AlignmentUpdate {
                slide_progress: self.active_slide_progress(),
                confidence: 0.0,
                decision: VoiceDecision::Hold,
            };
        }

        for word in &incoming {
            if self.recent_words.len() >= MAX_TRANSCRIPT_WINDOW {
                self.recent_words.pop_front();
            }
            self.recent_words.push_back(word.clone());
        }

        let window: Vec<IndexedWord> = self.recent_words.iter().cloned().collect();
        let (cur_score, furthest_idx) =
            self.slides.get(self.active_index).map_or((0.0, 0), |cur| {
                score_current_slide_progress(&cur.words, &window, self.progress_word_idx)
            });

        if cur_score >= 0.42 && furthest_idx > self.progress_word_idx {
            self.progress_word_idx = furthest_idx;
        }

        let progress = self.active_slide_progress();
        let in_cooldown =
            now_ms.saturating_sub(self.last_transition_ms) < config.cooldown_ms as i64;
        if (config.lock_during_override && self.locked_by_override) || in_cooldown {
            return AlignmentUpdate {
                slide_progress: progress,
                confidence: cur_score,
                decision: VoiceDecision::Hold,
            };
        }

        // 1. Check sequential advance: end-of-slide anchor OR opening words of slide `active + 1`.
        let next_idx = self.active_index + 1;
        if let Some(next_slide) = self.slides.get(next_idx) {
            let tail_window = tail_slice(&window, 8);
            let cur_tail_score = self.slides.get(self.active_index).map_or(0.0, |cur| {
                let tail = tail_slice(&cur.words, 5);
                phrase_similarity(tail, tail_window)
            });
            let next_head_score = {
                let head = head_slice(&next_slide.words, 6);
                phrase_similarity(head, tail_window)
            };

            let sequential_conf = if next_head_score >= 0.58 {
                next_head_score
            } else if progress >= 0.74 && cur_tail_score >= 0.58 {
                (cur_tail_score * 0.72 + progress * 0.28).min(1.0)
            } else {
                0.0
            };

            if sequential_conf >= config.suggest_confidence_threshold {
                let reason = if next_head_score >= 0.58 {
                    format!("Matched opening of {}", next_slide.label)
                } else {
                    format!("End-of-slide cue -> {}", next_slide.label)
                };
                return self.finalize_decision(
                    next_idx,
                    sequential_conf,
                    progress,
                    reason,
                    now_ms,
                    config,
                );
            }
        }

        // 2. Check spontaneous out-of-sequence repeat/jump (e.g. worship leader repeats Chorus).
        let recent_short = tail_slice(&window, 9);
        let mut best_jump: Option<(usize, f32)> = None;
        for (idx, slide) in self.slides.iter().enumerate() {
            if idx == self.active_index || slide.words.len() < 3 {
                continue;
            }
            let head_score = phrase_similarity(head_slice(&slide.words, 7), recent_short);
            let full_score = phrase_similarity(&slide.words, recent_short);
            let raw = head_score.max(full_score);
            let adjusted = (raw - config.jump_penalty).max(0.0);
            if raw >= 0.76
                && adjusted > cur_score + 0.14
                && best_jump.is_none_or(|(_, best)| adjusted > best)
            {
                best_jump = Some((idx, raw));
            }
        }

        if let Some((target_idx, jump_conf)) = best_jump {
            let reason = format!("Spontaneous jump -> {}", self.slides[target_idx].label);
            return self.finalize_decision(target_idx, jump_conf, progress, reason, now_ms, config);
        }

        AlignmentUpdate {
            slide_progress: progress,
            confidence: cur_score,
            decision: VoiceDecision::Hold,
        }
    }

    fn finalize_decision(
        &mut self,
        target_index: usize,
        confidence: f32,
        progress: f32,
        reason: String,
        now_ms: i64,
        config: &VoiceConfig,
    ) -> AlignmentUpdate {
        if config.mode == VoiceSyncMode::Auto && confidence >= config.advance_confidence_threshold {
            self.active_index = target_index;
            self.progress_word_idx = 0;
            self.recent_words.clear();
            self.last_transition_ms = now_ms;
            return AlignmentUpdate {
                slide_progress: 0.0,
                confidence,
                decision: VoiceDecision::Advance {
                    target_index,
                    confidence,
                    reason,
                },
            };
        }

        AlignmentUpdate {
            slide_progress: progress,
            confidence,
            decision: VoiceDecision::Suggest {
                target_index,
                confidence,
                reason,
            },
        }
    }
}

fn score_current_slide_progress(
    slide_words: &[IndexedWord],
    window: &[IndexedWord],
    current_cursor: usize,
) -> (f32, usize) {
    if slide_words.is_empty() || window.is_empty() {
        return (0.0, current_cursor);
    }
    let tail = tail_slice(window, 10);
    let mut best_pos = current_cursor;
    let mut matched = 0usize;
    let mut search_from = current_cursor.saturating_sub(2);

    for spoken in tail {
        let search_end = (search_from + 12).min(slide_words.len());
        if let Some(rel) = slide_words[search_from..search_end]
            .iter()
            .position(|w| word_similarity(w, spoken) >= 0.78)
        {
            let abs = search_from + rel + 1;
            matched += 1;
            search_from = abs;
            if abs > best_pos {
                best_pos = abs;
            }
        }
    }

    let score = (matched as f32 / tail.len().clamp(1, 6) as f32).clamp(0.0, 1.0);
    (score, best_pos)
}

fn phrase_similarity(anchor: &[IndexedWord], spoken: &[IndexedWord]) -> f32 {
    if anchor.is_empty() || spoken.is_empty() {
        return 0.0;
    }
    let mut matched_weight = 0.0f32;
    let mut spoken_cursor = 0usize;
    for target in anchor {
        let mut best = 0.0f32;
        let mut best_pos = None;
        for (offset, cand) in spoken[spoken_cursor..].iter().enumerate() {
            let sim = word_similarity(target, cand);
            if sim > best {
                best = sim;
                best_pos = Some(spoken_cursor + offset + 1);
            }
        }
        if best >= 0.76 {
            matched_weight += best;
            if let Some(next_pos) = best_pos {
                spoken_cursor = next_pos;
            }
        }
    }
    (matched_weight / anchor.len() as f32).clamp(0.0, 1.0)
}

fn word_similarity(a: &IndexedWord, b: &IndexedWord) -> f32 {
    if a.norm == b.norm {
        return 1.0;
    }
    if !a.phonetic.is_empty() && a.phonetic == b.phonetic {
        return 0.94;
    }
    let max_len = a.norm.len().max(b.norm.len());
    if max_len <= 2 {
        return 0.0;
    }
    let dist = levenshtein(&a.norm, &b.norm);
    1.0 - (dist as f32 / max_len as f32)
}

fn tokenize_indexed(input: &str) -> Vec<IndexedWord> {
    input
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter_map(|raw| {
            let norm: String = raw
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .map(|c| c.to_ascii_lowercase())
                .collect();
            if norm.len() < 2 || FILLER_WORDS.contains(&norm.as_str()) {
                return None;
            }
            let phonetic = phonetic_key(&norm);
            Some(IndexedWord { norm, phonetic })
        })
        .collect()
}

/// Compact English phonetic key for sung/reverberant church ASR drift (`wretch` == `retch`).
fn phonetic_key(word: &str) -> String {
    let mut s = word.to_owned();
    for (from, to) in [
        ("wr", "r"),
        ("kn", "n"),
        ("gn", "n"),
        ("ph", "f"),
        ("ck", "k"),
        ("qu", "k"),
        ("gh", ""),
        ("wh", "w"),
        ("sh", "s"),
        ("th", "t"),
        ("ch", "k"),
    ] {
        s = s.replace(from, to);
    }
    let mut out = String::with_capacity(s.len());
    let mut prev = '\0';
    for (idx, ch) in s.chars().enumerate() {
        let mapped = match ch {
            'c' => 'k',
            'z' => 's',
            'v' => 'f',
            'a' | 'e' | 'i' | 'o' | 'u' | 'y' if idx > 0 => continue,
            other => other,
        };
        if mapped != prev {
            out.push(mapped);
            prev = mapped;
        }
    }
    out
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b_chars: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b_chars.len()).collect();
    let mut curr = vec![0usize; b_chars.len() + 1];

    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, &cb) in b_chars.iter().enumerate() {
            let cost = usize::from(ca != cb);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b_chars.len()]
}

fn head_slice<T>(slice: &[T], max_len: usize) -> &[T] {
    &slice[..slice.len().min(max_len)]
}

fn tail_slice<T>(slice: &[T], max_len: usize) -> &[T] {
    let start = slice.len().saturating_sub(max_len);
    &slice[start..]
}
