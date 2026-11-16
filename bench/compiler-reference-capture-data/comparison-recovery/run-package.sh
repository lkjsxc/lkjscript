#!/usr/bin/env bash
set -euo pipefail
umask 077
export LC_ALL=C
pkg=/home/coder/workspace/lkjscript/target/compiler-reference-capture-comparison-recovery-2
capture="$pkg/capture"
mkdir "$capture"
printf '%s\n' '/home/coder/workspace/lkjscript' > "$capture/cwd"
pwd -P > "$capture/cwd.actual"
printf '%s\n' '["/usr/bin/time","-f","elapsed_s=%e user_s=%U system_s=%S peak_rss_kib=%M","-o","/home/coder/workspace/lkjscript/target/compiler-reference-capture-comparison-recovery-2/capture/time","/usr/bin/timeout","--signal=TERM","--kill-after=10s","900s","/usr/bin/prlimit","--as=4294967296","--","/usr/bin/perl","/home/coder/workspace/lkjscript/target/compiler-reference-capture-comparison-recovery-2/package.pl"]' > "$capture/argv.json"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$capture/start.utc"
date +%s%N > "$capture/start.ns"
cut -d' ' -f1-3 /proc/loadavg > "$capture/load-before"
df -PB1 "$pkg" > "$capture/free-space"
set +e
/usr/bin/time -f 'elapsed_s=%e user_s=%U system_s=%S peak_rss_kib=%M' -o "$capture/time" /usr/bin/timeout --signal=TERM --kill-after=10s 900s /usr/bin/prlimit --as=4294967296 -- /usr/bin/perl "$pkg/package.pl" > "$capture/stdout" 2> "$capture/stderr"
rc=$?
set -e
printf '%s\n' "$rc" > "$capture/status"
cut -d' ' -f1-3 /proc/loadavg > "$capture/load-after"
date +%s%N > "$capture/end.ns"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$capture/end.utc"
cat "$capture/stdout"
cat "$capture/stderr" >&2
cat "$capture/time"
exit "$rc"
