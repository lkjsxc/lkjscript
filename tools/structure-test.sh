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
{"op":"move","id":"a::increment","parent":"missing_module"}
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
# A missing spelling span for a known, unrelated standard-library intrinsic
# does not make a user free function ambiguous.
cp -a "$fixture/source" "$fixture/before-library-rename"
common=(main.lkj --source-dir "$fixture/source" --library-dir "$PWD/library" --no-store)
printf '%s\n' '{"version":1,"operations":[{"op":"rename","id":"a::increment","name":"increase"}]}' > "$fixture/request"
edit --store-key "$fixture/key" || { cat "$fixture/error" >&2; exit 1; }
test "$("$compiler" run "$fixture/source/main.lkj" --no-library --no-store)" = 47
"$compiler" compare "$fixture/source/main.lkj" --against "$fixture/before-library-rename/main.lkj" --library-dir "$PWD/library" --against-library-dir "$PWD/library" --no-store > "$fixture/comparison"
grep -q 'text differs, checked IR identical' "$fixture/comparison"
# A mixed request proves its move subset independently while allowing the
# explicitly requested body replacement to change the complete candidate.
cat > "$fixture/source/main.lkj" <<'SOURCE'
fn offset() -> i64 { 7i64 }
mod old { pub fn helper(n: i64) -> i64 { if n == 0i64 { crate::offset() } else { helper(n - 1i64) } } }
mod fresh {}
pub fn probe() -> i64 { old::helper(2i64) }
SOURCE
common=(main.lkj --source-dir "$fixture/source" --no-library --no-store)
printf '%s\n' '{"version":1,"operations":[{"op":"move","id":"old::helper","parent":"fresh"},{"op":"replace-body","id":"offset","source":"{ 8i64 }"}]}' > "$fixture/request"
edit --store-key "$fixture/key" || { cat "$fixture/error" >&2; exit 1; }
test "$("$compiler" run "$fixture/source/main.lkj" --no-library --no-store)" = 8
grep -q 'mode=apply' "$fixture/plan"
# An edited, previously undemanded library body must still be elaborated.
mkdir -p "$fixture/library-edit/library"
printf '%s\n' 'fn main() -> i64 { 0i64 }' > "$fixture/library-edit/main.lkj"
printf '%s\n' 'pub fn unused(value: i64) -> i64 { value + 1i64 }' > "$fixture/library-edit/library/lib.lkj"
cp "$fixture/library-edit/main.lkj" "$fixture/library-main-before"
library_common=(main.lkj --source-dir "$fixture/library-edit" --library-dir "$fixture/library-edit/library" --no-store --store-key "$fixture/key")
(cd "$fixture/library-edit"; sha256sum main.lkj library/lib.lkj) > "$fixture/library-before"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"std::unused","source":"{ missing() }"}]}' > "$fixture/library-request"
if "$compiler" edit "${library_common[@]}" --request "$fixture/library-request" > "$fixture/library-plan" 2> "$fixture/library-error"; then
    printf 'unexpected accepted invalid undemanded library body\n' >&2; exit 1
fi
grep -q 'E_EDIT_CHECK' "$fixture/library-error"
grep -q 'missing' "$fixture/library-error"
(cd "$fixture/library-edit"; sha256sum main.lkj library/lib.lkj) > "$fixture/library-after"
cmp "$fixture/library-before" "$fixture/library-after"
test ! -e "$fixture/library-edit/.lkjscript-edit.pending"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"std::unused","source":"{ value + 2i64 }"}]}' > "$fixture/library-request"
"$compiler" edit "${library_common[@]}" --request "$fixture/library-request" > "$fixture/library-plan" 2> "$fixture/library-error" || { cat "$fixture/library-error" >&2; exit 1; }
grep -q 'library-validation=all' "$fixture/library-plan"
grep -q 'mode=apply' "$fixture/library-plan"
grep -Fq '{ value + 2i64 }' "$fixture/library-edit/library/lib.lkj"
cmp "$fixture/library-main-before" "$fixture/library-edit/main.lkj"
test ! -e "$fixture/library-edit/.lkjscript-edit.pending"
printf '%s\n' 'fn main() -> i64 { std::unused(40i64) }' > "$fixture/library-edit/probe.lkj"
test "$("$compiler" run "$fixture/library-edit/probe.lkj" --library-dir "$fixture/library-edit/library" --no-store)" = 42
# Promotion keeps mixed transactions atomic, bypasses warm demanded-only
# checkpoints, and compares replay targets in the same all-library mode.
mkdir -p "$fixture/promotion/library" "$fixture/promotion-store"
printf '%s\n' 'fn main() -> i64 { 0i64 }' > "$fixture/promotion/main.lkj"
cat > "$fixture/promotion/library/lib.lkj" <<'SOURCE'
pub fn unused(value: i64) -> i64 { value + 1i64 }
pub fn unused_generic<T>(value: T) -> i64 { 1i64 }
SOURCE
promotion_common=(main.lkj --source-dir "$fixture/promotion" --library-dir "$fixture/promotion/library")
promotion_snapshot() { (cd "$fixture/promotion"; sha256sum main.lkj library/lib.lkj); }
promotion_snapshot > "$fixture/promotion-before"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"main","source":"{ 9i64 }"},{"op":"replace-body","id":"std::unused","source":"{ missing() }"}]}' > "$fixture/promotion-request"
if "$compiler" edit "${promotion_common[@]}" --no-store --store-key "$fixture/key" --request "$fixture/promotion-request" > "$fixture/promotion-plan" 2> "$fixture/promotion-error"; then
    printf 'unexpected accepted mixed invalid library transaction\n' >&2; exit 1
