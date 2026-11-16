#!/usr/bin/env bash
# Checked external-module publication: absence, pinned leaves and authenticated recovery.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
fixture=$(mktemp -d "$PWD/target/compiler-structure-create.XXXXXX")
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if ((code)); then
        for output in "$fixture/plan" "$fixture/error"; do
            if [[ -f "$output" ]]; then cat "$output" >&2; fi
        done
    fi
    rm -rf "$fixture"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
project="$fixture/source"
head -c 32 /dev/urandom > "$fixture/key"
common=(main.lkj --source-dir "$project" --no-library --no-store)
reset() {
    rm -rf "$project"
    mkdir "$project"
    printf '%s\n' 'pub fn main() -> i64 { 0i64 }' > "$project/main.lkj"
}
snapshot() {
    (
        cd "$project"
        find . -name '*.lkj' -printf '%y %P %l\n'
        find . -type f -name '*.lkj' -exec sha256sum {} +
    ) | LC_ALL=C sort > "$1"
}
request() { printf '{"version":1,"operations":[%s]}\n' "$1" > "$fixture/request.json"; }
refuse() {
    local expected=$1 status=0
    snapshot "$fixture/before"
    timeout --kill-after=5s 30s "$compiler" edit "${common[@]}" --store-key "$fixture/key" \
        --request "$fixture/request.json" > "$fixture/plan" 2> "$fixture/error" || status=$?
    [[ "$status" == 1 ]]
    [[ ! -s "$fixture/plan" ]]
    [[ -z "$expected" ]] || grep -Fq "$expected" "$fixture/error"
    snapshot "$fixture/after"
    cmp "$fixture/before" "$fixture/after"
    [[ ! -e "$project/.lkjscript-edit.pending" ]]
}
apply() {
    "$compiler" edit "${common[@]}" --store-key "$fixture/key" \
        --request "$fixture/request.json" > "$fixture/plan" 2> "$fixture/error"
    grep -q 'mode=apply' "$fixture/plan"
    [[ ! -e "$project/.lkjscript-edit.pending" ]]
    [[ -z "$(find "$project" -name '*.lkjscript-next' -print)" ]]
}
projection='{"op":"add-item","id":"crate","source":"pub mod added;","projection":{"path":"added.lkj","source":"pub fn value() -> i64 { 42i64 }"}}'
reset
request "$projection,"'{"op":"replace-body","id":"main","source":"{ added::value() }"}'
snapshot "$fixture/before"
"$compiler" edit "${common[@]}" --request "$fixture/request.json" --dry-run > "$fixture/plan"
grep -q 'before=absent' "$fixture/plan"
snapshot "$fixture/after"
cmp "$fixture/before" "$fixture/after"
[[ ! -e "$project/added.lkj" && ! -e "$project/.lkjscript-edit.lock" ]]
[[ -z "$(find "$project" \( -name '.lkjscript-edit*' -o -name '*.lkjscript-next' \) -print)" ]]
[[ ! -e "$project/.lkjscript-edit.pending" && ! -e "$project/.lkjscript-edit.generation" ]]
apply
printf '%s' 'pub fn value() -> i64 { 42i64 }' > "$fixture/expected"
cmp "$fixture/expected" "$project/added.lkj"
[[ "$("$compiler" run "${common[@]}")" == 42 ]]

# An empty new file is a real projection, distinct from an occupied empty file.
reset
request '{"op":"add-item","id":"crate","source":"mod added;","projection":{"path":"added.lkj","source":""}}'
apply
[[ -f "$project/added.lkj" && ! -s "$project/added.lkj" ]]
reset
: > "$project/added.lkj"
refuse E_EDIT_EXISTS

# Every occupied basename refuses. In particular no data open may hang on a FIFO.
for occupied in regular directory link dangling fifo hardlink; do
    reset
    request "$projection"
    printf '%s\n' 'outside sentinel' > "$fixture/sentinel"
    case "$occupied" in
        regular) printf '%s\n' 'occupied' > "$project/added.lkj";;
        directory) mkdir "$project/added.lkj";;
        link) ln -s "$fixture/sentinel" "$project/added.lkj";;
        dangling) ln -s absent-target "$project/added.lkj";;
        fifo) mkfifo "$project/added.lkj";;
        hardlink) ln "$fixture/sentinel" "$project/added.lkj";;
    esac
    cp "$fixture/sentinel" "$fixture/sentinel-before"
    refuse ''
    cmp "$fixture/sentinel-before" "$fixture/sentinel"
