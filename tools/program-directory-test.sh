#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
root=$(pwd)
suite_started_ns=$(date +%s%N)
suite_load_before=$(cat /proc/loadavg)
adoption_reuse=()
if [[ -n "${LKJSCRIPT_STORE:-}" && -n "${LKJSCRIPT_STORE_KEY:-}" ]]; then
    adoption_reuse=(--store "$LKJSCRIPT_STORE" --store-key "$LKJSCRIPT_STORE_KEY")
fi
if [[ "$compiler" != /* ]]; then compiler="$root/$compiler"; fi
work=$(mktemp -d "$root/target/program-directory.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)) && [[ -f "$work/errors" ]]; then cat "$work/errors" >&2; fi
    rm -rf "$work"
    exit "$status"
}
trap cleanup EXIT HUP INT TERM
umask 077
program="$work/program"
mkdir -p "$program/data" "$program/other" "$work/store"
printf '42\n' > "$program/data/number"
printf '7\n' > "$program/other/number"
head -c 32 /dev/urandom > "$work/key"
cat > "$program/program.lkj" <<'SOURCE'
const SOURCE: &str = "main.lkj";
const ENTRY: &str = "main";
const GRANTS: &[&str] = &["out", "root=data"];
const CASES: &[&str] = &["cases.lkj"];
SOURCE
cp "$program/program.lkj" "$work/manifest"
cat > "$program/main.lkj" <<'SOURCE'
mod helper;
pub fn main(mut out: Stdout, mut root: Directory) {
    assert_eq!(helper::answer(), 42);
    let mut file = root.open("number").unwrap();
    let bytes = file.read_to_end().unwrap();
    out.write(&bytes);
}
SOURCE
printf 'pub fn answer() -> i64 { 42 }\n' > "$program/helper.lkj"
cat > "$program/cases.lkj" <<'SOURCE'
// expect: value 42
pub fn probe() -> i64 { 42 }
SOURCE
fail() { printf 'program directory: %s\n' "$*" >&2; exit 1; }
reject() {
    local label=$1 expected=$2
    shift 2
    if "$compiler" "$@" >"$work/errors" 2>&1; then fail "$label was accepted"; fi
    grep -Fq "$expected" "$work/errors" || fail "$label did not name $expected"
}
cp "$program/main.lkj" "$program/renamed.rs"
reject source-suffix E_SOURCE_EXTENSION check "$program/renamed.rs"
rm "$program/renamed.rs"
mv "$program/helper.lkj" "$program/helper.rs"
reject module-suffix E_MODULE_MISSING check "$program/main.lkj"
mv "$program/helper.rs" "$program/helper.lkj"
"$compiler" build "$program" -o "$work/image"
(cd /; "$work/image") > "$work/output"
cmp "$program/data/number" "$work/output"
"$compiler" authority "$program" > "$work/authority"
grep -Fq "$program/program.lkj" "$work/authority" || fail 'authority omitted manifest'
grep -Fq 'root=data' "$work/authority" || fail 'authority omitted configured value'
"$compiler" test "$program" --jobs 1 > "$work/tests"
grep -Fq '2 route checks' "$work/tests" || fail 'directory cases missed an executor'
sed 's/"cases.lkj"/"."/' "$work/manifest" > "$program/program.lkj"
"$compiler" test "$program" --jobs 1 > "$work/tests"
grep -Fq '2 route checks' "$work/tests" || fail 'directory case collection did not run'
cp "$work/manifest" "$program/program.lkj"
"$compiler" build "$program"
cmp "$work/image" "$program/program.elf"

# Manifest context is part of the request key: raw relative direct-CLI grants
# have a different meaning even with the same source root, entry and spellings.
reuse=(--store "$work/store" --store-key "$work/key")
"$compiler" build "$program" "${reuse[@]}" -o "$work/cached"
"$compiler" build main.lkj --source-dir "$program" --entry main --grant out --grant root=data \
    "${reuse[@]}" -o "$work/direct"
"$compiler" build main.lkj --source-dir "$program" --entry main --grant out --grant root=data \
    --no-store -o "$work/reference"
cmp "$work/direct" "$work/reference"
sed 's/root=data/root=other/' "$work/manifest" > "$program/program.lkj"
"$compiler" build "$program" "${reuse[@]}" -o "$work/cached"
(cd /; "$work/cached") > "$work/output"
cmp "$program/other/number" "$work/output"
cp "$work/manifest" "$program/program.lkj"

# An existing env: directory grant selects its value at invocation; it is not
# a manifest-relative path. The same built image must observe either directory.
sed 's/root=data/root=env:LKJSCRIPT_PROGRAM_DIRECTORY_ROOT/' "$work/manifest" > "$program/program.lkj"
"$compiler" build "$program" "${reuse[@]}" -o "$work/environment"
"$compiler" authority "$program" > "$work/authority"
grep -Fq 'root=env:LKJSCRIPT_PROGRAM_DIRECTORY_ROOT' "$work/authority" || fail 'authority lost the environment grant'
for selection in data other; do
    (cd /; env LKJSCRIPT_PROGRAM_DIRECTORY_ROOT="$program/$selection" "$work/environment") > "$work/output"
    cmp "$program/$selection/number" "$work/output"
done
for engine in native interpreter; do
    env LKJSCRIPT_PROGRAM_DIRECTORY_ROOT="$program/other" "$compiler" run "$program" --engine "$engine" --no-store > "$work/output"
    cmp "$program/other/number" "$work/output"
done
sed 's/root=data/root=env:/' "$work/manifest" > "$program/program.lkj"
reject empty-environment 'directory environment grant needs one nonempty variable name' build "$program"
cp "$work/manifest" "$program/program.lkj"

for command in build test authority; do
    sed 's/"out", "root=data"/"out"/' "$work/manifest" > "$program/program.lkj"
    reject "missing-$command" 'missing --grant root' "$command" "$program"
    grep -Fq "$program/program.lkj" "$work/errors" || fail 'missing grant omitted manifest'
    grep -Fq 'main.lkj::main(out, root)' "$work/errors" || fail 'missing grant omitted entry signature'
    sed 's/"out", "root=data"/"out", "root=data", "clock"/' "$work/manifest" > "$program/program.lkj"
    reject "extra-$command" 'unknown grant clock' "$command" "$program"
    grep -Fq "$program/program.lkj" "$work/errors" || fail 'unused grant omitted manifest'
    grep -Fq 'main.lkj::main(out, root)' "$work/errors" || fail 'unused grant omitted entry signature'
done
cp "$work/manifest" "$program/program.lkj"
reject override-grant E_PROGRAM build "$program" --grant out
reject override-entry E_PROGRAM authority "$program" --entry main
reject override-root E_PROGRAM test "$program" --source-dir "$program"
sed 's/expect: value 42/expect: value 41/' "$program/cases.lkj" > "$work/bad-case"
cp "$work/bad-case" "$program/cases.lkj"
reject case-failure E_TEST test "$program" --jobs 1

sed 's/SOURCE: \&str = "main.lkj"/SOURCE: \&str = "..\/main.lkj"/' "$work/manifest" > "$program/program.lkj"
reject source-escape E_PROGRAM build "$program"
sed 's/\&\["cases.lkj"\]/\&["..\/cases.lkj"]/' "$work/manifest" > "$program/program.lkj"
reject case-escape E_PROGRAM test "$program"
sed '/const CASES/d' "$work/manifest" > "$program/program.lkj"
reject missing-cases 'missing CASES' authority "$program"
cp "$work/manifest" "$program/program.lkj"
printf 'const ENTRY: &str = "main";\n' >> "$program/program.lkj"
reject duplicate-entry 'duplicate ENTRY' build "$program"
sed 's/SOURCE: \&str = "main.lkj"/SOURCE: \&str = choose()/' "$work/manifest" > "$program/program.lkj"
reject literal-only 'SOURCE must be a string literal' build "$program"
sed 's/GRANTS: \&\[\&str\]/GRANTS: bool/' "$work/manifest" > "$program/program.lkj"
reject manifest-type 'GRANTS must have type &[&str]' build "$program"

# Each reviewed directory builds its real entry and runs a focused contract.
# Full module suites remain in the consumer campaigns; they are not repeated here.
printf 'Program directory fixture %s ms; load %s -> %s\n' \
    "$((($(date +%s%N) - suite_started_ns) / 1000000))" "$suite_load_before" "$(cat /proc/loadavg)"
for adopted in examples/site examples/journal examples/desk; do
    if [[ ! -f "$adopted/program.lkj" ]]; then
        printf 'pending program-directory %s: supply SOURCE, ENTRY, GRANTS and CASES in program.lkj\n' "$adopted"
        continue
    fi
    adopted_started_ns=$(date +%s%N)
    adopted_load_before=$(cat /proc/loadavg)
    "$compiler" build "$adopted" "${adoption_reuse[@]}" -o "$work/adopted"
    "$compiler" authority "$adopted" "${adoption_reuse[@]}" > "$work/adopted-authority"
    adopted_built_ns=$(date +%s%N)
    adopted_load_built=$(cat /proc/loadavg)
    "$compiler" test "$adopted" --jobs 1 --stats "${adoption_reuse[@]}"
    printf 'Program directory %s build+authority %s ms; cases %s ms; load %s -> %s -> %s\n' \
        "$adopted" "$(((adopted_built_ns - adopted_started_ns) / 1000000))" \
        "$((($(date +%s%N) - adopted_built_ns) / 1000000))" \
        "$adopted_load_before" "$adopted_load_built" "$(cat /proc/loadavg)"
done
printf 'ok program directories (module discovery, cases, authority, exact grants, invocation environment, literal source, isolated cache identity)\n'
