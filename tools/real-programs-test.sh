#!/usr/bin/env bash
# Required offline gate: original assertions, verified provenance and native corpus.
# The expensive full interpreted corpus is an explicit milestone rerun only.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=target/lkjscript
full_interpreter=no
if (($#)) && [[ $1 != --* ]]; then compiler=$1; shift; fi
while (($#)); do
    case "$1" in
        --full-interpreter) full_interpreter=yes; shift ;;
        *) printf 'usage: %s [COMPILER] [--full-interpreter]\n' "$0" >&2; exit 2 ;;
    esac
done
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler" ;; esac
[[ -x "$compiler" ]] || { printf 'compiler is not executable: %s\n' "$compiler" >&2; exit 1; }
source=examples/real_programs/levenshtein
entry="$source/main.lkj"
golden=tests/fixtures/real_programs_levenshtein.output.gz
[[ -f "$golden" ]] || { printf 'missing checked corpus: %s\n' "$golden" >&2; exit 1; }
mkdir -p target
work=$(mktemp -d "$PWD/target/real-programs.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT
    if ((status)); then
        for log in "$work"/*.err "$work"/*-tests.out "$work/authority.out"; do
            [[ ! -f "$log" ]] || { printf '%s\n' "$log" >&2; cat "$log" >&2; }
        done
    fi
    rm -rf -- "$work"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
# The untrusted source inventory/projection tools must preserve Git bytes and context.
node tools/real-programs/census.mjs --self-test > "$work/census-controls.json"
node tools/real-programs/project.mjs --self-test > "$work/projection-controls.json"
node tools/real-programs/probe-test.mjs > "$work/probe-controls.json"
node tools/real-programs/rerank-test.mjs > "$work/rerank-controls.json"
"$compiler" build tools/real-programs/levenshtein-provenance.lkj --library-dir library --no-store \
    --grant source=env:LKJSCRIPT_PROVENANCE_SOURCE --grant output --grant errors --grant status \
    -o "$work/provenance" > "$work/provenance-build.out" 2> "$work/provenance-build.err"
LKJSCRIPT_PROVENANCE_SOURCE="$PWD/$source" "$work/provenance" > "$work/provenance.json" 2> "$work/provenance.err"
test ! -s "$work/provenance.err"
"$compiler" build tools/real-programs/levenshtein-checksums.lkj --library-dir library --no-store \
    --grant report --grant output --grant errors --grant status -o "$work/checksums" \
    > "$work/checksums-build.out" 2> "$work/checksums-build.err"
"$work/checksums" < "$work/provenance.json" > "$work/restored.sha256" 2> "$work/checksums.err"
test ! -s "$work/checksums.err"
# Recompute the inverse projection from the specimen, compare every report byte,
# and run the independent mutation controls before consuming the native report.
node tools/real-programs/verify-levenshtein.mjs --check-report "$work/provenance.json" "$source" --checksums "$work/restored.sha256"
# A malformed second digest must refuse before emitting the first checksum line.
checksum_status=0
printf '{"files":{"src/lib.rs":{"sha256":"%064d"},"tests/lib.rs":{"sha256":"bad"}}}\n' 0 |
    "$work/checksums" > "$work/checksums-refused.out" 2> "$work/checksums-refused.err" || checksum_status=$?
test "$checksum_status" -eq 255
test ! -s "$work/checksums-refused.out"
printf 'levenshtein-checksums: invalid SHA-256 for tests/lib.rs\n' | cmp - "$work/checksums-refused.err"
mkdir "$work/changed"
cp "$source/upstream.json" "$source/LICENSE" "$work/changed/"
{ printf ' '; cat "$source/upstream.lkj"; } > "$work/changed/upstream.lkj"
provenance_status=0
LKJSCRIPT_PROVENANCE_SOURCE="$work/changed" "$work/provenance" \
    > "$work/provenance-refused.out" 2> "$work/provenance-refused.reason" || provenance_status=$?
test "$provenance_status" -eq 255
test ! -s "$work/provenance-refused.out"
printf 'provenance refusal: upstream bytes differ: src/lib.rs\n' | cmp - "$work/provenance-refused.reason"
# Keep the checked import and its normalization receipt intact. The executed
# specimen restores the original range syntax, then authenticates both complete
# upstream sections before the compiler reads the temporary .lkj projection.
restored="$work/restored"
mkdir "$restored"
"$compiler" build tools/real-programs/restore-levenshtein.lkj --library-dir library --no-store \
    --grant arguments --grant source=env:LKJSCRIPT_RESTORE_SOURCE --grant output=env:LKJSCRIPT_RESTORE_OUTPUT \
    --grant errors --grant status -o "$work/restore" > "$work/restore-build.out" 2> "$work/restore-build.err"
LKJSCRIPT_RESTORE_SOURCE="$PWD/$source" LKJSCRIPT_RESTORE_OUTPUT="$restored" "$work/restore"
original="$work/original"
mkdir "$original"
mv "$restored/upstream.lkj" "$original/upstream.lkj"
cp "$source/main.lkj" "$source/corpus.lkj" "$original/"
# The importer itself is now lkjscript: exercise both routes against the exact
# restored upstream tree and refuse a changed digest before writing a request.
import_source="$work/import-upstream"
mkdir -p "$import_source/src" "$import_source/tests"
LKJSCRIPT_RESTORE_SOURCE="$PWD/$source" LKJSCRIPT_RESTORE_OUTPUT="$restored" "$work/restore" --rust
mv "$restored/upstream.rs" "$import_source/src/lib.rs"
mv "$restored/upstream_tests.rs" "$import_source/tests/lib.rs"
# The compiler under test builds the restorer; foreign tools check every byte.
(cd "$import_source" && sha256sum --check --strict --quiet "$work/restored.sha256")
{ cat "$import_source/src/lib.rs"
  printf '\n// UPSTREAM INTEGRATION TESTS: exact bodies, package header removed.\n'
  tail -n +3 "$import_source/tests/lib.rs"; } | cmp - "$original/upstream.lkj"
# A refusal must precede publication of either restored file.
mkdir "$work/refused"
restore_status=0
LKJSCRIPT_RESTORE_SOURCE="$work/changed" LKJSCRIPT_RESTORE_OUTPUT="$work/refused" \
    "$work/restore" --rust > "$work/refused.out" 2> "$work/refused.reason" || restore_status=$?
test "$restore_status" -eq 1
test ! -s "$work/refused.out"
test -z "$(ls -A "$work/refused")"
printf 'restore-levenshtein: original library bytes differ\n' | cmp - "$work/refused.reason"
cp "$source/LICENSE" "$import_source/license"
"$compiler" build tools/real-programs/levenshtein-import.lkj --source-dir . --library-dir library --no-store -o "$work/importer" > "$work/import-build.out" 2> "$work/import-build.err"
"$work/importer" "$import_source" "$source/upstream.json" "$work/import-native.json" > "$work/import-native.out" 2> "$work/import-native.err"
"$compiler" run tools/real-programs/levenshtein-import.lkj --source-dir . --library-dir library --no-store --engine interpreter -- "$import_source" "$source/upstream.json" "$work/import-interpreter.json" > "$work/import-interpreter.out" 2> "$work/import-interpreter.err"
cmp "$work/import-native.out" "$work/import-interpreter.out"
test ! -s "$work/import-native.err"
test ! -s "$work/import-interpreter.err"
node --input-type=module - "$source" "$work" <<'NODE'
import fs from 'node:fs';
import assert from 'node:assert/strict';
const [source, work] = process.argv.slice(2);
const expected = {version:1, operations:[{op:'add-item', id:'crate', source:'mod upstream;',
    projection:{path:'examples/real_programs/levenshtein/upstream.lkj',
        source:fs.readFileSync(source + '/upstream.lkj','utf8')}}]};
for (const engine of ['native','interpreter']) {
    assert.deepEqual(JSON.parse(fs.readFileSync(work + '/import-' + engine + '.json','utf8')), expected);
}
NODE
printf '\nchanged digest\n' >> "$import_source/license"
if "$work/importer" "$import_source" "$source/upstream.json" "$work/import-refused.json" > "$work/import-refused.out" 2> "$work/import-refused.err"; then
    printf 'importer admitted changed upstream bytes\n' >&2
    exit 1
else
    test "$?" -eq 255
fi
test ! -e "$work/import-refused.json"
grep -Fq 'upstream digest mismatch: license' "$work/import-refused.err"
entry="${original#"$PWD/"}"/main.lkj
gzip -dc "$golden" > "$work/expected.out"
awk -F: 'NF != 2 || $1 != NR-1 || $2 !~ /^[0-9]+$/ { exit 1 } END { if (NR != 7239) exit 1 }' "$work/expected.out"
printf 'ok 8 tests\n' > "$work/expected-tests.out"
for engine in native interpreter; do
    "$compiler" test "$entry" --source-dir . --library-dir library --no-store --jobs 1 --engine "$engine" > "$work/$engine-tests.out" 2> "$work/$engine-tests.err"
    cmp "$work/expected-tests.out" "$work/$engine-tests.out"
    test ! -s "$work/$engine-tests.err"
done
# This upstream test reaches the algorithm. Its checked closure must have no
# grants and no observable operations, including any hidden file or oracle read.
"$compiler" authority "$entry" --source-dir . --library-dir library --no-store --entry upstream::should_work > "$work/authority.out" 2> "$work/authority.err"
cat > "$work/expected-authority.out" <<'AUTHORITY'
entry upstream::should_work
declared authority:
  none
reachable operations (conservative checked-IR branches and owned cleanup):
  none
AUTHORITY
sed -n '1,5p' "$work/authority.out" > "$work/authority-prefix.out"
cmp "$work/expected-authority.out" "$work/authority-prefix.out"
grep -Fxq '  upstream::levenshtein' "$work/authority.out"
test ! -s "$work/authority.err"
"$compiler" build "$entry" --source-dir . --library-dir library --no-store --grant output -o "$work/specimen" > "$work/build.out" 2> "$work/build.err"
"$work/specimen" > "$work/native.out" 2> "$work/native.err"
cmp "$work/expected.out" "$work/native.out"
test ! -s "$work/native.err"
if [[ "$full_interpreter" == yes ]]; then
    "$compiler" run "$entry" --source-dir . --library-dir library --no-store --grant output --engine interpreter > "$work/interpreter.out" 2> "$work/interpreter.err"
    cmp "$work/expected.out" "$work/interpreter.out"
    test ! -s "$work/interpreter.err"
    printf 'ok real programs: full interpreted 7,239-row corpus\n'
fi
printf 'ok real programs: original library syntax, independently checked native provenance, 18 provenance and 4 manifest-shape controls, native restorer with foreign hash check and refusal, native/interpreter importer, 8 tests on each engine, pure algorithm authority and 7,239 native corpus rows\n'
