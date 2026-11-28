#!/usr/bin/env bash
# Declarative expansions expose replayed facts and refuse invented edit spans.
set -euo pipefail
cd "$(dirname "$0")/.."
usage() { printf 'usage: %s [compiler [--native | --interpreter scope]]\n' "$0" >&2; exit 2; }
mode=${2:-all}
entry=${3:-main}
case "$mode:$#" in
    all:[01]|--native:2) ;;
    --interpreter:3)
        case "$entry" in
            expression_origins) cases=45 ;;
            type_origins) cases=13 ;;
            fragment_token_origins) cases=27 ;;
            lexical_origins) cases=3 ;;
            statement_origins) cases=18 ;;
            pattern_origins) cases=24 ;;
            *) usage ;;
        esac ;;
    *) usage ;;
esac
compiler=$(realpath "${1:-target/lkjscript}")
library="$PWD/library"
mkdir -p target
fixture=$(mktemp -d "$PWD/target/macro-origins.XXXXXX")
macro_label=macro-origins
if [[ $mode != all ]]; then macro_label+=" entry=$entry"; fi
macro_started=$SECONDS
macro_phase=setup
macro_phase_started=$SECONDS
phase() {
    printf '%s phase=%s status=0 seconds=%s\n' "$macro_label" "$macro_phase" "$((SECONDS - macro_phase_started))"
    macro_phase=$1
    macro_phase_started=$SECONDS
    printf '%s phase=%s started\n' "$macro_label" "$macro_phase"
}
cleanup() {
    local status=$?
    set +e
    trap - EXIT HUP INT TERM
    printf '%s phase=%s status=%s seconds=%s total_seconds=%s\n' \
        "$macro_label" "$macro_phase" "$status" "$((SECONDS - macro_phase_started))" "$((SECONDS - macro_started))"
    if ((status)); then
        # Only the synthetic contract output is exported, never the fixture key.
        for log in contracts-out contracts-error; do
            if [[ -s "$fixture/$log" ]]; then
                printf 'macro-origins diagnostic=%s\n' "$log" >&2
                cat "$fixture/$log" >&2
            fi
        done
    fi
    if [[ ${LKJSCRIPT_MACRO_KEEP:-0} == 1 ]] && rm -f -- "$fixture/key" && [[ ! -e "$fixture/key" ]]; then
        printf 'retained_fixture=%s\n' "$fixture" >&2
    else
        rm -rf -- "$fixture"
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 129' HUP
trap 'exit 143' TERM
umask 077
fail() { printf 'macro-origins: %s\n' "$*" >&2; exit 1; }
prepare_contracts() {
    mkdir "$fixture/contracts"
    for source in ast diagnostic names syntax resolve source; do cp "src/$source.lkj" "$fixture/contracts/$source.lkj"; done
    cp -R src/ast src/syntax src/resolve "$fixture/contracts/"
    cp src/semantic/origins.lkj "$fixture/contracts/origins.lkj"
    cp -R src/semantic/origins "$fixture/contracts/"
    cp tests/fixtures/macro_origins.lkj "$fixture/contracts/main.lkj"
}
contracts=(main.lkj --source-dir "$fixture/contracts" --library-dir "$library" --no-store)
run_interpreter() {
    phase contracts-interpreter
    "$compiler" run "${contracts[@]}" --engine interpreter "$@" >> "$fixture/contracts-out" 2>> "$fixture/contracts-error" \
        || { cat "$fixture/contracts-error" >&2; fail 'interpreted origin contracts'; }
}
if [[ $mode == --interpreter ]]; then
    prepare_contracts
    run_interpreter --entry "$entry"
    [[ $(<"$fixture/contracts-out") == "$cases" ]] || fail "expected $cases interpreted origin controls ($entry)"
    printf 'macro-origins scope=%s cases=%s\n' "$entry" "$cases"
    exit
fi

mkdir "$fixture/project" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
common=(main.lkj --source-dir "$fixture/project" --library-dir "$library" --store "$fixture/store" --store-key "$fixture/key")
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
        actual=$("$compiler" run main.lkj --source-dir "$fixture/project" --library-dir "$library" --no-store --engine "$engine")
        [[ $actual == "$1" ]] || fail "$engine returned $actual, expected $1"
    done
}
edit() {
    "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/plan" 2> "$fixture/error" || { cat "$fixture/error" >&2; fail edit; }
    grep -q 'mode=apply' "$fixture/plan" || fail 'edit was not applied'
}
refuse() {
    local expected=$1
    cp "$fixture/project/main.lkj" "$fixture/unchanged"
    if [[ -f "$fixture/project/.lkjscript-edit.generation" ]]; then
        cp "$fixture/project/.lkjscript-edit.generation" "$fixture/unchanged-generation"
    fi
    if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/refused" 2> "$fixture/refused-error"; then fail 'unsafe projection edit accepted'; fi
    grep -Fq "$expected" "$fixture/refused-error" || { cat "$fixture/refused-error" >&2; fail "missing $expected"; }
    cmp "$fixture/unchanged" "$fixture/project/main.lkj" || fail 'refusal wrote source'
    if [[ -f "$fixture/unchanged-generation" ]]; then
        cmp "$fixture/unchanged-generation" "$fixture/project/.lkjscript-edit.generation" || fail 'refusal changed generation'
    fi
    [[ ! -e $fixture/project/.lkjscript-edit.pending ]] || fail 'refusal left pending state'
    { cat "$fixture/request"; cat "$fixture/refused-error"; } >> "$fixture/refusals"
}
epoch() {
    sed -n 's/.*derived-epoch=\([0-9a-f]*\).*/\1/p' "$1" | head -1
}
phase source-queries
cat > "$fixture/project/main.lkj" <<'SOURCE'
macro_rules! make {
    ($name:ident) => {
        #[derive(Default, Debug)]
        struct Packet { code: i64 }
        fn $name() -> i64 { helper(40i64) }
    };
}
macro_rules! combine { ($value:expr) => { helper($value) }; }
macro_rules! relay { ($value:expr) => { combine!($value) }; }
macro_rules! read_code { ($input:expr) => { $input.code }; }
macro_rules! label { () => { include_str!("payload.txt") }; }
macro_rules! import_values {
    () => { use crate::imported::amount as imported_amount; use crate::borrowed::*; };
}
make!(generated);
import_values!();
mod imported { pub fn amount() -> i64 { 0i64 } }
mod borrowed { pub fn bonus() -> i64 { 0i64 } }
struct Input { code: i64 }
type CapturedType = i64;
macro_rules! typed_alias { ($t:ty) => { fn through_type(value: $t) -> $t { value } }; }
macro_rules! typed_local { ($t:ty, $e:expr) => {{ let value: $t = $e; value as i64 }}; }
typed_alias!(CapturedType);
fn typed_value() -> i64 { typed_local!(i64, 0i64) }
fn helper(value: i64) -> i64 { value + 1i64 }
fn field_value(input: &Input) -> i64 { read_code!(input) }
fn value() -> i64 { generated() + relay!(0i64) }
fn spare() -> i64 { 0i64 }
fn main() -> i64 {
    assert_eq!(format!("{:?}", Packet::default()), "Packet { code: 0 }");
    assert_eq!(label!().len(), 7usize);
    value() + spare() + field_value(&Input { code: 0i64 }) + imported_amount() + bonus()
        + through_type(0i64) + typed_value()
}
SOURCE
printf 'payload' > "$fixture/project/payload.txt"
publish
value 42
query text 'macro-definition:0:make' > "$fixture/definition-text"
grep -Fq 'macro_rules!' "$fixture/definition-text" || fail 'macro definition lost authored text'
query text 'macro-invocation:0' > "$fixture/invocation-text"
grep -Fq 'make!(generated)' "$fixture/invocation-text" || fail 'macro invocation lost authored text'
query meta generated > "$fixture/generated-meta"
grep -Fq 'expansion=macro-rules source-projection=false span=invocation-anchor' "$fixture/generated-meta" || fail 'macro projection origin'
initial_epoch=$(epoch "$fixture/generated-meta")
[[ ${#initial_epoch} == 64 ]] || fail 'macro expansion epoch missing'
query uses CapturedType > "$fixture/type-uses"
grep -Fq 'origin=generated expansion=macro-rules' "$fixture/type-uses" || fail 'captured type dependency missing'
query uses helper > "$fixture/helper-uses"
grep -Fq 'origin=generated expansion=macro-rules' "$fixture/helper-uses" || fail 'generated helper dependency missing'
query uses 'Input::field:code' > "$fixture/field-uses"
grep -Fq 'origin=generated expansion=macro-rules' "$fixture/field-uses" || fail 'generated typed field dependency missing'
for entry in 'imported:import-qualification' 'imported::amount:import' 'borrowed:glob-import'; do
    identity=${entry%:*}
    kind=${entry##*:}
    query uses "$identity" > "$fixture/import-uses"
    grep -Eq "origin=generated expansion=macro-rules .*kind=$kind([[:space:]]|$)" "$fixture/import-uses" \
        || { cat "$fixture/import-uses" >&2; fail "generated $kind dependency missing"; }
    if grep -Eq 'owner="crate".*kind=import' "$fixture/import-uses"; then
        fail 'generated import invented an authored use-tree spelling'
    fi
done
query references 'macro-expansion:0' > "$fixture/expansion-references"
grep -Fq 'macro-definition:0:make' "$fixture/expansion-references" || fail 'macro definition edge missing'
grep -Fq 'macro-invocation:0' "$fixture/expansion-references" || fail 'macro invocation edge missing'
query outline 'macro-expansion:0' > "$fixture/expansion-outline"
grep -Fq 'kind=call-site' "$fixture/expansion-outline" || fail 'generated call identity missing'
for id in generated Packet 'Packet::field:code' '<Packet as std::fmt::Debug>::fmt'; do
    query text "$id" > "$fixture/projected-text"
    grep -Fq 'fact=text-unavailable' "$fixture/projected-text" || fail "generated $id claims source text"
done
query meta '<Packet as std::fmt::Debug>::fmt' > "$fixture/debug-meta"
grep -Fq 'expansion=derive-debug source-projection=false' "$fixture/debug-meta" || fail 'macro-generated derive lost its strict Debug origin'

cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"spare","source":"{ 1i64 }"}]}
REQUEST
edit
value 43
publish
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"spare","name":"offset"}]}
REQUEST
edit
grep -Fq 'offset()' "$fixture/project/main.lkj" || fail 'authored caller rename missing'
value 43
publish
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"value","source":"{ generated() + relay!(1i64) }"}]}
REQUEST
edit
value 44
publish
query meta generated > "$fixture/after-body-meta"
[[ $(epoch "$fixture/after-body-meta") == "$initial_epoch" ]] || fail 'unrelated authored body changed generated item epoch'

