#!/usr/bin/env bash
# Anonymous parameter meaning, bound references and checked source publication.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
mkdir -p target
fixture=$(mktemp -d "$PWD/target/argument-impl-trait.XXXXXX")
cleanup() {
    status=$?
    if ((status)); then
        cat "$fixture/source/main.lkj" >&2
        for file in "$fixture"/*type "$fixture/rename" "$fixture/associated-rename" "$fixture/edit"; do
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
trait Value { type Item; fn get(&self) -> Self::Item; }
struct Number { value: i64 }
impl Value for Number { type Item = i64; fn get(&self) -> i64 { self.value } }
fn use_value(input: impl Value<Item = i64>) -> i64 { let held = input; held.get() }
fn main() -> i64 { use_value(Number { value: 40 }) }
SOURCE
common=(main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/store" --store-key "$fixture/key")
publish() {
    "$compiler" build "${common[@]}" --index -o "$fixture/program" > "$fixture/build"
    snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/build")
    [[ ${#snapshot} == 64 ]]
}
query() {
    "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$1" --id "$2" --limit 128 --bytes 100000
}
publish
query type use_value > "$fixture/type"
grep -Fq 'fn(impl Value<Item = i64>) -> i64' "$fixture/type"
query outline use_value > "$fixture/outline"
if grep -Fq '$impl:' "$fixture/outline"; then
    echo 'anonymous parameter leaked as an editable declaration' >&2; exit 1
fi
held=$(sed -n 's/^"\([^"]*::binding:held:0\)".*/\1/p' "$fixture/outline" | head -n 1)
[[ -n "$held" ]]
query type "$held" > "$fixture/held-type"
grep -Eq 'fact=inferred-type.*value="impl Value<Item = i64>"' "$fixture/held-type"
query references use_value > "$fixture/references"
grep -Fq 'Value' "$fixture/references"
grep -Fq 'associated-type-binding' "$fixture/references"
printf '%s\n' '{"version":1,"operations":[{"op":"rename","id":"Value","name":"Readable"}]}' > "$fixture/request"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rename"
grep -Eq 'input: impl (crate::)?Readable' "$fixture/source/main.lkj"
printf '%s\n' '{"version":1,"operations":[{"op":"rename","id":"Readable::type:Item","name":"Output"}]}' > "$fixture/request"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/associated-rename"
grep -Eq 'impl (crate::)?Readable<Output = i64>' "$fixture/source/main.lkj"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"use_value","source":"{ input.get() + 2i64 }"}]}' > "$fixture/request"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/edit"
for engine in native interpreter; do
    [[ $("$compiler" run "$fixture/source/main.lkj" --no-library --no-store --engine "$engine") == 42 ]]
done
publish
query type use_value > "$fixture/changed-type"
grep -Fq 'fn(impl Readable<Output = i64>) -> i64' "$fixture/changed-type"
test ! -e "$fixture/source/.lkjscript-edit.pending"
printf 'ok argument impl Trait: source meaning, trait and associated-bound rename, body publication and both engines\n'
