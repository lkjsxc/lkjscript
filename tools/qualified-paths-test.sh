#!/usr/bin/env bash
# Qualified receivers, trait identities and member spans survive source edits.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
mkdir -p target
fixture=$(mktemp -d "$PWD/target/programs-qualified-paths.XXXXXX")
trap 'if [[ ${LKJSCRIPT_QUALIFIED_PATHS_KEEP:-0} == 1 ]]; then printf "retained_fixture=%s\n" "$fixture" >&2; else rm -rf -- "$fixture"; fi' EXIT HUP INT TERM
umask 077
mkdir "$fixture/project" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
fail() { printf 'qualified-paths: %s\n' "$*" >&2; exit 1; }
common=(main.lkj --source-dir "$fixture/project" --no-library --store "$fixture/store" --store-key "$fixture/key")
publish() {
    "$compiler" build "${common[@]}" --index -o "$fixture/image" > "$fixture/index" 2> "$fixture/error" || { cat "$fixture/error" >&2; fail build; }
    snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/index")
    [[ ${#snapshot} == 64 ]] || fail 'missing indexed snapshot'
}
query() {
    local kind=$1 identity=$2
    shift 2
    "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$kind" --id "$identity" --limit 128 --bytes 65536 "$@"
}
source_type() {
    query type "$1" > "$fixture/type"
    grep -F "fact=$2" "$fixture/type" | grep -Fq "value=\"$3\"" || { cat "$fixture/type" >&2; fail "source type of $1"; }
}
sites() {
    # Check the resolved target and the precise editable token, not a matching
    # word somewhere in a qualified path or a different trait's same-name item.
    perl -MJSON::PP - "$1" "$2" "$3" "$4" "$5" "$fixture/project" <<'PERL'
use strict;
use warnings;
my ($facts,$wanted,$kind,$spelling,$expected,$root)=@ARGV;
open my $input,'<',$facts or die $!;
my $count=0;
while (<$input>) {
    next unless /^("(?:\\.|[^"])*")\s+level=source-claim\s+fact=references\s+from=("(?:\\.|[^"])*")\s+file=("(?:\\.|[^"])*")\s+start=(\d+)\s+end=(\d+)\s+kind=(\S+)/;
    my ($target,$file,$start,$end,$found)=(decode_json($1),decode_json($3),$4,$5,$6);
    next unless $target eq $wanted && $found eq $kind;
    $file="$root/$file" unless $file =~ m{^/};
    open my $source,'<:raw',$file or die "$file: $!";
    local $/;
    my $text=<$source>;
    my $actual=substr($text,$start,$end-$start);
    die "$wanted $kind span spells '$actual', expected '$spelling'\n" unless $actual eq $spelling;
    $count++;
}
die "$wanted $kind has $count sites, expected $expected\n" unless $count==$expected;
PERL
}
binding() {
    query outline "$1" > "$fixture/outline"
    perl -MJSON::PP -e '
        my ($file,$suffix)=@ARGV; open my $input,"<",$file or die $!; my @ids;
        while(<$input>) {
            next unless /^("(?:\\.|[^"])*")\s+level=source-claim\s+fact=signature\s+kind=binding/;
            my $id=decode_json($1); push @ids,$id if $id =~ /\Q$suffix\E$/;
        }
        die "expected one binding $suffix, found ".scalar(@ids)."\n" unless @ids==1;
        print $ids[0];
    ' "$fixture/outline" "::binding:$2:0"
}
edit() {
    "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/plan" 2> "$fixture/error" || { cat "$fixture/error" >&2; fail 'checked edit'; }
    grep -q 'mode=apply' "$fixture/plan" || fail 'edit did not apply'
    [[ ! -e $fixture/project/.lkjscript-edit.pending ]] || fail 'edit left a pending transaction'
    cmp "$fixture/untouched-right" "$fixture/project/right.lkj" || fail 'unrelated same-name trait changed'
}
rename() {
    perl -MJSON::PP -e 'print encode_json({version=>1,operations=>[{op=>"rename",id=>$ARGV[0],name=>$ARGV[1]}]}),"\n"' "$1" "$2" > "$fixture/request"
    edit
}
value() {
    for engine in native interpreter; do
        local actual
        actual=$("$compiler" run main.lkj --source-dir "$fixture/project" --no-library --no-store --engine "$engine")
        [[ $actual == "$1" ]] || fail "$engine returned $actual, expected $1"
    done
}
cat > "$fixture/project/main.lkj" <<'SOURCE'
mod left;
mod right;
use left::Value;
pub struct Number {}
fn roundtrip(value: <Number as left::Source>::Item) -> <Number as left::Source>::Item { value }
fn concrete_chain() -> i64 { <Number as left::Source>::Item::make() }
fn main() -> i64 {
    if right::selected::<Number>() && right::chained::<Number>() {
        roundtrip(left::project::<Number>(left::selected::<Number>()))
            + left::chained::<Number>() + concrete_chain() - 78
    } else { 0 }
}
SOURCE
cat > "$fixture/project/left.lkj" <<'SOURCE'
use crate::Number;
pub trait Source { type Item; fn get() -> <Self as Source>::Item; }
pub trait Value { fn make() -> Self; fn value(self) -> i64; }
impl Source for Number { type Item = i64; fn get() -> i64 { 40 } }
impl Value for i64 { fn make() -> i64 { 40 } fn value(self) -> i64 { self } }
pub fn selected<T: Source>() -> i64 where <T as Source>::Item: Value {
    let held: <T as Source>::Item = <T as Source>::get();
    <<T as Source>::Item as Value>::value(held)
}
pub fn project<T: Source>(value: <T as Source>::Item) -> <T as Source>::Item { value }
pub fn chained<T: Source>() -> i64 where <T as Source>::Item: Value {
    let created: <T as Source>::Item = <T as Source>::Item::make();
    <<T as Source>::Item as Value>::value(created)
}
SOURCE
cat > "$fixture/project/right.lkj" <<'SOURCE'
use crate::Number;
pub trait Source { type Item; fn get() -> <Self as Source>::Item; }
pub trait Value { fn make() -> Self; }
impl Source for Number { type Item = bool; fn get() -> bool { true } }
impl Value for bool { fn make() -> bool { true } }
pub fn selected<T: Source>() -> <T as Source>::Item { <T as Source>::get() }
pub fn chained<T: Source>() -> <T as Source>::Item where <T as Source>::Item: Value {
    <T as Source>::Item::make()
}
SOURCE
cp "$fixture/project/right.lkj" "$fixture/untouched-right"
publish
value 42

# Receiver, trait and member each have their own exact source binding. The
# generic source type keeps its trait identity even after a concrete call.
query references roundtrip > "$fixture/concrete-references"
sites "$fixture/concrete-references" Number type Number 2
sites "$fixture/concrete-references" left::Source type Source 2
sites "$fixture/concrete-references" left::Source::type:Item associated-type Item 2
query references concrete_chain > "$fixture/concrete-chain-references"
sites "$fixture/concrete-chain-references" Number type Number 1
sites "$fixture/concrete-chain-references" left::Source type Source 1
sites "$fixture/concrete-chain-references" left::Source::type:Item associated-type Item 1
query references left::selected > "$fixture/left-references"
sites "$fixture/left-references" left::Source type Source 5
sites "$fixture/left-references" left::Source::type:Item associated-type Item 3
sites "$fixture/left-references" left::Source::get associated-call get 1
sites "$fixture/left-references" right::Source::type:Item associated-type Item 0
sites "$fixture/left-references" right::Source::get associated-call get 0
query references right::selected > "$fixture/right-references"
sites "$fixture/right-references" right::Source::type:Item associated-type Item 1
sites "$fixture/right-references" right::Source::get associated-call get 1
sites "$fixture/right-references" left::Source::type:Item associated-type Item 0
sites "$fixture/right-references" left::Source::get associated-call get 0
query references left::chained > "$fixture/left-chain-references"
sites "$fixture/left-chain-references" left::Source type Source 5
sites "$fixture/left-chain-references" left::Source::type:Item associated-type Item 4
sites "$fixture/left-chain-references" left::Value::make associated-call make 1
sites "$fixture/left-chain-references" right::Value::make associated-call make 0
query references right::chained > "$fixture/right-chain-references"
sites "$fixture/right-chain-references" right::Source::type:Item associated-type Item 3
sites "$fixture/right-chain-references" right::Value::make associated-call make 1
sites "$fixture/right-chain-references" left::Value::make associated-call make 0
source_type Number declared-type Number
source_type left::Source source-kind left::Source
source_type left::project::param:value declared-type '<T as left::Source>::Item'
source_type roundtrip::param:value declared-type '<Number as left::Source>::Item'
held=$(binding left::selected held)
source_type "$held" inferred-type '<T as left::Source>::Item'
created=$(binding left::chained created)
source_type "$created" inferred-type '<T as left::Source>::Item'

# Both associated declarations and matching implementations follow the selected
# trait. The unrelated module is checked byte-for-byte after every transaction.
rename left::Source::type:Item Output
value 42
grep -Fq 'type Output = i64' "$fixture/project/left.lkj" || fail 'associated implementation was not renamed'
grep -Fq '<Number as left::Source>::Output' "$fixture/project/main.lkj" || fail 'concrete projection was not renamed'
grep -Fq 'where <T as Source>::Output: Value' "$fixture/project/left.lkj" || fail 'predicate projection was not renamed'
grep -Fq '<T as Source>::Output::make()' "$fixture/project/left.lkj" || fail 'chained projection was not renamed'
publish
source_type left::project::param:value declared-type '<T as left::Source>::Output'
rename left::Source::get fetch
value 42
grep -Fq '<T as Source>::fetch()' "$fixture/project/left.lkj" || fail 'qualified value call was not renamed'
publish
query references left::selected > "$fixture/renamed-references"
sites "$fixture/renamed-references" left::Source::type:Output associated-type Output 3
sites "$fixture/renamed-references" left::Source::fetch associated-call fetch 1
sites "$fixture/renamed-references" left::Source::type:Item associated-type Item 0
sites "$fixture/renamed-references" left::Source::get associated-call get 0
rename left::Value::make construct
value 42
grep -Fq '<T as Source>::Output::construct()' "$fixture/project/left.lkj" || fail 'generic chained member was not renamed'
grep -Fq '<Number as left::Source>::Output::construct()' "$fixture/project/main.lkj" || fail 'concrete chained member was not renamed'
publish
query references left::chained > "$fixture/renamed-chain-references"
sites "$fixture/renamed-chain-references" left::Source::type:Output associated-type Output 4
sites "$fixture/renamed-chain-references" left::Value::construct associated-call construct 1
sites "$fixture/renamed-chain-references" left::Value::make associated-call make 0
created=$(binding left::chained created)
source_type "$created" inferred-type '<T as left::Source>::Output'

cp "$fixture/project/main.lkj" "$fixture/unchanged-main"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"left::selected","source":"{ let held: <T as Source>::Output = <T as Source>::Output::construct(); <<T as Source>::Output as Value>::value(held) + 1 }"}]}
REQUEST
edit
cmp "$fixture/unchanged-main" "$fixture/project/main.lkj" || fail 'body replacement changed its caller'
value 43
publish
held=$(binding left::selected held)
source_type "$held" inferred-type '<T as left::Source>::Output'
query references left::selected > "$fixture/replaced-references"
sites "$fixture/replaced-references" left::Value::construct associated-call construct 1
sites "$fixture/replaced-references" left::Source::fetch associated-call fetch 0
printf 'ok qualified paths: direct and chained receiver/trait/member bindings, generic source types, distinct trait identities, associated type/member rename and body replacement on both engines\n'
