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

# Generate clean dark-linen & ivory SundayDeck.icns icon via Python + sips/iconutil
ICONSET_DIR="$DIST_DIR/SundayDeck.iconset"
rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"

python3 - "$ICONSET_DIR/base_1024.png" <<'PYEOF'
import struct, sys, zlib

w, h = 1024, 1024
raw = bytearray()
cx, cy = w / 2.0, h / 2.0
for y in range(h):
    raw.append(0)
    for x in range(w):
        # Rounded macOS squircle matte dark linen (#181A1F) with ivory emblem dot & ring
        dx, dy = abs(x - cx) / 460.0, abs(y - cy) / 460.0
        squircle = (dx ** 4.5 + dy ** 4.5)
        if squircle > 1.0:
            raw.extend((0, 0, 0, 0))
            continue
        r, g, b, a = 24, 26, 31, 255
        if squircle > 0.94:
            r, g, b = 46, 50, 61
        dist = ((x - cx) ** 2 + (y - cy) ** 2) ** 0.5
        if dist < 110:
            r, g, b = 242, 239, 233  # warm paper ivory center dot
        elif 195 < dist < 216:
            r, g, b = 217, 160, 54   # warm honey amber precision halo
        raw.extend((r, g, b, a))

def chunk(tag, data):
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

png = b"\x89PNG\r\n\x1a\n"
png += chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
png += chunk(b"IDAT", zlib.compress(bytes(raw), 6))
png += chunk(b"IEND", b"")
with open(sys.argv[1], "wb") as f:
    f.write(png)
PYEOF

for sz in 16 32 128 256 512; do
  sips -z "$sz" "$sz" "$ICONSET_DIR/base_1024.png" --out "$ICONSET_DIR/icon_${sz}x${sz}.png" >/dev/null
  dbl=$((sz * 2))
  sips -z "$dbl" "$dbl" "$ICONSET_DIR/base_1024.png" --out "$ICONSET_DIR/icon_${sz}x${sz}@2x.png" >/dev/null
done
rm -f "$ICONSET_DIR/base_1024.png"
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