# Fragment shape checking keeps tokens unexpanded, but publication must still
# expand and check them in this project's real macro scope.
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"value","source":"{ generated() + relay!(1i64, 2i64) }"}]}
REQUEST
refuse E_EDIT_CHECK
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"value","source":"{ generated() + relay!(true) }"}]}
REQUEST
refuse E_EDIT_CHECK
value 44

cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"generated","source":"{ 0i64 }"}]}
REQUEST
refuse E_EDIT_GENERATED_SOURCE
for id in Packet 'Packet::field:code' 'macro-expansion:0'; do
    printf '{"version":1,"operations":[{"op":"rename","id":"%s","name":"invented"}]}\n' "$id" > "$fixture/request"
    refuse E_EDIT_GENERATED_SOURCE
done
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"helper","name":"renamed"}]}
REQUEST
refuse E_EDIT_GENERATED_REFERENCE
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Input::field:code","name":"renamed"}]}
REQUEST
refuse E_EDIT_GENERATED_REFERENCE
for identity in imported 'imported::amount' borrowed CapturedType; do
    printf '{"version":1,"operations":[{"op":"rename","id":"%s","name":"changed_import"}]}\n' "$identity" > "$fixture/request"
    refuse E_EDIT_GENERATED_REFERENCE
done
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"<Packet as std::fmt::Debug>::fmt","source":"{ std::Result::Ok(()) }"}]}
REQUEST
refuse E_EDIT_GENERATED_SOURCE

