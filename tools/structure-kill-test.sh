#!/usr/bin/env bash
# Optional campaign: kill publication, authenticate recovery, require one whole projection.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
fixture=$(mktemp -d target/compiler-structure-kill.XXXXXX)
child=
cleanup() {
    if [[ -n "$child" ]]; then kill -KILL "$child" 2>/dev/null || true; wait "$child" 2>/dev/null || true; fi
    rm -rf "$fixture"
}
trap cleanup EXIT HUP INT TERM
mkdir "$fixture/old" "$fixture/new"
head -c 32 /dev/urandom > "$fixture/key"
head -c 32 /dev/urandom > "$fixture/wrong-key"
: > "$fixture/old/main.lkj"
printf '{"version":1,"operations":[' > "$fixture/request"
for ((i=0; i<64; i++)); do
    printf 'mod m%s;\n' "$i" >> "$fixture/old/main.lkj"
    { printf '// '; head -c 16384 /dev/zero | tr '\0' x; printf '\npub fn value() -> i64 { %s }\n' "$i"; } > "$fixture/old/m$i.lkj"
    { printf '// '; head -c 16384 /dev/zero | tr '\0' x; printf '\npub fn value() -> i64 { %s }\n' "$((i+1000))"; } > "$fixture/new/m$i.lkj"
    [[ "$i" == 0 ]] || printf ',' >> "$fixture/request"
    printf '{"op":"replace-body","id":"m%s::value","source":"{ %s }"}' "$i" "$((i+1000))" >> "$fixture/request"
done
printf 'pub fn probe() -> i64 { m0::value() }\n' >> "$fixture/old/main.lkj"
cp "$fixture/old/main.lkj" "$fixture/new/main.lkj"
printf ']}\n' >> "$fixture/request"
interrupted=0
committed=0
attacked=0
for ((trial=0; trial<32; trial++)); do
    rm -rf "$fixture/source"
    cp -a "$fixture/old" "$fixture/source"
    "$compiler" edit main.lkj --source-dir "$fixture/source" --no-library --no-store --request "$fixture/request" --store-key "$fixture/key" > "$fixture/plan" 2> "$fixture/error" &
    child=$!
    for ((poll=0; poll<200000; poll++)); do
        kill -0 "$child" 2>/dev/null || break
        if [[ -f "$fixture/source/.lkjscript-edit.pending" ]]; then
            if ((trial%2==0)) || cmp -s "$fixture/source/m$((trial/2)).lkj" "$fixture/new/m$((trial/2)).lkj"; then
                kill -KILL "$child" 2>/dev/null || true
                interrupted=$((interrupted+1))
                break
            fi
        fi
    done
    wait "$child" 2>/dev/null || true
    child=
    if [[ -f "$fixture/source/.lkjscript-edit.pending" && "$attacked" == 0 ]]; then
        # Retain the valid committed intent; hostile envelopes must change nothing.
        cp "$fixture/source/.lkjscript-edit.pending" "$fixture/intent"
        (cd "$fixture/source"; sha256sum *.lkj) > "$fixture/before"
        if "$compiler" edit main.lkj --source-dir "$fixture/source" --no-library --no-store --recover --store-key "$fixture/wrong-key" > "$fixture/recovery" 2> "$fixture/error"; then exit 1; fi
        (cd "$fixture/source"; sha256sum *.lkj) > "$fixture/after"
        cmp "$fixture/before" "$fixture/after"
        size=$(stat -c %s "$fixture/source/.lkjscript-edit.pending")
        printf '\377' | dd of="$fixture/source/.lkjscript-edit.pending" bs=1 seek="$((size-1))" conv=notrunc status=none
        if "$compiler" edit main.lkj --source-dir "$fixture/source" --no-library --no-store --recover --store-key "$fixture/key" > "$fixture/recovery" 2> "$fixture/error"; then exit 1; fi
        (cd "$fixture/source"; sha256sum *.lkj) > "$fixture/after"
        cmp "$fixture/before" "$fixture/after"
        cp "$fixture/intent" "$fixture/source/.lkjscript-edit.pending"
        # The ordinary compiler refuses a projection interrupted after commit.
        if "$compiler" check main.lkj --source-dir "$fixture/source" --no-library --no-store > "$fixture/check" 2> "$fixture/error"; then exit 1; fi
        grep -q E_EDIT_RECOVERY "$fixture/error"
        attacked=1
    fi
    "$compiler" edit main.lkj --source-dir "$fixture/source" --no-library --no-store --recover --store-key "$fixture/key" > "$fixture/recovery" 2> "$fixture/error"
    old=1
    new=1
    for ((i=0; i<64; i++)); do
        cmp -s "$fixture/source/m$i.lkj" "$fixture/old/m$i.lkj" || old=0
        cmp -s "$fixture/source/m$i.lkj" "$fixture/new/m$i.lkj" || new=0
    done
    test "$((old+new))" -eq 1
    cmp "$fixture/source/main.lkj" "$fixture/old/main.lkj"
    committed=$((committed+new))
    test ! -e "$fixture/source/.lkjscript-edit.pending"
    test ! -e "$fixture/source/..lkjscript-edit.pending.lkjscript-next"
    "$compiler" check main.lkj --source-dir "$fixture/source" --no-library --no-store > "$fixture/check" 2> "$fixture/error"
done
test "$interrupted" -gt 0
test "$attacked" -eq 1
printf 'ok source transaction kill campaign: attempts=32 killed=%s recovered-new=%s wrong-key/tampered-payload refused; every recovered tree wholly old/new\n' "$interrupted" "$committed"
