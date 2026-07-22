#!/usr/bin/env bash
# Scripted editor acceptance: open -> insert -> save -> reopen; missing path creates.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
LKJ="${LKJ:-$ROOT/target/debug/lkjscript2026}"
if [[ ! -x "$LKJ" ]]; then
  LKJ="$(command -v lkjscript2026)"
fi
TMP_DIR="$(mktemp -d)"
TMP="$TMP_DIR/existing.txt"
NEW="$TMP_DIR/new.txt"
trap 'rm -rf "$TMP_DIR"' EXIT
printf 'seed\n' > "$TMP"
printf 'iHELLO \x1b:wq\n' | "$LKJ" run "$ROOT/src/examples/lkjedit/main.lkjml" "$TMP" >/dev/null
grep -q 'HELLO seed' "$TMP"
printf 'q' | "$LKJ" run "$ROOT/src/examples/lkjedit/main.lkjml" "$TMP" >/dev/null
grep -q 'HELLO seed' "$TMP"
# Missing path -> new file on :wq.
printf 'ihi\x1b:wq\n' | "$LKJ" run "$ROOT/src/examples/lkjedit/main.lkjml" "$NEW" >/dev/null
test -f "$NEW"
grep -q 'hi' "$NEW"
# CR+LF: two content lines then quit; ensure redraw has CR before LF.
printf 'a\nb\n' > "$TMP"
printf 'q' | "$LKJ" run "$ROOT/src/examples/lkjedit/main.lkjml" "$TMP" >"$TMP_DIR/crlf.out"
grep -q $'\r' "$TMP_DIR/crlf.out"
# Command-line paint: :wq must appear in the redraw stream.
printf 'iZ\x1b:wq\n' | "$LKJ" run "$ROOT/src/examples/lkjedit/main.lkjml" "$TMP" >"$TMP_DIR/cmd.out"
grep -q ':wq' "$TMP_DIR/cmd.out"
grep -q 'Z' "$TMP"
printf 'lkjedit-smoke ok\n'
