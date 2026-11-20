#!/usr/bin/env bash
# Namespace identity, exact selection and checked source publication.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
baseline=${2:-}
mkdir -p target
fixture=$(mktemp -d "$PWD/target/catalog-namespace.XXXXXX")
cleanup() {
    local status=$?
    if ((status)); then
        cat "$fixture/error" "$fixture/plan" 2>/dev/null >&2 || true
        printf 'namespace fixtures retained: %s\n' "$fixture" >&2
    else rm -rf -- "$fixture"; fi
}
trap cleanup EXIT
umask 077
head -c 32 /dev/urandom > "$fixture/key"
mkdir "$fixture/store"
fail() { printf 'namespace: %s\n' "$*" >&2; exit 1; }
setup() {
    project="$fixture/$1"
    mkdir -p "$project"
    common=(main.lkj --source-dir "$project" --no-library)
}
index() {
    "$compiler" build "${common[@]}" --index --store "$fixture/store" --store-key "$fixture/key" -o "$fixture/image" > "$fixture/index" 2> "$fixture/error"
    pin=$(awk '$1=="LKJSCRIPT_SNAPSHOT" {print $2}' "$fixture/index")
    [[ ${#pin} == 64 ]] || fail 'no authenticated pin'
}
query() {
    "$compiler" query "${common[@]}" --store "$fixture/store" --store-key "$fixture/key" --snapshot "$pin" --kind "$1" --id "$2" --limit "${3:-256}" --bytes 131072
}
query_refuse() {
    local code=$1 kind=$2 id=$3 status=0
    query "$kind" "$id" > "$fixture/plan" 2> "$fixture/error" || status=$?
    [[ $status == 1 && ! -s "$fixture/plan" ]] || fail "query accepted $id"
    grep -Fq "$code" "$fixture/error" || fail "query missed $code"
}
request() { printf '{"version":1,"operations":[%s]}\n' "$1" > "$fixture/request.json"; }
edit() {
    "$compiler" edit "${common[@]}" --no-store --store-key "$fixture/key" --request "$fixture/request.json" "$@" > "$fixture/plan" 2> "$fixture/error"
}
snapshot() {
    (cd "$project"; find . -type f \( -name '*.lkj' -o -name '.lkjscript-edit.generation' \) -print0 | sort -z | xargs -0 sha256sum) > "$1"
}
refuse() {
    local code=$1 status=0
    snapshot "$fixture/before"
    edit || status=$?
    [[ $status == 1 && ! -s "$fixture/plan" ]] || fail "edit accepted $code"
    grep -Fq "$code" "$fixture/error" || fail "edit missed $code"
    snapshot "$fixture/after"
    cmp "$fixture/before" "$fixture/after"
    [[ ! -e "$project/.lkjscript-edit.pending" ]] || fail 'refusal left intent'
}
apply() {
    edit
    grep -q 'mode=apply' "$fixture/plan" || fail 'missing apply report'
    [[ ! -e "$project/.lkjscript-edit.pending" ]] || fail 'apply left intent'
}
value() {
    local engine actual
    for engine in native interpreter; do
        actual=$("$compiler" run "${common[@]}" --no-store --engine "$engine")
        [[ $actual == "$1" ]] || fail "$engine returned $actual instead of $1"
    done
}
metadata_digest() {
    query meta "$1" > "$fixture/meta"
    sed -n 's/.* item-sha256=\([0-9a-f]*\).*/\1/p' "$fixture/meta"
}
write_pair() {
    cat > "$project/main.lkj" <<'SOURCE'
mod shared { pub fn value() -> i64 { 42i64 } }
fn shared() -> i64 { shared::value() }
pub fn probe() -> i64 { shared() }
SOURCE
}

setup pair
write_pair
value 42
index
query outline crate > "$fixture/outline"
grep -Fq '"type@shared"' "$fixture/outline"
grep -Fq '"value@shared"' "$fixture/outline"
query outline type@shared > "$fixture/module-outline"
grep -Fq '"value@shared::value"' "$fixture/module-outline"
query item shared::value > "$fixture/bare"
grep -Fq 'selected="value@shared::value"' "$fixture/bare"
query_refuse E_QUERY_AMBIGUOUS item shared
query_refuse E_QUERY_AMBIGUOUS layout shared
for id in shared type@probe fn@probe; do
    status=0
    "$compiler" compare "${common[@]}" --against main.lkj --against-source-dir "$project" --id "$id" > "$fixture/plan" 2> "$fixture/error" || status=$?
    [[ $status == 1 && ! -s "$fixture/plan" ]] || fail "compare accepted $id"
    case "$id" in
        shared) grep -q E_COMPARE_AMBIGUOUS "$fixture/error";;
        type@probe) grep -q E_COMPARE_IDENTITY "$fixture/error";;
        *) grep -q E_COMPARE_SELECTOR "$fixture/error";;
    esac
