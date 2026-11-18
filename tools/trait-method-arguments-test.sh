#!/usr/bin/env bash
# Nonreceiver impl arguments remain exact through questions and checked edits.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
mkdir -p target
fixture=$(mktemp -d "$PWD/target/programs-trait-method-arguments.XXXXXX")
trap 'if [[ ${LKJSCRIPT_TRAIT_METHOD_ARGUMENTS_KEEP:-0} == 1 ]]; then printf "retained_fixture=%s\n" "$fixture" >&2; else rm -rf -- "$fixture"; fi' EXIT HUP INT TERM
umask 077
mkdir "$fixture/project" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/project/main.lkj" <<'SOURCE'
trait Parser<I> { fn parse(&mut self, input: I) -> i64; }
trait Pair<I> { fn parse_pair(&mut self, input: I) -> i64; }
struct Number { offset: i64 }
impl Parser<i64> for Number { fn parse(&mut self, input: i64) -> i64 { input + self.offset } }
impl<I, P: Parser<I>> Pair<I> for (P,) { fn parse_pair(&mut self, input: I) -> i64 { self.0.parse(input) } }
fn main() -> i64 { let mut pair = (Number { offset: 2 },); pair.parse_pair(40i64) }
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
query signature Pair::parse_pair > "$fixture/signature"
grep -Fq 'input: I' "$fixture/signature"
query references main > "$fixture/references"
grep -Eq 'parse_pair.*kind=method' "$fixture/references"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Pair::parse_pair","name":"parse_input"}]}
REQUEST
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/transaction"
grep -q 'mode=apply' "$fixture/transaction"
grep -Fq 'pair.parse_input(40i64)' "$fixture/project/main.lkj"
value
publish
query references main > "$fixture/references"
grep -Eq 'parse_input.*kind=method' "$fixture/references"
cp "$fixture/project/main.lkj" "$fixture/before"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"main","source":"{ let mut pair = (Number { offset: 2 },); pair.parse_input(true) }"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rejected" 2> "$fixture/error"; then
    printf 'trait method edit accepted an unwitnessed input application\n' >&2
    exit 1
fi
grep -q 'E_TRAIT' "$fixture/error"
cmp "$fixture/before" "$fixture/project/main.lkj"
[[ ! -e $fixture/project/.lkjscript-edit.pending ]]
value
printf 'ok trait method arguments: concrete tuple dispatch, exact method references/rename and rejected input transaction on both engines\n'

# Method-local arguments inferred from a result must survive header selection.
cp "$fixture/project/main.lkj" "$fixture/implementation-arguments-source.lkj"
cat > "$fixture/project/main.lkj" <<'SOURCE'
trait Seed { fn seed() -> Self; }
impl Seed for i64 { fn seed() -> Self { 42i64 } }
struct Envelope<T> { value: T }
impl<T> Envelope<T> { fn take(self) -> T { self.value } }
trait Factory { fn create<T: Seed>(&self) -> Envelope<T>; }
struct Maker;
impl Factory for Maker {
    fn create<T: Seed>(&self) -> Envelope<T> { Envelope { value: T::seed() } }
}
fn produce<R: Seed>(maker: &Maker) -> R { maker.create().take() }
fn main() -> i64 { produce::<i64>(&Maker) }
SOURCE
publish
value
query signature Factory::create > "$fixture/method-result-signature"
grep -Fq 'T: Seed' "$fixture/method-result-signature"
query references produce > "$fixture/method-result-references"
grep -Eq 'create.*kind=method' "$fixture/method-result-references"
query callees produce > "$fixture/method-result-callees"
grep -Eq 'create.*i64|i64.*create' "$fixture/method-result-callees"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Factory::create","name":"construct"}]}
REQUEST
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/method-result-rename"
grep -Fq 'maker.construct().take()' "$fixture/project/main.lkj"
value
publish
query references produce > "$fixture/method-result-renamed"
grep -Eq 'construct.*kind=method' "$fixture/method-result-renamed"
cp "$fixture/project/main.lkj" "$fixture/method-result-before"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-item","id":"produce","source":"fn produce<R>(maker: &Maker) -> R { maker.construct().take() }"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/method-result-bound-rejected" 2> "$fixture/method-result-bound-error"; then
    printf 'method result inference manufactured a missing generic bound\n' >&2
    exit 1
fi
grep -Fq E_TRAIT "$fixture/method-result-bound-error"
cmp "$fixture/method-result-before" "$fixture/project/main.lkj"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"produce","source":"{ maker.construct::<i64>().take() }"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/method-result-type-rejected" 2> "$fixture/method-result-type-error"; then
    printf 'method result inference replaced a rigid result with one instance\n' >&2
    exit 1
fi
grep -Fq E_TYPE "$fixture/method-result-type-error"
cmp "$fixture/method-result-before" "$fixture/project/main.lkj"
[[ ! -e $fixture/project/.lkjscript-edit.pending ]]
value
printf 'ok method result arguments: chained context, exact specialized calls and rename, missing-bound and wrong-result transactions on both engines\n'
