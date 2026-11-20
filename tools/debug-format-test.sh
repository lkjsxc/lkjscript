#!/usr/bin/env bash
# Debug formatting keeps ordinary source contracts and checked edit semantics.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
library="$PWD/library"
mkdir -p target
fixture=$(mktemp -d "$PWD/target/debug-format.XXXXXX")
trap 'if [[ ${LKJSCRIPT_DEBUG_KEEP:-0} == 1 ]]; then printf "retained_fixture=%s\n" "$fixture" >&2; else rm -rf -- "$fixture"; fi' EXIT HUP INT TERM
umask 077
mkdir "$fixture/project" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
fail() { printf 'debug-format: %s\n' "$*" >&2; exit 1; }
project="$fixture/project"
common=(main.lkj --source-dir "$project" --library-dir "$library" --store "$fixture/store" --store-key "$fixture/key")
publish() {
    "$compiler" build "${common[@]}" --index -o "$fixture/image" > "$fixture/index" 2> "$fixture/error" || { cat "$fixture/error" >&2; fail build; }
    snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/index")
    [[ ${#snapshot} == 64 ]] || fail 'missing indexed snapshot'
}
query() {
    "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$1" --id "$2" --depth 2 --limit 128 --bytes 65536
}
value() {
    for engine in native interpreter; do
        actual=$("$compiler" run main.lkj --source-dir "$project" --library-dir "$library" --no-store --engine "$engine")
        [[ $actual == "$1" ]] || fail "$engine returned $actual, expected $1"
    done
}
edit() {
    "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/plan" 2> "$fixture/error" || { cat "$fixture/error" >&2; fail edit; }
    grep -q 'mode=apply' "$fixture/plan" || fail 'edit was not applied'
}
cat > "$fixture/project/main.lkj" <<'SOURCE'
#[derive(Debug)]
struct Packet { code: i64 }
#[derive(Debug)]
struct Wrapper<T> { item: T }
#[derive(Debug)]
enum Choice<T> { Empty, One(T), Named { item: T } }
fn shown<T: std::fmt::Debug>(value: &T) -> String { format!("{:?}", value) }
fn value() -> i64 { 42 }
fn main() -> i64 {
    assert_eq!(shown(&Packet { code: 42i64 }), "Packet { code: 42 }");
    assert_eq!(shown(&Wrapper { item: 17i64 }), "Wrapper { item: 17 }");
    let empty: Choice<i64> = Choice::Empty;
    assert_eq!(shown(&empty), "Empty");
    assert_eq!(shown(&Choice::One(18i64)), "One(18)");
    assert_eq!(shown(&Choice::Named { item: 19i64 }), "Named { item: 19 }");
    value()
}
SOURCE
publish
value 42
query outline crate > "$fixture/outline"
query type shown::param:value > "$fixture/type"
grep -Fq 'value="&T"' "$fixture/type" || { cat "$fixture/type" >&2; fail 'generic source type'; }
query references shown > "$fixture/references"
grep -Fq 'std::fmt::Debug' "$fixture/references" || { cat "$fixture/references" >&2; fail 'Debug trait reference'; }
for nominal in Packet Wrapper Choice; do
    query impls "$nominal" > "$fixture/impls-$nominal"
    grep -Fq 'Debug' "$fixture/impls-$nominal" || { cat "$fixture/impls-$nominal" >&2; fail "derived implementation evidence for $nominal"; }
done
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"shown","name":"rendered"}]}
REQUEST
edit
grep -Fq 'rendered(&Packet' "$fixture/project/main.lkj" || fail 'caller was not renamed'
value 42
publish
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"value","source":"{ 43 }"}]}
REQUEST
edit
value 43
publish
cp "$fixture/project/main.lkj" "$fixture/unchanged"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"rendered","source":"{ format!(\"{:?}\", Missing) }"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/refused" 2> "$fixture/refused-error"; then fail 'invalid replacement accepted'; fi
cmp "$fixture/unchanged" "$fixture/project/main.lkj" || fail 'failed transaction wrote source'
[[ ! -e $fixture/project/.lkjscript-edit.pending ]] || fail 'failed transaction left pending state'
value 43
# Implicit captures keep their existing located edit refusal; no guessed byte span.
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"rendered","source":"{ format!(\"{value:?}\") }"}]}
REQUEST
edit
publish
query meta value@rendered > "$fixture/capture-meta"
grep -Eq '^"value@rendered".*references-complete=false' "$fixture/capture-meta" ||
    fail 'implicit capture did not mark its canonical owner partial'
query references value@rendered > "$fixture/capture-references"
grep -Eq '^"value@rendered".*fact=unresolved-reference.*kind=local-value( |$)' "$fixture/capture-references" ||
    fail 'implicit capture lost its unresolved local reference'