done
for id in type@ @x type@type@x fn@x macro@x; do query_refuse E_QUERY_SELECTOR item "$id"; done
query_refuse E_QUERY_ID item type@probe
query item probe 1 > "$fixture/page"
cursor=$(sed -n 's/.*cursor=\([^ ]*\).*/\1/p' "$fixture/page")
[[ -n $cursor && $cursor != none ]] || fail 'missing cursor'
"$compiler" query "${common[@]}" --store "$fixture/store" --store-key "$fixture/key" --snapshot "$pin" --kind item --id value@probe --limit 1 --bytes 131072 --cursor "$cursor" > "$fixture/continued"
status=0
"$compiler" query "${common[@]}" --store "$fixture/store" --store-key "$fixture/key" --snapshot "$pin" --kind item --id type@shared --limit 1 --bytes 131072 --cursor "$cursor" > "$fixture/plan" 2> "$fixture/error" || status=$?
[[ $status == 1 && ! -s "$fixture/plan" ]]
grep -q E_QUERY_CURSOR "$fixture/error"
expected=$(metadata_digest value@shared)
request "{\"op\":\"replace-body\",\"id\":\"shared\",\"source\":\"{ 44i64 }\",\"expect\":\"$expected\"}"
refuse E_EDIT_AMBIGUOUS
request "{\"op\":\"replace-body\",\"id\":\"type@shared\",\"source\":\"{ 44i64 }\",\"expect\":\"$expected\"}"
refuse E_EDIT_STALE
for id in type@ @x type@type@x fn@x macro@x; do
    request "{\"op\":\"replace-body\",\"id\":\"$id\",\"source\":\"{ 44i64 }\"}"
    refuse E_EDIT_SELECTOR
done
request '{"op":"replace-body","id":"type@probe","source":"{ 44i64 }"}'
refuse E_EDIT_IDENTITY
request '{"op":"move","id":"value@probe","parent":"shared"}'
refuse E_EDIT_AMBIGUOUS
cp "$project/main.lkj" "$fixture/pair-before"
request "{\"op\":\"replace-body\",\"id\":\"value@shared\",\"source\":\"{ shared::value() + 2i64 }\",\"expect\":\"$expected\"}"
edit --dry-run
cmp "$fixture/pair-before" "$project/main.lkj"
apply
perl -e 'local $/;my $old=<>; $old =~ s/fn shared\(\) -> i64 \{ shared::value\(\) \}/fn shared() -> i64 { shared::value() + 2i64 }/ or die; print $old' "$fixture/pair-before" > "$fixture/pair-desired"
cmp "$fixture/pair-desired" "$project/main.lkj"
value 44
refuse E_EDIT_STALE
request '{"op":"replace-body","id":"probe","source":"{ shared() + 1i64 }"}'
apply
grep -Fq 'selected="value@probe"' "$fixture/plan"
value 45

# Same-scope type/value renames and moves retain the sibling and checked meaning.
for which in function module module_value_name; do
    setup "rename-$which"
    write_pair
    index
    if [[ $which == function ]]; then
        query text type@shared > "$fixture/sibling-before"
        request '{"op":"rename","id":"value@shared","name":"changed"}'
        apply
        index
        query text type@shared > "$fixture/sibling-after"
    else
        query text value@probe > "$fixture/sibling-before"
        name=changed
        [[ $which != module_value_name ]] || name=probe
        request "{\"op\":\"rename\",\"id\":\"type@shared\",\"name\":\"$name\"}"
        apply
        index
        query text value@probe > "$fixture/sibling-after"
    fi
    sed -n '/level=source-claim/p' "$fixture/sibling-before" > "$fixture/old-facts"
    sed -n '/level=source-claim/p' "$fixture/sibling-after" > "$fixture/new-facts"
    cmp "$fixture/old-facts" "$fixture/new-facts"
    value 42
