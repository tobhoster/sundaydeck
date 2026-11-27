//! Tiered cache of decoded background images and video keyframes for render paths.
//!
//! Render code only reads it. A miss records a pending entry and queues the decode on a single
//! background lane (one decode in flight, deduplicated by uri and tier, first in first out with
//! still images ahead of video keyframes), so the UI thread never decodes and only one full-size
//! transient buffer exists at a time. Each finished decode refreshes every window.
//! Ready entries are evicted least-recently-used once their decoded bytes exceed
//! [`MEDIA_CACHE_BUDGET_BYTES`], and their GPU atlas tiles are freed with `App::drop_image`.
//!
//! The state is per thread because a GPUI `App`, its foreground executor and every render run on
//! one thread; parallel headless test apps therefore never share (or starve) a decode lane.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use gpui::{App, AsyncApp, RenderImage};

use crate::media_decode::{ImageTier, decode_background, decode_video_keyframes};

/// Upper bound on decoded bytes kept resident across every tier and window.
pub const MEDIA_CACHE_BUDGET_BYTES: usize = 384 * 1024 * 1024;

type Frames = Arc<[Arc<RenderImage>]>;

thread_local! {
    static MEDIA_CACHE: RefCell<MediaCache> = RefCell::new(MediaCache::default());
}

/// Render-time state of one cached decode, so callers can tell "still decoding" from "failed".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaLookup<T> {
    /// Queued or decoding; a repaint follows once it lands.
    Pending,
    /// Decoded and ready to paint.
    Ready(T),
    /// The decode failed (missing or unreadable media).
    Failed,
}

impl<T> MediaLookup<T> {
    /// The ready value, treating pending and failed alike.
    pub fn ready(self) -> Option<T> {
        match self {
            Self::Ready(value) => Some(value),
            Self::Pending | Self::Failed => None,
        }
    }
}

/// The decoded image for `uri` at `tier`, or `None` while it is still decoding or when it
/// cannot be decoded. A miss schedules the decode and repaints all windows once it lands.
pub fn background_image(uri: &str, tier: ImageTier, cx: &App) -> Option<Arc<RenderImage>> {
    background_image_lookup(uri, tier, cx).ready()
}

/// Like [`background_image`], but distinguishes a pending decode from a failed one.
pub fn background_image_lookup(
    uri: &str,
    tier: ImageTier,
    cx: &App,
) -> MediaLookup<Arc<RenderImage>> {
    let slot = MediaSlot {
        kind: MediaKind::Image,
        tier,
    };
    match cached_frames(uri, slot, cx) {
        MediaLookup::Ready(frames) => match frames.first() {
            Some(image) => MediaLookup::Ready(Arc::clone(image)),
            None => MediaLookup::Failed,
        },
        MediaLookup::Pending => MediaLookup::Pending,
        MediaLookup::Failed => MediaLookup::Failed,
    }
}

/// The keyframes of the looping video `uri` at `tier`, or `None` while they are still being
/// resolved. Ready keyframes may be empty when the video could not be read.
pub fn video_keyframes(uri: &str, tier: ImageTier, cx: &App) -> Option<Frames> {
    let slot = MediaSlot {
        kind: MediaKind::VideoKeyframes,
        tier,
    };
    cached_frames(uri, slot, cx).ready()
}

fn cached_frames(uri: &str, slot: MediaSlot, cx: &App) -> MediaLookup<Frames> {
    let looked_up = with_cache(|cache| match cache.lookup(uri, slot) {
        Lookup::Ready(frames) => (MediaLookup::Ready(frames), false),
        Lookup::Pending => (MediaLookup::Pending, false),
        Lookup::Failed => (MediaLookup::Failed, false),
        Lookup::Miss => {
            cache.enqueue(MediaKey {
                slot,
                uri: uri.to_owned(),
            });
            (MediaLookup::Pending, cache.claim_lane())
        }
    });
    let Some((frames, lane_claimed)) = looked_up else {
        return MediaLookup::Pending;
    };
    if lane_claimed {
        cx.spawn(async move |cx| run_decode_lane(cx).await).detach();
    }
    frames
}

