#!/usr/bin/env bash
# Independent expectations for the reviewed Levenshtein 1.0.5 gate specimen.
set -euo pipefail
export LC_ALL=C

specimen=$1
report=$2
checksums=$3
oracle=$4

commit=9c4730b1973d4e61e187f8fe6d5f299ad5c991fc
declare -Ar pinned=(
    [upstream.json]=3a1b46ae88949473a2f10ac38a565ce6048ade9198275f2e6e8ffc7030285a5e
    [upstream.lkj]=e118333ead31f962a23f2d6e1439b192bc007af3f5b58bffc400bff694d9ad0a
    [LICENSE]=ca4662cb5d1b738fbe5350c0d5485ba11773b4b7208974082ae6e129a52d631d
    [src/lib.rs]=6f295e027783399df21c081465b2ebc2d3a499895635a839c1c00aae4455f267
    [tests/lib.rs]=88c8be4758d544673107a4b932e5c00cef6cd7972dc7f9293ccfaa4e5e672665
)

refuse() {
    printf '%s\n' "$1" >&2
    exit 255
}

regular() { [[ -f $1 && ! -L $1 ]]; }

require_pin() {
    local file=$1 name=$2 actual
    regular "$file" || refuse "provenance refusal: not a regular file: $name"
    actual=$(sha256sum < "$file") || exit 1
    [[ ${actual%% *} == "${pinned[$name]}" ]] || refuse "provenance refusal: pinned bytes differ: $name"
}

compare() {
    regular "$1" && cmp -s -- "$1" "$2" || refuse "$3 bytes differ"
}

mkdir -p "$oracle/inputs" "$oracle/controls" "$oracle/src" "$oracle/tests"
for name in upstream.json upstream.lkj LICENSE; do
    regular "$specimen/$name" || refuse "provenance refusal: not a regular file: $name"
    cp -P -- "$specimen/$name" "$oracle/inputs/$name"
    require_pin "$oracle/inputs/$name" "$name"
done
projection="$oracle/inputs/upstream.lkj"
library="$oracle/src/lib.rs"
tests="$oracle/tests/lib.rs"
# The reviewed manifest describes a 65-line library with one range adaptation,
# followed by a blank line, the integration boundary and the test bodies.
sed -n '1,65{s/std::iter::range_from(1)/(1..)/;p;}' "$projection" > "$library"
{
    printf 'extern crate levenshtein;\nuse levenshtein::levenshtein;\n'
    tail -n +68 "$projection"
} > "$tests"
require_pin "$library" src/lib.rs
require_pin "$tests" tests/lib.rs

file_facts() {
    local path=$1 file=$2 counts lines bytes
    counts=$(wc -lc < "$file")
    read -r lines bytes <<< "$counts"
    printf '      "%s": {\n         "bytes": %s,\n         "lines": %s,\n         "sha256": "%s"\n      }' \
        "$path" "$bytes" "$lines" "${pinned[$3]}"
}

