#!/usr/bin/env bash
set -u
repo=/home/coder/workspace/lkjscript
out=$repo/target/compiler-reference-capture-preliminary-retention-1
cd "$repo"
perl -c "$out/pack.pl" > "$out/syntax.stdout" 2> "$out/syntax.stderr"
syntax_status=$?
printf '%s\n' "$syntax_status" > "$out/syntax.status"
if [ "$syntax_status" -ne 0 ]; then exit "$syntax_status"; fi
printf '%s\n' "$PWD" > "$out/cwd"
printf '%s\0' /usr/bin/time -o "$out/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' /usr/bin/timeout --signal=TERM --kill-after=10s 900s /usr/bin/prlimit --as=4294967296 -- perl "$out/pack.pl" > "$out/argv"
printf '%s\n' 'one compression worker; 4 GiB AS; 900 s timeout; 10 s TERM grace; no compiler/native workload' > "$out/limits"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/started.utc"
date +%s%N > "$out/started.ns"
cut -d ' ' -f 1-3 /proc/loadavg > "$out/load-before"
/usr/bin/time -o "$out/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' \
 /usr/bin/timeout --signal=TERM --kill-after=10s 900s \
 /usr/bin/prlimit --as=4294967296 -- perl "$out/pack.pl" > "$out/stdout" 2> "$out/stderr"
status=$?
printf '%s\n' "$status" > "$out/status"
date +%s%N > "$out/finished.ns"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/finished.utc"
cut -d ' ' -f 1-3 /proc/loadavg > "$out/load-after"
cat "$out/stdout"
cat "$out/stderr" >&2
cat "$out/time"
exit "$status"
