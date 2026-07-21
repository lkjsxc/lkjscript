#!/usr/bin/env bash
# Scripted editor acceptance: open → insert → save → reopen; missing path creates.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
LKJ="${LKJ:-$ROOT/target/debug/lkjscript2026}"
if [[ ! -x "$LKJ" ]]; then
  LKJ="$(command -v lkjscript2026)"
fi
TMP="$(mktemp)"
NEW="$(mktemp -u)"
trap 'rm -f "$TMP" "$NEW"' EXIT
printf 'seed\n' > "$TMP"
printf 'iHELLO \x1b:wq\n' | "$LKJ" run "$ROOT/examples/texteditor/main.lkjscript" "$TMP" >/dev/null
grep -q 'HELLO seed' "$TMP"
printf 'q' | "$LKJ" run "$ROOT/examples/texteditor/main.lkjscript" "$TMP" >/dev/null
grep -q 'HELLO seed' "$TMP"
# missing path → new file on :wq
rm -f "$NEW"
printf 'ihi\x1b:wq\n' | "$LKJ" run "$ROOT/examples/texteditor/main.lkjscript" "$NEW" >/dev/null
test -f "$NEW"
grep -q 'hi' "$NEW"
# CR+LF: two content lines then quit; ensure no error
printf 'a\nb\n' > "$TMP"
printf 'q' | "$LKJ" run "$ROOT/examples/texteditor/main.lkjscript" "$TMP" >/tmp/ed-crlf.out
# staircase bug emitted only LF; after fix, redraw should contain CR before LF
grep -q $'\r' /tmp/ed-crlf.out
# cmdline paint: :wq must appear in the redraw stream
printf 'iZ\x1b:wq\n' | "$LKJ" run "$ROOT/examples/texteditor/main.lkjscript" "$TMP" >/tmp/ed-cmd.out
grep -q ':wq' /tmp/ed-cmd.out
grep -q 'Z' "$TMP"
printf 'editor-smoke ok\n'
