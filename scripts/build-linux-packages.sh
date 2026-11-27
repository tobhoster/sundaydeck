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
ICON_PNG="$ROOT_DIR/crates/sundaydeck-ui/assets/backgrounds/default-church-logo.png"
if [[ ! -f "$ICON_PNG" ]]; then
  echo "error: missing church emblem icon at $ICON_PNG" >&2
  exit 1
fi

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
    "$DEB_STAGE/usr/share/icons/hicolor/512x512/apps" \
    "$DEB_STAGE/usr/share/sundaydeck"

  cp "$BIN_PATH" "$DEB_STAGE/usr/bin/sundaydeck"
  chmod 755 "$DEB_STAGE/usr/bin/sundaydeck"
  cp "$ROOT_DIR/packaging/linux/sundaydeck.desktop" "$DEB_STAGE/usr/share/applications/sundaydeck.desktop"
  cp "$ICON_PNG" "$DEB_STAGE/usr/share/icons/hicolor/512x512/apps/sundaydeck.png"
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
Description: Calm, free, offline church presentation software for Sunday volunteers
 SundayDeck is free, open-source church projection software with dual
 projector/stage monitor support, offline Protestant and Catholic Bibles,
 custom slide layouts, OBS livestream overlays, and offline voice auto-follow.
CONTROL

  dpkg-deb --build --root-owner-group "$DEB_STAGE" "$DIST_DIR/sundaydeck_${VERSION}_${DEB_ARCH}.deb"
  rm -rf "$DEB_STAGE"
fi

echo "DONE: Built Linux packages in $DIST_DIR:"
ls -lh "$DIST_DIR"/*.tar.gz "$DIST_DIR"/*.deb 2>/dev/null || true
