#!/usr/bin/env bash
# Frozen report from the predecessor; no JavaScript host is needed here.
set -euo pipefail
export LC_ALL=C
cd "$(dirname "$0")/../.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler" ;; esac
export LKJSCRIPT_COMPILER="$compiler"
unset LKJSCRIPT_VERIFY_LEVENSHTEIN_BINARY
specimen="$PWD/examples/real_programs/levenshtein"
wrapper="$PWD/tools/real-programs/verify-levenshtein.sh"
work=$(mktemp -d "${TMPDIR:-/tmp}/lkjscript-verify-levenshtein-test.XXXXXX")
work=$(cd "$work" && pwd)
trap 'rm -rf -- "$work"' EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM

cat > "$work/golden.json" <<'JSON'
{
   "assertion_count": 26,
   "commit": "9c4730b1973d4e61e187f8fe6d5f299ad5c991fc",
   "exact_upstream_bytes": true,
   "files": {
      "license": {
         "bytes": 1099,
         "lines": 22,
         "sha256": "ca4662cb5d1b738fbe5350c0d5485ba11773b4b7208974082ae6e129a52d631d"
      },
      "src/lib.rs": {
         "bytes": 1476,
         "lines": 65,
         "sha256": "6f295e027783399df21c081465b2ebc2d3a499895635a839c1c00aae4455f267"
      },
      "tests/lib.rs": {
         "bytes": 1627,
         "lines": 60,
         "sha256": "88c8be4758d544673107a4b932e5c00cef6cd7972dc7f9293ccfaa4e5e672665"
      }
   },
   "inverse_range_replacements": 1,
   "kind": "levenshtein-upstream-provenance",
   "projection_sha256": "e118333ead31f962a23f2d6e1439b192bc007af3f5b58bffc400bff694d9ad0a",
   "restored_header_lines": 2,
   "schema": 1,
   "test_count": 8,
   "test_names": [
      "empty_left",
      "empty_right",
      "empty_both",
      "equal_long",
      "case_sensitive",
      "ordering",
      "should_work",
      "unicode"
   ]
}
JSON
check_report() {
    bash "$wrapper" "$@" > "$work/out" 2> "$work/err"
    cmp "$work/golden.json" "$work/out"
    test ! -s "$work/err"
}
# First prove the standalone path really builds, then run the same report
# with the cached executable and a deliberately unavailable compiler.
check_report "$specimen"
"$compiler" build tools/real-programs/verify-levenshtein.lkj --library-dir library --no-store \
    --grant arguments --grant source=env:LKJSCRIPT_VERIFY_LEVENSHTEIN_SOURCE \
    --grant output --grant errors --grant status -o "$work/verify"
export LKJSCRIPT_VERIFY_LEVENSHTEIN_BINARY="$work/verify"
export LKJSCRIPT_COMPILER="$work/absent-compiler"
check_report "$specimen"
ln -s "$specimen" "$work/root-link"
check_report "$work/root-link"
check_report "$work/absent/../root-link"
(cd "$specimen"; check_report "")

refusals=0
refuse() {
    local expected=$1 code=0
    shift
    # A FIFO must refuse promptly, not merely be terminated by the watchdog.
    timeout 5s bash "$wrapper" "$@" > "$work/out" 2> "$work/err" || code=$?
    if [[ $code != 255 || -s $work/out ]] || ! grep -Fq -- "$expected" "$work/err"; then
        printf 'wrong refusal (%s) for %s: %s\n' "$code" "$*" "$expected" >&2
        cat "$work/out" "$work/err" >&2
        exit 1
    fi
    refusals=$((refusals+1))
}
refuse 'usage:'
refuse 'usage:' --help
refuse 'usage:' "$specimen" "$specimen"
refuse 'cannot resolve specimen directory' "$work/missing"
mutant="$work/mutant"
reset() {
    rm -rf -- "$mutant"
    mkdir "$mutant"
    cp "$specimen/upstream.json" "$specimen/upstream.lkj" "$specimen/LICENSE" "$mutant/"
}
for name in upstream.json upstream.lkj LICENSE; do
    for kind in missing symlink directory fifo; do
        reset
        rm "$mutant/$name"
        case "$kind" in
            missing) ;;
            symlink) ln -s "$specimen/$name" "$mutant/$name" ;;
            directory) mkdir "$mutant/$name" ;;
            fifo) mkfifo "$mutant/$name" ;;
        esac
        refuse "$name: errno" "$mutant"
    done
