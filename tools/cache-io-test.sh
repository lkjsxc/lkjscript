#!/usr/bin/env bash
# Counter semantics against actual owned envelopes; not a throughput benchmark.
set -euo pipefail
cd "$(dirname "$0")/.."
(($# == 1)) || { echo 'usage: cache-io-test.sh COMPILER' >&2; exit 2; }
compiler=$(realpath "$1")
fixture=$(mktemp -d target/compiler-cache-io-test.XXXXXX)
fixture=$(realpath "$fixture")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status != 0)); then
        for log in "$fixture"/*.stderr; do
            [[ -f $log ]] || continue
            printf '%s\n' "$log" >&2
            tail -12 "$log" >&2
        done
    fi
    rm -rf -- "$fixture"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
umask 077
mkdir "$fixture/source" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
printf 'mod dependency;\nfn main() -> i64 { dependency::answer() }\n' > "$fixture/source/main.lkj"
printf 'pub fn answer() -> i64 { 40 }\n' > "$fixture/source/dependency.lkj"
run() {
    local phase=$1; shift
    "$compiler" "$@" > "$fixture/$phase.stdout" 2> "$fixture/$phase.stderr"
}
cached() {
    local phase=$1; shift
    run "$phase" build main.lkj --source-dir "$fixture/source" --store "$fixture/store" \
        --store-key "$fixture/key" --inline-limit 0 -o "$fixture/image" "$@"
}
mkdir "$fixture/timing"
cp src/diagnostic.lkj src/ir.lkj src/ir_binary.lkj src/reuse.lkj src/store.lkj "$fixture/timing/"
cat > "$fixture/timing/main.lkj" <<'SOURCE'
mod diagnostic;
mod ir;
mod ir_binary;
mod reuse;
mod store;
pub use std::sha256;
fn main() { store::io_timing_contracts(); }
SOURCE
run timing-native run main.lkj --source-dir "$fixture/timing" --no-store --engine native
run timing-interpreter run main.lkj --source-dir "$fixture/timing" --no-store --engine interpreter
cmp "$fixture/timing-native.stdout" "$fixture/timing-interpreter.stdout"
run plain build main.lkj --source-dir "$fixture/source" --no-store --inline-limit 0 -o "$fixture/expected"
[[ ! -s $fixture/plain.stdout && ! -s $fixture/plain.stderr ]]
cached cold --stats
cmp "$fixture/expected" "$fixture/image"
find "$fixture/store" -maxdepth 1 -type f ! -name '.budget-lock' -printf '%f\t%s\n' | sort > "$fixture/cold.files"
cached unchanged --stats
cmp "$fixture/expected" "$fixture/image"
cached quiet
cmp "$fixture/expected" "$fixture/image"
[[ ! -s $fixture/quiet.stdout && ! -s $fixture/quiet.stderr ]]
cat > "$fixture/body.json" <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"dependency::answer","source":"{ 41 }"}]}
JSON
run edit edit main.lkj --source-dir "$fixture/source" --no-store --store-key "$fixture/key" --request "$fixture/body.json"
cached body --stats
cp "$fixture/image" "$fixture/body-image"
run control build main.lkj --source-dir "$fixture/source" --no-store --inline-limit 0 -o "$fixture/image"
cmp "$fixture/body-image" "$fixture/image"
if cmp -s "$fixture/expected" "$fixture/image"; then echo 'body change left image identical' >&2; exit 1; fi
find "$fixture/store" -maxdepth 1 -type f ! -name '.budget-lock' -printf '%f\t%s\n' | sort > "$fixture/body.files"
perl -MJSON::PP - "$fixture" <<'PERL'
use strict;
use warnings;
my ($fixture) = @ARGV;
my $header = length("lkjscript-store-2\n") + 8 + 64;
my %allowed = map { $_ => 1 } qw(front native check image inputs-image index inputs-index test inputs-test metadata other);
sub read_rows {
    my ($phase) = @_;
    open my $in, '<', "$fixture/$phase.stderr" or die $!;
    my (%rows, $stats, $summary);
    while (<$in>) {
        if (s/^LKJSCRIPT_STATS //) { die "duplicate stats\n" if $stats; $stats = decode_json($_); }
        elsif (s/^LKJSCRIPT_STORE_IO //) {
            my $row = decode_json($_);
            my $category = $row->{category};
            if ($category eq 'summary') {
                die "duplicate summary\n" if $summary;
                $summary = $row;
                for my $field (qw(global_timing_failures global_counter_overflow pending_checkpoint accounted_ns)) {
                    die "missing summary field $field\n" unless exists $row->{$field};
                }
                die "timing/counter/checkpoint failure\n" if $row->{global_timing_failures} ||
                    $row->{global_counter_overflow} || $row->{pending_checkpoint};
                next;
            }
            die "category\n" unless $allowed{$category} && !$rows{$category};
            for my $field (keys %$row) {
                next if $field eq 'category';
                die "noninteger $field\n" unless "$row->{$field}" =~ /^\d+$/;
            }
            die "read envelope identity\n" unless
                $row->{authenticated_envelope_bytes} ==
                $row->{authenticated_payload_bytes} + $header * $row->{authenticated_reads};
            die "write envelope identity\n" unless
                $row->{published_envelope_bytes} ==
                $row->{published_payload_bytes} + $header * $row->{published};
            $rows{$category} = $row;
        } else { die "unexpected diagnostic $_"; }
    }
    die "missing stats, summary or rows\n" unless $stats && $summary && keys %rows;
    my $sum = 0;
    for my $row (values %rows) {
        $sum += $row->{$_} for qw(read_ns pack_decode_ns pack_retain_ns seal_ns pack_reseal_ns publish_ns prepare_ns metadata_ns);
    }
    die "phase accounting mismatch $phase\n" unless $sum == $summary->{accounted_ns};
    die "overlapping stage totals $phase\n" if $sum > $stats->{store_io_ms} * 1_000_000;
    return (\%rows, $stats);
}
sub category {
    my ($name) = @_;
    return 'metadata' if $name eq '.budget-index';
    return 'native' if $name =~ /^native-/;
    return $1 if $name =~ /^(inputs-image|front|check|image)-/;
    die "unexpected owned envelope $name\n";
}
sub files {
    my ($phase) = @_;
    open my $in, '<', "$fixture/$phase.files" or die $!;
    my %sizes;
    while (<$in>) {
        chomp;
        my ($name, $size) = split /\t/;
        my $category = category($name);
        $sizes{$category}{envelope} += $size;
        $sizes{$category}{payload} += $size - $header;
        $sizes{$category}{count}++;
    }
    return \%sizes;
}
my ($cold, $cold_stats) = read_rows('cold');
my $cold_files = files('cold');
for my $category (keys %$cold_files) {
    my ($row, $file) = ($cold->{$category}, $cold_files->{$category});
    die "missing cold category $category\n" unless $row;
    die "cold publication bytes $category\n" unless $row->{published_envelope_bytes} == $file->{envelope};
    die "cold publication count $category\n" unless $row->{published} == $file->{count};
}
my ($warm, $warm_stats) = read_rows('unchanged');
for my $category (qw(image inputs-image)) {
    die "warm read bytes $category\n" unless
        $warm->{$category}{authenticated_envelope_bytes} == $cold_files->{$category}{envelope};
}
die "warm image missing\n" unless $warm_stats->{build_reused} == 1;
my ($body) = read_rows('body');
for my $category (qw(check native)) {
    die "pack not loaded $category\n" unless $body->{$category}{pack_loads} > 0;
    die "pack decode bytes $category\n" unless
        $body->{$category}{pack_decode_bytes} == $cold_files->{$category}{payload};
}
die "native pack not resealed\n" unless $body->{native}{pack_reseals} > 0;
die "native pack reseal bytes\n" unless
    $body->{native}{pack_reseal_payload_bytes} == $body->{native}{published_payload_bytes};
die "front checkpoint absent\n" unless
    $body->{front}{authenticated_reads} > 0 && $body->{front}{published} > 0;
my $published = 0;
for my $category (keys %$cold) {
    $published += $cold->{$category}{published} unless $category eq 'metadata';
}
die "legacy write count changed\n" unless $published == $cold_stats->{store_writes};
print "cache I/O: exact cold envelope bytes, warm reads, pack loads/reseal, disjoint durations and quiet default agree\n";
PERL
