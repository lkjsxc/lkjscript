#!/usr/bin/env bash
# RCT-1 E1(a): twelve frozen checked-contract requirement cases on ordinary edit.
# Every expected verdict below is authored by hand, not produced by the fact
# emitter. Two compilers built with one omitted fact each must fail this oracle.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
fixture=$(mktemp -d "$PWD/target/compiler-structure-require.XXXXXX")
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
umask 077
head -c 32 /dev/urandom > "$fixture/key"
reuse=()
if [[ -n "${LKJSCRIPT_STORE_KEY:-}" && -n "${LKJSCRIPT_STORE:-}" ]]; then
    store=$LKJSCRIPT_STORE
    case "$store" in /*) ;; *) store="$PWD/$store";; esac
    reuse=(--store "$store" --store-key "$LKJSCRIPT_STORE_KEY")
fi

fail() {
    printf 'require case %s: %s\n' "$name" "$*" >&2
    cat "$fixture/out" "$fixture/error" >&2 || true
    exit 1
}
# project NAME LIBRARY: create a fresh project and initialize its writer lock.
project() {
    name=$1
    directory="$fixture/$name"
    mkdir "$directory"
    cat > "$directory/main.lkj"
    if [[ $2 == library ]]; then libraries=(--library-dir "$PWD/library"); else libraries=(--no-library); fi
    "$compiler" edit main.lkj --source-dir "$directory" "${libraries[@]}" --no-store \
        --recover --store-key "$fixture/key" > "$fixture/out" 2> "$fixture/error" || fail 'writer initialization failed'
    grep -qx 'transaction recovered-files=0' "$fixture/out" || fail 'unexpected recovery'
}
request() { cat > "$fixture/$name.json"; }
edit() {
    local binary=$1
    shift
    "$binary" edit main.lkj --source-dir "$directory" "${libraries[@]}" --no-store \
        --request "$fixture/$name.json" "$@" > "$fixture/out" 2> "$fixture/error"
}
# Source files and writer metadata: names, kinds, sizes, modes and bytes.
state() {
    (cd "$directory" && find . -printf '%P\t%y\t%s\t%m\n' | LC_ALL=C sort \
        && find . -type f -exec sha256sum {} + | LC_ALL=C sort)
}
verdicts() {
    local file=$1
    shift
    grep '^require ' "$file" > "$fixture/actual" || true
    printf '%s\n' "$@" > "$fixture/expected"
    cmp -s "$fixture/expected" "$fixture/actual"
}
# refuse CODE REASON [REQUIRE_LINE ...]: an applied transaction must refuse
# with this code and exact verdict rows, leaving every byte in place.
refuse() {
    local code=$1 reason=$2
    shift 2
    state > "$fixture/before"
    if edit "$compiler" --store-key "$fixture/key"; then fail 'transaction was accepted'; fi
    [[ ! -s "$fixture/out" ]] || fail 'refusal wrote a report'
    grep -Fq "$code" "$fixture/error" || fail "expected $code"
    grep -Fq "$reason" "$fixture/error" || fail "expected reason $reason"
    if (($#)); then verdicts "$fixture/error" "$@" || fail 'unexpected verdict rows'
    elif grep -q '^require ' "$fixture/error"; then fail 'verdict before refusal'; fi
    state > "$fixture/after"
    cmp -s "$fixture/before" "$fixture/after" || fail 'refusal changed source or writer state'
    printf 'ok require %s\n' "$name"
}

# 1. A valid body edit under disjoint field views keeps all three contracts.
document_source() {
    cat <<'SOURCE'
struct Document { nodes: i64, style: i64 }
fn recalculate(nodes: &i64, style: &mut i64) { *style = *nodes + 1i64; }
fn update(document: &mut Document) -> i64 {
    recalculate(&document.nodes, &mut document.style);
    document.style
}
pub fn main() -> i64 {
    let mut document = Document { nodes: 40i64, style: 0i64 };
    update(&mut document)
}
SOURCE
}
project valid plain < <(document_source)
digest=$(sed -n 3,6p "$directory/main.lkj" | head -c -1 | sha256sum | cut -d' ' -f1)
valid_request() {
    printf '{"version":1,%s"operations":[{"op":"replace-body","id":"update","expect":"%s",' "$1" "$digest"
    printf '"source":"{ recalculate(&document.nodes, &mut document.style); document.style + 1i64 }"}],'
    printf '"require":[{"id":"update","predicate":"same-signature"},{"id":"update","predicate":"same-return-origins"},'
    printf '{"id":"update","predicate":"same-effect-summary"}]}\n'
}
valid_rows=(
    'require "value@update" predicate=same-signature verdict=PROVEN level=checked-ir instances=1'
    'require "value@update" predicate=same-return-origins verdict=PROVEN level=checked-ir instances=1'
    'require "value@update" predicate=same-effect-summary verdict=PROVEN level=checked-ir instances=1'
)
valid_request '' | request
state > "$fixture/before"
edit "$compiler" --dry-run || fail 'guarded dry run was refused'
verdicts "$fixture/out" "${valid_rows[@]}" || fail 'unexpected dry-run verdicts'
grep -q 'mode=dry-run' "$fixture/out" || fail 'missing dry-run report'
state > "$fixture/after"
cmp -s "$fixture/before" "$fixture/after" || fail 'dry run changed source or writer state'
snapshot=$(sed -n 's/^transaction source-before=\([0-9a-f]*\) .*/\1/p' "$fixture/out")
[[ ${#snapshot} == 64 ]] || fail 'missing source snapshot'
valid_request "\"snapshot\":\"$snapshot\"," | request
edit "$compiler" --store-key "$fixture/key" || fail 'guarded commit was refused'
verdicts "$fixture/out" "${valid_rows[@]}" || fail 'unexpected commit verdicts'
grep -q 'mode=apply' "$fixture/out" || fail 'missing apply report'
[[ -s "$directory/.lkjscript-edit.generation" && ! -e "$directory/.lkjscript-edit.pending" ]] || fail 'commit incomplete'
grep -Fq 'document.style + 1i64 }' "$directory/main.lkj" || fail 'commit did not publish the body'
for engine in native interpreter; do
    [[ $("$compiler" run main.lkj --source-dir "$directory" --no-library --no-store --engine "$engine") == 42 ]] \
        || fail "wrong $engine result"
done
printf 'ok require %s\n' "$name"
# Malformed requirements refuse before any candidate is checked.
while IFS='|' read -r code reason require; do
    printf '{"version":1,"operations":[{"op":"replace-body","id":"update","source":"{ document.style }"}],"require":%s}\n' \
        "$require" > "$fixture/$name.json"
    refuse "$code" "$reason"
done <<'REQUESTS'
E_EDIT_REQUEST|require names no predicate|[]
E_EDIT_REQUEST|require must be an array|{"id":"update","predicate":"same-signature"}
E_EDIT_REQUEST|unknown require predicate same-authority|[{"id":"update","predicate":"same-authority"}]
E_EDIT_REQUEST|unknown require field instance|[{"id":"update","predicate":"same-signature","instance":"ir:update"}]
E_EDIT_REQUEST|missing predicate|[{"id":"update"}]
E_EDIT_REQUEST|duplicate require predicate same-signature|[{"id":"update","predicate":"same-signature"},{"id":"value@update","predicate":"same-signature"}]
E_EDIT_IDENTITY|identity is absent from this source snapshot|[{"id":"missing","predicate":"same-signature"}]
REQUESTS

# 2. A new direct effect violates the effect summary; the signature is kept.
project direct library <<'SOURCE'
fn report(out: &mut Stdout, value: i64) -> i64 { value + 1i64 }
pub fn main(mut out: Stdout) -> i64 { report(&mut out, 41i64) }
SOURCE
request <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"report","source":"{ let _ = out.write(b\"changed\\n\"); value + 1i64 }"}],
 "require":[{"id":"report","predicate":"same-signature"},{"id":"report","predicate":"same-effect-summary"}]}
