#!/usr/bin/env bash
set -u
directory=/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116/target/compiler-copy-sites-cleanup-20261116
cd "$directory" || exit 125
date -u +%Y-%m-%dT%H:%M:%SZ > start.utc
date +%s%N > start.ns
read -r load1 load5 load15 rest < /proc/loadavg
printf '%s %s %s\n' "$load1" "$load5" "$load15" > load.before
/usr/bin/time -f 'wall_seconds=%e user_seconds=%U system_seconds=%S peak_rss_kib=%M' -o time \
    perl guard.pl --execute-after-root-release > stdout 2> stderr
status=$?
printf '%s\n' "$status" > status
read -r load1 load5 load15 rest < /proc/loadavg
printf '%s %s %s\n' "$load1" "$load5" "$load15" > load.after
date +%s%N > end.ns
date -u +%Y-%m-%dT%H:%M:%SZ > end.utc
exit "$status"
