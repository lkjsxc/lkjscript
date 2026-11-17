#!/usr/bin/env bash
# Exact associated-call source evidence and conservative refactoring consumers.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
mode=${2:-cases}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
[[ $mode == cases || $mode == baseline ]] || exit 2
mkdir -p target
fixture=$(mktemp -d "$PWD/target/compiler-associated-cases.XXXXXX")
trap 'if [[ ${COMPILER_ASSOCIATED_KEEP:-0} == 1 ]]; then printf "retained_fixture=%s\\n" "$fixture" >&2; else rm -rf "$fixture"; fi' EXIT HUP INT TERM
mkdir "$fixture/project" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
common=(main.lkj --source-dir "$fixture/project" --no-library --store "$fixture/store" --store-key "$fixture/key")
publish() {
    "$compiler" build "${common[@]}" --index -o "$fixture/image" > "$fixture/index"
    snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/index")
    [[ ${#snapshot} == 64 ]]
    query outline crate --depth 8 > "$fixture/outline"
}
query() {
    local kind=$1 identity=$2
    shift 2
    "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$kind" --id "$identity" --limit 128 --bytes 100000 "$@"
}
identity() {
    perl -MJSON::PP -e '
        my ($file,$suffix)=@ARGV; open my $fh,"<",$file or die $!; my @ids;
        while(<$fh>) {
            next unless /^(("(?:\\.|[^"])*"))\s+level=source-claim\s+fact=signature\s+kind=function/;
            my $id=decode_json($1); push @ids,$id if $id =~ /\Q$suffix\E$/;
        }
        die "expected one function identity for $suffix, found ".scalar(@ids) unless @ids==1;
        print $ids[0];
    ' "$fixture/outline" "$1"
}
uses() {
    query uses "$1" > "$fixture/uses"
    local actual
    actual=$(perl -ne '$n++ if /level=source-claim\s+fact=uses\s+.*kind=associated-call/; END {print $n//0}' "$fixture/uses")
    [[ $actual == "$2" ]]
}
rename() {
    perl -MJSON::PP -e 'print encode_json({version=>1,operations=>[{op=>"rename",id=>$ARGV[0],name=>$ARGV[1]}]}),"\n"' "$1" "$2" > "$fixture/request"
    "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/plan"
    grep -q 'mode=apply' "$fixture/plan"
    [[ ! -e $fixture/project/.lkjscript-edit.pending ]]
}
value() {
    [[ $("$compiler" run "$fixture/project/main.lkj" --no-library --no-store) == "$1" ]]
    [[ $("$compiler" run "$fixture/project/main.lkj" --no-library --no-store --engine interpreter) == "$1" ]]
}
refuse_rename() {
    cp "$fixture/project/main.lkj" "$fixture/unchanged"
    perl -MJSON::PP -e 'print encode_json({version=>1,operations=>[{op=>"rename",id=>$ARGV[0],name=>"changed"}]}),"\n"' "$1" > "$fixture/request"
    if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/plan" 2> "$fixture/error"; then
        echo 'associated call lacking a proved source target was renamed' >&2; exit 1
    fi
    grep -q E_EDIT_REFERENCES "$fixture/error"
    cmp "$fixture/unchanged" "$fixture/project/main.lkj"
    [[ ! -e $fixture/project/.lkjscript-edit.pending ]]
}
cat > "$fixture/project/main.lkj" <<'SOURCE'
struct Widget {}
impl Widget {
    fn pair(a: i64, b: i64) -> i64 { a * 10i64 + b }
    fn through() -> i64 { Self::pair(1i64, 2i64) }
}
fn idle() -> i64 { Widget::pair(2i64, 3i64) }
fn main() -> i64 { Widget::through() }
SOURCE
publish
pair=$(identity '::pair')
through=$(identity '::through')
if [[ $mode == baseline ]]; then
    uses "$pair" 0
    refuse_rename "$pair"
    printf 'ok baseline associated calls lack source identities and rename refuses unchanged\n'
    exit 0
fi
uses "$pair" 2
query callees "$through" > "$fixture/callees"
grep -F "$pair" "$fixture/callees" | grep -q 'level=checked-ir.*fact=callees'
rename "$pair" combine
value 12
publish
combine=$(identity '::combine')
perl -MJSON::PP -e 'print encode_json({version=>1,operations=>[{op=>"change-signature",id=>$ARGV[0],parameters=>[$ARGV[0]."::param:b",$ARGV[0]."::param:a"]}]}),"\n"' "$combine" > "$fixture/request"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/plan"
value 12
grep -q 'Widget::combine' "$fixture/project/main.lkj"
grep -q 'Self::combine' "$fixture/project/main.lkj"

# Existing frontend limitation: a trait default's rigid Self path is rejected
# as a constructor before associated-call selection. Preserve that refusal.
cat > "$fixture/project/main.lkj" <<'SOURCE'
trait Maker { fn make() -> i64; fn again() -> i64 { Self::make() } }
struct Concrete {}
impl Maker for Concrete { fn make() -> i64 { 9i64 } }
fn main() -> i64 { Concrete::make() }
SOURCE
if "$compiler" check "${common[@]}" --no-store > "$fixture/rejected" 2> "$fixture/error"; then
    echo 'trait-default Self routing unexpectedly changed' >&2; exit 1
fi
grep -q 'E_TYPE constructor requires a nominal struct or enum' "$fixture/error"

cat > "$fixture/project/main.lkj" <<'SOURCE'
trait Maker {
    fn make() -> i64;
}
struct Concrete {}
impl Maker for Concrete { fn make() -> i64 { 9i64 } }
fn idle<T: Maker>() -> i64 { T::make() }
fn main() -> i64 { Concrete::make() }
SOURCE
publish
uses Maker::make 1
query instances idle > "$fixture/instances"
if grep -q 'level=checked-ir.*fact=instances' "$fixture/instances"; then exit 1; fi
rename Maker::make fabricate_value
value 9
grep -q 'T::fabricate_value()' "$fixture/project/main.lkj"
grep -q 'Concrete::fabricate_value()' "$fixture/project/main.lkj"

cat > "$fixture/project/main.lkj" <<'SOURCE'
struct Original {}
impl Original { fn make() -> i64 { 11i64 } }
type Alias = Original;
mod calls {
    use crate::Original as Visible;
    pub fn run() -> i64 { Visible::make() }
}
fn main() -> i64 { Alias::make() + calls::run() }
SOURCE
publish
make=$(identity '::make')
uses "$make" 2
rename "$make" fabricate_value
value 22
grep -q 'Alias::fabricate_value()' "$fixture/project/main.lkj"
grep -q 'Visible::fabricate_value()' "$fixture/project/main.lkj"
grep -q 'use crate::Original as Visible' "$fixture/project/main.lkj"

cat > "$fixture/project/main.lkj" <<'SOURCE'
struct Local {}
impl Local { fn from_bits(value: u64) -> i64 { value as i64 } }
mod shadow {
    use crate::Local as f64;
    pub fn run() -> i64 { f64::from_bits(7u64) }
}
fn builtin() -> f64 { f64::from_bits(0u64) }
fn main() -> i64 { shadow::run() }
SOURCE
publish
from_bits=$(identity '::from_bits')
uses "$from_bits" 1
rename "$from_bits" reconstruct_value
value 7
grep -q 'f64::reconstruct_value(7u64)' "$fixture/project/main.lkj"
grep -q 'f64::from_bits(0u64)' "$fixture/project/main.lkj"

cat > "$fixture/project/main.lkj" <<'SOURCE'
struct Fixed {}
impl Fixed { fn make() -> i64 { 5i64 } }
struct Family<T> { value: T }
impl<T> Family<T> {
    fn selected() -> i64 {
        fn nested() -> i64 { Fixed::make() }
        nested()
    }
}
fn main() -> i64 { Family::<i64>::selected() }
SOURCE
if "$compiler" check "${common[@]}" --no-store > "$fixture/rejected" 2> "$fixture/error"; then
    echo 'local-function admission unexpectedly changed' >&2; exit 1
fi
grep -q 'E_UNSUPPORTED local declarations are not implemented yet' "$fixture/error"

cat > "$fixture/project/main.lkj" <<'SOURCE'
struct Fixed {}
impl Fixed { fn make() -> i64 { 5i64 } }
fn lifetime_only<'a>(value: &'a i64) -> i64 { Fixed::make() + *value }
fn main() -> i64 { let value = 7i64; lifetime_only(&value) }
SOURCE
publish
make=$(identity '::make')
uses "$make" 1
rename "$make" fabricate_value
value 12

cat > "$fixture/project/main.lkj" <<'SOURCE'
struct Boxed<T> { value: T }
impl<T> Boxed<T> { fn make(value: T) -> Self { Self { value } } }
fn generic<T>(value: T) -> Boxed<T> { Boxed::<T>::make(value) }
fn main() -> i64 { let value = generic(3i64); value.value }
SOURCE
publish
make=$(identity '::make')
uses "$make" 0
refuse_rename "$make"
value 3
"$compiler" build "${common[@]}" --index --stats -o "$fixture/image" > "$fixture/warm-index" 2> "$fixture/warm-stats"
grep -Eq '"build_reused":1' "$fixture/warm-stats"
query type generic > "$fixture/generic-type"
grep -Fq 'Boxed<T>' "$fixture/generic-type"
cp "$fixture/project/main.lkj" "$fixture/unchanged"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"generic","source":"{ Boxed::make::<T>(value) }"}]}' > "$fixture/request"
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/plan" 2> "$fixture/error"; then
    echo 'receiver arguments moved onto a nongeneric method were accepted' >&2; exit 1
fi
grep -Eq 'E_TYPE|E_EDIT_CHECK' "$fixture/error"
cmp "$fixture/unchanged" "$fixture/project/main.lkj"
[[ ! -e $fixture/project/.lkjscript-edit.pending ]]

cat > "$fixture/project/main.lkj" <<'SOURCE'
struct Fixed {}
impl Fixed { fn make() -> i64 { 5i64 } }
struct Family<T> { value: T }
impl<T> Family<T> { fn selected() -> i64 { Fixed::make() } }
fn main() -> i64 { Family::<i64>::selected() }
SOURCE
publish
make=$(identity '::make')
uses "$make" 0
refuse_rename "$make"
value 5

cat > "$fixture/project/main.lkj" <<'SOURCE'
struct Boxed<T> { value: T }
impl<T> Boxed<T> { fn make(value: T) -> Self { Self { value } } }
fn main() -> i64 { let value = Boxed::<i64>::make(4i64); value.value }
SOURCE
publish
make=$(identity '::make')
uses "$make" 0
refuse_rename "$make"
value 4
printf 'ok associated calls: closed/lifetime-only owners, declared Trait contracts and inherent Self, aliases, builtin distinction, rename/signature, and generic/turbofish refusals\n'
