#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
mkdir -p target
bootstrap_store="${LKJSCRIPT_STORE:-target/store}"
bootstrap_key="${LKJSCRIPT_STORE_KEY:-$(sh tools/store-key.sh || true)}"
mkdir -p "$bootstrap_store"
set --
if [ -n "$bootstrap_key" ]; then set -- --store "$bootstrap_store" --store-key "$bootstrap_key"; fi
bootstrap_directory=$(mktemp -d target/bootstrap.XXXXXX)
trap 'rm -rf "$bootstrap_directory"' EXIT HUP INT TERM
gzip -dc seed/lkjscript.gz > "$bootstrap_directory/lkjscript"
chmod +x "$bootstrap_directory/lkjscript"
"$bootstrap_directory/lkjscript" build src/main.rs --library-dir library --runtime-source library/runtime.rs -o target/lkjscript "$@"
target/lkjscript build src/main.rs --library-dir library --runtime-source library/runtime.rs -o "$bootstrap_directory/reproduced" "$@"
# A stored seed may predate a sealed-runtime change while still accepting all
# compiler source. Require two consecutive current-emitter generations to agree;
# a bridging generation does not refresh the seed or waive the fixed point.
if ! cmp -s target/lkjscript "$bootstrap_directory/reproduced"; then
    read -r bootstrap_load_before bootstrap_other < /proc/loadavg
    bootstrap_started=$(date +%s%N)
    "$bootstrap_directory/reproduced" build src/main.rs --library-dir library --runtime-source library/runtime.rs -o "$bootstrap_directory/confirmed" "$@"
    cmp "$bootstrap_directory/reproduced" "$bootstrap_directory/confirmed"
    cp "$bootstrap_directory/reproduced" target/lkjscript
    read -r bootstrap_load_after bootstrap_other < /proc/loadavg
    printf 'ok bootstrap seed bridge (%s ms, load %s -> %s)\n' "$((($(date +%s%N)-bootstrap_started)/1000000))" "$bootstrap_load_before" "$bootstrap_load_after"
fi
target/lkjscript check-library --library-dir library "$@"
printf 'ok bootstrap\n'
