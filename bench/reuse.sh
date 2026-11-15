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
        "$compiler" build src/main.rs --source-dir "$snapshot" \
        --library-dir "$snapshot/library" --runtime-source "$snapshot/library/runtime.rs" \
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
' "$snapshot/src/check.rs" > "$bench_directory/check.rs"
mv "$bench_directory/check.rs" "$snapshot/src/check.rs"
measure checker-edit --store "$store" --store-key "$key"
measure checker-edit-no-store --no-store
cmp "$bench_directory/checker-edit.image" "$bench_directory/checker-edit-no-store.image"
if cmp -s "$bench_directory/no-store.image" "$bench_directory/checker-edit.image"; then
    echo "checker body edit did not change emitted bytes" >&2
    exit 1
fi
awk -v baseline="$baseline_elapsed" -v cold="$cold_elapsed" 'BEGIN {
    if (baseline > 0) printf "cold_overhead_percent=%.2f\n", 100 * (cold / baseline - 1)
}'
printf 'reuse: all unchanged and changed cached images match their no-store images\n'
