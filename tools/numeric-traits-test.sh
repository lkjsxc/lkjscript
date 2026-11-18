#!/usr/bin/env bash
# Numeric syntax keeps canonical source dependencies and checked edit boundaries.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
mkdir -p target
fixture=$(mktemp -d "$PWD/target/programs-numeric-traits.XXXXXX")
trap 'if [[ ${LKJSCRIPT_NUMERIC_TRAITS_KEEP:-0} == 1 ]]; then printf "retained_fixture=%s\n" "$fixture" >&2; else rm -rf -- "$fixture"; fi' EXIT HUP INT TERM
umask 077
mkdir "$fixture/project" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/project/main.lkj" <<'SOURCE'
use std::ops::{AddAssign, Shl};
struct Word { value: i64 }
impl AddAssign for Word { fn add_assign(&mut self, rhs: Self) { self.value += rhs.value; } }
impl Shl<usize> for Word { type Output = i64; fn shl(self, rhs: usize) -> i64 { self.value << rhs } }
fn apply<T: Shl<usize, Output = i64>>(value: T) -> i64 { value << 1usize }
fn append<T: AddAssign>(value: &mut T, rhs: T) { *value += rhs; }
macro_rules! operators {
    ($shift:ident, $append:ident) => {
        fn $shift<T: Shl<usize, Output = i64>>(value: T) -> i64 { value << 1usize }
        fn $append<T: AddAssign>(value: &mut T, rhs: T) { *value += rhs; }
    };
}
operators!(generated_apply, generated_append);
fn main() -> i64 {
    let mut value = Word { value: 20 };
    append(&mut value, Word { value: 1 });
    generated_append(&mut value, Word { value: 0 });
    apply(value) + generated_apply(Word { value: 0 })
}
SOURCE
common=(main.lkj --source-dir "$fixture/project" --store "$fixture/store" --store-key "$fixture/key")
publish() {
    "$compiler" build "${common[@]}" --index -o "$fixture/image" > "$fixture/index"
    snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/index")
    [[ ${#snapshot} == 64 ]]
}
query() { "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$1" --id "$2" --limit 128 --bytes 65536; }
value() {
    for engine in native interpreter; do
        [[ $("$compiler" run main.lkj --source-dir "$fixture/project" --no-store --engine "$engine") == "$1" ]]
    done
}
edit() {
    "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/transaction"
    grep -q 'mode=apply' "$fixture/transaction"
    [[ ! -e $fixture/project/.lkjscript-edit.pending ]]
}
token_reference() {
    perl -e '
        use strict; use warnings;
        open my $input, "<", $ARGV[0] or die $!;
        my $source = do { local $/; <$input> }; close $input;
        open my $references, "<", $ARGV[1] or die $!;
        my $seen=0;
        while (<$references>) {
            next unless /kind=operator-contract/;
            /fact=references\b/ && /\bstart=(\d+) end=(\d+) kind=operator-contract(?:\s|$)/
                or die "operator reference is not an exact authored site";
            substr($source, $1, $2-$1) eq $ARGV[2] or die "operator token differs";
            $seen++;
        }
        die "operator reference count" unless $seen==1;
    ' "$fixture/project/main.lkj" "$1" "$2"
}
references() {
    query references apply > "$fixture/shift-references"
    grep -F '"std::ops::Shl::shl"' "$fixture/shift-references" | grep -Fq 'kind=operator-contract'
    query references append > "$fixture/assign-references"
    grep -F '"std::ops::AddAssign::add_assign"' "$fixture/assign-references" | grep -Fq 'kind=operator-contract'
    token_reference "$fixture/shift-references" '<<'
    token_reference "$fixture/assign-references" '+='
    query uses std::ops::Shl::shl > "$fixture/generated-shift-uses"
    grep -F 'kind=operator-contract' "$fixture/generated-shift-uses" | grep -Fq 'origin=generated'
    query uses std::ops::AddAssign::add_assign > "$fixture/generated-assign-uses"
    grep -F 'kind=operator-contract' "$fixture/generated-assign-uses" | grep -Fq 'origin=generated'
}
publish
value 42
references
query signature std::convert::From > "$fixture/conversion-signature"
grep -Fq 'From<T>' "$fixture/conversion-signature"
query text generated_apply > "$fixture/generated-text"
grep -Fq 'fact=text-unavailable' "$fixture/generated-text"
cp "$fixture/project/main.lkj" "$fixture/before-generated"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"generated_apply","name":"changed"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rejected-generated" 2> "$fixture/error-generated"; then
    printf 'generated operator function became editable\n' >&2
    exit 1
fi
grep -q 'E_EDIT_GENERATED_SOURCE' "$fixture/error-generated"
cmp "$fixture/before-generated" "$fixture/project/main.lkj"
cp "$fixture/project/main.lkj" "$fixture/before"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"std::ops::Shl::shl","name":"shift"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rejected" 2> "$fixture/error"; then
    printf 'operator punctuation was treated as an editable trait-method name\n' >&2
    exit 1
fi
## The generated functions depend on this exact protocol member. The existing
# generated-reference guard runs before the standard-trait rename guard.
grep -q 'E_EDIT_GENERATED_REFERENCE' "$fixture/error"
cmp "$fixture/before" "$fixture/project/main.lkj"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Word","name":"Number"}]}
REQUEST
edit
value 42
publish
references
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"apply","source":"{ value << 2usize }"}]}
REQUEST
edit
value 84
publish
references
cp "$fixture/project/main.lkj" "$fixture/before"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"apply","source":"{ value << true }"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rejected" 2> "$fixture/error"; then
    printf 'checked operator edit accepted a different applied RHS contract\n' >&2
    exit 1
fi
grep -q 'E_TRAIT' "$fixture/error"
cmp "$fixture/before" "$fixture/project/main.lkj"
value 84
printf 'ok numeric traits: exact authored/generated operator dependencies, generated item/protocol edit refusal, nominal rename, body edits and atomic wrong-contract refusal on both engines\n'
