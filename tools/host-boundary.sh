#!/usr/bin/env bash
# These process budgets test host refusal; they never reject source programs.
set -euo pipefail
cd "$(dirname "$0")/.."
binary=$(realpath "${1:-target/release/lkjscript}")
work=target/host-boundary
mkdir -p "$work"
"$binary" build examples/hello.rs -o "$work/hello.elf"
prlimit --as=8589934592 "$binary" build examples/hello.rs -o "$work/hello-limited.elf"
cmp "$work/hello.elf" "$work/hello-limited.elf"
"$binary" k examples/hello.rs > "$work/hello.k"
"$binary" build "$work/hello.k" -o "$work/hello-k.elf"
cmp "$work/hello.elf" "$work/hello-k.elf"
trap_case() {
    local name=$1 code=$2 line=$3
    local source="tests/fixtures/runtime_${name}.rs"
    "$binary" build "$source" --entry probe -o "$work/$name.elf"
    local status=0
    timeout -s KILL 5 prlimit --as=67108864 "$work/$name.elf" \
        > "$work/$name.stdout" 2> "$work/$name.stderr" || status=$?
    [[ "$status" == 1 ]]
    [[ ! -s "$work/$name.stdout" ]]
    rg -q "^$code at line $line( |$)" "$work/$name.stderr"
    status=0
    timeout -s KILL 5 "$binary" run "$source" --entry probe \
        > "$work/$name-run.stdout" 2> "$work/$name-run.stderr" || status=$?
    [[ "$status" == 1 ]]
    rg -q ":$line: $code " "$work/$name-run.stderr"
}
trap_case stack_exhaustion R0008 2
trap_case heap_exhaustion R0006 4
trap_case bounds_trap R0003 3
for name in stack_exhaustion heap_exhaustion; do
    status=0
    timeout -s KILL 5 "$binary" run "tests/fixtures/runtime_$name.rs" --entry probe --engine k \
        > "$work/$name-k.stdout" 2> "$work/$name-k.stderr" || status=$?
    [[ "$status" == 1 ]]
    line=2
    code=R0008
    if [[ "$name" == heap_exhaustion ]]; then line=4; code=R0006; fi
    rg -q ":$line: $code " "$work/$name-k.stderr"
done
printf 'Host boundary: deterministic ELF; K identity; located native and K exhaustion under Linux budgets\n'