done
reset
printf '\377' >> "$mutant/upstream.json"
refuse 'invalid UTF-8' "$mutant"
reset
printf '{' > "$mutant/upstream.json"
refuse 'JSON at byte' "$mutant"
reset
printf '[]\n' > "$mutant/upstream.json"
refuse 'manifest must be an object' "$mutant"
reset
sed -i 's/"schema": 1/"schema": 2/' "$mutant/upstream.json"
refuse 'unsupported manifest schema' "$mutant"
reset
sed -i 's/"schema": 1/"schema": "1.0"/' "$mutant/upstream.json"
refuse 'unsupported manifest schema' "$mutant"
reset
sed -i 's/"name": "levenshtein"/"name": "wrong"/' "$mutant/upstream.json"
refuse 'wrong package name' "$mutant"
reset
sed -i 's/"version": "1.0.5"/"version": "1.0.4"/' "$mutant/upstream.json"
refuse 'wrong package version' "$mutant"
reset
sed -i 's@wooorm/levenshtein-rs@wrong/source@' "$mutant/upstream.json"
refuse 'wrong source origin' "$mutant"
reset
sed -i 's/9c4730b1973d4e61e187f8fe6d5f299ad5c991fc/0/' "$mutant/upstream.json"
refuse 'wrong pinned commit' "$mutant"
reset
sed -i 's/"license": "MIT"/"license": "BSD"/' "$mutant/upstream.json"
refuse 'wrong license identity' "$mutant"
reset
sed -i 's/"files":/"missing_files":/' "$mutant/upstream.json"
refuse 'missing file hashes' "$mutant"
reset
sed -i 's/"adaptation":/"missing_adaptation":/' "$mutant/upstream.json"
refuse 'missing adaptation contract' "$mutant"
reset
sed -i 's/"range_expression": 1/"range_expression": 0/' "$mutant/upstream.json"
refuse 'range adaptation count must be one' "$mutant"
reset
sed -i 's/"integration_header_lines": 2/"integration_header_lines": 3/' "$mutant/upstream.json"
refuse 'integration header count must be two' "$mutant"
reset
sed -i 's/6f295e027783399df21c081465b2ebc2d3a499895635a839c1c00aae4455f267/BAD/' "$mutant/upstream.json"
refuse 'invalid hash for src/lib.rs' "$mutant"
reset
sed -i 's/6f295e027783399df21c081465b2ebc2d3a499895635a839c1c00aae4455f267/0f295e027783399df21c081465b2ebc2d3a499895635a839c1c00aae4455f267/' "$mutant/upstream.json"
refuse 'upstream bytes differ: src/lib.rs' "$mutant"
reset
printf 'tamper\n' >> "$mutant/LICENSE"
refuse 'upstream bytes differ: license' "$mutant"
reset
sed -i 's/"source_lines": 65/"source_lines": 64/' "$mutant/upstream.json"
refuse 'source line count differs' "$mutant"
reset
sed -i 's/"test_lines": 60/"test_lines": "60.000000000000004"/' "$mutant/upstream.json"
refuse 'test line count differs' "$mutant"
reset
sed -i 's/UPSTREAM INTEGRATION TESTS/LOST MARKER/' "$mutant/upstream.lkj"
refuse 'projection must contain exactly one integration boundary' "$mutant"
reset
printf '\n// UPSTREAM INTEGRATION TESTS: exact bodies, package header removed.\n' >> "$mutant/upstream.lkj"
refuse 'projection must contain exactly one integration boundary' "$mutant"
reset
sed -i 's/std::iter::range_from(1)/(1..)/' "$mutant/upstream.lkj"
refuse 'library must contain exactly one normalized range expression' "$mutant"
reset
sed -i '1i// std::iter::range_from(1)' "$mutant/upstream.lkj"
refuse 'library must contain exactly one normalized range expression' "$mutant"
reset
sed -i '1i// (1..)' "$mutant/upstream.lkj"
refuse 'library contains an unexpected original range expression' "$mutant"

