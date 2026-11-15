#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
resource_directory=$(mktemp -d target/resource-test.XXXXXX)
trap 'rm -rf "$resource_directory"' EXIT HUP INT TERM
"$compiler" build examples/hello.lkj -o "$resource_directory/hello"
(ulimit -v 4194304; exec "$resource_directory/hello") > "$resource_directory/hello.out"
"$compiler" build tests/fixtures/runtime_pressure.lkj -o "$resource_directory/pressure"
(ulimit -v 32768; exec "$resource_directory/pressure") > "$resource_directory/pressure.out"
grep -q '^ok pressure$' "$resource_directory/pressure.out"
"$compiler" build tests/fixtures/runtime_heap_exhaustion.lkj --grant _errors -o "$resource_directory/heap"
"$compiler" build tests/fixtures/runtime_stack_exhaustion.lkj --grant _errors -o "$resource_directory/stack"
for kind in heap stack; do
    code=0
    (ulimit -v 65536; exec "$resource_directory/$kind") > "$resource_directory/$kind.out" 2> "$resource_directory/$kind.err" || code=$?
    [[ "$code" == 1 && ! -s "$resource_directory/$kind.out" ]]
    if [[ "$kind" == heap ]]; then expected=R0006; else expected=R0008; fi
    grep -Eq "^[^:]+:[0-9]+: $expected " "$resource_directory/$kind.err"
done
printf 'ok kernel memory refusal, stack refusal, reservation reuse and small-limit startup\n'
