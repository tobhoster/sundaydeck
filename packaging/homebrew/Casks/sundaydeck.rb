cask "sundaydeck" do
  arch arm: "arm64", intel: "x86_64"

  version "0.1.0"
  sha256 arm:   "REPLACE_WITH_ARM64_DMG_SHA256_FROM_RELEASE",
         intel: "REPLACE_WITH_X86_64_DMG_SHA256_FROM_RELEASE"

  url "https://github.com/tobhoster/sundaydeck/releases/download/v#{version}/SundayDeck-#{version}-macOS-#{arch}.dmg"
  name "SundayDeck"
  desc "Calm, free, offline church presentation software for Sunday volunteers"
  homepage "https://github.com/tobhoster/sundaydeck"

  depends_on macos: ">= :ventura"

  app "SundayDeck.app"

  zap trash: [
    "~/.local/share/sundaydeck",
    "~/Library/Preferences/org.sundaydeck.app.plist",
    "~/Library/Saved Application State/org.sundaydeck.app.savedState",
  ]
end
