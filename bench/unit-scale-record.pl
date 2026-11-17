#!/usr/bin/env perl
use strict;
use warnings;
use JSON::PP;
my ($result, $label, $phase, $lines, $metadata, $prefix, $exit, $before, $after,
    $memory, $watchdog, $artifact, $store_bytes) = @ARGV;
die "unit-scale-record.pl needs 13 arguments\n" unless defined $store_bytes;
sub read_file {
    my ($path) = @_; return '' unless -f $path;
    open my $file, '<:raw', $path or die $!; local $/; my $text=<$file>; close $file; return $text;
}
my $stats = {};
for my $line (split /\n/, read_file("$prefix.stderr")) {
    if ($line =~ /^LKJSCRIPT_STATS (\{.*\})$/) {
        my $value = eval { decode_json($1) };
        $stats = $value if ref($value) eq 'HASH';
    }
}
my ($wall, $user, $system, $rss) = ('NA') x 4;
for my $line (split /\n/, read_file("$prefix.time")) {
    if ($line =~ /^([0-9.]+)\t([0-9.]+)\t([0-9.]+)\t([0-9]+)\t[0-9]+$/) {
        ($wall, $user, $system, $rss) = ($1,$2,$3,$4);
    }
}
my $shape = {};
$shape = decode_json(read_file($metadata)) if $metadata ne '-' && -f $metadata;
my @columns = qw(workload source_lines phase exit wall_s user_s system_s peak_rss_kib
    compiler_peak_rss_bytes load_before load_after memory_limit_mib watchdog_s
    units_checked units_reused units_emitted expected_units largest_source_unit_lines
    maximum_direct_dependencies dependency_depth max_definitions max_types max_ir_functions
    max_import_interfaces max_projection_bytes max_sources emitted_bytes store_bytes);
sub metric { my ($name) = @_; return defined($stats->{$name}) ? $stats->{$name} : 'NA'; }
sub shape { my ($name) = @_; return defined($shape->{$name}) ? $shape->{$name} : 'NA'; }
my @row = ($label,$lines,$phase,$exit,$wall,$user,$system,$rss,
    metric('peak_rss_bytes'),$before,$after,$memory,$watchdog,
    metric('units_checked'),metric('units_reused'),metric('units_emitted'),
    shape('units'),shape('largest_unit_lines'),shape('maximum_direct_dependencies'),shape('dependency_depth'),
    metric('units_max_definitions'),metric('units_max_types'),metric('units_max_ir_functions'),
    metric('units_max_import_interfaces'),metric('units_max_projection_bytes'),metric('units_max_sources'),
    $artifact,$store_bytes);
my $table = "$result/measurements.tsv";
my $new = !-e $table;
open my $file, '>>:raw', $table or die $!;
print {$file} join("\t", @columns), "\n" if $new;
print {$file} join("\t", @row), "\n";
close $file or die $!;
printf "%s %s %s: exit=%s wall=%ss RSS=%sKiB checked=%s reused=%s\n",
    $label,$lines,$phase,$exit,$wall,$rss,metric('units_checked'),metric('units_reused');
