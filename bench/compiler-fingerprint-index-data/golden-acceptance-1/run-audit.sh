#!/bin/bash
set -u
cd /home/coder/workspace/lkjscript || exit 120
audit_dir=bench/compiler-fingerprint-index-data/golden-acceptance-1
test ! -e "$audit_dir/audit.stdout" || exit 121
printf '%s\n' "$PWD" >"$audit_dir/audit.cwd"
cat >"$audit_dir/audit.argv.json" <<'JSON'
{
  "argv": ["/usr/bin/perl", "bench/compiler-fingerprint-index-data/golden-acceptance-1/audit.pl"],
  "cwd": "/home/coder/workspace/lkjscript",
  "wrapper_argv": ["/usr/bin/time", "-o", "bench/compiler-fingerprint-index-data/golden-acceptance-1/audit.time.tsv", "-f", "elapsed_seconds\t%e\nuser_seconds\t%U\nsystem_seconds\t%S\nmax_rss_kib\t%M\nexit\t%x", "/usr/bin/timeout", "--signal=TERM", "--kill-after=10s", "900s", "/usr/bin/prlimit", "--as=4294967296", "--", "/usr/bin/perl", "bench/compiler-fingerprint-index-data/golden-acceptance-1/audit.pl"],
  "environment": {"LC_ALL": "C", "TZ": "UTC"},
  "purpose": "Independent retained-record/Git/byte audit only; no capture helper, compiler, native image, decompression or archive command."
}
JSON
export LC_ALL=C TZ=UTC
date -u '+%Y-%m-%dT%H:%M:%S.%NZ' >"$audit_dir/audit.before.utc"
date '+%s%N' >"$audit_dir/audit.before.ns"
awk '{print $1,$2,$3}' /proc/loadavg >"$audit_dir/audit.load-before"
/usr/bin/time -o "$audit_dir/audit.time.tsv" -f $'elapsed_seconds\t%e\nuser_seconds\t%U\nsystem_seconds\t%S\nmax_rss_kib\t%M\nexit\t%x' \
  /usr/bin/timeout --signal=TERM --kill-after=10s 900s \
  /usr/bin/prlimit --as=4294967296 -- \
  /usr/bin/perl "$audit_dir/audit.pl" \
  >"$audit_dir/audit.stdout" 2>"$audit_dir/audit.stderr"
audit_status=$?
date '+%s%N' >"$audit_dir/audit.after.ns"
date -u '+%Y-%m-%dT%H:%M:%S.%NZ' >"$audit_dir/audit.after.utc"
awk '{print $1,$2,$3}' /proc/loadavg >"$audit_dir/audit.load-after"
printf '%s\n' "$audit_status" >"$audit_dir/audit.status"
printf 'audit exit %s\n' "$audit_status"
cat "$audit_dir/audit.stdout" "$audit_dir/audit.stderr" "$audit_dir/audit.time.tsv"
exit "$audit_status"
