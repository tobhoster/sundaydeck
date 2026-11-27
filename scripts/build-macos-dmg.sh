#!/usr/bin/env bash
set -euo pipefail

# Build a native macOS SundayDeck.app bundle and drag-to-Applications .dmg installer.
# Run on macOS (Apple Silicon arm64 or Intel x86_64) with Xcode Command Line Tools + Rust installed.

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "error: build-macos-dmg.sh must run on macOS (GPUI links native Metal/Cocoa/CoreText frameworks)." >&2
  echo "hint:  Run this script on your MacBook Pro, or trigger .github/workflows/macos-dmg.yml on GitHub Actions." >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ARCH="$(uname -m)"
VERSION="$(grep '^version =' Cargo.toml | head -n 1 | cut -d '"' -f 2)"
APP_NAME="SundayDeck"
DIST_DIR="$ROOT_DIR/dist"
APP_BUNDLE="$DIST_DIR/${APP_NAME}.app"
STAGING_DIR="$DIST_DIR/dmg-staging"
DMG_PATH="$DIST_DIR/${APP_NAME}-${VERSION}-macOS-${ARCH}.dmg"

echo "==> Building ${APP_NAME} v${VERSION} (${ARCH}) release binary..."
cargo build --release -p sundaydeck-ui --bin sundaydeck

rm -rf "$APP_BUNDLE" "$STAGING_DIR" "$DMG_PATH"
mkdir -p "$APP_BUNDLE/Contents/MacOS" "$APP_BUNDLE/Contents/Resources" "$STAGING_DIR"

cp "$ROOT_DIR/target/release/sundaydeck" "$APP_BUNDLE/Contents/MacOS/sundaydeck"
chmod +x "$APP_BUNDLE/Contents/MacOS/sundaydeck"

# Bundle optional offline looping sanctuary video asset alongside the binary
if [[ -d "$ROOT_DIR/crates/sundaydeck-ui/assets/video" ]]; then
  cp -R "$ROOT_DIR/crates/sundaydeck-ui/assets/video" "$APP_BUNDLE/Contents/Resources/video"
fi

# Generate macOS SundayDeck.icns directly from our bundled generic church emblem PNG via sips + iconutil
ICONSET_DIR="$DIST_DIR/SundayDeck.iconset"
rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"

LOGO_SRC="$ROOT_DIR/crates/sundaydeck-ui/assets/backgrounds/default-church-logo.png"
if [[ ! -f "$LOGO_SRC" ]]; then
  echo "error: missing church logo emblem at $LOGO_SRC" >&2
  exit 1
fi

for sz in 16 32 128 256 512; do
  sips -z "$sz" "$sz" "$LOGO_SRC" --out "$ICONSET_DIR/icon_${sz}x${sz}.png" >/dev/null
  dbl=$((sz * 2))
  sips -z "$dbl" "$dbl" "$LOGO_SRC" --out "$ICONSET_DIR/icon_${sz}x${sz}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET_DIR" -o "$APP_BUNDLE/Contents/Resources/SundayDeck.icns"
rm -rf "$ICONSET_DIR"

cat > "$APP_BUNDLE/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>${APP_NAME}</string>
  <key>CFBundleDisplayName</key>
  <string>${APP_NAME}</string>
  <key>CFBundleIdentifier</key>
  <string>org.sundaydeck.app</string>
  <key>CFBundleVersion</key>
  <string>${VERSION}</string>
  <key>CFBundleShortVersionString</key>
  <string>${VERSION}</string>
  <key>CFBundleExecutable</key>
  <string>sundaydeck</string>
  <key>CFBundleIconFile</key>
  <string>SundayDeck</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>LSMinimumSystemVersion</key>
  <string>13.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSMicrophoneUsageDescription</key>
  <string>SundayDeck listens locally to follow worship lyrics and Scripture automatically—audio never leaves your computer.</string>
</dict>
</plist>
PLIST

echo "==> Ad-hoc signing ${APP_BUNDLE}..."
codesign --force --deep --sign - "$APP_BUNDLE"

echo "==> Packaging ${DMG_PATH}..."
cp -R "$APP_BUNDLE" "$STAGING_DIR/${APP_NAME}.app"
ln -s /Applications "$STAGING_DIR/Applications"

hdiutil create \
  -volname "$APP_NAME" \
  -srcfolder "$STAGING_DIR" \
  -ov \
  -format UDZO \
  "$DMG_PATH"

rm -rf "$STAGING_DIR"
echo ""
echo "DONE: Built macOS installer DMG:"
echo "  $DMG_PATH"
