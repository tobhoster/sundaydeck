# Security Policy

SundayDeck is designed to run **100% offline** in church media booths with zero accounts, zero analytics, and zero background cloud uploads.

## Supported Versions

Security fixes are released on the `main` branch and published in the latest tagged GitHub Release.

| Version | Supported |
| :--- | :--- |
| Latest release (`v0.0.2`+) | Yes |
| Older snapshots | Please upgrade to the latest release |

## Reporting a Vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities.

Instead, report vulnerabilities privately through **GitHub Security Advisories**:
- https://github.com/tobhoster/sundaydeck/security/advisories/new

We aim to acknowledge reports within 72 hours and coordinate a fix before public disclosure.

## Built-In Security & Privacy Guarantees

1. **Offline-First & Zero Telemetry:** Songs, Bibles, service orders, and settings are stored strictly locally in SQLite. Optional microphone Auto-Follow processes audio locally and never uploads room audio to external servers.
2. **Safe Archive Extraction (`.sdeck`, `.sdeck-pack`, `.pptx`, `.docx`):** All ZIP-backed bundles validate every entry path against path traversal (`..`, absolute paths, Windows drive prefixes) and enforce strict decompression size limits.
3. **Local-Only OBS Overlay Server:** The optional OBS lower-third WebSocket/HTTP server binds exclusively to loopback (`127.0.0.1:8787`), never exposing your booth laptop to the public internet.
4. **Verifiable Release Artifacts:**
   - Every release publishes a `SHA256SUMS.txt` checksum manifest verified automatically by `install.sh` and `install.ps1`.
   - Official GitHub Actions releases attach **GitHub Artifact Build Provenance Attestations** (SLSA / Sigstore), verifiable with the GitHub CLI:
     ```bash
     gh attestation verify SundayDeck-<version>-macOS-arm64.dmg --repo tobhoster/sundaydeck
     ```
5. **Automated Public Repository & Supply-Chain Protection:**
   - Every push, pull request, and weekly schedule runs **`cargo-audit`** (RustSec CVEs), **`cargo-deny`** (license allowlist + crates.io-only source trust), **ShellCheck** on installer/packaging scripts, and **CSP / JSON-LD verification** on the website (`.github/workflows/security.yml`).
   - Pull requests additionally run GitHub **Dependency Review**, require **`CODEOWNERS`** approval, and are guarded by active GitHub **Branch (`main`)** and **Tag (`v*`)** rulesets alongside Dependabot Alerts, Secret Scanning, and Push Protection.
