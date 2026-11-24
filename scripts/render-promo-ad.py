#!/usr/bin/env python3
"""
Builds SundayDeck's studio-voiced product launch video (`docs/media/sundaydeck-promo-ad.mp4`)
using a local Kokoro v1.0 TTS server (OpenAI-compatible `/v1/audio/speech`),
pango-view (Plus Jakarta Sans transparent editorial lower-thirds), and ffmpeg.

Usage:
    python3 scripts/render-promo-ad.py --tts-url http://127.0.0.1:18090/v1/audio/speech
"""

from __future__ import annotations

import argparse
import html
import json
import shutil
import subprocess
import tempfile
import urllib.request
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SHOTS = ROOT / "target" / "ui-review-shots"
DEFAULT_OUT = ROOT / "docs" / "media" / "sundaydeck-promo-ad.mp4"


@dataclass
class AdScene:
    slug: str
    images: list[str]
    headline: str
    subline: str
    voiceover: str
    lead_pad: float = 0.25
    tail_pad: float = 0.45


SCENES: list[AdScene] = [
    AdScene(
        slug="01-intro",
        images=["01-operator-workspace-live.png"],
        headline="SundayDeck",
        subline="Calm church presentation software for Sunday volunteers",
        voiceover=(
            "Meet SundayDeck. Free, calm church presentation software built for "
            "real Sunday volunteers — clear, fast, and completely offline."
        ),
        lead_pad=0.35,
        tail_pad=0.45,
    ),
    AdScene(
        slug="02-plan-import",
        images=["14-pastor-plan-importer-modal.png"],
        headline="Notes to Service Order in Seconds",
        subline="Drop a pastor's PowerPoint, Word document, or sermon outline",
        voiceover=(
            "Drop in your pastor's PowerPoint or Word document, and SundayDeck "
            "builds your entire service order in seconds — songs, Scriptures, and "
            "sermon points ready to review."
        ),
    ),
    AdScene(
        slug="03-custom-slides",
        images=[
            "30-spec-custom-slide-builder-modal.png",
            "33-spec-audience-progressive-bullets.png",
        ],
        headline="Built-In Slide Layouts",
        subline="Welcome cards, progressive sermon points, quotes, and countdowns",
        voiceover=(
            "Create welcome cards, step-by-step sermon points, and pre-service "
            "countdowns right inside the app, with gentle transitions and no extra software."
        ),
    ),
    AdScene(
        slug="04-sanctuary-media",
        images=[
            "17-audience-unsplash-mountains.png",
            "18-audience-motion-video-golden.png",
            "03-stage-monitor-live.png",
        ],
        headline="Sanctuary, Stage & Livestream Ready",
        subline="Peaceful photography, looping motion backgrounds, and stage confidence monitor",
        voiceover=(
            "Project over peaceful photography and gentle motion backgrounds, "
            "with dedicated views for your sanctuary screen, stage monitor, and livestream."
        ),
    ),
    AdScene(
        slug="05-voice-follow",
        images=[
            "22-voice-auto-follow-live-tracking.png",
            "31-spec-custom-slides-scripture-queue-obs.png",
        ],
        headline="Hands-Free Voice Auto-Follow",
        subline="Keeps pace with singing and queues spoken Bible verses for one click",
        voiceover=(
            "Turn on offline microphone auto-follow to keep pace as your congregation "
            "sings, and catch spoken Bible verses in a one-click queue."
        ),
    ),
    AdScene(
        slug="06-americas-churches",
        images=[
            "35-global-audience-liturgy-response.png",
            "36-global-settings-church-style-packs.png",
        ],
        headline="Made for Every Church Tradition",
        subline="Protestant & Catholic Bibles · Numbered Hymnbooks · English, Spanish & Portuguese",
        voiceover=(
            "Built for churches across North and South America — with Catholic and "
            "Protestant Bibles, numbered hymnbooks, English, Spanish, and Portuguese "
            "bilingual lyrics, call-and-response readings, and portable service packs."
        ),
    ),
    AdScene(
        slug="07-outro",
        images=["29-phase4-audience-custom-logo.png"],
        headline="Free Forever. Zero Subscriptions.",
        subline="Download SundayDeck for Mac, Windows, and Linux",
        voiceover=(
            "No subscriptions. No internet required. Ready for Sunday morning on "
            "Mac, Windows, and Linux. Download SundayDeck free today."
        ),
        lead_pad=0.25,
        tail_pad=0.90,
    ),
]


