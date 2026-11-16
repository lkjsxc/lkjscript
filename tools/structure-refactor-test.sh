#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
mkdir -p target/gate
"$compiler" build src/structure_refactor_cases.lkj -o target/gate/structure-refactor-cases
target/gate/structure-refactor-cases
fixture=$(mktemp -d target/compiler-refactor-cli.XXXXXX)
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
mkdir "$fixture/project"
head -c 32 /dev/urandom > "$fixture/key"
common=(main.lkj --source-dir "$fixture/project" --no-library --no-store --store-key "$fixture/key")
apply() {
    printf '{"version":1,"operations":[%s]}\n' "$1" > "$fixture/request.json"
    "$compiler" edit "${common[@]}" --request "$fixture/request.json" > "$fixture/plan"
    test ! -e "$fixture/project/.lkjscript-edit.pending"
}
value() {
    test "$("$compiler" run "$fixture/project/main.lkj" --no-library --no-store)" = "$1"
}
cat > "$fixture/project/main.lkj" <<'SOURCE'
fn pair(a: i64, b: i64) -> i64 { a * 10i64 + b }
pub fn probe() -> i64 { pair(1i64, 2i64) }
SOURCE
apply '{"op":"change-signature","id":"pair","parameters":["pair::param:b","pair::param:a",{"name":"bias","type":"i64","argument":"9i64"}]}'
value 12
cp "$fixture/project/main.lkj" "$fixture/before"
printf '%s\n' '{"version":1,"operations":[{"op":"change-signature","id":"pair","parameters":[]}]}' > "$fixture/request.json"
if "$compiler" edit "${common[@]}" --request "$fixture/request.json" > "$fixture/plan" 2> "$fixture/error"; then exit 1; fi
grep -q E_EDIT_CHECK "$fixture/error"
grep -q pair "$fixture/error"
cmp "$fixture/before" "$fixture/project/main.lkj"
cat > "$fixture/project/main.lkj" <<'SOURCE'
struct Point { x: i64 }
// untouched:  spacing stays
pub fn probe() -> i64 { let point = Point { x: 7i64 }; point.x }
SOURCE
apply '{"op":"add-field","id":"Point","source":"tag: i64","value":"9i64"}'
value 7
apply '{"op":"remove-field","id":"Point::field:tag"}'
value 7
grep -Fq '// untouched:  spacing stays' "$fixture/project/main.lkj"
cat > "$fixture/project/main.lkj" <<'SOURCE'
enum Choice { Ready(i64) }
pub fn probe() -> i64 { match Choice::Ready(7i64) { Choice::Ready(value) => value } }
SOURCE
apply '{"op":"add-variant","id":"Choice","source":"Later","arm":"Choice::Later => 0i64"}'
value 7
apply '{"op":"remove-variant","id":"Choice::variant:Ready","replacement":"Choice::Later"}'
value 0
cat > "$fixture/project/main.lkj" <<'SOURCE'
fn increment(value: i64) -> i64 { value + 1i64 }
pub fn probe() -> i64 { increment(41i64) }
SOURCE
apply '{"op":"inline-function","id":"increment","remove":true}'
value 42
if grep -q '^fn increment' "$fixture/project/main.lkj"; then exit 1; fi
printf '%s\n' 'pub fn probe() -> i64 { 20i64 + 22i64 }' > "$fixture/project/main.lkj"
mkdir "$fixture/store"
"$compiler" build main.lkj --source-dir "$fixture/project" --no-library --store "$fixture/store" --store-key "$fixture/key" --index -o "$fixture/extract-program" > "$fixture/index" 2>&1
snapshot=$(awk '$1 == "LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/index")
test "${#snapshot}" -eq 64
"$compiler" query main.lkj --source-dir "$fixture/project" --no-library --store "$fixture/store" --store-key "$fixture/key" --snapshot "$snapshot" --kind meta --id probe > "$fixture/meta"
perl - "$fixture/project/main.lkj" "$fixture/meta" > "$fixture/request.json" <<'PERL'
use strict; use warnings;
open my $file, '<:raw', shift or die $!; local $/; my $text=<$file>;
open my $meta, '<:raw', shift or die $!; my $facts=<$meta>;
$facts =~ /start=([0-9]+) end=([0-9]+).*item-sha256=([0-9a-f]{64})/ or die "missing source metadata";
my ($begin,$end,$digest)=($1,$2,$3);
$text=substr($text,$begin,$end-$begin);
my $expression='20i64 + 22i64'; my $start=index($text,$expression); die "missing expression" if $start < 0;
printf '{"version":1,"operations":[{"op":"extract-function","id":"probe","expect":"%s","name":"calculate","start":%d,"end":%d,"parameters":[],"result":"i64"}]}', $digest,$start,$start+length($expression);
print "\n";
PERL
"$compiler" edit "${common[@]}" --request "$fixture/request.json" > "$fixture/plan"
value 42
grep -q 'fn calculate' "$fixture/project/main.lkj"
# Inferred annotations recognize every capability in the resolver inventory.
# No host values are fabricated: these unused callers are elaborated/checked.
for capability in ForeignSession Image Channel Worker ReadRegion; do
    printf 'fn discard(value: %s) -> i64 { 7i64 }\nfn caller(value: %s) -> i64 { discard(value) }\npub fn probe() -> i64 { 0i64 }\n' "$capability" "$capability" > "$fixture/project/main.lkj"
    apply '{"op":"change-signature","id":"discard","parameters":[]}'
    grep -Eq "let .*: $capability =" "$fixture/project/main.lkj"
    value 0
done
# The existing conservative shadow rule must also cover the new capabilities.
printf '%s\n' 'mod other { pub struct Image { pub value: i64 } } fn discard(value: Image) -> i64 { 7i64 } fn caller(value: Image) -> i64 { discard(value) } pub fn probe() -> i64 { 0i64 }' > "$fixture/project/main.lkj"
cp "$fixture/project/main.lkj" "$fixture/annotation-before"
printf '%s\n' '{"version":1,"operations":[{"op":"change-signature","id":"discard","parameters":[]}]}' > "$fixture/request.json"
if "$compiler" edit "${common[@]}" --request "$fixture/request.json" > "$fixture/plan" 2> "$fixture/error"; then exit 1; fi
grep -q 'E_EDIT_TYPE_IDENTITY' "$fixture/error"
cmp "$fixture/annotation-before" "$fixture/project/main.lkj"
test ! -e "$fixture/project/.lkjscript-edit.pending"

printf 'ok structural refactor CLI: seven kinds, exact checked results, untouched bytes and checked refusal\n'
