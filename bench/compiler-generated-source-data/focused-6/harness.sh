#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
baseline=${2:-}
if [[ -n "$baseline" ]]; then baseline=$(realpath "$baseline"); fi
fixture=$(mktemp -d target/compiler-generated-source.XXXXXX)
fixture=$(realpath "$fixture")
finish() {
    local status=$?
    trap - EXIT HUP INT TERM
    if [[ -f "$fixture/source/main.lkj" ]]; then cp "$fixture/source/main.lkj" "$fixture/final-source.txt"; fi
    if [[ -n "${LKJSCRIPT_GENERATED_EVIDENCE:-}" ]]; then
        mkdir -p "$LKJSCRIPT_GENERATED_EVIDENCE"
        while IFS= read -r -d '' file; do
            [[ $(basename "$file") == key ]] && continue
            if [[ ! -s "$file" ]] || LC_ALL=C grep -Iq '' "$file"; then cp "$file" "$LKJSCRIPT_GENERATED_EVIDENCE/"; fi
        done < <(find "$fixture" -maxdepth 1 -type f -print0)
    fi
    if [[ $status != 0 ]]; then
        for file in "$fixture/build.err" "$fixture/query.err" "$fixture/edit.err" "$fixture/contracts.err"; do
            [[ ! -f "$file" ]] || cat "$file" >&2
        done
    fi
    rm -rf "$fixture"
    exit "$status"
}
trap finish EXIT
trap 'exit 130' HUP INT TERM
umask 077
head -c 32 /dev/urandom > "$fixture/key"
mkdir "$fixture/source" "$fixture/store" "$fixture/desired" "$fixture/contracts"
fail() { printf 'generated-source: %s\n' "$*" >&2; exit 1; }
cat > "$fixture/source/main.lkj" <<'SOURCE'
mod external;
trait Marker {}
#[derive(Default)]
struct FieldType { value: i64 }
impl Marker for FieldType {}
#[derive(Default)]
struct Record { record_field: FieldType }
#[derive(Default)]
struct Generic<T: Marker> { fixed: FieldType, generic: T }
#[derive(Default)]
struct Tuple(FieldType);
#[derive(Default)]
struct Unit;
#[derive(Default)]
struct Spare { code: i64 }
struct Manual;
impl Default for Manual { fn default() -> Self { Manual } }
fn body() -> i64 { 4i64 }
pub fn probe() -> i64 {
    let record = Record::default();
    let generic = Generic::<FieldType>::default();
    let tuple = Tuple::default();
    let unit = Unit::default();
    let spare = Spare::default();
    let manual = Manual::default();
    record.record_field.value + generic.fixed.value + generic.generic.value + tuple.0.value
        + body() + external::value()
}
SOURCE
printf '%s\n' 'pub fn value() -> i64 { 2i64 }' > "$fixture/source/external.lkj"
cp "$fixture/source/main.lkj" "$fixture/original.lkj"
cp "$fixture/source/external.lkj" "$fixture/desired/external.lkj"
common=(main.lkj --source-dir "$fixture/source" --store "$fixture/store" --store-key "$fixture/key" --store-bytes 268435456)
generated='<Record as std::prelude::Default>::default'
manual='<Manual as std::prelude::Default>::default'
build() {
    "$compiler" build "${common[@]}" --index --stats -o "$fixture/image" > "$fixture/build.out" 2> "$fixture/build.err"
    snapshot=$(sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]*\) .*/\1/p' "$fixture/build.out")
    [[ ${#snapshot} == 64 ]] || snapshot=$(sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]*\) .*/\1/p' "$fixture/build.err")
    [[ ${#snapshot} == 64 ]] || fail 'snapshot missing'
}
query() { "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$1" --id "$2" --depth 1 --limit 96 --bytes 32768 > "$3" 2> "$fixture/query.err"; }
edit() {
    local status=0
    if "$compiler" edit "${common[@]}" --no-store --store-key "$fixture/key" --request "$1" "${@:2}" > "$fixture/edit.out" 2> "$fixture/edit.err"; then status=0; else status=$?; fi
    { cat "$1"; cat "$fixture/edit.out"; cat "$fixture/edit.err"; printf 'exit=%s\n' "$status"; } >> "$fixture/edits.log"
    return "$status"
}
refuse() {
    local expected=$1 request=$2
    sha256sum "$fixture/source/"*.lkj > "$fixture/before.sha256"
    if edit "$request"; then fail 'generated edit unexpectedly accepted'; fi
    grep -Fq "$expected" "$fixture/edit.err" || fail "missing $expected refusal"
    sha256sum "$fixture/source/"*.lkj > "$fixture/after.sha256"
    cmp "$fixture/before.sha256" "$fixture/after.sha256"
    if find "$fixture/source" -maxdepth 1 -name '.lkjscript*pending*' | grep -q .; then fail 'refusal published an intent'; fi
    { cat "$request"; cat "$fixture/edit.err"; } >> "$fixture/refusals.log"
}
request() {
    # These fixture IDs and fragments contain no JSON metacharacters.
    printf '{"version":1,"operations":[{"op":"%s","id":"%s","source":"%s"}]}\n' "$1" "$2" "$3" > "$fixture/request.json"
}
if [[ -n "$baseline" ]]; then
    "$baseline" build "${common[@]}" --index -o "$fixture/old-image" > "$fixture/old-build.out" 2> "$fixture/old-build.err"
    "$baseline" query "${common[@]}" --kind text --id "$generated" > "$fixture/old-text" 2> "$fixture/old-query.err"
    grep -Fq 'struct Record' "$fixture/old-text" || fail 'baseline no longer exposes the captured fake projection'
    printf 'baseline: generated method text incorrectly contains its defining struct\n' > "$fixture/baseline-observation"
fi
build
query meta "$generated" "$fixture/generated-meta"
grep -Fq 'origin=generated definition="Record" expansion=derive-default source-projection=false span=definition-anchor' "$fixture/generated-meta"
grep -Fq 'body-locals=derived-declaration-epoch' "$fixture/generated-meta"
query text "$generated" "$fixture/generated-text"
grep -Fq 'fact=text-unavailable' "$fixture/generated-text"
if grep -Fq 'struct Record' "$fixture/generated-text"; then fail 'generated text exposes struct projection'; fi
query item "$generated" "$fixture/generated-item"
grep -Fq 'origin=generated definition="Record"' "$fixture/generated-item"
query outline crate "$fixture/outline"
implementation=$(sed -n 's/^"\(impl:[^"]*\)".*origin=generated definition="Record".*/\1/p' "$fixture/outline" | head -1)
[[ -n "$implementation" ]] || fail 'generated implementation absent'
query outline "$generated" "$fixture/generated-calls"
call=$(sed -n 's/^"\([^"]*::derived:[0-9a-f]*::call:[0-9]*\)".*/\1/p' "$fixture/generated-calls" | head -1)
[[ -n "$call" ]] || fail 'generated call identity absent'
if grep -Fq '::body:' "$fixture/generated-calls"; then fail 'old source-body alias retained'; fi
query text "$call" "$fixture/generated-call-text"
grep -Fq 'fact=text-unavailable' "$fixture/generated-call-text"
query meta "$call" "$fixture/generated-call-meta"
epoch=$(printf '%s' "$call" | sed -n 's/.*::derived:\([0-9a-f]*\)::call:.*/\1/p')
grep -Fq "derived-epoch=$epoch " "$fixture/generated-call-meta"
query uses FieldType "$fixture/type-uses"
grep -Fq 'source-projection=false' "$fixture/type-uses"
grep -Fq 'kind=type' "$fixture/type-uses"
query uses Marker "$fixture/bound-uses"
grep -Fq 'kind=type' "$fixture/bound-uses"
query meta "$manual" "$fixture/manual-meta"
if grep -Fq 'origin=generated' "$fixture/manual-meta"; then fail 'authored method classified as generated'; fi
for operation in replace-body replace-item remove-item rename move inline-function; do
    request "$operation" "$generated" '{ 0i64 }'
    refuse E_EDIT_GENERATED_SOURCE "$fixture/request.json"
    grep -Fq 'edit defining item "Record"' "$fixture/edit.err"
done
request add-item "$implementation" 'fn extra() {}'
refuse E_EDIT_GENERATED_SOURCE "$fixture/request.json"
request replace-item "$call" 'fn extra() {}'
refuse E_EDIT_GENERATED_SOURCE "$fixture/request.json"
request change-signature 'std::prelude::__lkjscript_default' ''
refuse E_EDIT_GENERATED_REFERENCE "$fixture/request.json"
# A new small fixture, never the preserved historical case.
sed 's/struct Record { record_field: FieldType }/struct Record { record_field: FieldType, extra: i64 }/' "$fixture/original.lkj" > "$fixture/desired/main.lkj"
"$compiler" edit "${common[@]}" --no-store --store-key "$fixture/key" --against main.lkj --against-source-dir "$fixture/desired" > "$fixture/replay.out" 2> "$fixture/edit.err"
grep -Eq '^replay desired=([0-9a-f]{64}) candidate=\1 verified=true operations=[1-9]' "$fixture/replay.out"
cmp "$fixture/source/main.lkj" "$fixture/desired/main.lkj"
old_call=$call
build
query outline "$generated" "$fixture/generated-calls-after"
if grep -Fq "$old_call" "$fixture/generated-calls-after"; then fail 'derived epoch survived defining field change'; fi
cat > "$fixture/rename.json" <<'JSON'
{"version":1,"operations":[{"op":"rename","id":"FieldType","name":"RenamedField"},{"op":"rename","id":"Marker","name":"RenamedMarker"}]}
JSON
edit "$fixture/rename.json"
grep -Fq 'struct RenamedField' "$fixture/source/main.lkj"
grep -Fq 'trait RenamedMarker' "$fixture/source/main.lkj"
grep -Fq 'fixed: crate::RenamedField' "$fixture/source/main.lkj"
grep -Fq 'T: crate::RenamedMarker' "$fixture/source/main.lkj"
cat > "$fixture/field.json" <<'JSON'
{"version":1,"operations":[{"op":"rename","id":"Record::field:record_field","name":"renamed_record_field"}]}
JSON
# This authored associated-call receiver has no exact typed field evidence yet.
refuse E_EDIT_REFERENCES "$fixture/field.json"
cat > "$fixture/spare-field.json" <<'JSON'
{"version":1,"operations":[{"op":"rename","id":"Spare::field:code","name":"renamed_code"}]}
JSON
edit "$fixture/spare-field.json"
grep -Fq 'struct Spare { renamed_code: i64 }' "$fixture/source/main.lkj"
test "$("$compiler" run main.lkj --source-dir "$fixture/source" --no-store)" = 6
build
"$compiler" build "${common[@]}" --stats -o "$fixture/image" > "$fixture/unchanged.out" 2> "$fixture/unchanged.err"
request replace-body body '{ 5i64 }'
"$compiler" edit "${common[@]}" --stats --request "$fixture/request.json" > "$fixture/resumed-edit.out" 2> "$fixture/resumed-edit.err"
grep -Eq 'frontend-resumed=[1-9][0-9]*' "$fixture/resumed-edit.out"
test "$("$compiler" run main.lkj --source-dir "$fixture/source" --no-store)" = 7
query text "$generated" "$fixture/historical-text"
grep -Fq 'fact=text-unavailable' "$fixture/historical-text"
# Classifier contract: exact production dependencies, only reporting data stubbed.
for source in ast diagnostic names syntax resolve source; do cp "src/$source.lkj" "$fixture/contracts/$source.lkj"; done
cp -R src/syntax src/resolve "$fixture/contracts/"
cp src/semantic/origins.lkj "$fixture/contracts/origins.lkj"
cat > "$fixture/contracts/main.lkj" <<'SOURCE'
mod ast; mod diagnostic; mod names; mod syntax; mod resolve; mod source; mod origins;
mod diagnostics {
    #[derive(Clone)]
    pub struct BlockedRegion { pub path: String, pub span: crate::ast::Span, pub kind: String, pub reason: String }
}
fn parsed() -> ast::Program {
    let root = "mod external; trait Bound {} struct Field; #[derive(Default)] struct Named<T:Bound> { field:Field, value:T } #[derive(Default)] struct Tuple(Field); #[derive(Default)] struct Unit; struct Manual; impl std::default::Default for Manual { fn default()->Self { Manual } } fn free() { fn nested() {} nested(); }";
    let standard = "pub mod default { pub trait Default { fn default()->Self; } pub fn __lkjscript_default<T:Default>()->T { T::default() } }";
    let report = source::load_report("main.lkj", Some("$std/lib.lkj"), &mut |path:&str| -> Result<Option<Vec<u8>>,String> {
        Ok(match path { "main.lkj"=>Some(root.as_bytes().to_vec()), "$std/lib.lkj"=>Some(standard.as_bytes().to_vec()), "external.lkj"=>Some(b"pub fn external_function() {}".to_vec()), _=>None })
    });
    let program = report.into_result().unwrap();
    resolve::resolve(&program).unwrap()
}
fn refused(program:&ast::Program) {
    let resolver = resolve::Resolver::new(program).unwrap();
    match origins::collect(program, &resolver) { Err(message)=>assert!(message.contains("E_IDENTITY_ORIGIN")), Ok(_)=>panic!("unexplained origin accepted") }
}
fn main() {
    let program = parsed();
    let resolver = resolve::Resolver::new(&program).unwrap();
    let relation = origins::collect(&program, &resolver).unwrap();
    assert!(relation.functions.iter().filter(|value| value.is_some()).count() == 3usize);
    assert!(relation.implementations.iter().filter(|value| value.is_some()).count() == 3usize);
    for definition in &program.structs {
        for field in &definition.fields { assert!(!relation.types.contains_key(&field.ty)); }
        for parameter in &definition.generics {
            for bound in &parameter.bounds { assert!(!relation.types.contains_key(bound)); }
        }
    }
    let free = program.functions.iter().position(|value| value.name == "free").unwrap();
    let mut missing = program.clone();
    missing.items.retain(|item| *item != ast::Item::Function(free));
    refused(&missing);
    let explicit = program.impls.iter().position(|value| {
        value.methods.iter().any(|index| relation.functions[*index].is_none())
    }).unwrap();
    let mut missing_impl = program.clone();
    missing_impl.items.retain(|item| *item != ast::Item::Impl(explicit));
    refused(&missing_impl);
    let generated = relation.functions.iter().position(|value| value.is_some()).unwrap();
    let mut wrong_owner = program.clone();
    wrong_owner.function_owners[generated] = ast::FunctionOwner::Free;
    wrong_owner.items.push(ast::Item::Function(generated));
    refused(&wrong_owner);
    let mut wrong_body = program.clone();
    wrong_body.functions[generated].body = program.functions[free].body;
    refused(&wrong_body);
    let mut wrong_target = program.clone();
    let derived_impl = relation.implementations.iter().position(|value| value.is_some()).unwrap();
    wrong_target.impls[derived_impl].target = program.impls[explicit].target;
    refused(&wrong_target);
}
SOURCE
cp "$fixture/contracts/main.lkj" "$fixture/contracts-source.lkj"
"$compiler" build main.lkj --source-dir "$fixture/contracts" --no-store -o "$fixture/contracts-image" > "$fixture/contracts.out" 2> "$fixture/contracts.err"
"$fixture/contracts-image" >> "$fixture/contracts.out" 2>> "$fixture/contracts.err"
printf 'generated-source: projection, shared authored references, replay, refusals and malformed origins passed\n'
