#!/usr/bin/env bash
# Compiler/image proof gate; optional C is an independent Linux startup witness.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
native=no
if (($#)); then shift; fi
if (($#)); then [[ $1 == --native-witness ]] || exit 2; native=yes; shift; fi
(($# == 0)) || exit 2
work=$(mktemp -d target/compiler-child-image.XXXXXX)
before=$(cut -d' ' -f1 /proc/loadavg)
started=$(date +%s%N)
cleanup() {
    local result=$?
    trap - EXIT HUP INT TERM
    if ((result)); then
        for file in "$work/error" "$work/build.err"; do [[ ! -f $file ]] || tail -30 "$file" >&2; done
    fi
    local after elapsed
    after=$(cut -d' ' -f1 /proc/loadavg)
    elapsed=$(($(date +%s%N)-started))
    awk -v ns="$elapsed" -v before="$before" -v after="$after" -v status="$result" 'BEGIN { printf "child-image gate %.3f s load %s -> %s status=%s\n", ns/1000000000, before, after, status }'
    rm -rf -- "$work"
    exit "$result"
}
trap cleanup EXIT HUP INT TERM
umask 077
mkdir "$work/source" "$work/root"
head -c 32 /dev/urandom > "$work/key"
head -c 32 /dev/urandom > "$work/other-key"
printf 'inside\n' > "$work/root/inside.txt"
printf 'outside\n' > "$work/outside.txt"
"$compiler" build src/child_image_cases.lkj --library-dir library --no-store -o "$work/cases" 2>"$work/build.err"
"$work/cases"
"$compiler" build tests/fixtures/authority/sealed_file.lkj --library-dir library --no-store -o "$work/sealed" 2>"$work/build.err"
"$work/sealed" > "$work/output"
for engine in native interpreter; do
    "$compiler" run tests/fixtures/authority/sealed_file.lkj --library-dir library --no-store --engine "$engine" > "$work/output" 2>"$work/error"
    grep -q '^ok sealed file aliases$' "$work/output"
done
build() {
    "$compiler" build-child child_image.lkj --source-dir tests/fixtures/authority --library-dir library --no-store --entry "$1" --image-key "$work/key" -o "$work/$1" > "$work/$1.authority" 2>"$work/error"
}
verify() {
    "$compiler" image-authority "$work/child" --image-key "$work/key" "$@" > "$work/authority" 2>"$work/error"
}
reject() {
    if "$@" > "$work/output" 2>"$work/error"; then printf 'unexpected child-image admission: %s\n' "$*" >&2; exit 1; fi
    grep -q E_CHILD_IMAGE "$work/error"
}
build child
build probe
cp "$work/probe" "$work/executed-copy"
"$work/cases" --sealed-exec "$work/executed-copy" "$work/probe.admission" "$work/key" "$compiler"
verify
cmp "$work/child.authority" "$work/authority"
reject "$compiler" image-authority "$work/child" --image-key "$work/other-key"
reject "$compiler" image-authority "$work/child" --image-key "$work/key" --entry absent
cp "$work/child.admission" "$work/admission-copy"
printf x | dd of="$work/child.admission" bs=1 seek=3 conv=notrunc status=none
reject "$compiler" image-authority "$work/child" --image-key "$work/key"
cp "$work/admission-copy" "$work/child.admission"
printf x >> "$work/child.admission"
reject "$compiler" image-authority "$work/child" --image-key "$work/key"
head -c 16 "$work/admission-copy" > "$work/child.admission"
reject "$compiler" image-authority "$work/child" --image-key "$work/key"
cp "$work/admission-copy" "$work/child.admission"
cp "$work/child" "$work/image-copy"
printf x | dd of="$work/child" bs=1 seek=0 conv=notrunc status=none
reject "$compiler" image-authority "$work/child" --image-key "$work/key"
cp "$work/image-copy" "$work/child"
printf x >> "$work/child"
reject "$compiler" image-authority "$work/child" --image-key "$work/key"
cp "$work/image-copy" "$work/child"
cp "$compiler" "$work/other-compiler"
printf x >> "$work/other-compiler"
reject "$work/other-compiler" image-authority "$work/child" --image-key "$work/key"
for kind in World Process ForeignCommand Stop Child; do
    printf 'pub fn child(value: %s) {}\n' "$kind" > "$work/source/main.lkj"
    reject "$compiler" build-child main.lkj --source-dir "$work/source" --library-dir library --no-store --entry child --image-key "$work/key" -o "$work/refused"
    [[ ! -e $work/refused && ! -e $work/refused.admission ]]
done
# Key content must not reach a producer, whether copied or hardlinked into its root.
for kind in copy link; do
    if [[ $kind == copy ]]; then cp "$work/key" "$work/source/key-bytes"; else ln "$work/key" "$work/source/key-bytes"; fi
    printf 'pub fn child() -> usize { include_bytes!("key-bytes").len() }\n' > "$work/source/main.lkj"
    reject "$compiler" build-child main.lkj --source-dir "$work/source" --library-dir library --no-store --entry child --image-key "$work/key" -o "$work/refused"
    grep -q 'private admission key' "$work/error"
    rm "$work/source/key-bytes"
done
cp "$work/key" "$work/source/main.lkj"
reject "$compiler" build-child main.lkj --source-dir "$work/source" --library-dir library --no-store --image-key "$work/key" -o "$work/refused"
grep -q 'private admission key' "$work/error"
ln "$work/key" "$work/source/key-ir.lkjir"
reject "$compiler" build-child "$work/source/key-ir.lkjir" --library-dir library --no-store --image-key "$work/key" -o "$work/refused"
grep -q 'private admission key' "$work/error"
printf 'pub fn child() -> i64 { 37 }\n' > "$work/source/main.lkj"
reject "$compiler" build-child main.lkj --source-dir "$work/source" --library-dir library --no-store --entry child --runtime-source "$work/source/key-ir.lkjir" --image-key "$work/key" -o "$work/refused"
grep -q 'private admission key' "$work/error"
if [[ $native == yes ]]; then
    cc -std=c11 -O2 -Wall -Wextra -Werror tools/child-image-witness.c -o "$work/witness"
    "$work/cases" "$work/child.admission" "$work/child.packet"
    for mode in valid magic manifest count kind overflow alias-offset truncated trailing unsealed wrong-descriptor missing-descriptor unsealed-alias deny-signals deny-mapping packet-as-result image-as-packet packet-as-descriptor; do
        "$work/witness" "$work/child" "$work/child.packet" "$work/root" "$mode"
    done
    cp "$work/child" "$work/substituted"
    "$work/witness" "$work/substituted" "$work/child.packet" "$work/root" replace-path
    for entry in descriptors proc_probe; do
        build "$entry"
        "$work/cases" "$work/$entry.admission" "$work/$entry.packet"
    done
    for mode in valid readfile-directory listener-file readiness-file; do
        "$work/witness" "$work/descriptors" "$work/descriptors.packet" "$work/root" "$mode"
    done
    "$work/cases" "$work/probe.admission" "$work/probe.packet"
    "$work/witness" "$work/proc_probe" "$work/proc_probe.packet" "$work/root" proc
    "$work/witness" "$work/probe" "$work/probe.packet" "$work/root" valid
    printf 'ok child startup: 25 executions, immutable aliases, substitution, packet/descriptor refusal, silent bootstrap refusal, ambient cleanup\n'
fi
printf 'ok child-image compiler admission, exact producer/ELF/entry and source-key refusal\n'
