#!/usr/bin/env bash
set -euo pipefail
cd /home/coder/workspace/lkjscript
audit=bench/compiler-fingerprint-index-data/integration-acceptance-1
test ! -e "$audit/capture-5"
mkdir --mode=700 "$audit/capture-5"
set -C
pwd > "$audit/capture-5/cwd"
printf '%s\n' '{"argv":["/usr/bin/time","-o","bench/compiler-fingerprint-index-data/integration-acceptance-1/capture-5/time","-f","wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x","/usr/bin/timeout","--signal=TERM","--kill-after=10s","120s","/usr/bin/prlimit","--as=1073741824","--","/usr/bin/perl","bench/compiler-fingerprint-index-data/integration-acceptance-1/audit-v5.pl"]}' > "$audit/capture-5/argv.json"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$audit/capture-5/start.utc"
date +%s%N > "$audit/capture-5/start.ns"
cat /proc/loadavg > "$audit/capture-5/load-before"
set +e
/usr/bin/time -o "$audit/capture-5/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' \
 /usr/bin/timeout --signal=TERM --kill-after=10s 120s /usr/bin/prlimit --as=1073741824 -- /usr/bin/perl "$audit/audit-v5.pl" \
 > "$audit/capture-5/stdout" 2> "$audit/capture-5/stderr"
result=$?
set -e
date +%s%N > "$audit/capture-5/end.ns"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$audit/capture-5/end.utc"
cat /proc/loadavg > "$audit/capture-5/load-after"
printf '%s\n' "$result" > "$audit/capture-5/status"
cat "$audit/capture-5/stdout" "$audit/capture-5/stderr" "$audit/capture-5/time" "$audit/capture-5/load-before" "$audit/capture-5/load-after"
exit "$result"
