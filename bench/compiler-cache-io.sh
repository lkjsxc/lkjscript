#!/usr/bin/env bash
# Frozen ordinary-build attribution. No index, signature, rename or larger rung.
set -euo pipefail
cd "$(dirname "$0")/.."
repository=$PWD
if [[ $# == 2 && $1 == --cleanup ]]; then
    run=$(realpath -e "$2")
    [[ $run == "$repository"/target/compiler-cache-io-run.* && ! -L $2 ]] || exit 2
    [[ $(cat "$run/owner") == "$repository" ]] || exit 2
    pid=$(cat "$run/pid")
    [[ $pid =~ ^[1-9][0-9]*$ ]] || exit 2
    if kill -0 "$pid" 2>/dev/null; then echo 'run supervisor is alive' >&2; exit 2; fi
    ps -eo pid=,args= > "$run/process-snapshot"
    if awk -v run="$run" -v self="$$" -v parent="$PPID" \
        '$1!=self && $1!=parent && index($0,run)>0 {found=1} END {exit !found}' "$run/process-snapshot"; then
        echo 'a process still references this run' >&2; exit 2
    fi
    key_directory=$(cat "$run/key-directory")
    [[ $key_directory == /tmp/lkjscript-cache-io-key.* && ! -L $key_directory ]] || exit 2
    [[ $(cat "$key_directory/owner") == "$run" ]] || exit 2
    rm -rf -- "$run" "$key_directory"
    exit 0
fi
(($# == 4)) || { echo 'usage: compiler-cache-io.sh EMITTER BASELINE INSTRUMENTED RESULT_PREFIX' >&2; exit 2; }
emitter=$(realpath "$1")
baseline=$(realpath "$2")
instrumented=$(realpath "$3")
prefix=$(realpath -m "$4")
for binary in "$emitter" "$baseline" "$instrumented"; do [[ -x $binary ]] || exit 2; done
[[ ! -e $prefix.tsv && ! -e $prefix.raw ]] || { echo 'results already exist; no retries' >&2; exit 2; }
frozen_commit=b7e8ed035d155e75438406c7474a747220ff9537
expected_emitter=71b2fc066cd74e9e37cb3134173903c9b322b83472b9293f1a49df7c1d18f3b7
[[ $(sha256sum "$emitter" | awk '{print $1}') == "$expected_emitter" ]] || exit 2
free=$(df -Pk . | awk 'NR==2 {printf "%.0f", $4*1024}')
((free >= 8589934592)) || { echo 'requires 8 GiB free scratch space' >&2; exit 2; }
umask 077
mkdir -p "$(dirname "$prefix")"
mkdir "$prefix.raw"
run=$(mktemp -d "$repository/target/compiler-cache-io-run.XXXXXX")
key_directory=$(mktemp -d /tmp/lkjscript-cache-io-key.XXXXXX)
printf '%s\n' "$repository" > "$run/owner"
printf '%s\n' "$$" > "$run/pid"
printf '%s\n' "$key_directory" > "$run/key-directory"
printf '%s\n' "$run" > "$key_directory/owner"
head -c 32 /dev/urandom > "$key_directory/key"
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    printf 'exit_status=%s\n' "$status" >> "$prefix.meta"
    rm -rf -- "$run" "$key_directory"
    [[ ! -e $run && ! -e $key_directory ]] || { echo 'cleanup incomplete' >&2; exit 1; }
    printf 'cleanup=generated_source_store_key_and_images_removed\n' >> "$prefix.meta"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
load() { awk '{print $1","$2","$3}' /proc/loadavg; }
printf 'protocol=ordinary-cache-io-1\nsource_commit=%s\nseed=20261116\nrungs=10000,100000\norder=baseline_then_instrumented_per_rung\nphases=cold,unchanged,body_transaction,one_body,body_control\nfile_cache_reset=false\nmemory_limit_bytes=4294967296\nwatchdog_seconds=900\nstore_budget_bytes=1073741824\noutput_inline_limit=0\nproducer_inline_limit=64\nquery_index=false\nfree_preflight_bytes=%s\n' "$frozen_commit" "$free" > "$prefix.meta"
printf 'harness_sha256=%s\nprotocol_sha256=%s\n' \
    "$(sha256sum bench/compiler-cache-io.sh | awk '{print $1}')" \
    "$(sha256sum bench/compiler-cache-io.protocol.md | awk '{print $1}')" >> "$prefix.meta"
for role in emitter baseline instrumented; do
    case "$role" in emitter) binary=$emitter;; baseline) binary=$baseline;; instrumented) binary=$instrumented;; esac
    printf '%s_path=%s\n%s_sha256=%s\n%s_bytes=%s\n' "$role" "$binary" "$role" "$(sha256sum "$binary" | awk '{print $1}')" "$role" "$(wc -c < "$binary")" >> "$prefix.meta"
done
printf 'sequence\trung\trole\tphase\tstatus\twall_ms\ttime_wall_s\tuser_s\tkernel_s\trss_kib\tload_before\tload_after\tproducer_sha256\tsource_sha256\timage_sha256\timage_bytes\tstore_scope\tstore_bytes\n' > "$prefix.tsv"
printf 'rung\trole\tphase\timage_sha256\tsource_sha256\tverdict\n' > "$prefix.parity.tsv"
sequence=0
rung=setup role=setup source_digest=- image= store= store_scope=none
producer_digest=$expected_emitter
measure() {
    local phase=$1; shift
    sequence=$((sequence + 1))
    local stem before after started ended status=0 elapsed wall user kernel rss image_hash=- image_bytes=0 store_bytes=0
    stem=$(printf '%03d-%s-%s-%s' "$sequence" "$rung" "$role" "$phase")
    printf '%q ' "$@" > "$prefix.raw/$stem.argv"
    printf '\n' >> "$prefix.raw/$stem.argv"
    before=$(load)
    started=$(date +%s%N)
    printf 'sequence=%s\nrung=%s\nrole=%s\nphase=%s\nstarted_ns=%s\nload_before=%s\n' \
        "$sequence" "$rung" "$role" "$phase" "$started" "$before" > "$prefix.raw/$stem.checkpoint"
    /usr/bin/time -f '%e %U %S %M' -o "$prefix.raw/$stem.time" \
        prlimit --as=4294967296 -- timeout --kill-after=10s 900s "$@" \
        > "$prefix.raw/$stem.stdout" 2> "$prefix.raw/$stem.stderr" || status=$?
    ended=$(date +%s%N)
    after=$(load)
    elapsed=$(awk -v a="$started" -v b="$ended" 'BEGIN {printf "%.3f",(b-a)/1000000}')
    read -r wall user kernel rss < <(tail -n 1 "$prefix.raw/$stem.time")
    if [[ -n $image && -f $image ]]; then image_hash=$(sha256sum "$image" | awk '{print $1}'); image_bytes=$(wc -c < "$image"); fi
    if [[ -n $store && -d $store ]]; then store_bytes=$(du -sb "$store" | awk '{print $1}'); fi
    printf 'status=%s\nended_ns=%s\nload_after=%s\n' "$status" "$ended" "$after" >> "$prefix.raw/$stem.checkpoint"
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$sequence" "$rung" "$role" "$phase" "$status" "$elapsed" "$wall" "$user" "$kernel" "$rss" "$before" "$after" \
        "$producer_digest" "$source_digest" "$image_hash" "$image_bytes" "$store_scope" "$store_bytes" >> "$prefix.tsv"
    printf '%s %s %s status=%s wall_ms=%s load=%s->%s\n' "$rung" "$role" "$phase" "$status" "$elapsed" "$before" "$after"
    if ((status != 0)); then
        printf 'stopped_at=%s\nrefusal_status=%s\nlater_rows=not_run\n' "$stem" "$status" >> "$prefix.meta"
        tail -20 "$prefix.raw/$stem.stderr" >&2
        return "$status"
    fi
    last_stem=$stem
}
source_manifest() {
    local destination=$1
    (cd "$tree"; rg --files --hidden --no-ignore -g '*.lkj' | LC_ALL=C sort | xargs -r -d '\n' sha256sum) > "$destination"
    source_digest=$(sha256sum "$destination" | awk '{print $1}')
}
mkdir "$run/frozen"
git archive "$frozen_commit" src library | tar -xf - -C "$run/frozen"
printf 'source_tree=%s\nlibrary_tree=%s\nruntime_sha256=%s\n' \
    "$(git rev-parse "$frozen_commit:src")" "$(git rev-parse "$frozen_commit:library")" \
    "$(sha256sum "$run/frozen/library/runtime.lkj" | awk '{print $1}')" >> "$prefix.meta"
cp -R "$run/frozen/src" "$run/analysis"
cp bench/scale.lkj "$run/analysis/main.lkj"
printf 'generator_source_sha256=%s\n' "$(sha256sum bench/scale.lkj | awk '{print $1}')" >> "$prefix.meta"
image="$run/scale"
measure generator_build "$emitter" build analysis/main.lkj --source-dir "$run" \
    --library-dir "$run/frozen/library" --runtime-source "$run/frozen/library/runtime.lkj" \
    --inline-limit 0 --no-store --stats -o "$image"
producer_digest=$(sha256sum "$image" | awk '{print $1}')
image=
measure census "$run/scale" shape "$run/frozen"
cp "$prefix.raw/$last_stem.stdout" "$prefix.shape"
printf 'shape_sha256=%s\n' "$(sha256sum "$prefix.shape" | awk '{print $1}')" >> "$prefix.meta"
cat > "$run/body.json" <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"edit::target","source":"{\nvalue ^ 2u64\n}"}]}
JSON
cp "$run/body.json" "$prefix.body-request.json"
for rung in 10000 100000; do
    free=$(df -Pk "$run" | awk 'NR==2 {printf "%.0f",$4*1024}')
    ((free >= 8589934592)) || { printf 'disk_refusal_rung=%s\n' "$rung" >> "$prefix.meta"; exit 1; }
    tree="$run/source"
    mkdir "$tree"
    role=setup
    producer_digest=$(sha256sum "$run/scale" | awk '{print $1}')
    source_digest=- image= store= store_scope=none
    measure generate "$run/scale" generate "$tree" "$rung" 20261116 < "$prefix.shape"
    cp "$prefix.raw/$last_stem.stdout" "$prefix.$rung.workload"
    actual_lines=$(find "$tree" -name '*.lkj' -type f -exec cat {} + | wc -l)
    ((actual_lines == rung)) || { echo 'generated line count differs' >&2; exit 1; }
    source_manifest "$prefix.$rung.pristine.sha256"
    pristine_digest=$source_digest
    cp -R "$tree" "$run/pristine"
    for role in baseline instrumented; do
        case "$role" in baseline) compiler=$baseline;; instrumented) compiler=$instrumented;; esac
        rm -rf -- "$tree"
        cp -R "$run/pristine" "$tree"
        source_digest=$pristine_digest
        producer_digest=$(sha256sum "$compiler" | awk '{print $1}')
        store="$run/store-$role"
        store_scope=ordinary
        operation_store="$run/operation-store-$role"
        mkdir "$store" "$operation_store"
        image="$run/image"
        rm -f "$image"
        build() {
            local phase=$1; shift
            measure "$phase" "$compiler" build main.lkj --source-dir "$tree" \
                --library-dir "$run/frozen/library" --runtime-source "$run/frozen/library/runtime.lkj" \
                --inline-limit 0 --stats --store "$store" --store-key "$key_directory/key" \
                --store-bytes 1073741824 -o "$image" "$@"
        }
        build cold
        if [[ $role == baseline ]]; then cp "$image" "$run/cold-expected"; else cmp "$run/cold-expected" "$image"; fi
        printf '%s\t%s\tcold\t%s\t%s\texact\n' "$rung" "$role" "$(sha256sum "$image" | awk '{print $1}')" "$source_digest" >> "$prefix.parity.tsv"
        build unchanged
        cmp "$run/cold-expected" "$image"
        image=
        ordinary_store=$store
        store=$operation_store
        store_scope=operation
        producer_digest=$expected_emitter
        measure body_transaction "$emitter" edit main.lkj --source-dir "$tree" \
            --library-dir "$run/frozen/library" --runtime-source "$run/frozen/library/runtime.lkj" \
            --inline-limit 0 --stats --store "$operation_store" --store-key "$key_directory/key" \
            --store-bytes 1073741824 --request "$run/body.json"
        store=$ordinary_store
        store_scope=ordinary
        source_manifest "$prefix.$rung.$role.body.sha256"
        if [[ $role == baseline ]]; then body_digest=$source_digest; else [[ $source_digest == "$body_digest" ]]; fi
        image="$run/image"
        producer_digest=$(sha256sum "$compiler" | awk '{print $1}')
        build one_body
        if cmp -s "$run/cold-expected" "$image"; then echo 'body edit left image unchanged' >&2; exit 1; fi
        if [[ $role == baseline ]]; then cp "$image" "$run/body-expected"; else cmp "$run/body-expected" "$image"; fi
        printf '%s\t%s\tone_body\t%s\t%s\texact\n' "$rung" "$role" "$(sha256sum "$image" | awk '{print $1}')" "$source_digest" >> "$prefix.parity.tsv"
        ordinary_store=$store
        store=
        store_scope=none
        measure body_control "$compiler" build main.lkj --source-dir "$tree" \
            --library-dir "$run/frozen/library" --runtime-source "$run/frozen/library/runtime.lkj" \
            --inline-limit 0 --stats --no-store -o "$image"
        store=$ordinary_store
        store_scope=ordinary
        cmp "$run/body-expected" "$image"
        printf '%s\t%s\tbody_control\t%s\t%s\texact\n' "$rung" "$role" "$(sha256sum "$image" | awk '{print $1}')" "$source_digest" >> "$prefix.parity.tsv"
        rm -rf -- "$store" "$operation_store" "$image"
    done
    rm -rf -- "$tree" "$run/pristine" "$run/cold-expected" "$run/body-expected"
done
printf 'completed_rows=%s\nparity=all_emitted_outputs_exact_between_producers_and_no_store_controls\n' "$sequence" >> "$prefix.meta"
