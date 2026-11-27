# Contributing to SundayDeck

Thank you for caring about free, accessible church presentation software. SundayDeck is built for real Sunday volunteers — and **you don't need to be a programmer to make it dramatically better.**

---

## Four Ways to Help

### 1. Volunteer Feedback (No Coding Needed)
The highest-value contribution is running SundayDeck during a rehearsal or service and telling us what felt unclear ten minutes before worship started.
- Open a [**Volunteer Confusion Report**](.github/ISSUE_TEMPLATE/volunteer-confusion.md) whenever a button label, shortcut, or modal made you pause.
- Let us know what church laptop, projector resolution, and display adapter you tested on.

### 2. Translations (Español, Português, Français & More)
Churches across North and South America rely on clear labels in their heart language.
- Help refine Spanish (`ES`), Portuguese (`PT`), or French (`FR`) copy in `docs/i18n.js` and localized Bible book aliases in `crates/sundaydeck-core/src/bible_aliases.rs`.

### 3. Verified Public-Domain Starter Packs (`.sdeck-pack`)
We strictly respect copyright and **never** accept copyrighted contemporary worship songs or copyrighted Bible translations into this repository.
- You are very welcome to contribute verified **Public Domain / CC0 / CC-BY** hymns, classic liturgical responses, or build-script definitions in `scripts/build-starter-packs.py` with explicit license and attribution metadata.

### 4. Code & Importers
1. Keep every file focused and easy to read (every Rust source file stays under 500 lines).
2. Keep the interface calm, warm, and free of clutter or jargon.
3. Run the local verification checks before opening a pull request:
   ```bash
   cargo fmt --check
   cargo clippy --workspace --all-targets -- -D warnings
   ./scripts/verify-gates.sh --all
   ```

---

## Maintainer Notes: Releases & Custom Domain

- **Publishing a release:** Push a semantic tag (`git tag v0.0.3 && git push origin v0.0.3`) to run `.github/workflows/release.yml`, which gates on static + release perf/stress checks and builds macOS (`.dmg`), Windows (`.exe` + `.zip`), and Linux (`.deb` + `.tar.gz`) installers, `.sdeck-pack` starter packs, `SHA256SUMS.txt`, SLSA provenance, and an updated Homebrew Cask (`packaging/homebrew/Casks/sundaydeck.rb`).
- **Custom Domain on GitHub Pages:** Enable **Settings → Pages → Source: GitHub Actions**, add your domain in `docs/CNAME` (see `docs/CNAME.example`), and configure DNS `A` / `CNAME` records.

---

## Core Design Principles
- **Volunteer-first plain English:** Labels should make immediate sense to someone stepping into the booth ten minutes before service.
- **100% offline & private:** Never require accounts, cloud logins, or internet access on Sunday morning.
- **Free forever:** Never gate features behind licenses, paywalls, or watermarks.
- **Respect copyright:** Bundle only verified Public Domain / open-licensed content; give churches clean local importers for their own licensed libraries.

We're grateful for every issue, translation, pull request, and word shared with another church.
