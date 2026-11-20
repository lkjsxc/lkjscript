#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
mkdir -p target
fixture=$(mktemp -d "$PWD/target/opaque-callable-return.XXXXXX")
trap 'rm -rf "$fixture"' EXIT
trap 'exit 130' HUP INT TERM
mkdir "$fixture/source" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/source/main.lkj" <<'LKJ'
trait Convert<T> { fn convert(value: T) -> Self; }
impl<T> Convert<T> for T { fn convert(value: T) -> T { value } }
fn choose<L, R: Convert<L>>(flag: bool) -> impl Fn(L, R) -> R {
    |_: L, right: R| right
}
pub fn probe() -> i64 {
    let left = 1i64;
    let right = 42i64;
    let held = choose::<&i64, &i64>(true);
    *held(&left, &right)
}
LKJ
common=(main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/store" --store-key "$fixture/key")
publish() {
    "$compiler" build "${common[@]}" --index --stats -o "$fixture/program" > "$fixture/$1.log" 2>&1 || {
        cat "$fixture/$1.log" >&2
        return 1
    }
    snapshot=$(awk '$1 == "LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/$1.log")
    [[ ${#snapshot} == 64 ]]
}
query() {
    "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$1" --id "$2" --limit 128 --bytes 100000
}
meaning() {
    query type choose > "$fixture/producer"
    query outline probe > "$fixture/outline"
    held=$(sed -n 's/^"\([^"]*::binding:held:0\)".*/\1/p' "$fixture/outline")
    [[ -n $held ]]
    query type "$held" > "$fixture/held"
    grep -Fq 'impl Fn' "$fixture/producer"
    grep -Eq 'fact=inferred-type.*value="impl Fn' "$fixture/held"
    if grep -Eq '\$closure|\$callable|\$opaque' "$fixture/held"; then
        return 1
    fi
}
publish cold
original_snapshot=$snapshot
meaning
cp "$fixture/producer" "$fixture/original-producer"
cp "$fixture/held" "$fixture/original-held"
publish warm
[[ $snapshot == "$original_snapshot" ]]
grep -Eq '"build_reused":[[:space:]]*1([,}])' "$fixture/warm.log"
cat > "$fixture/request" <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"choose","source":"{ move |_: L, right: R| { if flag { right } else { right } } }"}]}
JSON
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/edit"
publish edited
[[ $snapshot != "$original_snapshot" ]]
grep -Eq '"front_reused":[[:space:]]*0([,}])' "$fixture/edited.log"
meaning
# Query envelopes include the new snapshot. Compare the source type facts.
grep 'fact=declared-type' "$fixture/original-producer" > "$fixture/original-producer-facts"
grep 'fact=declared-type' "$fixture/producer" > "$fixture/producer-facts"
grep 'fact=inferred-type' "$fixture/original-held" > "$fixture/original-held-facts"
grep 'fact=inferred-type' "$fixture/held" > "$fixture/held-facts"
diff -u "$fixture/original-producer-facts" "$fixture/producer-facts"
diff -u "$fixture/original-held-facts" "$fixture/held-facts"
cp "$fixture/source/main.lkj" "$fixture/accepted.lkj"
cat > "$fixture/request" <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"choose","source":"{ |left: L, _: R| R::convert(left) }"}]}
JSON
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/refusal" 2>&1; then
    echo 'checked edit published a body outside its advertised origins' >&2
    exit 1
fi
grep -E 'main.lkj:[0-9]+:[0-9]+: E_UNSUPPORTED hidden callable returns a view outside its advertised argument origins' "$fixture/refusal"
cmp "$fixture/accepted.lkj" "$fixture/source/main.lkj"
test ! -e "$fixture/source/.lkjscript-edit.pending"
for engine in native interpreter; do
    [[ $("$compiler" run "${common[@]}" --engine "$engine") == 42 ]]
done
"$compiler" ir "${common[@]}" -o "$fixture/program.lkjir"
if grep -Eq '\$callable:|\$opaque|\$rigid:|\$capture:' "$fixture/program.lkjir"; then
    echo 'source verification declarations escaped into concrete IR' >&2
    exit 1
fi
"$compiler" check "$fixture/program.lkjir"
for engine in native interpreter; do
    [[ $("$compiler" run "$fixture/program.lkjir" --engine "$engine") == 42 ]]
done
grep -Fxq 'return-leaf 0 exact 2' "$fixture/program.lkjir"
sed 's/^return-leaf 0 exact 2$/return-leaf 0 exact 1/' "$fixture/program.lkjir" > "$fixture/wrong-origin.lkjir"
if "$compiler" check "$fixture/wrong-origin.lkjir" > "$fixture/ir-refusal" 2>&1; then
    echo 'independent IR checker accepted the forged origin' >&2
    exit 1
fi
grep -Fq 'E_BORROW' "$fixture/ir-refusal"
printf 'ok opaque callable origins: checked edits, warm image, both engines and forged IR refusal\n'