def synthesize_kokoro(tts_url: str, voice: str, text: str, dest: Path) -> None:
    payload = json.dumps(
        {
            "model": "kokoro",
            "voice": voice,
            "input": text,
            "response_format": "wav",
        }
    ).encode("utf-8")
    req = urllib.request.Request(
        tts_url,
        data=payload,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=60) as resp:
        dest.write_bytes(resp.read())


def probe_duration(media: Path) -> float:
    out = subprocess.check_output(
        [
            "ffprobe",
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            str(media),
        ],
        text=True,
    ).strip()
    return float(out)


def render_lower_third_png(headline: str, subline: str, out_png: Path) -> None:
    h_esc = html.escape(headline)
    s_esc = html.escape(subline)
    markup = (
        f"<span font_desc='Plus Jakarta Sans SemiBold 22' foreground='#F5F3EF'>{h_esc}</span>\n"
        f"<span font_desc='Plus Jakarta Sans Regular 14' foreground='#A5AAB5'>{s_esc}</span>"
    )
    subprocess.run(
        [
            "pango-view",
            "--backend=cairo",
            "--background=transparent",
            "--markup",
            "--dpi=96",
            "--margin=0",
            "--line-spacing=1.18",
            "-q",
            "-t",
            markup,
            "-o",
            str(out_png),
        ],
        check=True,
    )


def render_image_clip(
    image_path: Path,
    duration: float,
    caption_png: Path,
    out_mp4: Path,
    zoom_in: bool = True,
) -> None:
    fps = 30
    frames = max(15, int(round(duration * fps)))
    if zoom_in:
        zoom_expr = f"1.0+0.038*on/{frames}"
    else:
        zoom_expr = f"1.038-0.038*on/{frames}"

    filter_complex = (
        f"[0:v]scale=1280:720:force_original_aspect_ratio=decrease,"
        f"pad=1280:720:(ow-iw)/2:(oh-ih)/2:color=#141517,"
        f"scale=2560:1440,"
        f"zoompan=z='{zoom_expr}':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d={frames}:s=1280x720:fps={fps},"
        f"drawbox=x=0:y=606:w=1280:h=114:color=#141517@0.90:t=fill,"
        f"drawbox=x=0:y=606:w=1280:h=2:color=#2B2F3A@0.95:t=fill,"
        f"drawbox=x=44:y=630:w=4:h=64:color=#E2A83B@0.95:t=fill[bg];"
        f"[bg][1:v]overlay=64:627:format=auto,format=yuv420p[vout]"
    )

    subprocess.run(
        [
            "ffmpeg",
            "-y",
            "-v",
            "error",
            "-loop",
            "1",
            "-i",
            str(image_path),
            "-i",
            str(caption_png),
            "-filter_complex",
            filter_complex,
            "-map",
            "[vout]",
            "-t",
            f"{duration:.3f}",
            "-r",
            str(fps),
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            "18",
            "-pix_fmt",
            "yuv420p",
            str(out_mp4),
        ],
        check=True,
    )


def pad_audio(wav_in: Path, lead_pad: float, tail_pad: float, wav_out: Path) -> float:
    lead_ms = int(round(lead_pad * 1000))
    subprocess.run(
        [
            "ffmpeg",
            "-y",
            "-v",
            "error",
            "-i",
            str(wav_in),
            "-af",
            f"adelay={lead_ms}|{lead_ms},apad=pad_dur={tail_pad:.3f}",
            "-ar",
            "48000",
            "-ac",
            "2",
            str(wav_out),
        ],
        check=True,
    )
    return probe_duration(wav_out)


