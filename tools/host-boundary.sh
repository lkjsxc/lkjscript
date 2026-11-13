#!/usr/bin/env bash
# These process budgets test host refusal; they never reject source programs.
set -Eeuo pipefail
cd "$(dirname "$0")/.."
binary=$(realpath "${1:-target/release/lkjscript}")
work=target/host-boundary
mkdir -p "$work"
report_failure() {
    local status=$1 line=$2 failed_command=$3 capture
    printf 'Host boundary failed at line %s (status %s): %s\n' \
        "$line" "$status" "$failed_command" >&2
    for capture in "$work"/*.stderr "$work"/*.stdout; do
        if [[ -s "$capture" ]]; then
            printf '%s (last 2048 bytes):\n' "$capture" >&2
            tail -c 2048 "$capture" >&2
            printf '\n' >&2
        fi
    done
    exit "$status"
}
trap 'report_failure "$?" "$LINENO" "$BASH_COMMAND"' ERR
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
    timeout -s KILL 5 prlimit --as=67108864 --stack=1048576 "$work/$name.elf" \
        > "$work/$name.stdout" 2> "$work/$name.stderr" || status=$?
    [[ "$status" == 1 ]]
    [[ ! -s "$work/$name.stdout" ]]
    grep -Eq "^$code at line $line( |$)" "$work/$name.stderr"
    status=0
    timeout -s KILL 5 env LKJ_RUN_MEMORY_BYTES=67108864 LKJ_RUN_STACK_BYTES=1048576 \
        "$binary" run "$source" --entry probe \
        > "$work/$name-run.stdout" 2> "$work/$name-run.stderr" || status=$?
    [[ "$status" == 1 ]]
    grep -Eq ":$line: $code " "$work/$name-run.stderr"
}
trap_case stack_exhaustion R0008 2
trap_case mutual_stack_exhaustion R0008 '(2|5)'
trap_case heap_exhaustion R0006 4
trap_case bounds_trap R0003 3
for name in stack_exhaustion mutual_stack_exhaustion heap_exhaustion; do
    status=0
    timeout -s KILL 5 env LKJ_RUN_MEMORY_BYTES=268435456 LKJ_RUN_STACK_BYTES=1048576 \
        "$binary" run "tests/fixtures/runtime_$name.rs" --entry probe --engine k \
        > "$work/$name-k.stdout" 2> "$work/$name-k.stderr" || status=$?
    [[ "$status" == 1 ]]
    line=2
    code=R0008
    if [[ "$name" == heap_exhaustion ]]; then line=4; code=R0006; fi
    if [[ "$name" == mutual_stack_exhaustion ]]; then line='(2|5)'; fi
    grep -Eq ":$line: $code " "$work/$name-k.stderr"
done
# No runner address-space quota is added by default. Requested stack bytes
# cannot override a tighter inherited soft limit, even with an unlimited hard
# limit. The K model receives its own extent without shrinking the Rust worker.
for engine in native k; do
    env -u LKJ_RUN_MEMORY_BYTES -u LKJ_RUN_STACK_BYTES \
        prlimit --as=2147483648 --stack=1048576:unlimited \
        "$binary" run tests/fixtures/runtime_resource_limits.rs --engine "$engine" \
        > "$work/inherited-$engine.stdout" 2> "$work/inherited-$engine.stderr"
    [[ ! -s "$work/inherited-$engine.stderr" ]]
    grep -Eq '^Max address space +2147483648 +2147483648 +bytes' "$work/inherited-$engine.stdout"
    grep -Eq '^Max stack size +1048576 +unlimited +bytes' "$work/inherited-$engine.stdout"
    env -u LKJ_RUN_MEMORY_BYTES LKJ_RUN_STACK_BYTES=67108864 \
        prlimit --as=2147483648 --stack=1048576:unlimited \
        "$binary" run tests/fixtures/runtime_resource_limits.rs --engine "$engine" \
        > "$work/clamped-$engine.stdout" 2> "$work/clamped-$engine.stderr"
    [[ ! -s "$work/clamped-$engine.stderr" ]]
    grep -Eq '^Max stack size +1048576 +unlimited +bytes' "$work/clamped-$engine.stdout"
done
env -u LKJ_RUN_MEMORY_BYTES LKJ_RUN_STACK_BYTES=65536 \
    prlimit --as=2147483648 --stack=1048576:unlimited \
    "$binary" run tests/fixtures/runtime_resource_limits.rs --engine k \
    > "$work/model-stack.stdout" 2> "$work/model-stack.stderr"
[[ ! -s "$work/model-stack.stderr" ]]
grep -Eq '^Max stack size +1048576 +unlimited +bytes' "$work/model-stack.stdout"
# Native-only batches capture guest streams in a Rust worker too. Infinite
# output must exhaust that explicit process envelope, never the compiler parent.
printf 'run\t\ttests/fixtures/runtime_output_exhaustion.rs\n' > "$work/output.tsv"
status=0
timeout -s KILL 5 env LKJ_RUN_MEMORY_BYTES=268435456 "$binary" batch "$work/output.tsv" --engine native \
    > "$work/output.stdout" 2> "$work/output.stderr" || status=$?
[[ "$status" == 1 ]]
[[ ! -s "$work/output.stdout" ]]
grep -Eq 'E_HOST execution worker terminated by signal [0-9]+ under Linux address-space budget 268435456 bytes' "$work/output.stderr"
printf 'Host boundary: deterministic ELF; K identity; located native/K exhaustion; inherited limits; separate model stack; bounded batch capture\n'
