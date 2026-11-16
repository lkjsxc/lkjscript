#!/usr/bin/env bash
# Run the predeclared small ABBA read-contract observations, once.
set -euo pipefail
cd "$(dirname "$0")/.."
(($# == 3)) || { echo 'usage: compiler-cache-reads.sh BASELINE CANDIDATE OUTPUT_PREFIX' >&2; exit 2; }
baseline=$(realpath "$1")
candidate=$(realpath "$2")
prefix=$(realpath -m "$3")
[[ ! -e $prefix.tsv && ! -e $prefix.raw ]] || { echo 'refusing to overwrite observations' >&2; exit 2; }
[[ -x /usr/bin/time ]] || { echo 'this local observation needs GNU time' >&2; exit 2; }
free=$(df -Pk target | awk 'NR==2 { print $4 }')
((free >= 262144)) || { echo 'disk preflight refused' >&2; exit 2; }
fixture=$(mktemp -d target/compiler-cache-reads-observation.XXXXXX)
fixture=$(realpath "$fixture")
mkdir -p "$prefix.raw"
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    printf 'final_status=%s\n' "$status" >> "$prefix.meta"
    rm -rf -- "$fixture"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
umask 077
mkdir "$fixture/source" "$fixture/baseline-store" "$fixture/candidate-store"
head -c 32 /dev/urandom > "$fixture/key"
printf 'fn answer() -> i64 { 42i64 }\nfn main() -> i64 { answer() }\n' > "$fixture/source/main.lkj"
cp "$fixture/source/main.lkj" "$prefix.source.lkj"
printf 'protocol_sha256=%s\nbaseline_sha256=%s\ncandidate_sha256=%s\nbaseline_bytes=%s\ncandidate_bytes=%s\nsource_sha256=%s\nruntime_sha256=%s\nfixture=%s\nlimit_bytes=4294967296\ntimeout_seconds=900\norder=baseline,candidate,candidate,baseline\n' \
    "$(sha256sum bench/compiler-cache-reads.protocol.md | cut -d' ' -f1)" \
    "$(sha256sum "$baseline" | cut -d' ' -f1)" "$(sha256sum "$candidate" | cut -d' ' -f1)" \
    "$(stat -c '%s' "$baseline")" "$(stat -c '%s' "$candidate")" \
    "$(sha256sum "$fixture/source/main.lkj" | cut -d' ' -f1)" \
    "$(sha256sum library/runtime.lkj | cut -d' ' -f1)" "$fixture" > "$prefix.meta"
printf 'sequence\tphase\trole\texit\twall_ns\twall_s\tuser_s\tsystem_s\tpeak_kib\tload_before\tload_after\tstdout_sha256\tstore_files\tstore_bytes\n' > "$prefix.tsv"
sequence=0
last=
measure() {
    local phase=$1 role=$2 expected=$3 directory=$4; shift 4
    sequence=$((sequence+1))
    local stem before after started ended status wall user system rss files bytes
    printf -v stem '%02d-%s-%s' "$sequence" "$phase" "$role"
    before=$(awk '{print $1 " " $2 " " $3}' /proc/loadavg)
    started=$(date +%s%N)
    { printf 'load_before=%s\nstarted_ns=%s\nargv=' "$before" "$started"; printf ' %q' "$@"; printf '\n'; } > "$prefix.raw/$stem.meta"
    set +e
    /usr/bin/time -f '%e %U %S %M' -o "$prefix.raw/$stem.time" \
        prlimit --as=4294967296 -- timeout --kill-after=10s 900s "$@" \
        > "$prefix.raw/$stem.stdout" 2> "$prefix.raw/$stem.stderr"
    status=$?
    set -e
    ended=$(date +%s%N)
    after=$(awk '{print $1 " " $2 " " $3}' /proc/loadavg)
    read -r wall user system rss < <(tail -1 "$prefix.raw/$stem.time")
    files=0 bytes=0
    if [[ -n $directory ]]; then
        files=$(find "$directory" -maxdepth 1 -type f | wc -l)
        bytes=$(find "$directory" -maxdepth 1 -type f -printf '%s\n' | awk '{sum += $1} END {print sum+0}')
        find "$directory" -mindepth 1 -maxdepth 1 -printf '%f\t%y\t%s\n' | LC_ALL=C sort > "$prefix.raw/$stem.files"
    fi
    printf 'status=%s\nload_after=%s\nended_ns=%s\n' "$status" "$after" "$ended" >> "$prefix.raw/$stem.meta"
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$sequence" "$phase" "$role" "$status" "$((ended-started))" "$wall" "$user" "$system" "$rss" \
        "$before" "$after" "$(sha256sum "$prefix.raw/$stem.stdout" | cut -d' ' -f1)" "$files" "$bytes" >> "$prefix.tsv"
    printf '%s %s status=%s wall_ns=%s load=%s -> %s\n' "$phase" "$role" "$status" "$((ended-started))" "$before" "$after"
    last=$stem
    [[ $status == "$expected" ]] || { printf 'unexpected status at %s\n' "$stem" >&2; return 1; }
}
common=(main.lkj --source-dir "$fixture/source" --no-library)
for role in baseline candidate; do
    executable=${!role}
    measure setup "$role" 0 "$fixture/$role-store" "$executable" build "${common[@]}" --inline-limit 0 --index --stats \
        --store "$fixture/$role-store" --store-key "$fixture/key" -o "$fixture/$role-image"
    snapshot=$(sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]*\) .*/\1/p' "$prefix.raw/$last.stdout")
    [[ ${#snapshot} == 64 ]]
    printf '%s_snapshot=%s\n' "$role" "$snapshot" >> "$prefix.meta"
    [[ $role != baseline ]] || baseline_snapshot=$snapshot
    sha256sum "$fixture/$role-image" >> "$prefix.images.sha256"
done
cmp "$fixture/baseline-image" "$fixture/candidate-image"
cp "$fixture/baseline-image" "$fixture/expected-image"
for role in baseline candidate candidate baseline; do
    executable=${!role}
    measure warm-build "$role" 0 "$fixture/$role-store" "$executable" build "${common[@]}" --inline-limit 0 --stats \
        --store "$fixture/$role-store" --store-key "$fixture/key" -o "$fixture/$role-image"
    cmp "$fixture/expected-image" "$fixture/$role-image"
    grep -Eq '"build_reused":[1-9]' "$prefix.raw/$last.stderr"
done
first_query=
for role in baseline candidate candidate baseline; do
    executable=${!role}
    measure pinned-query "$role" 0 "$fixture/baseline-store" "$executable" query "${common[@]}" \
        --store "$fixture/baseline-store" --store-key "$fixture/key" --snapshot "$baseline_snapshot" --kind outline --id main
    if [[ -z $first_query ]]; then first_query=$last; else cmp "$prefix.raw/$first_query.stdout" "$prefix.raw/$last.stdout"; fi
done
absent=0000000000000000000000000000000000000000000000000000000000000000
for role in baseline candidate candidate baseline; do
    executable=${!role}
    directory="$fixture/missing-$sequence"
    mkdir "$directory"
    measure absent-locator "$role" 1 "$directory" "$executable" query "${common[@]}" \
        --store "$directory" --store-key "$fixture/key" --snapshot "$absent" --kind meta --id crate
    [[ ! -s $prefix.raw/$last.stdout ]]
    grep -q 'E_QUERY_SNAPSHOT' "$prefix.raw/$last.stderr"
done
# Separate labeled controls after all twelve observations; no extra warmup.
measure control-no-store baseline 0 '' "$baseline" build "${common[@]}" --inline-limit 0 --no-store -o "$fixture/control-image"
cmp "$fixture/control-image" "$fixture/expected-image"
measure control-original-interpreter baseline 0 '' "$baseline" run "${common[@]}" --no-store --engine interpreter
[[ $(cat "$prefix.raw/$last.stdout") == 42 ]]
printf 'observations=12\noutput_images=identical\npinned_query_stdout=identical\ncontrol_interpreter=42\n' >> "$prefix.meta"
