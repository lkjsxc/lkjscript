#!/usr/bin/env bash
# Offline source-store gate. Optional second argument retains the tested tool image
# for an explicit real-source round trip; all other outputs remain disposable.
set -euo pipefail
cd "$(dirname "$0")/.."
io_only=no
if [[ ${1:-} == --io-only ]]; then io_only=yes; shift; fi
[[ $# -le 2 ]] || { printf 'usage: %s [--io-only] [COMPILER [NEW-IMAGE-OUTPUT]]\n' "$0" >&2; exit 2; }
compiler=${1:-target/lkjscript}
image_output=${2:-}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler" ;; esac
[[ -x "$compiler" ]] || { printf 'missing compiler: %s\n' "$compiler" >&2; exit 1; }
[[ -z "$image_output" || ! -e "$image_output" ]] || { printf 'image output already exists\n' >&2; exit 1; }
umask 077
mkdir -p target
work=$(mktemp -d "$PWD/target/programs-package-test.XXXXXX")
cleanup() {
    local result=$?
    trap - EXIT
    if ((result)); then
        for log in "$work"/*.err "$work"/*-tests.out; do
            [[ ! -f "$log" ]] || { printf '%s\n' "$log" >&2; cat "$log" >&2; }
        done
    fi
    rm -rf -- "$work"
    exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
if [[ "$io_only" == no ]]; then
    perl tools/programs-package-manifest-test.pl tools/programs-package-manifest.pl "$work"
    printf 'ok 6 tests\n' > "$work/expected-tests.out"
    for engine in native interpreter; do
        "$compiler" test src/programs_package_tool.lkj --source-dir . --library-dir library --no-store --jobs 1 --engine "$engine" > "$work/$engine-tests.out" 2> "$work/$engine-tests.err"
        cmp "$work/expected-tests.out" "$work/$engine-tests.out"
        test ! -s "$work/$engine-tests.err"
    done
    printf 'ok package contracts: 6 tests on each engine\n'
fi
"$compiler" build src/programs_package_tool.lkj --source-dir . --library-dir library --no-store \
    --grant arguments --grant source=env:LKJSCRIPT_PACKAGE_SOURCE --grant policy=env:LKJSCRIPT_PACKAGE_POLICY --grant store=env:LKJSCRIPT_PACKAGE_STORE \
    --grant issuer=env:LKJSCRIPT_PACKAGE_ISSUER --grant sink=env:LKJSCRIPT_PACKAGE_SINK \
    --grant output --grant errors --grant status -o "$work/package" > "$work/build.out" 2> "$work/build.err"
mkdir -p "$work/source/src" "$work/catalog" "$work/store" "$work/issuer" "$work/wrong-issuer"
dd if=/dev/urandom of="$work/issuer/source-key" bs=32 count=1 status=none
dd if=/dev/urandom of="$work/wrong-issuer/source-key" bs=32 count=1 status=none
printf 'permission\n' > "$work/source/LICENSE"
printf 'pub fn answer() -> u8 { 7 }\n' > "$work/source/src/lib.rs"
: > "$work/source/empty"
cp "$work/source/src/lib.rs" "$work/original.rs"
git -C "$work/source" -c core.hooksPath=/dev/null init -q
git -C "$work/source" -c core.hooksPath=/dev/null add LICENSE src/lib.rs empty
git -C "$work/source" -c core.hooksPath=/dev/null -c user.name=Fixture -c user.email=fixture@example.invalid commit -qm fixture
revision=$(git -C "$work/source" rev-parse HEAD)
export LKJSCRIPT_PACKAGE_SOURCE="$work/source"
export LKJSCRIPT_PACKAGE_POLICY="$work/catalog"
export LKJSCRIPT_PACKAGE_STORE="$work/store"
export LKJSCRIPT_PACKAGE_ISSUER="$work/issuer"
export LKJSCRIPT_PACKAGE_SINK="$work/store"
producer=(perl tools/programs-package-manifest.pl --git --source "$work/source" --revision "$revision" \
    --origin https://example.invalid/fixture --notice LICENSE)
"${producer[@]}" --name fixture --version 1.0 --output "$work/catalog/fixture" > "$work/inventory.json"
"$work/package" issue fixture > "$work/issue.out" 2> "$work/issue.err"
printf 'issued fixture\n' > "$work/expected-issue.out"
cmp "$work/expected-issue.out" "$work/issue.out"
test ! -s "$work/issue.err"
"$work/package" read fixture src/lib.rs > "$work/read.out" 2> "$work/read.err"
cmp "$work/original.rs" "$work/read.out"
test ! -s "$work/read.err"
"$work/package" read fixture empty > "$work/empty.out" 2> "$work/empty.err"
test ! -s "$work/empty.out" && test ! -s "$work/empty.err"
package_controls=3
expect_reject() {
    local code=$1
    shift
    local code_status=0
    "$@" > "$work/rejected.out" 2> "$work/rejected.err" || code_status=$?
    if ((code_status != 1)); then
        printf 'expected normal source-package refusal (%s), got status %s\n' "$code" "$code_status" >&2
        exit 1
    fi
    test ! -s "$work/rejected.out"
    printf 'package %s\n' "$code" > "$work/expected-rejected.err"
    cmp "$work/expected-rejected.err" "$work/rejected.err"
    ((package_controls+=1))
}
printf 'not admitted\n' > "$work/source/unlisted"
expect_reject path "$work/package" read fixture unlisted
expect_reject path "$work/package" read fixture ../issuer/source-key
printf 'pub fn answer() -> u8 { 8 }\n' > "$work/source/src/lib.rs"
expect_reject source-digest "$work/package" read fixture src/lib.rs
: > "$work/source/src/lib.rs"
expect_reject source-length "$work/package" read fixture src/lib.rs
truncate -s 1048576 "$work/source/src/lib.rs"
expect_reject source-length "$work/package" read fixture src/lib.rs
rm "$work/source/src/lib.rs"
expect_reject source-missing "$work/package" read fixture src/lib.rs
cp "$work/original.rs" "$work/source/src/lib.rs"
cp "$work/catalog/fixture.pin.json" "$work/original.pin"
# The producer writes compact JSON with no trailing newline. Change only the target fields.
LC_ALL=C sed -E 's/^(\["lkjscript-package-pin-1",\["fixture",)"1\.0"/\1"other-version"/' \
    "$work/original.pin" > "$work/catalog/fixture.pin.json"
expect_reject identity "$work/package" read fixture src/lib.rs
LC_ALL=C sed -E 's/"([0-9a-f]{64})"\],\[\]\]$/"\1"],[["unused","1","local","revision","\1"]]]/' \
    "$work/original.pin" > "$work/catalog/fixture.pin.json"
expect_reject dependencies "$work/package" read fixture src/lib.rs
cp "$work/original.pin" "$work/catalog/fixture.pin.json"
cp "$work/store/fixture.package" "$work/original.package"
last_offset=$(($(wc -c < "$work/original.package") - 1))
last_byte=$(od -An -tu1 -j "$last_offset" -N 1 "$work/original.package")
printf '%b' "\\0$(printf '%03o' "$((last_byte ^ 1))")" |
    dd of="$work/store/fixture.package" bs=1 seek="$last_offset" conv=notrunc status=none
expect_reject authentication "$work/package" read fixture src/lib.rs
cp "$work/original.package" "$work/store/fixture.package"
store_magic=$'lkjscript-store-2\n'
# Overwrite the eight-byte little-endian length with 1048576 (1 MiB).
printf '\x00\x00\x10\x00\x00\x00\x00\x00' |
    dd of="$work/store/fixture.package" bs=1 seek="${#store_magic}" conv=notrunc status=none
expect_reject authentication "$work/package" read fixture src/lib.rs
cp "$work/original.package" "$work/store/fixture.package"
truncate -s "$last_offset" "$work/store/fixture.package"
expect_reject authentication "$work/package" read fixture src/lib.rs
cp "$work/original.package" "$work/store/fixture.package"
printf '\0' >> "$work/store/fixture.package"
expect_reject authentication "$work/package" read fixture src/lib.rs
cp "$work/original.package" "$work/store/fixture.package"
expect_reject authentication env LKJSCRIPT_PACKAGE_ISSUER="$work/wrong-issuer" "$work/package" read fixture src/lib.rs
cp "$work/original.pin" "$work/catalog/candidate.pin.json"
cp "$work/catalog/fixture.manifest.json" "$work/catalog/candidate.manifest.json"
printf 'pub fn answer() -> u8 { 8 }\n' > "$work/source/src/lib.rs"
expect_reject source-digest "$work/package" issue candidate
test ! -e "$work/store/candidate.package"
cp "$work/original.rs" "$work/source/src/lib.rs"
{ printf ' '; cat "$work/catalog/fixture.manifest.json"; } > "$work/catalog/candidate.manifest.json"
expect_reject canonical "$work/package" issue candidate
LC_ALL=C sed 's/\["LICENSE"\]\]$/["unlisted"]]/' \
    "$work/catalog/fixture.manifest.json" > "$work/catalog/candidate.manifest.json"
expect_reject notice "$work/package" issue candidate
env LKJSCRIPT_PACKAGE_SINK="$work/source" "$work/package" owned-snapshot fixture src/lib.rs > "$work/snapshot.out" 2> "$work/snapshot.err"
cmp "$work/original.rs" "$work/snapshot.out"
test ! -s "$work/snapshot.err"
((package_controls+=1))
cp "$work/original.rs" "$work/source/src/lib.rs"
"${producer[@]}" --name child --version 1.0 --output "$work/catalog/child" > "$work/child-inventory.json"
"$work/package" issue child > "$work/child-issue.out" 2> "$work/child-issue.err"
"${producer[@]}" --name parent --version 1.0 --dependency-pin "$work/catalog/child.pin.json" --output "$work/catalog/parent" > "$work/parent-inventory.json"
"$work/package" issue parent > "$work/parent-issue.out" 2> "$work/parent-issue.err"
"$work/package" read-edge parent child src/lib.rs > "$work/edge.out" 2> "$work/edge.err"
cmp "$work/original.rs" "$work/edge.out"
test ! -s "$work/edge.err"
((package_controls+=1))
"${producer[@]}" --name child --version 2.0 --output "$work/catalog/child-v2" > "$work/child-v2-inventory.json"
"$work/package" issue child-v2 > "$work/child-v2-issue.out" 2> "$work/child-v2-issue.err"
expect_reject dependencies "$work/package" read-edge parent child-v2 src/lib.rs
sha256sum "$work/catalog/child.pin.json" > "$work/catalog-before.sha256"
cp "$work/store/child-v2.package" "$work/store/child.package"
cp "$work/catalog/child-v2.pin.json" "$work/store/child.pin.json"
expect_reject identity "$work/package" read child src/lib.rs
expect_reject identity "$work/package" read-edge parent child src/lib.rs
sha256sum "$work/catalog/child.pin.json" > "$work/catalog-after.sha256"
cmp "$work/catalog-before.sha256" "$work/catalog-after.sha256"
if [[ -n "$image_output" ]]; then ln "$work/package" "$image_output"; fi
printf 'ok package source: %s native I/O and substitution controls, owned-buffer preservation, pinned dependency edge and trusted-catalog isolation\n' "$package_controls"
