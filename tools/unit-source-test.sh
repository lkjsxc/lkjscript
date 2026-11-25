#!/usr/bin/env bash
# Source-unit cache/link integration, kept small; scale and agent trials are separate.
set -euo pipefail
cd "$(dirname "$0")/.."
source tools/fixture-bytes.sh
unit_compiler=$(realpath "${LKJSCRIPT_BINARY:-target/lkjscript}")
unit_fixture=$(mktemp -d target/unit-source.XXXXXX)
# Keep the key outside the source grant but on its filesystem for the hard-link attack.
unit_key_root=$(mktemp -d "$(pwd -P)/target/unit-key.XXXXXX")
unit_cleanup() {
    local result=$?
    if (( result != 0 )); then
        for diagnostic in "$unit_fixture"/*.err; do
            [[ -f $diagnostic ]] || continue
            printf '%s\n' "$diagnostic" >&2
            cat "$diagnostic" >&2
        done
    fi
    rm -rf "$unit_fixture" "$unit_key_root"
}
trap unit_cleanup EXIT
trap 'exit 130' HUP INT TERM
mkdir "$unit_fixture/store"
unit_key=$(XDG_STATE_HOME="$unit_key_root" sh tools/store-key.sh)
unit_root=$(realpath "$unit_fixture")
# Source-hash reuse must bind fresh loaded bytes and reject A -> B -> A races.
for unit_engine in native interpreter; do
    "$unit_compiler" test src/driver/units/inputs_test.lkj --library-dir library \
        --engine "$unit_engine" --jobs 1 --no-store \
        > "$unit_root/inputs-$unit_engine.out" 2> "$unit_root/inputs-$unit_engine.err"
    grep -Fxq -- 'ok 8 tests' "$unit_root/inputs-$unit_engine.out"
done
# Compare disk-retained selected inputs with the independent snapshot oracle.
"$unit_compiler" build src/unit_input_records_cases.lkj --library-dir library \
    --entry main --no-store -o "$unit_root/input-records" \
    > "$unit_root/input-records-build.out" 2> "$unit_root/input-records-build.err"
"$unit_root/input-records" > "$unit_root/input-records-native.out" 2> "$unit_root/input-records-native.err"
grep -Fxq -- 'ok unit input records 10 contract groups' "$unit_root/input-records-native.out"
"$unit_compiler" run src/unit_input_records_cases.lkj --library-dir library \
    --entry main --engine interpreter --no-store \
    > "$unit_root/input-records-interpreter.out" 2> "$unit_root/input-records-interpreter.err"
grep -Fxq -- 'ok unit input records 10 contract groups' "$unit_root/input-records-interpreter.out"
cat > "$unit_root/units.lkj" <<'SOURCE'
const UNITS: &[&str] = &["base=base.lkj", "geometry=geometry.lkj", "app=app.lkj"];
const DEPENDS: &[&str] = &["geometry=base", "app=geometry"];
const ENTRY: &str = "app::main";
SOURCE
cat > "$unit_root/base.lkj" <<'SOURCE'
#[derive(Clone, Copy)]
pub struct Pair { pub left: i64, pub right: i64 }
fn adjustment() -> i64 { 0i64 }
impl Pair { pub fn total(&self) -> i64 { self.left + self.right + adjustment() } }
pub fn sum(value: Pair) -> i64 {
    use crate::base::Pair as LocalPair;
    let value: LocalPair = value;
    value.total()
}
pub fn first<'a>(value: &'a Pair) -> &'a i64 { &value.left }
pub fn identity<T: Copy>(value: T) -> T { value }
pub fn included<T: Copy>(_: T) -> &'static str { include_str!("message.txt") }
SOURCE
printf 'unit include' > "$unit_root/message.txt"
cat > "$unit_root/geometry.lkj" <<'SOURCE'
use crate::base::Pair;
pub fn answer() -> i64 {
    let value = Pair { left: 17i64, right: 25i64 };
    let first = crate::base::first(&value);
    assert_eq!(*first, 17i64);
    let original = crate::base::included(0usize);
    assert_eq!(original, "unit include");
    crate::base::identity(crate::base::sum(value))
}
SOURCE
cat > "$unit_root/app.lkj" <<'SOURCE'
pub fn main(mut out: Stdout) {
    let answer = crate::geometry::answer();
    out.write(format!("{answer}\n").as_bytes());
}
SOURCE
unit_no_scratch() {
    if compgen -G "$unit_root/store/.lkjscript-unit-*.scratch" > /dev/null; then
        printf '%s unit build left a named scratch spool\n' "$1" >&2
        exit 1
    fi
}
unit_options=(units.lkj --source-dir "$unit_root" --store "$unit_root/store" --store-key "$unit_key" --stats)
"$unit_compiler" check-units "${unit_options[@]}" > "$unit_root/check.out" 2> "$unit_root/check.err"
grep -Fq -- 'checked=3 reused=0' "$unit_root/check.out"
"$unit_compiler" build-units "${unit_options[@]}" --grant out -o "$unit_root/program" > "$unit_root/build.out" 2> "$unit_root/build.err"
[[ $("$unit_root/program") == 42 ]]
unit_no_scratch successful
"$unit_compiler" build-units "${unit_options[@]}" --grant out -o "$unit_root/warm" > "$unit_root/warm.out" 2> "$unit_root/warm.err"
grep -Fq -- 'checked=0 reused=3 emitted=0' "$unit_root/warm.out"
cmp "$unit_root/program" "$unit_root/warm"
# The local use in sum belongs to the erased body; it creates no second range.
# Private helpers and nongeneric methods cross the same authenticated boundary.
sed -i 's/adjustment() -> i64 { 0i64 }/adjustment() -> i64 { 1i64 }/' "$unit_root/base.lkj"
"$unit_compiler" build-units "${unit_options[@]}" --grant out -o "$unit_root/helper" > "$unit_root/helper.out" 2> "$unit_root/helper.err"
grep -Fq -- 'checked=1 reused=2 emitted=1' "$unit_root/helper.out"
[[ $("$unit_root/helper") == 43 ]]
sed -i 's/self\.left + self\.right + adjustment()/self.left + self.right + adjustment() + 1i64/' "$unit_root/base.lkj"
"$unit_compiler" build-units "${unit_options[@]}" --grant out -o "$unit_root/method" > "$unit_root/method.out" 2> "$unit_root/method.err"
grep -Fq -- 'checked=1 reused=2 emitted=1' "$unit_root/method.out"
[[ $("$unit_root/method") == 44 ]]
sed -i 's/value\.total()/value.total() + 1i64/' "$unit_root/base.lkj"
"$unit_compiler" build-units "${unit_options[@]}" --grant out -o "$unit_root/edited" > "$unit_root/edit.out" 2> "$unit_root/edit.err"
grep -Fq -- 'checked=1 reused=2 emitted=1' "$unit_root/edit.out"
[[ $("$unit_root/edited") == 45 ]]
# Generic templates are producer inputs, not proofs of future instantiations.
sed -i 's/identity<T: Copy>(value: T) -> T { value }/identity<T: Copy>(value: T) -> T { let result = value; result }/' "$unit_root/base.lkj"
"$unit_compiler" build-units "${unit_options[@]}" --grant out -o "$unit_root/template" > "$unit_root/template.out" 2> "$unit_root/template.err"
grep -Fq -- 'checked=2 reused=1 emitted=2' "$unit_root/template.out"
[[ $("$unit_root/template") == 45 ]]
# Including a source that is also projected must preserve its original bytes.
sed -i 's/include_str!("message\.txt")/include_str!("base.lkj")/' "$unit_root/base.lkj"
sed -i 's/assert_eq!(original, "unit include");/assert!(original.contains("value.total() + 1i64"));/' "$unit_root/geometry.lkj"
"$unit_compiler" build-units "${unit_options[@]}" --grant out -o "$unit_root/include" > "$unit_root/include.out" 2> "$unit_root/include.err"
[[ $("$unit_root/include") == 45 ]]
# Corrupt every retained interface, including the currently selected versions.
unit_interfaces=("$unit_root/store"/lkjscript-unit-interface-1-*)
[[ -f ${unit_interfaces[0]} ]] || { echo 'no retained interfaces to corrupt' >&2; exit 1; }
for unit_interface in "${unit_interfaces[@]}"; do
    fixture_flip_last_byte "$unit_interface"
done
"$unit_compiler" check-units "${unit_options[@]}" > "$unit_root/corrupt.out" 2> "$unit_root/corrupt.err"
grep -Fq -- 'checked=3 reused=0' "$unit_root/corrupt.out"
# Rejection never overwrites the already published executable.
cp "$unit_root/base.lkj" "$unit_root/base.saved"
printf '\npub fn invalid() -> bool { 42i64 }\n' >> "$unit_root/base.lkj"
if "$unit_compiler" build-units "${unit_options[@]}" --grant out -o "$unit_root/edited" > "$unit_root/reject.out" 2> "$unit_root/reject.err"; then
    echo 'ill-typed unit was accepted' >&2; exit 1
fi
[[ $("$unit_root/edited") == 45 ]]
unit_no_scratch refused
if compgen -G "$unit_root/.lkjscript-unit-*.tmp" > /dev/null; then
    echo 'refused unit build left an unpublished image' >&2; exit 1
fi
mv "$unit_root/base.saved" "$unit_root/base.lkj"
# Even a hard link cannot turn the private store key into source/include data.
ln "$unit_key" "$unit_root/private.key"
cp "$unit_root/base.lkj" "$unit_root/base.saved"
printf '\npub fn leaked() -> &'\''static [u8] { include_bytes!("private.key") }\n' >> "$unit_root/base.lkj"
if "$unit_compiler" check-units "${unit_options[@]}" > "$unit_root/key.out" 2> "$unit_root/key.err"; then
    echo 'private authentication key was readable as source' >&2; exit 1
fi
grep -Fq -- 'private unit authentication key' "$unit_root/key.err"
mv "$unit_root/base.saved" "$unit_root/base.lkj"
rm "$unit_root/private.key"
cp "$unit_root/units.lkj" "$unit_root/units.saved"
sed -i 's/"geometry=base"/"base=app", "geometry=base"/' "$unit_root/units.lkj"
if "$unit_compiler" check-units "${unit_options[@]}" > "$unit_root/cycle.out" 2> "$unit_root/cycle.err"; then
    echo 'cyclic units were accepted' >&2; exit 1
fi
grep -Fq -- 'unit dependency cycle' "$unit_root/cycle.err"
mv "$unit_root/units.saved" "$unit_root/units.lkj"
# A retained generic template needs a declaration behind its direct dependency.
# Warm source-frontier reuse must come from each selected projection's claim.
mkdir "$unit_root/ancestor" "$unit_root/ancestor/store"
cat > "$unit_root/ancestor/units.lkj" <<'SOURCE'
const UNITS: &[&str] = &["base=base.lkj", "middle=middle.lkj", "app=app.lkj"];
const DEPENDS: &[&str] = &["middle=base", "app=middle"];
const ENTRY: &str = "app::main";
SOURCE
cat > "$unit_root/ancestor/base.lkj" <<'SOURCE'
pub fn anchor() -> i64 { 0i64 }
pub fn identity<T: Copy>(value: T) -> T { value }
SOURCE
cat > "$unit_root/ancestor/middle.lkj" <<'SOURCE'
pub fn anchor() -> i64 { 0i64 }
pub fn forward<T: Copy>(value: T) -> T { crate::base::identity(value) }
SOURCE
cat > "$unit_root/ancestor/app.lkj" <<'SOURCE'
pub fn main(mut out: Stdout) {
    let value = crate::middle::forward(42i64);
    out.write(format!("{value}\n").as_bytes());
}
SOURCE
ancestor_options=(units.lkj --source-dir "$unit_root/ancestor" --store "$unit_root/ancestor/store" --store-key "$unit_key" --stats)
for ancestor_phase in cold warm; do
    "$unit_compiler" build-units "${ancestor_options[@]}" --grant out -o "$unit_root/ancestor/$ancestor_phase" > "$unit_root/ancestor-$ancestor_phase.out" 2> "$unit_root/ancestor-$ancestor_phase.err"
    [[ $("$unit_root/ancestor/$ancestor_phase") == 42 ]]
done
grep -Fq -- 'checked=3 reused=0' "$unit_root/ancestor-cold.out"
grep -Fq -- '"units_max_import_interfaces":2' "$unit_root/ancestor-cold.err"
grep -Fq -- 'checked=0 reused=3' "$unit_root/ancestor-warm.out"
sed -i 's/-> T { value }/-> T { let output = value; output }/' "$unit_root/ancestor/base.lkj"
"$unit_compiler" build-units "${ancestor_options[@]}" --grant out -o "$unit_root/ancestor/edited" > "$unit_root/ancestor-edited.out" 2> "$unit_root/ancestor-edited.err"
grep -Fq -- 'checked=3 reused=0' "$unit_root/ancestor-edited.out"
[[ $("$unit_root/ancestor/edited") == 42 ]]
# The compact in-memory claim must not replace later artifact authentication.
ancestor_projections=("$unit_root/ancestor/store"/unit-source-*)
[[ -f ${ancestor_projections[0]} ]] || { echo 'no retained source projections to corrupt' >&2; exit 1; }
for ancestor_projection in "${ancestor_projections[@]}"; do
    fixture_flip_last_byte "$ancestor_projection"
done
"$unit_compiler" build-units "${ancestor_options[@]}" --grant out -o "$unit_root/ancestor/recovered" > "$unit_root/ancestor-recovered.out" 2> "$unit_root/ancestor-recovered.err"
grep -Fq -- 'checked=3 reused=0' "$unit_root/ancestor-recovered.out"
[[ $("$unit_root/ancestor/recovered") == 42 ]]
printf 'ok source units: independent check, streamed link, unchanged/body/template reuse, authentication and atomic refusal\n'
