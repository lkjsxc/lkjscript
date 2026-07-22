#!/usr/bin/env bash
# Compare Leibniz partial-sum timing: lkjscript vs C.
# Honest: the interpreter is expected to lose until JIT exists.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
BIN="${LKJSCRIPT:-${LKJ:-$ROOT/target/release/lkjscript}}"
if [[ ! -x "$BIN" ]]; then
  BIN="${LKJ:-$ROOT/target/debug/lkjscript}"
fi
N="${N:-50000}"
CC="${CC:-cc}"
OUT="$ROOT/target/bench-leibniz-c"
mkdir -p "$ROOT/target"
"$CC" -O2 -o "$OUT" "$ROOT/meta/bench/c/leibniz.c"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
TMP="$TMP_DIR/bench.lkjml"
cat > "$TMP" <<EOF
import/
examples/bench/leibniz-loop.lkjml
/import
do/
print/
str-from-f64/
leibniz/
${N}
/leibniz
/str-from-f64
/print
/do
EOF
echo "N=$N"
TIMEFORMAT='%R sec'
echo -n "lkjscript: "
{ time "$BIN" run "$TMP" >"$TMP_DIR/lkjs.out"; } 2>&1
echo -n "c:             "
{ time "$OUT" "$N" >"$TMP_DIR/c.out"; } 2>&1
echo "lkjscript result: $(<"$TMP_DIR/lkjs.out")"
echo "c result:             $(<"$TMP_DIR/c.out")"
printf 'bench-compare done (C expected faster until JIT)\n'
