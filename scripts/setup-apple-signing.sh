#!/usr/bin/env bash
set -euo pipefail

# Configure Apple Developer ID signing & notarization secrets on GitHub Actions
# without ever committing certificates or credentials into git.
#
# Usage:
#   ./scripts/setup-apple-signing.sh --check
#   ./scripts/setup-apple-signing.sh --p12 /path/to/DeveloperID.p12 --identity "Developer ID Application: Name (TEAMID)" \
#       [--api-key /path/to/AuthKey_XXXX.p8 --key-id XXXX --issuer-id UUID] \
#       [--apple-id you@example.com --team-id TEAMID]

REPO="${GITHUB_REPO:-tobhoster/sundaydeck}"
MODE="configure"
P12_PATH=""
SIGNING_IDENTITY=""
API_KEY_PATH=""
API_KEY_ID=""
API_ISSUER_ID=""
APPLE_ID_VAL=""
TEAM_ID_VAL=""

usage() {
  cat <<'EOF'
Configure Apple Developer ID signing & notarization GitHub Actions secrets for SundayDeck.

Commands:
  --check                         List which Apple signing/notarization secrets are set on GitHub.
  --list-local                    On macOS, show installed "Developer ID Application" identities.

Configure (pass one notarization method — App Store Connect API key recommended):
  --repo OWNER/REPO               Target GitHub repo (default: tobhoster/sundaydeck)
  --p12 FILE                      Exported Developer ID Application .p12 certificate
  --identity "Developer ID ..."   Exact signing identity common name
  --api-key FILE                  App Store Connect AuthKey_XXXX.p8 file (recommended)
  --key-id ID                     App Store Connect Key ID (10 characters)
  --issuer-id UUID                App Store Connect Issuer ID (UUID)
  --apple-id EMAIL                Alternative: Apple ID email (prompts for app-specific password)
  --team-id TEAMID                Alternative: 10-character Apple Team ID
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check) MODE="check"; shift ;;
    --list-local) MODE="list-local"; shift ;;
    --repo) REPO="$2"; shift 2 ;;
    --p12) P12_PATH="$2"; shift 2 ;;
    --identity) SIGNING_IDENTITY="$2"; shift 2 ;;
    --api-key) API_KEY_PATH="$2"; shift 2 ;;
    --key-id) API_KEY_ID="$2"; shift 2 ;;
    --issuer-id) API_ISSUER_ID="$2"; shift 2 ;;
    --apple-id) APPLE_ID_VAL="$2"; shift 2 ;;
    --team-id) TEAM_ID_VAL="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "error: unknown option '$1'" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ "$MODE" == "list-local" ]]; then
  if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "error: --list-local must run on macOS." >&2
    exit 1
  fi
  security find-identity -v -p codesigning
  exit 0
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "error: GitHub CLI (gh) is required. Install with: brew install gh && gh auth login" >&2
  exit 1
fi

if [[ "$MODE" == "check" ]]; then
  echo "Checking Apple Developer secrets on $REPO:"
  secrets="$(gh secret list --repo "$REPO" --json name -q '.[].name' || true)"
  for name in \
    APPLE_CERTIFICATE_P12 \
    APPLE_CERTIFICATE_PASSWORD \
    APPLE_SIGNING_IDENTITY \
    APPLE_API_KEY_P8 \
    APPLE_API_KEY_ID \
    APPLE_API_ISSUER_ID \
    APPLE_ID \
    APPLE_APP_PASSWORD \
    APPLE_TEAM_ID; do
    if printf '%s\n' "$secrets" | grep -qx "$name"; then
      printf '  [OK]      %s\n' "$name"
    else
      printf '  [missing] %s\n' "$name"
    fi
  done
  exit 0
fi

if [[ -z "$P12_PATH" || ! -f "$P12_PATH" ]]; then
  echo "error: --p12 must point to an existing exported Developer ID .p12 file." >&2
  exit 1
fi
if [[ -z "$SIGNING_IDENTITY" ]]; then
  echo "error: --identity is required (e.g. \"Developer ID Application: Your Name (TEAMID)\")." >&2
  exit 1
fi

read -r -s -p "Enter password used when exporting $P12_PATH: " P12_PASS
echo ""

echo "==> Uploading Developer ID certificate & identity secrets to $REPO..."
base64 < "$P12_PATH" | tr -d '\n' | gh secret set APPLE_CERTIFICATE_P12 --repo "$REPO"
printf '%s' "$P12_PASS" | gh secret set APPLE_CERTIFICATE_PASSWORD --repo "$REPO"
printf '%s' "$SIGNING_IDENTITY" | gh secret set APPLE_SIGNING_IDENTITY --repo "$REPO"

if [[ -n "$API_KEY_PATH" ]]; then
  if [[ ! -f "$API_KEY_PATH" || -z "$API_KEY_ID" || -z "$API_ISSUER_ID" ]]; then
    echo "error: --api-key requires an existing .p8 file plus --key-id and --issuer-id." >&2
    exit 1
  fi
  echo "==> Uploading App Store Connect API Key notarization secrets to $REPO..."
  base64 < "$API_KEY_PATH" | tr -d '\n' | gh secret set APPLE_API_KEY_P8 --repo "$REPO"
  printf '%s' "$API_KEY_ID" | gh secret set APPLE_API_KEY_ID --repo "$REPO"
  printf '%s' "$API_ISSUER_ID" | gh secret set APPLE_API_ISSUER_ID --repo "$REPO"
elif [[ -n "$APPLE_ID_VAL" ]]; then
  if [[ -z "$TEAM_ID_VAL" ]]; then
    echo "error: --apple-id also requires --team-id." >&2
    exit 1
  fi
  read -r -s -p "Enter Apple App-Specific Password (from appleid.apple.com): " APP_PASS
  echo ""
  echo "==> Uploading Apple ID notarization secrets to $REPO..."
  printf '%s' "$APPLE_ID_VAL" | gh secret set APPLE_ID --repo "$REPO"
  printf '%s' "$APP_PASS" | gh secret set APPLE_APP_PASSWORD --repo "$REPO"
  printf '%s' "$TEAM_ID_VAL" | gh secret set APPLE_TEAM_ID --repo "$REPO"
else
  echo "warning: no notarization method provided — DMGs will be signed but NOT notarized." >&2
fi

echo "Done! Next tagged release (e.g. v0.0.5) will build signed & notarized macOS DMGs automatically."
