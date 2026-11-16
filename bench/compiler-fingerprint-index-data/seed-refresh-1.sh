#!/usr/bin/env bash
set -euo pipefail
repo=/home/coder/workspace/lkjscript
out="$repo/bench/compiler-fingerprint-index-data/seed-1"
script="$repo/bench/compiler-fingerprint-index-data/seed-refresh-1.sh"
candidate="$repo/seed/compiler-fingerprint-index-seed.tmp"
cd "$repo"
export LC_ALL=C TZ=UTC
if [[ "${1-}" == publish ]]; then
    test ! -e "$candidate" && test ! -L "$candidate"
    test "$(sha256sum target/lkjscript | cut -d ' ' -f 1)" = a92e81e2d2c18fe1bd645a9b5e287c3a1a2af5eaffa1be16e400289590e7625a
    test "$(sha256sum seed/lkjscript.gz | cut -d ' ' -f 1)" = c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae
    (set -o noclobber; gzip -n -9 -c target/lkjscript > "$candidate")
    gzip -n -9 -c target/lkjscript | cmp - "$candidate"
    gzip -dc "$candidate" | cmp - target/lkjscript
    candidate_sha=$(sha256sum "$candidate" | cut -d ' ' -f 1)
    printf '%s\n' "$candidate_sha" > "$out/candidate.sha256"
    chmod 644 "$candidate"
    mv -T -- "$candidate" seed/lkjscript.gz
    test "$(sha256sum seed/lkjscript.gz | cut -d ' ' -f 1)" = "$candidate_sha"
    test "$(sha256sum target/lkjscript | cut -d ' ' -f 1)" = a92e81e2d2c18fe1bd645a9b5e287c3a1a2af5eaffa1be16e400289590e7625a
    exit 0
fi
test -z "${1-}"
test ! -e "$out" && test ! -L "$out"
test "$(git rev-parse HEAD:src)" = 59fcc8447761f00edaae5dacefbf5edb9b23df36
git diff --quiet -- src library tests tools
mkdir -m 700 "$out"
printf '%s\n' "$repo" > "$out/cwd"
printf 'LC_ALL=C\nTZ=UTC\n' > "$out/environment"
printf '%s\n' 31f412a0a7df47d897b6b39e794a9b152b4fa29e > "$out/source-commit"
git rev-parse HEAD > "$out/base-commit"
sha256sum target/lkjscript seed/lkjscript.gz "$script" > "$out/inputs-before.sha256"
argv=(/usr/bin/time -o "$out/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x'
    /usr/bin/timeout --signal=TERM --kill-after=10s 900s /usr/bin/prlimit --as=4294967296 -- /bin/bash "$script" publish)
printf '%s\0' "${argv[@]}" > "$out/argv"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/start.utc"
awk '{print $1,$2,$3}' /proc/loadavg > "$out/load-before"
date +%s%N > "$out/start.ns"
set +e
"${argv[@]}" < /dev/null > "$out/stdout" 2> "$out/stderr"
refresh_status=$?
set -e
date +%s%N > "$out/end.ns"
awk '{print $1,$2,$3}' /proc/loadavg > "$out/load-after"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/end.utc"
printf '%s\n' "$refresh_status" > "$out/status"
sha256sum target/lkjscript seed/lkjscript.gz "$script" > "$out/outputs.sha256"
stat -c '%n %s %a' target/lkjscript seed/lkjscript.gz > "$out/sizes"
test ! -e "$candidate" && test ! -L "$candidate"
cat "$out/time" "$out/load-before" "$out/load-after" "$out/outputs.sha256"
exit "$refresh_status"