# A real template change and a changed immutable include receipt both affect
# expanded identities; same-length data must not reuse an old body epoch.
perl -0pi -e 's/helper\(40i64\)/helper(41i64)/' "$fixture/project/main.lkj"
publish
value 45
query meta generated > "$fixture/changed-template-meta"
template_epoch=$(epoch "$fixture/changed-template-meta")
[[ ${#template_epoch} == 64 && $template_epoch != "$initial_epoch" ]] || fail 'template content did not affect macro epoch'
query meta '<Packet as std::fmt::Debug>::fmt' > "$fixture/changed-debug-meta"
[[ $(epoch "$fixture/changed-debug-meta") != "$(epoch "$fixture/debug-meta")" ]] || fail 'macro-created derive epoch omitted its template'
printf 'changed' > "$fixture/project/payload.txt"
publish
value 45
query meta generated > "$fixture/changed-input-meta"
[[ $(epoch "$fixture/changed-input-meta") != "$template_epoch" ]] || fail 'include content did not affect macro epoch'

# An authored body can change the type fragment; a bad replacement leaves both
# the source and transaction generation unchanged.
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"typed_value","source":"{ typed_local!(u8, 1u8) }"}]}
REQUEST
edit
value 46
publish
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"typed_value","source":"{ typed_local!(bool, 1u8) }"}]}
REQUEST
refuse E_EDIT_CHECK
value 46

