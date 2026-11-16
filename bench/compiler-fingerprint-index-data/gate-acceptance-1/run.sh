#!/usr/bin/env bash
set -euo pipefail
repo=/home/coder/workspace/lkjscript
out="$repo/bench/compiler-fingerprint-index-data/gate-acceptance-1"
cd "$repo"
export LC_ALL=C TZ=UTC
test ! -e "$out/start.ns"
set +e
/usr/bin/perl -c "$out/audit.pl" > "$out/syntax.stdout" 2> "$out/syntax.stderr"
syntax_status=$?
set -e
printf '%s\n' "$syntax_status" > "$out/syntax.status"
test "$syntax_status" = 0
printf '%s\n' "$repo" > "$out/cwd"
printf '%s\0' /usr/bin/time -o "$out/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' /usr/bin/timeout --kill-after=10s 120s /usr/bin/prlimit --as=1073741824 -- /usr/bin/perl "$out/audit.pl" > "$out/argv"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/start.utc"
awk '{print $1,$2,$3}' /proc/loadavg > "$out/load-before"
date +%s%N > "$out/start.ns"
set +e
/usr/bin/time -o "$out/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' /usr/bin/timeout --kill-after=10s 120s /usr/bin/prlimit --as=1073741824 -- /usr/bin/perl "$out/audit.pl" > "$out/stdout" 2> "$out/stderr"
audit_status=$?
set -e
date +%s%N > "$out/end.ns"
awk '{print $1,$2,$3}' /proc/loadavg > "$out/load-after"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/end.utc"
printf '%s\n' "$audit_status" > "$out/status"
cat "$out/stdout" "$out/stderr" "$out/time" "$out/load-before" "$out/load-after"
exit "$audit_status"
