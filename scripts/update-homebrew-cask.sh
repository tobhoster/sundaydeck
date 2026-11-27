#!/usr/bin/env bash
set -euo pipefail

# Generates an updated Homebrew Cask formula with real SHA-256 hashes from built macOS DMGs.
# Usage: ./scripts/update-homebrew-cask.sh <github-owner/repo> <version> <dist-dir>

REPO="${1:-tobhoster/sundaydeck}"
VERSION="${2:-0.1.0}"
ASSET_DIR="${3:-dist}"

ARM_DMG="$ASSET_DIR/SundayDeck-${VERSION}-macOS-arm64.dmg"
INTEL_DMG="$ASSET_DIR/SundayDeck-${VERSION}-macOS-x86_64.dmg"

hash_of() {
  if [[ -f "$1" ]]; then
    sha256sum "$1" | awk '{print $1}'
  else
    echo "REPLACE_WITH_SHA256_FROM_RELEASE"
  fi
}

SHA_ARM="$(hash_of "$ARM_DMG")"
SHA_INTEL="$(hash_of "$INTEL_DMG")"

cat <<CASK
cask "sundaydeck" do
  arch arm: "arm64", intel: "x86_64"

  version "${VERSION}"
  sha256 arm:   "${SHA_ARM}",
         intel: "${SHA_INTEL}"

  url "https://github.com/${REPO}/releases/download/v#{version}/SundayDeck-#{version}-macOS-#{arch}.dmg"
  name "SundayDeck"
  desc "Fast, offline-first church presentation studio with local AI and voice auto-follow"
  homepage "https://github.com/${REPO}"

  depends_on macos: ">= :ventura"

  app "SundayDeck.app"

  zap trash: [
    "~/.local/share/sundaydeck",
    "~/Library/Preferences/org.sundaydeck.app.plist",
    "~/Library/Saved Application State/org.sundaydeck.app.savedState",
  ]
end
CASK
