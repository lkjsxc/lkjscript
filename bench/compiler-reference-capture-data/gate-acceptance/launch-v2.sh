#!/usr/bin/env bash
set -u
repo=/home/coder/workspace/lkjscript
out=$repo/target/compiler-reference-capture-gate-acceptance-1
cd "$repo"
perl -c "$out/audit-v2.pl" > "$out/syntax-v2.stdout" 2> "$out/syntax-v2.stderr"
syntax_status=$?
printf '%s\n' "$syntax_status" > "$out/syntax-v2.status"
if [ "$syntax_status" -ne 0 ]; then exit "$syntax_status"; fi
printf '%s\n' "$PWD" > "$out/cwd"
printf '%s\0' /usr/bin/time -o "$out/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' /usr/bin/timeout --signal=TERM --kill-after=10s 900s /usr/bin/prlimit --as=4294967296 -- perl "$out/audit-v2.pl" > "$out/argv"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/started.utc"
date +%s%N > "$out/started.ns"
cut -d ' ' -f 1-3 /proc/loadavg > "$out/load-before"
/usr/bin/time -o "$out/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' /usr/bin/timeout --signal=TERM --kill-after=10s 900s /usr/bin/prlimit --as=4294967296 -- perl "$out/audit-v2.pl" > "$out/stdout" 2> "$out/stderr"
result=$?
printf '%s\n' "$result" > "$out/status"
date +%s%N > "$out/finished.ns"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/finished.utc"
cut -d ' ' -f 1-3 /proc/loadavg > "$out/load-after"
cat "$out/stdout"
cat "$out/stderr" >&2
cat "$out/time"
exit "$result"
