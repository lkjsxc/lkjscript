#!/usr/bin/env bash
set -euo pipefail
cd /home/coder/workspace/lkjscript
audit=bench/compiler-fingerprint-index-data/integration-acceptance-1
test ! -e "$audit/capture-2"
mkdir --mode=700 "$audit/capture-2"
set -C
pwd > "$audit/capture-2/cwd"
printf '%s\n' '{"argv":["/usr/bin/time","-o","bench/compiler-fingerprint-index-data/integration-acceptance-1/capture-2/time","-f","wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x","/usr/bin/timeout","--signal=TERM","--kill-after=10s","120s","/usr/bin/prlimit","--as=1073741824","--","/usr/bin/perl","bench/compiler-fingerprint-index-data/integration-acceptance-1/audit-v2.pl"]}' > "$audit/capture-2/argv.json"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$audit/capture-2/start.utc"
date +%s%N > "$audit/capture-2/start.ns"
cat /proc/loadavg > "$audit/capture-2/load-before"
set +e
/usr/bin/time -o "$audit/capture-2/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' \
 /usr/bin/timeout --signal=TERM --kill-after=10s 120s /usr/bin/prlimit --as=1073741824 -- /usr/bin/perl "$audit/audit-v2.pl" \
 > "$audit/capture-2/stdout" 2> "$audit/capture-2/stderr"
result=$?
set -e
date +%s%N > "$audit/capture-2/end.ns"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$audit/capture-2/end.utc"
cat /proc/loadavg > "$audit/capture-2/load-after"
printf '%s\n' "$result" > "$audit/capture-2/status"
cat "$audit/capture-2/stdout" "$audit/capture-2/stderr" "$audit/capture-2/time" "$audit/capture-2/load-before" "$audit/capture-2/load-after"
exit "$result"
