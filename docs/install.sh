#!/bin/sh
# SundayDeck one-line installer for macOS (Apple Silicon & Intel) and Linux (x86_64).
# Usage:
#   curl -fsSL https://tobhoster.github.io/sundaydeck/install.sh | sh
#
# Environment overrides:
#   SUNDAYDECK_VERSION=0.0.1     Install a specific release version (without leading 'v')
#   SUNDAYDECK_INSTALL_DIR=...   Override Linux install prefix (default: $HOME/.local)
#   SUNDAYDECK_APP_DIR=...       Override macOS Applications directory (default: /Applications)

set -eu

REPO="tobhoster/sundaydeck"
FALLBACK_VERSION="0.0.1"

say() {
  printf '%s\n' "$@"
}

err() {
  printf 'error: %s\n' "$@" >&2
  exit 1
}

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    err "required command not found: $1"
  fi
}

fetch() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -H "User-Agent: SundayDeck-Installer" "$1"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "$1"
  else
    err "either curl or wget is required"
  fi
}

download_file() {
  _url="$1"
  _dest="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fL --progress-bar -H "User-Agent: SundayDeck-Installer" "$_url" -o "$_dest"
  elif command -v wget >/dev/null 2>&1; then
    wget -q --show-progress "$_url" -O "$_dest"
  else
    err "either curl or wget is required"
  fi
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    printf ''
  fi
}

resolve_version() {
  if [ -n "${SUNDAYDECK_VERSION:-}" ]; then
    printf '%s' "${SUNDAYDECK_VERSION#v}"
    return
  fi
  _tag="$(fetch "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
    | head -n 1 || true)"
  if [ -n "$_tag" ]; then
    printf '%s' "${_tag#v}"
  else
    printf '%s' "$FALLBACK_VERSION"
  fi
}

verify_checksum() {
  _file="$1"
  _asset_name="$2"
  _sums_url="$3"
  _expected="$(fetch "$_sums_url" 2>/dev/null | awk -v name="$_asset_name" '$2 == name || $2 == ("*" name) {print $1; exit}' || true)"
  if [ -z "$_expected" ]; then
    return 0
  fi
  _actual="$(sha256_file "$_file")"
  if [ -z "$_actual" ]; then
    return 0
  fi
  if [ "$_actual" != "$_expected" ]; then
    err "SHA-256 mismatch for $_asset_name (expected $_expected, got $_actual)"
  fi
  say "Verified SHA-256 checksum for $_asset_name."
}

