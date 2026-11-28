#!/usr/bin/env bash
# Optional independent witness; no Rust dependency enters the required gate.
set -euo pipefail
cd "$(dirname "$0")/.."
(($# <= 1)) || { printf 'usage: %s [RUSTC]\n' "$0" >&2; exit 2; }
rustc_bin=${1:-rustc}
command -v "$rustc_bin" >/dev/null || { printf 'Rust witness compiler unavailable: %s\n' "$rustc_bin" >&2; exit 1; }
source=examples/real_programs/levenshtein
golden=tests/fixtures/real_programs_levenshtein.output.gz
[[ -f "$golden" ]] || { printf 'missing checked corpus: %s\n' "$golden" >&2; exit 1; }
lkj_compiler=${LKJSCRIPT_COMPILER:-"$PWD/target/lkjscript"}
case "$lkj_compiler" in /*) ;; *) lkj_compiler="$PWD/$lkj_compiler" ;; esac
if [[ ! -x "$lkj_compiler" && -z ${LKJSCRIPT_COMPILER:-} ]]; then sh tools/bootstrap.sh; fi
[[ -x "$lkj_compiler" ]] || { printf 'checked compiler unavailable: %s\n' "$lkj_compiler" >&2; exit 1; }
export LKJSCRIPT_COMPILER="$lkj_compiler"
mkdir -p target
work=$(mktemp -d "$PWD/target/real-programs-rust.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT
    if ((status)); then
        for log in "$work"/*.err "$work"/*-tests.out; do
            [[ ! -f "$log" ]] || { printf '%s\n' "$log" >&2; cat "$log" >&2; }
        done
    fi
    rm -rf -- "$work"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
# These handwritten form kernels remain valid Rust and add no upstream sites.
# The accepted macro kernel retains its original body at its promoted location.
for foreign_case in macro_rules impl_trait qualified_type qualified_call nested_type associated_binding const_generic maybe_sized unsafe_block doctest_cfg dyn_trait associated_const at_pattern async_block doc_include; do
    foreign_source="tests/diagnostics/programs_foreign_$foreign_case.lkj"
    if [[ $foreign_case == macro_rules ]]; then
        foreign_source=tests/conformance/pass/programs_foreign_macro_rules.lkj
    fi
    "$rustc_bin" --edition=2021 --crate-name "foreign_$foreign_case" --crate-type lib --emit=metadata -Awarnings \
        "$foreign_source" -o "$work/foreign_$foreign_case.rmeta" \
        > "$work/foreign_$foreign_case.out" 2> "$work/foreign_$foreign_case.err"
done
env -u LKJSCRIPT_VERIFY_LEVENSHTEIN_BINARY bash tools/real-programs/verify-levenshtein.sh "$source" > "$work/provenance.json"
# Check the files rustc will read against the checked provenance report.
"$lkj_compiler" build tools/real-programs/levenshtein-checksums.lkj --library-dir library --no-store \
    --grant report --grant output --grant errors --grant status -o "$work/checksums" \
    > "$work/checksums-build.out" 2> "$work/checksums-build.err"
"$work/checksums" < "$work/provenance.json" > "$work/original.sha256"
sed -e 's@  src/lib\.rs$@  upstream.rs@' -e 's@  tests/lib\.rs$@  upstream_tests.rs@' \
    "$work/original.sha256" > "$work/restored.sha256"
restored="$work/restored"
mkdir "$restored"
"$lkj_compiler" build tools/real-programs/restore-levenshtein.lkj --library-dir library --no-store \
    --grant arguments --grant source="$PWD/$source" --grant output="$restored" \
    --grant errors --grant status -o "$work/restore" > "$work/restore-build.out" 2> "$work/restore-build.err"
"$work/restore" --rust
(cd "$restored" && sha256sum --check --strict --quiet "$work/restored.sha256")
cp "$source/LICENSE" "$work/LICENSE"
cp "$source/corpus.lkj" "$work/corpus.rs"
cat > "$work/main.rs" <<'RUST'
extern crate levenshtein;
mod upstream {
    pub use ::levenshtein::levenshtein;
}
#[path = "corpus.rs"]
mod corpus;
fn main() {
    print!("{}", corpus::output());
}
RUST
gzip -dc "$golden" > "$work/expected.out"
awk -F: 'NF != 2 || $1 != NR-1 || $2 !~ /^[0-9]+$/ { exit 1 } END { if (NR != 7239) exit 1 }' "$work/expected.out"
for optimization in 0 3; do
    mkdir "$work/opt-$optimization"
    library="$work/opt-$optimization/liblevenshtein.rlib"
    "$rustc_bin" --edition=2015 --crate-name levenshtein --crate-type rlib -C "opt-level=$optimization" "$restored/upstream.rs" -o "$library" > "$work/opt-$optimization-library.out" 2> "$work/opt-$optimization-library.err"
    "$rustc_bin" --edition=2015 --crate-name upstream_levenshtein_tests --test -C "opt-level=$optimization" --extern "levenshtein=$library" "$restored/upstream_tests.rs" -o "$work/opt-$optimization/tests" > "$work/opt-$optimization-test-build.out" 2> "$work/opt-$optimization-test-build.err"
    "$work/opt-$optimization/tests" --test-threads=1 --color never > "$work/opt-$optimization-tests.out" 2> "$work/opt-$optimization-tests.err"
    grep -Fq 'test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;' "$work/opt-$optimization-tests.out"
    test ! -s "$work/opt-$optimization-tests.err"
    "$rustc_bin" --edition=2021 --crate-name levenshtein_corpus -C "opt-level=$optimization" --extern "levenshtein=$library" "$work/main.rs" -o "$work/opt-$optimization/corpus" > "$work/opt-$optimization-corpus-build.out" 2> "$work/opt-$optimization-corpus-build.err"
    "$work/opt-$optimization/corpus" > "$work/opt-$optimization-corpus.out" 2> "$work/opt-$optimization-corpus.err"
    cmp "$work/expected.out" "$work/opt-$optimization-corpus.out"
    test ! -s "$work/opt-$optimization-corpus.err"
done
printf 'ok optional real-program Rust witness: exact original source/tests, 8 tests and 7,239 corpus rows at opt-level 0 and 3; 15 Rust-valid reduced forms (14 refusal kernels and the promoted macro kernel)\n'
