#!/usr/bin/env bash
# Partial source diagnostics never authorize a source program or erase its refusals.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ $# -le 3 ]] || { printf 'usage: %s [compiler [rustc [baseline-compiler]]]\n' "$0" >&2; exit 2; }
compiler=$(realpath "${1:-target/lkjscript}")
witness=${2:-}
baseline=${3:-}
if [[ -n $witness ]]; then witness=$(command -v "$witness"); fi
if [[ -n $baseline ]]; then baseline=$(realpath "$baseline"); fi
library=$PWD/library
mkdir -p target
work=$(mktemp -d "$PWD/target/partial-check.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)); then
        for name in error verifier-error rust-error; do
            if [[ -s $work/$name ]]; then tail -n 40 "$work/$name" >&2; fi
        done
    fi
    if [[ ${LKJSCRIPT_PARTIAL_KEEP:-0} == 1 ]]; then printf 'retained_fixture=%s\n' "$work" >&2
    else rm -rf -- "$work"; fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
fail() { printf 'partial-check: %s\n' "$*" >&2; exit 1; }
mkdir "$work/cases"
cp tests/fixtures/partial_check/*.lkj "$work/cases/"
verifier_compiler=${baseline:-$compiler}
"$verifier_compiler" build tests/fixtures/partial_check_verify.lkj --library-dir "$library" --no-store -o "$work/verify" > "$work/verifier-output" 2> "$work/verifier-error" || fail 'verifier build'
cases=(independent unsafe_binding local_import global_static mutable_static qualified_header global_macro glob_import trait_impl type_failure qualified_import normalized_self import_chain library_inputs grow_context grow_module receiver retained_impl qualified_chain scoped_coherence glob_anchor scalar_size macro_footprint)
sed '/^fn invalid</d; /^fn refusal()/d' "$work/cases/grow_context.lkj" > "$work/grow-valid.lkj"
cat >> "$work/grow-valid.lkj" <<'EOF'
impl Marker for i64 {}
fn probe() -> i64 {
    let counter = Counter;
    let (left, right) = counter.generic(21i64);
    let (x, y) = concrete();
    if left + right == 42 && x == 42 && y == 42 && library()
        && identity(42i64) == 42 && counter.measured() == 42 { expanded() } else { 0 }
}
EOF
mkdir "$work/library-mutant"
printf 'pub mod char;\n' > "$work/library-mutant/lib.lkj"
printf 'pub fn __lkjscript_is_ascii_digit(_value: char) -> bool { 1i64 }\n' > "$work/library-mutant/char.lkj"
sed 's/1i64 };/false };/' "$work/cases/grow_context.lkj" > "$work/macro-mutant.lkj"
cp "$work/cases/grow_context.lkj" "$work/method-mutant.lkj"
printf 'impl Counter { fn measured(&self) -> i64 { 0 } }\n' >> "$work/method-mutant.lkj"
sed '/^use crate::absent::Ghost;$/d; /^fn refusal()/d' "$work/cases/receiver.lkj" > "$work/receiver-valid.lkj"
for variant in duplicate blanket qualifier alias; do
    cp "$work/cases/receiver.lkj" "$work/receiver-$variant.lkj"
done
cat >> "$work/receiver-duplicate.lkj" <<'EOF'

impl HasBits<u16> for Ready {
    fn has_bits(&self, bits: u16) -> bool { bits != 0 }
}
EOF
cat >> "$work/receiver-blanket.lkj" <<'EOF'

impl<T> HasBits<u16> for T {
    fn has_bits(&self, bits: u16) -> bool { bits != 0 }
}
EOF
printf '\nimpl HasBits::Missing for Ready {}\n' >> "$work/receiver-qualifier.lkj"
cat >> "$work/receiver-alias.lkj" <<'EOF'

use crate::absent::Ghost as Receiver;
impl HasBits<u16> for Receiver {
    fn has_bits(&self, bits: u16) -> bool { bits != 0 }
}
EOF
sed 's/self.end = self.capacity;/println!("unsupported destructor");/' \
    "$work/cases/receiver.lkj" > "$work/receiver-destructor.lkj"
sed '/fn denied(/d; /fn unresolved(/d' "$work/cases/retained_impl.lkj" > "$work/retained-impl-valid.lkj"
sed 's/fn unresolved(\&self)/fn reading(\&self)/' "$work/cases/retained_impl.lkj" > "$work/retained-impl-duplicate.lkj"
sed 's/fn unresolved(\&self)/fn unresolved(\&self, _value: dyn Marker)/' \
    "$work/cases/retained_impl.lkj" > "$work/retained-impl-header.lkj"
sed 's/    fn denied(.*//' "$work/cases/retained_impl.lkj" > "$work/retained-impl-semantic.lkj"
sed -e 's/, absent::Missing//' -e '/^mod unrelated {$/,/^}$/d' \
    -e '/^fn poisoned()/d; /^fn malformed()/d; /^fn refusal()/d' \
    "$work/cases/qualified_chain.lkj" > "$work/qualified-chain-valid.lkj"
sed '5,7d' "$work/cases/scoped_coherence.lkj" > "$work/scoped-coherence-valid.lkj"
sed -n '2,3p' "$work/cases/scalar_size.lkj" > "$work/scalar-size-valid.lkj"
sed 's/impl Bits<u16> for Ready/impl Bits<u8> for Ready/' \
    "$work/cases/scoped_coherence.lkj" > "$work/scoped-coherence-same.lkj"
sed 's/dyn Missing) -> bool { false }/dyn Missing) -> bool { opaque!() }/' \
    "$work/cases/scoped_coherence.lkj" > "$work/scoped-coherence-opaque.lkj"
sed 's/impl Bits<u16> for Ready/impl Drop for Ready/; s/fn test(\&self, _value: dyn Missing) -> bool { false }/fn drop(\&mut self, _value: dyn Missing) {}/' \
    "$work/cases/scoped_coherence.lkj" > "$work/scoped-coherence-drop.lkj"
footprint="$work/cases/macro_footprint.lkj"
sed "s/ where T: for<'a> Compare<\&'a \[u8\]>//; s/{ 42 }/{ receiver::Ready(42u8).exact() as i64 }/" \
    "$footprint" > "$work/macro_footprint-valid.lkj"
sed 's/functions first, second/functions probe, second/' "$footprint" > "$work/macro_footprint_collision.lkj"
sed 's/> {})+/> { hidden!(); })+/' "$footprint" > "$work/macro_footprint_nested.lkj"
sed 's/> {})+/> { impl receiver::Ready { fn exact(\&self) -> u8 { 0 } } })+/' \
    "$footprint" > "$work/macro_footprint_implementation.lkj"
sed '14s/}/} missing!();/' "$footprint" > "$work/macro_footprint_intervening.lkj"
sed 's/pub fn \$name/pub(in ::receiver) fn $name/' "$footprint" > "$work/macro_footprint_malformed.lkj"
mkdir "$work/receiver-edit"
cp "$work/cases/receiver.lkj" "$work/receiver-edit/main.lkj"
head -c 32 /dev/urandom > "$work/receiver-edit-key"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"probe","source":"{ 0i64 }"}]}' \
    > "$work/receiver-edit-request"