done
reset
request '{"op":"add-item","id":"crate","source":"mod added;","projection":{"path":"added/mod.lkj","source":""}}'
refuse E_EDIT_PARENT
[[ ! -e "$project/added" ]]
reset
mkdir "$project/added"
printf '%s\n' 'pub fn existing() {}' > "$project/added/mod.lkj"
request "$projection"
refuse E_EDIT_EXISTS
reset
request '{"op":"add-item","id":"crate","source":"mod added;","projection":{"path":"added.lkj","source":"fn unused() { missing(); }"}}'
refuse E_EDIT_CHECK
reset
request '{"op":"add-item","id":"crate","source":"mod added;","projection":{"path":"unrelated.lkj","source":""}}'
refuse E_EDIT_PROJECTION

# Held directory identities distinguish real parents from aliased spellings.
# Equal basenames in distinct parents succeed; one physical leaf cannot be two changes.
for parents in distinct alias; do
    reset
    printf '%s\n' 'mod one {} mod two {} pub fn main() -> i64 { 0i64 }' > "$project/main.lkj"
    mkdir "$project/one"
    if [[ "$parents" == distinct ]]; then mkdir "$project/two"; else ln -s one "$project/two"; fi
    request '{"op":"add-item","id":"one","source":"pub mod added;","projection":{"path":"one/added.lkj","source":""}},{"op":"add-item","id":"two","source":"pub mod added;","projection":{"path":"two/added.lkj","source":""}}'
    if [[ "$parents" == distinct ]]; then
        apply
        [[ -f "$project/one/added.lkj" && -f "$project/two/added.lkj" ]]
    else
        refuse E_EDIT_PARENT_ALIAS
    fi
done

# Candidate inputs cannot include bytes that this same publication will replace
# as private writer metadata or use as a source staging slot. Exact spellings
# are refused too, rather than being mistaken for projected source overlays.
for reserved in .lkjscript-edit.generation .main.lkj.lkjscript-next; do
    reset
    printf '%064d' 0 > "$project/$reserved"
    printf 'pub fn main() -> i64 { let _ = include_bytes!("%s"); 0i64 }\n' "$reserved" > "$project/main.lkj"
    cp "$project/$reserved" "$fixture/reserved-before"
    reserved_inode=$(stat -c %i "$project/$reserved")
    request '{"op":"replace-body","id":"main","source":"{ 1i64 }"}'
    refuse E_EDIT_RESERVED_INPUT
    cmp "$fixture/reserved-before" "$project/$reserved"
    [[ "$(stat -c %i "$project/$reserved")" == "$reserved_inode" ]]
done

# Metadata observation itself is a regular, read-only open. A planted special
# leaf must refuse a key-free dry run without creating a lock or touching it.
for metadata in .lkjscript-edit.pending .lkjscript-edit.generation; do
    for occupied in dangling fifo directory; do
        reset
        case "$occupied" in
            dangling) ln -s missing "$project/$metadata";;
            fifo) mkfifo "$project/$metadata";;
            directory) mkdir "$project/$metadata";;
        esac
        metadata_inode=$(stat -c %i "$project/$metadata")
        request '{"op":"replace-body","id":"main","source":"{ 1i64 }"}'
        snapshot "$fixture/before"
        status=0
        timeout --kill-after=5s 30s "$compiler" edit "${common[@]}" --request "$fixture/request.json" --dry-run > "$fixture/plan" 2> "$fixture/error" || status=$?
        [[ "$status" == 1 && ! -s "$fixture/plan" ]]
        grep -q E_EDIT_IO "$fixture/error"
        snapshot "$fixture/after"
        cmp "$fixture/before" "$fixture/after"
        [[ "$(stat -c %i "$project/$metadata")" == "$metadata_inode" ]]
        [[ ! -e "$project/.lkjscript-edit.lock" ]]
        [[ -z "$(find "$project" -name '*.lkjscript-next' -print)" ]]
    done
done

# Without a committed intent, recovery knows only the two root metadata slots.
# It must not sweep unlisted per-source stages or alter source bytes.
reset
printf '%s' 'partial intent' > "$project/..lkjscript-edit.pending.lkjscript-next"
printf '%s' 'partial generation' > "$project/..lkjscript-edit.generation.lkjscript-next"
printf '%s' 'unlisted stage sentinel' > "$project/.main.lkj.lkjscript-next"
cp "$project/.main.lkj.lkjscript-next" "$fixture/unlisted-before"
snapshot "$fixture/before"
"$compiler" edit "${common[@]}" --store-key "$fixture/key" --recover > "$fixture/plan"
snapshot "$fixture/after"
cmp "$fixture/before" "$fixture/after"
cmp "$fixture/unlisted-before" "$project/.main.lkj.lkjscript-next"
[[ ! -e "$project/..lkjscript-edit.pending.lkjscript-next" ]]
[[ ! -e "$project/..lkjscript-edit.generation.lkjscript-next" ]]
[[ ! -e "$project/.lkjscript-edit.pending" && ! -e "$project/.lkjscript-edit.generation" ]]