fi
grep -q 'E_EDIT_CHECK' "$fixture/promotion-error"
grep -q 'missing' "$fixture/promotion-error"
promotion_snapshot > "$fixture/promotion-after"
cmp "$fixture/promotion-before" "$fixture/promotion-after"
test ! -e "$fixture/promotion/.lkjscript-edit.pending"
for pass in 1 2; do
    "$compiler" check "${promotion_common[@]}" --store "$fixture/promotion-store" --store-key "$fixture/key" --inline-limit 0 --stats > "$fixture/promotion-warm" 2> "$fixture/promotion-warm-stats"
done
grep -Eq 'front_reused[^0-9]*[1-9][0-9]*' "$fixture/promotion-warm-stats"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"std::unused_generic","source":"{ missing() }"}]}' > "$fixture/promotion-request"
if "$compiler" edit "${promotion_common[@]}" --store "$fixture/promotion-store" --store-key "$fixture/key" --request "$fixture/promotion-request" > "$fixture/promotion-plan" 2> "$fixture/promotion-error"; then
    printf 'unexpected accepted invalid unused generic library body after warm check\n' >&2; exit 1
fi
grep -q 'E_EDIT_CHECK' "$fixture/promotion-error"
grep -q 'std::unused_generic' "$fixture/promotion-error"
grep -q 'missing' "$fixture/promotion-error"
promotion_snapshot > "$fixture/promotion-after"
cmp "$fixture/promotion-before" "$fixture/promotion-after"
test ! -e "$fixture/promotion/.lkjscript-edit.pending"
mkdir -p "$fixture/promotion-desired/library"
cp "$fixture/promotion/main.lkj" "$fixture/promotion-desired/main.lkj"
sed 's/value + 1i64/value + 2i64/' "$fixture/promotion/library/lib.lkj" > "$fixture/promotion-desired/library/lib.lkj"
"$compiler" edit "${promotion_common[@]}" --no-store --store-key "$fixture/key" --against "$fixture/promotion-desired/main.lkj" --against-library-dir "$fixture/promotion-desired/library" > "$fixture/promotion-plan" 2> "$fixture/promotion-error" || { cat "$fixture/promotion-error" >&2; exit 1; }
grep -q 'library-validation=all' "$fixture/promotion-plan"
grep -q 'mode=apply' "$fixture/promotion-plan"
grep -Eq '^replay desired=([0-9a-f]{64}) candidate=\1 verified=true operations=[1-9][0-9]*$' "$fixture/promotion-plan"
before_identity=$(sed -n 's/^program level=checked-ir before=\([^ ]*\).*/\1/p' "$fixture/promotion-plan")
after_identity=$(sed -n 's/^program level=checked-ir .* after=\([^ ]*\).*/\1/p' "$fixture/promotion-plan")
test -n "$before_identity"
test "$before_identity" != "$after_identity"
cmp "$fixture/promotion/main.lkj" "$fixture/promotion-desired/main.lkj"
cmp "$fixture/promotion/library/lib.lkj" "$fixture/promotion-desired/library/lib.lkj"
test ! -e "$fixture/promotion/.lkjscript-edit.pending"
printf 'ok structural transactions: checks, refusals, untouched bytes, rename/comment identity, authenticated recovery\n'
