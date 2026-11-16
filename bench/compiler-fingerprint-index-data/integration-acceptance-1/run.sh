#!/usr/bin/env bash
set -euo pipefail
cd /home/coder/workspace/lkjscript
audit=bench/compiler-fingerprint-index-data/integration-acceptance-1
test ! -e "$audit/capture-1"
mkdir --mode=700 "$audit/capture-1"
set -C
pwd > "$audit/capture-1/cwd"
printf '%s\n' '{"argv":["/usr/bin/time","-o","bench/compiler-fingerprint-index-data/integration-acceptance-1/capture-1/time","-f","wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x","/usr/bin/timeout","--signal=TERM","--kill-after=10s","120s","/usr/bin/prlimit","--as=1073741824","--","/usr/bin/perl","bench/compiler-fingerprint-index-data/integration-acceptance-1/audit.pl"]}' > "$audit/capture-1/argv.json"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$audit/capture-1/start.utc"
date +%s%N > "$audit/capture-1/start.ns"
cat /proc/loadavg > "$audit/capture-1/load-before"
set +e
/usr/bin/time -o "$audit/capture-1/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' \
 /usr/bin/timeout --signal=TERM --kill-after=10s 120s /usr/bin/prlimit --as=1073741824 -- /usr/bin/perl "$audit/audit.pl" \
 > "$audit/capture-1/stdout" 2> "$audit/capture-1/stderr"
result=$?
set -e
date +%s%N > "$audit/capture-1/end.ns"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$audit/capture-1/end.utc"
cat /proc/loadavg > "$audit/capture-1/load-after"
printf '%s\n' "$result" > "$audit/capture-1/status"
cat "$audit/capture-1/stdout" "$audit/capture-1/stderr" "$audit/capture-1/time" "$audit/capture-1/load-before" "$audit/capture-1/load-after"
exit "$result"
