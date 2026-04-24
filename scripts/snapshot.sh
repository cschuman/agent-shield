#!/usr/bin/env bash
#
# Snapshot regression harness for agent-shield Week 1 refactor.
#
# Usage:
#   scripts/snapshot.sh capture   # writes snapshots/<name>.json from current binary
#   scripts/snapshot.sh verify    # diffs current binary output against snapshots/
#
# The script runs the release binary against each directory in fixtures/ and
# either writes (capture) or diffs against (verify) the raw output stored at
# snapshots/<name>.json. Verify exits non-zero on any drift.
#
# Note: when no agents are found, the CLI emits plain text instead of JSON.
# This is pre-existing behavior preserved unchanged by Week 1 — the snapshot
# captures whatever the binary actually produces.
#
# Run `cargo build --release` before invoking. Each fixture is scanned with
# CWD set to the fixture root so file_path values in the JSON are repo-relative
# and stable across machines.

set -euo pipefail

MODE="${1:-}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURES_DIR="$ROOT/fixtures"
SNAPSHOTS_DIR="$ROOT/snapshots"
BINARY="$ROOT/target/release/agent-shield"

if [[ ! -x "$BINARY" ]]; then
  echo "error: $BINARY not found or not executable. Run \`cargo build --release\` first." >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required to strip the nondeterministic scan_date field." >&2
  exit 2
fi

# Run the scanner against a fixture and normalize output for diffing:
#   - strip the wall-clock `scan_date` field via jq
#   - canonicalize key ordering via `jq -S`
#   - fall through raw if the binary emits non-JSON (the "no agents detected"
#     plain-text output is part of the contract today)
run_scan() {
  local fixture="$1"
  local raw
  raw="$(cd "$fixture" && "$BINARY" scan . --format json)"
  if echo "$raw" | jq -e . >/dev/null 2>&1; then
    echo "$raw" | jq -S 'del(.scan_date)'
  else
    echo "$raw"
  fi
}

case "$MODE" in
  capture)
    mkdir -p "$SNAPSHOTS_DIR"
    for fixture in "$FIXTURES_DIR"/*/; do
      name="$(basename "$fixture")"
      echo "capturing $name"
      run_scan "$fixture" > "$SNAPSHOTS_DIR/$name.json"
    done
    echo "done. wrote $(ls "$SNAPSHOTS_DIR"/*.json | wc -l | tr -d ' ') snapshots."
    ;;
  verify)
    failed=0
    for fixture in "$FIXTURES_DIR"/*/; do
      name="$(basename "$fixture")"
      expected="$SNAPSHOTS_DIR/$name.json"
      if [[ ! -f "$expected" ]]; then
        echo "MISSING $name (no snapshot at $expected)" >&2
        failed=1
        continue
      fi
      if diff -u "$expected" <(run_scan "$fixture"); then
        echo "ok $name"
      else
        echo "DRIFT $name" >&2
        failed=1
      fi
    done
    exit "$failed"
    ;;
  *)
    echo "Usage: $0 capture|verify" >&2
    exit 2
    ;;
esac
