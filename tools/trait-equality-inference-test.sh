#!/usr/bin/env bash
# Prerequisite equalities retain applied identities through checked source edits.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
mkdir -p target
fixture=$(mktemp -d "$PWD/target/programs-trait-equality-inference.XXXXXX")
trap 'if [[ ${LKJSCRIPT_TRAIT_EQUALITY_KEEP:-0} == 1 ]]; then printf "retained_fixture=%s\n" "$fixture" >&2; else rm -rf -- "$fixture"; fi' EXIT HUP INT TERM
umask 077
mkdir "$fixture/project" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/project/main.lkj" <<'SOURCE'
trait Source<I> { type Output; fn produce(self, input: I) -> Self::Output; }
struct Number;
impl Source<u8> for Number { type Output = i64; fn produce(self, input: u8) -> i64 { input as i64 + 40 } }
impl Source<bool> for Number { type Output = i32; fn produce(self, input: bool) -> i32 { if input { 42 } else { 0 } } }
trait Adapter<I> { type Output; fn adapt(self, input: I) -> Self::Output; }
impl<I, O, P: Source<I, Output = O>> Adapter<I> for (P,) {
    type Output = O;
    fn adapt(self, input: I) -> O { self.0.produce(input) }
}
fn run<P: Adapter<u8, Output = i64>>(parser: P) -> i64 { parser.adapt(2u8) }
fn run_bool<P: Adapter<bool, Output = i32>>(parser: P) -> i32 { parser.adapt(true) }
fn keep<P: Source<u8, Output = i64>>(parser: P) -> i64 { run((parser,)) }
fn identity(value: <(Number,) as Adapter<u8>>::Output) -> i64 { value }
fn main() -> i64 { identity(keep(Number)) }
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
publish
value
query signature keep > "$fixture/signature"
grep -Fq 'Source<u8, Output = i64>' "$fixture/signature"
query type identity::param:value > "$fixture/type"
grep -Fq '<(Number,) as Adapter<u8>>::Output' "$fixture/type"
query references keep > "$fixture/references"
grep -Fq '"Source::type:Output"' "$fixture/references"
! grep -Fq '"Adapter::type:Output"' "$fixture/references"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Source::type:Output","name":"Value"}]}
REQUEST
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/transaction"
grep -q 'mode=apply' "$fixture/transaction"
[[ ! -e $fixture/project/.lkjscript-edit.pending ]]
grep -Fq 'Source<I, Value = O>' "$fixture/project/main.lkj"
grep -Fq 'trait Adapter<I> { type Output;' "$fixture/project/main.lkj"
value
publish
query references keep > "$fixture/references"
grep -Fq '"Source::type:Value"' "$fixture/references"
cp "$fixture/project/main.lkj" "$fixture/before"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"keep","source":"{ run_bool((parser,)) as i64 }"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rejected" 2> "$fixture/error"; then
    printf 'prerequisite inference accepted an unproved applied trait\n' >&2
    exit 1
fi
grep -q 'E_TRAIT' "$fixture/error"
cmp "$fixture/before" "$fixture/project/main.lkj"
value
printf 'ok prerequisite equalities: applied projection queries, exact member rename, and atomic refusal of a missing generic contract on both engines\n'