def main() -> None:
    parser = argparse.ArgumentParser(description="Render SundayDeck Kokoro-voiced launch ad video.")
    parser.add_argument("--tts-url", default="http://127.0.0.1:18090/v1/audio/speech")
    parser.add_argument("--voice", default="af_heart")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = parser.parse_args()

    tmp = Path(tempfile.mkdtemp(prefix="sundaydeck-promo-ad-"))
    try:
        scene_videos: list[tuple[Path, float]] = []
        scene_audios: list[Path] = []
        xfade_dur = 0.35

        for idx, scene in enumerate(SCENES):
            raw_wav = tmp / f"{scene.slug}.raw.wav"
            padded_wav = tmp / f"{scene.slug}.pad.wav"
            caption_png = tmp / f"{scene.slug}.caption.png"

            print(f"[{idx + 1}/{len(SCENES)}] Synthesizing Kokoro narration: {scene.slug}")
            synthesize_kokoro(args.tts_url, args.voice, scene.voiceover, raw_wav)
            scene_dur = pad_audio(raw_wav, scene.lead_pad, scene.tail_pad, padded_wav)
            scene_audios.append(padded_wav)
            render_lower_third_png(scene.headline, scene.subline, caption_png)

            n_imgs = len(scene.images)
            target_visual_dur = scene_dur + (xfade_dur if idx < len(SCENES) - 1 else 0.0)
            per_img = target_visual_dur / n_imgs

            img_clips: list[Path] = []
            for j, img_name in enumerate(scene.images):
                img_path = SHOTS / img_name
                if not img_path.exists():
                    raise FileNotFoundError(f"Missing screenshot: {img_path}")
                clip_path = tmp / f"{scene.slug}_{j}.mp4"
                render_image_clip(
                    img_path,
                    per_img,
                    caption_png,
                    clip_path,
                    zoom_in=((idx + j) % 2 == 0),
                )
                img_clips.append(clip_path)

            if len(img_clips) == 1:
                scene_mp4 = img_clips[0]
            else:
                concat_txt = tmp / f"{scene.slug}.concat.txt"
                concat_txt.write_text(
                    "\n".join(f"file '{p}'" for p in img_clips) + "\n",
                    encoding="utf-8",
                )
                scene_mp4 = tmp / f"{scene.slug}.scene.mp4"
                subprocess.run(
                    [
                        "ffmpeg",
                        "-y",
                        "-v",
                        "error",
                        "-f",
                        "concat",
                        "-safe",
                        "0",
                        "-i",
                        str(concat_txt),
                        "-c",
                        "copy",
                        str(scene_mp4),
                    ],
                    check=True,
                )
            scene_videos.append((scene_mp4, probe_duration(scene_mp4)))

        audio_concat_txt = tmp / "audio.concat.txt"
        audio_concat_txt.write_text(
            "\n".join(f"file '{p}'" for p in scene_audios) + "\n",
            encoding="utf-8",
        )
        full_audio = tmp / "full_narration.wav"
        subprocess.run(
            [
                "ffmpeg",
                "-y",
                "-v",
                "error",
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
                str(audio_concat_txt),
                "-af",
                "loudnorm=I=-16:TP=-1.5:LRA=11",
                "-ar",
                "48000",
                "-ac",
                "2",
                str(full_audio),
            ],
            check=True,
        )

        v_inputs: list[str] = []
        for path, _ in scene_videos:
            v_inputs.extend(["-i", str(path)])

        filter_chains: list[str] = []
        prev_label = "0:v"
        offset = 0.0
        for i in range(1, len(scene_videos)):
            offset += scene_videos[i - 1][1] - xfade_dur
            out_label = f"v{i}"
            filter_chains.append(
                f"[{prev_label}][{i}:v]xfade=transition=fade:duration={xfade_dur:.2f}:offset={offset:.3f}[{out_label}]"
            )
            prev_label = out_label

        total_audio_dur = probe_duration(full_audio)
        fade_out_start = max(0.0, total_audio_dur - 0.55)
        filter_chains.append(
            f"[{prev_label}]fade=t=in:st=0:d=0.35,fade=t=out:st={fade_out_start:.3f}:d=0.50,format=yuv420p[vout]"
        )

        args.out.parent.mkdir(parents=True, exist_ok=True)
        cmd = [
            "ffmpeg",
            "-y",
            "-v",
            "error",
            *v_inputs,
            "-i",
            str(full_audio),
            "-filter_complex",
            ";".join(filter_chains),
            "-map",
            "[vout]",
            "-map",
            f"{len(scene_videos)}:a",
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            "20",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-movflags",
            "+faststart",
            "-shortest",
            str(args.out),
        ]
        print(f"Muxing final narrated launch video -> {args.out} ({total_audio_dur:.1f}s)")
        subprocess.run(cmd, check=True)
        final_dur = probe_duration(args.out)
        size_mb = args.out.stat().st_size / (1024 * 1024)
        print(f"Done! {args.out} — {final_dur:.1f}s, {size_mb:.2f} MB")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    main()