main() {
  OS="$(uname -s)"
  ARCH="$(uname -m)"
  VERSION="$(resolve_version)"
  TAG="v${VERSION}"
  BASE_URL="https://github.com/${REPO}/releases/download/${TAG}"

  TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/sundaydeck-install.XXXXXX")"
  MOUNT_DIR=""
  cleanup() {
    if [ -n "$MOUNT_DIR" ] && [ -d "$MOUNT_DIR" ]; then
      hdiutil detach "$MOUNT_DIR" -quiet >/dev/null 2>&1 || true
    fi
    rm -rf "$TMP_DIR"
  }
  trap cleanup EXIT INT TERM

  say "Installing SundayDeck ${TAG} (${OS}/${ARCH})..."

  case "$OS" in
    Darwin)
      need_cmd hdiutil
      case "$ARCH" in
        arm64|aarch64) MAC_ARCH="arm64" ;;
        x86_64|amd64)  MAC_ARCH="x86_64" ;;
        *) err "unsupported macOS architecture: $ARCH" ;;
      esac

      ASSET="SundayDeck-${VERSION}-macOS-${MAC_ARCH}.dmg"
      DMG_PATH="${TMP_DIR}/${ASSET}"
      say "Downloading ${ASSET}..."
      download_file "${BASE_URL}/${ASSET}" "$DMG_PATH"
      verify_checksum "$DMG_PATH" "$ASSET" "${BASE_URL}/SHA256SUMS.txt"

      MOUNT_DIR="${TMP_DIR}/mnt"
      mkdir -p "$MOUNT_DIR"
      hdiutil attach "$DMG_PATH" -nobrowse -quiet -mountpoint "$MOUNT_DIR"

      APP_DIR="${SUNDAYDECK_APP_DIR:-/Applications}"
      if [ ! -w "$APP_DIR" ]; then
        APP_DIR="${HOME}/Applications"
        mkdir -p "$APP_DIR"
      fi

      DEST_APP="${APP_DIR}/SundayDeck.app"
      rm -rf "$DEST_APP"
      cp -R "${MOUNT_DIR}/SundayDeck.app" "$DEST_APP"
      hdiutil detach "$MOUNT_DIR" -quiet || true
      MOUNT_DIR=""

      # Clear download quarantine attribute on ad-hoc signed community builds if permitted
      xattr -dr com.apple.quarantine "$DEST_APP" >/dev/null 2>&1 || true

      say ""
      say "SundayDeck ${TAG} installed to: ${DEST_APP}"
      say "Launch it from Applications, Spotlight, or run:"
      say "  open -a SundayDeck"
      ;;

    Linux)
      need_cmd tar
      case "$ARCH" in
        x86_64|amd64) LINUX_ARCH="x86_64" ;;
        *) err "prebuilt Linux archives are currently x86_64 only (detected $ARCH). Build from source: https://github.com/${REPO}" ;;
      esac

      ASSET="SundayDeck-${VERSION}-linux-${LINUX_ARCH}.tar.gz"
      TAR_PATH="${TMP_DIR}/${ASSET}"
      say "Downloading ${ASSET}..."
      download_file "${BASE_URL}/${ASSET}" "$TAR_PATH"
      verify_checksum "$TAR_PATH" "$ASSET" "${BASE_URL}/SHA256SUMS.txt"

      PREFIX="${SUNDAYDECK_INSTALL_DIR:-${HOME}/.local}"
      BIN_DIR="${PREFIX}/bin"
      SHARE_DIR="${PREFIX}/share"
      mkdir -p "$BIN_DIR" "${SHARE_DIR}/applications" "${SHARE_DIR}/icons/hicolor/512x512/apps" "${SHARE_DIR}/sundaydeck"

      tar -xzf "$TAR_PATH" -C "$TMP_DIR"
      STAGE_DIR="${TMP_DIR}/SundayDeck-${VERSION}-linux-${LINUX_ARCH}"
      [ -d "$STAGE_DIR" ] || err "unexpected archive structure inside ${ASSET}"

      cp "${STAGE_DIR}/sundaydeck" "${BIN_DIR}/sundaydeck"
      chmod 755 "${BIN_DIR}/sundaydeck"

      if [ -f "${STAGE_DIR}/sundaydeck.png" ]; then
        cp "${STAGE_DIR}/sundaydeck.png" "${SHARE_DIR}/icons/hicolor/512x512/apps/sundaydeck.png"
      fi
      if [ -d "${STAGE_DIR}/video" ]; then
        rm -rf "${SHARE_DIR}/sundaydeck/video"
        cp -R "${STAGE_DIR}/video" "${SHARE_DIR}/sundaydeck/video"
      fi
      if [ -f "${STAGE_DIR}/sundaydeck.desktop" ]; then
        sed "s|^Exec=.*|Exec=${BIN_DIR}/sundaydeck|" "${STAGE_DIR}/sundaydeck.desktop" \
          > "${SHARE_DIR}/applications/sundaydeck.desktop"
      fi

      say ""
      say "SundayDeck ${TAG} installed to: ${BIN_DIR}/sundaydeck"
      case ":${PATH}:" in
        *":${BIN_DIR}:"*)
          say "Run 'sundaydeck' or open SundayDeck from your application launcher."
          ;;
        *)
          say "Add ${BIN_DIR} to your PATH to launch from the terminal:"
          say "  export PATH=\"${BIN_DIR}:\$PATH\""
          say "  sundaydeck"
          ;;
      esac
      ;;

    *)
      err "unsupported operating system: $OS. Download Windows installers at https://tobhoster.github.io/sundaydeck/download.html"
      ;;
  esac
}

main "$@"