checks=0
refusals=0
for mode in on off; do
    expected=42
    if [[ $mode == off ]]; then expected=17; fi
    "$compiler" check "$work/cases/valid.lkj" --library-dir "$library" --debug-assertions "$mode" --no-store > "$work/output" 2> "$work/error"
    [[ ! -s $work/output && ! -s $work/error ]] || fail "valid $mode check changed its silent contract"
    "$compiler" ir "$work/cases/valid.lkj" --library-dir "$library" --debug-assertions "$mode" --no-store -o "$work/valid-$mode.lkjir" > "$work/output" 2> "$work/error"
    for engine in native interpreter; do
        actual=$("$compiler" run "$work/cases/valid.lkj" --library-dir "$library" --debug-assertions "$mode" --engine "$engine" --no-store 2> "$work/error")
        [[ $actual == "$expected" ]] || fail "valid $mode/$engine value"
        actual=$("$compiler" run "$work/valid-$mode.lkjir" --entry probe --engine "$engine" --no-store 2> "$work/error")
        [[ $actual == "$expected" ]] || fail "valid serialized $mode/$engine value"
        checks=$((checks + 2))
        actual=$("$compiler" run "$work/grow-valid.lkj" --library-dir "$library" --debug-assertions "$mode" --engine "$engine" --no-store 2> "$work/error")
        [[ $actual == 42 ]] || fail "grown closure $mode/$engine value"
        actual=$("$compiler" run "$work/receiver-valid.lkj" --library-dir "$library" --debug-assertions "$mode" --engine "$engine" --no-store 2> "$work/error")
        [[ $actual == 42 ]] || fail "receiver closure $mode/$engine value"
        actual=$("$compiler" run "$work/retained-impl-valid.lkj" --library-dir "$library" --debug-assertions "$mode" --engine "$engine" --no-store 2> "$work/error")
        [[ $actual == 42 ]] || fail "retained implementation $mode/$engine value"
        checks=$((checks + 3))
        for valid in qualified-chain scoped-coherence scalar-size macro_footprint; do
            actual=$("$compiler" run "$work/$valid-valid.lkj" --library-dir "$library" --debug-assertions "$mode" --engine "$engine" --no-store 2> "$work/error")
            [[ $actual == 42 ]] || fail "$valid $mode/$engine value"
            checks=$((checks + 1))
        done
    done
    for variant in library macro method; do
        source="$work/cases/grow_context.lkj"
        selected_library=$library
        if [[ $variant == library ]]; then selected_library="$work/library-mutant"
        else source="$work/$variant-mutant.lkj"; fi
        status=0
        "$compiler" check "$source" --library-dir "$selected_library" --debug-assertions "$mode" --no-store > "$work/output" 2> "$work/error" || status=$?
        [[ $status == 1 && ! -s $work/output ]] || fail "$variant mutant $mode kept no source refusal"
        "$work/verify" "$source" "grow_${variant}_mutant" < "$work/error" || fail "$variant mutant $mode item check"
    done
    for variant in duplicate blanket qualifier alias destructor; do
        source="$work/receiver-$variant.lkj"
        status=0
        "$compiler" check "$source" --library-dir "$library" --debug-assertions "$mode" --no-store > "$work/output" 2> "$work/error" || status=$?
        [[ $status == 1 && ! -s $work/output ]] || fail "receiver $variant $mode kept no source refusal"
        "$work/verify" "$source" "receiver_$variant" < "$work/error" 2> "$work/verifier-error" ||
            fail "receiver $variant $mode item check"
    done
    for variant in duplicate header semantic; do
        source="$work/retained-impl-$variant.lkj"
        status=0
        "$compiler" check "$source" --library-dir "$library" --debug-assertions "$mode" --no-store > "$work/output" 2> "$work/error" || status=$?
        [[ $status == 1 && ! -s $work/output ]] || fail "retained implementation $variant $mode kept no source refusal"
        "$work/verify" "$source" "retained_impl_$variant" < "$work/error" 2> "$work/verifier-error" ||
            fail "retained implementation $variant $mode item check"
    done
    for variant in same opaque drop; do
        source="$work/scoped-coherence-$variant.lkj"
        status=0
        "$compiler" check "$source" --library-dir "$library" --debug-assertions "$mode" --no-store > "$work/output" 2> "$work/error" || status=$?
        [[ $status == 1 && ! -s $work/output ]] || fail "scoped coherence $variant $mode kept no source refusal"
        "$work/verify" "$source" "scoped_coherence_$variant" < "$work/error" 2> "$work/verifier-error" ||
            fail "scoped coherence $variant $mode item check"
    done
    for variant in collision nested implementation intervening malformed; do
        name="macro_footprint_$variant"
        source="$work/$name.lkj"
        status=0
        "$compiler" check "$source" --library-dir "$library" --debug-assertions "$mode" --no-store > "$work/output" 2> "$work/error" || status=$?
        [[ $status == 1 && ! -s $work/output ]] || fail "$name $mode kept no source refusal"
        "$work/verify" "$source" "$name" < "$work/error" 2> "$work/verifier-error" ||
            fail "$name $mode item check"
    done
    if [[ -n $baseline ]]; then
        "$baseline" ir "$work/cases/valid.lkj" --library-dir "$library" --debug-assertions "$mode" --no-store -o "$work/baseline-$mode.lkjir" > "$work/output" 2> "$work/error"
        cmp "$work/baseline-$mode.lkjir" "$work/valid-$mode.lkjir" || fail "valid $mode IR changed"
        for engine in native interpreter; do
            actual=$("$baseline" run "$work/cases/valid.lkj" --library-dir "$library" --debug-assertions "$mode" --engine "$engine" --no-store 2> "$work/error")
            [[ $actual == "$expected" ]] || fail "baseline valid $mode/$engine value"
        done
    fi
    for name in "${cases[@]}"; do
        source="$work/cases/$name.lkj"
        status=0
        "$compiler" check "$source" --library-dir "$library" --debug-assertions "$mode" --no-store --stats > "$work/output" 2> "$work/error" || status=$?
        [[ $status == 1 && ! -s $work/output ]] || fail "$name/$mode must keep the source refusal"
        "$work/verify" "$source" "$name" < "$work/error" > "$work/verified" 2> "$work/verifier-error" || fail "$name/$mode item inventory or upstream-cause mismatch"
        cat "$work/verified"
        if [[ -n $baseline ]]; then
            status=0
            baseline_command=check
            baseline_args=()
            if [[ $name == grow_module ]]; then
                # Compare loader errors before the old partial catalog reports
                # a duplicate from its different cfg recovery boundary.
                baseline_command=build
                baseline_args=(-o "$work/baseline-refused")
            fi
            "$baseline" "$baseline_command" "$source" --library-dir "$library" --debug-assertions "$mode" --no-store "${baseline_args[@]}" > "$work/baseline-output" 2> "$work/baseline-error" || status=$?
            [[ $status == 1 && ! -s $work/baseline-output ]] || fail "baseline $name/$mode refusal"
            while IFS= read -r line; do
                [[ $line == *': E_'* && $line != *': E_BLOCKED '* ]] || continue
                grep -Fxq -- "$line" "$work/error" || fail "$name/$mode lost an original diagnostic"
            done < "$work/baseline-error"
        fi
        for engine in native interpreter; do
            status=0
            "$compiler" run "$source" --entry probe --library-dir "$library" --debug-assertions "$mode" --engine "$engine" --no-store > "$work/output" 2> "$work/refused-error" || status=$?
            [[ $status == 1 && ! -s $work/output ]] || fail "$name/$mode/$engine executed an invalid root"
            if grep -q '^LKJSCRIPT_ITEMS ' "$work/refused-error"; then fail 'item reporting leaked into execution admission'; fi
            refusals=$((refusals + 1))
        done
    done
    # Even when probe itself checks independently, the invalid source cannot publish an image or IR.
    for name in independent receiver qualified_chain scalar_size macro_footprint; do
        for command in build ir; do
            printf 'unreplaced destination\n' > "$work/destination"
            cp "$work/destination" "$work/destination-before"
            status=0
            "$compiler" "$command" "$work/cases/$name.lkj" --entry probe --library-dir "$library" --debug-assertions "$mode" --no-store -o "$work/destination" > "$work/output" 2> "$work/refused-error" || status=$?
            [[ $status == 1 && ! -s $work/output ]] || fail "$command published an invalid source root"
            cmp "$work/destination-before" "$work/destination" || fail "$command changed the destination after refusal"
            if grep -q '^LKJSCRIPT_ITEMS ' "$work/refused-error"; then fail 'item reporting leaked into artifact admission'; fi
            refusals=$((refusals + 1))
        done
    done
    for edit_mode in dry-run apply; do
        edit_flags=()
        if [[ $edit_mode == dry-run ]]; then edit_flags=(--dry-run); fi
        status=0
        "$compiler" edit main.lkj --source-dir "$work/receiver-edit" --library-dir "$library" \
            --debug-assertions "$mode" --no-store --store-key "$work/receiver-edit-key" \
            --request "$work/receiver-edit-request" "${edit_flags[@]}" \
            > "$work/output" 2> "$work/refused-error" || status=$?
        [[ $status == 1 && ! -s $work/output ]] || fail "receiver edit $edit_mode $mode admitted a refused root"
        grep -q 'E_CAPABILITY' "$work/refused-error" || fail 'receiver edit lost the original refusal'
        cmp "$work/cases/receiver.lkj" "$work/receiver-edit/main.lkj"
        [[ ! -e $work/receiver-edit/.lkjscript-edit.pending ]] || fail 'receiver edit left a pending transaction'
        refusals=$((refusals + 1))
    done
