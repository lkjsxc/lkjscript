#!/usr/bin/env bash
set -u
umask 077
export PATH=/usr/bin:/bin LC_ALL=C TZ=UTC
D=/home/coder/workspace/lkjscript/target/compiler-reference-capture-comparison-acceptance-2
mkdir "$D/phase-1" || exit 1
P="$D/phase-1"
printf '%s\n' '/usr/bin/perl -c audit.pl' > "$P/syntax.argv"
perl -c "$D/audit.pl" > "$P/syntax.stdout" 2> "$P/syntax.stderr"
syntax_status=$?
printf '%s\n' "$syntax_status" > "$P/syntax.status"
[ "$syntax_status" -eq 0 ] || exit "$syntax_status"
sha256sum "$D/audit.pl" > "$P/auditor.sha256"
printf '%s\n' '/usr/bin/time -f wall_s=%e\nuser_s=%U\nsystem_s=%S\nmax_rss_kib=%M\nexit=%x -o phase-1/time /usr/bin/timeout --kill-after=10s 900s /usr/bin/prlimit --as=4294967296 -- /usr/bin/perl audit.pl' > "$P/argv"
pwd > "$P/cwd"
date -u +%Y-%m-%dT%H:%M:%SZ > "$P/start.utc"
date +%s%N > "$P/start.ns"
cut -d' ' -f1-3 /proc/loadavg > "$P/load-before"
/usr/bin/time -f 'wall_s=%e\nuser_s=%U\nsystem_s=%S\nmax_rss_kib=%M\nexit=%x' -o "$P/time" /usr/bin/timeout --kill-after=10s 900s /usr/bin/prlimit --as=4294967296 -- /usr/bin/perl "$D/audit.pl" > "$P/stdout" 2> "$P/stderr" < /dev/null
audit_status=$?
printf '%s\n' "$audit_status" > "$P/status"
cut -d' ' -f1-3 /proc/loadavg > "$P/load-after"
date +%s%N > "$P/end.ns"
date -u +%Y-%m-%dT%H:%M:%SZ > "$P/end.utc"
cat "$P/stdout" "$P/stderr"
exit "$audit_status"
