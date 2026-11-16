#!/usr/bin/env bash
# The historical reader cannot acquire compiler-proof APIs or writer authority.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
fixture=$(mktemp -d target/compiler-query-store-test.XXXXXX)
fixture="$PWD/$fixture"
finish() {
    local status=$?
    trap - EXIT HUP INT TERM
    if [[ -n ${LKJSCRIPT_QUERY_STORE_EVIDENCE:-} ]]; then
        mkdir -p "$LKJSCRIPT_QUERY_STORE_EVIDENCE"
        while IFS= read -r -d '' record; do
            case $(basename "$record") in key|producer-a|reader-b) continue ;; esac
            if [[ ! -s $record ]] || LC_ALL=C grep -Iq '' "$record"; then
                cp "$record" "$LKJSCRIPT_QUERY_STORE_EVIDENCE/"
            fi
        done < <(find "$fixture" -maxdepth 1 -type f -print0)
    fi
    rm -rf "$fixture"
    exit "$status"
}
trap finish EXIT
trap 'exit 130' HUP INT TERM
mkdir "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
mkdir "$fixture/build-source"
cp src/query_store_cases.lkj src/diagnostic.lkj src/ir.lkj src/ir_binary.lkj src/reuse.lkj src/store.lkj "$fixture/build-source/"
"$compiler" build query_store_cases.lkj --source-dir "$fixture/build-source" --no-store --library-dir library \
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
# Reopening the signed budget preserves complete containers, locators and proofs.
"$fixture/producer-a" touch "${common[@]}" > "$fixture/touch"
accounted "$(value bytes "$fixture/touch")"
# Retire only canonical v2 semantic families from an otherwise valid signed
# Budget v2. Recognizing its legacy locator row must not invalidate the complete
# ledger and erase ordinary proofs or the new-format generation.
container=$(find "$fixture/store" -maxdepth 1 -type f -name "semantic-container-v3-*-$identity" -print -quit)
cp "$container" "$fixture/migration-container-before"
cp "$fixture/store/semantic-locator-v3-$identity" "$fixture/migration-locator-before"
proof=$(find "$fixture/store" -maxdepth 1 -type f -name 'check-*' -print -quit)
cp "$proof" "$fixture/migration-proof-before"
perl - "$fixture/store" "$fixture/key" "$producer" "$selector" > "$fixture/legacy-fixture" <<'PERL'
use strict;
use warnings;
use Digest::SHA qw(sha256 sha256_hex hmac_sha256);
my ($directory, $key_path, $producer_hex, $selector) = @ARGV;
sub read_file { open my $f, '<:raw', $_[0] or die "$_[0]: $!"; local $/; return <$f>; }
sub write_file { open my $f, '>:raw', $_[0] or die "$_[0]: $!"; print {$f} $_[1] or die $!; close $f or die $!; }
my $secret = read_file($key_path); die "key size\n" unless length($secret) == 32;
my $producer = pack('H*', $producer_hex);
my $slot = sha256_hex($producer);
my $magic = "lkjscript-store-2\n";
sub word { return pack('Q<', $_[0]); }
sub field { return word(length($_[0])) . $_[0]; }
sub tag {
    my ($owner, $namespace, $key, $length, $digest) = @_;
    return hmac_sha256(field('lkjscript-authenticated-store-root-2') . field($owner)
        . field('linux-x86_64') . field($namespace) . field($key) . word($length) . field($digest), $secret);
}
sub seal {
    my ($owner, $namespace, $key, $payload) = @_;
    my $digest = sha256($payload);
    return $magic . word(length($payload)) . $digest . tag($owner, $namespace, $key, length($payload), $digest) . $payload;
}
my $budget_owner = sha256('lkjscript-store-size-index-1');
my $budget = read_file("$directory/.budget-index");
die "budget magic\n" unless substr($budget, 0, 18) eq $magic;
my $size = unpack('Q<', substr($budget, 18, 8));
my $payload = substr($budget, 90);
die "budget length/digest\n" unless length($payload) == $size && substr($budget, 26, 32) eq sha256($payload);
die "budget MAC\n" unless substr($budget, 58, 32) eq tag($budget_owner, 'size-index', '1', $size, sha256($payload));
my $at = 0;
sub take_word { die "word overrun\n" if length($payload) - $at < 8; my $v = unpack('Q<', substr($payload, $at, 8)); $at += 8; return $v; }
sub take_field { my $n = take_word(); die "field overrun\n" if $n > length($payload) - $at; my $v = substr($payload, $at, $n); $at += $n; return $v; }
die "budget version\n" unless take_word() == 2;
my @generations; my $count = take_word(); push @generations, take_field() for 1..$count;
die "producer generation absent\n" unless grep { $_ eq $slot } @generations;
my %rows; $count = take_word();
for (1..$count) { my $name = take_field(); my $owner = take_field(); my $bytes = take_word(); die "duplicate row\n" if exists $rows{$name}; $rows{$name} = [$owner, $bytes]; }
die "budget tail\n" unless $at == length($payload);
my $page = 'old semantic page';
my $root = sha256_hex($page);
my $snapshot = word(2) . field($producer_hex) . field(sha256_hex('old source'))
    . field(sha256_hex('old IR')) . field($root) . field($selector) . word(0);
