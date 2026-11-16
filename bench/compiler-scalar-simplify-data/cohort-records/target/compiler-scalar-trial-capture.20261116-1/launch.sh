#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
cd /home/coder/workspace/lkjscript
capture="$PWD/target/compiler-scalar-trial-capture.20261116-1"
command=(bash bench/compiler_scalar_simplify.sh --pin "$PWD/bench/compiler-scalar-simplify-data/producer-pin.json" --results target/compiler-scalar-simplify.20261116-1)
printf '%s\0' "${command[@]}" > "$capture/argv"
printf '%s\n' "$PWD" > "$capture/cwd"
git rev-parse HEAD > "$capture/release-commit"
sha256sum bench/compiler_scalar_simplify.sh bench/compiler-scalar-simplify-plan.md bench/compiler-scalar-simplify-data/producer-pin.json > "$capture/inputs.sha256"
date -u +%Y-%m-%dT%H:%M:%SZ > "$capture/started.utc"
date +%s%N > "$capture/started.ns"
cut -d ' ' -f 1-3 /proc/loadavg > "$capture/load-before"
status=0
"${command[@]}" > "$capture/stdout" 2> "$capture/stderr" || status=$?
date +%s%N > "$capture/finished.ns"
date -u +%Y-%m-%dT%H:%M:%SZ > "$capture/finished.utc"
cut -d ' ' -f 1-3 /proc/loadavg > "$capture/load-after"
printf '%s\n' "$status" > "$capture/status"
cat "$capture/stdout"
cat "$capture/stderr" >&2
printf 'outer_status=%s\n' "$status"
exit "$status"
