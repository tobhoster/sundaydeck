#!/usr/bin/env python3
"""
Builds a crisp 1200x630 OpenGraph / social sharing preview card (`docs/media/og-card.png`)
for SundayDeck using pango-view (Plus Jakarta Sans) and ffmpeg.
"""

from __future__ import annotations

import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
HERO = ROOT / "docs" / "media" / "hero-workspace.png"
OUT = ROOT / "docs" / "media" / "og-card.png"


def main() -> None:
    tmp = Path(tempfile.mkdtemp(prefix="sundaydeck-og-"))
    try:
        copy_png = tmp / "og-copy.png"
        markup = (
            "<span font_desc='Plus Jakarta Sans SemiBold 13' foreground='#38B780'>"
            "FREE &amp; OPEN SOURCE  ·  NO LICENSES"
            "</span>\n\n"
            "<span font_desc='Plus Jakarta Sans Bold 44' foreground='#F5F3EF'>"
            "SundayDeck"
            "</span>\n\n"
            "<span font_desc='Plus Jakarta Sans Regular 20' foreground='#EAE7E1'>"
            "Calm, free church slides built\nfor real Sunday volunteers."
            "</span>\n\n"
            "<span font_desc='Plus Jakarta Sans Regular 14' foreground='#A5AAB5'>"
            "100% Offline  ·  Mac, Windows &amp; Linux\n"
            "Songs · Bibles · Stage · Livestream · Auto-Follow"
            "</span>"
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
                str(copy_png),
            ],
            check=True,
        )

        # 1200x630 warm dark-linen canvas + framed hero screenshot on right + left typography + amber rule
        filter_complex = (
            "color=c=#131416:s=1200x630:d=1,"
            "drawbox=x=0:y=0:w=1200:h=4:color=#E2A83B:t=fill,"
            "drawbox=x=548:y=72:w=604:h=486:color=#282C36:t=fill,"
            "drawbox=x=550:y=74:w=600:h=482:color=#0E1013:t=fill[canvas];"
            "[0:v]scale=600:482:force_original_aspect_ratio=increase,crop=600:482:0:0[shot];"
            "[canvas][shot]overlay=550:74[with_shot];"
            "[with_shot][1:v]overlay=54:118:format=auto[out]"
        )

        subprocess.run(
            [
                "ffmpeg",
                "-y",
                "-v",
                "error",
                "-i",
                str(HERO),
                "-i",
                str(copy_png),
                "-filter_complex",
                filter_complex,
                "-map",
                "[out]",
                "-frames:v",
                "1",
                str(OUT),
            ],
            check=True,
        )
        print(f"Built {OUT} ({OUT.stat().st_size} bytes)")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    main()
