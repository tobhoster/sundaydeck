#!/usr/bin/env bash
# sync-icons.sh — Download only the curated Phosphor SVG icons needed by SundayDeck (MIT license).
# Assets are vendored into crates/sundaydeck-ui/assets/icons/ and embedded via rust-embed so
# offline Sunday-morning operation never fetches icons over the network.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/crates/sundaydeck-ui/assets/icons"
BASE="https://raw.githubusercontent.com/phosphor-icons/core/main/assets"
mkdir -p "$OUT"

# Regular weight SVGs
REGULAR_ICONS=(
  arrow-counter-clockwise
  bell-ringing
  book-open-text
  broadcast
  caret-down
  caret-left
  caret-right
  caret-up
  chalkboard-simple
  chat-centered-text
  check
  clock
  command
  crosshair
  eye-slash
  gear-six
  image
  lightning
  list-numbers
  lock-simple
  magnifying-glass
  megaphone
  microphone
  microphone-slash
  monitor
  music-notes
  palette
  pencil-simple
  plus
  rows
  sliders-horizontal
  stack
  text-t-slash
  timer
  trash
  upload-simple
  waveform
  x
)

# Fill weight SVGs for crisp active/on-air state toggling
FILL_ICONS=(
  bell-ringing
  book-open-text
  broadcast
  microphone
  monitor
  music-notes
  pause
  play
  stack
  stop
  timer
)

fetch_svg() {
  local weight=$1 name=$2
  local remote_name="$name"
  [[ $weight == "fill" ]] && remote_name="${name}-fill"
  local dest="$OUT/${remote_name}.svg"
  if [[ -s "$dest" ]]; then
    return
  fi
  echo "  fetch  $weight/$remote_name.svg"
  curl -fsSL "$BASE/$weight/$remote_name.svg" -o "$dest.tmp"
  mv "$dest.tmp" "$dest"
}

echo "Syncing curated Phosphor SVGs into $OUT..."
for icon in "${REGULAR_ICONS[@]}"; do
  fetch_svg regular "$icon"
done
for icon in "${FILL_ICONS[@]}"; do
  fetch_svg fill "$icon"
done

du -sh "$OUT"
ls -1 "$OUT"/*.svg | wc -l | xargs -I{} echo "Done: {} SVG icons synced."
