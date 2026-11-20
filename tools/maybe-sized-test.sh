#!/usr/bin/env bash
# Relaxed Sized contracts retain source meaning through checked edits and caches.
set -euo pipefail
export LC_ALL=C
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
mkdir -p target
fixture=$(mktemp -d "$PWD/target/maybe-sized.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)); then
        [[ ! -f "$fixture/source/main.lkj" ]] || cat "$fixture/source/main.lkj" >&2
        for file in "$fixture"/*.log "$fixture"/*.query; do
            [[ ! -f "$file" ]] || cat "$file" >&2
        done
    fi
    if [[ ${LKJSCRIPT_MAYBE_SIZED_KEEP:-0} == 1 ]]; then
        printf 'retained_fixture=%s\n' "$fixture" >&2
    else
        rm -rf -- "$fixture"
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
umask 077
mkdir "$fixture/source" "$fixture/store" "$fixture/front-store"
head -c 32 /dev/urandom > "$fixture/key"
fail() { printf 'maybe-sized: %s\n' "$*" >&2; exit 1; }
cat > "$fixture/source/main.lkj" <<'SOURCE'
trait Value { fn value(&self) -> i64; }
impl Value for str { fn value(&self) -> i64 { self.len() as i64 } }
impl Value for i64 { fn value(&self) -> i64 { *self } }
fn read<T: ?Sized + Value>(input: &T) -> i64 { input.value() }
fn relay<T: Value>(input: &T) -> i64 { read(input) }
fn main() -> i64 { read::<str>("abc") + relay(&37i64) }
SOURCE
common=(main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/store" --store-key "$fixture/key")
publish() {
    "$compiler" build "${common[@]}" --index --stats -o "$fixture/program" > "$fixture/$1.log" 2>&1
    snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/$1.log")
    [[ ${#snapshot} == 64 ]] || fail 'missing indexed snapshot'
}
# Full reference capture skips frontend checkpoints. This store deliberately
# builds without --index so the bound-change invalidation exercises that route.
checkpoint() {
    "$compiler" build main.lkj --source-dir "$fixture/source" --no-library \
        --store "$fixture/front-store" --store-key "$fixture/key" --stats \
        -o "$fixture/front-program" > "$fixture/$1.log" 2>&1
}
query() {
    "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$1" --id "$2" --limit 128 --bytes 65536
}
meaning() {
    local phase=$1 trait=$2
    query signature read > "$fixture/$phase-read-signature.query"
    grep -Eq "read<T: \\?Sized \\+ (crate::)?$trait>" "$fixture/$phase-read-signature.query" \
        || fail 'function signature lost its relaxed bound'
    query signature read::generic:T > "$fixture/$phase-generic-signature.query"
    grep -Eq "T: \\?Sized \\+ (crate::)?$trait" "$fixture/$phase-generic-signature.query" \
        || fail 'named parameter lost its relaxed bound'
    query type read::param:input > "$fixture/$phase-input-type.query"
    grep -F 'fact=declared-type' "$fixture/$phase-input-type.query" | grep -Fq 'value="&T"' \
        || fail 'borrowed source parameter lost its named type'
}
bound_sites() {
    local owner=$1 target=$2 spelling=$3 expected=$4 phase=$5
    query references "$owner" > "$fixture/$phase-references.query"
    perl -MJSON::PP - "$fixture/$phase-references.query" "$target" "$spelling" "$expected" "$fixture/source" <<'PERL'
use strict;
use warnings;
my ($facts,$wanted,$spelling,$expected,$root)=@ARGV;
$wanted = "type\@$wanted";
open my $input,'<',$facts or die $!;
my $count=0;
while (<$input>) {
    next unless /^("(?:\\.|[^"])*")\s+level=source-claim\s+fact=references\s+from=("(?:\\.|[^"])*")\s+file=("(?:\\.|[^"])*")\s+start=(\d+)\s+end=(\d+)\s+kind=(\S+)/;
    my ($target,$file,$start,$end,$kind)=(decode_json($1),decode_json($3),$4,$5,$6);
    next unless $target eq $wanted && $kind eq 'type';
    $file="$root/$file" unless $file =~ m{^/};
    open my $source,'<:raw',$file or die "$file: $!";
    local $/;
    my $text=<$source>;
    my $actual=substr($text,$start,$end-$start);
    die "$wanted span spells '$actual', expected '$spelling'\n" unless $actual eq $spelling;
    $count++;
}
die "$wanted has $count bound sites, expected $expected\n" unless $count==$expected;
PERL
}
edit() {
    "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/$1.log" 2>&1
    grep -Fq 'mode=apply' "$fixture/$1.log" || fail 'checked edit did not apply'
    [[ ! -e "$fixture/source/.lkjscript-edit.pending" ]] || fail 'edit left a pending transaction'
}
execute() {
    local expected=$1 actual engine
    for engine in native interpreter; do
        actual=$("$compiler" run "${common[@]}" --engine "$engine")
        [[ $actual == "$expected" ]] || fail "$engine returned $actual, expected $expected"
    done
}
publish cold
original_snapshot=$snapshot
meaning original Value
bound_sites read Value Value 1 original-read
bound_sites relay Value Value 1 original-relay
execute 40
publish warm
[[ $snapshot == "$original_snapshot" ]] || fail 'unchanged source changed its snapshot'
grep -Eq '"build_reused":1' "$fixture/warm.log" || fail 'unchanged indexed image was not reused'

printf '%s\n' '{"version":1,"operations":[{"op":"rename","id":"Value","name":"Readable"}]}' > "$fixture/request"
edit rename
publish renamed
[[ $snapshot != "$original_snapshot" ]] || fail 'trait rename kept the old snapshot'
meaning renamed Readable
bound_sites read Readable Readable 1 renamed-read
bound_sites read Value Value 0 old-trait
bound_sites relay Readable Readable 1 renamed-relay
execute 40
checkpoint before-bound
checkpoint before-bound-warm
grep -Eq '"build_reused":1' "$fixture/before-bound-warm.log" || fail 'nonindexed control did not reuse its image'

# The forwarding definition becomes valid for every borrowed Readable, including
# str. Moving the constraint into a where clause must retain its relaxation.
renamed_snapshot=$snapshot
printf '%s\n' '{"version":1,"operations":[{"op":"replace-item","id":"relay","source":"fn relay<T>(input: &T) -> i64 where T: ?Sized + Readable { read(input) }"}]}' > "$fixture/request"
edit relax-bound
checkpoint after-bound
grep -Eq '"build_reused":0' "$fixture/after-bound.log" || fail 'bound edit reused the old image'
grep -Eq '"front_reused":0' "$fixture/after-bound.log" || fail 'bound edit reused the old frontend checkpoint'
publish relaxed
[[ $snapshot != "$renamed_snapshot" ]] || fail 'bound edit kept the old snapshot'
query signature relay > "$fixture/relaxed-relay-signature.query"
grep -Fq 'where T: ?Sized + Readable' "$fixture/relaxed-relay-signature.query" \
    || fail 'where-clause signature lost its relaxed bound'
meaning relaxed Readable
bound_sites relay Readable Readable 1 relaxed-relay

printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"main","source":"{ read::<str>(\"hello\") + relay::<str>(\"rust\") + 33i64 }"}]}' > "$fixture/request"
edit unsized-call
publish changed
execute 42
accepted_snapshot=$snapshot
publish changed-warm
[[ $snapshot == "$accepted_snapshot" ]] || fail 'unchanged relaxed source changed its snapshot'
grep -Eq '"build_reused":1' "$fixture/changed-warm.log" || fail 'relaxed image was not reusable'

# An explicit Sized requirement would invalidate the existing str call. The
# transaction must preserve both source bytes and the previously valid image.
cp "$fixture/source/main.lkj" "$fixture/before-rejected"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-item","id":"relay","source":"fn relay<T: Sized + Readable>(input: &T) -> i64 { read(input) }"}]}' > "$fixture/request"
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rejected.log" 2>&1; then
    fail 'checked edit published an invalid Sized requirement'
fi
grep -Eq 'E_TYPE|E_TRAIT' "$fixture/rejected.log" || fail 'rejected bound edit omitted a type diagnostic'
grep -Fq 'Sized' "$fixture/rejected.log" || fail 'rejected bound edit omitted its Sized reason'
cmp "$fixture/before-rejected" "$fixture/source/main.lkj" || fail 'rejected edit changed source bytes'
[[ ! -e "$fixture/source/.lkjscript-edit.pending" ]] || fail 'rejected edit left a pending transaction'
publish rollback
[[ $snapshot == "$accepted_snapshot" ]] || fail 'rejected edit changed the admitted snapshot'
grep -Eq '"build_reused":1' "$fixture/rollback.log" || fail 'rejected edit lost the reusable valid image'
query signature relay > "$fixture/rollback-relay-signature.query"
grep -Fq 'where T: ?Sized + Readable' "$fixture/rollback-relay-signature.query" \
    || fail 'rollback lost the accepted relaxed contract'
execute 42
printf 'ok ?Sized: source and parameter meaning, exact bound references, checked rename/relaxation, rejected Sized rollback, frontend invalidation, warm images and both engines\n'
