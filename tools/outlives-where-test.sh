#!/usr/bin/env bash
# Named lifetime predicates retain scope, direction and meaning through edits.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
mkdir -p target
fixture=$(mktemp -d "$PWD/target/outlives-where.XXXXXX")
cleanup() {
    status=$?
    if ((status)); then
        cat "$fixture/source/main.lkj" >&2
        for file in "$fixture"/*.log "$fixture"/*.query; do [[ ! -f "$file" ]] || cat "$file" >&2; done
    fi
    rm -rf "$fixture"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
mkdir "$fixture/source" "$fixture/store" "$fixture/front-store"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/source/main.lkj" <<'SOURCE'
struct Holder<'long> { value: &'long i64 }
impl<'long> Holder<'long> {
    fn read<'short>(&self, _marker: &'short i64) -> i64
    where 'long: 'short {
        let shortened: &'short i64 = self.value;
        *shortened
    }
}
fn choose<'long, 'short>(first: &'long i64, second: &'long i64, _marker: &'short i64) -> i64
where 'long: 'short {
    let shortened: &'short i64 = first;
    *shortened
}
fn main() -> i64 {
    let first = 40i64;
    let second = 42i64;
    let marker = 2i64;
    let holder = Holder { value: &marker };
    choose(&first, &second, &marker) + holder.read(&marker)
}
SOURCE
common=(main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/store" --store-key "$fixture/key")
publish() {
    local phase=${1:-build}
    "$compiler" build "${common[@]}" --index --stats -o "$fixture/program" > "$fixture/$phase.log" 2>&1
    snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/$phase.log")
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
    query type "$1" > "$fixture/meaning.log"
    grep -Fq "where 'long: 'short" "$fixture/meaning.log"
    query signature "$1" > "$fixture/signature.log"
    grep -Fq "where 'long: 'short" "$fixture/signature.log"
}
references() {
    local identity=$1 mode=$2
    query references "$identity" > "$fixture/references.log"
    perl -MJSON::PP - "$fixture/references.log" "$fixture/source" "$identity" "$mode" <<'PERL'
use strict;
use warnings;
my ($facts,$root,$owner,$mode)=@ARGV;
open my $input,'<',$facts or die $!;
my %seen;
while (<$input>) {
    next unless /^("(?:\\.|[^"])*")\s+level=source-claim\s+fact=references\s+from=("(?:\\.|[^"])*")\s+file=("(?:\\.|[^"])*")\s+start=(\d+)\s+end=(\d+)\s+kind=lifetime-bound\b/;
    my ($target,$file,$start,$end)=(decode_json($1),decode_json($3),$4,$5);
    $file="$root/$file" unless $file =~ m{^/};
    open my $source,'<:raw',$file or die $!;
    local $/;
    my $text=<$source>;
    my $spelling=substr($text,$start,$end-$start);
    die "unexpected lifetime token $spelling\n" unless $spelling eq "'long" || $spelling eq "'short";
    my $expected=$owner."::generic:".$spelling;
    if ($mode eq 'outer' && $spelling eq "'long") {
        die "outer lifetime did not select its impl declaration\n"
            unless $target =~ /^impl:Holder<'long>#[0-9a-f]+::generic:'long$/;
    } else {
        die "$spelling selected $target instead of $expected\n" unless $target eq $expected;
    }
    $seen{$spelling}++;
}
die "predicate lifetime spans missing or duplicated\n"
    unless ($seen{"'long"} // 0)==1 && ($seen{"'short"} // 0)==1;
PERL
}
publish
execute 42
meaning choose
references choose free
meaning "Holder<'long>::read"
references "Holder<'long>::read" outer
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"choose","source":"{ let shortened: &'short i64 = second; *shortened }"}]}
REQUEST
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/edit.log" 2>&1
execute 44
publish
meaning choose
references choose free
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"choose","name":"choose_value"}]}
REQUEST
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rename.log" 2>&1
execute 44
publish
meaning choose_value
references choose_value free
cp "$fixture/source/main.lkj" "$fixture/before-invalid"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"choose_value","source":"{ let local = 0i64; let invalid: &'long i64 = &local; *invalid }"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rejected.log" 2>&1; then
    echo 'invalid fixed-region body edit was accepted' >&2
    exit 1
fi
grep -Eq 'E_EDIT_CHECK|E_BORROW' "$fixture/rejected.log"
cmp "$fixture/before-invalid" "$fixture/source/main.lkj"
test ! -e "$fixture/source/.lkjscript-edit.pending"
execute 44

# Const qualification and all three binder domains retain their independent
# source identities, including a relaxed type referent and an array count.
cat > "$fixture/source/main.lkj" <<'SOURCE'
const fn shorten<'long, 'short, 'other, T: ?Sized, const N: usize>(
    first: &'long T, other: &'other T, _array: &[u8; N]
) -> &'short T
where 'long: 'short { first }
fn main() -> i64 {
    let bytes = [0u8; 2];
    shorten::<str, 2>("ab", "cd", &bytes).len() as i64 + 40
}
SOURCE
mixed_meaning() {
    local phase=$1 parameter=$2
    meaning shorten
    references shorten free
    query type shorten > "$fixture/$phase-function.query"
    grep -Fq 'const fn' "$fixture/$phase-function.query"
    grep -Fq "const $parameter: usize" "$fixture/$phase-function.query"
    grep -Fq "[u8; $parameter]" "$fixture/$phase-function.query"
    query signature shorten > "$fixture/$phase-signature.query"
    grep -Fq 'T: ?Sized' "$fixture/$phase-signature.query"
    query signature shorten::generic:T > "$fixture/$phase-type-parameter.query"
    grep -Fq '?Sized' "$fixture/$phase-type-parameter.query"
    query type "shorten::generic:$parameter" > "$fixture/$phase-const-parameter.query"
    grep -Fq 'value="usize"' "$fixture/$phase-const-parameter.query"
    query uses "shorten::generic:$parameter" > "$fixture/$phase-const-uses.query"
    grep -Fq 'const-parameter' "$fixture/$phase-const-uses.query"
    if grep -Eq '\$const-(value|rigid|array)' "$fixture/$phase-function.query" "$fixture/$phase-const-parameter.query"; then
        echo 'private const identity escaped into mixed source meaning' >&2
        exit 1
    fi
}
publish mixed-cold
mixed_meaning mixed-cold N
execute 42
mixed_snapshot=$snapshot
publish mixed-warm
[[ $snapshot == "$mixed_snapshot" ]]
grep -Eq '"build_reused":1' "$fixture/mixed-warm.log"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"shorten::generic:N","name":"COUNT"}]}
REQUEST
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/mixed-rename.log" 2>&1
publish mixed-renamed
mixed_meaning mixed-renamed COUNT
execute 42
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"shorten","source":"{ let shortened: &'short T = first; shortened }"}]}
REQUEST
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/mixed-edit.log" 2>&1
publish mixed-edited
mixed_meaning mixed-edited COUNT
execute 42
mixed_snapshot=$snapshot
publish mixed-edited-warm
[[ $snapshot == "$mixed_snapshot" ]]
grep -Eq '"build_reused":1' "$fixture/mixed-edited-warm.log"
cp "$fixture/source/main.lkj" "$fixture/mixed-before-invalid"
cat > "$fixture/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"shorten","source":"{ other }"}]}
REQUEST
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/mixed-rejected.log" 2>&1; then
    echo 'mixed binder edit accepted an unrelated input lifetime' >&2
    exit 1
fi
grep -Eq 'E_EDIT_CHECK|E_BORROW' "$fixture/mixed-rejected.log"
cmp "$fixture/mixed-before-invalid" "$fixture/source/main.lkj"
test ! -e "$fixture/source/.lkjscript-edit.pending"
publish mixed-rollback
[[ $snapshot == "$mixed_snapshot" ]]
grep -Eq '"build_reused":1' "$fixture/mixed-rollback.log"
execute 42

# Indexed builds intentionally skip frontend checkpoints. A separate lifetime-
# only source and store exercise a same-width provenance edit on that route.
cat > "$fixture/source/main.lkj" <<'SOURCE'
fn retain<'long, 'short>(first: &'long i64, other: &'short i64) -> &'long i64
where 'long: 'short { first }
fn main() -> i64 {
    let first = 42i64;
    let other = 0i64;
    *retain(&first, &other)
}
SOURCE
checkpoint() {
    "$compiler" build main.lkj --source-dir "$fixture/source" --no-library \
        --store "$fixture/front-store" --store-key "$fixture/key" --stats \
        -o "$fixture/front-program" > "$fixture/$1.log" 2>&1
}
checkpoint front-cold
checkpoint front-warm
grep -Eq '"build_reused":1' "$fixture/front-warm.log"
perl - "$fixture/source/main.lkj" <<'SOURCE_EDIT'
use strict;
use warnings;
my $path = $ARGV[0];
open my $input, '<:raw', $path or die $!;
local $/;
my $text = <$input>;
close $input;
my $count = ($text =~ s/let first = 42i64;/let first = 41i64;/g);
die "expected one same-width literal change\n" unless $count == 1;
open my $output, '>:raw', $path or die $!;
print {$output} $text;
close $output or die $!;
SOURCE_EDIT
checkpoint front-literal
grep -Eq '"build_reused":0' "$fixture/front-literal.log"
grep -Eq '"front_reused":1' "$fixture/front-literal.log"
execute 41
cp "$fixture/source/main.lkj" "$fixture/front-valid-source"
cp "$fixture/front-program" "$fixture/front-valid-program"
perl - "$fixture/source/main.lkj" <<'SOURCE_EDIT'
use strict;
use warnings;
my $path = $ARGV[0];
open my $input, '<:raw', $path or die $!;
local $/;
my $text = <$input>;
close $input;
my $count = ($text =~ s/where 'long: 'short \{ first \}/where 'long: 'short { other }/g);
die "expected one same-width provenance change\n" unless $count == 1;
open my $output, '>:raw', $path or die $!;
print {$output} $text;
close $output or die $!;
SOURCE_EDIT
if checkpoint front-invalid; then
    echo 'cached lifetime-only definition accepted an unrelated return origin' >&2
    exit 1
fi
grep -Fq 'E_BORROW' "$fixture/front-invalid.log"
cmp "$fixture/front-valid-program" "$fixture/front-program"
cp "$fixture/front-valid-source" "$fixture/source/main.lkj"
checkpoint front-restored
grep -Eq '"build_reused":1' "$fixture/front-restored.log"
execute 41
printf 'ok outlives where: exact lifetime identities, mixed const qualification/type/const meaning, checked edits and rollback, warm images, frontend provenance rejection and both engines\n'
