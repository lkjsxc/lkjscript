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
cleanup() {
    restore
    if [[ -n ${added:-} ]]; then rm -f -- "$added"; fi
    if [[ -n ${missing_directory:-} ]]; then rmdir -- "$missing_directory"; fi
    if [[ -n ${scratch:-} ]]; then rm -rf -- "$scratch"; fi
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM
scratch=$(mktemp -d /tmp/lkjscript-handoff-test.XXXXXX)
mkdir "$scratch/bin"
# Every negative check must stop at the bootstrap step. A regression must
# not launch a real fallback bootstrap or any later timeout-wrapped suite.
cat > "$scratch/bin/timeout" <<'NO_FALLBACK'
#!/bin/sh
echo 'unexpected timeout invocation' >&2
exit 99
NO_FALLBACK
chmod +x "$scratch/bin/timeout"
expect_check_refusal() {
    local label=$1 reason=$2
    shift 2
    if PATH="$scratch/bin:$PATH" "$@" bash tools/check.sh >"$directory/refusal.log" 2>&1; then
        echo "FAIL accepted $label" >&2
        exit 1
    fi
    if grep -F 'unexpected timeout invocation' "$directory/refusal.log"; then
        echo "FAIL fell through to bootstrap or gate: $label" >&2
        exit 1
    fi
    grep -F "FAIL bootstrap handoff: $reason" "$directory/refusal.log"
    [[ -f $receipt ]]
    printf 'ok refusal: %s\n' "$label"
}
missing_directory=$(mktemp -d target/bootstrap.XXXXXX)
cp "$receipt" "$scratch/outside-receipt"
expect_check_refusal 'empty receipt' 'missing private receipt' env LKJSCRIPT_BOOTSTRAP_HANDOFF=
expect_check_refusal 'missing receipt' 'missing private receipt' env -u LKJSCRIPT_STORE -u LKJSCRIPT_STORE_KEY LKJSCRIPT_BOOTSTRAP_HANDOFF="$missing_directory/handoff"
expect_check_refusal 'outside-target receipt' 'missing private receipt' env LKJSCRIPT_BOOTSTRAP_HANDOFF="$scratch/outside-receipt"
rmdir "$missing_directory"
missing_directory=

for file in src/main.lkj library/runtime.lkj seed/lkjscript.gz target/lkjscript; do
    [[ $(stat -c %h "$file") == 1 ]] || { echo "refuse shared inode: $file" >&2; exit 1; }
    size=$(stat -c %s "$file")
    mutated=$file
    printf '\n' >> "$file"
    expect_check_refusal "$file" 'changed inputs or compiler identity' env
    restore
done
added=$(mktemp src/.bootstrap-handoff-test.XXXXXX)
expect_check_refusal 'untracked bootstrap input' 'changed inputs or compiler identity' env
rm -- "$added"
ln -s main.lkj "$added"
expect_check_refusal 'symlink bootstrap input' 'non-regular bootstrap input' env
rm -- "$added"
added=

expect_check_refusal 'different store' 'changed inputs or compiler identity' env LKJSCRIPT_STORE="$scratch/other-store"
cp "$LKJSCRIPT_STORE_KEY" "$scratch/other-key"
expect_check_refusal 'different store key path (same bytes)' 'changed inputs or compiler identity' env LKJSCRIPT_STORE_KEY="$scratch/other-key"
ln -s "$scratch/other-key" "$scratch/symlink-key"
expect_check_refusal 'symlink store key' 'invalid store key' env LKJSCRIPT_STORE_KEY="$scratch/symlink-key"
mkfifo "$scratch/fifo-key"
if timeout 5s bash tools/bootstrap-handoff.sh inputs "$scratch/fifo-key" "$LKJSCRIPT_STORE" >"$directory/refusal.log" 2>&1; then
    echo 'FAIL accepted FIFO store key' >&2
    exit 1
fi
grep -F 'FAIL bootstrap handoff: invalid store key' "$directory/refusal.log"
printf 'ok refusal: FIFO store key\n'

# A record from a different producer lifetime cannot become a local cache.
cp "$receipt" "$directory/receipt-original"
sed -i '2s/^[0-9][0-9]* [0-9][0-9]*/1 invalid/' "$receipt"
expect_check_refusal 'stale producer lifetime' 'not a live same-run bootstrap ancestor' env
mv "$directory/receipt-original" "$receipt"

# Exercise the real bootstrap's unconsumed-receipt branch without a second
# expensive compiler build. This isolated seed fixture only copies itself.
fixture="$scratch/unconsumed"
mkdir -p "$fixture"/{src,library,seed,tools}
cp tools/{bootstrap.sh,bootstrap-handoff.sh,store-key.sh} "$fixture/tools/"
printf 'fixture key' > "$fixture/key"
gzip -c > "$fixture/seed/lkjscript.gz" <<'SEED'
#!/bin/sh
set -eu
case "$1" in
    build)
        while [ "$#" -gt 0 ]; do
            if [ "$1" = -o ]; then cp "$0" "$2"; exit 0; fi
            shift
        done
        exit 1
        ;;
    check-library) exit 0 ;;
    *) exit 1 ;;
esac
SEED
if (cd "$fixture" && LKJSCRIPT_STORE=target/store LKJSCRIPT_STORE_KEY="$fixture/key" sh tools/bootstrap.sh --handoff true) >"$directory/refusal.log" 2>&1; then
    echo 'FAIL accepted unconsumed receipt' >&2
    exit 1
fi
grep -Fx 'ok bootstrap' "$directory/refusal.log"
grep -Fx 'FAIL bootstrap handoff: receipt was not consumed' "$directory/refusal.log"
if compgen -G "$fixture/target/bootstrap.*" > /dev/null; then
    echo 'FAIL unconsumed receipt scratch survived' >&2
    exit 1
fi
printf 'ok refusal: unconsumed receipt and scratch cleanup\n'

preamble=$(awk '{ print } /^check_binary=target\/lkjscript$/ { found=1; exit } END { if (!found) exit 1 }' tools/check.sh)
bash -c "$preamble" tools/check.sh
if bash tools/bootstrap-handoff.sh verify "$receipt" "$LKJSCRIPT_STORE_KEY" "$LKJSCRIPT_STORE"; then
    echo 'FAIL receipt replay accepted' >&2
    exit 1
fi
printf 'ok single-use receipt\n'
