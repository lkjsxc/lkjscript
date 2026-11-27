#!/usr/bin/env bash
# Opaque source meaning survives body edits that change the hidden ABI.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
mkdir -p target
fixture=$(mktemp -d "$PWD/target/return-impl-trait.XXXXXX")
cleanup() {
    status=$?
    if ((status)); then
        cat "$fixture/source/main.lkj" >&2
        for file in "$fixture"/*type "$fixture"/rename "$fixture"/associated-rename "$fixture"/edit "$fixture"/*.log; do
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
struct Small { value: i64 }
struct Large { left: i64, right: i64 }
impl Value for Small { type Item = i64; fn get(&self) -> i64 { self.value } }
impl Value for Large { type Item = i64; fn get(&self) -> i64 { self.left + self.right } }
fn make() -> impl Value<Item = i64> { Small { value: 40i64 } }
fn main() -> i64 { let held = make(); held.get() }
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
    local phase=$1 bound=$2
    query type make > "$fixture/$phase-type"
    grep -Fq "fn() -> impl $bound" "$fixture/$phase-type"
    query outline main > "$fixture/outline"
    held=$(sed -n 's/^"\([^"]*::binding:held:0\)".*/\1/p' "$fixture/outline" | head -n 1)
    [[ -n "$held" ]]
    query type "$held" > "$fixture/$phase-held-type"
    grep -Eq "fact=inferred-type.*value=\"impl $bound\"" "$fixture/$phase-held-type"
    if grep -Eq 'Small|Large|\$impl:|\$rigid:' "$fixture/$phase-held-type"; then
        echo 'hidden layout escaped into the source type query' >&2; exit 1
    fi
}
execute() {
    for engine in native interpreter; do
        [[ $("$compiler" run "${common[@]}" --engine "$engine") == "$1" ]]
    done
}
publish cold
first_snapshot=$snapshot
meaning cold 'Value<Item = i64>'
execute 40
publish warm
[[ $snapshot == "$first_snapshot" ]]
[[ $(stat warm build_reused) == 1 ]]
printf '%s\n' '{"version":1,"operations":[{"op":"rename","id":"Value","name":"Readable"}]}' > "$fixture/request"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rename"
grep -Eq -- '-> impl (crate::)?Readable' "$fixture/source/main.lkj"
printf '%s\n' '{"version":1,"operations":[{"op":"rename","id":"Readable::type:Item","name":"Output"}]}' > "$fixture/request"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/associated-rename"
grep -Eq -- '-> impl (crate::)?Readable<Output = i64>' "$fixture/source/main.lkj"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"make","source":"{ Large { left: 20i64, right: 22i64 } }"}]}' > "$fixture/request"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/edit"
publish wide
[[ $snapshot != "$first_snapshot" ]]
[[ $(stat wide front_reused) == 0 ]]
meaning wide 'Readable<Output = i64>'
execute 42
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"make","source":"{ Small { value: 42i64 } }"}]}' > "$fixture/request"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/edit"
publish narrow
[[ $(stat narrow front_reused) == 0 ]]
meaning narrow 'Readable<Output = i64>'
execute 42
test ! -e "$fixture/source/.lkjscript-edit.pending"
# Later callable use supplies the producer's T through its advertised contract.
# Rename and body publication must rebuild those call applications from source.
cat > "$fixture/source/main.lkj" <<'SOURCE'
fn make<T>(offset: i64) -> impl Fn(T) -> (T, i64) {
    move |value: T| (value, offset)
}
fn main() -> i64 {
    let callable = make(40i64);
    let (flag, value) = callable(true);
    if flag { value + 2i64 } else { 0i64 }
}
SOURCE
publish inferred
query type make > "$fixture/inferred-type"
grep -Fq 'fn(i64) -> impl Fn<(T,), (T, i64)>' "$fixture/inferred-type"
execute 42
printf '%s\n' '{"version":1,"operations":[{"op":"rename","id":"make","name":"factory"}]}' > "$fixture/request"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/inferred-rename.log"
execute 42
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"factory","source":"{ move |value: T| (value, offset + 1i64) }"}]}' > "$fixture/request"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/inferred-edit.log"
publish inferred_changed
execute 43
cp "$fixture/source/main.lkj" "$fixture/before-refusal"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"factory","source":"{ move |value: T| { drop(value); (true, offset) } }"}]}' > "$fixture/request"
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/inferred-refusal.log" 2>&1; then
    echo 'opaque caller specialization admitted an invalid generic body edit' >&2; exit 1
fi
grep -Fq 'E_TYPE' "$fixture/inferred-refusal.log"
cmp "$fixture/before-refusal" "$fixture/source/main.lkj"
execute 43
test ! -e "$fixture/source/.lkjscript-edit.pending"
printf 'ok return impl Trait: opaque queries, inferred calls, renames, body edits and rollback, warm cache and both engines\n'
