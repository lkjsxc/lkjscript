#!/usr/bin/env bash
set -u
set -o noclobber
root=/home/coder/workspace/lkjscript
pkg="$root/target/compiler-scalar-cohort-package-1"
cd "$root" || exit 90
printf '%s\n' "$PWD" > "$pkg/capture/cwd"
printf '%s\n' 'one sequential compression worker; 4 GiB AS; 900s timeout; 10s TERM grace; no compiler/native/performance/deletion' > "$pkg/capture/limits"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$pkg/capture/start.utc"
date +%s%N > "$pkg/capture/start.ns"
cut -d ' ' -f 1-3 /proc/loadavg > "$pkg/capture/load-before"
/usr/bin/time -o "$pkg/capture/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' /usr/bin/timeout --signal=TERM --kill-after=10s 900s /usr/bin/prlimit --as=4294967296 -- perl "$pkg/pack.pl" > "$pkg/capture/stdout" 2> "$pkg/capture/stderr"
status=$?
printf '%s\n' "$status" > "$pkg/capture/status"
cut -d ' ' -f 1-3 /proc/loadavg > "$pkg/capture/load-after"
date +%s%N > "$pkg/capture/end.ns"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$pkg/capture/end.utc"
printf '%s\n' complete > "$pkg/capture/capture.status"
cat "$pkg/capture/stdout"
cat "$pkg/capture/stderr" >&2
printf 'package_status=%s\n' "$status"
cat "$pkg/capture/time" "$pkg/capture/load-before" "$pkg/capture/load-after"
exit "$status"
