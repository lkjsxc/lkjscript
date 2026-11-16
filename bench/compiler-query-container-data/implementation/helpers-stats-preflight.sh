#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
evidence="$PWD/target/compiler-container-helper-preflight-1"
stats() {
    local label=$1 expected=$2
    perl -MJSON::PP - "$evidence/$label.stderr" "$expected" > "$evidence/$label.stats-validation.json" <<'PERL'
use strict; use warnings;
my ($path,$expected)=@ARGV;
open my $in,'<',$path or die $!;
my ($stats,$summary,%rows);
while (<$in>) {
    if (s/^LKJSCRIPT_STATS //) { die "duplicate stats\n" if $stats; $stats=decode_json($_); }
    elsif (s/^LKJSCRIPT_STORE_IO //) {
        my $row=decode_json($_);
        my $category=$row->{category};
        if ($category eq 'summary') { die "duplicate summary\n" if $summary; $summary=$row; }
        else { die "duplicate category\n" if $rows{$category}; $rows{$category}=$row; }
    } else { die "unexpected stderr $_"; }
}
die "missing stats\n" unless $stats;
die "missing measured-build Store I/O summary/categories\n" if $expected eq 'build' && (!$summary || !keys(%rows));
if ($expected eq 'query') {
    die "pinned query current-producer hash\n" unless exists($stats->{producer_bytes_hashed}) && $stats->{producer_bytes_hashed}==0;
}
my $sum=0;
for my $row(values %rows) {
    $sum += $row->{$_}//0 for qw(read_ns pack_decode_ns pack_retain_ns seal_ns pack_reseal_ns publish_ns prepare_ns metadata_ns stream_write_ns);
}
if ($summary) {
    die "counter failure\n" if $summary->{global_timing_failures} || $summary->{global_counter_overflow} || $summary->{pending_checkpoint};
    die "exclusive counter sum\n" unless $sum==$summary->{accounted_ns};
}
print JSON::PP->new->canonical->encode({status=>'captured-stats-validated',stats=>$stats,
    exclusive_store_counter_sum_ns=>$sum,store_io_summary=>$summary,store_io_categories=>\%rows}),"\n";
PERL
}
stats baseline-index build
stats candidate-index build
for label in baseline-query-1 candidate-query-1 baseline-query-96 candidate-query-96; do stats "$label" query; done
printf "captured build summaries, exclusive sums and pinned zero-hash query stats validated\n"