done
for which in value type; do
    setup "move-$which"
    write_pair
    sed -i 's/^mod shared/pub mod shared/; s/^fn shared/pub fn shared/' "$project/main.lkj"
    # Preserve function order: the existing checked-meaning proof is order-sensitive.
    sed -i '1a mod destination {}' "$project/main.lkj"
    request "{\"op\":\"move\",\"id\":\"$which@shared\",\"parent\":\"destination\"}"
    apply
    value 42
    index
    if [[ $which == value ]]; then
        query item type@shared > "$fixture/preserved"
        query item value@destination::shared > "$fixture/moved"
    else
        query item value@shared > "$fixture/preserved"
        query item type@destination::shared > "$fixture/moved"
    fi
done

setup shared_import
cat > "$project/main.lkj" <<'SOURCE'
mod names {
    pub mod shared { pub fn value() -> i64 { 21i64 } }
    pub fn shared() -> i64 { shared::value() }
}
use names::shared;
pub fn probe() -> i64 { shared() + shared::value() }
SOURCE
value 42
for which in type value; do
    request "{\"op\":\"rename\",\"id\":\"$which@names::shared\",\"name\":\"changed\"}"
    refuse E_EDIT_REFERENCES
done

setup generic_pair
cat > "$project/main.lkj" <<'SOURCE'
struct Pair<T> { a: T }
fn Pair<T>(value: T) -> T { value }
impl<T> Pair<T> { fn take(self) -> T { self.a } }
pub fn probe() -> i64 { Pair { a: Pair(42i64) }.take() }
SOURCE
value 42
index
query outline type@Pair > "$fixture/type-pair"
query outline value@Pair > "$fixture/value-pair"
grep -Fq '"type@Pair::generic:T"' "$fixture/type-pair"
grep -Fq '"type@Pair::field:a"' "$fixture/type-pair"
grep -Fq '"value@Pair::generic:T"' "$fixture/value-pair"
query references probe > "$fixture/pair-references"
grep -Fq '"type@Pair"' "$fixture/pair-references"
grep -Fq '"value@Pair"' "$fixture/pair-references"
query layout value@Pair > "$fixture/function-layout"
! grep -q 'fact=layout' "$fixture/function-layout"
query layout type@Pair > "$fixture/type-layout"
grep -q 'fact=layout' "$fixture/type-layout"
query impls type@Pair > "$fixture/type-impls"
grep -q 'fact=impls' "$fixture/type-impls"
query impls value@Pair > "$fixture/value-impls"
! grep -q 'fact=impls' "$fixture/value-impls"
request '{"op":"replace-body","id":"value@Pair","source":"{ let result = value; result }"}'
apply
value 42
# The independently checked candidate must reject a malformed replacement too.
request '{"op":"replace-body","id":"value@Pair","source":"{ let next = value; next }"},{"op":"replace-item","id":"type@Pair","source":"struct Pair<T> { a: T, extra: i64 }"}'
refuse E_EDIT_CHECK

# An enum and its same-spelled variant remain distinct import targets.
setup enum_import
cat > "$project/main.lkj" <<'SOURCE'
enum E { E }
use crate::E as Alias;
use crate::E::E as Variant;
pub fn probe() -> i64 { match Variant { Alias::E => 42i64 } }
SOURCE
index
query references crate > "$fixture/enum-imports"
grep -Fq '"type@E"' "$fixture/enum-imports"
grep -Fq '"type@E::variant:E"' "$fixture/enum-imports"
request '{"op":"rename","id":"type@E","name":"Choice"}'
apply
request '{"op":"rename","id":"type@Choice::variant:E","name":"Ready"}'
apply
value 42

# Structural intrinsic types keep their own evidence beside source names.
for declaration in 'mod Vec {}'; do
    setup "structural-${declaration%% *}"
    common=(main.lkj --source-dir "$project" --library-dir "$PWD/library")
    printf '%s\n' "$declaration" > "$project/main.lkj"
    cat >> "$project/main.lkj" <<'SOURCE'
trait Count { fn count(&self) -> usize; }
impl Count for std::vec::Vec<i64> { fn count(&self) -> usize { self.len() } }
pub fn probe() -> i64 {
    let mut values: std::vec::Vec<i64> = std::vec::Vec::new();
    values.push(42i64);
    values[0] + values.count() as i64 - 1i64
}
SOURCE
    index
    query layout type@Vec > "$fixture/source-layout"
    ! grep -q 'fact=layout' "$fixture/source-layout"
    query impls type@Vec > "$fixture/source-impls"
    ! grep -q 'fact=impls' "$fixture/source-impls"
    query layout 'Vec<i64>' > "$fixture/intrinsic-layout"
    grep -q 'fact=layout' "$fixture/intrinsic-layout"
    query impls 'Vec<i64>' > "$fixture/intrinsic-impls"
    grep -q 'fact=impls' "$fixture/intrinsic-impls"
    value 42
