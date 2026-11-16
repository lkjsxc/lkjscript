#!/usr/bin/env perl
# Independently check the retained supplemental replay records; never runs a compiler.
use strict;
use warnings;
use JSON::PP;
use Digest::SHA qw(sha256_hex);

my $base = shift // die "usage: $0 REPORT_PREFIX\n";
@ARGV == 0 or die "unexpected arguments\n";
sub bytes {
    my ($path) = @_;
    open my $file, '<:raw', $path or die "$path: $!\n";
    local $/;
    return <$file> // '';
}
my $meta = bytes("$base.tsv.meta");
$meta =~ /^history_head=(a4fd685a9bb2e7736110a4ed2a43651dfe040d56)$/m
    or die "wrong frozen population\n";
$meta =~ /^4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09  /m
    or die "wrong frozen producer\n";
$meta =~ /^mode=replay root_policy=common-main-otherwise-common-lib scan=300 wanted=30 memory_bytes=4294967296 timeout_seconds=900 store_bytes=2147483648$/m
    or die "campaign policy changed\n";
$meta =~ /^retained_fixture=(.+)$/m or die "campaign has not retained its completed fixture\n";
my $before = "$1/before";
my $population_bytes = bytes("$base.revisions");
sha256_hex($population_bytes) eq 'dfe6dd8098ed931ce5ccd847cab1ab6411d769c6b4c22a80bc8682babbc0cc4b'
    or die "frozen revision-list hash differs\n";
my @population = grep { length } split /\n/, $population_bytes;
@population == 300 or die "wrong population size\n";
my %population;
for (@population) {
    my ($revision, $parent) = split / /;
    $population{$revision} = $parent // '';
}
my $ordered_bytes = bytes("$base.order");
sha256_hex($ordered_bytes) eq '1bc2a477d3655301f30edef8a3d18b468d3af2489b06d8879838ebc224bc89e4'
    or die "frozen candidate-order hash differs\n";
my @ordered = map { [split /\t/] } grep { length } split /\n/, $ordered_bytes;
for my $candidate (@ordered) {
    @$candidate == 3 or die "bad candidate record\n";
    $population{$candidate->[1]} eq $candidate->[2] or die "candidate outside frozen population\n";
}
my @records = split /\n/, bytes("$base.tsv");
my $header = shift @records;
my @head = split /\t/, $header;
@head == 16 && $head[-1] eq 'source_root' or die "wrong result schema\n";
my (%seen, %status, %reasons, $operations, $passed);
$operations = $passed = 0;
print join("\t", qw(ordinal revision parent operations changed_source_files desired_meaning candidate_meaning source_before source_after source_root library_environment stdout_sha256 stderr_sha256)), "\n";
for my $index (0 .. $#records) {
    my @row = split /\t/, $records[$index], -1;
    @row == 16 or die "bad result row " . ($index+1) . "\n";
    my ($ordinal, $revision, $parent, $status, $projection, $count) = @row;
    $ordinal == $index+1 or die "nonsequential ordinal\n";
    !$seen{$revision}++ or die "duplicate revision\n";
    my $candidate = $ordered[$index] // die "result beyond candidate population\n";
    $revision eq $candidate->[1] && $parent eq $candidate->[2]
        or die "candidate ordering changed\n";
    $status{$status}++;
    $reasons{$row[13]}++;
    next unless $status eq 'passed';
    my $stem = sprintf "%s.tsv.logs/%03d-%s", $base, $ordinal, $revision;
    my $out = bytes("$stem.out");
    my $err = bytes("$stem.err");
    $out =~ /^replay desired=([0-9a-f]{64}) candidate=([0-9a-f]{64}) verified=true operations=([1-9][0-9]*)$/m
        or die "$revision: missing nonempty replay proof\n";
    my ($desired, $actual, $proved_count) = ($1, $2, $3);
    $desired eq $actual && $count == $proved_count
        or die "$revision: equality/count mismatch\n";
    $out =~ /^transaction source-before=([0-9a-f]{64}) source-after=([0-9a-f]{64}) mode=apply$/m
        or die "$revision: transaction was not applied\n";
    my ($source_before, $source_after) = ($1, $2);
    my %applied;
    for my $line (split /\n/, $out) {
        if ($line =~ /^("(?:\\.|[^"])*") level=source-claim file before=[0-9a-f]{64} after=[0-9a-f]{64}$/) {
            $applied{decode_json($1)} = 1;
        }
    }
    my @changed = grep { length } split /\0/, bytes("$stem.changed-paths");
    @changed or die "$revision: no changed source paths\n";
    for my $path (@changed) {
        my $projected = $path;
        $projected =~ s/\.rs$/.lkj/;
        $applied{"$before/$projected"}
            or die "$revision: changed source outside applied files: $path\n";
    }
    bytes("$stem.scope") eq '' or die "$revision: scope proof was not empty\n";
    print join("\t", $ordinal, $revision, $parent, $count, scalar(@changed),
        $desired, $actual, $source_before, $source_after, $row[15], $row[14],
        sha256_hex($out), sha256_hex($err)), "\n";
    $passed++;
    $operations += $count;
}
$meta =~ /^attempted=([0-9]+) passed=([0-9]+) wanted=30$/m
    or die "missing completion summary\n";
$1 == @records && $2 == $passed or die "completion counts disagree\n";
printf STDERR "verified attempted=%d passed=%d operations=%d candidates=%d unattempted=%d\n",
    scalar(@records), $passed, $operations, scalar(@ordered), scalar(@ordered)-@records;
for my $name (sort keys %status) { print STDERR "status $name $status{$name}\n"; }
for my $name (sort keys %reasons) { print STDERR "reason $name $reasons{$name}\n"; }
