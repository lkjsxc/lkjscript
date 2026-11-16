#!/usr/bin/env bash
set -euo pipefail
cd /home/coder/workspace/lkjscript
bundle=bench/compiler-reference-capture-data/final-cleanup-1
capture="$bundle/outer-1"
guard=/home/coder/workspace/lkjscript/bench/compiler-reference-capture-data/final-cleanup-1/guard.pl
release=/home/coder/workspace/lkjscript/bench/compiler-reference-capture-data/final-cleanup-1/release.json
test ! -e "$capture"
test "$(sha256sum "$guard" | cut -d ' ' -f1)" = 984c7e86d7cda2c9ffed7b20cb51d150b9c680ff0ec3218c9cce85563f4e2977
test "$(sha256sum target/lkjscript | cut -d ' ' -f1)" = b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db
mkdir "$capture"
set -C
pwd > "$capture/cwd"
sha256sum target/lkjscript > "$capture/canonical.before"
sha256sum "$guard" "$release" "$bundle/expected-scope.json" "$bundle/auxiliary-inventory.json" > "$capture/controls.before"
df -Pk . > "$capture/free-space"
printf '%s\n' '{"argv":["/usr/bin/time","-o","bench/compiler-reference-capture-data/final-cleanup-1/outer-1/time","-f","wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x","/usr/bin/timeout","--signal=TERM","--kill-after=10s","900s","/usr/bin/prlimit","--as=4294967296","--","/usr/bin/perl","/home/coder/workspace/lkjscript/bench/compiler-reference-capture-data/final-cleanup-1/guard.pl","--release","/home/coder/workspace/lkjscript/bench/compiler-reference-capture-data/final-cleanup-1/release.json","--execute-after-root-release"]}' > "$capture/argv.json"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$capture/start.utc"
date +%s%N > "$capture/start.ns"
cat /proc/loadavg > "$capture/load-before"
set +e
/usr/bin/time -o "$capture/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' \
  /usr/bin/timeout --signal=TERM --kill-after=10s 900s \
  /usr/bin/prlimit --as=4294967296 -- \
  /usr/bin/perl "$guard" --release "$release" --execute-after-root-release \
  > "$capture/stdout" 2> "$capture/stderr"
result=$?
set -e
date +%s%N > "$capture/end.ns"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$capture/end.utc"
cat /proc/loadavg > "$capture/load-after"
printf '%s\n' "$result" > "$capture/status"
sha256sum target/lkjscript > "$capture/canonical.after"
sha256sum "$guard" "$release" "$bundle/expected-scope.json" "$bundle/auxiliary-inventory.json" > "$capture/controls.after"
cmp "$capture/canonical.before" "$capture/canonical.after"
cmp "$capture/controls.before" "$capture/controls.after"
cat "$capture/stdout" "$capture/stderr" "$capture/time" "$capture/load-before" "$capture/load-after"
exit "$result"