/// Decodes queued keys one at a time until the queue is empty.
async fn run_decode_lane(cx: &mut AsyncApp) {
    let _release = LaneReleaseOnDrop;
    while let Some(key) = with_cache(MediaCache::next_job).flatten() {
        let (key, decoded) = cx
            .background_executor()
            .spawn(async move {
                let decoded = key.decode();
                (key, decoded)
            })
            .await;
        let evicted = with_cache(|cache| {
            cache.store(key.clone(), decoded);
            cache.evict_over_budget(&key)
        })
        .unwrap_or_default();
        cx.update(|cx| {
            for image in evicted {
                cx.drop_image(image, None);
            }
            cx.refresh_windows();
        });
    }
}

/// Runs `f` on this thread's cache; `None` only if the cache is unavailable (re-entered or
/// torn down at thread exit), which callers treat like a pending entry instead of panicking.
fn with_cache<R>(f: impl FnOnce(&mut MediaCache) -> R) -> Option<R> {
    MEDIA_CACHE
        .try_with(|cell| cell.try_borrow_mut().ok().map(|mut cache| f(&mut cache)))
        .ok()
        .flatten()
}

/// Frees the lane when its task is dropped mid-decode (e.g. the app shut down), so pending
/// entries are requested again instead of waiting forever.
struct LaneReleaseOnDrop;

impl Drop for LaneReleaseOnDrop {
    fn drop(&mut self) {
        with_cache(MediaCache::abandon_lane);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MediaKind {
    Image,
    VideoKeyframes,
}

/// What a cached uri was decoded into: a still image or video keyframes, at one tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MediaSlot {
    kind: MediaKind,
    tier: ImageTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MediaKey {
    slot: MediaSlot,
    uri: String,
}

impl MediaKey {
    /// Runs the blocking decode for this key; `None` marks an image that cannot be decoded.
    fn decode(&self) -> Option<Frames> {
        let tier = self.slot.tier;
        match self.slot.kind {
            MediaKind::Image => decode_background(&self.uri, tier).map(|image| Arc::from([image])),
            MediaKind::VideoKeyframes => Some(Arc::from(decode_video_keyframes(&self.uri, tier))),
        }
    }
}

enum CacheEntry {
    Pending,
    Ready {
        frames: Frames,
        bytes: usize,
        last_used: u64,
    },
    Failed,
}

/// What a render-time lookup found for one uri in one slot.
enum Lookup {
    /// Never requested (or forgotten): the caller queues its decode.
    Miss,
    /// Requested but still decoding.
    Pending,
    /// The decode failed.
    Failed,
    /// Decoded frames, now marked most recently used.
    Ready(Frames),
}

/// Entries are grouped per slot so render-time lookups borrow the uri instead of allocating.
struct MediaCache {
    entries: HashMap<MediaSlot, HashMap<String, CacheEntry>>,
    queue: VecDeque<MediaKey>,
    lane_busy: bool,
    resident_bytes: usize,
    budget_bytes: usize,
    clock: u64,
}

impl Default for MediaCache {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            queue: VecDeque::new(),
            lane_busy: false,
            resident_bytes: 0,
            budget_bytes: MEDIA_CACHE_BUDGET_BYTES,
            clock: 0,
        }
    }
}

impl MediaCache {
    #[cfg(test)]
    fn contains(&self, uri: &str, slot: MediaSlot) -> bool {
        self.entries
            .get(&slot)
            .is_some_and(|shelf| shelf.contains_key(uri))
    }

    /// Looks `uri` up once; a hit advances the LRU clock and a ready hit is stamped with it.
    fn lookup(&mut self, uri: &str, slot: MediaSlot) -> Lookup {
        let Some(entry) = self
            .entries
            .get_mut(&slot)
            .and_then(|shelf| shelf.get_mut(uri))
        else {
            return Lookup::Miss;
        };
        self.clock += 1;
        match entry {
            CacheEntry::Ready {
                frames, last_used, ..
            } => {
                *last_used = self.clock;
                Lookup::Ready(Arc::clone(frames))
            }
            CacheEntry::Pending => Lookup::Pending,
            CacheEntry::Failed => Lookup::Failed,
        }
    }

