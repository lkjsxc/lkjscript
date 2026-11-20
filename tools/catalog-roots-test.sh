#!/usr/bin/env bash
# Canonical crate roots, exact selectors and explicit builtin type families.
set -euo pipefail
cd "$(dirname "$0")/.."
repo=$PWD
compiler=$(realpath "${1:-target/lkjscript}")
mkdir -p target
fixture=$(mktemp -d "$PWD/target/catalog-roots.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)); then
        cat "$fixture/error" "$fixture/plan" 2>/dev/null >&2 || true
        printf 'catalog roots fixture retained: %s\n' "$fixture" >&2
    else
        rm -rf -- "$fixture"
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
umask 077
head -c 32 /dev/urandom > "$fixture/key"
mkdir "$fixture/store"
fail() { printf 'catalog-roots: %s\n' "$*" >&2; exit 1; }
setup() {
    project="$fixture/$1"
    library="$project/library"
    mkdir -p "$library" "$project/std"
    cat > "$project/main.lkj" <<'SOURCE'
mod std { pub fn answer() -> i64 { 42i64 } }
pub fn probe() -> i64 { crate::std::answer() }
SOURCE
    printf 'pub fn %s() -> i64 { 7i64 }\n' "${2:-answer}" > "$library/lib.lkj"
    common=(main.lkj --source-dir "$project" --library-dir "$library")
}
source_state() {
    (cd "$project"; find . -type f \( -name '*.lkj' -o -name '.lkjscript-edit.generation' \) -print0 |
        LC_ALL=C sort -z | xargs -0 sha256sum) > "$1"
}
source_count() {
    [[ $(find "$project" -type f -name '*.lkj' | wc -l) == "$1" ]] ||
        fail "unexpected source-file count in $project"
}
value() {
    local engine actual
    for engine in native interpreter; do
        actual=$("$compiler" run "${common[@]}" --no-store --engine "$engine")
        [[ $actual == "$1" ]] || fail "$engine returned $actual instead of $1"
    done
}
index() {
    "$compiler" build "${common[@]}" --index --store "$fixture/store" --store-bytes 268435456 \
        --store-key "$fixture/key" -o "$fixture/image" > "$fixture/index" 2> "$fixture/error" || return $?
    pin=$(awk '$1=="LKJSCRIPT_SNAPSHOT" {print $2}' "$fixture/index")
    [[ ${#pin} == 64 ]] || fail 'missing authenticated snapshot'
}
index_refuse() {
    local code=$1 status=0
    source_state "$fixture/before-state"
    index || status=$?
    [[ $status == 1 ]] || fail "index accepted $code"
    grep -Fq "$code" "$fixture/error" || fail "index missed $code"
    source_state "$fixture/after-state"
    cmp "$fixture/before-state" "$fixture/after-state"
    [[ ! -e "$project/.lkjscript-edit.pending" ]] || fail 'index refusal left intent'
}
query() {
    "$compiler" query "${common[@]}" --store "$fixture/store" --store-key "$fixture/key" \
        --snapshot "$pin" --kind "$1" --id "$2" --limit 256 --bytes 131072
}
query_refuse() {
    local code=$1 status=0
    shift
    query "$@" > "$fixture/plan" 2> "$fixture/error" || status=$?
    [[ $status == 1 && ! -s "$fixture/plan" ]] || fail "query accepted $*"
    grep -Fq "$code" "$fixture/error" || fail "query missed $code"
}
request() { printf '{"version":1,"operations":[%s]}\n' "$1" > "$fixture/request"; }
edit() {
    "$compiler" edit "${common[@]}" --no-store --store-key "$fixture/key" \
        --request "$fixture/request" "$@" > "$fixture/plan" 2> "$fixture/error"
}
apply() {
    edit
    grep -q 'mode=apply' "$fixture/plan" || fail 'missing apply report'
    [[ ! -e "$project/.lkjscript-edit.pending" ]] || fail 'apply left intent'
}
refuse() {
    local code=$1 status=0
    source_state "$fixture/before-state"
    edit || status=$?
    [[ $status == 1 && ! -s "$fixture/plan" ]] || fail "edit accepted $code"
    grep -Fq "$code" "$fixture/error" || fail "edit missed $code"
    source_state "$fixture/after-state"
    cmp "$fixture/before-state" "$fixture/after-state"
    [[ ! -e "$project/.lkjscript-edit.pending" ]] || fail 'refusal left intent'
}
meta() {
    query meta "$1" > "$fixture/meta"
    grep -Fq "parent=\"$2\"" "$fixture/meta" || fail "wrong parent for $1"
    grep -Fq "file=\"$3\"" "$fixture/meta" || fail "wrong source file for $1"
}
root_outline() {
    local parent=$1 included=$2 excluded=$3
    query outline "$parent" > "$fixture/outline"
    grep -Fq "\"$included\"" "$fixture/outline" || fail "$parent lost $included"
    if grep -Fq "\"$excluded\"" "$fixture/outline"; then fail "$parent includes $excluded"; fi
}
trait_method() {
    query outline "$1" > "$fixture/method-outline"
    perl -MJSON::PP - "$fixture/method-outline" <<'PERL'
use strict; use warnings;
open my $input, '<', shift @ARGV or die $!;
my @ids;
while (<$input>) {
    next unless /\tfact=signature\tkind=function /;
    my ($id)=split /\t/;
    push @ids, decode_json($id);
}
die "expected one trait method\n" unless @ids==1;
print $ids[0], "\n";
PERL
}

# Distinct child names must not merge outlines; identical names are legal.
for child in spare answer; do
    setup "children-$child" "$child"
    source_count 2
    "$compiler" check "${common[@]}" --no-store
    value 42
    index
    root_outline type@crate::std value@crate::std::answer "value@std::$child"
    root_outline type@std "value@std::$child" value@crate::std::answer
    meta type@crate::std crate "$project/main.lkj"
    meta value@crate::std::answer type@crate::std "$project/main.lkj"
    meta "value@std::$child" type@std "$library/lib.lkj"
    query outline std > "$fixture/bare-library"
    grep -Fq 'selected="type@std"' "$fixture/bare-library"
    query item crate::std::answer > "$fixture/bare-application"
    grep -Fq 'selected="value@crate::std::answer"' "$fixture/bare-application"
    for selector in '$std' '$std::answer' 'type@$std' 'value@$std::answer'; do
        query_refuse E_QUERY_ID meta "$selector"
    done
    request '{"op":"add-item","id":"$std","source":"pub fn wrong() {}"}'
    refuse E_EDIT_IDENTITY
done

# The parser still admits keyword-named modules; catalog admission must refuse.
setup keyword
cat > "$project/main.lkj" <<'SOURCE'
mod std { pub fn answer() -> i64 { 42i64 } }
mod crate { pub mod std { pub fn answer() -> i64 { 9i64 } } }
pub fn probe() -> i64 { crate::std::answer() + crate::crate::std::answer() }
SOURCE
"$compiler" check "${common[@]}" --no-store
index_refuse E_IDENTITY
grep -Fq 'crate' "$fixture/error"
grep -Fq 'main.lkj' "$fixture/error" || fail 'keyword-module refusal is not located'
request '{"op":"replace-body","id":"probe","source":"{ 0i64 }"}'
refuse E_IDENTITY
source_count 2

# A function named crate is allowed and must not become the unmarked root.
project="$fixture/keyword-function"
mkdir "$project"
common=(main.lkj --source-dir "$project" --no-library)
cat > "$project/main.lkj" <<'SOURCE'
pub(crate) fn crate() -> i64 { 42i64 }
mod destination {}
pub fn probe() -> i64 { crate::crate() }
SOURCE
value 42
index
meta value@crate crate "$project/main.lkj"
request '{"op":"move","id":"value@crate","parent":"type@destination"}'
apply
source_count 1
index
meta value@destination::crate type@destination "$project/main.lkj"
query references value@probe > "$fixture/keyword-function-references"
[[ $(grep -Ec '^"value@destination::crate".*kind=value( |$)' "$fixture/keyword-function-references") == 1 ]]
perl -0777 -e 'my $text=<>; my $count=()=$text=~/\bcrate::destination::crate\(\)/g; die "expected one qualified moved call\n" unless $count==1; die "root marker duplicated\n" if $text =~ /crate::destination::crate::crate/;' "$project/main.lkj"
value 42

# Ordinary additions and anchors use exact semantic parents in each direction.
for side in application library; do
    setup "insert-$side"
    parent=type@crate::std
    prefix=crate::std
    changed="$project/main.lkj"
    untouched="$library/lib.lkj"
    other=value@std::answer
    if [[ $side == library ]]; then
        parent=type@std
        prefix=std
        changed="$library/lib.lkj"
        untouched="$project/main.lkj"
        other=value@crate::std::answer
    fi
    cp "$untouched" "$fixture/untouched"
    cp "$changed" "$fixture/changed"
    request "{\"op\":\"add-item\",\"id\":\"$parent\",\"source\":\"pub fn inserted() -> i64 { 9i64 }\"}"
    apply
    cmp "$fixture/untouched" "$untouched"
    if cmp -s "$fixture/changed" "$changed"; then fail 'addition changed no intended source'; fi
    grep -Fq 'pub fn inserted() -> i64 { 9i64 }' "$changed"
    source_count 2
    index
    meta "value@$prefix::inserted" "$parent" "$changed"
    request "{\"op\":\"add-item\",\"id\":\"$parent\",\"before\":\"value@$prefix::answer\",\"source\":\"pub fn anchored() -> i64 { 8i64 }\"}"
    apply
    cmp "$fixture/untouched" "$untouched"
    request "{\"op\":\"add-item\",\"id\":\"$parent\",\"before\":\"$other\",\"source\":\"pub fn misplaced() -> i64 { 1i64 }\"}"
    refuse E_EDIT_PARENT
    source_count 2
    value 42
done

# The projection's physical parent must agree with its selected semantic root.
for side in application library; do
    for placement in correct crossed; do
        setup "projection-$side-$placement"
        parent=type@crate::std
        prefix=crate::std
        relative=std/child.lkj
        changed="$project/main.lkj"
        untouched="$library/lib.lkj"
        if [[ $side == library ]]; then
            parent=type@std
            prefix=std
            relative=library/child.lkj
            changed="$library/lib.lkj"
            untouched="$project/main.lkj"
        fi
        if [[ $placement == crossed ]]; then
            if [[ $side == application ]]; then relative=library/child.lkj; else relative=std/child.lkj; fi
        fi
        cp "$untouched" "$fixture/untouched"
        request "{\"op\":\"add-item\",\"id\":\"$parent\",\"source\":\"pub mod child;\",\"projection\":{\"path\":\"$relative\",\"source\":\"pub fn child_value() -> i64 { 9i64 }\"}}"
        if [[ $placement == crossed ]]; then
            refuse E_EDIT_PROJECTION
            source_count 2
            [[ ! -e "$project/$relative" ]] || fail 'refused projection created source'
            continue
        fi
        apply
        cmp "$fixture/untouched" "$untouched"
        grep -Fq 'pub mod child;' "$changed"
        printf '%s' 'pub fn child_value() -> i64 { 9i64 }' > "$fixture/projected-expected"
        cmp "$fixture/projected-expected" "$project/$relative"
        source_count 3
        index
        meta "type@$prefix::child" "$parent" "$changed"
        meta "value@$prefix::child::child_value" "type@$prefix::child" "$project/$relative"
        value 42
    done
done

# Replay pairs the two roots independently, including matching added names.
for side in application library both; do
    setup "replay-$side"
    desired="$fixture/replay-$side-desired"
    mkdir -p "$desired/library"
    cp "$project/main.lkj" "$desired/main.lkj"
    cp "$library/lib.lkj" "$desired/library/lib.lkj"
    cp "$project/main.lkj" "$fixture/replay-main-before"
    cp "$library/lib.lkj" "$fixture/replay-library-before"
    if [[ $side != library ]]; then
        printf '%s\n' 'mod std { pub fn answer() -> i64 { 42i64 } pub fn replayed() -> i64 { 9i64 } }' \
            'pub fn probe() -> i64 { crate::std::answer() }' > "$desired/main.lkj"
    fi
    if [[ $side != application ]]; then
        printf '%s\n' 'pub fn answer() -> i64 { 7i64 }' 'pub fn replayed() -> i64 { 9i64 }' > "$desired/library/lib.lkj"
    fi
    "$compiler" edit "${common[@]}" --no-store --store-key "$fixture/key" --against main.lkj \
        --against-source-dir "$desired" --against-library-dir "$desired/library" > "$fixture/plan" 2> "$fixture/error"
    grep -q 'verified=true' "$fixture/plan"
    [[ ! -e "$project/.lkjscript-edit.pending" ]]
    if [[ $side == application ]]; then cmp "$fixture/replay-library-before" "$library/lib.lkj"; fi
    if [[ $side == library ]]; then cmp "$fixture/replay-main-before" "$project/main.lkj"; fi
    source_count 2
    index
    if [[ $side != library ]]; then meta value@crate::std::replayed type@crate::std "$project/main.lkj"; fi
    if [[ $side != application ]]; then meta value@std::replayed type@std "$library/lib.lkj"; fi
    "$compiler" compare "${common[@]}" --no-store --against main.lkj --against-source-dir "$desired" \
        --against-library-dir "$desired/library" > "$fixture/comparison" 2> "$fixture/error"
    grep -Fq 'checked IR identical' "$fixture/comparison"
    value 42
done

# Projection replay uses existing external parents, preserving its framing rule.
setup replay-projections
printf '%s\n' 'mod std;' 'pub fn probe() -> i64 { crate::std::answer() }' > "$project/main.lkj"
printf '%s\n' 'pub fn answer() -> i64 { 42i64 }' > "$project/std.lkj"
source_count 3
desired="$fixture/replay-projections-desired"
mkdir -p "$desired/library" "$desired/std"
cp "$project/main.lkj" "$desired/main.lkj"
printf '%s\n' 'pub fn answer() -> i64 { 42i64 }' 'pub mod child;' > "$desired/std.lkj"
printf '%s\n' 'pub fn answer() -> i64 { 7i64 }' 'pub mod child;' > "$desired/library/lib.lkj"
printf '%s\n' 'pub fn child_value() -> i64 { 9i64 }' > "$desired/std/child.lkj"
printf '%s\n' 'pub fn child_value() -> i64 { 8i64 }' > "$desired/library/child.lkj"
"$compiler" edit "${common[@]}" --no-store --store-key "$fixture/key" --against main.lkj \
    --against-source-dir "$desired" --against-library-dir "$desired/library" > "$fixture/plan" 2> "$fixture/error"
grep -q 'verified=true' "$fixture/plan"
source_count 5
cmp "$desired/std/child.lkj" "$project/std/child.lkj"
cmp "$desired/library/child.lkj" "$library/child.lkj"
index
meta value@crate::std::child::child_value type@crate::std::child "$project/std/child.lkj"
meta value@std::child::child_value type@std::child "$library/child.lkj"
value 42

# Associated and implementation stems must retain the resolver root as well.
setup associated
cat > "$project/main.lkj" <<'SOURCE'
mod std {
    pub struct Data { pub value: i64 }
    pub trait Read { type Output; fn read(&self) -> Self::Output; }
    impl Read for Data { type Output = i64; fn read(&self) -> i64 { self.value } }
    pub fn answer() -> i64 { Data { value: 42i64 }.read() }
}
pub fn probe() -> i64 { crate::std::answer() }
SOURCE
cat > "$library/lib.lkj" <<'SOURCE'
pub struct Data { pub value: i64 }
pub trait Read { type Output; fn read(&self) -> Self::Output; }
impl Read for Data { type Output = i64; fn read(&self) -> i64 { self.value } }
pub fn answer() -> i64 { Data { value: 7i64 }.read() }
SOURCE
index
meta type@crate::std::Read::type:Output type@crate::std::Read "$project/main.lkj"
meta type@std::Read::type:Output type@std::Read "$library/lib.lkj"
application_method=$(trait_method type@crate::std::Read)
library_method=$(trait_method type@std::Read)
[[ $application_method == crate::std::Read::read && $library_method == std::Read::read ]]
meta "$application_method" type@crate::std::Read "$project/main.lkj"
meta "$library_method" type@std::Read "$library/lib.lkj"
query impls type@crate::std::Read > "$fixture/application-impls"
query impls type@std::Read > "$fixture/library-impls"
grep -Fq 'impl:<crate::std::Data as crate::std::Read>' "$fixture/application-impls"
grep -Fq 'impl:<std::Data as std::Read>' "$fixture/library-impls"
value 42

# A derived owner already carries its module path; never qualify it twice.
# Derives and the following Vec group need the full standard library.
setup derived-root
library="$repo/library"
common=(main.lkj --source-dir "$project" --library-dir "$library")
cat > "$project/main.lkj" <<'SOURCE'
mod std {
    #[derive(Default)]
    pub struct Foo { pub value: i64 }
    pub fn answer() -> i64 { Foo::default().value + 42i64 }
}
pub fn probe() -> i64 { crate::std::answer() }
SOURCE
index
meta type@crate::std::Foo type@crate::std "$project/main.lkj"
generated='<crate::std::Foo as std::prelude::Default>::default'
query meta "$generated" > "$fixture/derived-meta"
definitions=$(sed -n 's/.* definition="\([^"]*\)".*/\1/p' "$fixture/derived-meta" | LC_ALL=C sort -u)
[[ $definitions == type@crate::std::Foo ]] || fail 'derived origin duplicated or changed its root'
grep -Fq 'expansion=derive-default' "$fixture/derived-meta"
grep -Fq "file=\"$project/main.lkj\"" "$fixture/derived-meta"
request "{\"op\":\"replace-body\",\"id\":\"$generated\",\"source\":\"{ Self { value: 1i64 } }\"}"
refuse E_EDIT_GENERATED_SOURCE
grep -Fq 'edit defining item "type@crate::std::Foo"' "$fixture/error"
value 42

setup builtin-family
library="$repo/library"
common=(main.lkj --source-dir "$project" --library-dir "$library")
cat > "$project/main.lkj" <<'SOURCE'
trait Count { fn count(&self) -> i64; }
impl Count for std::vec::Vec<i64> { fn count(&self) -> i64 { self.len() as i64 } }
impl Count for std::vec::Vec<bool> { fn count(&self) -> i64 { self.len() as i64 } }
pub fn probe() -> i64 {
    let mut numbers = std::vec::Vec::new(); numbers.push(42i64);
    let mut flags = std::vec::Vec::new(); flags.push(true);
    numbers.count() + flags.count()
}
SOURCE
index
query impls builtin@Vec > "$fixture/family-before"
grep $'\tfact=impls\t' "$fixture/family-before" > "$fixture/family-before-facts"
query impls Vec > "$fixture/bare-family"
grep -Fq 'selected="builtin@Vec"' "$fixture/bare-family"
cat >> "$project/main.lkj" <<'SOURCE'
mod Vec {}
fn Vec() -> i64 { 1i64 }
SOURCE
index
query_refuse E_QUERY_AMBIGUOUS impls Vec
for id in builtin@Vec type@Vec value@Vec; do grep -Fq "\"$id\"" "$fixture/error"; done
query impls builtin@Vec > "$fixture/family-after"
grep $'\tfact=impls\t' "$fixture/family-after" > "$fixture/family-after-facts"
cmp "$fixture/family-before-facts" "$fixture/family-after-facts"
[[ $(grep -Fc 'trait="Count"' "$fixture/family-after-facts") == 2 ]]
for element in i64 bool; do
    query impls "builtin@Vec<$element>" > "$fixture/concrete"
    [[ $(grep -Fc 'trait="Count"' "$fixture/concrete") == 1 ]]
    grep -Fq "target=\"Vec<$element>\"" "$fixture/concrete"
done
query meta builtin@Vec > "$fixture/builtin-meta"
grep -Fq 'kind=builtin' "$fixture/builtin-meta"
query impls type@Vec > "$fixture/module-impls"
grep -Fq 'fact=empty-indexed-result' "$fixture/module-impls"
if grep -Fq 'trait="Count"' "$fixture/module-impls"; then fail 'module acquired builtin implementations'; fi
for id in builtin@ type@builtin@x prim@x type-name@x; do query_refuse E_QUERY_SELECTOR meta "$id"; done

# A same-spelled nominal application and builtin concrete type have separate keys.
setup builtin-nominal
library="$repo/library"
common=(main.lkj --source-dir "$project" --library-dir "$library")
cat > "$project/main.lkj" <<'SOURCE'
struct Vec<T> { value: T }
pub fn probe() -> i64 {
    let local: Vec<i64> = Vec { value: 42i64 };
    let mut builtin: std::vec::Vec<i64> = std::vec::Vec::new();
    builtin.push(7i64);
    local.value + builtin.len() as i64
}
SOURCE
index
query type 'type@Vec<i64>' > "$fixture/nominal-type"
query type 'builtin@Vec<i64>' > "$fixture/intrinsic-type"
query_refuse E_QUERY_AMBIGUOUS type 'Vec<i64>'
for id in 'type@Vec<i64>' 'builtin@Vec<i64>'; do grep -Fq "\"$id\"" "$fixture/error"; done
value 43

# Same-spelled nominal/builtin associated stems still collide: refuse indexing.
# Unlike overlapping trait implementations, this source passes ordinary checking.
cat > "$project/main.lkj" <<'SOURCE'
struct Vec<T> { value: T }
impl Vec<i64> { fn read(&self) -> i64 { self.value } }
impl std::vec::Vec<i64> { fn read(&self) -> i64 { self.len() as i64 } }
pub fn probe() -> i64 { 42i64 }
SOURCE
"$compiler" check "${common[@]}" --no-store
index_refuse E_IDENTITY

printf 'ok catalog roots: exact application/library parents, anchored insertion, projection and replay, associated stems, keyword refusal, literal selectors and builtin family separation\n'
