#!/usr/bin/env bash
# The published producer is extracted from its exact seed; no changed library is used.
set -euo pipefail
cd "$(dirname "$0")/../../.."
work=target/anonymous-baseline
mkdir -p "$work"
git show ca749bd44e1cece139fe5f875adc57b27bcfadd8:seed/lkjscript.gz > "$work/ca749bd44-lkjscript.gz"
gzip -dc "$work/ca749bd44-lkjscript.gz" > "$work/lkjscript"
chmod +x "$work/lkjscript"
for source in main stdout; do
    if "$work/lkjscript" build "bench/anonymous-files-data/baseline/$source.lkj" --no-store \
        --grant "$([[ $source == main ]] && printf storage || printf output)" \
        -o "$work/refused-$source" > "$work/$source.out" 2> "$work/$source.err"; then
        printf 'baseline unexpectedly admitted %s\n' "$source" >&2
        exit 1
    fi
    [[ ! -e $work/refused-$source ]]
    cat "$work/$source.err"
done
sha256sum "$work/ca749bd44-lkjscript.gz" "$work/lkjscript"
