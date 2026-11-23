#!/usr/bin/env bash
set -euo pipefail

# Build Linux release packages:
#   1. Portable tarball: dist/SundayDeck-<version>-linux-<arch>.tar.gz
#   2. Debian/Ubuntu .deb: dist/sundaydeck_<version>_<deb-arch>.deb

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

VERSION="$(grep '^version =' Cargo.toml | head -n 1 | cut -d '"' -f 2)"
ARCH="$(uname -m)"
DEB_ARCH="amd64"
if [[ "$ARCH" == "aarch64" ]]; then
  DEB_ARCH="arm64"
fi

DIST_DIR="$ROOT_DIR/dist"
mkdir -p "$DIST_DIR"

echo "==> Building SundayDeck v${VERSION} (${ARCH}) release binary..."
cargo build --release -p sundaydeck-ui --bin sundaydeck

BIN_PATH="$ROOT_DIR/target/release/sundaydeck"
ICON_PNG="$DIST_DIR/sundaydeck-256.png"

# Generate clean 256x256 PNG app icon (matte dark linen + warm ivory emblem)
python3 - "$ICON_PNG" <<'PYEOF'
import struct, sys, zlib
w, h = 256, 256
raw = bytearray()
cx, cy = w / 2.0, h / 2.0
for y in range(h):
    raw.append(0)
    for x in range(w):
        dx, dy = abs(x - cx) / 115.0, abs(y - cy) / 115.0
        sq = (dx ** 4.5 + dy ** 4.5)
        if sq > 1.0:
            raw.extend((0, 0, 0, 0))
            continue
        r, g, b, a = 24, 26, 31, 255
        if sq > 0.93:
            r, g, b = 46, 50, 61
        dist = ((x - cx) ** 2 + (y - cy) ** 2) ** 0.5
        if dist < 27:
            r, g, b = 242, 239, 233
        elif 48 < dist < 54:
            r, g, b = 217, 160, 54
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

# 1. Portable tar.gz archive
TAR_STAGE="$DIST_DIR/SundayDeck-${VERSION}-linux-${ARCH}"
rm -rf "$TAR_STAGE"
mkdir -p "$TAR_STAGE"
cp "$BIN_PATH" "$TAR_STAGE/sundaydeck"
chmod +x "$TAR_STAGE/sundaydeck"
cp "$ROOT_DIR/packaging/linux/sundaydeck.desktop" "$TAR_STAGE/sundaydeck.desktop"
cp "$ICON_PNG" "$TAR_STAGE/sundaydeck.png"
if [[ -d "$ROOT_DIR/crates/sundaydeck-ui/assets/video" ]]; then
  cp -R "$ROOT_DIR/crates/sundaydeck-ui/assets/video" "$TAR_STAGE/video"
fi
tar -C "$DIST_DIR" -czf "$DIST_DIR/SundayDeck-${VERSION}-linux-${ARCH}.tar.gz" "$(basename "$TAR_STAGE")"
rm -rf "$TAR_STAGE"

# 2. Debian .deb package (when dpkg-deb is available)
if command -v dpkg-deb >/dev/null 2>&1; then
  DEB_STAGE="$DIST_DIR/deb-root"
  rm -rf "$DEB_STAGE"
  mkdir -p \
    "$DEB_STAGE/DEBIAN" \
    "$DEB_STAGE/usr/bin" \
    "$DEB_STAGE/usr/share/applications" \
    "$DEB_STAGE/usr/share/icons/hicolor/256x256/apps" \
    "$DEB_STAGE/usr/share/sundaydeck"

  cp "$BIN_PATH" "$DEB_STAGE/usr/bin/sundaydeck"
  chmod 755 "$DEB_STAGE/usr/bin/sundaydeck"
  cp "$ROOT_DIR/packaging/linux/sundaydeck.desktop" "$DEB_STAGE/usr/share/applications/sundaydeck.desktop"
  cp "$ICON_PNG" "$DEB_STAGE/usr/share/icons/hicolor/256x256/apps/sundaydeck.png"
  if [[ -d "$ROOT_DIR/crates/sundaydeck-ui/assets/video" ]]; then
    cp -R "$ROOT_DIR/crates/sundaydeck-ui/assets/video" "$DEB_STAGE/usr/share/sundaydeck/video"
  fi

  cat > "$DEB_STAGE/DEBIAN/control" <<CONTROL
Package: sundaydeck
Version: ${VERSION}
Section: graphics
Priority: optional
Architecture: ${DEB_ARCH}
Maintainer: SundayDeck Contributors <maintainers@sundaydeck.org>
Depends: libasound2t64 | libasound2, libfontconfig1, libvulkan1, libxkbcommon0
Description: Fast, offline-first church presentation studio
 SundayDeck is a native Rust + GPUI church projection desktop app with dual
 projector/stage monitor support, offline Bibles, local AI service planning,
 and offline voice-synced slide following.
CONTROL

  dpkg-deb --build --root-owner-group "$DEB_STAGE" "$DIST_DIR/sundaydeck_${VERSION}_${DEB_ARCH}.deb"
  rm -rf "$DEB_STAGE"
fi

rm -f "$ICON_PNG"
echo "DONE: Built Linux packages in $DIST_DIR:"
ls -lh "$DIST_DIR"/*.tar.gz "$DIST_DIR"/*.deb 2>/dev/null || true