# Construct an independently encoded, correctly authenticated journal. This
# helper never claims IR admission: it exercises the text writer's wire contract.
intent() {
    perl - "$project" "$fixture/key" "$1" <<'PERL'
use strict; use warnings;
use Digest::SHA qw(sha256 sha256_hex hmac_sha256);
use JSON::PP;
my ($root,$keyfile,$case)=@ARGV;
open my $key,'<:raw',$keyfile or die $!; local $/; my $secret=<$key>; close $key;
die "key size" unless length($secret)==32;
my $old="pub fn main() -> i64 { 0i64 }\n";
my $new="mod created;\npub fn main() -> i64 { created::value() }\n";
my @changes=(
    {path=>'main.lkj',before=>$old,after=>$new},
    {path=>'created.lkj',before=>undef,after=>'pub fn value() -> i64 { 42i64 }'},
);
my $version=2;
if ($case eq 'v1') {
    $version=1; @changes=({path=>'main.lkj',before=>$old,after=>"pub fn main() -> i64 { 1i64 }\n"});
} elsif ($case eq 'missing') {
    delete $changes[1]->{before};
} elsif ($case eq 'present-empty') {
    $changes[1]->{before}='';
} elsif ($case eq 'bad-path') {
    $changes[1]->{path}='../outside.lkj';
} elsif ($case ne 'normal') { die "unknown case"; }
my $payload=JSON::PP->new->utf8->canonical->encode({
    version=>$version,root=>$root,generation=>('a'x64),changes=>\@changes,
});
my $digest=sha256($payload);
sub field { return pack('Q<',length($_[0])).$_[0]; }
my $mac_input=field('lkjscript-authenticated-store-root-2').
    field(sha256('lkjscript-source-transaction-writer-1')).
    field('linux-x86_64').field('source-transaction').field(sha256_hex($root)).
    pack('Q<',length($payload)).field($digest);
my $envelope="lkjscript-store-2\n".pack('Q<',length($payload)).$digest.
    hmac_sha256($mac_input,$secret).$payload;
open my $out,'>:raw',"$root/.lkjscript-edit.pending" or die $!;
print {$out} $envelope; close $out or die $!;
PERL
}
recover_refusal() {
    local expected=$1 status=0
    snapshot "$fixture/before"
    timeout --kill-after=5s 30s "$compiler" edit "${common[@]}" --store-key "$fixture/key" \
        --recover > "$fixture/plan" 2> "$fixture/error" || status=$?
    [[ "$status" == 1 && ! -s "$fixture/plan" ]]
    grep -Fq "$expected" "$fixture/error"
    snapshot "$fixture/after"
    cmp "$fixture/before" "$fixture/after"
    [[ -f "$project/.lkjscript-edit.pending" && ! -e "$project/.lkjscript-edit.generation" ]]
    [[ -z "$(find "$project" -name '*.lkjscript-next' -print)" ]]
}
for malformed in v1 missing present-empty bad-path; do
    reset
    intent "$malformed"
    case "$malformed" in
        v1) recover_refusal 'unsupported intent version';;
        missing) recover_refusal 'missing old source state';;
        present-empty) recover_refusal E_EDIT_STALE;;
        bad-path) recover_refusal E_EDIT_JOURNAL;;
    esac
done
for collision in empty wrong link dangling fifo; do
    reset
    intent normal
    case "$collision" in
        empty) : > "$project/created.lkj";;
        wrong) printf '%s' 'hostile' > "$project/created.lkj";;
        link) printf '%s' 'pub fn value() -> i64 { 42i64 }' > "$fixture/sentinel"; ln -s "$fixture/sentinel" "$project/created.lkj";;
        dangling) ln -s missing "$project/created.lkj";;
        fifo) mkfifo "$project/created.lkj";;
    esac
    recover_refusal E_EDIT_
done
# Absence and exact already-published new bytes are both valid recovery states.
for current in absent new; do
    reset
    intent normal
    if [[ "$current" == new ]]; then
        printf '%s' 'pub fn value() -> i64 { 42i64 }' > "$project/created.lkj"
        inode=$(stat -c %i "$project/created.lkj")
    fi
    "$compiler" edit "${common[@]}" --store-key "$fixture/key" --recover > "$fixture/plan"
    [[ ! -e "$project/.lkjscript-edit.pending" ]]
    [[ -z "$(find "$project" -name '*.lkjscript-next' -print)" ]]
    if [[ "$current" == new ]]; then [[ "$(stat -c %i "$project/created.lkj")" == "$inode" ]]; fi
    [[ "$("$compiler" run "${common[@]}")" == 42 ]]
done
printf '%s\n' 'ok checked module publication: explicit absence, safe occupied leaves, parent fences, v2 journal refusals and exact recovery'
