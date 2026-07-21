#!/usr/bin/env bash
# Compare Leibniz partial-sum timing: lkjscript2026 vs C.
# Honest: the interpreter is expected to lose until JIT exists.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="${LKJSCRIPT2026:-${LKJ:-$ROOT/target/release/lkjscript2026}}"
if [[ ! -x "$BIN" ]]; then
  BIN="${LKJ:-$ROOT/target/debug/lkjscript2026}"
fi
N="${N:-50000}"
CC="${CC:-cc}"
OUT="$ROOT/target/bench-leibniz-c"
mkdir -p "$ROOT/target"
"$CC" -O2 -o "$OUT" "$ROOT/meta/bench/c/leibniz.c"
TMP="$(mktemp --suffix=.lkjsxc)"
trap 'rm -f "$TMP"' EXIT
cat > "$TMP" <<EOF
<import>examples/bench/leibniz-loop.lkjsxc</import>
<do>
  <print><leibniz><${N}/></leibniz></print>
</do>
EOF
echo "N=$N"
TIMEFORMAT='%R sec'
echo -n "lkjscript2026: "
{ time "$BIN" run "$TMP" >/tmp/bench-lkjs.out; } 2>&1
echo -n "c:             "
{ time "$OUT" "$N" >/tmp/bench-c.out; } 2>&1
echo "lkjscript2026 result: $(cat /tmp/bench-lkjs.out)"
echo "c result:             $(cat /tmp/bench-c.out)"
printf 'bench-compare done (C expected faster until JIT)\n'
