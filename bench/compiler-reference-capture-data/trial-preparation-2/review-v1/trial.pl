#!/usr/bin/env perl
use strict;
use warnings;
use Cwd qw(abs_path);
use Fcntl qw(:DEFAULT :mode);
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use IO::Uncompress::Gunzip qw(gunzip $GunzipError);
use File::Find;

my ($mode,$run,$controls_path,$release_path,$phase)=@ARGV;
die "usage: trial.pl prepare|complete RUN CONTROLS RELEASE | row RUN CONTROLS RELEASE PHASE\n"
    unless defined($mode) && ((@ARGV==4 && $mode =~ /\A(?:prepare|complete)\z/) || (@ARGV==5 && $mode eq 'row'));
my $json=JSON::PP->new->canonical->pretty;
sub read_regular {
    my($path)=@_;
    sysopen my$f,$path,O_RDONLY|O_NOFOLLOW or die "open $path: $!\n";
    my@st=stat$f;
    die "not a bounded regular file: $path\n" unless @st && S_ISREG($st[2]) && $st[7]<=134217728;
    binmode$f;local$/;my$b=<$f>;$b='' unless defined$b;
    close$f or die "close $path: $!\n";
    die "changed/short read: $path\n" unless length($b)==$st[7];
    return$b;
}
sub new_file {
    my($path,$bytes,$permissions)=@_;
    sysopen my$f,$path,O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW,0600 or die "create $path: $!\n";
    binmode$f;print{$f}$bytes or die "write $path: $!\n";close$f or die "close $path: $!\n";
    chmod($permissions//0600,$path)==1 or die "chmod $path: $!\n";
}
sub write_json {new_file($_[0],$json->encode($_[1]));}
sub digest_record {return{bytes=>length($_[0]),sha256=>sha256_hex($_[0])};}
sub assert_pin {
    my($bytes,$pin,$label)=@_;
    die "$label digest mismatch\n" unless defined($pin->{sha256}) && sha256_hex($bytes) eq $pin->{sha256};
    die "$label size mismatch\n" if defined($pin->{bytes}) && length($bytes)!=$pin->{bytes};
}
sub plain_path {
    my($path)=@_;
    die "unsafe relative path\n" unless defined($path) && length($path) && $path !~ m{\A/|\\|[\x00-\x1f\x7f]};
    for(split m{/},$path,-1){die "unsafe path component\n" if $_ eq '' || $_ eq '.' || $_ eq '..';}
    return$path;
}
my$c=decode_json(read_regular($controls_path));
my$r=decode_json(read_regular($release_path));
die "wrong trial scope\n" unless $run eq '/home/coder/workspace/lkjscript/target/compiler-reference-capture-comparison-2'
    && $c->{run_root} eq $run && $c->{setup_root} eq '/home/coder/workspace/lkjscript/target/compiler-reference-capture-measurement-2'
    && $c->{repository} eq '/home/coder/workspace/lkjscript' && $c->{cohort} eq 'compiler-reference-capture-20261116-2';
my$repo=$c->{repository};my$setup=$c->{setup_root};
my@root=lstat$run;
die "unsafe owned trial root\n" unless @root && S_ISDIR($root[2]) && $root[4]==$> && abs_path($run) eq $run;
die "comparison release remains pending\n" unless $r->{schema} eq 'compiler-reference-capture-trial-release-2'
    && $r->{status} eq 'ROOT_RELEASED_ONCE' && $r->{released} && $r->{cohort} eq $c->{cohort}
    && $r->{setup_accepted} && $r->{gate_accepted};
die "release identity mismatch\n" unless $r->{candidate_sha256} eq $c->{producers}{B}{sha256}
    && $r->{candidate_source_commit} eq $c->{producers}{B}{source_commit}
    && $r->{setup_summary_sha256} eq 'e08b9aa27dcb08da50bf6f296c0bf347f322d0fb2a64d8594999a43920d95ece';
my$manifest=read_regular("$run/control/controls.manifest.sha256");
die "release control manifest mismatch\n" unless sha256_hex($manifest) eq $r->{controls_manifest_sha256};
my%wanted=map{$_=>1}qw(controls.json order.json protocol.md run.sh trial.pl);
my%seen;
for(split/\n/,$manifest){
    /\A([0-9a-f]{64})  ([a-z.-]+)\z/ or die "invalid control manifest\n";
    my($sha,$name)=($1,$2);
    die "unexpected/repeated control member\n" unless $wanted{$name} && !$seen{$name}++;
    assert_pin(read_regular("$run/control/$name"),{sha256=>$sha},"control $name");
}
die "incomplete controls\n" unless keys(%seen)==keys(%wanted);
my$order_bytes=read_regular("$run/control/order.json");
die "release order mismatch\n" unless sha256_hex($order_bytes) eq $r->{order_sha256};
my$order=decode_json($order_bytes);
die "wrong ordered population\n" unless ref($order) eq 'ARRAY' && @$order==12;
my@labels=qw(A1 B1 B2 A2 A3 B3);
for my$i(0..11){
    my$input=$c->{inputs}[int($i/6)];
    my$label=$labels[$i%6];my$producer=substr($label,0,1);
    my$expected_phase=sprintf('%02d-%s-%s',$i+1,$input->{name},$label);
    my$output="$run/outputs/$input->{name}.image";
    my$expected_argv=["$run/producer-$producer",'build',$input->{entry},'--source-dir',$input->{cwd},
        '--library-dir',$c->{library_root},'--runtime-source',$c->{runtime_path},
        '--no-store','--inline-limit','0','--stats','-o',$output];
    my$x=$order->[$i];
    die "ordered command changed\n" unless $x->{ordinal}==$i+1 && $x->{phase} eq $expected_phase
        && $x->{input} eq $input->{name} && $x->{label} eq $label && $x->{producer} eq $producer
        && $x->{cwd} eq $input->{cwd} && $x->{output} eq $output
        && $x->{retained_output} eq "$run/images/$expected_phase.image"
        && $x->{reference} eq sprintf('%s/images/%02d-%s-A1.image',$run,int($i/6)*6+1,$input->{name})
        && $json->encode($x->{argv}) eq $json->encode($expected_argv);
}
sub metadata_command {
    my(@argv)=@_;
    sysopen my$f,"$run/records/metadata-subcommands.jsonl",O_WRONLY|O_CREAT|O_APPEND|O_NOFOLLOW,0600 or die$!;
    print{$f}JSON::PP->new->canonical->encode({mode=>$mode,argv=>\@argv}),"\n" or die$!;
    close$f or die$!;
    open my$p,'-|',@argv or die "metadata command launch: $!\n";binmode$p;local$/;my$b=<$p>;$b='' unless defined$b;
    close$p or die "metadata command failed: $?\n";
    return$b;
}
sub tree_inventory {
    my($base)=@_;my(@files,@directories);
    my@st=lstat$base;
    die "unsafe source root\n" unless @st && S_ISDIR($st[2]) && $st[4]==$> && abs_path($base) eq $base;
    find({no_chdir=>1,follow=>0,wanted=>sub{
        my$path=$File::Find::name;return if$path eq$base;
        my@st=lstat$path;die "missing/foreign input member\n" unless @st && $st[4]==$>;
        my$relative=plain_path(substr($path,length($base)+1));
        if(S_ISDIR($st[2])){push@directories,$relative;return;}
        die "special/hardlinked input member\n" unless S_ISREG($st[2]) && $st[3]==1;
        push@files,{path=>$relative,%{digest_record(read_regular($path))},mode=>$st[2]&07777};
    }},$base);
    return{files=>[sort{$a->{path}cmp$b->{path}}@files],directories=>[sort@directories]};
}
sub verify_inputs {
    my($label)=@_;my@checks;
    for my$input(@{$c->{inputs}}){
        my$expected=decode_json(read_regular("$setup/$input->{inventory}"));
        my$actual=tree_inventory($input->{cwd});
        die "input population differs\n" unless @{$actual->{files}}==$input->{regular_members};
        die "input bytes/path/mode set changed: $input->{name}\n" unless $json->encode($actual) eq $json->encode($expected);
        write_json("$run/records/$label-$input->{name}.json",$actual);
        push@checks,{name=>$input->{name},regular_members=>scalar@{$actual->{files}},
            inventory_sha256=>sha256_hex($json->encode($actual))};
    }
    assert_pin(read_regular($c->{runtime_path}),{sha256=>$c->{runtime_sha256}},'runtime');
    return\@checks;
}
sub verify_producers {
    my@pins;
    for my$name(qw(A B)){
        my$pin=$c->{producers}{$name};
        my$bytes=read_regular("$run/producer-$name");
        assert_pin($bytes,$pin,"producer $name");
        push@pins,{producer=>$name,%{digest_record($bytes)}};
    }
    return\@pins;
}
sub timing_record {
    my($record,$status)=@_;my%time;
    for(split/\n/,read_regular("$record/time")){
        next unless /\A(wall_s|user_s|system_s|max_rss_kib|exit)=([0-9]+(?:\.[0-9]+)?)\z/;
        die "repeated GNU field\n" if exists$time{$1};$time{$1}=$2;
    }
    die "incomplete GNU timing\n" unless keys(%time)==5;
    die "GNU/wrapper status mismatch\n" unless $time{exit}=~/\A[0-9]+\z/ && $time{exit}==$status;
    die "invalid peak RSS\n" unless $time{max_rss_kib}=~/\A[0-9]+\z/;
    for my$key(qw(wall_s user_s system_s)){
        $time{$key}=~/\A([0-9]+)\.([0-9]{2})\z/ or die "unexpected GNU seconds format\n";
        $time{$key.'_cs'}=100*$1+$2;
    }
    my%capture;
    for my$key(qw(start.ns end.ns)){
        my$b=read_regular("$record/$key");chomp$b;
        die "invalid outer timestamp\n" unless$b=~/\A[0-9]{19}\z/;$capture{$key}=$b;
    }
    die "backward outer timestamps\n" if$capture{'end.ns'}<$capture{'start.ns'};
    for my$key(qw(start.utc end.utc)){
        my$b=read_regular("$record/$key");die "incomplete UTC\n" unless$b=~/\A[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z\n\z/;
        chomp$b;$capture{$key}=$b;
    }
    for my$key(qw(load-before load-after)){
        my$b=read_regular("$record/$key");die "incomplete load\n" unless$b=~/\A[0-9]+(?:\.[0-9]+)? [0-9]+(?:\.[0-9]+)? [0-9]+(?:\.[0-9]+)?\n\z/;
        chomp$b;$capture{$key}=$b;
    }
    return{gnu=>\%time,capture=>\%capture};
}
if($mode eq 'prepare'){
    die "missing retained acceptance records\n" unless ref($r->{retained_records})eq'ARRAY' && @{$r->{retained_records}};
    my(%purposes,@proofs);
    for my$proof(@{$r->{retained_records}}){
        die "invalid retained proof pin\n" unless $proof->{commit}=~/\A[0-9a-f]{40}\z/ && $proof->{sha256}=~/\A[0-9a-f]{64}\z/
            && $proof->{purpose}=~/\A(?:setup-acceptance|input-recovery|gate-acceptance|ci-observation)\z/;
        my$path=plain_path($proof->{path});die "retained record outside bench\n" unless$path=~m{\Abench/};
        my$b=metadata_command('/usr/bin/git','-C',$repo,'show',"$proof->{commit}:$path");
        assert_pin($b,$proof,'retained acceptance record');
        ++$purposes{$proof->{purpose}};push@proofs,{%$proof,verified=>JSON::PP::true};
    }
    for(qw(setup-acceptance input-recovery gate-acceptance ci-observation)){die "missing proof purpose $_\n" unless$purposes{$_};}
    die "candidate CI observation missing\n" unless ref($r->{candidate_ci})eq'HASH'
        && $r->{candidate_ci}{run_id}==43925403668
        && $r->{candidate_ci}{status}=~/\A(?:in_progress|completed_success|completed_failure)\z/
        && $r->{candidate_ci}{observed_at}=~/\A[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z\z/;
    for my$tool(@{$c->{tools}}){assert_pin(read_regular($tool->{path}),$tool,"tool $tool->{path}");}
    for my$pin(@{$c->{setup_files}}){
        my$path=plain_path($pin->{path});assert_pin(read_regular("$setup/$path"),$pin,"setup $path");
    }
    die "setup did not complete\n" unless read_regular("$setup/records/status") eq "0\n"
        && read_regular("$setup/records/completion") eq "setup-complete; generated archive verified; A/B remains unreleased\n";
    my$source_checks=verify_inputs('before');
    my$a=read_regular($c->{producers}{A}{source_path});
    assert_pin($a,$c->{producers}{A},'setup baseline A');
    new_file("$run/producer-A",$a,0500);
    my$bpin=$c->{producers}{B};
    my$seed=metadata_command('/usr/bin/git','-C',$repo,'show',"$bpin->{seed_commit}:$bpin->{seed_path}");
    assert_pin($seed,{sha256=>$bpin->{seed_sha256},bytes=>$bpin->{seed_bytes}},'candidate seed');
    new_file("$run/producer-B.gz",$seed);
    my$b='';gunzip(\$seed=>\$b)or die "candidate recovery: $GunzipError\n";
    assert_pin($b,$bpin,'recovered candidate B');
    new_file("$run/producer-B",$b,0500);
    for my$name(qw(A B)){
        my$p=$c->{producers}{$name};
        for my$key(qw(src library)){
            my$tree=metadata_command('/usr/bin/git','-C',$repo,'rev-parse',"$p->{source_commit}:$key");chomp$tree;
            die "producer source tree mismatch\n" unless$tree eq$p->{$key eq'src'?'src_tree':'library_tree'};
        }
    }
    write_json("$run/records/preflight.json",{status=>'ready-for-fixed-twelve-builds',inputs=>$source_checks,
        producers=>verify_producers(),retained_records=>\@proofs,candidate_ci=>$r->{candidate_ci},
        operating_system_cache_reset=>JSON::PP::false,comparison_executed=>JSON::PP::false});
    print "Preflight verified; no measured build executed by this helper.\n";
    exit 0;
}
if($mode eq 'row'){
    my($entry)=grep{$_->{phase}eq$phase}@$order;die "unknown fixed phase\n" unless$entry;
    my$record="$run/records/$phase";my$row={%$entry,valid=>JSON::PP::false,output_freshness=>'not established on failure'};
    my$error='';
    eval{
        my$status=read_regular("$record/status");chomp$status;
        die "invalid wrapper status\n" unless$status=~/\A[0-9]+\z/ && $status<=255;
        $row->{command_status}=0+$status;
        if(lstat$entry->{output}){
            my$image=read_regular($entry->{output});
            new_file($entry->{retained_output},$image,0500);
            $row->{output_identity}=digest_record($image);
            $row->{output_slot_retained}=JSON::PP::true;
        }
        my$timing=timing_record($record,$status);$row->{timing}=$timing;
        die "cwd mismatch\n" unless read_regular("$record/cwd") eq "$entry->{cwd}\n"
            && read_regular("$record/cwd.actual") eq "$entry->{cwd}\n";
        my$argv=decode_json(read_regular("$record/argv.json"));
        die "measured argv mismatch\n" unless$json->encode($argv)eq$json->encode($entry->{argv});
        my$expected_wrapper=['/usr/bin/time','-f','wall_s=%e\nuser_s=%U\nsystem_s=%S\nmax_rss_kib=%M\nexit=%x',
            '-o',"$record/time",'/usr/bin/timeout','--kill-after=10s','900s','/usr/bin/prlimit','--as=4294967296','--',@{$entry->{argv}}];
        my$wrapper=decode_json(read_regular("$record/wrapper-argv.json"));
        die "resource wrapper argv mismatch\n" unless$json->encode($wrapper)eq$json->encode($expected_wrapper);
        my$stdout=read_regular("$record/stdout");my$stderr=read_regular("$record/stderr");
        die "measured command failed with status $status\n" if$status!=0;
        my@stats=($stdout."\n".$stderr)=~/^LKJSCRIPT_STATS (.+)$/mg;
        die "missing/repeated compiler statistics\n" unless@stats==1;
        my$stats=decode_json($stats[0]);$row->{stats}=$stats;
        die "compiler did not finish\n" unless$stats->{route}eq'compiler' && $stats->{stopped_stage}eq'done';
        for my$key(qw(front_reused checked_reused checked_local_reused emitted_reused build_reused inline_attempts)){
            die "missing/nonzero no-store inline0 field $key\n" unless defined($stats->{$key}) && $stats->{$key}==0;
        }
        for my$key(qw(store_hits store_misses store_rejected store_writes store_retained_bytes store_evicted)){
            die "unexpected store activity $key\n" if exists($stats->{$key}) && $stats->{$key}!=0;
        }
        die "unexpected cache attribution row\n" if ($stdout."\n".$stderr)=~/^LKJSCRIPT_CACHE_IO(?:_SUMMARY)? /m;
        die "successful output missing\n" unless$row->{output_slot_retained};
        my$image=read_regular($entry->{retained_output});
        die "image is not ELF or emitted size differs\n" unless substr($image,0,4)eq"\x7fELF"
            && defined($stats->{emitted_bytes}) && $stats->{emitted_bytes}==length($image);
        my$reference=read_regular($entry->{reference});
        $row->{full_byte_equal}=($image eq$reference)?JSON::PP::true:JSON::PP::false;
        die "output differs from this input's first A1\n" unless$row->{full_byte_equal};
        $row->{reference_identity}=digest_record($reference);
        $row->{output_freshness}='successful compiler completion with matching emitted size; phase slot copied before reuse';
        $row->{valid}=JSON::PP::true;
        1;
    }or$error=$@||'unknown row validation failure';
    $row->{error}=$error if length$error;
    write_json("$record/row.json",$row);
    print$json->encode({phase=>$phase,valid=>$row->{valid},error=>$error,output_identity=>$row->{output_identity}});
    exit(length($error)?70:0);
}
my@rows;
for my$entry(@$order){
    my$row=decode_json(read_regular("$run/records/$entry->{phase}/row.json"));
    die "incomplete cohort\n" unless$row->{valid} && $row->{phase}eq$entry->{phase} && $row->{command_status}==0
        && $row->{full_byte_equal} && read_regular("$run/records/$entry->{phase}/validation.status")eq"0\n";
    push@rows,$row;
}
my$after_inputs=verify_inputs('after');my$after_producers=verify_producers();
for my$pin(@{$c->{setup_files}}){assert_pin(read_regular("$setup/$pin->{path}"),$pin,"setup after $pin->{path}");}
my@summary;
for my$input(@{$c->{inputs}}){
    my%by;
    for my$row(grep{$_->{input}eq$input->{name}}@rows){
        my$t=$row->{timing}{gnu};
        push@{$by{$row->{producer}}{wall_s}},$t->{wall_s_cs}/100;
        push@{$by{$row->{producer}}{cpu_s}},($t->{user_s_cs}+$t->{system_s_cs})/100;
        push@{$by{$row->{producer}}{max_rss_kib}},0+$t->{max_rss_kib};
    }
    my%metrics;
    for my$metric(qw(wall_s cpu_s max_rss_kib)){
        my%values;
        for my$producer(qw(A B)){
            my@v=@{$by{$producer}{$metric}};die "wrong sample count\n" unless@v==3;
            my@sorted=sort{$a<=>$b}@v;my$sum=0;$sum+=$_ for@v;
            $values{$producer}={samples=>\@v,mean=>$sum/3,median=>$sorted[1],minimum=>$sorted[0],maximum=>$sorted[2],sum=>$sum};
        }
        die "zero descriptive denominator\n" unless$values{A}{sum}>0;
        $metrics{$metric}={%values,mean_B_over_A=>$values{B}{sum}/$values{A}{sum}};
    }
    push@summary,{input=>$input->{name},metrics=>\%metrics};
}
write_json("$run/summary.json",{schema=>'compiler-reference-capture-trial-result-2',status=>'complete',
    cohort=>$c->{cohort},rows=>\@rows,inputs_after=>$after_inputs,producers_after=>$after_producers,
    summaries=>\@summary,adoption_threshold=>undef,
    interpretation=>'Descriptive fixed-order ordinary-build observations; no significance, allocation-census, generated-code milestone, or 1M claim.',
    source_ci=>$r->{candidate_ci},original_cohort=>$c->{prior_cohort}});
print "All twelve fixed builds and input/producer/output controls passed; no adoption threshold applied.\n";