# Type captures grant no return-origin authority to the producer.
cat > "$fixture/project/view.lkj" <<'SOURCE'
macro_rules! borrowed { ($t:ty) => { fn borrowed<'a>(value: $t) -> $t { value } }; }
borrowed!(&'a i64);
fn main() -> i64 { let value = 42i64; *borrowed(&value) }
SOURCE
"$compiler" ir view.lkj --source-dir "$fixture/project" --library-dir "$library" --no-store -o "$fixture/view.lkjir"
"$compiler" check "$fixture/view.lkjir" --no-store
awk '
    $1 == "fn" { selected = index($0, "borrowed") != 0 }
    selected && $1 == "return-leaf" { changed += sub(/ (exact|bound) .*/, " static") }
    { print }
    END { if (changed != 1) exit 1 }
' "$fixture/view.lkjir" > "$fixture/forged-view.lkjir" || fail 'captured view origin mutation target'
if "$compiler" check "$fixture/forged-view.lkjir" --no-store > "$fixture/forged-out" 2> "$fixture/forged-error"; then
    fail 'captured type forged a static return origin'
fi
grep -Eq 'E_BORROW|E_RETURN' "$fixture/forged-error" || fail 'independent return-origin refusal missing'

# Local identities include their current token-body epoch; always obtain them
# from the indexed source instead of constructing an identity across edits.
raw_binding() {
    query outline 'value@main' > "$fixture/raw-binding-outline"
    local found
    found=$(awk -F '\t' -v suffix="::binding:$1:0\"" '$1 ~ /^"value@main::body:/ && index($1, suffix) { print substr($1,2,length($1)-2); exit }' "$fixture/raw-binding-outline")
    [[ -n $found ]] || fail "missing raw binding $1"
    printf '%s' "$found"
}
raw_rename_binding() {
    local identity
    identity=$(raw_binding "$1")
    printf '{"version":1,"operations":[{"op":"rename","id":"%s","name":"%s"}]}\n' "$identity" "$2" > "$fixture/request"
}
raw_body_digest() {
    query meta 'value@main' > "$fixture/raw-body-meta"
    sed -n 's/.* body=\([0-9a-f]*\) .*/\1/p' "$fixture/raw-body-meta"
}
phase raw-identifier-edits
cat > "$fixture/project/main.lkj" <<'SOURCE'
mod r#type { pub fn r#fn() -> i64 { 42i64 } }
struct Packet { r#match: i64 }
fn main() -> i64 {
    let r#mut = r#type::r#fn();
    let Packet { r#match } = Packet { r#match: r#mut };
    r#match
}
SOURCE
publish
query meta 'value@type::fn' > "$fixture/raw-function-meta"
query uses 'value@type::fn' > "$fixture/raw-function-refs"
raw_site=$(grep -E '^"value@main"[[:space:]].*fact=uses[[:space:]].*target="value@type::fn"' "$fixture/raw-function-refs")
raw_start=$(sed -n 's/.* start=\([0-9]*\) end=.*/\1/p' <<< "$raw_site")
raw_end=$(sed -n 's/.* end=\([0-9]*\) kind=.*/\1/p' <<< "$raw_site")
[[ $raw_start =~ ^[0-9]+$ && $raw_end =~ ^[0-9]+$ && $((raw_end - raw_start)) == 4 ]] || fail 'raw call query lost full token extent'
[[ $(dd if="$fixture/project/main.lkj" bs=1 skip="$raw_start" count=4 status=none) == 'r#fn' ]] || fail 'raw call query chose the wrong source spelling'
query signature 'value@type::fn' > "$fixture/raw-function-signature"
grep -Fq 'r#fn' "$fixture/raw-function-signature" || fail 'raw function query lost source spelling'
value 42
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"value@type::fn","name":"r#return"}]}
REQUEST
edit
value 42
publish
grep -Fq 'fn r#return()' "$fixture/project/main.lkj" || fail 'raw name extent chose fn keyword'
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"type@type","name":"r#mod"}]}
REQUEST
edit
value 42
publish
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Packet::field:match","name":"r#type"}]}
REQUEST
edit
value 42
publish
grep -Fq 'r#type: r#match' "$fixture/project/main.lkj" || fail 'field shorthand lost raw binding spelling'
raw_rename_binding match renamed
edit
value 42
publish
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"value@mod::return","source":"{ let r#match = 41i64; r#match + 1i64 }"}]}
REQUEST
edit
value 42
publish
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"value@mod::return","source":"{ let r#match = true; r#match }"}]}
REQUEST
refuse E_EDIT_CHECK
for name in r#self r#Self r#super r#crate r#_ mut; do
    printf '{"version":1,"operations":[{"op":"rename","id":"value@mod::return","name":"%s"}]}\n' "$name" > "$fixture/request"
    refuse E_EDIT_NAME
