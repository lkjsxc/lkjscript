#!/usr/bin/env perl
# Verify this separate four-case segment from retained records; never runs a compiler.
use strict;
use warnings;
use JSON::PP;
use Digest::SHA qw(sha256_hex);
my ($repository, $base) = @ARGV;
defined $base && @ARGV == 2 or die "usage: $0 REPOSITORY REPORT_PREFIX\n";
sub bytes {
    my ($path) = @_;
    open my $file, '<:raw', $path or die "$path: $!\n";
    local $/;
    return <$file> // '';
}
sub git_bytes {
    open my $pipe, '-|', 'git', '-C', $repository, @_ or die "git: $!\n";
    binmode $pipe;
    local $/;
    my $value = <$pipe> // '';
    close $pipe or die "git failed\n";
    return $value;
}
sub checked {
    my ($path, $expected) = @_;
    my $value = bytes($path);
    sha256_hex($value) eq $expected or die "$path: control hash differs\n";
    return $value;
}
my $imports = checked('bench/compiler-strict-history-imported.tsv',
    '0aafd3a891bf224bd3113ab9de58219fecba44b84e0c5605a28f8ced0f25acb1');
my $prior = checked('bench/compiler-self-history.proofs.tsv',
    '8fd505fc658b051491ddc157e43783bff39befcde721e3ab4b97115661a93eae');
my $outside = checked('bench/compiler-self-history.outside-language.tsv',
    'e5a7ba7ffb4a2f60b241eabbbdfd41e354be9ed962d4c219b512161f92d5401e');
my %outside;
for (split /\n/, $outside) {
    next if /^revision\t/;
    my @row = split /\t/;
    $outside{$row[0]} = 1;
}
my @prior = split /\n/, $prior;
my $proof_header = shift @prior;
my @eligible = grep { my @row = split /\t/; !$outside{$row[1]} } @prior;
@eligible == 23 && $imports eq join("\n", $proof_header, @eligible) . "\n"
    or die "imported rows are not the exact eligible prior subset\n";
my %imported = map { my @row = split /\t/; $row[1] => 1 } @eligible;
my $ordered = checked('bench/compiler-strict-history-four.order',
    '1241ad75095c3492af9ab18b9435d88bc6797d74b03590622264498c5ce3f5bd');
my @order = map { [split /\t/] } split /\n/, $ordered;
@order == 4 or die "wrong selected count\n";
my $population = git_bytes('log', '-n', '300', '--format=%H %P',
    'a4fd685a9bb2e7736110a4ed2a43651dfe040d56');
sha256_hex($population) eq 'dfe6dd8098ed931ce5ccd847cab1ab6411d769c6b4c22a80bc8682babbc0cc4b'
    or die "actual frozen Git population differs\n";
my $meta = bytes("$base.tsv.meta");
$meta =~ /^segment=separate-four-candidate-probe$/m or die "wrong segment\n";
$meta =~ /^mode=replay root_policy=common-main-otherwise-common-lib scan=300 probe_count=4 memory_bytes=4294967296 timeout_seconds=900 store_bytes=2147483648$/m
    or die "resource or root policy differs\n";
$meta =~ /^4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09  /m
    or die "producer metadata differs\n";
$meta =~ /^retained_fixture=(.+)$/m or die "missing retained fixture path\n";
my $before = "$1/before";
my @records = split /\n/, bytes("$base.tsv");
my @header = split /\t/, shift @records;
@header == 16 && $header[-1] eq 'source_root' or die "wrong result schema\n";
@records == 4 or die "probe did not observe exactly four candidates\n";
print "$proof_header\n";
my ($passed, $operations, $facts, $text_equal) = (0, 0, 0, 0);
my (%seen, %reasons);
for my $index (0 .. 3) {
    my @row = split /\t/, $records[$index], -1;
    @row == 16 or die "wrong result columns\n";
    my ($ordinal, $revision, $parent, $status, $projection, $count) = @row;
    $ordinal == $index+1 && $revision eq $order[$index][2] && $parent eq $order[$index][3]
        or die "candidate order differs\n";
    !$seen{$revision}++ && !$imported{$revision} or die "duplicate or reexecuted prior success\n";
    my $stem = sprintf "%s.tsv.logs/%03d-%s", $base, $ordinal, $revision;
    bytes("$stem.original-ordinal") eq "$order[$index][0]\n" or die "original ordinal differs\n";
    my $all_changed = git_bytes('diff', '--name-only', '--no-renames', '-z', $parent, $revision);
    bytes("$stem.all-changed-paths") eq $all_changed or die "changed-path record differs from Git\n";
    my @required = grep { m{^(src|library)/} || /\.(lkj|rs)$/ } grep { length } split /\0/, $all_changed;
    bytes("$stem.changed-paths") eq join('', map { "$_\0" } @required)
        or die "required-path record differs from all-language policy\n";
    my $out = bytes("$stem.out");
    my $err = bytes("$stem.err");
    $reasons{$row[13]}++;
    next unless $status eq 'passed';
    $out =~ /^replay desired=([0-9a-f]{64}) candidate=([0-9a-f]{64}) verified=true operations=([1-9][0-9]*)$/m
        or die "$revision: missing nonempty equality proof\n";
    my ($desired, $actual, $proved_count) = ($1, $2, $3);
    $desired eq $actual && $count == $proved_count or die "$revision: equality/count differs\n";
    $out =~ /^transaction source-before=([0-9a-f]{64}) source-after=([0-9a-f]{64}) mode=apply$/m
        or die "$revision: missing applied transaction\n";
    my ($source_before, $source_after) = ($1, $2);
    my %applied;
    for my $line (split /\n/, $out) {
        if ($line =~ /^("(?:\\.|[^"])*") level=source-claim file before=([0-9a-f]{64}) after=([0-9a-f]{64})$/) {
            $applied{decode_json($1)} = [$2, $3];
        }
    }
    @required or die "$revision: empty changed-source scope\n";
    for my $path (@required) {
        (my $projected = $path) =~ s/\.rs$/.lkj/;
        my $fact = $applied{"$before/$projected"} // die "$revision: uncovered required path $path\n";
        $fact->[0] eq sha256_hex(git_bytes('show', "$parent:$path"))
            or die "$revision: before fact differs from historical bytes at $path\n";
        $text_equal++ if $fact->[1] eq sha256_hex(git_bytes('show', "$revision:$path"));
        $facts++;
    }
    bytes("$stem.scope") eq '' or die "$revision: nonempty scope refusal\n";
    print join("\t", $ordinal, $revision, $parent, $count, scalar(@required),
        $desired, $actual, $source_before, $source_after, $row[15], $row[14],
        sha256_hex($out), sha256_hex($err)), "\n";
    $passed++;
    $operations += $count;
}
$meta =~ /^attempted=4 passed=([0-9]+) expected_probes=4$/m && $1 == $passed
    or die "completion counts differ\n";
printf STDERR "verified new_attempts=4 new_passed=%d new_operations=%d imported_passed=23 combined_language_scope=%d\n",
    $passed, $operations, 23+$passed;
printf STDERR "new_file_facts=%d exact_historical_before=%d exact_target_text=%d target_text_differs=%d\n",
    $facts, $facts, $text_equal, $facts-$text_equal;
for my $reason (sort keys %reasons) { print STDERR "reason $reason $reasons{$reason}\n"; }
