#!/usr/bin/env bash
set -euo pipefail
repo=/home/coder/workspace/lkjscript
out="$repo/bench/compiler-fingerprint-index-data/baseline-seed-recovery-1"
baseline="$repo/target/compiler-fingerprint-index-golden-1/bin/compiler-baseline"
cd "$repo"
export LC_ALL=C TZ=UTC
test ! -e "$out" && test ! -L "$out"
test -f "$baseline" && test ! -L "$baseline"
test -f seed/lkjscript.gz && test ! -L seed/lkjscript.gz
test "$(sha256sum "$baseline" | cut -d ' ' -f 1)" = b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db
test "$(sha256sum seed/lkjscript.gz | cut -d ' ' -f 1)" = c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae
mkdir -m 700 "$out"
git -c core.packedGitWindowSize=1m -c core.packedGitLimit=64m show e5294c339d9aefbbafc2e951c997f4fefabcc9ad:seed/lkjscript.gz | cmp - seed/lkjscript.gz
printf '0\n' > "$out/git-seed-comparison.status"
printf '%s\n' "$repo" > "$out/cwd"
printf 'LC_ALL=C\nTZ=UTC\n' > "$out/environment"
sha256sum seed/lkjscript.gz "$baseline" > "$out/inputs-before.sha256"
stat -c '%n %s %a %h' seed/lkjscript.gz "$baseline" > "$out/input-sizes"
argv=(/usr/bin/time -o "$out/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x'
    /usr/bin/timeout --signal=TERM --kill-after=10s 900s
    /usr/bin/prlimit --as=4294967296 -- /bin/bash -o pipefail -c 'gzip -dc "$1" | cmp - "$2"' baseline-seed-check "$repo/seed/lkjscript.gz" "$baseline")
printf '%s\0' "${argv[@]}" > "$out/argv"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/start.utc"
awk '{print $1,$2,$3}' /proc/loadavg > "$out/load-before"
date +%s%N > "$out/start.ns"
set +e
"${argv[@]}" < /dev/null > "$out/stdout" 2> "$out/stderr"
seed_check_status=$?
set -e
date +%s%N > "$out/end.ns"
awk '{print $1,$2,$3}' /proc/loadavg > "$out/load-after"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/end.utc"
printf '%s\n' "$seed_check_status" > "$out/status"
sha256sum seed/lkjscript.gz "$baseline" > "$out/inputs-after.sha256"
cmp "$out/inputs-before.sha256" "$out/inputs-after.sha256"
cat "$out/time" "$out/load-before" "$out/load-after"
exit "$seed_check_status"
