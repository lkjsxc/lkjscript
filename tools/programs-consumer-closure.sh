#!/usr/bin/env bash
# consumer_gate_closure: pure HTTP/manifest contracts are closed native and
# interpreter images. Real listener/filesystem observations still execute in
# http-test.sh and program-directory-test.sh against fresh state.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
work=$(mktemp -d target/programs-consumer-closure.XXXXXX)
active=
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if [[ -n "$active" ]]; then kill "$active" 2>/dev/null || :; wait "$active" 2>/dev/null || :; fi
    rm -rf "$work"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
run() {
    local status=0
    "$@" & active=$!
    wait "$active" || status=$?
    active=
    return "$status"
}
umask 077
mkdir "$work/store"
head -c 32 /dev/urandom >"$work/key"
export LKJSCRIPT_STORE="$work/store"
export LKJSCRIPT_STORE_KEY="$work/key"
# As in gate-reuse-test.sh, change executable producer identity without changing
# its semantics. Every case is freshly checked/emitted by the renewed producer.
# This is a producer-renewal proof, not a compiler rebuild benchmark.
cp "$compiler" "$work/compiler"
cp "$work/compiler" "$work/compiler-next"
printf 'programs closed consumer producer renewal\n' >>"$work/compiler-next"
read -r original _ < <(sha256sum "$work/compiler")
read -r renewed _ < <(sha256sum "$work/compiler-next")
[[ "$original" != "$renewed" ]]
read -r before _ </proc/loadavg
started=$(date +%s%N)
total=0
for source in library/http_protocol.lkj library/http_policy.lkj tools/http_client.lkj src/programs_directory_cases.lkj; do
    run bash tools/programs-unit-cases.sh "$work/compiler" "$source" >"$work/cold" 2>&1 || { cat "$work/cold" >&2; exit 1; }
    summary=$(rg '^LKJSCRIPT_UNIT_GATE ' "$work/cold")
    [[ $(printf '%s\n' "$summary" | wc -l) -eq 1 ]] || { cat "$work/cold" >&2; exit 1; }
    read -r tag cases observations cached fresh rest <<<"$summary"
    [[ "$cases" =~ ^cases=[1-9][0-9]*$ && "$observations" =~ ^observations=[1-9][0-9]*$ ]] \
        || { cat "$work/cold" >&2; exit 1; }
    count=${cases#cases=}
    expected=$((count * 2))
    [[ "$observations" == "observations=$expected" && "$cached" == reused=0 && "$fresh" == "rerun=$expected" ]] \
        || { cat "$work/cold" >&2; exit 1; }
    run bash tools/programs-unit-cases.sh "$work/compiler-next" "$source" >"$work/renewed" 2>&1 || { cat "$work/renewed" >&2; exit 1; }
    expected_summary="LKJSCRIPT_UNIT_GATE cases=$count observations=$expected reused=$expected rerun=0 source=$source"
    [[ $(rg '^LKJSCRIPT_UNIT_GATE ' "$work/renewed") == "$expected_summary" ]] \
        || { cat "$work/renewed" >&2; exit 1; }
    cat "$work/renewed"
    total=$((total + expected))
done
read -r after _ </proc/loadavg
elapsed=$(($(date +%s%N) - started))
awk -v ns="$elapsed" -v before="$before" -v after="$after" -v total="$total" \
    'BEGIN { printf "ok consumer_gate_closure: %d native/interpreter observations reused across renewed producer; zero reruns; live HTTP/filesystem observations stay fresh (%.3f s, load %s -> %s)\n", total, ns/1000000000, before, after }'
