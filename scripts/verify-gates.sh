#!/usr/bin/env bash
set -euo pipefail

# SundayDeck Static, Performance & Stress Verification Gate
# Usage:
#   ./scripts/verify-gates.sh [--static-only | --release-perf | --macos-stress | --all]
#
# Environment:
#   PERF_METRICS_OUT   Optional path to save extracted "PERF key=val" lines.

MODE="${1:---all}"

detect_lavapipe_icd() {
  if [[ -n "${VK_ICD_FILENAMES:-}" && -f "${VK_ICD_FILENAMES}" ]]; then
    return 0
  fi
  local candidate
  for candidate in \
    /usr/share/vulkan/icd.d/lvp_icd*.json \
    /home/linuxbrew/.linuxbrew/share/vulkan/icd.d/lvp_icd*.json; do
    if [[ -f "$candidate" ]]; then
      export VK_ICD_FILENAMES="$candidate"
      export VK_DRIVER_FILES="$candidate"
      echo "Detected Lavapipe ICD: $candidate"
      return 0
    fi
  done
  return 0
}

run_static_gates() {
  echo "=== Static Gate 1: Rust file line budget (<= 500 lines) ==="
  local over
  over="$(find crates -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 500')"
  if [[ -n "$over" ]]; then
    echo "ERROR: Rust file(s) exceed the 500-line limit:"
    echo "$over"
    exit 1
  fi
  echo "OK: all .rs files under crates/ are <= 500 lines."

  echo "=== Static Gate 2: Zero runtime unwrap/expect/panic/unreachable + perf footguns ==="
  python3 - <<'PY'
import pathlib
import re
import sys

panic_pat = re.compile(r'\.unwrap\(\)|\.expect\(|panic!|unreachable!|todo!|unimplemented!')
runtime_hits = []
footgun_hits = []

for path in sorted(pathlib.Path("crates").glob("*/src/**/*.rs")):
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    test_start = next((idx for idx, line in enumerate(lines) if "#[cfg(test)]" in line), len(lines))
    runtime_lines = lines[:test_start]

    for lineno, line in enumerate(runtime_lines, 1):
        stripped = line.lstrip()
        if stripped.startswith("//"):
            continue
        if panic_pat.search(line):
            runtime_hits.append(f"{path}:{lineno}: {stripped}")

    if str(path).startswith("crates/sundaydeck-ui/src/"):
        runtime_src = "\n".join(runtime_lines)
        for lineno, line in enumerate(runtime_lines, 1):
            stripped = line.lstrip()
            if stripped.startswith("//"):
                continue
            if "all_font_names(" in line:
                footgun_hits.append(
                    f"{path}:{lineno}: banned all_font_names() — use TextSystem::has_font_family()"
                )
        for m in re.finditer(r'\.repeat\(\)', runtime_src):
            start = max(0, m.start() - 250)
            end = min(len(runtime_src), m.end() + 250)
            window = runtime_src[start:end]
            if "Animation::" in window and "with_max_fps(" not in window:
                lineno = runtime_src[:m.start()].count("\n") + 1
                footgun_hits.append(
                    f"{path}:{lineno}: repeating Animation missing .with_max_fps() throttle"
                )

if runtime_hits:
    print("ERROR: Runtime panic/unwrap macro(s) found outside #[cfg(test)]:")
    for hit in runtime_hits:
        print(f"  {hit}")
    sys.exit(1)

if footgun_hits:
    print("ERROR: Performance footgun(s) found in crates/sundaydeck-ui/src:")
    for hit in footgun_hits:
        print(f"  {hit}")
    sys.exit(1)

print("OK: 0 runtime panic/unwrap hits and 0 UI perf footguns.")
PY
}

append_perf_summary() {
  local metrics_file="$1"
  local title="$2"
  echo "=== Extracted Performance Metrics ($title) ==="
  cat "$metrics_file"
  if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
    {
      echo "### $title"
      echo ""
      echo "| Metric | Value |"
      echo "|---|---:|"
      while IFS='=' read -r key val; do
        [[ -z "$key" ]] && continue
        echo "| \`$key\` | \`$val\` |"
      done < <(sed 's/^PERF //' "$metrics_file" | sort -u)
      echo ""
    } >> "$GITHUB_STEP_SUMMARY"
  fi
}

run_release_perf_linux() {
  detect_lavapipe_icd
  local tmp_log
  tmp_log="$(mktemp)"
  trap 'rm -f "$tmp_log"' RETURN

  echo "=== Release Perf & Stress Suite (sundaydeck-ui) ==="
  cargo test --release -p sundaydeck-ui \
    --test perf_regression \
    --test media_pipeline_tests \
    --test store_import_tests \
    --test window_lifecycle_tests \
    --test text_input_tests \
    --test ux_interaction_tests \
    -- --nocapture --test-threads=1 | tee "$tmp_log"

  echo "=== Release Stress Suite (overlay, core zip/office bombs, db FTS5 & 31k Bible search) ==="
  cargo test --release -p sundaydeck-overlay --test overlay_server_tests -- --test-threads=1
  cargo test --release -p sundaydeck-core --test office_budget_tests --test sdeck_oversize_tests --test zip_limits_tests -- --test-threads=1
  cargo test --release -p sundaydeck-db --test db_integration --test service_tolerance_tests --test production_ready_tests --test bible_search_perf -- --nocapture --test-threads=1 | tee -a "$tmp_log"

  local metrics_file="${PERF_METRICS_OUT:-$(mktemp)}"
  grep -oE 'PERF [A-Za-z0-9_]+=[^[:space:]]+' "$tmp_log" | sort -u > "$metrics_file" || true
  if [[ -s "$metrics_file" ]]; then
    append_perf_summary "$metrics_file" "Release Performance & Stress Metrics (Linux)"
  fi
  if [[ -z "${PERF_METRICS_OUT:-}" ]]; then
    rm -f "$metrics_file"
  fi
}

run_macos_stress() {
  echo "=== Apple Silicon Clippy (all workspace targets including Metal/Cocoa/CoreAudio) ==="
  cargo clippy --workspace --all-targets -- -D warnings

  echo "=== Apple Silicon Release Stress Tests (core, db, overlay, planner, voice, ui non-Lavapipe unit tests) ==="
  cargo test --release \
    -p sundaydeck-core \
    -p sundaydeck-db \
    -p sundaydeck-overlay \
    -p sundaydeck-planner \
    -p sundaydeck-voice \
    -- --test-threads=1
  cargo test --release -p sundaydeck-ui --lib -- --skip audience_window --test-threads=1
}

case "$MODE" in
  --static-only)
    run_static_gates
    ;;
  --release-perf)
    run_release_perf_linux
    ;;
  --macos-stress)
    run_static_gates
    run_macos_stress
    ;;
  --all)
    run_static_gates
    run_release_perf_linux
    ;;
  *)
    echo "Unknown mode: $MODE" >&2
    echo "Usage: $0 [--static-only | --release-perf | --macos-stress | --all]" >&2
    exit 2
    ;;
esac
