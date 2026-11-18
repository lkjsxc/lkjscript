#!/usr/bin/env bash
# Authored defaults retain references and binder identity through checked edits.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
mkdir -p target
fixture=$(mktemp -d "$PWD/target/programs-type-defaults.XXXXXX")
trap 'if [[ ${LKJSCRIPT_TYPE_DEFAULTS_KEEP:-0} == 1 ]]; then printf "retained_fixture=%s\n" "$fixture" >&2; else rm -rf -- "$fixture"; fi' EXIT HUP INT TERM
umask 077
mkdir "$fixture/project" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/project/main.lkj" <<'SOURCE'
struct Payload { value: i64 }
struct Envelope<T = Payload, U = T> { first: T, second: U }
type Alias<T = Payload> = Envelope<T>;
fn explicit(value: Alias) -> Envelope<Payload, Payload> { value }
fn produce() -> Alias { Envelope { first: Payload { value: 19 }, second: Payload { value: 23 } } }
fn main() -> i64 { let result = explicit(produce()); result.first.value + result.second.value }
SOURCE
common=(main.lkj --source-dir "$fixture/project" --no-library --store "$fixture/store" --store-key "$fixture/key")
publish() {
    "$compiler" build "${common[@]}" --index -o "$fixture/image" > "$fixture/index"
    snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/index")
    [[ ${#snapshot} == 64 ]]
}
query() { "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$1" --id "$2" --limit 128 --bytes 65536; }
value() {
    for engine in native interpreter; do
        [[ $("$compiler" run main.lkj --source-dir "$fixture/project" --no-library --no-store --engine "$engine") == 42 ]]
    done
}
edit() {
    "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/transaction"
    grep -q 'mode=apply' "$fixture/transaction"
    [[ ! -e $fixture/project/.lkjscript-edit.pending ]]
}
publish
value
query signature Envelope > "$fixture/signature"
grep -Fq 'T = Payload, U = T' "$fixture/signature"
query references Envelope > "$fixture/references"
grep -Fq '"Payload"' "$fixture/references"
grep -Fq '"Envelope::generic:T"' "$fixture/references"
query outline main > "$fixture/outline"
binding=$(sed -n 's/^"\([^"]*::binding:result:0\)".*/\1/p' "$fixture/outline" | head -n 1)
[[ -n $binding ]]
query type "$binding" > "$fixture/type"
grep -Fq 'Envelope<Payload, Payload>' "$fixture/type"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Payload","name":"PayloadRenamed"}]}
REQUEST
edit
value
publish
query references Envelope > "$fixture/references"
grep -Fq '"PayloadRenamed"' "$fixture/references"
! grep -Fq '"Payload"' "$fixture/references"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Envelope::generic:T","name":"Element"}]}
REQUEST
edit
value
grep -Fq 'U = Element' "$fixture/project/main.lkj"
grep -Fq 'first: Element' "$fixture/project/main.lkj"
grep -Fq 'type Alias<T =' "$fixture/project/main.lkj"
cp "$fixture/project/main.lkj" "$fixture/before"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"Envelope::generic:Element","name":"U"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/capture" 2> "$fixture/error"; then
    printf 'default binder rename captured another parameter\n' >&2
    exit 1
fi
grep -q 'CAPTURE' "$fixture/error"
cmp "$fixture/before" "$fixture/project/main.lkj"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-item","id":"Alias","source":"type Alias<T = bool> = Envelope<T>;"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rejected" 2> "$fixture/error"; then
    printf 'default edit accepted a different nominal application\n' >&2
    exit 1
fi
grep -q 'E_TYPE' "$fixture/error"
cmp "$fixture/before" "$fixture/project/main.lkj"
value
printf 'ok type defaults: canonical applications, authored defaults and references, nominal/binder rename, rejected capture/default edit on both engines\n'
