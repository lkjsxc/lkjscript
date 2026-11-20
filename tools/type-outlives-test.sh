#!/usr/bin/env bash
# Type-outlives predicates keep scoped identities and checked edit obligations.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
mkdir -p target
fixture=$(mktemp -d "$PWD/target/type-outlives.XXXXXX")
cleanup() {
    status=$?
    if ((status)); then
        cat "$fixture/source/main.lkj" >&2
        for file in "$fixture"/*.log "$fixture"/*.query; do
            [[ ! -f "$file" ]] || cat "$file" >&2
        done
    fi
    if [[ ${LKJSCRIPT_TYPE_OUTLIVES_KEEP:-0} != 1 ]]; then rm -rf "$fixture"; fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
mkdir "$fixture/source" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/source/main.lkj" <<'SOURCE'
fn hold<'a, T>(value: T, number: i64) -> impl Fn() -> i64 + 'a
where T: Clone + 'a { move || { drop(value.clone()); number } }
struct Maker<'outer> { marker: &'outer i64 }
impl<'outer> Maker<'outer> {
    fn keep<'inner, T>(&self, value: T, number: i64) -> impl Fn() -> i64 + 'inner
    where T: Clone + 'outer, 'outer: 'inner {
        move || { drop(value.clone()); number }
    }
}
fn main() -> i64 {
    let marker = 7i64;
    let maker = Maker { marker: &marker };
    hold(&marker, 40i64)() + maker.keep(&marker, 2i64)()
}
SOURCE
common=(main.lkj --source-dir "$fixture/source" --library-dir "$PWD/library" --store "$fixture/store" --store-key "$fixture/key")
publish() {
    "$compiler" build "${common[@]}" --index --stats -o "$fixture/program" > "$fixture/$1.log" 2>&1
    snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/$1.log")
    [[ ${#snapshot} == 64 ]]
}
query() {
    "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$1" --id "$2" --limit 128 --bytes 65536
}
execute() {
    for engine in native interpreter; do
        [[ $("$compiler" run "${common[@]}" --no-store --engine "$engine") == "$1" ]]
    done
}
meaning() {
    local owner=$1 lifetime=$2 mode=$3
    for kind in type signature; do
        query "$kind" "$owner" > "$fixture/$mode-$kind.query"
        grep -Fq "T:" "$fixture/$mode-$kind.query"
        grep -Fq "$lifetime" "$fixture/$mode-$kind.query"
        if [[ $kind == type ]]; then grep -Fq "T: $lifetime" "$fixture/$mode-$kind.query"; fi
    done
    query references "$owner" > "$fixture/$mode-references.query"
    perl -MJSON::PP - "$fixture/$mode-references.query" "$fixture/source" "$owner" "$lifetime" "$mode" <<'PERL'
use strict;
use warnings;
my ($facts,$root,$owner,$lifetime,$mode)=@ARGV;
$owner = "value\@$owner" if $mode eq 'free';
open my $input,'<',$facts or die $!;
my $seen=0;
while (<$input>) {
    next unless /^("(?:\\.|[^"])*")\s+level=source-claim\s+fact=references\s+from=("(?:\\.|[^"])*")\s+file=("(?:\\.|[^"])*")\s+start=(\d+)\s+end=(\d+)\s+kind=type-outlives-bound\b/;
    my ($target,$file,$start,$end)=(decode_json($1),decode_json($3),$4,$5);
    $file="$root/$file" unless $file =~ m{^/};
    open my $source,'<:raw',$file or die $!;
    local $/;
    my $text=<$source>;
    die "type-outlives span does not select its exact lifetime token\n"
        unless substr($text,$start,$end-$start) eq $lifetime;
    if ($mode eq 'outer') {
        die "enclosing lifetime lost its implementation binder\n"
            unless $target =~ /^impl:Maker<'outer>#[0-9a-f]+::generic:'outer$/;
    } else {
        die "type-outlives bound selected the wrong function binder\n"
            unless $target eq $owner."::generic:".$lifetime;
    }
    ++$seen;
}
die "type-outlives bound missing or duplicated\n" unless $seen==1;
PERL
}
publish cold
first_snapshot=$snapshot
execute 42
meaning hold "'a" free
meaning "Maker<'outer>::keep" "'outer" outer
publish warm
[[ $snapshot == "$first_snapshot" ]]
grep -Eq '"build_reused":1' "$fixture/warm.log"

cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"hold","source":"{ move || { drop(value.clone()); number + 1i64 } }"}]}
REQUEST
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/body-edit.log" 2>&1
execute 43
publish body-edited
meaning hold "'a" free
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"hold","name":"hold_value"}]}
REQUEST
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rename.log" 2>&1
execute 43
publish renamed
meaning hold_value "'a" free
meaning "Maker<'outer>::keep" "'outer" outer
valid_snapshot=$snapshot
cp "$fixture/source/main.lkj" "$fixture/before-invalid"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"main","source":"{ hold_value(7i64, 43i64)() }"},{"op":"replace-item","id":"hold_value","source":"fn hold_value<'a, T>(value: T, number: i64) -> impl Fn() -> i64 + 'a where T: Clone { move || { drop(value.clone()); number } }"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rejected-bound.log" 2>&1; then
    echo 'checked edit removed a required whole-value type-outlives obligation' >&2
    exit 1
fi
grep -Eq 'E_EDIT_CHECK|E_BORROW' "$fixture/rejected-bound.log"
cmp "$fixture/before-invalid" "$fixture/source/main.lkj"
test ! -e "$fixture/source/.lkjscript-edit.pending"
publish rollback
[[ $snapshot == "$valid_snapshot" ]]
grep -Eq '"build_reused":1' "$fixture/rollback.log"
execute 43
printf 'ok type outlives: exact function/enclosing lifetime identities, body and rename edits, whole-value bound-removal rollback, warm snapshots and both engines\n'
