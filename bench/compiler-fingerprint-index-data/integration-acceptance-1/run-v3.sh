#!/usr/bin/env bash
set -euo pipefail
cd /home/coder/workspace/lkjscript
audit=bench/compiler-fingerprint-index-data/integration-acceptance-1
test ! -e "$audit/capture-3"
mkdir --mode=700 "$audit/capture-3"
set -C
pwd > "$audit/capture-3/cwd"
printf '%s\n' '{"argv":["/usr/bin/time","-o","bench/compiler-fingerprint-index-data/integration-acceptance-1/capture-3/time","-f","wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x","/usr/bin/timeout","--signal=TERM","--kill-after=10s","120s","/usr/bin/prlimit","--as=1073741824","--","/usr/bin/perl","bench/compiler-fingerprint-index-data/integration-acceptance-1/audit-v3.pl"]}' > "$audit/capture-3/argv.json"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$audit/capture-3/start.utc"
date +%s%N > "$audit/capture-3/start.ns"
cat /proc/loadavg > "$audit/capture-3/load-before"
set +e
/usr/bin/time -o "$audit/capture-3/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' \
 /usr/bin/timeout --signal=TERM --kill-after=10s 120s /usr/bin/prlimit --as=1073741824 -- /usr/bin/perl "$audit/audit-v3.pl" \
 > "$audit/capture-3/stdout" 2> "$audit/capture-3/stderr"
result=$?
set -e
date +%s%N > "$audit/capture-3/end.ns"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$audit/capture-3/end.utc"
cat /proc/loadavg > "$audit/capture-3/load-after"
printf '%s\n' "$result" > "$audit/capture-3/status"
cat "$audit/capture-3/stdout" "$audit/capture-3/stderr" "$audit/capture-3/time" "$audit/capture-3/load-before" "$audit/capture-3/load-after"
exit "$result"
