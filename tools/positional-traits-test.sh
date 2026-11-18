#!/usr/bin/env bash
# Applied trait evidence survives source queries and checked transactions.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
mkdir -p target
fixture=$(mktemp -d "$PWD/target/programs-positional-traits.XXXXXX")
trap 'if [[ ${LKJSCRIPT_POSITIONAL_TRAITS_KEEP:-0} == 1 ]]; then printf "retained_fixture=%s\n" "$fixture" >&2; else rm -rf -- "$fixture"; fi' EXIT HUP INT TERM
umask 077
mkdir "$fixture/project" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/project/main.lkj" <<'SOURCE'
trait Parser<Input> { type Output; fn parse(self, input: Input) -> Self::Output; }
struct Number {}
impl Parser<u8> for Number { type Output = i64; fn parse(self, input: u8) -> i64 { input as i64 + 40 } }
impl Parser<bool> for Number { type Output = bool; fn parse(self, input: bool) -> bool { input } }
trait Other { type Output; fn parse(self) -> Self::Output; }
fn keep<P: Parser<u8>>(value: <P as Parser<u8>>::Output) -> <P as Parser<u8>>::Output { value }
fn apply<P: Parser<u8, Output = i64>>(parser: P, input: u8) -> i64 {
    let parsed: <P as Parser<u8>>::Output = parser.parse(input);
    parsed
}
fn flag<P: Parser<bool, Output = bool>>(parser: P) -> bool { <P as Parser<bool>>::parse(parser, true) }
fn main() -> i64 { if flag(Number {}) { keep::<Number>(apply(Number {}, 2u8)) } else { 0 } }
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
query signature apply > "$fixture/signature"
grep -Fq 'Parser<u8, Output = i64>' "$fixture/signature"
query type keep::param:value > "$fixture/type"
grep -Fq 'value="<P as Parser<u8>>::Output"' "$fixture/type"
query references apply > "$fixture/references"
grep -Fq '"Parser::type:Output"' "$fixture/references"
grep -Fq '"Parser::parse"' "$fixture/references"
! grep -Fq '"Other::type:Output"' "$fixture/references"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Parser::type:Output","name":"Result"}]}
REQUEST
edit
value
grep -Fq 'Parser<u8, Result = i64>' "$fixture/project/main.lkj"
grep -Fq 'Parser<bool, Result = bool>' "$fixture/project/main.lkj"
grep -Fq 'trait Other { type Output; fn parse(self) -> Self::Output; }' "$fixture/project/main.lkj"
publish
query type keep::param:value > "$fixture/type"
grep -Fq 'value="<P as Parser<u8>>::Result"' "$fixture/type"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Parser::parse","name":"consume"}]}
REQUEST
edit
value
grep -Fq 'parser.consume(input)' "$fixture/project/main.lkj"
grep -Fq '<P as Parser<bool>>::consume(parser, true)' "$fixture/project/main.lkj"
grep -Fq 'trait Other { type Output; fn parse(self) -> Self::Output; }' "$fixture/project/main.lkj"
cp "$fixture/project/main.lkj" "$fixture/before"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"apply","source":"{ if <P as Parser<bool>>::consume(parser, true) { 42 } else { 0 } }"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rejected" 2> "$fixture/error"; then
    printf 'positional trait edit accepted a different argument witness\n' >&2
    exit 1
fi
grep -q 'E_TRAIT' "$fixture/error"
cmp "$fixture/before" "$fixture/project/main.lkj"
value
printf 'ok positional traits: applied signatures/projections/references, member/method rename, distinct argument witnesses and rejected body transaction on both engines\n'
