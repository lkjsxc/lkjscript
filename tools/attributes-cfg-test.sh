#!/usr/bin/env bash
# Focused attribute/cfg contracts, direct Rust oracles, and authenticated doc inputs.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ $# -le 3 ]] || { printf 'usage: %s [compiler [rustc [--rust-only]]]\n' "$0" >&2; exit 2; }
compiler=$(realpath "${1:-target/lkjscript}")
witness=${2:-}
if [[ -n $witness ]]; then witness=$(command -v "$witness"); fi
mode=${3:-all}
[[ $mode == all || $mode == --rust-only ]] || { printf 'unknown mode: %s\n' "$mode" >&2; exit 2; }
[[ $mode != --rust-only || -n $witness ]] || { printf "%s\n" "--rust-only requires a Rust executable argument" >&2; exit 2; }
mkdir -p target
work=$(mktemp -d "$PWD/target/attributes-cfg.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)); then
        for log in "$work/error" "$work/rust-error" "$work/test-error"; do
            [[ ! -s $log ]] || tail -n 45 "$log" >&2
        done
    fi
    if [[ ${LKJSCRIPT_ATTRIBUTES_KEEP:-0} == 1 ]]; then
        printf 'retained_fixture=%s\n' "$work" >&2
    else rm -rf -- "$work"; fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
fail() { printf 'attributes-cfg: %s\n' "$*" >&2; exit 1; }
refuse() {
    local code=$1 status=0
    shift
    "$@" > "$work/output" 2> "$work/error" || status=$?
    [[ $status == 1 ]] || fail "expected refusal $code, status=$status: $*"
    grep -Eq "$code" "$work/error" || fail "missing diagnostic $code: $*"
}
field() {
    sed -n "s/.*\"$2\":\([0-9][0-9]*\).*/\1/p" "$1" |
        awk '{ count += $1 } END { print count + 0 }'
}
library=$PWD/library
cases=$PWD/tests/conformance/attributes_cfg
mkdir "$work/rust"
cp "$cases/guide.txt" "$work/rust/"
rust_values=0
rust_shared=0
rust_boundaries=0
if [[ -n $witness ]]; then
for source in "$cases"/*.lkj; do
    name=$(basename "$source" .lkj)
    expected=$(sed -n 's|^// expect: ||p' "$source")
    oracle=$(sed -n 's|^// attr-rust: ||p' "$source")
    [[ $oracle == accept || $oracle == reject* ]] || fail "missing Rust oracle: $name"
    rust_expected=${oracle%% *}
    rust_code=
    [[ $oracle != *' '* ]] || rust_code=${oracle#* }
    { cat "$source"; printf '\nfn main() { println!("{}", probe()); }\n'; } > "$work/rust/$name.rs"
    for setting in on off; do
        for optimization in 0 3; do
            status=0
            "$witness" --edition=2021 -A warnings -C "opt-level=$optimization" \
                -C "debug-assertions=$setting" "$work/rust/$name.rs" \
                -o "$work/rust/image" > "$work/rust-output" 2> "$work/rust-error" || status=$?
            if [[ $rust_expected == accept ]]; then
                [[ $status == 0 ]] || fail "Rust unexpectedly refused $name/$setting/$optimization"
                if [[ $expected == 'value '* ]]; then
                    actual=$("$work/rust/image")
                    [[ $actual == "${expected#value }" ]] || fail "Rust value $name/$setting/$optimization"
                    rust_values=$((rust_values + 1))
                fi
            else
                [[ $status == 1 ]] || fail "Rust did not produce an ordinary refusal for $name/$setting/$optimization"
                [[ -z $rust_code ]] || grep -Fq "$rust_code" "$work/rust-error" ||
                    fail "Rust diagnostic $name/$setting/$optimization: $rust_code"
            fi
            if [[ $expected == 'value '* && $rust_expected == accept ||
                  $expected == 'reject '* && $rust_expected == reject ]]; then
                rust_shared=$((rust_shared + 1))
            else rust_boundaries=$((rust_boundaries + 1)); fi
        done
    done
done
printf 'attributes-cfg Rust: values=%s matching-verdicts=%s explicit-boundaries=%s\n' \
    "$rust_values" "$rust_shared" "$rust_boundaries"
fi
[[ $mode != --rust-only ]] || exit 0

# The compiler's case runner checks native, interpreter and serialized-IR routes.
for setting in on off; do
    for limit in 0 64; do
        if ! "$compiler" test "$cases" --library-dir "$library" --debug-assertions "$setting" \
            --inline-limit "$limit" --jobs 1 --no-store > "$work/test-output" 2> "$work/test-error"; then
            cat "$work/test-output" >&2
            fail "conformance $setting/inline$limit"
        fi
        printf 'attributes-cfg %s/inline%s: ' "$setting" "$limit"
        cat "$work/test-output"
    done
done

mkdir "$work/source" "$work/store" "$work/units" "$work/unit-store"
head -c 32 /dev/urandom > "$work/key"
cp "$cases/guide.txt" "$work/source/guide.txt"
"$compiler" build macro_doc.lkj --source-dir "$cases" --library-dir "$library" --entry probe \
    --index --store "$work/store" --store-key "$work/key" -o "$work/macro-doc-image" \
    > "$work/macro-doc-index" 2> "$work/error"
grep -q '^LKJSCRIPT_SNAPSHOT ' "$work/macro-doc-index" || fail 'macro documentation source replay/index'
cat > "$work/source/main.lkj" <<'LKJ'
#![doc = include_str!("guide.txt")]
#[doc = include_str!("guide.txt")]
fn probe() -> i64 { 42i64 }
LKJ
cat > "$work/source/nested.lkj" <<'LKJ'
mod nested { pub mod child; }
fn probe() -> i64 { nested::child::answer() }
LKJ
mkdir "$work/source/nested"
cat > "$work/source/nested/child.lkj" <<'LKJ'
#![doc = include_str!("../guide.txt")]
#[doc = include_str!("../guide.txt")]
pub fn answer() -> i64 { 42i64 }
LKJ
for setting in on off; do
    for engine in native interpreter; do
        for source in main.lkj nested.lkj; do
            [[ $("$compiler" run "$source" --source-dir "$work/source" --library-dir "$library" \
                --debug-assertions "$setting" --engine "$engine" --no-store 2> "$work/error") == 42 ]] ||
                fail "UTF-8 or source-relative doc $source/$setting/$engine"
        done
    done
done

# A doc-only equal-length byte change must invalidate input identity and image reuse.
cache_args=(main.lkj --source-dir "$work/source" --library-dir "$library" --entry probe
    --store "$work/store" --store-key "$work/key")
for setting in on off; do
    printf 'Alpha\n' > "$work/source/guide.txt"
    "$compiler" build "${cache_args[@]}" --debug-assertions "$setting" --index --stats \
        -o "$work/image" > "$work/cold.index" 2> "$work/error"
    source_before=$(sed -n 's/.* source=\([^ ]*\).*/\1/p' "$work/cold.index")
    snapshot_before=$(awk '$1=="LKJSCRIPT_SNAPSHOT" {print $2}' "$work/cold.index")
    [[ ${#source_before} == 64 && ${#snapshot_before} == 64 ]] || fail 'missing initial identities'
    "$compiler" build "${cache_args[@]}" --debug-assertions "$setting" --stats \
        -o "$work/image" > "$work/output" 2> "$work/warm.log"
    (( $(field "$work/warm.log" build_reused) > 0 )) || fail "$setting unchanged doc image not reused"
    printf 'Bravo\n' > "$work/source/guide.txt"
    "$compiler" build "${cache_args[@]}" --debug-assertions "$setting" --index --stats \
        -o "$work/image" > "$work/changed.index" 2> "$work/changed.log"
    (( $(field "$work/changed.log" build_reused) == 0 )) || fail "$setting doc change reused an old image"
    source_after=$(sed -n 's/.* source=\([^ ]*\).*/\1/p' "$work/changed.index")
    snapshot_after=$(awk '$1=="LKJSCRIPT_SNAPSHOT" {print $2}' "$work/changed.index")
    [[ ${#source_after} == 64 && $source_before != "$source_after" &&
       ${#snapshot_after} == 64 && $snapshot_before != "$snapshot_after" ]] ||
        fail "$setting included doc bytes absent from source/snapshot identity"
    # Historical query snapshots remain authentic history; edits require current inputs.
    "$compiler" query "${cache_args[@]}" --debug-assertions "$setting" --snapshot "$snapshot_before" \
        --kind outline --limit 20 --bytes 16384 > "$work/historical-query" 2> "$work/error"
    grep -Fq probe "$work/historical-query" || fail 'historical snapshot lost its declaration'
    cp "$work/source/main.lkj" "$work/main-before"
    printf '{"version":1,"snapshot":"%s","operations":[{"op":"replace-body","id":"probe","source":"{ 43i64 }"}]}\n' \
        "$source_before" > "$work/request"
    refuse E_EDIT_STALE "$compiler" edit "${cache_args[@]}" --debug-assertions "$setting" --request "$work/request"
    cmp "$work/main-before" "$work/source/main.lkj" || fail 'stale doc edit changed source'
    printf '{"version":1,"snapshot":"%s","operations":[{"op":"replace-body","id":"probe","source":"{ 43i64 }"}]}\n' \
        "$source_after" > "$work/request"
    "$compiler" edit "${cache_args[@]}" --debug-assertions "$setting" --request "$work/request" \
        > "$work/edit-output" 2> "$work/error"
    grep -Fq '#[doc = include_str!("guide.txt")]' "$work/source/main.lkj" || fail 'edit erased doc input'
    for engine in native interpreter; do
        [[ $("$compiler" run "${cache_args[@]}" --debug-assertions "$setting" --engine "$engine" \
            2> "$work/error") == 43 ]] || fail "$setting/$engine checked doc edit value"
    done
    cp "$work/main-before" "$work/source/main.lkj"
    "$compiler" build "${cache_args[@]}" --debug-assertions "$setting" -o "$work/image" \
        > "$work/output" 2> "$work/error"
    cp "$work/image" "$work/image-before"
    rm "$work/source/guide.txt"
    refuse E_INCLUDE_MISSING "$compiler" build "${cache_args[@]}" --debug-assertions "$setting" -o "$work/image"
    cmp "$work/image-before" "$work/image" || fail 'missing doc replaced the prior image'
    refuse E_INCLUDE_MISSING "$compiler" edit "${cache_args[@]}" --debug-assertions "$setting" --request "$work/request"
    cmp "$work/main-before" "$work/source/main.lkj" || fail 'missing doc edit changed source'
    printf '\377invalid\n' > "$work/source/guide.txt"
    refuse E_INCLUDE_UTF8 "$compiler" build "${cache_args[@]}" --debug-assertions "$setting" -o "$work/image"
    cmp "$work/image-before" "$work/image" || fail 'invalid UTF-8 doc replaced the prior image'
done
cp "$cases/guide.txt" "$work/source/guide.txt"

# Use the same source Directory and include loader for doc and expression forms.
printf 'outside source grant\n' > "$work/outside.txt"
ln -s "$work/outside.txt" "$work/source/escape-link.txt"
for spelling in ../outside.txt escape-link.txt; do
    printf '#[doc = include_str!("%s")]\nfn probe() -> i64 { 42i64 }\n' "$spelling" > "$work/source/escape.lkj"
    printf 'fn probe() -> usize { include_str!("%s").len() }\n' "$spelling" > "$work/source/expression.lkj"
    for setting in on off; do
        refuse 'E_INCLUDE_PATH|E_IO' "$compiler" check escape.lkj --source-dir "$work/source" \
            --debug-assertions "$setting" --no-store
        doc_code=$(sed -n 's/.*: \(E_[A-Z_]*\) .*/\1/p' "$work/error" | head -1)
        refuse "$doc_code" "$compiler" check expression.lkj --source-dir "$work/source" \
            --debug-assertions "$setting" --no-store
    done
done
if [[ -n $witness ]]; then
printf '\377invalid\n' > "$work/rust/invalid.txt"
printf '#[doc = include_str!("invalid.txt")]\nfn main() {}\n' > "$work/rust/invalid.rs"
for setting in on off; do
    for optimization in 0 3; do
        status=0
        "$witness" --edition=2021 -C "opt-level=$optimization" -C "debug-assertions=$setting" \
            "$work/rust/invalid.rs" -o "$work/rust/image" > "$work/rust-output" 2> "$work/rust-error" || status=$?
        [[ $status == 1 ]] || fail "Rust did not reject invalid UTF-8 normally $setting/$optimization"
        grep -Fq 'utf-8' "$work/rust-error" || fail 'Rust invalid UTF-8 cause'
    done
done

fi

# Unit interfaces retain included documentation as authenticated source input.
cat > "$work/units/units.lkj" <<'LKJ'
const UNITS: &[&str] = &["base=base.lkj", "app=app.lkj"];
const DEPENDS: &[&str] = &["app=base"];
const ENTRY: &str = "app::main";
LKJ
cat > "$work/units/base.lkj" <<'LKJ'
#![doc = include_str!("guide.txt")]
#[cfg(doctest)] extern crate doc_comment;
#[doc = include_str!("guide.txt")]
pub fn number() -> i64 { 42i64 }
LKJ
cat > "$work/units/app.lkj" <<'LKJ'
pub fn main(mut out: Stdout) { out.write(format!("{}\n", crate::base::number()).as_bytes()); }
LKJ
unit_args=(units.lkj --source-dir "$work/units" --library-dir "$library"
    --store "$work/unit-store" --store-key "$work/key" --grant out)
for setting in on off; do
    printf 'Alpha\n' > "$work/units/guide.txt"
    "$compiler" build-units "${unit_args[@]}" --debug-assertions "$setting" -o "$work/unit-image" \
        > "$work/output" 2> "$work/error"
    [[ $("$work/unit-image") == 42 ]] || fail "$setting unit value"
    "$compiler" build-units "${unit_args[@]}" --debug-assertions "$setting" -o "$work/unit-image" \
        > "$work/unit-warm" 2> "$work/error"
    grep -Fq 'checked=0 reused=2' "$work/unit-warm" || fail "$setting unchanged doc units not reused"
    printf 'Bravo\n' > "$work/units/guide.txt"
    "$compiler" build-units "${unit_args[@]}" --debug-assertions "$setting" -o "$work/unit-image" \
        > "$work/unit-changed" 2> "$work/error"
    checked=$(sed -n 's/.* checked=\([0-9][0-9]*\).*/\1/p' "$work/unit-changed")
    [[ -n $checked && $checked -gt 0 ]] || fail "$setting changed doc reused every unit"
    [[ $("$work/unit-image") == 42 ]] || fail "$setting changed doc unit value"
    cp "$work/unit-image" "$work/unit-before"
    rm "$work/units/guide.txt"
    refuse E_INCLUDE_MISSING "$compiler" build-units "${unit_args[@]}" --debug-assertions "$setting" -o "$work/unit-image"
    cmp "$work/unit-before" "$work/unit-image" || fail 'missing unit doc replaced the linked image'
done

# Read real item diagnostics, preserving checked siblings and the upstream cause.
cat > "$work/verify.lkj" <<'LKJ'
use std::json::{self, Value};
fn main(mut world: World) {
    let args = world.args();
    let mode = String::from_utf8(args[0usize].clone()).unwrap();
    let expected = String::from_utf8(args[1usize].clone()).unwrap();
    let text = String::from_utf8(world.stdin().read_to_end().unwrap()).unwrap();
    let mut reports = 0usize;
    let mut checked = 0usize;
    let mut probe = false;
    let mut dependent = false;
    for line in text.as_str().lines() {
        if !line.starts_with("LKJSCRIPT_ITEMS ") { continue; }
        reports += 1usize;
        let report = json::parse(line[16usize..].as_bytes()).unwrap();
        for item in report.get("items").unwrap().as_array().unwrap() {
            let name = item.get("name").unwrap().as_str().unwrap();
            let status = item.get("status").unwrap().as_str().unwrap();
            if status == "checked" { checked += 1usize; }
            if name == "probe" {
                if mode == "global" { assert_ne!(status, "checked"); }
                else { assert_eq!(status, "checked"); }
                probe = true;
            }
            if name == "dependent" && mode != "global" {
                assert_eq!(status, "blocked");
                let cause = item.get("cause").unwrap();
                assert_eq!(cause.get("code").unwrap().as_str().unwrap(), expected.as_str());
                dependent = true;
            }
        }
    }
    assert_eq!(reports, 1usize);
    let count = if mode == "owned" { 1usize } else if mode == "replay" { 2usize } else { 0usize };
    assert_eq!(checked, count);
    if mode != "global" { assert!(probe); assert!(dependent); }
}
LKJ
"$compiler" build "$work/verify.lkj" --library-dir "$library" --no-store -o "$work/verify" \
    > "$work/output" 2> "$work/error"
partial_check() {
    local kind=$1 code=$2
    refuse "$code" "$compiler" check partial.lkj --source-dir "$work/source" \
        --library-dir "$library" --debug-assertions "$setting" --no-store
    cp "$work/error" "$work/partial-error"
    if grep -q ': E_PARSE ' "$work/partial-error"; then fail "$specimen became a parser refusal"; fi
    "$work/verify" "$kind" "$code" < "$work/partial-error" > "$work/output" 2> "$work/error" ||
        fail "$specimen/$setting partial ownership ($kind)"
}
for setting in on off; do
    # An unsupported qualified attribute may be an arbitrary item producer.
    for specimen in unknown_function unknown_inner \
        leading_tool qualified_doc qualified_cfg qualified_tool_extra; do
        cp "$cases/$specimen.lkj" "$work/source/partial.lkj"
        partial_check global E_UNSUPPORTED
    done
    for specimen in unknown_field unknown_variant; do
        cp "$cases/$specimen.lkj" "$work/source/partial.lkj"
        partial_check owned E_UNSUPPORTED
    done
    specimen=doc_missing
    cp "$cases/$specimen.lkj" "$work/source/partial.lkj"
    printf 'fn dependent() -> i64 { refused() }\n' >> "$work/source/partial.lkj"
    cp "$work/source/partial.lkj" "$work/partial-before"
    partial_check owned E_INCLUDE_MISSING
    for engine in native interpreter; do
        refuse E_INCLUDE_MISSING "$compiler" run partial.lkj --source-dir "$work/source" \
            --library-dir "$library" --debug-assertions "$setting" --engine "$engine" --entry probe --no-store
    done
    for command in build ir; do
        printf 'unreplaced destination\n' > "$work/destination"
        cp "$work/destination" "$work/destination-before"
        refuse E_INCLUDE_MISSING "$compiler" "$command" partial.lkj --source-dir "$work/source" \
            --library-dir "$library" --debug-assertions "$setting" --entry probe --no-store -o "$work/destination"
        cmp "$work/destination-before" "$work/destination" || fail "$command published a refused doc root"
    done
    printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"probe","source":"{ 43i64 }"}]}' > "$work/request"
    refuse E_INCLUDE_MISSING "$compiler" edit partial.lkj --source-dir "$work/source" \
        --library-dir "$library" --debug-assertions "$setting" --store "$work/store" --store-key "$work/key" --request "$work/request"
    cmp "$work/partial-before" "$work/source/partial.lkj" || fail 'edited a refused doc root'
    for specimen in doc_missing_inner doc_missing_module; do
        cp "$cases/$specimen.lkj" "$work/source/partial.lkj"
        partial_check global E_INCLUDE_MISSING
    done
    # Recovery reparses a documented closure using its real included input.
    specimen=doc_partial_replay
    cp "$cases/$specimen.lkj" "$work/source/partial.lkj"
    for contents in Alpha Bravo; do
        printf '%s\n' "$contents" > "$work/source/guide.txt"
        partial_check replay E_CAPABILITY
    done
    rm "$work/source/guide.txt"
    partial_check global E_INCLUDE_MISSING
    cp "$cases/guide.txt" "$work/source/guide.txt"
done
printf 'ok attributes-cfg: native/interpreter, doc confinement, replay, reuse, units and checked edits\n'
