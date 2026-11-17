#!/usr/bin/env bash
# Source meaning and checked transactions retain associated equality bindings.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
mkdir -p target
fixture=$(mktemp -d "$PWD/target/programs-associated-bindings.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)); then
        for log in signature references transaction; do
            [[ ! -f "$fixture/$log" ]] || cat "$fixture/$log" >&2
        done
    fi
    rm -rf -- "$fixture"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
mkdir "$fixture/project" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/project/main.lkj" <<'SOURCE'
trait Source { type Item; fn get(self) -> Self::Item; }
struct Number {}
impl Source for Number { type Item = i64; fn get(self) -> i64 { 42 } }
fn take<S: Source<Item = i64>>(source: S) -> i64 {
    let value: <S as Source>::Item = source.get();
    value
}
fn main() -> i64 { take(Number {}) }
SOURCE
common=(main.lkj --source-dir "$fixture/project" --no-library --store "$fixture/store" --store-key "$fixture/key")
"$compiler" build "${common[@]}" --index -o "$fixture/image" > "$fixture/index"
snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/index")
[[ ${#snapshot} == 64 ]]
"$compiler" query "${common[@]}" --snapshot "$snapshot" --kind signature --id take --limit 32 --bytes 8192 > "$fixture/signature"
grep -Fq 'Source<Item = i64>' "$fixture/signature"
"$compiler" query "${common[@]}" --snapshot "$snapshot" --kind uses --id 'Source::type:Item' --limit 32 --bytes 8192 > "$fixture/references"
grep -Fq 'kind=associated-type-binding' "$fixture/references"
grep -Fq 'kind=associated-type' "$fixture/references"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Source::type:Item","name":"Output"}]}
REQUEST
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/transaction"
grep -q 'mode=apply' "$fixture/transaction"
grep -Fq 'Source<Output = i64>' "$fixture/project/main.lkj"
grep -Fq '<S as Source>::Output' "$fixture/project/main.lkj"
grep -Fq 'type Output = i64' "$fixture/project/main.lkj"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"take","name":"read_value"}]}
REQUEST
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/transaction"
grep -q 'mode=apply' "$fixture/transaction"
grep -Fq 'fn read_value<S: Source<Output = i64>>' "$fixture/project/main.lkj"
grep -Fq 'read_value(Number {})' "$fixture/project/main.lkj"
for engine in native interpreter; do
    [[ $("$compiler" run main.lkj --source-dir "$fixture/project" --no-library --no-store --engine "$engine") == 42 ]]
done
printf 'ok associated equality and qualified source meaning, member references and checked rename on both engines\n'
