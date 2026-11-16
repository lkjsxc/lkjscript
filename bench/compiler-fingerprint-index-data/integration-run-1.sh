#!/usr/bin/env bash
set -euo pipefail
repo=/home/coder/workspace/lkjscript
prepared="$repo/target/compiler-fingerprint-index-integration-1"
out="$repo/bench/compiler-fingerprint-index-data/integration-execution-1"
cd "$repo"
export LC_ALL=C TZ=UTC
unset LKJSCRIPT_STORE LKJSCRIPT_STORE_KEY LKJSCRIPT_STORE_IO
test ! -e "$out" && test ! -L "$out"
test "$(sha256sum "$prepared/manifest.sha256" | cut -d ' ' -f 1)" = 3bd8cf979c22f3a64143870e27c395cc52d1b6e6a95e83e6b541dddb80abbca3
test "$(sha256sum "$prepared/guard.pl" | cut -d ' ' -f 1)" = e87b21fe9b6f691f1575484e16bd735c5f7b3cc67f4a7e3a9cf0f719a147590a
mkdir -m 700 "$out"
(cd "$prepared" && sha256sum -c manifest.sha256) > "$out/preparation-check.stdout" 2> "$out/preparation-check.stderr"
set +e
/usr/bin/perl "$prepared/guard.pl" > "$out/guard.stdout" 2> "$out/guard.stderr"
guard_status=$?
set -e
printf '%s\n' "$guard_status" > "$out/guard.status"
test "$guard_status" = 0
cp "$prepared/commands.json" "$out/command-plan.json"
cp "$prepared/before/inputs.sha256" "$out/inputs-before.sha256"
cp "$prepared/before/tracked-language.sha256" "$out/language-before.sha256"
git rev-parse HEAD > "$out/base-commit"
printf '%s\n' "$repo" > "$out/cwd"
printf 'LC_ALL=C\nTZ=UTC\nLKJSCRIPT_STORE=unset\nLKJSCRIPT_STORE_KEY=unset; explicit argv path only\nLKJSCRIPT_STORE_IO=unset\n' > "$out/environment"
sha256sum target/lkjscript seed/lkjscript.gz > "$out/producer-seed-before.sha256"
sha256sum bench/compiler-fingerprint-index-data/integration-run-1.sh > "$out/wrapper-before.sha256"
df -Pk "$repo" > "$out/free-space"
edit_argv=("$repo/target/lkjscript" edit src/semantic_main.lkj --source-dir "$repo" --library-dir "$repo/library" --request "$prepared/request.json" --no-store --store-key /home/coder/.local/state/lkjscript/store-key)
wrapped_argv=(/usr/bin/time -o "$out/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x'
    /usr/bin/timeout --signal=TERM --kill-after=10s 900s
    /usr/bin/prlimit --as=4294967296 -- "${edit_argv[@]}")
printf '%s\0' "${edit_argv[@]}" > "$out/edit.argv"
printf '%s\0' "${wrapped_argv[@]}" > "$out/wrapped.argv"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/start.utc"
awk '{print $1,$2,$3}' /proc/loadavg > "$out/load-before"
date +%s%N > "$out/start.ns"
set +e
"${wrapped_argv[@]}" < /dev/null > "$out/stdout" 2> "$out/stderr"
edit_status=$?
set -e
date +%s%N > "$out/end.ns"
awk '{print $1,$2,$3}' /proc/loadavg > "$out/load-after"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/end.utc"
printf '%s\n' "$edit_status" > "$out/status"
git ls-files --cached --others --exclude-standard -z -- src library tests | sort -zu | xargs -0 sha256sum > "$out/inputs-after.sha256"
git ls-files --cached --others --exclude-standard -z -- '*.lkj' | sort -zu | xargs -0 sha256sum > "$out/language-after.sha256"
sha256sum target/lkjscript seed/lkjscript.gz > "$out/producer-seed-after.sha256"
sha256sum bench/compiler-fingerprint-index-data/integration-run-1.sh > "$out/wrapper-after.sha256"
cmp "$out/producer-seed-before.sha256" "$out/producer-seed-after.sha256"
cmp "$out/wrapper-before.sha256" "$out/wrapper-after.sha256"
git diff --numstat -- src library tests > "$out/tracked-numstat"
git diff -- src/elaborate/incremental.lkj src/semantic_tests.lkj src/semantic_main.lkj > "$out/tracked.diff"
cat "$out/time" "$out/load-before" "$out/load-after"
cat "$out/stdout" "$out/stderr"
exit "$edit_status"
