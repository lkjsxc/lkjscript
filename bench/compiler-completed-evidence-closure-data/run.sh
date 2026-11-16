#!/usr/bin/env bash
set -euo pipefail
umask 077
repo=/home/coder/workspace/lkjscript
bundle=bench/compiler-completed-evidence-closure-data
result="$repo/bench/compiler-completed-evidence-closure-result"
[[ $# == 1 && $1 =~ ^[0-9a-f]{40}$ ]] || { printf 'exact release commit required\n' >&2; exit 2; }
release=$1
[[ ${BASH_SOURCE[0]} == "$repo/$bundle/run.sh" ]] || { printf 'run the retained absolute main path\n' >&2; exit 2; }
git -C "$repo" cat-file blob "$release:$bundle/run.sh" | cmp - "$repo/$bundle/run.sh"
mkdir "$result"
printf '%s\n' "$release" > "$result/release.commit"
printf '%s\n' "$BASHPID" > "$result/owner-pid"
printf '%s\n' "$PWD" > "$result/cwd"
command=(/usr/bin/time -f 'wall=%e\nuser=%U\nsystem=%S\nmax_rss_kib=%M\nexit=%x' -o "$result/time"
 /usr/bin/prlimit --as=4294967296 -- /usr/bin/timeout --kill-after=10s 300s
 /usr/bin/perl "$repo/$bundle/guard.pl" --execute "$release")
/usr/bin/perl -MJSON::PP -e 'print JSON::PP->new->canonical->encode(\@ARGV),"\n"' "${command[@]}" > "$result/argv.json"
cut -d' ' -f1-3 /proc/loadavg > "$result/load.before"
date -u '+%Y-%m-%dT%H:%M:%SZ' > "$result/start.utc"
date -u '+%s%N' > "$result/start.ns"
set +e
"${command[@]}" > "$result/stdout" 2> "$result/stderr"
status=$?
set -e
printf '%s\n' "$status" > "$result/status"
date -u '+%s%N' > "$result/end.ns"
date -u '+%Y-%m-%dT%H:%M:%SZ' > "$result/end.utc"
cut -d' ' -f1-3 /proc/loadavg > "$result/load.after"
exit "$status"
