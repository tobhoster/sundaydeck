<div align="center">

# SundayDeck

**Calm, free, offline church presentation software built for real Sunday volunteers.**

*Zero subscriptions. Zero internet required. Instant startup on Mac, Windows, and Linux — with songs, Protestant & Catholic Bibles, numbered hymnbooks, built-in slide layouts, stage monitor, livestream lower-thirds, and hands-free microphone auto-follow.*

[![Platforms](https://img.shields.io/badge/Platforms-macOS%20%7C%20Windows%20%7C%20Linux-1a1c20?style=flat-square)](.github/workflows/release.yml)
[![License](https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-1a1c20?style=flat-square)](LICENSE-MIT)
[![Offline First](https://img.shields.io/badge/Works-100%25%20Offline-38b780?style=flat-square)](#why-sundaydeck)

<br />

<img src="docs/media/hero-workspace.png" alt="SundayDeck Operator Workspace — Service Order, 16:9 Slide Grid, and Live Sanctuary Preview" width="100%" />

</div>

---

## Product Film & Walkthrough

Watch the 45-second narrated overview (`docs/media/sundaydeck-promo-ad.mp4`) or preview the live workflow below:

<div align="center">
  <img src="docs/media/walkthrough.gif" alt="SundayDeck live Sunday morning workflow walkthrough" width="100%" />
  <br />
  <sub>
    <strong>Narrated launch video:</strong> <a href="docs/media/sundaydeck-promo-ad.mp4"><code>docs/media/sundaydeck-promo-ad.mp4</code></a> ·
    <strong>Silent HD walkthrough:</strong> <a href="docs/media/sundaydeck-walkthrough.mp4"><code>docs/media/sundaydeck-walkthrough.mp4</code></a> ·
    <strong>Website:</strong> <a href="docs/index.html"><code>docs/index.html</code></a>
  </sub>
</div>

---

## Why SundayDeck?

Most church presentation apps are either expensive subscriptions, sluggish on older church laptops, or cluttered with broadcast jargon that intimidates a volunteer stepping into the booth ten minutes before service.

SundayDeck is designed around quiet, predictable Sunday morning craft:

- **Calm, volunteer-first workspace** — Warm dark surfaces, plain-English controls (`Show on Screen`, `Hide Words`, `Black Screen`, `Show Logo`), strict Preview vs. Live separation, and live **Quick Edit** to fix a typo without disturbing the sanctuary screen.
- **Built-in Custom Slide Layouts & Gentle Transitions** — Create welcome cards, key sermon quotes, click-to-reveal bullet points, image + text announcements, pre-service countdowns, and call-and-response liturgy right inside the app.
- **Pastor Notes to Service Order** — Drop a pastor's PowerPoint (`.pptx`), Word document (`.docx`), or plain sermon outline to build a reviewable service order in seconds — matching songs, pulling Scriptures offline, and laying out sermon points.
- **Sanctuary, Stage & Livestream Ready** — Independent Audience Projector and Stage Confidence Monitor windows (with current + next slide, clock, speaker timers, and private stage cues), plus a built-in local **OBS Studio** transparent lower-third overlay and chroma-key theme.
- **Hands-Free Microphone Auto-Follow** — Listens locally to your room mic or soundboard to follow sung lyrics in safe Suggest/Auto modes and queues spoken sermon Bible verses for one click — completely offline.
- **Built for Churches Across the Americas** — Full **66-book Protestant & 73-book Catholic Bibles**, accent-tolerant search (English, Spanish, Portuguese, French), **parallel dual-translation Scripture**, **numbered hymnbooks** (`BH 330`, `UMH 378`, `HC 15`, `#42`), **bilingual stacked lyrics**, and portable **`.sdeck` / `.sdeck-pack`** offline bundles.
- **Easy Library Migration** — Import existing libraries from EasyWorship, OpenLP, ProPresenter, VideoPsalm, FreeShow, ChordPro, OpenLyrics, SongSelect, and standard USFM/USFX/OSIS/Zefania/SQLite Bibles.

---

## Download & Install

### macOS (Homebrew — Apple Silicon & Intel)

> Replace `OWNER` with your GitHub username or organization once published (or host [`packaging/homebrew/Casks/sundaydeck.rb`](packaging/homebrew/Casks/sundaydeck.rb) in a `homebrew-sundaydeck` tap repository).

```bash
brew tap OWNER/sundaydeck
brew install --cask sundaydeck
```

Or download `SundayDeck-<version>-macOS-arm64.dmg` (Apple Silicon M1–M4) or `SundayDeck-<version>-macOS-x86_64.dmg` (Intel) from **GitHub Releases**, open the disk image, and drag **SundayDeck** into **Applications**.

### Windows (64-bit)

Download from **GitHub Releases**:
- **Installer (recommended):** `SundayDeck-<version>-windows-x86_64-setup.exe` (adds Start Menu shortcut and `.sdeck` bundle support)
- **Portable ZIP:** `SundayDeck-<version>-windows-x86_64.zip` (unzip and run `SundayDeck.exe` with zero installation)

### Linux

Download from **GitHub Releases**:
- **Debian / Ubuntu:**
  ```bash
  sudo apt install ./sundaydeck_<version>_amd64.deb
  ```
- **Portable Archive (Fedora, Arch, openSUSE, etc.):**
  ```bash
  tar -xzf SundayDeck-<version>-linux-x86_64.tar.gz
  cd SundayDeck-<version>-linux-x86_64
  ./sundaydeck
  ```

---

## 5-Minute Volunteer Guide

### 1. Plan Your Service — or Drop the Pastor's PowerPoint / Word File

Click the **Upload** button in the **Service Order** rail to drop a `.pptx`, `.docx`, or plain outline. SundayDeck matches your songs, fetches Scriptures offline, formats sermon points, and lets you review every slide before adding it to your Sunday schedule.

| Plan Importer | Built-In Slide Layouts |
| :---: | :---: |
| <img src="docs/media/plan-importer.png" alt="SundayDeck Plan Importer modal" width="100%" /> | <img src="docs/media/custom-slide-builder.png" alt="SundayDeck Custom Slide Builder modal" width="100%" /> |

### 2. Choose Peaceful Public Domain Backgrounds & Connect Your Displays

Pick bundled **Public Domain** sanctuary photography and National Park Service motion timelapse loops, paste any online photo (including Unsplash) or `.mp4` / `.webm` video URL, load local worship loops with custom contrast scrims, send clean output to your projector and **Stage Confidence Monitor**, or toggle the built-in **OBS Livestream Overlay** (`http://127.0.0.1:8787/overlay`) directly from the **Projector** menu.

| Sanctuary Photography (Public Domain) | Stage Confidence Monitor |
| :---: | :---: |
| <img src="docs/media/sanctuary-mountains.png" alt="Public Domain sanctuary mountain photography background" width="100%" /> | <img src="docs/media/stage-monitor.png" alt="Stage Confidence Monitor" width="100%" /> |

### 3. Protestant, Catholic & Bilingual Churches Across the Americas

Open **Settings → Bibles & Packs** to choose your **Church Tradition** (`Contemporary`, `Baptist & Evangelical`, `Methodist / Mainline`, `Catholic Parish`, `Spanish / Portuguese`), toggle **Bilingual Stacked / Primary / Secondary** lyrics, import 66- or 73-book offline Bibles, or install verified Public Domain `.sdeck-pack` starter packs.

> **Copyright Respect:** SundayDeck never bundles copyrighted contemporary worship songs or copyrighted Bible translations. Verified Public Domain starter packs can be generated reproducibly with `python3 scripts/build-starter-packs.py`, and churches can import their own legally licensed hymnbooks and Bibles in seconds.

| Bilingual Call & Response | Church Tradition & Offline Packs |
| :---: | :---: |
| <img src="docs/media/liturgy-call-response.png" alt="Bilingual Catholic Order of Mass Call and Response slide" width="100%" /> | <img src="docs/media/global-church-packs.png" alt="Church Tradition, Bilingual Lyrics, and Offline Packs Settings" width="100%" /> |

### 4. Hands-Free Microphone Auto-Follow

Click **Auto-Follow** in the top bar, choose your microphone or soundboard input, and select **Suggest next slide** (highlights the next slide with a warm amber ring) or **Advance automatically**. During the sermon, spoken Bible references appear cleanly in a **1-Click Scripture Queue** on the right without ever hijacking the live screen.

<div align="center">
  <img src="docs/media/voice-autofollow.png" alt="Auto-Follow popover with live listening meter and next slide suggestion" width="90%" />
</div>

---

## Keyboard Shortcuts

| Action | Shortcut |
| :--- | :--- |
| Next live slide (or reveal next bullet point) | `Space` / `Right` / `Down` |
| Previous live slide | `Left` / `Up` |
| Show staged preview on screen | `Cmd/Ctrl + Enter` |
| Hide Words (keep background) | `F1` (`Shift + F1` instant cut) |
| Black Screen | `F2` (`Shift + F2` instant cut) |
| Show Church Logo | `F3` (`Shift + F3` instant cut) |
| Fix a typo live (Quick Edit) | `Cmd/Ctrl + E` |
| Toggle Audience Projector window | `Cmd/Ctrl + Shift + A` |
| Toggle Stage Confidence Monitor | `Cmd/Ctrl + Shift + S` |

---

## Church Software Should Be Free — Open to Contributions

We believe church presentation software should be **free, simple, and accessible to every congregation** — from a church plant meeting in a school cafeteria to a full sanctuary with a volunteer media team.

- **Completely free — no seat licenses, no activation keys, no annual renewals, no paywalls, and no watermarks.** Install SundayDeck on as many church computers and volunteer laptops as you need.
- **Open to contributions:** Read [CONTRIBUTING.md](CONTRIBUTING.md) and our [Code of Conduct](CODE_OF_CONDUCT.md) — pull requests, volunteer feedback, Spanish / Portuguese / French translations, and verified public-domain starter packs are all warmly welcome.
- **Helpful Resources for Your Booth:** [1-Page Printable Sunday Volunteer Cheat Sheet](docs/cheat-sheet.html) · [Public Domain Starter Packs & Migration Guide](docs/packs.html).

### Support the Movement (Optional)

SundayDeck will always remain 100% free with zero locked features. If it blesses your church on Sunday mornings and you'd like to help cover Apple & Windows code-signing certificates, hosting, and ongoing maintenance:

- **[Buy Me a Coffee](https://buymeacoffee.com/OWNER)** — one-time coffee or encouragement
- **[GitHub Sponsors](https://github.com/sponsors/OWNER)** — zero-fee monthly or one-time church & community sponsorship
- **[Ko-fi](https://ko-fi.com/OWNER)** — zero-platform-fee tip jar alternative
- **Or simply share SundayDeck** with a neighboring church or volunteer team!

> Replace `OWNER` in [`README.md`](README.md), [`docs/index.html`](docs/index.html), and [`.github/FUNDING.yml`](.github/FUNDING.yml) with your handle before publishing.

---

## Publishing Releases, Homebrew & Custom Domain

SundayDeck includes automated release and website publishing workflows:

- **Tag a release (`git tag v0.1.0 && git push origin main --tags`)** to build macOS (`.dmg`), Windows (`.exe` + `.zip`), and Linux (`.deb` + `.tar.gz`) installers along with `SHA256SUMS.txt` and an updated Homebrew Cask formula (`packaging/homebrew/Casks/sundaydeck.rb`).
- **Custom Domain on GitHub Pages:** Enable **Settings → Pages → Source: GitHub Actions**, add your domain in `docs/CNAME` (see [`docs/CNAME.example`](docs/CNAME.example)), and point your DNS `A` / `CNAME` records to GitHub Pages.

<details>
<summary><strong>For Contributors: Building from Source</strong></summary>

```bash
git clone https://github.com/OWNER/sundaydeck.git
cd sundaydeck
cargo run --release -p sundaydeck-ui --bin sundaydeck

# Build a local macOS .dmg installer on macOS:
./scripts/build-macos-dmg.sh

# Build verified Public Domain starter packs (.sdeck-pack):
python3 scripts/build-starter-packs.py
```

</details>

---

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option. All bundled photographs, motion loops, fonts, icons, and starter packs are verified Public Domain / open-licensed — see [ASSETS_LICENSE.md](ASSETS_LICENSE.md).
