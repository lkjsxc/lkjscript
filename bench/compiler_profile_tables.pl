#!/usr/bin/env perl
# Bench-only extraction of counters from LKJPROF3, after the existing profiler validates it.
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use Encode qw(decode FB_CROAK LEAVE_SRC);
use JSON::PP;
@ARGV == 3 or die "usage: compiler_profile_tables.pl PREFIX PROFILE0 PROFILE64\n";
my ($prefix, @files) = @ARGV;
my @policies = (0, 64);
my @kinds = ('unknown', 'inline copy', 'checks', 'call instruction', 'frame load/store', 'other memory', 'arithmetic / comparison / conversion', 'branch', 'allocator', 'runtime', 'register-to-register move', 'address formation (LEA)', 'immediate constant', 'argument setup / result capture', 'prologue / epilogue', 'return');
for my $suffix (qw(counts kinds functions)) { -e "$prefix.$suffix.tsv" and die "existing output $suffix\n"; }
open my $counts, '>', "$prefix.counts.tsv" or die $!;
open my $kind_table, '>', "$prefix.kinds.tsv" or die $!;
open my $functions, '>', "$prefix.functions.tsv" or die $!;
print {$counts} "policy\tfile_sha256\tfile_bytes\tperiod_us\tserial\ttotal_samples\tsymbols\tobserved_symbols\tzero_symbols\tself_sum\tunattributed_self\tkind_regions\tcode_start\tcode_end\n";
print {$kind_table} "policy\tkind\tsamples\tpercent\tclass\n";
print {$functions} "policy\tsymbol_index\tself_samples\tinclusive_samples\tself_percent\tinclusive_percent\tstart_address\tend_address\tname_json\n";
my $json = JSON::PP->new->ascii->allow_nonref;
sub word {
    my ($bytes, $offset) = @_;
    $offset >= 0 && $offset <= length($$bytes) - 8 or die "counter offset outside profile\n";
    return unpack('Q<', substr($$bytes, $offset, 8));
}
for my $index (0..1) {
    open my $input, '<:raw', $files[$index] or die $!;
    local $/;
    my $bytes = <$input>;
    close $input;
    length($bytes) >= 80 && substr($bytes, 0, 8) eq 'LKJPROF3' or die "profile format differs\n";
    my $digest = sha256_hex($bytes);
    my $symbol_count = word(\$bytes, 8);
    $symbol_count <= int((length($bytes) - 80) / 64) or die "invalid symbol count\n";
    my $period_us = word(\$bytes, 16) / 1000;
    my $serial = word(\$bytes, 24);
    my $kind_regions = word(\$bytes, 40);
    my $counter_offset = word(\$bytes, 48);
    my $code_start = word(\$bytes, 56);
    my $code_end = word(\$bytes, 64);
    my @samples = map { word(\$bytes, $counter_offset + $_ * 8) } 0..15;
    my $total = 0; $total += $_ for @samples;
    $total > 0 && $serial >= $total && $serial - $total <= 1 or die "incomplete sample totals\n";
    for my $kind (0..15) {
        printf {$kind_table} "%s\t%s\t%s\t%.6f\t%s\n", $policies[$index], $kind, $samples[$kind], 100 * $samples[$kind] / $total, $kinds[$kind];
    }
    my ($observed, $self_sum) = (0, 0);
    for my $symbol (0..$symbol_count - 1) {
        my $row = 80 + $symbol * 64;
        my $start = word(\$bytes, $row);
        my $end = word(\$bytes, $row + 8);
        my $self = word(\$bytes, $row + 16);
        my $inclusive = word(\$bytes, $row + 24);
        my $name_start = word(\$bytes, $row + 40);
        my $name_length = word(\$bytes, $row + 48);
        $name_start <= length($bytes) && $name_length <= length($bytes) - $name_start or die "name outside profile\n";
        $self <= $inclusive && $inclusive <= $serial or die "inconsistent function counts\n";
        $self_sum += $self;
        next unless $self || $inclusive;
        ++$observed;
        my $name = decode('UTF-8', substr($bytes, $name_start, $name_length), FB_CROAK | LEAVE_SRC);
        printf {$functions} "%s\t%s\t%s\t%s\t%.6f\t%.6f\t%s\t%s\t%s\n", $policies[$index], $symbol, $self, $inclusive, 100 * $self / $total, 100 * $inclusive / $total, $start, $end, $json->encode($name);
    }
    $self_sum <= $total or die "function self counts exceed samples\n";
    print {$counts} join("\t", $policies[$index], $digest, length($bytes), $period_us, $serial, $total, $symbol_count, $observed, $symbol_count-$observed, $self_sum, $total-$self_sum, $kind_regions, $code_start, $code_end), "\n";
}
close $counts; close $kind_table; close $functions;