done
printf 'ok partial-check: both configurations, authored item IDs/counts/dependencies/causes, %s source/IR value routes and %s retained execution, publication and checked-edit refusals\n' "$checks" "$refusals"
if [[ -n $baseline ]]; then
    printf 'ok partial-check baseline: valid IR byte identity, both-engine values, and every original primary refusal retained\n'
fi
if [[ -n $witness ]]; then
    rust_values=0
    rust_verdicts=0
    for mode in on off; do
        expected=42
        if [[ $mode == off ]]; then expected=17; fi
        for level in 0 3; do
            cp "$work/cases/valid.lkj" "$work/rust.lkj"
            printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/rust.lkj"
            "$witness" --edition=2021 --crate-name partial_valid -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error" || fail "Rust valid $mode/$level"
            [[ $("$work/rust") == "$expected" ]] || fail "Rust valid $mode/$level value"
            rust_values=$((rust_values + 1))
            for name in "${cases[@]}"; do
                cp "$work/cases/$name.lkj" "$work/rust.lkj"
                printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/rust.lkj"
                status=0
                "$witness" --edition=2021 --crate-name partial_case -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error" || status=$?
                case "$name" in
                    independent|unsafe_binding|library_inputs|macro_footprint)
                        [[ $status == 0 && $("$work/rust") == 42 ]] || fail "Rust capability-boundary twin $name/$mode/$level"
                        ;;
                    local_import|global_static|mutable_static|qualified_header|global_macro|glob_import|trait_impl|type_failure|qualified_import|normalized_self|import_chain|grow_context|grow_module|receiver|retained_impl|qualified_chain|scoped_coherence|glob_anchor|scalar_size)
                        [[ $status != 0 ]] || fail "Rust accepted malformed context $name/$mode/$level"
                        ;;
                    *) fail "missing Rust oracle for $name" ;;
                esac
                rust_verdicts=$((rust_verdicts + 1))
            done
            sed "s/std::char::__lkjscript_is_ascii_digit('9')/'9'.is_ascii_digit()/" "$work/grow-valid.lkj" > "$work/rust.lkj"
            printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/rust.lkj"
            "$witness" --edition=2021 --crate-name partial_grow -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error" || fail "Rust grown closure $mode/$level"
            [[ $("$work/rust") == 42 ]] || fail "Rust grown closure $mode/$level value"
            rust_values=$((rust_values + 1))
            cp "$work/receiver-valid.lkj" "$work/rust.lkj"
            printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/rust.lkj"
            "$witness" --edition=2021 --crate-name partial_receiver -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error" || fail "Rust receiver closure $mode/$level"
            [[ $("$work/rust") == 42 ]] || fail "Rust receiver closure $mode/$level value"
            rust_values=$((rust_values + 1))
            cp "$work/retained-impl-valid.lkj" "$work/rust.lkj"
            printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/rust.lkj"
            "$witness" --edition=2021 --crate-name partial_retained_impl -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error" || fail "Rust retained implementation $mode/$level"
            [[ $("$work/rust") == 42 ]] || fail "Rust retained implementation $mode/$level value"
            rust_values=$((rust_values + 1))
            for valid in qualified-chain scoped-coherence scalar-size macro_footprint; do
                cp "$work/$valid-valid.lkj" "$work/rust.lkj"
                printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/rust.lkj"
                "$witness" --edition=2021 --crate-name partial_lookup -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error" || fail "Rust $valid $mode/$level"
                [[ $("$work/rust") == 42 ]] || fail "Rust $valid $mode/$level value"
                rust_values=$((rust_values + 1))
            done
            for variant in collision nested implementation intervening malformed; do
                cp "$work/macro_footprint_$variant.lkj" "$work/rust.lkj"
                printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/rust.lkj"
                if "$witness" --edition=2021 --crate-name partial_footprint -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error"; then
                    fail "Rust accepted macro footprint $variant/$mode/$level"
                fi
                rust_verdicts=$((rust_verdicts + 1))
            done
            for variant in duplicate blanket qualifier; do
                sed '/^use crate::absent::Ghost;$/d; /^fn refusal()/d' "$work/receiver-$variant.lkj" > "$work/rust.lkj"
                printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/rust.lkj"
                if "$witness" --edition=2021 --crate-name partial_receiver -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error"; then
                    fail "Rust accepted receiver $variant $mode/$level"
                fi
                rust_verdicts=$((rust_verdicts + 1))
            done
            cp "$work/cases/unsafe_binding.lkj" "$work/rust.lkj"
            printf '\nfn main() { println!("{}", unsafe_local()); }\n' >> "$work/rust.lkj"
            "$witness" --edition=2021 --crate-name partial_shadow -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error"
            [[ $("$work/rust") == 9 ]] || fail "Rust unsafe binding lost shadowing in $mode/$level"
            rust_values=$((rust_values + 1))
        done
    done
    printf 'ok partial-check Rust: %s values and %s context/boundary verdicts; opt0/opt3 and both configurations\n' "$rust_values" "$rust_verdicts"
fi
