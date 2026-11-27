#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
mkdir -p target
# --handoff runs a local consumer while the private fixed-point record is live.
handoff=
if [ "$#" -ne 0 ]; then
    [ "$1" = --handoff ] && [ "$#" -gt 1 ] || { echo "usage: bootstrap.sh [--handoff command ...]" >&2; exit 1; }
    shift
    handoff=1
fi
# Measured producer policy; ordinary program builds still default to zero.
bootstrap_inline_limit=64
bootstrap_store="${LKJSCRIPT_STORE:-target/store}"
bootstrap_key="${LKJSCRIPT_STORE_KEY:-$(sh tools/store-key.sh || true)}"
mkdir -p "$bootstrap_store"
bootstrap_directory=$(mktemp -d target/bootstrap.XXXXXX)
trap 'rm -rf "$bootstrap_directory"' EXIT
trap 'exit 1' HUP INT TERM
if [ -n "$handoff" ]; then
    before=$(bash tools/bootstrap-handoff.sh inputs "$bootstrap_key" "$bootstrap_store")
fi
bootstrap() {
    set --
    if [ -n "$bootstrap_key" ]; then set -- --store "$bootstrap_store" --store-key "$bootstrap_key"; fi
    gzip -dc seed/lkjscript.gz > "$bootstrap_directory/lkjscript"
    chmod +x "$bootstrap_directory/lkjscript"
    "$bootstrap_directory/lkjscript" build src/main.lkj --library-dir library --runtime-source library/runtime.lkj --inline-limit "$bootstrap_inline_limit" -o target/lkjscript "$@"
    target/lkjscript build src/main.lkj --library-dir library --runtime-source library/runtime.lkj --inline-limit "$bootstrap_inline_limit" -o "$bootstrap_directory/reproduced" "$@"
    # A stored seed may predate a sealed-runtime change while still accepting all
    # compiler source. Require two consecutive current-emitter generations to agree;
    # a bridging generation does not refresh the seed or waive the fixed point.
    if ! cmp -s target/lkjscript "$bootstrap_directory/reproduced"; then
        read -r bootstrap_load_before bootstrap_other < /proc/loadavg
        bootstrap_started=$(date +%s%N)
        "$bootstrap_directory/reproduced" build src/main.lkj --library-dir library --runtime-source library/runtime.lkj --inline-limit "$bootstrap_inline_limit" -o "$bootstrap_directory/confirmed" "$@"
        cmp "$bootstrap_directory/reproduced" "$bootstrap_directory/confirmed"
        cp "$bootstrap_directory/reproduced" target/lkjscript
        read -r bootstrap_load_after bootstrap_other < /proc/loadavg
        printf 'ok bootstrap seed bridge (%s ms, load %s -> %s)\n' "$((($(date +%s%N)-bootstrap_started)/1000000))" "$bootstrap_load_before" "$bootstrap_load_after"
    fi
    target/lkjscript check-library --library-dir library "$@"
    printf 'ok bootstrap\n'
}
bootstrap
if [ -n "$handoff" ]; then
    after=$(bash tools/bootstrap-handoff.sh inputs "$bootstrap_key" "$bootstrap_store")
    [ "$before" = "$after" ] || { echo 'FAIL bootstrap handoff: changed inputs during bootstrap' >&2; exit 1; }
    cmp target/lkjscript "$bootstrap_directory/reproduced"
    compiler_digest=$(sha256sum "$bootstrap_directory/reproduced")
    owner=$(bash tools/bootstrap-handoff.sh owner "$$")
    compiler_digest=${compiler_digest%% *}
    receipt="$bootstrap_directory/handoff"
    (umask 077; printf 'lkjscript-bootstrap-handoff-v2\n%s\n%s\n%s\n' "$owner" "$after" "$compiler_digest" > "$receipt")
    LKJSCRIPT_BOOTSTRAP_HANDOFF="$receipt" LKJSCRIPT_STORE="$bootstrap_store" LKJSCRIPT_STORE_KEY="$bootstrap_key" "$@"
    [ ! -e "$receipt" ] || { echo 'FAIL bootstrap handoff: receipt was not consumed' >&2; exit 1; }
fi
