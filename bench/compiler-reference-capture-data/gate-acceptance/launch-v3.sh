#!/usr/bin/env bash
set -u
repo=/home/coder/workspace/lkjscript
out=$repo/target/compiler-reference-capture-gate-acceptance-1
cd "$repo"
perl -c "$out/audit-v3.pl" > "$out/v3-syntax.stdout" 2> "$out/v3-syntax.stderr"
syntax_status=$?
printf '%s\n' "$syntax_status" > "$out/v3-syntax.status"
if [ "$syntax_status" -ne 0 ]; then exit "$syntax_status"; fi
printf '%s\n' "$PWD" > "$out/v3-cwd"
printf '%s\0' /usr/bin/time -o "$out/v3-time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' /usr/bin/timeout --signal=TERM --kill-after=10s 900s /usr/bin/prlimit --as=4294967296 -- perl "$out/audit-v3.pl" > "$out/v3-argv"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/v3-started.utc"
date +%s%N > "$out/v3-started.ns"
cut -d ' ' -f 1-3 /proc/loadavg > "$out/v3-load-before"
/usr/bin/time -o "$out/v3-time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' /usr/bin/timeout --signal=TERM --kill-after=10s 900s /usr/bin/prlimit --as=4294967296 -- perl "$out/audit-v3.pl" > "$out/v3-stdout" 2> "$out/v3-stderr"
result=$?
printf '%s\n' "$result" > "$out/v3-status"
date +%s%N > "$out/v3-finished.ns"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/v3-finished.utc"
cut -d ' ' -f 1-3 /proc/loadavg > "$out/v3-load-after"
cat "$out/v3-stdout"
cat "$out/v3-stderr" >&2
cat "$out/v3-time"
exit "$result"
