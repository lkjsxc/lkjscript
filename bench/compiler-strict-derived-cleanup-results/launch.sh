set -u
out=bench/compiler-strict-derived-cleanup-results
mkdir "$out" || exit 2
printf %s 'Root release after the prospective scale measurement completed.
Authorized invocation, exactly once:
perl bench/compiler-strict-derived-cleanup.pl --execute-after-root-release

Approved guard SHA256:
0ce6f900067446d081dad5b1239409d971a74bd3bb088bcc4093c9327d28b894
Approved inventory helper SHA256:
a8cfa01bf8db01e07cc80a8191b5c018ab1354942acb44b7a33f840739fc1d94
Prepared commit:
d74b638e4f20b9772d9f34f1f144148a021ccac4

Only strict-derived.ZQv7cu may be deleted. Preserve completed lock/generation
copies and every committed result, source/producer control and historical-screen
record. On guard refusal, retain the failure and stop without broadening
exclusions or invoking another deletion route. No compiler workload is released.
' > "$out/release.txt"
git rev-parse HEAD > "$out/preparation-commit"
sha256sum bench/compiler-strict-derived-cleanup.pl bench/compiler-strict-derived-cleanup-data/inventory.pl bench/compiler-strict-derived-cleanup.prepared.sha256 bench/compiler-history-comment-rename.sha256 > "$out/inputs.sha256"
sha256sum --status -c bench/compiler-history-comment-rename.sha256 || exit 2
sha256sum bench/compiler-strict-derived-cleanup-data/metadata/* > "$out/metadata.before.sha256"
if [[ -d target/compiler-strict-derived.ZQv7cu && ! -L target/compiler-strict-derived.ZQv7cu ]]; then printf '%s\n' present > "$out/fixture.before"; else printf '%s\n' absent-or-invalid > "$out/fixture.before"; exit 2; fi
date -u '+%Y-%m-%dT%H:%M:%SZ' > "$out/utc.before"
date +%s%N > "$out/time.before.ns"
cat /proc/loadavg > "$out/load.before"
/usr/bin/time -f 'elapsed_s=%e\nuser_s=%U\nsystem_s=%S\npeak_rss_kib=%M' -o "$out/time" perl bench/compiler-strict-derived-cleanup.pl --execute-after-root-release > "$out/stdout" 2> "$out/stderr"
status=$?
cat /proc/loadavg > "$out/load.after"
date +%s%N > "$out/time.after.ns"
date -u '+%Y-%m-%dT%H:%M:%SZ' > "$out/utc.after"
printf '%s\n' "$status" > "$out/exit"
if [[ -e target/compiler-strict-derived.ZQv7cu || -L target/compiler-strict-derived.ZQv7cu ]]; then printf '%s\n' present > "$out/fixture.after"; else printf '%s\n' absent > "$out/fixture.after"; fi
cat "$out/stdout" "$out/stderr" "$out/time"
printf 'cleanup_exit=%s\n' "$status"
exit "$status"