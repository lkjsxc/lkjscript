#!/usr/bin/env perl
use strict;
use warnings;
use JSON::PP;
my ($prefix, $output) = @ARGV;
die "usage: compiler-cache-index-condition-report.pl RESULT_PREFIX OUTPUT_PREFIX\n" unless defined $output && @ARGV == 2;
open my $input, '<', "$prefix.tsv" or die $!;
my $header = <$input>;
chomp $header;
my @columns = split /\t/, $header;
my @fields = qw(read_attempts authenticated_reads authenticated_payload_bytes authenticated_envelope_bytes read_ns pack_loads pack_decode_bytes pack_decode_ns pack_retain_calls pack_retain_ns put_requests seals sealed_payload_bytes seal_ns pack_reseal_calls pack_reseals pack_reseal_payload_bytes pack_reseal_ns publish_requests publish_attempts published published_payload_bytes published_envelope_bytes publish_ns prepare_ns metadata_ns budget_headers budget_header_bytes);
my @phases = qw(read_ns pack_decode_ns pack_retain_ns seal_ns pack_reseal_ns publish_ns prepare_ns metadata_ns);
my %allowed = map { $_ => 1 } qw(front native check image inputs-image index inputs-index test inputs-test metadata other);
my $json = JSON::PP->new->canonical;
open my $raw, '>', "$output.jsonl" or die $!;
open my $io, '>', "$output.io.tsv" or die $!;
open my $checks, '>', "$output.checks.tsv" or die $!;
print $io join("\t", qw(sequence rung role phase category), @fields, 'total_ns'), "\n";
print $checks join("\t", qw(sequence rung role phase status has_summary global_timing_failures global_counter_overflow pending_checkpoint accounted_ns category_sum_ns store_io_ns semantic_index_ns coverage_scope coverage_ns residual_ns validity)), "\n";
my $invalid = 0;
my $envelope_header = length("lkjscript-store-2\n") + 8 + 64;
while (<$input>) {
    chomp;
    my @values = split /\t/, $_, -1;
    die "malformed timing row\n" unless @values == @columns;
    my %row;
    @row{@columns} = @values;
    my $stem = sprintf("%03d-%s-%s-%s", @row{qw(sequence rung role phase)});
    open my $error, '<', "$prefix.raw/$stem.stderr" or die $!;
    my ($stats, $summary, %categories);
    my @problems;
    while (my $line = <$error>) {
        my $kind;
        if ($line =~ s/^LKJSCRIPT_STATS //) { $kind = 'stats'; }
        elsif ($line =~ s/^LKJSCRIPT_STORE_IO //) { $kind = 'store_io'; }
        else { next; }
        my $fact = decode_json($line);
        print $raw $json->encode({ observation => \%row, kind => $kind, fact => $fact }), "\n";
        if ($kind eq 'stats') {
            push @problems, 'duplicate-stats' if $stats;
            $stats = $fact;
        } elsif ($fact->{category} eq 'summary') {
            push @problems, 'duplicate-summary' if $summary;
            $summary = $fact;
        } else {
            my $category = $fact->{category};
            push @problems, 'unknown-or-duplicate-category' if !$allowed{$category} || $categories{$category};
            $categories{$category} = $fact;
        }
    }
    close $error;
    my $sum = 0;
    for my $category (sort keys %categories) {
        my $fact = $categories{$category};
        for my $field (@fields) {
            push @problems, "missing-or-invalid-$field" unless exists $fact->{$field} && "$fact->{$field}" =~ /^\d+$/;
        }
        my $total = 0;
        $total += $fact->{$_} // 0 for @phases;
        $sum += $total;
        push @problems, 'read-envelope-bytes' unless
            $fact->{authenticated_envelope_bytes} == $fact->{authenticated_payload_bytes} + $envelope_header * $fact->{authenticated_reads};
        push @problems, 'published-envelope-bytes' unless
            $fact->{published_envelope_bytes} == $fact->{published_payload_bytes} + $envelope_header * $fact->{published};
        print $io join("\t", @row{qw(sequence rung role phase)}, $category, map({ $fact->{$_} // '-' } @fields), $total), "\n";
    }
    my $needs = $row{role} eq 'instrumented' && $row{phase} =~ /^(cold_A|populate_B|body_A|body_B)$/;
    push @problems, 'missing-store-summary' if $needs && !$summary;
    if ($summary) {
        for my $field (qw(global_timing_failures global_counter_overflow pending_checkpoint accounted_ns)) {
            push @problems, "missing-$field" unless exists $summary->{$field};
        }
        push @problems, 'failed-timing' if $summary->{global_timing_failures};
        push @problems, 'counter-overflow' if $summary->{global_counter_overflow};
        push @problems, 'pending-checkpoint' if $summary->{pending_checkpoint};
        push @problems, 'accounting-mismatch' unless $sum == $summary->{accounted_ns};
    }
    my $store_ns = $stats && exists $stats->{store_io_ms} ? sprintf("%.0f", $stats->{store_io_ms} * 1_000_000) : '-';
    my $index_ns = $stats && exists $stats->{semantic_index_ms} ? sprintf("%.0f", $stats->{semantic_index_ms} * 1_000_000) : '-';
    my $coverage_scope = $row{phase} eq 'populate_B' ? 'store_io+semantic_index' : 'store_io';
    my $coverage = $store_ns;
    if ($row{phase} eq 'populate_B') {
        push @problems, 'missing-index-preparation-phase' unless $index_ns ne '-';
        $coverage = $store_ns ne '-' && $index_ns ne '-' ? $store_ns + $index_ns : '-';
    }
    my $residual = $summary && $coverage ne '-' ? $coverage - $sum : '-';
    push @problems, 'category-time-exceeds-declared-driver-coverage' if $residual ne '-' && $residual < 0;
    my $validity = $row{status} ne '0' ? 'refused-command' : @problems ? join(',', @problems) : 'valid';
    $invalid++ if @problems;
    print $checks join("\t", @row{qw(sequence rung role phase status)}, $summary ? 'yes' : 'no',
        map({ $summary ? ($summary->{$_} // '-') : '-' } qw(global_timing_failures global_counter_overflow pending_checkpoint accounted_ns)),
        $sum, $store_ns, $index_ns, $coverage_scope, $coverage, $residual, $validity), "\n";
}
close $input;
close $raw;
close $io;
close $checks;
die "$invalid observations have invalid attribution; raw evidence retained\n" if $invalid;
