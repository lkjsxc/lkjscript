#!/usr/bin/env perl
# Read-only validation of frozen inputs and completed phase records.
use strict;
use warnings;
use Cwd qw(realpath);
use Digest::SHA;
use JSON::PP;
use POSIX qw(ceil);
my $json = JSON::PP->new->canonical->pretty;
sub read_text {
    my ($path) = @_;
    open my $file, '<', $path or die "$path: $!\n";
    local $/;
    return <$file>;
}
sub digest {
    my ($path) = @_;
    open my $file, '<', $path or die "$path: $!\n";
    binmode $file;
    return Digest::SHA->new(256)->addfile($file)->hexdigest;
}
sub object {
    my ($path) = @_;
    my $value = decode_json(read_text($path));
    ref($value) eq 'HASH' or die "$path: expected object\n";
    return $value;
}
sub hex_digest {
    my ($value) = @_;
    return defined($value) && !ref($value) && $value =~ /\A[0-9a-f]{64}\z/;
}
sub record {
    my ($repository, $entry, $label) = @_;
    ref($entry) eq 'HASH' && hex_digest($entry->{sha256})
        or die "$label: record and SHA256 required\n";
    my $relative = $entry->{path};
    defined($relative) && !ref($relative) && $relative =~ m{\Abench/[A-Za-z0-9_./-]+\z}
        && $relative !~ m{(?:\A|/)\.\.(?:/|\z)}
        or die "$label: expected a bench record in this repository\n";
    my $path = realpath("$repository/$relative");
    defined($path) && index($path, "$repository/bench/") == 0
        && -f $path && !-l "$repository/$relative"
        or die "$label: missing regular record\n";
    digest($path) eq $entry->{sha256} or die "$label: record digest changed\n";
}
my $mode = shift @ARGV // '';
if ($mode eq 'release') {
    @ARGV == 3 or die "release REPOSITORY RELEASE PRODUCER\n";
    my ($repository, $path, $producer) = @ARGV;
    my $release = object($path);
    ($release->{protocol} // '') eq 'lkjscript-scale-refusal-1'
        or die "wrong release protocol\n";
    JSON::PP::is_bool($release->{released}) && $release->{released}
        or die "root has not released this experiment\n";
    ($release->{release_id} // '') =~ /\Acompiler-[a-z0-9][a-z0-9-]*\z/
        or die "invalid release identity\n";
    ($release->{source_revision} // '') =~ /\A[0-9a-f]{40}\z/
        or die "exact gated source revision required\n";
    ($release->{producer_inline_limit} // -1) == 64
        or die "producer inline limit must be 64\n";
    hex_digest($release->{producer_sha256})
        or die "exact gated producer SHA256 required\n";
    -f $producer && -x $producer && digest($producer) eq $release->{producer_sha256}
        or die "producer differs from the released bytes\n";
    for my $entry (
        ['protocol_sha256', 'bench/compiler-scale-refusal-plan.md'],
        ['harness_sha256', 'bench/compiler-scale-refusal.sh'],
        ['helper_sha256', 'bench/compiler-scale-refusal.pl'],
    ) {
        hex_digest($release->{$entry->[0]})
            && digest("$repository/$entry->[1]") eq $release->{$entry->[0]}
            or die "$entry->[0]: frozen implementation changed\n";
    }
    for my $label (qw(gate_record fixed_point_record source_manifest current50_record historical_case_record)) {
        record($repository, $release->{$label}, $label);
    }
    print "$release->{producer_sha256}\n$release->{source_revision}\n$release->{release_id}\n";
} elsif ($mode eq 'phase') {
    @ARGV == 6 or die "phase EVIDENCE LABEL EXIT MEMORY_BYTES DEADLINE KIND\n";
    my ($evidence, $label, $status, $memory, $deadline, $kind) = @ARGV;
    $label =~ /\A[a-z0-9-]+\z/ && $status =~ /\A[0-9]+\z/
        && $memory =~ /\A[0-9]+\z/ && $deadline =~ /\A[0-9]+\z/
        or die "invalid phase fields\n";
    my %meta;
    for (split /\n/, read_text("$evidence/$label.meta")) {
        my ($key, $value) = split /=/, $_, 2;
        $meta{$key} = $value if defined $value;
    }
    my $loads_ok = defined($meta{load_before}) && defined($meta{load_after})
        && $meta{load_before} =~ /\A[0-9.]+,[0-9.]+,[0-9.]+\z/
        && $meta{load_after} =~ /\A[0-9.]+,[0-9.]+,[0-9.]+\z/;
    my $result = {
        phase => $label, kind => $kind, wrapper_exit => 0 + $status,
        memory_limit_bytes => 0 + $memory, timeout_seconds => 0 + $deadline,
        load_before => $meta{load_before}, load_after => $meta{load_after},
        complete_metrics => JSON::PP::false,
        outcome => $status == 0 ? 'ok' : "exit_$status",
    };
    if (-f "$evidence/$label.time") {
        my @rows = grep { /\S/ } split /\n/, read_text("$evidence/$label.time");
        if (@rows && $rows[-1] =~ /\A([0-9.]+)\t([0-9.]+)\t([0-9.]+)\t([0-9]+)\t([0-9]+)\z/) {
            @{$result}{qw(wall_s user_s kernel_s rss_kib gnu_exit)} = map { 0 + $_ } ($1,$2,$3,$4,$5);
            $result->{cpu_s} = $result->{user_s} + $result->{kernel_s};
            $result->{complete_metrics} = JSON::PP::true if $loads_ok;
        }
    }
    open my $stderr, '<', "$evidence/$label.stderr" or die "$label stderr: $!\n";
    my ($stats_count, $io_count) = (0, 0);
    while (my $line = <$stderr>) {
        if ($line =~ /^LKJSCRIPT_STATS (.*)$/) {
            ++$stats_count;
            my $value = eval { decode_json($1) };
            $result->{stats} = $value if ref($value) eq 'HASH';
        }
        if ($line =~ /^LKJSCRIPT_STORE_IO (.*)$/) {
            my $value = eval { decode_json($1) };
            if (ref($value) eq 'HASH' && ($value->{category} // '') eq 'summary') {
                ++$io_count;
                $result->{store_io} = $value;
            }
        }
        if (!exists($result->{located_resource_trap}) && $line =~ /:\d+(?::\d+)?:\s+R000[68]\b/) {
            chomp $line;
            $result->{located_resource_trap} = $line;
        }
    }
    $result->{stats_records} = $stats_count;
    $result->{store_io_summary_records} = $io_count;
    if ($status != 0) {
        $result->{outcome} = 'watchdog' if $status == 124;
        $result->{outcome} = 'signal_or_watchdog' if $status == 137;
        $result->{outcome} = 'located_resource_refusal' if exists $result->{located_resource_trap};
    }
    $result->{capture_status} = $result->{complete_metrics} ? 'complete' : 'incomplete';
    print $json->encode($result);
} elsif ($mode eq 'row') {
    @ARGV == 1 or die "row RESULT\n";
    my $value = object($ARGV[0]);
    print join("\t", map { defined($value->{$_}) ? $value->{$_} : '-' }
        qw(phase kind outcome wall_s user_s kernel_s rss_kib wrapper_exit gnu_exit memory_limit_bytes timeout_seconds load_before load_after capture_status)), "\n";
} elsif ($mode eq 'field') {
    @ARGV == 2 or die "field RESULT FIELD\n";
    my $value = object($ARGV[0]);
    exists($value->{$ARGV[1]}) && !ref($value->{$ARGV[1]})
        or die "missing scalar $ARGV[1]\n";
    print "$value->{$ARGV[1]}\n";
} elsif ($mode eq 'validate-build') {
    @ARGV == 2 && ($ARGV[1] eq 'build' || $ARGV[1] eq 'check')
        or die "validate-build RESULT build|check\n";
    my ($path, $kind) = @ARGV;
    my $value = object($path);
    $value->{complete_metrics} && $value->{wrapper_exit} == 0 && $value->{gnu_exit} == 0
        or die "build did not complete with metrics\n";
    my $stats = $value->{stats};
    ref($stats) eq 'HASH' && $value->{stats_records} == 1
        && ($stats->{route} // '') eq 'compiler' && ($stats->{stopped_stage} // '') eq 'done'
        or die "missing successful compiler statistics\n";
    for my $key (qw(front_reused checked_reused checked_local_reused emitted_reused build_reused inline_calls inline_attempts store_hits)) {
        exists($stats->{$key}) && $stats->{$key} == 0
            or die "fresh inline-0 control violated: $key\n";
    }
    ($stats->{ir_functions} // 0) > 0 && ($stats->{ir_instructions} // 0) > 0
        or die "missing checked instance counts\n";
    ($kind ne 'build' || ($stats->{emitted_bytes} // 0) > 0)
        or die "missing image count\n";
    my $io = $value->{store_io};
    ref($io) eq 'HASH' && $value->{store_io_summary_records} == 1
        or die "missing Store I/O summary\n";
    for my $key (qw(global_timing_failures global_counter_overflow pending_checkpoint)) {
        exists($io->{$key}) && $io->{$key} == 0 or die "Store I/O control violated: $key\n";
    }
    print $json->encode({status => 'accepted', command => $kind,
        ir_instances => $stats->{ir_functions}, ir_instructions => $stats->{ir_instructions},
        store_retained_bytes => $stats->{store_retained_bytes},
        emitted_bytes => $stats->{emitted_bytes}});
} elsif ($mode eq 'predict') {
    @ARGV == 1 or die "predict CONTROL_RESULT\n";
    my $value = object($ARGV[0]);
    $value->{complete_metrics} && $value->{wrapper_exit} == 0
        && $value->{rss_kib} > 0 && $value->{wall_s} > 0 or die "invalid completed control\n";
    my $predicted = $value->{rss_kib} * 10;
    my $desired = ceil(($predicted * 2 + 262144) / 1024);
    my $memory = $desired > 8192 ? 8192 : $desired;
    my $deadline = ceil($value->{wall_s} * 10 * 3 + 120);
    $deadline = 900 if $deadline > 900;
    print $json->encode({control => $value->{phase}, control_lines => 100000, next_lines => 1000000,
        memory_exponent_for_bound => 1, predicted_rss_kib => $predicted,
        desired_limit_mib => $desired, memory_limit_mib => $memory,
        memory_limit_bytes => $memory * 1024 * 1024,
        ceiling_mib => 8192, timeout_seconds => $deadline,
        timeout_ceiling_seconds => 900, kill_grace_seconds => 10});
} else {
    die "modes: release, phase, row, field, validate-build, predict\n";
}
