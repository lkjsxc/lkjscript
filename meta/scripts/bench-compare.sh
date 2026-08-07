#!/usr/bin/env bash
# Compare Leibniz partial-sum timing: lkjscript vs C.
# This smoke reports timings but makes no performance claim or pass/fail comparison.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
BIN="${LKJSCRIPT_BIN:-$ROOT/target/release/lkjscript}"
if [[ ! -x "$BIN" ]]; then
  printf 'missing release binary: %s (run cargo build --release or set LKJSCRIPT_BIN)\n' "$BIN" >&2
  exit 1
fi
N=200000
CC="${CC:-cc}"
OUT="$ROOT/target/bench-leibniz-c"
mkdir -p "$ROOT/target"
"$CC" -O2 -o "$OUT" "$ROOT/meta/bench/c/leibniz.c"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
echo "N=$N"
TIMEFORMAT='%R sec'
echo -n "lkjscript: "
{ time "$BIN" run "$ROOT/src/examples/bench/main.lkjscript" >"$TMP_DIR/lkjs.out"; } 2>&1
echo -n "c:             "
{ time "$OUT" "$N" >"$TMP_DIR/c.out"; } 2>&1
echo "lkjscript result: $(<"$TMP_DIR/lkjs.out")"
echo "c result:             $(<"$TMP_DIR/c.out")"
printf 'bench-compare smoke ok\n'