    fn enqueue(&mut self, key: MediaKey) {
        self.entries
            .entry(key.slot)
            .or_default()
            .insert(key.uri.clone(), CacheEntry::Pending);
        self.queue.push_back(key);
    }

    /// Test-and-set: `true` when the caller now owns the (previously idle) decode lane.
    fn claim_lane(&mut self) -> bool {
        !std::mem::replace(&mut self.lane_busy, true)
    }

    /// Pops the next key to decode, releasing the lane when none is left. Images go before
    /// video keyframes so a loop's poster can show while `ffmpeg` extracts its frames.
    fn next_job(&mut self) -> Option<MediaKey> {
        let image_job = self
            .queue
            .iter()
            .position(|key| key.slot.kind == MediaKind::Image);
        let next = self.queue.remove(image_job.unwrap_or(0));
        self.lane_busy = next.is_some();
        next
    }

    fn store(&mut self, key: MediaKey, decoded: Option<Frames>) {
        let entry = match decoded {
            Some(frames) => {
                let bytes = decoded_bytes(&frames);
                self.resident_bytes += bytes;
                self.clock += 1;
                CacheEntry::Ready {
                    frames,
                    bytes,
                    last_used: self.clock,
                }
            }
            None => CacheEntry::Failed,
        };
        let shelf = self.entries.entry(key.slot).or_default();
        if let Some(CacheEntry::Ready { bytes, .. }) = shelf.insert(key.uri, entry) {
            self.resident_bytes = self.resident_bytes.saturating_sub(bytes);
        }
    }

    /// Removes least-recently-used ready entries (never `keep`) until the cache fits its
    /// budget, returning the evicted images so their atlas tiles can be freed.
    fn evict_over_budget(&mut self, keep: &MediaKey) -> Vec<Arc<RenderImage>> {
        let mut evicted = Vec::new();
        while self.resident_bytes > self.budget_bytes {
            let Some(victim) = self.least_recently_used(keep) else {
                break;
            };
            let removed = self
                .entries
                .get_mut(&victim.slot)
                .and_then(|shelf| shelf.remove(&victim.uri));
            if let Some(CacheEntry::Ready { frames, bytes, .. }) = removed {
                self.resident_bytes = self.resident_bytes.saturating_sub(bytes);
                evicted.extend(frames.iter().cloned());
            }
        }
        evicted
    }

    fn least_recently_used(&self, keep: &MediaKey) -> Option<MediaKey> {
        self.entries
            .iter()
            .flat_map(|(slot, shelf)| shelf.iter().map(move |(uri, entry)| (*slot, uri, entry)))
            .filter(|(slot, uri, _)| *slot != keep.slot || **uri != keep.uri)
            .filter_map(|(slot, uri, entry)| match entry {
                CacheEntry::Ready { last_used, .. } => Some((slot, uri, *last_used)),
                CacheEntry::Pending | CacheEntry::Failed => None,
            })
            .min_by_key(|(_, _, last_used)| *last_used)
            .map(|(slot, uri, _)| MediaKey {
                slot,
                uri: uri.clone(),
            })
    }

    /// Forgets queued and in-flight work so the next render requests it again.
    fn abandon_lane(&mut self) {
        self.queue.clear();
        for shelf in self.entries.values_mut() {
            shelf.retain(|_, entry| !matches!(entry, CacheEntry::Pending));
        }
        self.lane_busy = false;
    }
}

fn decoded_bytes(frames: &[Arc<RenderImage>]) -> usize {
    frames
        .iter()
        .flat_map(|image| (0..image.frame_count()).filter_map(|ix| image.as_bytes(ix)))
        .map(<[u8]>::len)
        .sum()
}

#[cfg(test)]
mod tests {
    use image::{Frame, RgbaImage};

    use super::*;