cp "$fixture/project/main.lkj" "$fixture/capture-unchanged"
cp "$fixture/project/.lkjscript-edit.generation" "$fixture/capture-generation"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"rendered","name":"captured"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/capture-plan" 2> "$fixture/capture-error"; then fail 'implicit capture rename guessed source spans'; fi
grep -Fq E_EDIT_REFERENCES "$fixture/capture-error" || { cat "$fixture/capture-error" >&2; fail 'capture refusal'; }
cmp "$fixture/capture-unchanged" "$fixture/project/main.lkj" || fail 'capture refusal wrote source'
cmp "$fixture/capture-generation" "$fixture/project/.lkjscript-edit.generation" || fail 'capture refusal changed generation'
[[ ! -e $fixture/project/.lkjscript-edit.pending ]] || fail 'capture refusal left pending state'
value 43

# Canonical ancestry must keep a same-named module independent of the function.
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"add-item","id":"crate","source":"mod rendered {}"}]}
REQUEST
edit
cp "$fixture/project/main.lkj" "$fixture/capture-with-module"
cp "$fixture/project/.lkjscript-edit.generation" "$fixture/capture-generation"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"value@rendered","name":"captured"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/canonical-capture-plan" 2> "$fixture/canonical-capture-error"; then
    fail 'canonical function rename ignored its implicit capture'
fi
grep -Fq E_EDIT_REFERENCES "$fixture/canonical-capture-error" || fail 'canonical capture refusal'
cmp "$fixture/capture-with-module" "$fixture/project/main.lkj" || fail 'canonical capture refusal wrote source'
cmp "$fixture/capture-generation" "$fixture/project/.lkjscript-edit.generation" || fail 'canonical capture refusal changed generation'
[[ ! -e $fixture/project/.lkjscript-edit.pending ]] || fail 'canonical capture refusal left pending state'
perl -0777 -e '
my $text=<>; my $before="mod rendered {}"; my $at=index($text,$before);
die "expected exactly one same-named module\n" unless $at>=0 && index($text,$before,$at+length($before))<0;
substr($text,$at,length($before),"mod rendered_module {}"); print $text;
' "$fixture/capture-with-module" > "$fixture/module-renamed-expected"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"type@rendered","name":"rendered_module"}]}
REQUEST
edit
cmp "$fixture/module-renamed-expected" "$fixture/project/main.lkj" || fail 'module rename changed the captured function'
publish
query meta value@rendered > "$fixture/capture-meta-after-module"
grep -Eq '^"value@rendered".*references-complete=false' "$fixture/capture-meta-after-module" ||
    fail 'module rename lost the function partial-reference metadata'
query meta type@rendered_module > "$fixture/renamed-module-meta"
value 43
cp "$fixture/project/main.lkj" "$fixture/capture-unchanged"

# Query the generated functions for concrete and generic structs and a generic
# enum, and refuse direct mutation of every unwritten body.
for target in Packet 'Wrapper<T>' 'Choice<T>'; do
    generated="<$target as std::fmt::Debug>::fmt"
    query meta "$generated" > "$fixture/generated-meta-$target"
    grep -Fq 'expansion=derive-debug source-projection=false' "$fixture/generated-meta-$target" || fail "generated Debug metadata for $target"
    query text "$generated" > "$fixture/generated-text-$target"
    grep -Fq 'fact=text-unavailable' "$fixture/generated-text-$target" || fail "generated Debug claims authored text for $target"
    printf '{"version":1,"operations":[{"op":"replace-body","id":"%s","source":"{ std::Result::Ok(()) }"}]}\n' "$generated" > "$fixture/request"
    if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/generated-plan-$target" 2> "$fixture/generated-error-$target"; then fail "generated body is editable for $target"; fi
    grep -Fq E_EDIT_GENERATED_SOURCE "$fixture/generated-error-$target" || { cat "$fixture/generated-error-$target" >&2; fail "generated edit refusal for $target"; }
    cmp "$fixture/capture-unchanged" "$fixture/project/main.lkj" || fail "generated refusal wrote source for $target"
    [[ ! -e $fixture/project/.lkjscript-edit.pending ]] || fail "generated refusal left pending state for $target"
done

