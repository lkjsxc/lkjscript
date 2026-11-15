#!/usr/bin/env bash
# Signed intent publication is complete after authenticated recovery.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
fixture=$(mktemp -d target/compiler-structure-test.XXXXXX)
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
mkdir "$fixture/source"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/source/main.lkj" <<'SOURCE'
mod a;
mod b;
pub fn probe() -> i64 { a::first(20) + b::second(20) }
SOURCE
printf '%s\n' 'pub fn first(value: i64) -> i64 { value + 1 }' > "$fixture/source/a.lkj"
printf '%s\n' '// untouched byte projection: π' 'pub fn second(value: i64) -> i64 { value + 1 }' > "$fixture/source/b.lkj"
cp "$fixture/source/b.lkj" "$fixture/untouched"
common=(main.lkj --source-dir "$fixture/source" --no-library --no-store)
edit() { "$compiler" edit "${common[@]}" --request "$fixture/request" "$@" > "$fixture/plan" 2> "$fixture/error"; }
snapshot() { (cd "$fixture/source"; sha256sum *.lkj); }
refuse() {
    snapshot > "$fixture/before"
    if edit --store-key "$fixture/key"; then
        printf 'unexpected accepted structural transaction\n' >&2; exit 1
    fi
    snapshot > "$fixture/after"
    cmp "$fixture/before" "$fixture/after"
    test ! -e "$fixture/source/.lkjscript-edit.pending"
    test -s "$fixture/error"
}
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"a::first","source":"{ value + 2 }"}]}' > "$fixture/request"
snapshot > "$fixture/before"
edit --dry-run
snapshot > "$fixture/after"
cmp "$fixture/before" "$fixture/after"
grep -q 'mode=dry-run' "$fixture/plan"
edit --store-key "$fixture/key"
test "$("$compiler" run "$fixture/source/main.lkj" --no-library --no-store)" = 43
cmp "$fixture/source/b.lkj" "$fixture/untouched"
cp -a "$fixture/source" "$fixture/before-rename"
printf '%s\n' '{"version":1,"operations":[{"op":"rename","id":"a::first","name":"increment"}]}' > "$fixture/request"
edit --store-key "$fixture/key"
test "$("$compiler" run "$fixture/source/main.lkj" --no-library --no-store)" = 43
cmp "$fixture/source/b.lkj" "$fixture/untouched"
"$compiler" compare "$fixture/source/main.lkj" --against "$fixture/before-rename/main.lkj" --no-library --no-store > "$fixture/comparison"
grep -q 'text differs, checked IR identical' "$fixture/comparison"
test "$(wc -l < "$fixture/comparison")" -le 30
cp -a "$fixture/source" "$fixture/before-comment"
printf '%s\n' '// a comment has no checked operation' >> "$fixture/source/main.lkj"
"$compiler" compare "$fixture/source/main.lkj" --against "$fixture/before-comment/main.lkj" --no-library --no-store > "$fixture/comparison"
grep -q 'text differs, checked IR identical' "$fixture/comparison"
while IFS= read -r operation; do
    printf '{"version":1,"operations":[%s]}\n' "$operation" > "$fixture/request"
    refuse
done <<'OPERATIONS'
{"op":"replace-body","id":"a::increment","source":"{ missing(value) }"}
{"op":"replace-item","id":"a::increment","source":"pub fn increment(value: i64) -> i64 { false }"}
{"op":"add-item","id":"a","source":"pub fn bad() -> i64 { missing() }"}
{"op":"remove-item","id":"a::increment"}
{"op":"rename","id":"a::increment","name":"not-an-identifier"}
{"op":"move","id":"a::increment","parent":"b"}
{"op":"replace-body","id":"a::increment","expect":"wrong","source":"{ value }"}
OPERATIONS
printf '%s\n' '{"version":1,"snapshot":"wrong","operations":[{"op":"replace-body","id":"a::increment","source":"{ value }"}]}' > "$fixture/request"
refuse
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"a::increment","source":"{ value }"},{"op":"replace-body","id":"a::increment","source":"{ value + 7 }"}]}' > "$fixture/request"
refuse
# A copied or hardlinked private key is refused before candidate elaboration.
ln "$fixture/key" "$fixture/source/private.lkj"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"a::increment","source":"{ let _ = include_bytes!(\"private.lkj\"); value }"}]}' > "$fixture/request"
refuse
grep -q 'private.*key' "$fixture/error"
rm "$fixture/source/private.lkj"
# Planted data and FIFO intents neither mutate sources nor block indefinitely.
snapshot > "$fixture/before"
printf '%s\n' 'untrusted journal' > "$fixture/source/.lkjscript-edit.pending"
if "$compiler" edit "${common[@]}" --recover --store-key "$fixture/key" > "$fixture/plan" 2> "$fixture/error"; then exit 1; fi
snapshot > "$fixture/after"
cmp "$fixture/before" "$fixture/after"
rm "$fixture/source/.lkjscript-edit.pending"
mkfifo "$fixture/source/.lkjscript-edit.pending"
if timeout --kill-after=2s 10s "$compiler" edit "${common[@]}" --recover --store-key "$fixture/key" > "$fixture/plan" 2> "$fixture/error"; then exit 1; else code=$?; test "$code" -ne 124; fi
snapshot > "$fixture/after"
cmp "$fixture/before" "$fixture/after"
rm "$fixture/source/.lkjscript-edit.pending"
# Nonoverlapping edits and independent insertions share one checked candidate.
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"a::increment","source":"{ value + 3 }"},{"op":"replace-body","id":"b::second","source":"{ value + 4 }"},{"op":"add-item","id":"crate","source":"fn added_one() -> i64 { 1 }"},{"op":"add-item","id":"crate","source":"fn added_two() -> i64 { 2 }"}]}' > "$fixture/request"
edit --store-key "$fixture/key"
test "$("$compiler" run "$fixture/source/main.lkj" --no-library --no-store)" = 47
test ! -e "$fixture/source/.lkjscript-edit.pending"
printf 'ok structural transactions: checks, refusals, untouched bytes, rename/comment identity, authenticated recovery\n'