done
# Same symbol with different spelling still collides, and raw literal-arm
# selection remains covered by the ordinary capture-replay and meaning proof.
cat > "$fixture/project/main.lkj" <<'SOURCE'
macro_rules! read { ($value:ident) => { $value }; }
fn plain() -> i64 { 0i64 }
fn r#type() -> i64 { 1i64 }
fn main() -> i64 { let r#mut = 42i64; read!(r#mut) }
SOURCE
publish
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"value@type","name":"r#plain"}]}
REQUEST
refuse E_EDIT_NAME
raw_rename_binding mut r#value
edit
value 42
publish
raw_digest=$(raw_body_digest)
raw_rename_binding value value
edit
value 42
publish
plain_digest=$(raw_body_digest)
[[ $raw_digest =~ ^[0-9a-f]{64}$ && $plain_digest =~ ^[0-9a-f]{64}$ && $raw_digest != "$plain_digest" ]] || fail 'raw/plain spelling collapsed token-body digest'
cat > "$fixture/project/main.lkj" <<'SOURCE'
macro_rules! choose { (r#value) => { 0i64 }; ($value:ident) => { $value }; }
fn main() -> i64 { let value = 42i64; choose!(value) }
SOURCE
publish
raw_rename_binding value r#value
refuse E_EDIT_REFERENCES
value 42

phase contract-build
prepare_contracts
"$compiler" build "${contracts[@]}" -o "$fixture/contracts-image" > "$fixture/contracts-out" 2> "$fixture/contracts-error" || { cat "$fixture/contracts-error" >&2; fail 'origin contract build'; }
phase contracts-native
"$fixture/contracts-image" >> "$fixture/contracts-out" 2>> "$fixture/contracts-error" || { cat "$fixture/contracts-error" >&2; fail 'origin contract execution'; }
if [[ $mode == --native ]]; then exit; fi
run_interpreter
printf 'ok declarative macro origins: authenticated source/topology and include replay; generated item, field, import, call and strict derive queries; checked authored rename/body edits; projection/reference refusals; template/include epochs; 130 malformed origins; statement positions/splices/roles/rules; pattern aliases/captures/exports/parameters; typed capture queries/edits, structural block ownership and independent return-origin refusal; both engines\n'