# Rehash modified test sections so the inventory, not merely a stale digest,
# must refuse. The predecessor's invariant is >0 assertions; an extra
# assertion with a stale hash refuses, while a rehashed extra is admissible.
rehash_tests() {
    { printf 'extern crate levenshtein;\nuse levenshtein::levenshtein;\n'
      sed -n '/^\/\/ UPSTREAM INTEGRATION TESTS: exact bodies, package header removed\.$/,$ { /^\/\/ UPSTREAM INTEGRATION TESTS:/d; p; }' "$mutant/upstream.lkj"
    } > "$work/reconstructed-tests"
    local digest lines
    digest=$(sha256sum < "$work/reconstructed-tests")
    lines=$(wc -l < "$work/reconstructed-tests")
    sed -i -e "s/88c8be4758d544673107a4b932e5c00cef6cd7972dc7f9293ccfaa4e5e672665/${digest%% *}/" \
        -e "s/\"test_lines\": 60/\"test_lines\": $lines/" "$mutant/upstream.json"
}
reset
sed -i 's/"empty_left"/"not_a_test"/' "$mutant/upstream.json"
refuse 'ordered test names differ' "$mutant"
reset
sed -i 's/"empty_left"/false/' "$mutant/upstream.json"
refuse 'manifest must name eight tests' "$mutant"
reset
sed -i '0,/^#\[test\]$/s//\/\/ missing attribute/' "$mutant/upstream.lkj"
rehash_tests
refuse 'projection must contain eight tests' "$mutant"
reset
printf '\n#[test]\n' >> "$mutant/upstream.lkj"
rehash_tests
refuse 'projection must contain eight tests' "$mutant"
reset
sed -i 's/fn empty_right()/fn empty_left()/' "$mutant/upstream.lkj"
sed -i 's/"empty_right"/"empty_left"/' "$mutant/upstream.json"
rehash_tests
refuse 'duplicate test name' "$mutant"
reset
sed -i '/^[[:blank:]]*assert_eq!/d' "$mutant/upstream.lkj"
rehash_tests
refuse 'no upstream assertions' "$mutant"
reset
printf '    assert!(true);\n' >> "$mutant/upstream.lkj"
refuse 'upstream bytes differ: tests/lib.rs' "$mutant"

# Preserve the predecessor's scalar coercions, including IEEE midpoint ties.
reset
sed -i -e 's/"schema": 1/"schema": 1.0000000000000001/' \
    -e 's/"range_expression": 1/"range_expression": "1"/' \
    -e 's/"integration_header_lines": 2/"integration_header_lines": 2e0/' \
    -e 's/"source_lines": 65/"source_lines": " 0x41 "/' \
    -e 's/"test_lines": 60/"test_lines": "60.000000000000003"/' "$mutant/upstream.json"
check_report "$mutant"
reset
sed -i -e 's/"schema": 1/"schema": 0.999999999999999944488848768742172978818416595458984375/' \
    -e 's/"source_lines": 65/"source_lines": "+006.5e1"/' \
    -e 's/"test_lines": 60/"test_lines": "0b111100"/' "$mutant/upstream.json"
check_report "$mutant"
reset
sed -i 's/"schema": 1/"schema": 0.999999999999999944488848768742172978818416595458984374/' "$mutant/upstream.json"
refuse 'unsupported manifest schema' "$mutant"

# The JavaScript oracle decoded projection bytes as Latin-1, not UTF-8.
# After rehashing, CR anchors and cross-line NBSP whitespace admit exactly
# one additional assertion; the vertical-tab-indented invocation is ignored.
reset
printf '\r\tassert_ne!\240\n  (1, 2);\r\vassert!(false);\r// \377\n' >> "$mutant/upstream.lkj"
rehash_tests
bash "$wrapper" "$mutant" > "$work/out" 2> "$work/err"
test ! -s "$work/err"
printf '%s  %s\n' 8931f03da03d2d4d8e55a00a151aa278643a66942a4b8c6bb2da5e8c2f7250c4 "$work/out" | sha256sum --check --strict --quiet
grep -Fq '"assertion_count": 27,' "$work/out"

# Standard checked JSON rejects ambiguous decoded keys and invalid scalar
# escapes. These are fail-closed even when JS would have ignored the field.
reset
sed -i 's/"schema": 1/"schema": 1, "schema": 1/' "$mutant/upstream.json"
refuse 'duplicate object key' "$mutant"
reset
sed -i 's/"publication": ".*"/"publication": "\\ud800"/' "$mutant/upstream.json"
refuse 'surrogate' "$mutant"
reset
printf '\357\273\277' > "$work/bom"
cat "$mutant/upstream.json" >> "$work/bom"
cp "$work/bom" "$mutant/upstream.json"
refuse 'JSON at byte' "$mutant"
bad_path="$work/"$'invalid-\xff'
mkdir "$bad_path"
cp "$specimen/upstream.json" "$specimen/upstream.lkj" "$specimen/LICENSE" "$bad_path/"
refuse 'usage:' "$bad_path"
printf 'ok verify-levenshtein: cached and uncached golden, Latin-1 oracle, %s refusals\n' "$refusals"