done

setup alias_constant
cat > "$project/main.lkj" <<'SOURCE'
type Shared = i64;
const Shared: Shared = 41i64;
pub fn probe() -> i64 { let value: Shared = Shared; value + 1i64 }
SOURCE
value 42
index
query_refuse E_QUERY_AMBIGUOUS item Shared
query references probe > "$fixture/alias-references"
grep -Fq '"type@Shared"' "$fixture/alias-references"
grep -Fq '"value@Shared"' "$fixture/alias-references"
request '{"op":"replace-item","id":"value@Shared","source":"const Shared: Shared = 42i64;"}'
apply
value 43

setup constructor
cat > "$project/main.lkj" <<'SOURCE'
struct S(i64);
pub fn probe() -> i64 { S(42i64).0 }
SOURCE
index
query item S > "$fixture/constructor-facts"
grep -Fq 'selected="type@S"' "$fixture/constructor-facts"
query_refuse E_QUERY_ID item value@S
grep -Fq '"type@S"' "$fixture/error"
request '{"op":"replace-item","id":"value@S","source":"struct S(i64);"}'
refuse E_EDIT_IDENTITY
request '{"op":"add-item","id":"crate","source":"fn S() -> i64 { 0i64 }"}'
refuse E_NAME
for source in 'fn probe() -> i64 { 0i64 }' 'const probe: i64 = 0i64;' 'type S = i64;' 'mod S {}'; do
    request "{\"op\":\"add-item\",\"id\":\"crate\",\"source\":\"$source\"}"
    refuse E_NAME
done

setup replay
cat > "$project/main.lkj" <<'SOURCE'
mod shared { pub fn value() -> i64 { 42i64 } }
pub fn probe() -> i64 { shared::value() }
SOURCE
desired="$fixture/replay-desired"
mkdir "$desired"
cat > "$desired/main.lkj" <<'SOURCE'
mod shared { pub fn value() -> i64 { 42i64 } }

fn shared() -> i64 { shared::value() }
pub fn probe() -> i64 { shared() }
SOURCE
"$compiler" edit "${common[@]}" --no-store --store-key "$fixture/key" --against main.lkj --against-source-dir "$desired" > "$fixture/plan" 2> "$fixture/error"
cmp "$project/main.lkj" "$desired/main.lkj"
grep -q 'verified=true' "$fixture/plan"
value 42
"$compiler" compare "${common[@]}" --against main.lkj --against-source-dir "$desired" > "$fixture/compare"
grep -q 'text identical, checked IR identical' "$fixture/compare"

setup projection
cat > "$project/main.lkj" <<'SOURCE'
fn shared() -> i64 { 42i64 }
pub fn probe() -> i64 { shared() }
SOURCE
request '{"op":"add-item","id":"crate","source":"mod shared;","projection":{"path":"shared.lkj","source":"pub fn value() -> i64 { 7i64 }"}}'
apply
[[ -f "$project/shared.lkj" ]]
value 42
index
query item type@shared > "$fixture/projected-module"
query item value@shared > "$fixture/projected-function"

if [[ -n $baseline ]]; then
    baseline=$(realpath "$baseline")
    setup historical
    printf 'pub fn probe() -> i64 { 42i64 }\n' > "$project/main.lkj"
    "$baseline" build "${common[@]}" --index --store "$fixture/store" --store-key "$fixture/key" -o "$fixture/historical-image" > "$fixture/index"
    pin=$(awk '$1=="LKJSCRIPT_SNAPSHOT" {print $2}' "$fixture/index")
    "$baseline" query "${common[@]}" --store "$fixture/store" --store-key "$fixture/key" --snapshot "$pin" --kind item --id probe > "$fixture/historical-before"
    query item probe > "$fixture/historical-after"
    sed -n '/level=source-claim/p' "$fixture/historical-before" > "$fixture/old-facts"
    sed -n '/level=source-claim/p' "$fixture/historical-after" > "$fixture/new-facts"
    cmp "$fixture/old-facts" "$fixture/new-facts"
    query_refuse E_QUERY_ID item value@probe
fi
printf 'ok namespace catalog: checked collisions, exact selectors, refactors, constructors, replay, projection and both engines\n'
