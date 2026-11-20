#!/usr/bin/env bash
# Family binders and projection applications survive checked source operations.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
mkdir -p target
fixture=$(mktemp -d "$PWD/target/programs-type-families.XXXXXX")
trap 'if [[ ${LKJSCRIPT_TYPE_FAMILIES_KEEP:-0} == 1 ]]; then printf "retained_fixture=%s\n" "$fixture" >&2; else rm -rf -- "$fixture"; fi' EXIT HUP INT TERM
umask 077
mkdir "$fixture/project" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/project/main.lkj" <<'SOURCE'
trait Mode { type Output<T>; fn bind<T>(value: T) -> Self::Output<T>; }
struct Emit {}
impl Mode for Emit { type Output<Element> = Element; fn bind<Value>(value: Value) -> Value { value } }
trait Other { type Output<T>; }
trait Identity<T> {}
trait Scoped { type Family<T: Identity<T>>; }
fn keep<M: Mode, T>(value: <M as Mode>::Output<T>) -> M::Output<T> { value }
fn produce<M: Mode>(value: i64) -> M::Output<i64> { M::bind(value) }
fn main() -> i64 { keep::<Emit, i64>(produce::<Emit>(42)) }
SOURCE
common=(main.lkj --source-dir "$fixture/project" --no-library --store "$fixture/store" --store-key "$fixture/key")
publish() {
    "$compiler" build "${common[@]}" --index -o "$fixture/image" > "$fixture/index"
    snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/index")
    [[ ${#snapshot} == 64 ]]
}
query() { "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$1" --id "$2" --limit 128 --bytes 65536; }
value() {
    for engine in native interpreter; do
        [[ $("$compiler" run main.lkj --source-dir "$fixture/project" --no-library --no-store --engine "$engine") == 42 ]]
    done
}
edit() {
    "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/transaction"
    grep -q 'mode=apply' "$fixture/transaction"
    [[ ! -e $fixture/project/.lkjscript-edit.pending ]]
}
publish
value
query type keep::param:value > "$fixture/type"
grep -Fq 'value="<M as Mode>::Output<T>"' "$fixture/type"
query signature Mode::type:Output > "$fixture/signature"
grep -Fq 'type Output<T>' "$fixture/signature"
query type Mode::type:Output::generic:T > "$fixture/binder"
grep -Fq 'value="T"' "$fixture/binder"
query references keep > "$fixture/references"
grep -Fq '"type@Mode::type:Output"' "$fixture/references"
[[ $(grep -Fc '"type@Mode::type:Output"' "$fixture/references") == 2 ]]
! grep -Fq '"type@Other::type:Output"' "$fixture/references"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Mode::type:Output","name":"Value"}]}
REQUEST
edit
value
grep -Fq 'type Value<Element> = Element;' "$fixture/project/main.lkj"
grep -Fq '<M as Mode>::Value<T>' "$fixture/project/main.lkj"
grep -Fq 'trait Other { type Output<T>; }' "$fixture/project/main.lkj"
publish
query type keep::param:value > "$fixture/type"
grep -Fq 'value="<M as Mode>::Value<T>"' "$fixture/type"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Scoped::type:Family::generic:T","name":"Element"}]}
REQUEST
edit
value
grep -Fq 'type Family<Element: Identity<Element>>;' "$fixture/project/main.lkj"
grep -Fq 'fn bind<T>(value: T)' "$fixture/project/main.lkj"
cp "$fixture/project/main.lkj" "$fixture/before"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Scoped::type:Family::generic:Element","name":"Identity"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/capture" 2> "$fixture/error"; then
    printf 'associated family binder rename captured an existing type path\n' >&2
    exit 1
fi
grep -q 'CAPTURE' "$fixture/error"
cmp "$fixture/before" "$fixture/project/main.lkj"
cp "$fixture/project/main.lkj" "$fixture/before"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"keep","source":"{ let changed: <M as Mode>::Value<bool> = value; changed }"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rejected" 2> "$fixture/error"; then
    printf 'associated family edit accepted distinct argument identities\n' >&2
    exit 1
fi
grep -q 'E_TYPE' "$fixture/error"
cmp "$fixture/before" "$fixture/project/main.lkj"
value
printf 'ok associated families: application types/signatures/references, member and scoped binder rename, rejected distinct-argument transaction on both engines\n'