JSON
refuse E_EDIT_REQUIRE 'no source was published' \
    'require "value@report" predicate=same-signature verdict=PROVEN level=checked-ir instances=1' \
    'require "value@report" predicate=same-effect-summary verdict=VIOLATED level=checked-ir instance="ir:report" before="" after="stdout_write"'

# 3. An effect added to a destructor reaches the function that drops its owner.
project destructor library <<'SOURCE'
struct Guard { out: Stdout }
impl Drop for Guard {
    fn drop(&mut self) { let _ = &mut self.out; }
}
fn scope(out: Stdout) { let _guard = Guard { out }; }
pub fn main(out: Stdout) { scope(out); }
SOURCE
request <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"<Guard as Drop>::drop","source":"{ let _ = self.out.write(b\"cleanup\\n\"); }"}],
 "require":[{"id":"scope","predicate":"same-effect-summary"}]}
JSON
refuse E_EDIT_REQUIRE 'no source was published' \
    'require "value@scope" predicate=same-effect-summary verdict=VIOLATED level=checked-ir instance="ir:scope" before="" after="stdout_write"'

# 4. An effect added to a callee reaches its unedited caller.
callee_source() {
    cat <<'SOURCE'
fn leaf(out: &mut Stdout) -> i64 { 1i64 }
fn relay(out: &mut Stdout) -> i64 { leaf(out) + 1i64 }
pub fn main(mut out: Stdout) -> i64 { relay(&mut out) + 40i64 }
SOURCE
}
callee_request() {
    cat <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"leaf","source":"{ let _ = out.write(b\"leaf\\n\"); 1i64 }"}],
 "require":[{"id":"relay","predicate":"same-effect-summary"}]}
