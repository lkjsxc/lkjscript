#!/usr/bin/env bash
set -eu
phase=$1
shift
test ! -e "$phase"
mkdir "$phase"
printf '%q ' "$@" > "$phase/command"
printf '\n' >> "$phase/command"
pwd > "$phase/cwd"
date -u +%FT%TZ > "$phase/start.utc"
cat /proc/loadavg > "$phase/load.before"
set +e
/usr/bin/time -f 'wall=%e user=%U sys=%S rss_kib=%M exit=%x' -o "$phase/time" "$@" > "$phase/stdout" 2> "$phase/stderr"
code=$?
set -e
printf '%s\n' "$code" > "$phase/status"
cat /proc/loadavg > "$phase/load.after"
date -u +%FT%TZ > "$phase/end.utc"
cat "$phase/status" "$phase/time" "$phase/load.before" "$phase/load.after"
cat "$phase/stderr"
tail -8 "$phase/stdout"
exit "$code"
