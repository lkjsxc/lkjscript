set -euo pipefail
cleanup_dir=bench/compiler-query-current-cleanup
sha256sum -c > "$cleanup_dir/guard-hash-check.stdout" <<'CHECK'
4c21c6d498ef37684fb325855744a1f841d7f28869b1cc6aece8375e184e165f  bench/compiler-query-current-cleanup/guarded-cleanup.pl
CHECK
printf '%s\n' 'perl bench/compiler-query-current-cleanup/guarded-cleanup.pl --execute-reviewed-six-path-cleanup' > "$cleanup_dir/invocation"
pwd > "$cleanup_dir/cwd"
cat /proc/loadavg > "$cleanup_dir/load-before"
date -u +%Y-%m-%dT%H:%M:%SZ > "$cleanup_dir/started-at"
date +%s%N > "$cleanup_dir/start-ns"
set +e
(ulimit -v 4194304; /usr/bin/time -f 'wall_seconds=%e user_seconds=%U system_seconds=%S peak_rss_kib=%M exit=%x' -o "$cleanup_dir/time" timeout 900s perl bench/compiler-query-current-cleanup/guarded-cleanup.pl --execute-reviewed-six-path-cleanup) > "$cleanup_dir/cleanup.stdout" 2> "$cleanup_dir/cleanup.stderr"
cleanup_status=$?
set -e
date +%s%N > "$cleanup_dir/end-ns"
date -u +%Y-%m-%dT%H:%M:%SZ > "$cleanup_dir/finished-at"
cat /proc/loadavg > "$cleanup_dir/load-after"
printf '%s\n' "$cleanup_status" > "$cleanup_dir/status"
cat "$cleanup_dir/cleanup.stdout" "$cleanup_dir/cleanup.stderr" "$cleanup_dir/time" "$cleanup_dir/load-before" "$cleanup_dir/load-after"
exit "$cleanup_status"
