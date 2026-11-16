#!/usr/bin/env bash
set -euo pipefail
umask 077
capture=/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116/target/compiler-legacy-root-inventory-20261116/cleanup-capture
mkdir -m 700 "$capture"
finish() {
    result=$?
    set +e
    date -u +%Y-%m-%dT%H:%M:%SZ > "$capture/end.utc"
    date +%s%N > "$capture/end.ns"
    cat /proc/loadavg > "$capture/load.after"
    printf '%s\n' "$result" > "$capture/status"
    exit "$result"
}
trap finish EXIT
date -u +%Y-%m-%dT%H:%M:%SZ > "$capture/start.utc"
date +%s%N > "$capture/start.ns"
cat /proc/loadavg > "$capture/load.before"
printf '%s\n' 'perl /home/coder/workspace/lkjscript/bench/compiler-legacy-root-cleanup-data/guard.pl --execute-after-root-release cd1a63435ee856a334fc08340b4748c987f4838b' > "$capture/command.txt"
printf '%s\n' 'root release: bundle cd1a63435ee856a334fc08340b4748c987f4838b; published main be0b6ad9; one invocation; no retry or broader deletion; no observer or private-key reads' > "$capture/release.txt"
printf '%s\n' \
 '2caa0525e8a011ff5c014e27e1f10ba90cc151221ea3ea68ed157807ddb7ab37  /home/coder/workspace/lkjscript/bench/compiler-legacy-root-cleanup-data/guard.pl' \
 '436360a7987b8d34a015dede97589b60cd5b89ef565ee1bc8fd51e9e60945807  /home/coder/workspace/lkjscript/bench/compiler-legacy-root-cleanup-data/manifest.sha256' \
 | sha256sum --check --strict > "$capture/preflight.stdout" 2> "$capture/preflight.stderr"
printf '%s\n' 'guard-invocation-started' > "$capture/phase"
/usr/bin/time -f 'wall_s=%e user_s=%U system_s=%S max_rss_kib=%M exit=%x' -o "$capture/time" \
 perl /home/coder/workspace/lkjscript/bench/compiler-legacy-root-cleanup-data/guard.pl \
 --execute-after-root-release cd1a63435ee856a334fc08340b4748c987f4838b \
 > "$capture/stdout" 2> "$capture/stderr"
printf '%s\n' 'guard-invocation-ended-successfully' > "$capture/phase"
