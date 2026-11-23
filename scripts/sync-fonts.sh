#!/usr/bin/env bash
# sync-fonts.sh — Download vendored SIL OFL static TTF fonts for offline bundling via include_bytes!.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/crates/sundaydeck-ui/assets/fonts"
mkdir -p "$OUT"

fetch() {
  local name=$1 url=$2
  local dest="$OUT/$name"
  if [[ -s "$dest" ]]; then
    echo "  keep   $name"
    return
  fi
  echo "  fetch  $name"
  curl -fsSL "$url" -o "$dest.tmp"
  mv "$dest.tmp" "$dest"
}

echo "Syncing Operator Console & Sanctuary Projection OFL fonts into $OUT..."

# Operator Display + Minimal Grotesk Projection: Space Grotesk (SIL OFL)
fetch "SpaceGrotesk-Regular.ttf"  "https://fonts.gstatic.com/s/spacegrotesk/v22/V8mQoQDjQSkFtoMM3T6r8E7mF71Q-gOoraIAEj7oUUsj.ttf"
fetch "SpaceGrotesk-Medium.ttf"   "https://fonts.gstatic.com/s/spacegrotesk/v22/V8mQoQDjQSkFtoMM3T6r8E7mF71Q-gOoraIAEj7aUUsj.ttf"
fetch "SpaceGrotesk-SemiBold.ttf" "https://fonts.gstatic.com/s/spacegrotesk/v22/V8mQoQDjQSkFtoMM3T6r8E7mF71Q-gOoraIAEj42Vksj.ttf"
fetch "SpaceGrotesk-Bold.ttf"     "https://fonts.gstatic.com/s/spacegrotesk/v22/V8mQoQDjQSkFtoMM3T6r8E7mF71Q-gOoraIAEj4PVksj.ttf"

# Operator Body & Lyric Lists: Plus Jakarta Sans (SIL OFL)
fetch "PlusJakartaSans-Regular.ttf"  "https://fonts.gstatic.com/s/plusjakartasans/v12/LDIbaomQNQcsA88c7O9yZ4KMCoOg4IA6-91aHEjcWuA_qU7NSg.ttf"
fetch "PlusJakartaSans-Medium.ttf"   "https://fonts.gstatic.com/s/plusjakartasans/v12/LDIbaomQNQcsA88c7O9yZ4KMCoOg4IA6-91aHEjcWuA_m07NSg.ttf"
fetch "PlusJakartaSans-SemiBold.ttf" "https://fonts.gstatic.com/s/plusjakartasans/v12/LDIbaomQNQcsA88c7O9yZ4KMCoOg4IA6-91aHEjcWuA_d0nNSg.ttf"
fetch "PlusJakartaSans-Bold.ttf"     "https://fonts.gstatic.com/s/plusjakartasans/v12/LDIbaomQNQcsA88c7O9yZ4KMCoOg4IA6-91aHEjcWuA_TknNSg.ttf"

# Technical Telemetry & Keycaps: JetBrains Mono (SIL OFL)
fetch "JetBrainsMono-Regular.ttf"  "https://raw.githubusercontent.com/JetBrains/JetBrainsMono/master/fonts/ttf/JetBrainsMono-Regular.ttf"
fetch "JetBrainsMono-Medium.ttf"   "https://raw.githubusercontent.com/JetBrains/JetBrainsMono/master/fonts/ttf/JetBrainsMono-Medium.ttf"
fetch "JetBrainsMono-SemiBold.ttf" "https://raw.githubusercontent.com/JetBrains/JetBrainsMono/master/fonts/ttf/JetBrainsMono-SemiBold.ttf"
fetch "JetBrainsMono-Bold.ttf"     "https://raw.githubusercontent.com/JetBrains/JetBrainsMono/master/fonts/ttf/JetBrainsMono-Bold.ttf"

# Sanctuary Projection Pack: Manrope (Bold/ExtraBold), Syne (Bold/ExtraBold), Newsreader (Regular/SemiBold) (SIL OFL)
fetch "Manrope-Bold.ttf"         "https://fonts.gstatic.com/s/manrope/v23/xn7_YHE41ni1AdIRqAuZuw1Bx9mbZk4aE-_F.ttf"
fetch "Manrope-ExtraBold.ttf"    "https://fonts.gstatic.com/s/manrope/v23/xn7_YHE41ni1AdIRqAuZuw1Bx9mbZk59E-_F.ttf"
fetch "Syne-Bold.ttf"            "https://fonts.gstatic.com/s/syne/v24/8vIS7w4qzmVxsWxjBZRjr0FKM_3fvj6k.ttf"
fetch "Syne-ExtraBold.ttf"       "https://fonts.gstatic.com/s/syne/v24/8vIS7w4qzmVxsWxjBZRjr0FKM_24vj6k.ttf"
fetch "Newsreader-Regular.ttf"   "https://fonts.gstatic.com/s/newsreader/v26/cY9qfjOCX1hbuyalUrK49dLac06G1ZGsZBtoBCzBDXXD9JVF438weI_ADA.ttf"
fetch "Newsreader-SemiBold.ttf"  "https://fonts.gstatic.com/s/newsreader/v26/cY9qfjOCX1hbuyalUrK49dLac06G1ZGsZBtoBCzBDXXD9JVF438wpojADA.ttf"

du -sh "$OUT"
ls -1 "$OUT" | wc -l | xargs -I{} echo "Done: {} font files synced."
