#!/usr/bin/env bash
set -euo pipefail
repo=/home/coder/workspace/lkjscript
out="$repo/bench/compiler-fingerprint-index-data/gate-1"
gate_store="$repo/target/compiler-fingerprint-index-gate-store-1"
cd "$repo"
export LC_ALL=C TZ=UTC
unset LKJSCRIPT_STORE_IO
test ! -e "$out" && test ! -L "$out"
test ! -e "$gate_store" && test ! -L "$gate_store"
test "$(sha256sum target/lkjscript | cut -d ' ' -f 1)" = b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db
test "$(sha256sum seed/lkjscript.gz | cut -d ' ' -f 1)" = c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae
test "$(sed -n '/^## Programs$/,$p' docs/status.md | sha256sum | cut -d ' ' -f 1)" = 235aa6c283d334688289ab73e472cae920d25b1e8cbad76cb4461f30d18cc9f4
mkdir -m 700 "$out"
mkdir -m 700 "$gate_store"
export LKJSCRIPT_GATE_JOBS=4
export LKJSCRIPT_STORE="$gate_store"
export LKJSCRIPT_STORE_KEY=/home/coder/.local/state/lkjscript/store-key
test -f "$LKJSCRIPT_STORE_KEY" && test -r "$LKJSCRIPT_STORE_KEY"
manifest_inputs() {
    git ls-files --cached --others --exclude-standard -z -- src library tests tools examples witness .github |
        sort -zu | xargs -0 sha256sum
}
manifest_inputs > "$out/inputs-before.sha256"
sha256sum target/lkjscript seed/lkjscript.gz > "$out/producer-seed-before.sha256"
sha256sum bench/compiler-fingerprint-index-data/gate-run-1.sh > "$out/wrapper-before.sha256"
git rev-parse HEAD > "$out/base-commit"
git diff --stat -- src library tests tools > "$out/source-diff-before"
printf '%s\n' "$repo" > "$out/cwd"
printf 'LC_ALL=C\nTZ=UTC\nLKJSCRIPT_STORE_IO=unset\nLKJSCRIPT_GATE_JOBS=4\nLKJSCRIPT_STORE=%s\nLKJSCRIPT_STORE_KEY=%s\n' "$LKJSCRIPT_STORE" "$LKJSCRIPT_STORE_KEY" > "$out/environment"
df -Pk "$repo" > "$out/free-space"
gate_argv=(/usr/bin/time -o "$out/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x'
    /usr/bin/timeout --signal=TERM --kill-after=10s 2400s
    /usr/bin/prlimit --as=4294967296 -- bash tools/check.sh)
printf '%s\0' "${gate_argv[@]}" > "$out/argv"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/start.utc"
awk '{print $1,$2,$3}' /proc/loadavg > "$out/load-before"
date +%s%N > "$out/start.ns"
set +e
"${gate_argv[@]}" < /dev/null > "$out/stdout" 2> "$out/stderr"
gate_status=$?
set -e
date +%s%N > "$out/end.ns"
awk '{print $1,$2,$3}' /proc/loadavg > "$out/load-after"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/end.utc"
printf '%s\n' "$gate_status" > "$out/status"
manifest_inputs > "$out/inputs-after.sha256"
sha256sum target/lkjscript seed/lkjscript.gz > "$out/producer-seed-after.sha256"
sha256sum bench/compiler-fingerprint-index-data/gate-run-1.sh > "$out/wrapper-after.sha256"
stat -c '%n %s' target/lkjscript seed/lkjscript.gz > "$out/output-sizes"
cmp "$out/inputs-before.sha256" "$out/inputs-after.sha256"
cmp "$out/wrapper-before.sha256" "$out/wrapper-after.sha256"
test "$(sha256sum seed/lkjscript.gz | cut -d ' ' -f 1)" = c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae
mkdir "$out/logs"
rg --files --no-ignore -g '*.log' target/gate | sort > "$out/log-paths"
while IFS= read -r log_path; do
    test -f "$log_path" && test ! -L "$log_path"
    cp --parents -- "$log_path" "$out/logs/"
    cmp -- "$log_path" "$out/logs/$log_path"
done < "$out/log-paths"
sha256sum "$out/stdout" "$out/stderr" "$out/time" > "$out/raw.sha256"
cat "$out/time" "$out/load-before" "$out/load-after"
tail -n 3 "$out/stdout"
exit "$gate_status"
