#!/usr/bin/env bash
# Optional independent OS witness; not a production host or default gate dependency.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=target/lkjscript
part=all
while (($#)); do
    case "$1" in
        --compiler) compiler=$2; shift 2 ;;
        --part) part=$2; shift 2 ;;
        *) printf 'usage: %s [--compiler PATH] [--part all|image|legacy]\n' "$0" >&2; exit 2 ;;
    esac
done
case "$part" in all|image|legacy) ;; *) printf 'unknown witness part\n' >&2; exit 2 ;; esac
work=$(mktemp -d "$PWD/target/programs-process-witness.XXXXXX")
cleanup() {
    local code=$?
    trap - EXIT
    if ((code)); then
        for file in "$work/build.err" "$work/authority"; do [[ ! -f "$file" ]] || cat "$file" >&2; done
    fi
    rm -rf -- "$work"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
before=$(cat /proc/loadavg)
started=$(date +%s%N)
# A concurrent compiler rebuild must not change admission producer halfway through.
cp "$compiler" "$work/compiler"
compiler="$work/compiler"
cc -std=c11 -O2 -Wall -Wextra -Werror tools/programs-process-witness.c -o "$work/witness"
if [[ "$part" != legacy ]]; then
    head -c 32 /dev/urandom >"$work/key"
    "$compiler" build-child tests/fixtures/authority/programs_channel_receiver.lkj --library-dir library --no-store --image-key "$work/key" -o "$work/receiver" >"$work/authority" 2>"$work/build.err"
    manifest=$(awk '{for(i=1;i<=NF;i++) if($i ~ /^manifest=/) {sub(/^manifest=/,"",$i); print $i}}' "$work/authority")
    [[ "$manifest" =~ ^[0-9a-f]{64}$ ]]
    "$compiler" build tests/fixtures/authority/programs_spawn_deadline.lkj --library-dir library --no-store --grant "image=$work/receiver" --grant clock --image-key "$work/key" -o "$work/deadline-parent" 2>"$work/build.err"
    "$work/witness" spawn-deadline "$work/deadline-parent" "$work"
    "$work/witness" ancillary "$work/receiver" "$manifest"
fi
if [[ "$part" != image ]]; then
    "$compiler" build tests/fixtures/world/programs_legacy_parent.lkj --library-dir library --no-store -o "$work/legacy-parent" 2>"$work/build.err"
    "$work/witness" legacy-death "$work/legacy-parent" "$work"
    test -f "$work/child-body"
    test ! -s "$work/child-body"
    "$work/witness" ambient "$work/legacy-parent" "$work"
    test "$(cat "$work/child-body")" = 'body executed'
fi
printf 'process independent witness %s: %s ms; load %s -> %s\n' "$part" "$((($(date +%s%N)-started)/1000000))" "$before" "$(cat /proc/loadavg)"
