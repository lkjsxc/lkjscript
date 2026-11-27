#!/usr/bin/env bash
# Run only in a disposable, otherwise idle checkout:
# sh tools/bootstrap.sh --handoff bash tools/bootstrap-handoff-test.sh
set -euo pipefail
cd "$(dirname "$0")/.."
: "${LKJSCRIPT_BOOTSTRAP_HANDOFF:?run through bootstrap.sh --handoff}"
receipt=$LKJSCRIPT_BOOTSTRAP_HANDOFF
directory=${receipt%/*}
cmp target/lkjscript "$directory/reproduced"
if [[ -f $directory/confirmed ]]; then cmp target/lkjscript "$directory/confirmed"; fi
printf 'ok fixed-point compiler identity: '
sha256sum target/lkjscript

restore() {
    if [[ -n ${mutated:-} ]]; then
        truncate -s "$size" "$mutated"
        mutated=
    fi
}
trap restore EXIT
trap 'exit 1' HUP INT TERM
for file in src/main.lkj library/runtime.lkj seed/lkjscript.gz target/lkjscript; do
    [[ $(stat -c %h "$file") == 1 ]] || { echo "refuse shared inode: $file" >&2; exit 1; }
    size=$(stat -c %s "$file")
    mutated=$file
    printf '\n' >> "$file"
    if bash tools/check.sh >"$directory/refusal.log" 2>&1; then
        echo "FAIL accepted changed $file" >&2
        exit 1
    fi
    grep -F 'FAIL bootstrap handoff: changed inputs or compiler identity' "$directory/refusal.log"
    [[ -f $receipt ]]
    restore
    printf 'ok refusal: %s\n' "$file"
done
# A record from a different producer lifetime cannot become a local cache.
cp "$receipt" "$directory/receipt-original"
sed -i "2s/^[0-9][0-9]*/1/" "$receipt"
if bash tools/check.sh >"$directory/refusal.log" 2>&1; then
    echo "FAIL accepted non-ancestor receipt" >&2; exit 1
fi
grep -F "FAIL bootstrap handoff: not a live same-run bootstrap ancestor" "$directory/refusal.log"
mv "$directory/receipt-original" "$receipt"
printf "ok refusal: stale producer lifetime\n"
preamble=$(awk '{ print } /^check_binary=target\/lkjscript$/ { found=1; exit } END { if (!found) exit 1 }' tools/check.sh)
bash -c "$preamble" tools/check.sh
if bash tools/bootstrap-handoff.sh verify "$receipt" "$LKJSCRIPT_STORE_KEY" "$LKJSCRIPT_STORE"; then
    echo 'FAIL receipt replay accepted' >&2
    exit 1
fi
printf 'ok single-use receipt\n'