mkdir "$fixture/contracts"
for source in ast diagnostic names syntax resolve source; do cp "src/$source.lkj" "$fixture/contracts/$source.lkj"; done
cp -R src/ast src/syntax src/resolve "$fixture/contracts/"
cp src/semantic/origins.lkj "$fixture/contracts/origins.lkj"
cp -R src/semantic/origins "$fixture/contracts/"
cp tests/fixtures/debug_origins.lkj "$fixture/contracts/main.lkj"
"$compiler" build main.lkj --source-dir "$fixture/contracts" --library-dir "$library" --no-store -o "$fixture/contracts-image" > "$fixture/contracts-out" 2> "$fixture/contracts-error" || { cat "$fixture/contracts-error" >&2; fail 'origin contract build'; }
"$fixture/contracts-image" >> "$fixture/contracts-out" 2>> "$fixture/contracts-error" || { cat "$fixture/contracts-error" >&2; fail 'origin contract execution'; }
printf 'ok Debug formatting: trait/type/implementation and concrete/generic struct/enum generated-origin queries; checked rename/body edit; capture and generated-edit refusals; independent same-named module rename; twelve malformed origins rejected; both engines\n'

# Map/set builders use ordinary selected library methods and exclusive loans.
mkdir "$fixture/collection-project"
project="$fixture/collection-project"
common=(main.lkj --source-dir "$fixture/collection-project" --library-dir "$library" --store "$fixture/store" --store-key "$fixture/key")
cat > "$fixture/collection-project/main.lkj" <<'SOURCE'
use std::fmt;
struct Entry(i64);
impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entry(&"n", &self.0).finish()
    }
}
fn shown(value: &Entry) -> String { format!("{:?}", value) }
fn main() -> i64 { shown(&Entry(42i64)).len() as i64 }
SOURCE
publish
value 9
query references '<Entry as std::fmt::Debug>::fmt' > "$fixture/map-references"
grep -Fq 'debug_map' "$fixture/map-references" || fail 'map method reference'
"$compiler" ir main.lkj --source-dir "$fixture/collection-project" --library-dir "$library" --no-store -o "$fixture/map.lkjir"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"<Entry as std::fmt::Debug>::fmt","source":"{ f.debug_set().entry(&self.0).finish() }"}]}
REQUEST
edit
value 4
publish
query references '<Entry as std::fmt::Debug>::fmt' > "$fixture/set-references"
grep -Fq 'debug_set' "$fixture/set-references" || fail 'set method reference'
if grep -Fq 'debug_map' "$fixture/set-references"; then fail 'stale map method reference'; fi
"$compiler" ir main.lkj --source-dir "$fixture/collection-project" --library-dir "$library" --no-store -o "$fixture/set.lkjir"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"shown","name":"rendered"}]}
REQUEST
edit
grep -Fq 'rendered(&Entry' "$fixture/collection-project/main.lkj" || fail 'collection caller rename'
value 4
publish
cp "$fixture/collection-project/main.lkj" "$fixture/collection-unchanged"
cp "$fixture/collection-project/.lkjscript-edit.generation" "$fixture/collection-generation"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"<Entry as std::fmt::Debug>::fmt","source":"{ let mut builder = f.debug_map(); let _ = f.write_str(\"conflict\"); builder.finish() }"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/collection-plan" 2> "$fixture/collection-error"; then
    fail 'builder loan conflict accepted by checked edit'
fi
grep -Fq E_BORROW "$fixture/collection-error" || { cat "$fixture/collection-error" >&2; fail 'builder loan refusal'; }
cmp "$fixture/collection-unchanged" "$fixture/collection-project/main.lkj" || fail 'builder refusal changed source'
cmp "$fixture/collection-generation" "$fixture/collection-project/.lkjscript-edit.generation" || fail 'builder refusal changed generation'
[[ ! -e $fixture/collection-project/.lkjscript-edit.pending ]] || fail 'builder refusal left pending state'
value 4

# An untrusted producer cannot turn either exclusive builder view into static data.
for builder in map set; do
    "$compiler" check "$fixture/$builder.lkjir" --no-store
    awk -v needle="::debug_$builder\$" '
        $1 == "fn" { selected = index($0, needle) != 0 }
        selected && $1 == "return-leaf" { changed += sub(/ (exact|bound) .*/, " static") }
        { print }
        END { if (changed != 1) exit 1 }
    ' "$fixture/$builder.lkjir" > "$fixture/$builder-forged.lkjir" || fail "$builder origin mutation target"
    if "$compiler" check "$fixture/$builder-forged.lkjir" --no-store > "$fixture/$builder-forged.out" 2> "$fixture/$builder-forged.err"; then
        fail "$builder forged static origin accepted"
    fi
    grep -Eq 'E_BORROW|E_RETURN' "$fixture/$builder-forged.err" || {
        cat "$fixture/$builder-forged.err" >&2; fail "$builder independent origin refusal";
    }
done
printf 'ok Debug map/set: selected method queries, checked replacement and rename, loan rollback, independent forged-origin refusal\n'