JSON
}
callee_row='require "value@relay" predicate=same-effect-summary verdict=VIOLATED level=checked-ir instance="ir:relay" before="" after="stdout_write"'
project callee library < <(callee_source)
callee_request | request
refuse E_EDIT_REQUIRE 'no source was published' "$callee_row"

# 5. A changed returned-view origin keeps the erased signature.
project return-origin plain <<'SOURCE'
fn choose<'a, 'b>(first: &'a i64, second: &'b i64) -> &'a i64 { first }
pub fn main() -> i64 { *choose(&42i64, &1i64) }
SOURCE
request <<'JSON'
{"version":1,"operations":[{"op":"replace-item","id":"choose","source":"fn choose<'a, 'b>(first: &'a i64, second: &'b i64) -> &'b i64 { second }"}],
 "require":[{"id":"choose","predicate":"same-signature"},{"id":"choose","predicate":"same-return-origins"}]}
JSON
refuse E_EDIT_REQUIRE 'no source was published' \
    'require "value@choose" predicate=same-signature verdict=PROVEN level=checked-ir instances=1' \
    'require "value@choose" predicate=same-return-origins verdict=VIOLATED level=checked-ir instance="ir:choose"'

# 6. A changed parameter type; origins over a changed signature are UNKNOWN.
project signature plain <<'SOURCE'
fn scale(value: i64) -> i64 { value * 2i64 }
pub fn main() -> i64 { scale(21i64) }
SOURCE
request <<'JSON'
{"version":1,"operations":[{"op":"replace-item","id":"scale","source":"fn scale(value: i32) -> i64 { (value as i64) * 2i64 }"},
  {"op":"replace-body","id":"main","source":"{ scale(21i32) }"}],
 "require":[{"id":"scale","predicate":"same-signature"},{"id":"scale","predicate":"same-return-origins"},
  {"id":"scale","predicate":"same-effect-summary"}]}
JSON
refuse E_EDIT_REQUIRE 'no source was published' \
    'require "value@scale" predicate=same-signature verdict=VIOLATED level=checked-ir instance="ir:scale"' \
    'require "value@scale" predicate=same-return-origins verdict=UNKNOWN level=checked-ir reason=signature-changed' \
    'require "value@scale" predicate=same-effect-summary verdict=PROVEN level=checked-ir instances=1'

# 7. A stale item digest from before an external edit refuses before checking.
project stale-item plain < <(document_source)
digest=$(sed -n 3,6p "$directory/main.lkj" | head -c -1 | sha256sum | cut -d' ' -f1)
sed -i 's/^    document.style$/    document.style + 0i64/' "$directory/main.lkj"
valid_request '' | request
refuse E_EDIT_STALE 'expected item digest differs'

# 8. A stale source snapshot from before an external edit refuses before checking.
project stale-snapshot plain < <(document_source)
digest=$(sed -n 3,6p "$directory/main.lkj" | head -c -1 | sha256sum | cut -d' ' -f1)
valid_request '' | request
edit "$compiler" --dry-run || fail 'snapshot capture was refused'
snapshot=$(sed -n 's/^transaction source-before=\([0-9a-f]*\) .*/\1/p' "$fixture/out")
valid_request "\"snapshot\":\"$snapshot\"," | request
edit "$compiler" --dry-run || fail 'fresh snapshot twin was refused'
printf '%s\n' '// external edit' >> "$directory/main.lkj"
refuse E_EDIT_STALE 'expected source snapshot differs'

# 9. An uninstantiated generic has no checked instance and is never PROVEN;
#    a changed instance set is incomplete coverage, also UNKNOWN.
project generic plain <<'SOURCE'
fn wrap<T>(value: T) -> T { value }
fn pass<T>(value: T) -> T { value }
pub fn main() -> i64 { pass(42i64) }
SOURCE
request <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"wrap","source":"{ pass(value) }"}],
 "require":[{"id":"wrap","predicate":"same-signature"},{"id":"pass","predicate":"same-signature"}]}
JSON
refuse E_EDIT_REQUIRE 'no source was published' \
    'require "value@wrap" predicate=same-signature verdict=UNKNOWN level=checked-ir reason=no-checked-instance' \
    'require "value@pass" predicate=same-signature verdict=PROVEN level=checked-ir instances=1'
name=generic-coverage
request <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"main","source":"{ if pass(true) { pass(42i64) } else { 0i64 } }"}],
 "require":[{"id":"pass","predicate":"same-signature"}]}