read -r test_count assertion_count < <(awk -v names="$oracle/test-names" '
    /^#\[test\]$/ { test = 1; next }
    test && /^fn [a-z_]+\(\) \{$/ { test_name[++count] = $2; sub(/\(\)$/, "", test_name[count]) }
    { test = 0 }
    /^[ \t]*assert(_eq|_ne)?![ \t]*\(/ { assertions++ }
    END {
        if (count != 8 || assertions != 26) exit 1
        for (i = 1; i <= count; i++)
            printf "      \"%s\"%s\n", test_name[i], (i < count ? "," : "") > names
        print count, assertions
    }
' "$tests")

license_facts=$(file_facts license "$oracle/inputs/LICENSE" LICENSE)
library_facts=$(file_facts src/lib.rs "$library" src/lib.rs)
test_facts=$(file_facts tests/lib.rs "$tests" tests/lib.rs)
test_names=$(< "$oracle/test-names")
cat > "$oracle/provenance.json" <<REPORT
{
   "assertion_count": $assertion_count,
   "commit": "$commit",
   "exact_upstream_bytes": true,
   "files": {
$license_facts,
$library_facts,
$test_facts
   },
   "inverse_range_replacements": 1,
   "kind": "levenshtein-upstream-provenance",
   "projection_sha256": "${pinned[upstream.lkj]}",
   "restored_header_lines": 2,
   "schema": 1,
   "test_count": $test_count,
   "test_names": [
$test_names
   ]
}
REPORT
printf '%s  %s\n' "${pinned[src/lib.rs]}" src/lib.rs "${pinned[tests/lib.rs]}" tests/lib.rs > "$oracle/restored.sha256"
compare "$report" "$oracle/provenance.json" 'provenance report'
compare "$checksums" "$oracle/restored.sha256" 'checksum list'

# Exercise the same input check against each historical provenance and manifest
# mutation. A failed command and a refusal have different exit statuses.
while IFS='|' read -r name label edit; do
    changed="$oracle/controls/$name-$label"
    sed -e "$edit" "$oracle/inputs/$name" > "$changed"
    status=0
    reason=$(require_pin "$changed" "$name" 2>&1) || status=$?
    if [[ $status != 255 || $reason != "provenance refusal: pinned bytes differ: $name" ]]; then
        refuse "expected provenance refusal for $label, got $status"
    fi
done <<'MUTATIONS'
upstream.lkj|missing-range-normalization|s/std::iter::range_from(1)/(1..)/
upstream.lkj|extra-normalized-expression|1i\// std::iter::range_from(1)
upstream.lkj|algorithm-change|s/cache\[index_a\] = result;/cache[index_a] = result + 1;/
upstream.lkj|deleted-assertion|/assert_eq!(levenshtein("", "a"), 1);/d
upstream.lkj|changed-expected-value|s/assert_eq!(levenshtein("", "a"), 1);/assert_eq!(levenshtein("", "a"), 0);/
upstream.lkj|renamed-test|s/fn empty_left()/fn replaced_left()/
upstream.lkj|excluded-test|:a;N;$!ba;s/\n#\[test\]\nfn empty_left()/\n#[cfg(any())]\n#[test]\nfn empty_left()/
upstream.lkj|injected-test|$a\\n#[test]\nfn injected() {}
upstream.lkj|changed-test-literal|s/"DwAyNE"/"DWAYNE"/
upstream.lkj|changed-boundary|s|^// UPSTREAM INTEGRATION TESTS:.*$|// changed boundary|
upstream.lkj|duplicate-boundary|$a\\n// UPSTREAM INTEGRATION TESTS: exact bodies, package header removed.
upstream.lkj|unexpected-integration-header|/UPSTREAM INTEGRATION TESTS/a\extern crate levenshtein;\nuse levenshtein::levenshtein;
upstream.lkj|trailing-source|$a\// unrecorded source
LICENSE|changed-license|$G
upstream.json|reordered-test-names|:a;N;$!ba;s/"should_work",\n    "unicode"/"unicode",\n    "should_work"/
upstream.json|false-source-count|s/"source_lines": 65/"source_lines": 66/
upstream.json|wrong-pinned-commit|s/"commit": "[a-f0-9]*"/"commit": "0000000000000000000000000000000000000000"/
upstream.json|false-adaptation-count|s/"range_expression": 1/"range_expression": 0/
upstream.json|array-schema|s/"schema": 1/"schema": [1]/
upstream.json|array-commit|s/"commit": \("[a-f0-9]*"\)/"commit": [\1]/
upstream.json|array-source-count|s/"source_lines": 65/"source_lines": [65]/
upstream.json|nested-test-names|s/^    \("[a-z_]*"\)\(,\?\)$/    [\1]\2/
MUTATIONS

# Author the canonical request envelope, then JSON-quote the authenticated
# UTF-8 projection byte by byte. Its only control bytes are line feeds.
awk '
    BEGIN {
        printf "%s", "{\"operations\":[{\"id\":\"crate\",\"op\":\"add-item\",\"projection\":{" \
            "\"path\":\"examples/real_programs/levenshtein/upstream.lkj\",\"source\":\""
    }
    /[[:cntrl:]]/ { invalid = 1; exit }
    {
        for (i = 1; i <= length($0); i++) {
            byte = substr($0, i, 1)
            if (byte == "\\" || byte == "\"") printf "\\"
            printf "%s", byte
        }
        printf "\\n"
    }
    END {
        if (invalid) exit 1
        print "\"},\"source\":\"mod upstream;\"}],\"version\":1}"
    }
' "$projection" > "$oracle/import.json"
