#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
source tools/fixture-bytes.sh

work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT

# Generate complete reference files with a binary prefix and every final byte.
LC_ALL=C awk -v dir="$work" 'BEGIN {
    prefix = sprintf("fixture%c\r\n%c", 0, 255)
    for (byte = 0; byte < 256; byte++) {
        path = dir "/" byte
        printf "%s%c", prefix, byte > path
        if (close(path) != 0) exit 1
    }
}'
for ((byte = 0; byte < 256; byte++)); do
    # Flipping the low bit exchanges the even and odd members of each pair.
    expected=$((byte + (byte % 2 == 0 ? 1 : -1)))
    cp "$work/$byte" "$work/edited"
    fixture_flip_last_byte "$work/edited"
    cmp "$work/$expected" "$work/edited"
    fixture_flip_last_byte "$work/edited"
    cmp "$work/$byte" "$work/edited"
done

touch "$work/empty"
if fixture_flip_last_byte "$work/empty" 2> "$work/empty.err"; then
    printf 'empty fixture was accepted\n' >&2
    exit 1
fi
grep -Fxq "fixture has no final byte: $work/empty" "$work/empty.err"
cmp /dev/null "$work/empty"
printf 'ok fixture byte flips: all 256 values, full-file restoration and empty-file refusal\n'
