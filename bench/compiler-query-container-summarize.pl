#!/usr/bin/env perl
# Postprocess the immutable, single-run container experiment. No compiler calls.
use strict;
use warnings;
use JSON::PP;
use Digest::SHA qw(sha256_hex);
use File::Path qw(make_path);
my ($raw, $out) = @ARGV;
die "usage: summarize RAW NEW_OUTPUT_DIRECTORY\n" unless defined $out && !-e $out;
make_path($out);
sub bytes {
    my ($path) = @_;
    open my $file, '<:raw', $path or die "$path: $!";
    local $/; my $value = <$file>; return defined($value) ? $value : '';
}
sub json { return decode_json(bytes($_[0])); }
sub tsv {
    my ($name, $columns, $rows) = @_;
    open my $file, '>:raw', "$out/$name.tsv" or die $!;
    print {$file} join("\t", @$columns), "\n";
    for my $row (@$rows) {
        my @values = map { defined($row->{$_}) ? $row->{$_} : 'not-recorded' } @$columns;
        die "TSV field separator\n" if grep { /\t|\r|\n/ } @values;
        print {$file} join("\t", @values), "\n";
    }
    close $file or die $!;
}
my @timing = split /\n/, bytes("$raw/timings.tsv");
my @columns = split /\t/, shift @timing;
my %timing;
for my $line (@timing) {
    my @values = split /\t/, $line, -1;
    die "timing columns\n" unless @values == @columns;
    my %row; @row{@columns} = @values;
    die "duplicate timing\n" if $timing{$row{phase}};
    die "nonzero recorded command\n" unless $row{exit} == 0;
    $timing{$row{phase}} = \%row;
}
my %generated;
for my $rung (10000, 100000) {
    my %fields;
    for my $line (split /\n/, bytes("$raw/$rung-generate.stdout")) {
        my ($key,$value)=split / /,$line,2; $fields{$key}=$value;
    }
    die "generated input pin\n" unless $fields{lines}==$rung && $fields{seed}==20261116;
    $generated{$rung}=\%fields;
}
my (@builds, %build, @body_counters, @body_phases, @footprint, @maps, @queries, @exponents, @controls);
my @phase_order = split /\n/, bytes("$raw/phase-order.tsv");
shift @phase_order;
for my $line (@phase_order) {
    my ($label,$rung,$producer,$state,$indexed) = split /\t/,$line;
    my $time=$timing{$label} or die "missing timing $label\n";
    my $stats=json("$raw/$label.stats-validation.json");
    die "stats validation\n" unless $stats->{status} eq 'captured-stats-validated';
    my $census=json("$raw/$label.census.json");
    my $budget=$census->{budget};
    die "budget mismatch\n" unless $budget->{status} eq 'authenticated' && $budget->{within_capacity}
        && !$budget->{missing_entries} && !$budget->{physical_length_mismatches}
        && !@{$budget->{unindexed_regular_names}} && !@{$census->{nonregular}};
    my $s=$stats->{stats}; my $p=$label; $p=~s/^\Q$rung-$producer-\E//;
    my $row={%$time,rung=>$rung,producer=>$producer,state=>$state,kind=>$p,
        total_cpu_s=>sprintf('%.2f',$time->{user_s}+$time->{kernel_s}),
        store_bytes=>$census->{stored_bytes},budget_bytes=>$budget->{envelope_bytes},
        image_bytes=>$s->{emitted_bytes},ir_functions=>$s->{ir_functions},ir_instructions=>$s->{ir_instructions},
        source_bytes=>$generated{$rung}{source_bytes},source_lines=>$generated{$rung}{lines},
        store_io_ms=>$s->{store_io_ms},semantic_catalog_ms=>$s->{semantic_catalog_ms}//0,
        semantic_index_ms=>$s->{semantic_index_ms}//0};
    push @builds,$row; $build{$rung}{$producer}{$p}=$row;
    if ($p eq 'body-s1') {
        my $m=$stats->{store_io_categories}{metadata} or die "missing metadata row\n";
        push @body_counters,{rung=>$rung,producer=>$producer,wall_s=>$time->{wall_s},
            load_before=>$time->{load_before},load_after=>$time->{load_after},
            driver_store_io_ms=>$s->{store_io_ms},exclusive_store_io_ns=>$stats->{exclusive_store_counter_sum_ns},
            metadata_ns=>$m->{metadata_ns},budget_read_ns=>$m->{read_ns},budget_seal_ns=>$m->{seal_ns},
            budget_publish_ns=>$m->{publish_ns},budget_authenticated_envelope_bytes=>$m->{authenticated_envelope_bytes},
            budget_published_envelope_bytes=>$m->{published_envelope_bytes},budget_headers=>$m->{budget_headers},
            budget_header_bytes=>$m->{budget_header_bytes},budget_manifest_bytes=>$m->{budget_manifest_bytes}//0,
            budget_container_authenticated=>$m->{budget_container_authenticated}//0,
            budget_container_extent_checks=>$m->{budget_container_extent_checks}//0};
        push @body_phases,{rung=>$rung,producer=>$producer,load_before=>$time->{load_before},load_after=>$time->{load_after},
            map { $_=>$s->{$_}//0 } qw(total_ms producer_hash_ms source_read_ms load_parse_ms resolve_ms
                front_restore_ms front_encode_ms type_ms build_ir_ms key_hash_ms store_io_ms check_ms
                check_body_ms check_verification_ms emit_ms native_encode_ms assemble_ms runtime_ms write_ms
                front_reused checked_reused emitted_reused ir_functions ir_instructions)};
    }
    if ($p eq 'index-s0' || $p eq 'index-s1') {
        my ($count,$size)=(0,0);
        for my $family (qw(semantic-page semantic-root semantic-latest semantic-locator-v2 semantic-container-v3 semantic-latest-v3 semantic-locator-v3)) {
            my $f=$census->{namespaces}{$family} // {};
            $count += $f->{files}//0; $size += $f->{stored_bytes}//0;
        }
        my $state_pin=bytes("$raw/$rung-$producer-$state.snapshot");chomp $state_pin;
        my @selected=grep {$_->{identity} eq $state_pin} @{$census->{snapshots}};
        die "selected snapshot census\n" unless @selected==1;
        my $snapshot=$selected[0];
        push @footprint,{rung=>$rung,producer=>$producer,state=>$state,snapshot=>$state_pin,
            dedicated_snapshot_files=>$count,dedicated_snapshot_bytes=>$size,
            total_store_bytes=>$census->{stored_bytes},budget_bytes=>$budget->{envelope_bytes},
            accounted_entry_bytes=>$budget->{accounted_entry_bytes},accounted_total_bytes=>$budget->{accounted_total_bytes},
            retained_snapshot_manifests=>scalar @{$census->{snapshots}},selected_manifest_or_container_bytes=>$snapshot->{physical_bytes},
            selected_source=>$snapshot->{source},selected_checked_ir=>$snapshot->{checked_ir},
            selected_root=>$snapshot->{root},selected_facts=>$snapshot->{facts},
            inputs_semantic_bytes=>$census->{namespaces}{'inputs-semantic'}{stored_bytes}//0,
            other_bytes=>$census->{namespaces}{other}{stored_bytes}//0};
    }
}
die "build count\n" unless @builds==20;
for my $rung (10000,100000) {
    for my $state (qw(S0 S1)) {
        my $label="$rung-$state-map-oracle";
        my $m=json("$raw/$label.stdout");
        die "map status\n" unless $m->{status} eq 'exact-logical-map' && $m->{continuation_chains_verified};
        my %row=(rung=>$rung,state=>$state,%{$timing{$label}},%$m);
        delete $row{kinds};
        push @maps,\%row;
    }
}
my @query_status=split /\n/,bytes("$raw/queries.tsv");shift @query_status;
my %query_group;
for my $line(@query_status) {
    my ($rung,$state,$kind,$identity,$a,$b,$comparison)=split /\t/,$line;
    die "query status\n" unless $a eq '0' && $b eq '0' && $comparison eq '0';
    my $proof=json("$raw/$rung-$state-query-$kind-parity.stdout");
    die "query parity\n" unless $proof->{status} eq 'exact-facts-and-decoded-state';
    for my $producer(qw(baseline candidate)) {
        my $label="$rung-$state-$producer-query-$kind";
        my $stats=json("$raw/$label.stats-validation.json")->{stats};
        my $output=bytes("$raw/$label.stdout");
        die "query bound/hash\n" unless length($output)<=32768 && $stats->{producer_bytes_hashed}==0;
        my ($facts,$pages,$steps)=$output=~/\tfact=continuation\tcursor=[^ ]+ facts=([0-9]+) pages-read=([0-9]+) traversal-steps=([0-9]+)\n\z/;
        die "query continuation fields\n" unless defined($steps) && $facts<=96;
        my $row={%{$timing{$label}},rung=>$rung,state=>$state,producer=>$producer,kind=>$kind,identity=>$identity,
            depth=>1,limit=>96,byte_bound=>32768,stdout_bytes=>length($output),facts=>$facts,pages_read=>$pages,
            traversal_steps=>$steps,continued=>$proof->{continued}?1:0,producer_bytes_hashed=>$stats->{producer_bytes_hashed},
            semantic_query_ms=>$stats->{semantic_query_ms},snapshot_auth_ms=>$stats->{snapshot_auth_ms},
            total_ms=>$stats->{total_ms},stdout_sha256=>sha256_hex($output),
            normalized_sha256=>$proof->{normalized_sha256}};
        push @queries,$row; push @{$query_group{$producer}},$row;
    }
}
die "query count\n" unless @queries==104;
for my $producer(qw(baseline candidate)) {
    for my $kind(qw(cold-s0 body-s1 index-s0 index-s1)) {
        for my $metric(qw(wall_s total_cpu_s rss_kib)) {
            my ($a,$b)=map {$build{$_}{$producer}{$kind}{$metric}} (10000,100000);
            die "nonpositive exponent metric\n" unless $a>0 && $b>0;
            push @exponents,{producer=>$producer,kind=>$kind,metric=>$metric,value_10000=>$a,value_100000=>$b,
                line_ratio=>10,ratio=>sprintf('%.9f',$b/$a),two_point_exponent=>sprintf('%.6f',log($b/$a)/log(10))};
        }
    }
}
for my $label(sort keys %timing) {
    next if $label=~/\A(?:10000|100000)-(?:baseline|candidate)-/ || $label=~/-query-/;
    push @controls,$timing{$label};
}
sub distribution {
    my ($rows,$field)=@_;
    my @v=sort {$a<=>$b} map {0+$_->{$field}} @$rows;
    die "empty distribution\n" unless @v;
    return {n=>scalar(@v),minimum=>$v[0],median=>(@v%2?$v[int(@v/2)]:($v[@v/2-1]+$v[@v/2])/2),maximum=>$v[-1]};
}
my %query_summary;
for my $producer(qw(baseline candidate)) {
    $query_summary{$producer}={map {$_=>distribution($query_group{$producer},$_)}
        qw(wrapper_wall_ns semantic_query_ms snapshot_auth_ms total_ms stdout_bytes facts pages_read rss_kib)};
    $query_summary{$producer}{continued}=scalar grep {$_->{continued}} @{$query_group{$producer}};
}
my $retention=bytes("$raw/retention.tsv");
my @retention_rows=split /\n/,$retention;shift @retention_rows;
die "pin retention count\n" unless @retention_rows==8;
for(@retention_rows){my @v=split /\t/;die "lost pin\n" unless $v[4]==1 && $v[5]==1;}
tsv('builds',[qw(rung producer state kind wall_s user_s kernel_s total_cpu_s rss_kib wrapper_wall_ns load_before load_after store_bytes budget_bytes image_bytes ir_functions ir_instructions source_bytes source_lines store_io_ms semantic_catalog_ms semantic_index_ms)],\@builds);
tsv('body-counters',[qw(rung producer wall_s load_before load_after driver_store_io_ms exclusive_store_io_ns metadata_ns budget_read_ns budget_seal_ns budget_publish_ns budget_authenticated_envelope_bytes budget_published_envelope_bytes budget_headers budget_header_bytes budget_manifest_bytes budget_container_authenticated budget_container_extent_checks)],\@body_counters);
tsv('body-phases',[qw(rung producer load_before load_after total_ms producer_hash_ms source_read_ms load_parse_ms resolve_ms front_restore_ms front_encode_ms type_ms build_ir_ms key_hash_ms store_io_ms check_ms check_body_ms check_verification_ms emit_ms native_encode_ms assemble_ms runtime_ms write_ms front_reused checked_reused emitted_reused ir_functions ir_instructions)],\@body_phases);
tsv('footprint',[qw(rung producer state dedicated_snapshot_files dedicated_snapshot_bytes total_store_bytes budget_bytes accounted_entry_bytes accounted_total_bytes retained_snapshot_manifests selected_manifest_or_container_bytes selected_facts inputs_semantic_bytes other_bytes snapshot selected_source selected_checked_ir selected_root)],\@footprint);
tsv('maps',[qw(rung state wall_s user_s kernel_s rss_kib wrapper_wall_ns load_before load_after facts logical_pages logical_chains physical_nodes branches leaves max_depth logical_key_bytes logical_value_bytes fact_utf8_bytes authenticated_node_envelope_bytes_a authenticated_node_envelope_bytes_b container_bytes_b sorted_logical_map_fingerprint source checked_ir snapshot_a snapshot_b root_a root_b)],\@maps);
tsv('queries',[qw(rung state producer kind identity depth limit byte_bound stdout_bytes facts pages_read traversal_steps continued wrapper_wall_ns wall_s user_s kernel_s rss_kib semantic_query_ms snapshot_auth_ms total_ms producer_bytes_hashed load_before load_after stdout_sha256 normalized_sha256)],\@queries);
tsv('exponents',[qw(producer kind metric value_10000 value_100000 line_ratio ratio two_point_exponent)],\@exponents);
tsv('controls',\@columns,\@controls);
open my $summary,'>:raw',"$out/summary.json" or die $!;
print {$summary} JSON::PP->new->canonical->pretty->encode({status=>'all-recorded-controls-accepted',
    source_revision=>'d459ab2bb8b5cdfe5cac7e861e4eff09256197c1',builds=>\@builds,generated=>\%generated,
    body_counters=>\@body_counters,footprint=>\@footprint,maps=>\@maps,
    query_summary=>\%query_summary,build_commands=>scalar @builds,query_commands=>scalar @queries,
    map_controls=>scalar @maps,query_pairs=>scalar(@queries)/2,snapshot_pins=>scalar @retention_rows,
    exponents=>\@exponents});
close $summary or die $!;
print "accepted 20 builds, 4 maps, 104 queries, 52 query pairs and 8 pins; no compiler was invoked\n";