my $id = sha256_hex($snapshot);
my @legacy;
for my $entry (
    ['semantic-page', $root, $page],
    ['semantic-root', $id, $snapshot],
    ['semantic-latest', $selector, $snapshot],
) {
    my ($namespace, $key, $body) = @$entry;
    my $name = "$namespace-$slot-$key";
    my $bytes = seal($producer, $namespace, $key, $body);
    die "fixture collision\n" if -e "$directory/$name";
    write_file("$directory/$name", $bytes); $rows{$name} = [$slot, length($bytes)]; push @legacy, $name;
}
my $locator = "semantic-locator-$id";
my $locator_payload = word(2) . $producer . pack('H*', $selector) . pack('H*', $root);
my $locator_bytes = seal(sha256('lkjscript-semantic-locator-2'), 'semantic-locator', $id, $locator_payload);
die "legacy locator size\n" unless length($locator_bytes) == 194;
write_file("$directory/$locator", $locator_bytes); $rows{$locator} = [$slot, length($locator_bytes)]; push @legacy, $locator;
my $similar_key = sha256_hex('similar prefix');
my $similar = "semantic-latest-extra-$slot-$similar_key";
my $similar_bytes = seal($producer, 'semantic-latest-extra', $similar_key, 'preserve exact namespace');
write_file("$directory/$similar", $similar_bytes); $rows{$similar} = [$slot, length($similar_bytes)];
my $new = word(2) . word(scalar @generations) . join('', map { field($_) } @generations) . word(scalar keys %rows);
for my $name (sort keys %rows) { $new .= field($name) . field($rows{$name}[0]) . word($rows{$name}[1]); }
write_file("$directory/.budget-index", seal($budget_owner, 'size-index', '1', $new));
print "id=$id root=$root similar=$similar similar-sha=", sha256_hex($similar_bytes), "\n";
print "legacy=$_\n" for @legacy;
PERL
legacy_id=$(value id "$fixture/legacy-fixture")
legacy_root=$(value root "$fixture/legacy-fixture")
refuse "$fixture/reader-b" read "$fixture/store" "$fixture/key" "$legacy_id" "$selector" "$legacy_root" "$producer"
"$fixture/producer-a" touch "$fixture/store" "$fixture/key" "$identity" "$selector" "$root" "$producer" > "$fixture/migration-touch"
while IFS= read -r name; do [[ ! -e "$fixture/store/$name" ]] || fail "legacy entry retained: $name"; done < <(sed -n 's/^legacy=//p' "$fixture/legacy-fixture")
similar=$(value similar "$fixture/legacy-fixture")
[[ $(sha256sum "$fixture/store/$similar" | cut -d' ' -f1) == "$(value similar-sha "$fixture/legacy-fixture")" ]] || fail 'similar namespace was retired'
cmp "$container" "$fixture/migration-container-before"
cmp "$fixture/store/semantic-locator-v3-$identity" "$fixture/migration-locator-before"
cmp "$proof" "$fixture/migration-proof-before"
accounted "$(value bytes "$fixture/migration-touch")"
cp -a "$fixture/store" "$fixture/saved-store"
# The pinned reader removes only its fixed root and locator. It never rewrites
# accounting or erases a newer latest pointer / unrelated admission record.
"$fixture/producer-a" publish "$fixture/store" "$fixture/key" second > "$fixture/second"
next_identity=$(value id "$fixture/second")
next_root=$(value root "$fixture/second")
[[ "$identity" != "$next_identity" ]] || fail 'fixture snapshots coincide'
latest=$(find "$fixture/store" -maxdepth 1 -type f -name 'semantic-latest-v3-*' -print -quit)
proof=$(find "$fixture/store" -maxdepth 1 -type f -name 'check-*' -print -quit)
cp "$latest" "$fixture/latest-before"
cp "$proof" "$fixture/proof-before"
cp "$fixture/store/.budget-index" "$fixture/budget-before"
"$fixture/reader-b" invalidate "${common[@]}" > "$fixture/invalidate"
[[ ! -e "$fixture/store/semantic-locator-v3-$identity" ]] || fail 'old locator survived invalidation'
[[ -z "$(find "$fixture/store" -maxdepth 1 -name "semantic-container-v3-*-$identity" -print -quit)" ]] || fail 'old root survived invalidation'
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
[[ ! -e "$fixture/store/semantic-locator-v3-$identity" ]] || fail 'locator escaped producer-generation eviction'
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
# Total container size is not the historical reader's allocation cap. It opens
# a fixed manifest and independently bounded node even when total bytes exceed it.
mkdir "$fixture/large-store"
"$fixture/producer-a" publish "$fixture/large-store" "$fixture/key" larger-than-reader-cap > "$fixture/large"
[[ $(value total "$fixture/large") -gt 8192 ]] || fail 'large-container control is too small'
"$fixture/reader-b" read "$fixture/large-store" "$fixture/key" \
    "$(value id "$fixture/large")" "$(value selector "$fixture/large")" \
    "$(value root "$fixture/large")" "$(value producer "$fixture/large")" > "$fixture/large-read"
# Sequential controlled I/O refusal: the first generation row is removed, then
# a directory occupying its second row makes eviction fail. Both publication
# paths must retain that poisoned state even after the first deletion freed room.
for mode in ordinary staged; do
    mkdir "$fixture/poison-$mode"
    "$fixture/producer-a" "poison-$mode" "$fixture/poison-$mode" "$fixture/key" > "$fixture/poison-$mode.out"
    [[ -z $(find "$fixture/poison-$mode" -maxdepth 1 \
        \( -name 'c-poison-*' -o -name 'd-poison-*' -o -name 'semantic-*' -o -name '.tmp-*' \) -print -quit) ]] || fail "poisoned $mode writer published or retained a stage"
done
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
