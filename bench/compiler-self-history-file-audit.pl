#!/usr/bin/env perl
# Reproduce the per-file and outside-language-scope audit from Git and retained records.
use strict;
use warnings;
use JSON::PP;
use Digest::SHA qw(sha256_hex);
my ($repository, $base) = @ARGV;
@ARGV == 2 or die "usage: $0 REPOSITORY REPORT_PREFIX\n";
sub bytes {
    open my $file, '<:raw', $_[0] or die "$_[0]: $!\n";
    local $/;
    return <$file> // '';
}
sub git_bytes {
    open my $file, '-|', 'git', '-C', $repository, @_ or die "git: $!\n";
    binmode $file;
    local $/;
    my $bytes = <$file> // '';
    close $file or die "git command failed: @_\n";
    return $bytes;
}
my $meta = bytes("$base.tsv.meta");
$meta =~ /^retained_fixture=(.+)$/m or die "missing completed fixture identity\n";
my $before = "$1/before";
my @proofs = split /\n/, bytes("$base.proofs.tsv");
shift @proofs;
open my $files, '>', "$base.file-proofs.tsv" or die $!;
open my $outside, '>', "$base.outside-language.tsv" or die $!;
print $files join("\t", qw(revision historical_path projected_path published_before published_after historical_before historical_after exact_target_text)), "\n";
print $outside "revision\tparent\toutside_language_path\n";
my ($total, $exact, $outside_paths, $outside_commits) = (0, 0, 0, 0);
for my $line (@proofs) {
    my @row = split /\t/, $line;
    @row == 13 or die "bad proof schema\n";
    my ($ordinal, $revision, $parent) = @row;
    $revision =~ /^[0-9a-f]{40}$/ && $parent =~ /^[0-9a-f]{40}$/ or die "bad revisions\n";
    my $stem = sprintf "%s.tsv.logs/%03d-%s", $base, $ordinal, $revision;
    my %published;
    for my $fact (split /\n/, bytes("$stem.out")) {
        if ($fact =~ /^("(?:\\.|[^"])*") level=source-claim file before=([0-9a-f]{64}) after=([0-9a-f]{64})$/) {
            $published{decode_json($1)} = [$2, $3];
        }
    }
    my @changed = grep { length } split /\0/,
        git_bytes('diff', '--name-only', '--no-renames', '-z', $parent, $revision);
    my $has_outside = 0;
    for my $path (@changed) {
        if ($path !~ m{^(src|library)/}) {
            if ($path =~ /\.(lkj|rs)$/) {
                print $outside join("\t", $revision, $parent, $path), "\n";
                $outside_paths++;
                $has_outside = 1;
            }
            next;
        }
        my $projected = $path;
        $projected =~ s/\.rs$/.lkj/;
        my $fact = $published{"$before/$projected"} // die "$revision: missing file fact $path\n";
        my $old = sha256_hex(git_bytes('show', "$parent:$path"));
        my $new = sha256_hex(git_bytes('show', "$revision:$path"));
        $old eq $fact->[0] or die "$revision: before hash disagrees with Git $path\n";
        my $same = $new eq $fact->[1];
        print $files join("\t", $revision, $path, $projected, @$fact, $old, $new, $same ? 'true' : 'false'), "\n";
        $total++;
        $exact++ if $same;
    }
    $outside_commits++ if $has_outside;
}
printf "file_facts=%d exact_historical_before=%d exact_target_text=%d target_text_differs=%d\n",
    $total, $total, $exact, $total-$exact;
printf "counted_commits=%d outside_language_commits=%d outside_language_paths=%d all_changed_language_within_compiler_scope=%d\n",
    scalar(@proofs), $outside_commits, $outside_paths, scalar(@proofs)-$outside_commits;