    const THUMB_IMAGE: MediaSlot = MediaSlot {
        kind: MediaKind::Image,
        tier: ImageTier::Thumbnail,
    };
    const THUMB_LOOP: MediaSlot = MediaSlot {
        kind: MediaKind::VideoKeyframes,
        tier: ImageTier::Thumbnail,
    };

    fn key(slot: MediaSlot, uri: &str) -> MediaKey {
        MediaKey {
            slot,
            uri: uri.to_owned(),
        }
    }

    /// One decoded `width`×1 RGBA image: `width * 4` bytes.
    fn image_frames(width: u32) -> Frames {
        let image = RenderImage::new(vec![Frame::new(RgbaImage::new(width, 1))]);
        Arc::from([Arc::new(image)])
    }

    #[test]
    fn eviction_keeps_the_cache_within_budget_and_drops_the_least_recently_used() {
        // Room for two 1,000-byte images, not three.
        let mut cache = MediaCache {
            budget_bytes: 2_500,
            ..MediaCache::default()
        };
        let b_frames = image_frames(250);
        for (uri, frames) in [("a", image_frames(250)), ("b", Arc::clone(&b_frames))] {
            cache.store(key(THUMB_IMAGE, uri), Some(frames));
            assert!(cache.evict_over_budget(&key(THUMB_IMAGE, uri)).is_empty());
        }
        // Showing `a` again makes `b` the least recently used.
        assert!(matches!(cache.lookup("a", THUMB_IMAGE), Lookup::Ready(_)));

        cache.store(key(THUMB_IMAGE, "c"), Some(image_frames(250)));
        let evicted = cache.evict_over_budget(&key(THUMB_IMAGE, "c"));

        assert_eq!(evicted.len(), 1);
        assert!(Arc::ptr_eq(&evicted[0], &b_frames[0]), "`b` is evicted");
        assert_eq!(cache.resident_bytes, 2_000);
        assert!(cache.resident_bytes <= cache.budget_bytes);
        assert!(!cache.contains("b", THUMB_IMAGE));
        assert!(cache.contains("a", THUMB_IMAGE) && cache.contains("c", THUMB_IMAGE));
    }

    #[test]
    fn misses_share_one_decode_lane_that_decodes_one_key_at_a_time() {
        let mut cache = MediaCache::default();
        cache.enqueue(key(THUMB_LOOP, "loop.mp4"));
        assert!(cache.claim_lane(), "the first miss starts the decode lane");
        cache.enqueue(key(THUMB_IMAGE, "a.jpg"));
        assert!(
            !cache.claim_lane(),
            "a miss while the lane is busy only queues its key"
        );
        // Pending entries count as cached, so a repeated miss is never queued twice.
        assert!(cache.contains("a.jpg", THUMB_IMAGE));

        // Still images go first so a loop's poster can show while its keyframes decode.
        assert_eq!(cache.next_job(), Some(key(THUMB_IMAGE, "a.jpg")));
        assert!(
            !cache.claim_lane(),
            "the lane stays claimed while a key decodes"
        );
        cache.store(key(THUMB_IMAGE, "a.jpg"), Some(image_frames(1)));
        assert_eq!(cache.next_job(), Some(key(THUMB_LOOP, "loop.mp4")));
        cache.store(key(THUMB_LOOP, "loop.mp4"), None);

        assert_eq!(
            cache.next_job(),
            None,
            "the lane is released once the queue drains"
        );
        assert!(cache.claim_lane(), "the next miss starts a new lane");
    }

    #[test]
    fn lookup_tells_a_pending_decode_from_a_failed_one() {
        let mut cache = MediaCache::default();
        cache.enqueue(key(THUMB_IMAGE, "slow.jpg"));
        assert!(matches!(
            cache.lookup("slow.jpg", THUMB_IMAGE),
            Lookup::Pending
        ));
        cache.store(key(THUMB_IMAGE, "slow.jpg"), None);
        assert!(matches!(
            cache.lookup("slow.jpg", THUMB_IMAGE),
            Lookup::Failed
        ));
    }
}
