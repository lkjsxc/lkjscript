#!/bin/sh
# Disposable self-build measurements; elapsed times are observations, not limits.
set -eu
cd "$(dirname "$0")/.."
root=$(pwd)
compiler=target/lkjscript
if [ "$#" -gt 0 ]; then
    if [ "$#" -ne 2 ] || [ "$1" != "--compiler" ]; then
        echo "usage: sh bench/reuse.sh [--compiler PATH]" >&2
        exit 2
    fi
    compiler=$2
fi
[ -x "$compiler" ] || { echo "compiler is not executable: $compiler" >&2; exit 1; }
umask 077
mkdir -p target
bench_directory=$(mktemp -d "$root/target/reuse-bench.XXXXXX")
trap 'rm -rf "$bench_directory"' EXIT
trap 'exit 1' HUP INT TERM
snapshot="$bench_directory/source"
store="$bench_directory/store"
mkdir "$snapshot" "$store"
cp -R src library "$snapshot/"
key=$(XDG_STATE_HOME="$bench_directory/authority" sh tools/store-key.sh)
load() { awk '{ print $1 "," $2 "," $3 }' /proc/loadavg; }

measure() {
    label=$1
    shift
    before=$(load)
    if /usr/bin/time -f '%e %M' -o "$bench_directory/$label.time" \
        "$compiler" build src/main.lkj --source-dir "$snapshot" \
        --library-dir "$snapshot/library" --runtime-source "$snapshot/library/runtime.lkj" \
        --stats -o "$bench_directory/$label.image" "$@" \
        > "$bench_directory/$label.out" 2> "$bench_directory/$label.log"; then
        :
    else
        cat "$bench_directory/$label.log" >&2
        return 1
    fi
    after=$(load)
    read -r elapsed rss < "$bench_directory/$label.time"
    store_bytes=$(du -sb "$store" | awk '{ print $1 }')
    printf '%s elapsed_s=%s peak_rss_kib=%s load_before=%s load_after=%s store_bytes=%s\n' \
        "$label" "$elapsed" "$rss" "$before" "$after" "$store_bytes"
    sed -n '/^LKJSCRIPT_STATS /p' "$bench_directory/$label.log"
}

printf 'compiler=%s (load averages are 1,5,15 minutes; store bytes include directories)\n' "$compiler"
measure no-store --no-store
baseline_elapsed=$elapsed
measure cold --store "$store" --store-key "$key"
cold_elapsed=$elapsed
cmp "$bench_directory/no-store.image" "$bench_directory/cold.image"
measure warm --store "$store" --store-key "$key"
cmp "$bench_directory/no-store.image" "$bench_directory/warm.image"

# Keep source locations stable: change only the first diagnostic column in error().
awk '
    /^fn error\(/ { inside = 1 }
    inside && !changed && /column: 1,/ {
        sub(/column: 1,/, "column: 2,")
        changed = 1
    }
    { print }
    /^}/ { inside = 0 }
    END { if (!changed) exit 1 }
' "$snapshot/src/check.lkj" > "$bench_directory/check.lkj"
mv "$bench_directory/check.lkj" "$snapshot/src/check.lkj"
measure checker-edit --store "$store" --store-key "$key"
measure checker-edit-no-store --no-store
cmp "$bench_directory/checker-edit.image" "$bench_directory/checker-edit-no-store.image"
if cmp -s "$bench_directory/no-store.image" "$bench_directory/checker-edit.image"; then
    echo "checker body edit did not change emitted bytes" >&2
    exit 1
fi
# The two actual compiler images must also run and agree on every closed gate
# root. Both producers compile the same live-worktree semantic inputs here;
# this is a producer-change proof, not reuse after editing the semantic inputs.
# Build afresh with each producer; only execution proofs share the store.
closed_before="$bench_directory/closed-before"
closed_after="$bench_directory/closed-after"
closed_store="$bench_directory/closed-store"
mkdir "$closed_store"
proof_load_before=$(load)
proof_started_ns=$(date +%s%N)
LKJSCRIPT_STORE_KEY= bash tools/closed-semantics.sh "$bench_directory/no-store.image" build "$closed_before"
LKJSCRIPT_STORE_KEY= bash tools/closed-semantics.sh "$bench_directory/checker-edit.image" build "$closed_after"
for artifact in runner cases cases.authority structure-main structure-main.authority \
    structure-source_0 structure-source_0.authority structure-source_1 structure-source_1.authority \
    structure-source_2 structure-source_2.authority structure-source_3 structure-source_3.authority; do
    cmp "$closed_before/$artifact" "$closed_after/$artifact"
done
for generation in before after; do
    if [ "$generation" = before ]; then
        proof_compiler="$bench_directory/no-store.image"
        proof_directory="$closed_before"
    else
        proof_compiler="$bench_directory/checker-edit.image"
        proof_directory="$closed_after"
    fi
    # Populate sequentially so this exact reuse assertion does not assume that
    # every nonblocking concurrent writer wins. The second pass is read-only
    # and uses the ordinary four-worker structural schedule.
    proof_jobs=4
    if [ "$generation" = before ]; then proof_jobs=1; fi
    for action in run structure; do
        if ! LKJSCRIPT_GATE_JOBS="$proof_jobs" LKJSCRIPT_STORE="$closed_store" LKJSCRIPT_STORE_KEY="$key" \
            bash tools/closed-semantics.sh "$proof_compiler" "$action" "$proof_directory" \
            >> "$bench_directory/closed-$generation.log" 2>&1; then
            cat "$bench_directory/closed-$generation.log" >&2
            exit 1
        fi
    done
done
# Six exact images cover 28 independent + 185 core + four 134-source shards.
proof_counts() {
    awk -v expected_reused="$1" -v expected_rerun="$2" '
        /^LKJSCRIPT_CLOSED_GATE / {
            records++
            for (i=1; i<=NF; i++) {
                if ($i ~ /^reused=/) { value=$i; sub(/^reused=/, "", value); reused += value }
                if ($i ~ /^rerun=/) { value=$i; sub(/^rerun=/, "", value); rerun += value }
            }
        }
        END { exit !(records == 6 && reused == expected_reused && rerun == expected_rerun) }
    ' "$3"
}
if ! proof_counts 0 749 "$bench_directory/closed-before.log" \
    || ! proof_counts 749 0 "$bench_directory/closed-after.log"; then
    cat "$bench_directory/closed-before.log" "$bench_directory/closed-after.log" >&2
    echo "checker body edit did not reuse the identical closed gate artifacts" >&2
    exit 1
fi
proof_elapsed_ns=$(($(date +%s%N)-proof_started_ns))
proof_load_after=$(load)
awk -v elapsed_ns="$proof_elapsed_ns" -v before="$proof_load_before" -v after="$proof_load_after" 'BEGIN {
    printf "checker-producer-edit-closed-proof elapsed_s=%.3f load_before=%s load_after=%s reused_before=0 reused_after=749 independent=28 structure=721\n", elapsed_ns / 1000000000, before, after
}'
cat "$bench_directory/closed-before.log" "$bench_directory/closed-after.log"
awk -v baseline="$baseline_elapsed" -v cold="$cold_elapsed" 'BEGIN {
    if (baseline > 0) printf "cold_overhead_percent=%.2f\n", 100 * (cold / baseline - 1)
}'
printf 'reuse: all unchanged and changed cached images match their no-store images\n'
