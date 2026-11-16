#!/usr/bin/env perl
# Independent raw-capture audit; never executes lkjscript or reads store nodes.
use strict;
use warnings;
use JSON::PP qw(decode_json);
use Digest::SHA qw(sha256_hex);
use Encode qw(decode FB_CROAK);
my ($result, $attempt, $questions, $fixture, $inventory) = @ARGV;
die "usage: audit RESULT ATTEMPT QUESTIONS FIXTURE INVENTORY\n" unless defined $inventory;
sub raw {
    my ($p) = @_;
    die "nonregular input $p\n" unless -f $p && !-l $p;
    open my $f, '<:raw', $p or die "$p: $!"; local $/; return <$f>;
}
sub object { return decode_json(raw($_[0])); }
sub file_hash {
    open my $f, '<:raw', $_[0] or die "$_[0]: $!";
    return Digest::SHA->new(256)->addfile($f)->hexdigest;
}
my $question_bytes = raw($questions);
die "question population\n" unless sha256_hex($question_bytes) eq '417a4adfe357aab04915c5cdee3ebe2f691ee46ca2fb74663735e5d35a6e81e9';
my @q = split /\n/, $question_bytes; shift @q;
die "question count\n" unless @q == 50;
my $pins = object("$result/pins.json");
my @times;
my @continuations;
my @slice_bytes;
my $last_finish = 0;
my ($first_start, $last_load, $first_load, $all_bytes, $owner_bytes, $all_facts) = (0, '', '', 0, 0, 0);
my @captures = glob("$result/raw/query-*.capture.json");
die "capture count\n" unless @captures == 50;
die "prior questions present\n" if glob("$attempt/raw/query-*");
for my $i (1 .. 50) {
    my @qv = split /\t/, $q[$i-1], -1;
    die "question ordinal\n" unless @qv == 7 && $qv[0] == $i;
    my ($ordinal,$group,$kind,$identity,$depth,$limit,$bound) = @qv;
    my $tag = sprintf('query-%02d', $i);
    my $cap = object("$result/raw/$tag.capture.json");
    die "nonzero query status $i\n" unless $cap->{status} == 0;
    die "clock interval $i\n" unless $cap->{finish_ns} >= $cap->{start_ns}
        && $cap->{wrapper_wall_ns} == $cap->{finish_ns} - $cap->{start_ns}
        && $cap->{start_ns} >= $last_finish;
    $first_start ||= $cap->{start_ns}; $last_finish = $cap->{finish_ns};
    $first_load = $cap->{load_before} if $i == 1; $last_load = $cap->{load_after};
    for my $name (qw(load_before load_after)) {
        die "load record $i\n" unless $cap->{$name} =~ /\A[0-9.]+,[0-9.]+,[0-9.]+\z/;
    }
    push @times, $cap->{wrapper_wall_ns};
    my $argv = object("$result/raw/$tag.argv.json");
    my @expected = ('env','-u','LKJSCRIPT_STORE_IO',"$fixture/bin/producer",
        'query','src/main.lkj','--library-dir','library','--store',"$fixture/store",
        '--store-key',"/tmp/compiler-query-current-key.seAq58/key",'--snapshot',$pins->{snapshot},'--stats',
        '--kind',$kind,'--id',$identity,'--depth',"$depth",'--limit',"$limit",'--bytes',"$bound");
    die "argv $i\n" unless ref($argv) eq 'ARRAY' && join("\0",@$argv) eq join("\0",@expected);
    die "cwd $i\n" unless raw("$result/raw/$tag.cwd") eq "$fixture/source\n";
    my $out = raw("$result/raw/$tag.stdout");
    die "output bound $i\n" unless length($out) <= $bound;
    my $copy = $out; decode('UTF-8', $copy, FB_CROAK);
    my @lines = split /\n/, $out, -1;
    die "newline $i\n" unless pop(@lines) eq '';
    my $header = shift @lines; my $tail = pop @lines;
    my @hf = split /\t/, $header, 4;
    die "header $i\n" unless @hf == 4 && decode_json($hf[0]) eq $identity
        && $hf[1] eq 'level=protocol' && $hf[2] eq 'fact=query';
    for my $pair (['version',3], ['producer',$pins->{producer}], ['snapshot',$pins->{snapshot}],
        ['source',$pins->{source}], ['checked-ir',$pins->{checked_ir}], ['kind',$kind], ['depth',$depth],
        ['freshness','not-asserted'], ['selection','explicit'],
        ['current-check','not-performed'], ['validation','authenticated-producing-compiler']) {
        die "header pin $i $pair->[0]\n" unless $hf[3] =~ /(?:\A| )\Q$pair->[0]=$pair->[1]\E(?: |\z)/;
    }
    my @tf = split /\t/, $tail, 4;
    die "tail $i\n" unless @tf == 4 && decode_json($tf[0]) eq $identity
        && $tf[1] eq 'level=protocol' && $tf[2] eq 'fact=continuation'
        && $tf[3] =~ /\Acursor=(none|[0-9a-f]+) facts=([0-9]+) pages-read=([0-9]+) traversal-steps=([0-9]+)\z/;
    my ($cursor,$facts) = ($1,$2);
    die "fact count $i\n" unless $facts == @lines && $facts <= $limit;
    push @continuations, $i if $cursor ne 'none';
    my $substantive = 0;
    for my $line (@lines) {
        my @f = split /\t/, $line, 4;
        die "fact framing $i\n" unless @f == 4 && defined decode_json($f[0]);
        die "fact level $i\n" unless $f[1] =~ /\Alevel=(source-claim|checked-ir|protocol)\z/;
        $substantive++ unless $f[1] eq 'level=protocol';
        if ($f[2] =~ /\Afact=(authority|layout|variant-layout|instance|ownership|loan)\z/) {
            die "checked fact label $i\n" unless $f[1] eq 'level=checked-ir';
        }
    }
    die "empty answer $i\n" unless $substantive;
    my @stats = raw("$result/raw/$tag.stderr") =~ /^LKJSCRIPT_STATS (.+)$/gm;
    die "stats count $i\n" unless @stats == 1;
    my $stats = decode_json($stats[0]);
    die "producer hash work $i\n" unless defined($stats->{producer_bytes_hashed}) && $stats->{producer_bytes_hashed} == 0;
    my $row = object("$result/raw/$tag.validation.json");
    die "derived output identity $i\n" unless $row->{answer_sha256} eq sha256_hex($out)
        && $row->{answer_bytes} == length($out) && $row->{facts} == $facts
        && $row->{wrapper_wall_ns} == $cap->{wrapper_wall_ns};
    $all_bytes += length($out); $all_facts += $facts;
    $owner_bytes += length($out) if $group eq 'owner';
    push @slice_bytes, length($out) if $group eq 'slice';
}
my $inventory_count = 0;
for my $line (split /\n/, raw($inventory)) {
    die "inventory framing\n" unless $line =~ /\A([0-9a-f]{64})  ((?:src|library|tools|tests)\/[^\r\n]+)\z/;
    my ($hash,$path) = ($1,$2);
    die "live source differs: $path\n" unless file_hash("$fixture/source/$path") eq $hash;
    $inventory_count++;
}
die "inventory count\n" unless $inventory_count == 1860;
for my $path ("$fixture/bin/producer", "$fixture/output/cold-index.image", "$fixture/output/unchanged-index.image") {
    die "canonical image mismatch\n" unless -s $path == 32196802
        && file_hash($path) eq '34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377';
}
my @ordered = sort { $a <=> $b } @times;
my @slices = sort { $a <=> $b } @slice_bytes;
my $warm = object("$attempt/raw/unchanged-index.capture.json");
my $under = grep { $_ < 100000000 } @times;
print "queries=50 raw_status_zero=50 first_time_original_query_files=0\n";
print "under_100ms=$under min_ns=$ordered[0] median_ns=",($ordered[24]+$ordered[25])/2," max_ns=$ordered[49]\n";
print "load_before=$first_load load_after=$last_load\n";
print "all_reply_bytes=$all_bytes owner_reply_bytes=$owner_bytes all_facts=$all_facts\n";
print "slice_median_bytes=",($slices[9]+$slices[10])/2," continuation_ordinals=",join(',',@continuations),"\n";
print "pause_ns=",$first_start-$warm->{finish_ns},"\n";
print "source_inventory=$inventory_count canonical_images=3 bytes_each=32196802 producer_bytes_hashed_each=0\n";
