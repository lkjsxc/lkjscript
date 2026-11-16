#!/usr/bin/env perl
# Static assessment only: reads frozen Git history and earlier result records.
use strict;
use warnings;
use JSON::PP;
my ($repository, $base, $output) = @ARGV;
@ARGV == 3 or die "usage: $0 REPOSITORY SCOPED_PREFIX OUTPUT\n";
sub bytes { open my $f, '<:raw', $_[0] or die "$_[0]: $!\n"; local $/; return <$f> // ''; }
sub git {
    open my $f, '-|', 'git', '-C', $repository, @_ or die $!;
    local $/; my $value = <$f> // '';
    close $f or die "git command failed: @_\n";
    return $value;
}
my %prior;
my @prior = split /\n/, bytes('bench/compiler-history-replay.tsv');
shift @prior;
for (@prior) { my @row = split /\t/, $_; $prior{$row[1]} = \@row; }
my @order = split /\n/, bytes("$base.order");
@order == 197 or die "wrong ordered population\n";
open my $out, '>', $output or die $!;
print $out join("\t", qw(ordinal weight revision parent outside_language_count outside_language_paths projected_added_count projected_added_paths projected_removed_count projected_removed_paths prior_status prior_reason common_root)), "\n";
my (%prior_reasons, $clean, $clean_no_new_files, $clean_new_files);
($clean, $clean_no_new_files, $clean_new_files) = (0, 0, 0);
for my $at (103 .. $#order) {
    my ($weight, $revision, $parent) = split /\t/, $order[$at];
    my @changed = grep { length } split /\0/,
        git('diff', '--name-only', '--no-renames', '-z', $parent, $revision);
    my @outside = grep { /\.(lkj|rs)$/ && !m{^(src|library)/} } @changed;
    my (%before, %after);
    for my $path (split /\n/, git('ls-tree', '-r', '--name-only', $parent, '--', 'src', 'library')) {
        next unless $path =~ /\.(lkj|rs)$/;
        $path =~ s/\.rs$/.lkj/;
        $before{$path} = 1;
    }
    for my $path (split /\n/, git('ls-tree', '-r', '--name-only', $revision, '--', 'src', 'library')) {
        next unless $path =~ /\.(lkj|rs)$/;
        $path =~ s/\.rs$/.lkj/;
        $after{$path} = 1;
    }
    my @added = sort grep { !$before{$_} } keys %after;
    my @removed = sort grep { !$after{$_} } keys %before;
    my $root = $before{'src/main.lkj'} && $after{'src/main.lkj'} ? 'src/main.lkj'
        : $before{'src/lib.lkj'} && $after{'src/lib.lkj'} ? 'src/lib.lkj' : 'missing';
    my $old = $prior{$revision};
    my $status = $old ? $old->[3] : 'not-in-original-data';
    my $reason = $old ? $old->[13] : '-';
    if (!@outside) {
        $clean++;
        $prior_reasons{"$status/$reason"}++;
        if (!@added && !@removed) { $clean_no_new_files++; }
        else { $clean_new_files++; }
    }
    print $out join("\t", $at+1, $weight+0, $revision, $parent, scalar(@outside),
        encode_json(\@outside), scalar(@added), encode_json(\@added),
        scalar(@removed), encode_json(\@removed), $status, $reason, $root), "\n";
}
printf "remaining=%d no_outside_language=%d with_outside_language=%d no_outside_and_no_projected_file_set_change=%d no_outside_with_projected_file_set_change=%d\n",
    @order-103, $clean, @order-103-$clean, $clean_no_new_files, $clean_new_files;
for my $key (sort keys %prior_reasons) { print "prior_for_no_outside $key $prior_reasons{$key}\n"; }
