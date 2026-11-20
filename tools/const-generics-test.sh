#!/usr/bin/env bash
# Const source identities survive queries, checked edits and cached specialization.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
mkdir -p target
fixture=$(mktemp -d "$PWD/target/const-generics.XXXXXX")
cleanup() {
    status=$?
    if ((status)); then
        cat "$fixture/source/main.lkj" >&2
        for file in "$fixture"/*type "$fixture"/*references "$fixture"/rename "$fixture"/edit "$fixture"/*.log; do
            [[ ! -f "$file" ]] || cat "$file" >&2
        done
    fi
    rm -rf "$fixture"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
mkdir "$fixture/source" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/source/main.lkj" <<'SOURCE'
const OFFSET: usize = 0;
fn repeat<const N: usize>(value: u8) -> [u8; N] { [value; N] }
fn width<const N: usize>(value: [u8; N]) -> usize { N + value.len() + OFFSET }
fn main() -> i64 { width::<20>(repeat::<20>(1u8)) as i64 }
SOURCE
common=(main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/store" --store-key "$fixture/key")
publish() {
    "$compiler" build "${common[@]}" --index --stats -o "$fixture/program" > "$fixture/$1.log" 2>&1
    snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/$1.log")
    [[ ${#snapshot} == 64 ]]
}
query() {
    "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$1" --id "$2" --limit 128 --bytes 100000
}
stat() { sed -n "s/.*\"$2\":\([0-9][0-9]*\).*/\1/p" "$fixture/$1.log" | tail -n 1; }
meaning() {
    local phase=$1 name=$2
    query type repeat > "$fixture/$phase-type"
    grep -Fq "fn<const $name: usize>(u8) -> [u8; $name]" "$fixture/$phase-type"
    query type "repeat::generic:$name" > "$fixture/$phase-parameter-type"
    grep -Fq 'value="usize"' "$fixture/$phase-parameter-type"
    query uses "repeat::generic:$name" > "$fixture/$phase-references"
    grep -Fq 'const-parameter' "$fixture/$phase-references"
    if grep -Eq '\$const-(value|rigid|array)' "$fixture/$phase-type" "$fixture/$phase-parameter-type"; then
        echo 'private const identity escaped into source meaning' >&2; exit 1
    fi
}
execute() {
    for engine in native interpreter; do
        [[ $("$compiler" run "${common[@]}" --engine "$engine") == "$1" ]]
    done
}
publish cold
first_snapshot=$snapshot
meaning cold N
execute 40
publish warm
[[ $snapshot == "$first_snapshot" ]]
[[ $(stat warm build_reused) == 1 ]]
before=$(sha256sum "$fixture/source/main.lkj")
printf '%s\n' '{"version":1,"operations":[{"op":"rename","id":"width::generic:N","name":"OFFSET"}]}' > "$fixture/request"
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/capture-edit.log" 2>&1; then
    echo 'const rename captured an unrelated resolved global value' >&2; exit 1
fi
[[ $(sha256sum "$fixture/source/main.lkj") == "$before" ]]
grep -Fq 'CAPTURE' "$fixture/capture-edit.log"
printf '%s\n' '{"version":1,"operations":[{"op":"rename","id":"repeat::generic:N","name":"COUNT"}]}' > "$fixture/request"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rename"
grep -Fq 'repeat<const COUNT: usize>(value: u8) -> [u8; COUNT] { [value; COUNT] }' "$fixture/source/main.lkj"
publish renamed
meaning renamed COUNT
before=$(sha256sum "$fixture/source/main.lkj")
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"repeat","source":"{ [value; 0] }"}]}' > "$fixture/request"
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/invalid-edit.log" 2>&1; then
    echo 'checked edit accepted a literal array for a rigid const result' >&2; exit 1
fi
[[ $(sha256sum "$fixture/source/main.lkj") == "$before" ]]
grep -Fq 'E_TYPE' "$fixture/invalid-edit.log"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"main","source":"{ width::<21>(repeat::<21>(1u8)) as i64 }"}]}' > "$fixture/request"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/edit"
publish changed
[[ $snapshot != "$first_snapshot" ]]
[[ $(stat changed front_reused) == 0 ]]
meaning changed COUNT
execute 42
test ! -e "$fixture/source/.lkjscript-edit.pending"
printf 'ok const generics: symbolic source types, exact references, checked rename, rollback, layout edits, warm cache and both engines\n'

cp tests/conformance/pass/module_constant_array_length_import.lkj "$fixture/source/main.lkj"
publish module-length
query type imported > "$fixture/module-length-type"
grep -Fq 'fn() -> [u8; COUNT]' "$fixture/module-length-type"
query uses value@widths::WIDTH > "$fixture/module-length-references"
grep -Fq 'kind=alias-array-length' "$fixture/module-length-references"
grep -Fq 'value@widths::local' "$fixture/module-length-references"
query dependents value@widths::WIDTH > "$fixture/module-length-dependents"
grep -Fq 'value@imported' "$fixture/module-length-dependents"
query uses 'shadow::generic:COUNT' > "$fixture/shadow-references"
grep -Fq 'const-parameter' "$fixture/shadow-references"
printf '%s\n' '{"version":1,"operations":[{"op":"rename","id":"value@widths::WIDTH","name":"COLUMNS"}]}' > "$fixture/request"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/module-length-rename.log"
grep -Fq 'pub const COLUMNS: usize' "$fixture/source/main.lkj"
grep -Fq 'fn local() -> [u8; crate::widths::COLUMNS]' "$fixture/source/main.lkj"
grep -Fq 'use widths::COLUMNS as COUNT;' "$fixture/source/main.lkj"
grep -Fq 'fn imported() -> [u8; COUNT]' "$fixture/source/main.lkj"
grep -Fq 'fn shadow<const COUNT: usize>(value: u8) -> [u8; COUNT]' "$fixture/source/main.lkj"
grep -Fq 'let root_value: [u8; WIDTH]' "$fixture/source/main.lkj"
publish module-length-renamed
query uses value@widths::COLUMNS > "$fixture/module-length-renamed-references"
grep -Fq 'kind=alias-array-length' "$fixture/module-length-renamed-references"
execute 42
printf 'ok module constant array length: lexical value identity, exact references, checked rename and both engines\n'
