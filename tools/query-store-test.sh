#!/usr/bin/env bash
# The historical reader cannot acquire compiler-proof APIs or writer authority.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
fixture=$(mktemp -d target/compiler-query-store-test.XXXXXX)
fixture="$PWD/$fixture"
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
mkdir "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
"$compiler" build src/query_store_cases.lkj --no-store --library-dir library \
    --runtime-source library/runtime.lkj -o "$fixture/producer-a"
cp "$fixture/producer-a" "$fixture/reader-b"
printf 'different query executable\n' >> "$fixture/reader-b"
fail() { printf 'query-store: %s\n' "$*" >&2; exit 1; }
value() {
    awk -v key="$1" '{ for (i=1;i<=NF;i++) if (index($i,key "=")==1) print substr($i,length(key)+2) }' "$2"
}
accounted() {
    local expected=$1
    local actual
    actual=$(find "$fixture/store" -maxdepth 1 -type f -printf '%s\n' | awk '{ n += $1 } END { print n+0 }')
    [[ "$actual" == "$expected" ]] || fail "accounted $expected bytes, files contain $actual"
}
restore() { rm -rf "$fixture/store"; cp -a "$fixture/saved-store" "$fixture/store"; }
refuse() {
    if "$@" > "$fixture/refused.out" 2> "$fixture/refused.err"; then fail 'unexpected acceptance'; fi
    [[ ! -s "$fixture/refused.out" ]] || fail 'refusal wrote stdout'
    [[ -s "$fixture/refused.err" ]] || fail 'refusal omitted diagnostic'
}
"$fixture/producer-a" publish "$fixture/store" "$fixture/key" first > "$fixture/first"
identity=$(value id "$fixture/first")
selector=$(value selector "$fixture/first")
root=$(value root "$fixture/first")
producer=$(value producer "$fixture/first")
[[ "${#identity}" == 64 && "${#producer}" == 64 ]] || fail 'wrong identity encoding'
[[ "$producer" == "$(sha256sum "$fixture/producer-a" | cut -d ' ' -f1)" ]] || fail 'producer is not the raw executable digest'
[[ "$producer" != "$(sha256sum "$fixture/reader-b" | cut -d ' ' -f1)" ]] || fail 'reader bytes did not change'
accounted "$(value bytes "$fixture/first")"
common=("$fixture/store" "$fixture/key" "$identity" "$selector" "$root" "$producer")
"$fixture/producer-a" current "${common[@]}" > "$fixture/current"
"$fixture/reader-b" read "${common[@]}" > "$fixture/read"
refuse "$fixture/reader-b" current "${common[@]}"
grep -q 'producer mismatch' "$fixture/refused.err" || fail 'foreign current store did not retain its producer'
# Reopening the v2 budget preserves existing roots, locators and proof entries.
"$fixture/producer-a" touch "${common[@]}" > "$fixture/touch"
accounted "$(value bytes "$fixture/touch")"
cp -a "$fixture/store" "$fixture/saved-store"
# The pinned reader removes only its fixed root and locator. It never rewrites
# accounting or erases a newer latest pointer / unrelated admission record.
"$fixture/producer-a" publish "$fixture/store" "$fixture/key" second > "$fixture/second"
next_identity=$(value id "$fixture/second")
next_root=$(value root "$fixture/second")
[[ "$identity" != "$next_identity" ]] || fail 'fixture snapshots coincide'
latest=$(find "$fixture/store" -maxdepth 1 -type f -name 'semantic-latest-*' -print -quit)
proof=$(find "$fixture/store" -maxdepth 1 -type f -name 'check-*' -print -quit)
cp "$latest" "$fixture/latest-before"
cp "$proof" "$fixture/proof-before"
cp "$fixture/store/.budget-index" "$fixture/budget-before"
"$fixture/reader-b" invalidate "${common[@]}" > "$fixture/invalidate"
[[ ! -e "$fixture/store/semantic-locator-$identity" ]] || fail 'old locator survived invalidation'
[[ -z "$(find "$fixture/store" -maxdepth 1 -name "semantic-root-*-$identity" -print -quit)" ]] || fail 'old root survived invalidation'
cmp "$latest" "$fixture/latest-before"
cmp "$proof" "$fixture/proof-before"
cmp "$fixture/store/.budget-index" "$fixture/budget-before"
next_common=("$fixture/store" "$fixture/key" "$next_identity" "$selector" "$next_root" "$producer")
"$fixture/reader-b" read "${next_common[@]}" > "$fixture/newer"
# The ordinary writer compares latest under its lock and removes only a match.
"$fixture/producer-a" current-invalidate "${common[@]}" > "$fixture/current-old"
cmp "$latest" "$fixture/latest-before"
"$fixture/producer-a" current-invalidate "${next_common[@]}" > "$fixture/current-new"
[[ ! -e "$latest" ]] || fail 'matching current latest survived invalidation'
cmp "$proof" "$fixture/proof-before"
# A low-capacity new producer evicts the previous producer's locator along with
# its ordinary records; there is no uncharged or separate locator generation.
restore
"$fixture/reader-b" pressure "$fixture/store" "$fixture/key" > "$fixture/pressure"
[[ ! -e "$fixture/store/semantic-locator-$identity" ]] || fail 'locator escaped producer-generation eviction'
[[ -z "$(find "$fixture/store" -maxdepth 1 -name 'semantic-*' -print -quit)" ]] || fail 'old semantic generation survived eviction'
pressure=$(value bytes "$fixture/pressure")
[[ "$pressure" -le 2048 ]] || fail 'budget exceeded capacity'
accounted "$pressure"
# Even correctly authenticated oversized payloads cannot authorize metadata/page
# allocations beyond the fixed snapshot/page representation.
restore
"$fixture/producer-a" oversized-root "${common[@]}" > "$fixture/root-bound"
restore
"$fixture/producer-a" oversized-page "${common[@]}" > "$fixture/page-bound"
# There is no public arbitrary-namespace read/write/record conversion on the
# restricted reader, even in an otherwise well-typed consumer.
mkdir "$fixture/source"
cp src/query_store_cases.lkj src/diagnostic.lkj src/ir.lkj src/ir_binary.lkj src/reuse.lkj src/store.lkj "$fixture/source/"
cat >> "$fixture/source/query_store_cases.lkj" <<'SOURCE'
fn cannot_read_proofs(reader: &mut store::QueryReader) {
    let _ = reader.get("check", "untrusted");
}
fn cannot_write_proofs(reader: &mut store::QueryReader) {
    reader.put("check", "untrusted", b"arbitrary");
}
fn cannot_obtain_records(reader: &mut store::QueryReader) {
    let keys: Vec<String> = Vec::new();
    let _ = reader.records("check", "untrusted", &keys);
}
SOURCE
refuse "$compiler" check query_store_cases.lkj --source-dir "$fixture/source" --no-store --library-dir "$PWD/library"
for method in get put records; do
    grep -Fq "E_UNSUPPORTED method $method on store::QueryReader has 0 applicable implementations" "$fixture/refused.err" || {
        cat "$fixture/refused.err" >&2
        fail "reader unexpectedly exposes $method"
    }
done
printf 'ok query store: pinned producer, generic access, cursor domains, bounded payloads, writer isolation and accounted generation eviction\n'