JSON
refuse E_EDIT_REQUIRE 'no source was published' \
    'require "value@pass" predicate=same-signature verdict=UNKNOWN level=checked-ir reason=instance-coverage-changed before=1 after=2'

# 10. Overlapping exclusive views are still refused by checking, before any verdict.
project overlap plain < <(document_source)
request <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"update","source":"{ let first = &mut document.style; let second = &mut document.style; *first = 1i64; *second + 41i64 }"}],
 "require":[{"id":"update","predicate":"same-signature"}]}
JSON
refuse E_EDIT_CHECK 'main.lkj:3:91: E_BORROW document.*.style conflicts with exclusive borrow held by first'

# 11-12. Two compilers each omit one fact: one reverse-call edge, one cleanup
# effect. Each turns a VIOLATED case into a false PROVEN, so this oracle fails.
mkdir "$fixture/mutant"
cp -a src library "$fixture/mutant/"
function_index() {
    awk -v name="\"$1\"" '/^fn / { if ($2 == name) { print ordinal + 0; found++ } ordinal++ }
        END { if (found != 1) exit 1 }' "$fixture/control.lkjir"
}
# mutant OLD NEW OUTPUT: build a compiler whose fact source differs in one line.
mutant() {
    awk -v old="$1" -v new="$2" '$0 == old { print new; count++; next } { print }
        END { if (count != 1) exit 1 }' src/semantic_facts.lkj > "$fixture/mutant/src/semantic_facts.lkj" \
        || fail 'mutation anchor is not unique'
    cmp -s src/semantic_facts.lkj "$fixture/mutant/src/semantic_facts.lkj" && fail 'mutation changed no byte'
    (cd "$fixture/mutant" && "$compiler" build src/main.lkj --library-dir library \
        --runtime-source library/runtime.lkj -o "$3" "${reuse[@]}") > "$fixture/out" 2> "$fixture/error" \
        || fail 'mutant compiler build failed'
}
# false_proof BINARY ROW: the mutant accepts the dry run with exactly this row.
false_proof() {
    state > "$fixture/before"
    edit "$1" --dry-run || fail 'mutant failed before its false proof'
    verdicts "$fixture/out" "$2" || fail 'mutant did not give the expected false proof'
    state > "$fixture/after"
    cmp -s "$fixture/before" "$fixture/after" || fail 'mutant dry run changed state'
}
project mutant-edge library < <(callee_source)
callee_request | request
"$compiler" ir main.lkj --source-dir "$directory" "${libraries[@]}" --no-store -o "$fixture/control.lkjir"
relay=$(function_index relay) || fail 'relay is not one checked function'
leaf=$(function_index leaf) || fail 'leaf is not one checked function'
mutant '        for (callee, _) in &relation.calls { parents[*callee].push(index); }' \
    "        for (callee, _) in &relation.calls { if index != ${relay}usize || *callee != ${leaf}usize { parents[*callee].push(index); } }" \
    "$fixture/edge-mutant"
false_proof "$fixture/edge-mutant" \
    'require "value@relay" predicate=same-effect-summary verdict=PROVEN level=checked-ir instances=1'
verdicts "$fixture/out" "$callee_row" && fail 'edge mutant passed the oracle'
refuse E_EDIT_REQUIRE 'no source was published' "$callee_row"

release_source() {
    cat <<'SOURCE'
fn release(directory: Directory) -> Option<Directory> { Some(directory) }
pub fn main(root: Directory) { let _kept = release(root); }
SOURCE
}
project mutant-cleanup library < <(release_source)
request <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"release","source":"{ None }"}],
 "require":[{"id":"release","predicate":"same-effect-summary"}]}
JSON
release_row='require "value@release" predicate=same-effect-summary verdict=VIOLATED level=checked-ir instance="ir:release" before="" after="directory_close"'
"$compiler" ir main.lkj --source-dir "$directory" "${libraries[@]}" --no-store -o "$fixture/control.lkjir"
release=$(function_index release) || fail 'release is not one checked function'
mutant '        for operation in &relation.operations { names.insert(operation.name.clone()); }' \
    "        for operation in &relation.operations { if index != ${release}usize || !operation.cleanup { names.insert(operation.name.clone()); } }" \
    "$fixture/cleanup-mutant"
false_proof "$fixture/cleanup-mutant" \
    'require "value@release" predicate=same-effect-summary verdict=PROVEN level=checked-ir instances=1'
verdicts "$fixture/out" "$release_row" && fail 'cleanup mutant passed the oracle'
refuse E_EDIT_REQUIRE 'no source was published' "$release_row"
printf 'ok require qualification cases=12 mutants-refused=2\n'
