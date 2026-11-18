#!/usr/bin/env bash
# Defaults remain authored or generated according to their exact source owner.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
shift $(( $# > 0 ? 1 : 0 ))
witness=
if (($#)); then
    [[ $1 == --witness && $# == 2 ]] || { printf 'usage: %s [COMPILER] [--witness RUSTC]\n' "$0" >&2; exit 2; }
    witness=$2
fi
library="$PWD/library"
mkdir -p target
fixture=$(mktemp -d "$PWD/target/type-defaults-origins.XXXXXX")
trap 'if [[ ${LKJSCRIPT_TYPE_DEFAULTS_KEEP:-0} == 1 ]]; then printf "retained_fixture=%s\n" "$fixture" >&2; else rm -rf -- "$fixture"; fi' EXIT HUP INT TERM
umask 077
mkdir "$fixture/project" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
fail() { printf 'type-defaults-origins: %s\n' "$*" >&2; exit 1; }
common=(main.lkj --source-dir "$fixture/project" --library-dir "$library" --store "$fixture/store" --store-key "$fixture/key")
publish() {
    "$compiler" build "${common[@]}" --index -o "$fixture/image" > "$fixture/index" 2> "$fixture/error" || { cat "$fixture/error" >&2; fail build; }
    snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/index")
    [[ ${#snapshot} == 64 ]] || fail 'missing snapshot'
}
query() { "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$1" --id "$2" --limit 128 --bytes 65536; }
value() {
    for engine in native interpreter; do
        [[ $("$compiler" run main.lkj --source-dir "$fixture/project" --library-dir "$library" --no-store --engine "$engine") == 42 ]] || fail "$engine value"
    done
}
edit() {
    "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/transaction" 2> "$fixture/error" || { cat "$fixture/error" >&2; fail edit; }
    grep -q 'mode=apply' "$fixture/transaction"
    [[ ! -e $fixture/project/.lkjscript-edit.pending ]]
}
epoch() { sed -n 's/.*derived-epoch=\([0-9a-f]*\).*/\1/p' "$1" | head -n 1; }
binding_type() {
    query outline main > "$fixture/outline"
    binding=$(sed -n "s/^\"\([^\"]*::binding:$1:0\)\".*/\1/p" "$fixture/outline" | head -n 1)
    [[ -n $binding ]] || fail "missing $1 binding"
    query type "$binding" > "$fixture/type-$1"
    grep -Fq "$2" "$fixture/type-$1" || { cat "$fixture/type-$1" >&2; fail "wrong $1 application"; }
}
cat > "$fixture/project/main.lkj" <<'SOURCE'
#[derive(Default, Debug)]
struct Authored<T = i64, U = T> { first: T, second: U }
macro_rules! relay { ($($source:tt)*) => { $($source)* }; }
macro_rules! emit_defaults {
    ($name:ident, $type:tt) => {
        relay! { #[derive(Default, Debug)] struct $name<T = $type, U = T> { first: T, second: U } }
    };
}
emit_defaults!(Generated, i64);
fn main() -> i64 {
    let authored: Authored = Authored::default();
    let generated: Generated = Generated::default();
    assert_eq!(format!("{:?}", authored), "Authored { first: 0, second: 0 }");
    assert_eq!(format!("{:?}", generated), "Generated { first: 0, second: 0 }");
    authored.first as i64 + authored.second as i64 + generated.first as i64 + generated.second as i64 + 42
}
SOURCE
publish
value
query signature Authored > "$fixture/signature"
grep -Fq 'T = i64, U = T' "$fixture/signature"
query references Authored > "$fixture/references"
grep -Fq '"Authored::generic:T"' "$fixture/references"
binding_type authored 'Authored<i64, i64>'
binding_type generated 'Generated<i64, i64>'
query meta Generated > "$fixture/generated-meta"
grep -Fq 'expansion=macro-rules source-projection=false' "$fixture/generated-meta"
query text Generated > "$fixture/generated-text"
grep -Fq 'fact=text-unavailable' "$fixture/generated-text"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Authored::generic:T","name":"Element"}]}
REQUEST
edit
value
grep -Fq 'U = Element' "$fixture/project/main.lkj"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-item","id":"Authored","source":"#[derive(Default, Debug)]\nstruct Authored<Element = u64, U = Element> { first: Element, second: U }"}]}
REQUEST
edit
value
publish
binding_type authored 'Authored<u64, u64>'
query meta Generated > "$fixture/before-meta"
before_epoch=$(epoch "$fixture/before-meta")
[[ ${#before_epoch} == 64 ]]
cp "$fixture/project/main.lkj" "$fixture/before"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Generated","name":"Changed"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/refused" 2> "$fixture/error"; then fail 'generated declaration became editable'; fi
grep -Fq E_EDIT_GENERATED_SOURCE "$fixture/error"
cmp "$fixture/before" "$fixture/project/main.lkj"
sed -i 's/emit_defaults!(Generated, i64);/emit_defaults!(Generated, u64);/' "$fixture/project/main.lkj"
publish
value
binding_type generated 'Generated<u64, u64>'
query meta Generated > "$fixture/after-meta"
after_epoch=$(epoch "$fixture/after-meta")
[[ ${#after_epoch} == 64 && $before_epoch != "$after_epoch" ]] || fail 'default capture edit kept the previous macro epoch'
if [[ -n $witness ]]; then
    {
        printf '#![allow(dead_code,unused_imports,unused_variables)]\n'
        cat tests/conformance/pass/type_defaults_macro_namespace.lkj
        printf '\nfn main() { assert_eq!(probe(), 42); }\n'
    } > "$fixture/namespace.rs"
    for optimization in 0 3; do
        "$witness" --edition=2024 --crate-name type_defaults_namespace -A warnings -C "opt-level=$optimization" "$fixture/namespace.rs" -o "$fixture/namespace" > "$fixture/rust-output" 2> "$fixture/rust-error"
        "$fixture/namespace" >> "$fixture/rust-output" 2>> "$fixture/rust-error"
    done
    printf 'ok type defaults Rust root-crate witness: captured versus $crate default namespaces at opt-level 0 and 3\n'
fi
printf 'ok type defaults origins: Default/Debug and nested macros; authored binder/default edits; generated edit refusal; changed default capture invalidates its epoch; both engines\n'
