#!/usr/bin/env perl
use strict;
use warnings FATAL => 'all';
use Config ();
use Digest::SHA qw(sha256_hex);
use JSON::PP ();
use Fcntl qw(O_RDONLY O_WRONLY O_CREAT O_EXCL O_NOFOLLOW);
use IPC::Open2 qw(open2);
use IO::Handle ();
use Archive::Tar ();
use IO::Uncompress::Gunzip qw(gunzip $GunzipError);
use Time::Local qw(timegm);
use POSIX qw(strftime);
die "64-bit Perl required\n" unless $Config::Config{ivsize} >= 8;

my $main='/home/coder/workspace/lkjscript';
my $cohort="$main/target/compiler-scalar-simplify.20261116-1";
my $outer="$main/target/compiler-scalar-trial-capture.20261116-1";
my $out="$main/target/compiler-scalar-cohort-acceptance-1";
my $fixture="$cohort/fixture";
my $source="$fixture/source";
my $candidate='/home/coder/workspace/lkjscript-compiler-scalar-20261116';
my $release='c887b7c5aca7bb80d70ca26d7dfdf1a44213a163';
my $revision='26bf2c3c021dd79810a81fa1d9054d59f3768b06';
my $producer_commit='5b5ce7759c035ae0437e3512e18efc440aa50d2e';
my $oracle_sha='8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132';
my $oracle_bytes=16228028;
my $emitter_sha='f85624e3cc694f6a3a3e10c33f822789ae6084e2d5a5b41c3b1fc8fcc9bd2d64';
my $emitter_bytes=32761734;
my %inputs;
my @commands;
my $json=JSON::PP->new->canonical->pretty;
sub require_true { my ($yes,$why)=@_; die "$why\n" unless $yes; }
sub same { my ($a,$b,$why)=@_; require_true(defined($a)&&defined($b)&&$a eq $b,$why); }
sub read_bytes {
    my ($p)=@_;
    my @a=lstat($p); require_true(@a && -f _ && !-l _,"not a regular file: $p");
    sysopen(my $f,$p,O_RDONLY|O_NOFOLLOW) or die "open $p: $!\n";
    binmode $f;
    my @b=stat($f); require_true($a[0]==$b[0] && $a[1]==$b[1] && $a[7]==$b[7],"read identity: $p");
    local $/; my $s=<$f>; $s='' unless defined $s;
    close $f or die "close $p: $!\n";
    my @c=lstat($p); require_true(@c && $a[0]==$c[0] && $a[1]==$c[1] && $a[7]==$c[7] && $a[9]==$c[9],"changed while reading: $p");
    require_true(length($s)==$a[7],"short read: $p");
    my $record={bytes=>length($s),sha256=>sha256_hex($s),mode=>sprintf('%04o',$a[2]&07777)};
    if (exists $inputs{$p}) { same($json->encode($record),$json->encode($inputs{$p}),"input changed between reads: $p"); }
    $inputs{$p}=$record;
    return $s;
}
sub line { my $s=read_bytes($_[0]); require_true($s =~ s/\n\z// && $s !~ /[\r\n\0]/,"not one line: $_[0]"); return $s; }
sub decode { return JSON::PP->new->decode(read_bytes($_[0])); }
sub save {
    my ($name,$s)=@_; require_true($name !~ m{/} && $name !~ /^\./,"bad audit output");
    sysopen(my $f,"$out/$name",O_WRONLY|O_CREAT|O_EXCL,0600) or die "new output $name: $!\n";
    binmode $f; print {$f} $s or die "write $name: $!\n"; close $f or die "close $name: $!\n";
}
sub git {
    my @args=@_; push @commands,['git','-C',$main,@args];
    open my $f,'-|','git','-C',$main,@args or die "git open: $!\n";
    binmode $f; local $/; my $s=<$f>; $s='' unless defined $s;
    close $f or die "git @args exited $?\n"; return $s;
}
sub gitline { my $s=git(@_); $s =~ s/\n\z// or die "git missing LF\n"; return $s; }
sub map_lines {
    my ($text)=@_; my %m;
    for my $l (split /\n/,$text) { require_true($l =~ /\A([^=\r\n]+)=(.*)\z/,"bad key/value"); require_true(!exists $m{$1},"duplicate key/value"); $m{$1}=$2; }
    return \%m;
}
sub hash_list {
    my ($path,$prefix)=@_; my %m;
    for my $l (split /\n/,read_bytes($path)) {
        require_true($l =~ /\A([0-9a-f]{64})  (.+)\z/,"bad hash line: $path");
        my ($sha,$p)=($1,$2); $p="$prefix/$p" if $p !~ m{^/};
        require_true(!exists($m{$p}),"duplicate hash path"); $m{$p}=$sha;
        same(sha256_hex(read_bytes($p)),$sha,"recorded digest mismatch: $p");
    }
    return \%m;
}
sub exact_keys { my ($got,$want,$why)=@_; same(join("\0",sort keys %$got),join("\0",sort @$want),$why); }
sub walk {
    my ($dir,$prefix)=@_; my @st=lstat($dir); require_true(@st && -d _ && !-l _,"not a directory: $dir");
    opendir my $d,$dir or die "opendir $dir: $!\n"; my @names=sort grep {$_ ne '.' && $_ ne '..'} readdir $d; closedir $d;
    my @paths;
    for my $n (@names) {
        my $p="$dir/$n"; my $relative=length($prefix)?"$prefix/$n":$n;
        my @s=lstat($p); require_true(@s && !-l _,"bad path: $p");
        if (-d _) { push @paths,walk($p,$relative); }
        else { require_true(-f _,"special file: $p"); push @paths,$relative; }
    }
    return @paths;
}
sub nul_args { my $s=read_bytes($_[0]); require_true($s =~ s/\0\z//,"argv terminator"); return [split /\0/,$s,-1]; }
sub seconds_ns {
    my ($s)=@_; require_true($s =~ /\A([0-9]+)(?:\.([0-9]{1,9}))?\z/,"bad seconds: $s");
    my ($w,$f)=($1,$2//''); return $w*1_000_000_000+($f.('0'x(9-length($f))));
}
sub utc_seconds {
    my ($s)=@_; require_true($s =~ /\A(\d{4})-(\d\d)-(\d\d)T(\d\d):(\d\d):(\d\d)Z\z/,"bad UTC: $s");
    return timegm($6,$5,$4,$3,$2-1,$1);
}
sub load_value { my $s=line($_[0]); require_true($s =~ /\A[0-9]+\.[0-9]+ [0-9]+\.[0-9]+ [0-9]+\.[0-9]+\z/,"bad load"); return $s; }
sub interval {
    my ($base)=@_; my $start=line("$base.started.ns"); my $end=line("$base.finished.ns");
    require_true($start =~ /^\d+$/ && $end =~ /^\d+$/ && $end >= $start,"bad interval $base");
    my $su=line("$base.started.utc"); my $eu=line("$base.finished.utc");
    require_true(utc_seconds($su)*1_000_000_000 <= $start,"UTC start order $base");
    require_true((utc_seconds($eu)+1)*1_000_000_000 > $end,"UTC end order $base");
    return {start_ns=>"$start",end_ns=>"$end",launch_inclusive_wall_ns=>"".($end-$start),started_utc=>$su,finished_utc=>$eu,
        load_before=>load_value("$base.load-before"),load_after=>load_value("$base.load-after")};
}

my %frozen=(
 'bench/compiler_scalar_simplify.sh'=>'092f0187705dc5136284d2cb9deed82d6c35664138e3c631a9887c01c280aad3',
 'bench/compiler-scalar-simplify-plan.md'=>'5e7c5b605250bb3cc5e5153f1f2fac9a95a686cae5495bebfe5c6ee5798716d4',
 'bench/compiler-scalar-simplify-data/producer-pin.json'=>'d4ff362499768e6f19ca73a918c2234773c2bc62bc5bc145fa31c1dd104961f3',
 'bench/compiler-scalar-simplify-data/execution-release.md'=>'caac1ba49044e4f4a2e0072cc57ad9516078a5850a73d9f4cc7f7864658ffc8a',
 'bench/compiler-scalar-simplify-data/focused-summary.json'=>'5f2df61db6decc3eaf91dbaaa3885600a56015da1f7b77b8d7db8799b654c508',
 'bench/compiler-scalar-simplify-data/gate/summary.json'=>'0bc00b8d36c8f909490cf3edd0d4578c052b90ccbc3d0754ee4b477d06531431'
);
same(gitline('rev-parse',"$release^{commit}"),$release,'release commit');
for my $p (sort keys %frozen) {
    my $live=read_bytes("$main/$p");
    same(sha256_hex($live),$frozen{$p},"frozen live $p");
    same($live,git('show',"$release:$p"),"release blob $p");
}
my $pin=decode("$cohort/producer-pin.json");
same(read_bytes("$cohort/producer-pin.json"),read_bytes("$main/bench/compiler-scalar-simplify-data/producer-pin.json"),'copied pin');
same(read_bytes("$cohort/protocol.md"),read_bytes("$main/bench/compiler-scalar-simplify-plan.md"),'copied protocol');
same($pin->{schema},'lkjscript-scalar-six-run-1','schema');
require_true($pin->{execution_authorized} && $pin->{root_release}{single_cohort} && $pin->{root_release}{no_retries},'release grants');
same($pin->{planned_results},'target/compiler-scalar-simplify.20261116-1','cohort path');
same($pin->{emitter}{source_commit},$producer_commit,'candidate source');
same($pin->{emitter}{sha256},$emitter_sha,'candidate SHA');
same($pin->{emitter}{bytes},$emitter_bytes,'candidate size');
same($pin->{candidate_ci},'pending-until-if-main-adoption','CI qualification');
same(join(',',@{$pin->{execution_order}}),'A1,B1,B2,A2,A3,B3','declared order');
same($pin->{primary_decision},'10*sum_B_wall_ns <= 9*sum_A_wall_ns','declared decision');
for my $stage (qw(focused full_gate)) {
    my $v=$pin->{validation}{$stage}; same($v->{status},'passed',"$stage passed");
    same($v->{source_commit},$producer_commit,"$stage source");
    same($v->{producer_sha256},$emitter_sha,"$stage producer");
    same(read_bytes("$cohort/$stage.evidence"),read_bytes($v->{evidence_path}),"$stage copied proof");
    same(sha256_hex(read_bytes($v->{evidence_path})),$v->{evidence_sha256},"$stage evidence pin");
    my $s=decode($v->{evidence_path}); same($s->{source_commit},$producer_commit,"$stage summary source");
    same($s->{producer_sha256},$emitter_sha,"$stage summary producer");
    if ($stage eq 'full_gate') { same($s->{status},'passed','gate status'); same($s->{outer_status},0,'gate exit'); }
    else { for my $r (@{$s->{rows}}) { same($r->{status},0,'focused command exit'); } }
}
my $pinvalues=join("\n",@{$pin->{emitter}}{qw(path sha256 bytes source_commit source_tree library_tree)})."\n";
same(read_bytes("$cohort/pin-values"),$pinvalues,'decoded emitter values');
same(gitline('rev-parse',"$producer_commit:src"),$pin->{emitter}{source_tree},'producer src tree');
same(gitline('rev-parse',"$producer_commit:library"),$pin->{emitter}{library_tree},'producer library tree');
same(gitline('rev-parse',"$revision:src"),'3431671024d0a550b3130f2401bde067026d9bd3','witness src tree');
same(gitline('rev-parse',"$revision:library"),'e442fb3920db7eda2377f92fde0d92761f0562be','witness library tree');

same(line("$outer/release-commit"),$release,'outer release');
same(line("$outer/cwd"),$main,'outer cwd');
same(line("$cohort/cwd"),$main,'cohort cwd');
my @outer_expected=('bash','bench/compiler_scalar_simplify.sh','--pin',"$main/bench/compiler-scalar-simplify-data/producer-pin.json",'--results','target/compiler-scalar-simplify.20261116-1');
same(join("\0",@{nul_args("$outer/argv")}),join("\0",@outer_expected),'outer command');
same(line("$outer/status"),'0','outer status');
same(read_bytes("$outer/stderr"),'','outer stderr');
my $outer_inputs=hash_list("$outer/inputs.sha256",$main);
exact_keys($outer_inputs,[map {"$main/$_"} qw(bench/compiler_scalar_simplify.sh bench/compiler-scalar-simplify-plan.md bench/compiler-scalar-simplify-data/producer-pin.json)],'outer pinned file set');
hash_list("$outer/launch.sha256",$main);
my %outer_stats;
for my $k (qw(started.ns finished.ns started.utc finished.utc load-before load-after)) { $outer_stats{$k}=line("$outer/$k"); }
require_true($outer_stats{'started.ns'} =~ /^\d+$/ && $outer_stats{'finished.ns'} =~ /^\d+$/ && $outer_stats{'finished.ns'}>$outer_stats{'started.ns'},'outer ns');
$outer_stats{launch_inclusive_wall_ns}="".($outer_stats{'finished.ns'}-$outer_stats{'started.ns'});
same(read_bytes("$outer/stdout"),"ok fixed scalar-policy cohort; six outputs match the frozen seed; no policy change\nretained_results=$cohort\nstatus=0\ncompleted=true\n",'outer stdout');
my $outcome=map_lines(read_bytes("$cohort/outcome"));
same($outcome->{status},0,'cohort exit'); same($outcome->{completed},'true','cohort complete'); same($outcome->{last_phase},'complete','last phase'); same($outcome->{fixture},$fixture,'fixture path');
same(line("$cohort/checkpoint"),'execute.B.3','last timed checkpoint');
require_true(line("$cohort/free-space-kib") >= 3*1024*1024,'free space');
my $meta=map_lines(read_bytes("$cohort/metadata"));
for my $p (['frozen_revision',$revision],['emitter_sha256',$emitter_sha],['emitter_bytes',$emitter_bytes],['emitter_source_commit',$producer_commit],
 ['source_root',$source],['source_argument','src/main.lkj'],['construction_order','A,B'],['execution_order','A1,B1,B2,A2,A3,B3'],
 ['execution_output_inline_limit','0'],['execution_scalar_flag','absent'],['store','disabled'],['profile','disabled'],['warmups','0'],['retries','0'],
 ['file_cache_reset','false'],['workers','1'],['address_space_bytes','4294967296'],['watchdog_seconds','900'],['termination_grace_seconds','10'],
 ['primary_threshold','10*sum_B<=9*sum_A'],['cleanup_mode','none'],['candidate_ci','pending-until-if-main-adoption']) { same($meta->{$p->[0]},$p->[1],"metadata $p->[0]"); }

my $rowtext=read_bytes("$cohort/rows.tsv"); my @rowlines=split /\n/,$rowtext;
same(shift @rowlines,join("\t",qw(phase route repeat status wall_s user_s kernel_s rss_kib image_bytes image_sha256 load_before load_after)),'rows header');
my @expected=('construct.A.1','construct.B.1','execute.A.1','execute.B.1','execute.B.2','execute.A.2','execute.A.3','execute.B.3');
same(scalar @rowlines,8,'row count');
my (@phases,%construction,%sums,%cpus,%peaks,%walls,@raw_expected);
my $previous=$outer_stats{'started.ns'};
for my $i (0..7) {
    my @r=split /\t/,$rowlines[$i],-1; same(scalar @r,12,'row arity'); my ($kind,$route,$repeat)=@r[0..2];
    my $name=join('.',@r[0..2]); same($name,$expected[$i],'phase order'); same($r[3],0,'row exit');
    my $base="$cohort/raw/$name";
    my $artifact=$kind eq 'construct'?"$fixture/producer-$route":"$fixture/output-$route$repeat";
    my @cmd=($kind eq 'construct'?"$fixture/emitter":"$fixture/producer-$route",'build','src/main.lkj','--source-dir',$source,'--library-dir',"$source/library",'--runtime-source',"$source/library/runtime.lkj",'--no-store','--inline-limit',$kind eq 'construct'?'64':'0');
    if ($kind eq 'construct') { push @cmd,'--scalar-simplify' if $route eq 'B'; push @cmd,'--stats'; }
    push @cmd,'-o',$artifact;
    same(join("\0",@{nul_args("$base.argv")}),join("\0",@cmd),"argv $name");
    my @wrapper=('/usr/bin/time','-f','%e\t%U\t%S\t%M','-o',"$base.time",'prlimit','--as=4294967296','--','timeout','--signal=TERM','--kill-after=10s','900s',@cmd);
    same(join("\0",@{nul_args("$base.wrapper-argv")}),join("\0",@wrapper),"wrapper $name");
    same(line("$base.cwd"),$main,"cwd $name"); same(line("$base.status"),'0',"exit $name");
    my $time=line("$base.time"); same($time,join("\t",@r[4..7]),"GNU row $name");
    my @t=split /\t/,$time,-1; same(scalar @t,4,"GNU arity $name");
    my ($wall,$cpu)=(seconds_ns($t[0]),seconds_ns($t[1])+seconds_ns($t[2]));
    require_true($t[3] =~ /^\d+$/,"RSS $name");
    my $iv=interval($base);
    require_true($iv->{start_ns}>=$previous && $iv->{end_ns}<=$outer_stats{'finished.ns'},"phase/outer interval order $name");
    $previous=$iv->{end_ns};
    same(read_bytes("$base.capture"),join("\t",$iv->{start_ns},$iv->{end_ns},$iv->{load_before},$iv->{load_after},'0')."\n","capture $name");
    same($r[10],$iv->{load_before},"load before $name"); same($r[11],$iv->{load_after},"load after $name");
    same(read_bytes("$base.stdout"),'',"stdout $name");
    my $stderr=read_bytes("$base.stderr");
    if ($kind eq 'construct') {
        require_true($stderr =~ /\ALKJSCRIPT_STATS ([^\r\n]+)\n\z/,"unexpected construction diagnostics $name");
        my $s=JSON::PP->new->decode($1); same($s->{route},'compiler','stats route'); same($s->{stopped_stage},'done','stats stage');
        for my $k (qw(scalar_attempts scalar_accepted scalar_rejected scalar_constants scalar_identities scalar_branches scalar_functions scalar_trap_guard_rejections scalar_message_mismatches inline_attempts inline_accepted inline_rejected inline_calls inline_functions emitted_bytes build_reused front_reused checked_reused emitted_reused)) {
            require_true(exists($s->{$k}) && "$s->{$k}" =~ /^\d+$/,"missing integer stat $k");
        }
        require_true($s->{inline_accepted}>0 && $s->{inline_rejected}==0,'inline admission');
        for my $k (qw(build_reused front_reused checked_reused emitted_reused)) { same($s->{$k},0,"no persistent reuse $k"); }
        if ($route eq 'A') { for my $k (grep {/^scalar_/} keys %$s) { same($s->{$k},0,"baseline scalar $k"); } }
        else { require_true($s->{scalar_attempts}>0 && $s->{scalar_accepted}>0 && $s->{scalar_functions}>0 && $s->{scalar_constants}+$s->{scalar_identities}+$s->{scalar_branches}>0,'nonempty scalar admission'); same($s->{scalar_rejected},0,'scalar rejections'); same($s->{scalar_message_mismatches},0,'message mismatch'); }
        same($s->{emitted_bytes},$r[8],'stats image bytes'); $construction{$route}=$s;
    } else { same($stderr,'',"execution diagnostics $name"); push @{$walls{$route}},$wall; $sums{$route}+=$wall; $cpus{$route}+=$cpu; $peaks{$route}=$r[7] if !exists($peaks{$route}) || $r[7]>$peaks{$route}; }
    my $a=read_bytes($artifact); same(length($a),$r[8],"artifact size $name"); same(sha256_hex($a),$r[9],"artifact digest $name");
    push @phases,{name=>$name,%$iv,status=>0,argv=>\@cmd,wrapper_argv=>\@wrapper,gnu_wall_s=>$t[0],user_s=>$t[1],system_s=>$t[2],cpu_ns=>"$cpu",wall_ns=>"$wall",peak_rss_kib=>$t[3],artifact_bytes=>length($a),artifact_sha256=>sha256_hex($a)};
    push @raw_expected,map {"$name.$_"} qw(cwd argv wrapper-argv started.utc load-before started.ns stdout stderr finished.ns finished.utc load-after status capture time);
}
same(join("\0",sort walk("$cohort/raw",'')),join("\0",sort @raw_expected),'exact raw capture file set');
same(read_bytes("$cohort/construction-controls.stdout"),"both constructions admitted; B has a nonempty scalar selection\n",'construction controls');
same(read_bytes("$cohort/construction-controls.stderr"),'','construction control stderr');
for my $k (qw(inline_attempts inline_accepted inline_rejected inline_calls inline_functions front_definitions front_functions ir_functions ir_instructions)) { same($construction{A}{$k},$construction{B}{$k},"matched construction $k"); }

my $oracle=read_bytes("$fixture/expected"); same(length($oracle),$oracle_bytes,'oracle bytes'); same(sha256_hex($oracle),$oracle_sha,'oracle digest');
my $emitter=read_bytes("$fixture/emitter"); same(length($emitter),$emitter_bytes,'emitter bytes'); same(sha256_hex($emitter),$emitter_sha,'emitter digest');
same($emitter,read_bytes($pin->{emitter}{path}),'current producer full bytes'); undef $emitter;
my @parity=split /\n/,read_bytes("$cohort/parity.tsv"); same(shift @parity,join("\t",qw(row image_sha256 image_bytes seed_bytes_equal stdout_empty stderr_empty)),'parity header');
my @labels=qw(A1 B1 B2 A2 A3 B3); same(scalar @parity,6,'parity rows');
for my $i (0..5) { same($parity[$i],join("\t",$labels[$i],$oracle_sha,$oracle_bytes,qw(yes yes yes)),'parity record'); same(read_bytes("$fixture/output-$labels[$i]"),$oracle,"FULL output bytes $labels[$i]"); }
require_true(read_bytes("$fixture/producer-A") ne read_bytes("$fixture/producer-B"),'distinct producers');
hash_list("$cohort/producer-pin.sha256",$main);
my $ih=hash_list("$cohort/input-artifacts.sha256",$main); exact_keys($ih,["$fixture/source.tar","$source/seed/lkjscript.gz","$fixture/expected","$fixture/emitter"],'input manifest population');
my $ph=hash_list("$cohort/constructed-producers.sha256",$main); exact_keys($ph,["$fixture/producer-A","$fixture/producer-B"],'producer manifest population');
my $fh=hash_list("$cohort/final-artifacts.sha256",$main); exact_keys($fh,["$fixture/emitter","$fixture/producer-A","$fixture/producer-B",map {"$fixture/output-$_"} @labels],'final artifact population');
same(read_bytes("$cohort/final-input-check.stderr"),'','final controls stderr');
my $finalok=join('',map {"$_: OK\n"} ("$fixture/source.tar","$source/seed/lkjscript.gz","$fixture/expected","$fixture/emitter","$fixture/producer-A","$fixture/producer-B"));
same(read_bytes("$cohort/final-input-check.stdout"),$finalok,'final controls exact output');
same(read_bytes("$cohort/source-before.sha256"),read_bytes("$cohort/source-after.sha256"),'before/after source inventory');
my $source_before=hash_list("$cohort/source-before.sha256",$source);

# Read Git objects once through cat-file; compare the complete file bytes, not only hashes.
sub tree_records {
    my ($rev,@paths)=@_; my %records;
    for my $r (split /\0/,git('ls-tree','-rz','--full-tree',$rev,'--',@paths)) {
        require_true($r =~ /\A(100644|100755) blob ([0-9a-f]{40})\t([^\0\r\n]+)\z/,"unsupported Git entry");
        my ($mode,$oid,$path)=($1,$2,$3); require_true(!exists($records{$path}),"duplicate Git path"); $records{$path}={mode=>$mode,oid=>$oid};
    }
    return \%records;
}
my $frozen_tree=tree_records($revision,qw(src library seed/lkjscript.gz));
my $producer_tree=tree_records($producer_commit,qw(src library));
my $main_tree=tree_records($release,qw(src library));
my %oids=map { $_->{oid}=>1 } (values %$frozen_tree,values %$producer_tree,values %$main_tree);
push @commands,['git','-C',$main,'cat-file','--batch'];
my $pid=open2(my $reader,my $writer,'git','-C',$main,'cat-file','--batch');
binmode $reader; binmode $writer; $writer->autoflush(1);
my %blobs;
for my $oid (sort keys %oids) {
    print {$writer} "$oid\n" or die "cat-file write: $!\n";
    my $h=<$reader>; require_true(defined($h)&&$h =~ /\A([0-9a-f]{40}) blob ([0-9]+)\n\z/,"cat-file header");
    my ($actual,$length)=($1,$2); same($actual,$oid,'cat-file oid');
    my $s=''; while (length($s)<$length) { my $n=read($reader,my $b,$length-length($s)); require_true(defined($n)&&$n>0,'cat-file short bytes'); $s.=$b; }
    my $n=read($reader,my $lf,1); require_true(defined($n)&&$n==1&&$lf eq "\n",'cat-file LF'); $blobs{$oid}=$s;
}
close $writer or die "cat-file writer close\n"; close $reader or die "cat-file reader close\n";
waitpid($pid,0); same($?,0,'cat-file exit');
my $tar=Archive::Tar->new;
$tar->read("$fixture/source.tar",0) or die "read uncompressed archive: ".$tar->error."\n";
my (%tarfiles,$tarbytes,$sourcebytes,@source_records);
for my $f ($tar->get_files) {
    my $p=$f->full_path; next if $f->is_dir;
    require_true($f->is_file && !exists($tarfiles{$p}) && exists($frozen_tree->{$p}),"unexpected/duplicate tar member: $p");
    my $s=$f->get_content; my $g=$frozen_tree->{$p}; same($s,$blobs{$g->{oid}},"archive exact Git bytes $p");
    same($f->mode & 0111,oct($g->{mode}) & 0111,"archive executable mode $p");
    same(read_bytes("$source/$p"),$s,"archive/live source bytes $p");
    $tarfiles{$p}=1; $tarbytes+=length($s);
    push @source_records,{scope=>'frozen',path=>$p,git_oid=>$g->{oid},bytes=>length($s),sha256=>sha256_hex($s)};
    $sourcebytes+=length($s) if $p =~ m{^(?:src|library)/};
}
exact_keys(\%tarfiles,[keys %$frozen_tree],'archive exact file population');
same(join("\0",sort walk($source,'')),join("\0",sort keys %$frozen_tree),'live source exact file population');
exact_keys($source_before,[map {"$source/$_"} grep {m{^(?:src|library)/}} keys %$frozen_tree],'source inventory complete Git coverage');
my $seed=read_bytes("$source/seed/lkjscript.gz"); my $expanded='';
gunzip \$seed => \$expanded, MultiStream=>0, Transparent=>0 or die "read frozen seed gzip: $GunzipError\n";
same($expanded,$oracle,'frozen Git seed FULL expanded oracle bytes'); undef $expanded; undef $oracle;
my %scope_totals;
for my $spec (['producer',$candidate,$producer_tree],['main',$main,$main_tree]) {
    my ($name,$root,$tree)=@$spec; my @actual=(walk("$root/src",'src'),walk("$root/library",'library'));
    same(join("\0",sort @actual),join("\0",sort keys %$tree),"complete current $name source population");
    my $bytes=0;
    for my $p (sort keys %$tree) {
        my $s=read_bytes("$root/$p"); same($s,$blobs{$tree->{$p}{oid}},"current $name exact Git bytes $p"); $bytes+=length($s);
        push @source_records,{scope=>$name,path=>$p,git_oid=>$tree->{$p}{oid},bytes=>length($s),sha256=>sha256_hex($s)};
    }
    $scope_totals{$name}={files=>scalar(keys %$tree),bytes=>$bytes};
}
my $recovery=decode("$main/bench/compiler-scalar-simplify-data/source/source-retention.json");
my $retention=decode("$main/bench/compiler-scalar-simplify-data/retention-copy.json");
my ($sourcecopy)=grep {$_->{destination} eq "$main/bench/compiler-scalar-simplify-data/source"} @{$retention->{copies}};
require_true(defined($sourcecopy),'source recovery copy');
for my $name (sort keys %{$sourcecopy->{records}}) {
    my $p="$sourcecopy->{destination}/$name"; my $s=read_bytes($p);
    same(sha256_hex($s),$sourcecopy->{records}{$name}{sha256},"source recovery $name");
    same(length($s),$sourcecopy->{records}{$name}{bytes},"source recovery size $name");
    same($s,git('show',"$release:bench/compiler-scalar-simplify-data/source/$name"),"committed source recovery $name");
}

my $summary=decode("$cohort/summary.json"); my %median;
for my $r (qw(A B)) { my @sorted=sort {$a<=>$b} @{$walls{$r}}; $median{$r}=$sorted[1];
    same($summary->{runtime_sum_ns}{$r},$sums{$r},"sum $r");
    same($summary->{runtime_median_ns}{$r},$median{$r},"median $r");
    same($summary->{runtime_cpu_sum_ns}{$r},$cpus{$r},"CPU sum $r");
    same($summary->{peak_runtime_rss_kib}{$r},$peaks{$r},"peak $r");
}
my ($lhs,$rhs)=(10*$sums{B},9*$sums{A}); my $qualifies=$lhs<=$rhs;
same($summary->{qualifies}?1:0,$qualifies?1:0,'integer threshold result');
my $decision=$qualifies?'threshold-cleared-awaiting-separate-review':'discard-scalar-below-threshold';
same($summary->{decision},$decision,'decision');
same($summary->{policy_change},'none','no policy change');
my $ratio=$sums{B}/$sums{A};
require_true(abs($summary->{mean_B_over_A}-$ratio)<1e-12,'reported ratio');
require_true(abs($summary->{mean_wall_reduction_percent}-100*(1-$ratio))<1e-10,'reported reduction');
for my $r (qw(A B)) { require_true(abs($summary->{runtime_mean_ns}{$r}-$sums{$r}/3)<0.001,"reported mean $r"); }
my $image_a=$phases[0]{artifact_bytes}; my $image_b=$phases[1]{artifact_bytes};
my $report={
 schema=>'compiler-scalar-six-run-independent-acceptance-1',status=>'accepted',release_commit=>$release,
 source_commit=>$producer_commit,producer_sha256=>$emitter_sha,producer_bytes=>$emitter_bytes,frozen_revision=>$revision,
 cohort=>$cohort,outer_capture=>$outer,phases=>\@phases,outer=>\%outer_stats,
 raw_phase_files=>scalar(@raw_expected),output_full_comparisons=>6,output_bytes_each=>$oracle_bytes,output_sha256=>$oracle_sha,
 archived_files=>scalar(keys %tarfiles),archived_payload_bytes=>$tarbytes,frozen_source_files=>scalar(keys %$source_before),frozen_source_bytes=>$sourcebytes,
 current_source_verification=>\%scope_totals,construction_stats=>\%construction,
 runtime_sum_ns=>{map {$_=>"$sums{$_}"} qw(A B)},runtime_cpu_sum_ns=>{map {$_=>"$cpus{$_}"} qw(A B)},
 runtime_mean_s=>{map {$_=>$sums{$_}/3e9} qw(A B)},runtime_median_s=>{map {$_=>$median{$_}/1e9} qw(A B)},peak_runtime_rss_kib=>\%peaks,
 primary_integer_test=>{lhs_10_sum_B_ns=>"$lhs",rhs_9_sum_A_ns=>"$rhs",qualifies=>$qualifies?JSON::PP::true:JSON::PP::false},
 mean_B_over_A=>$ratio,mean_wall_reduction_percent=>100*(1-$ratio),decision=>$decision,policy_change=>'none',
 image=>{A_bytes=>$image_a,B_bytes=>$image_b,delta_bytes=>$image_b-$image_a,B_over_A=>$image_b/$image_a,growth_percent=>100*($image_b/$image_a-1)},
 construction=>{wall_B_over_A=>$phases[1]{wall_ns}/$phases[0]{wall_ns},cpu_B_over_A=>$phases[1]{cpu_ns}/$phases[0]{cpu_ns},rss_B_over_A=>$phases[1]{peak_rss_kib}/$phases[0]{peak_rss_kib}},
 candidate_ci=>$pin->{candidate_ci},baseline_ci_context=>$pin->{baseline_ci_context},
 source_recovery=>$recovery,frozen_pins=>\%frozen,
 qualifications=>[
  'All eight raw commands/status/cwd/GNU/ns/load/capture rows were independently joined. The GNU format has no %x field; wrapper status is the retained exit observation.',
  'All six outputs were compared in full to the held expected image; that image was independently compared in full to the decompressed exact Git seed blob.',
  'The uncompressed source archive and every live frozen source file were compared in full to exact Git blobs. Complete current candidate and main src/library populations were also compared to their declared revisions.',
  'No compiler, emitted native program, frozen harness, benchmark, official profile reader or accounting helper was executed. Only independent offline parsing, file comparison, Git reads and in-memory gzip reading were used. No compression, deletion, source change, process scan or key read occurred.',
  'The arithmetic mean uses all three GNU wall observations per policy. Medians are secondary; construction, GNU elapsed and launch-inclusive ns boundaries remain separate.',
  'One fixed-order six-run cohort on a shared host with no cache reset is an observed policy comparison, not a randomized causal speedup, Rust-O ratio, backend floor or global optimization bound.',
  'Candidate CI remains pending until/if adoption. The named baseline CI observation does not validate this experimental producer; no new CI lookup is part of this audit.',
  'Source recovery bundle bytes and retained records were checked against the committed release. This audit does not claim a fresh-repository bundle reconstruction.',
  'No retry or additional measurement was performed; the below-threshold decision follows the prospectively frozen integer test.'
 ]
};
save('source-comparisons.json',$json->encode(\@source_records));
save('git-read-commands.json',$json->encode(\@commands));
save('input-inventory.json',$json->encode(\%inputs));
save('acceptance.json',$json->encode($report));
my $md="# Scalar cohort: independent acceptance\n\nThe completed cohort is accepted as a retained observation. The scalar candidate misses the frozen 10% threshold and is to be discarded without broadening or another trial.\n\n";
$md.="Release: \`$release\`. Candidate source: \`$producer_commit\`. Producer: \`$emitter_sha\` ($emitter_bytes bytes).\n\n";
$md.="| Phase | GNU wall s | CPU s | Peak RSS KiB | Load before → after |\n| --- | ---: | ---: | ---: | --- |\n";
for my $p (@phases) { $md.=sprintf("| %s | %s | %.2f | %s | %s → %s |\n",$p->{name},$p->{gnu_wall_s},$p->{cpu_ns}/1e9,$p->{peak_rss_kib},$p->{load_before},$p->{load_after}); }
$md.=sprintf("\nAll three execution observations give A mean %.9f s and B mean %.9f s: B/A %.12f, an observed %.9f%% wall reduction. The exact test is **%s ≤ %s**, which is false. Runtime CPU sums are %.2f s and %.2f s. Medians %.2f s and %.2f s remain secondary.\n\n",$sums{A}/3e9,$sums{B}/3e9,$ratio,100*(1-$ratio),$lhs,$rhs,$cpus{A}/1e9,$cpus{B}/1e9,$median{A}/1e9,$median{B}/1e9);
$md.=sprintf("A and B images are %d and %d bytes: delta %d bytes (%.9f%%). Construction wall increases %.6f%%, CPU %.6f%% and peak RSS %.6f%%; these are separate from execution and do not isolate individual rewrite/recheck costs.\n\n",$image_a,$image_b,$image_b-$image_a,100*($image_b/$image_a-1),100*($phases[1]{wall_ns}/$phases[0]{wall_ns}-1),100*($phases[1]{cpu_ns}/$phases[0]{cpu_ns}-1),100*($phases[1]{peak_rss_kib}/$phases[0]{peak_rss_kib}-1));
$md.="B admits one scalar rewrite: $construction{B}{scalar_constants} constants, $construction{B}{scalar_identities} identities and $construction{B}{scalar_branches} branches in $construction{B}{scalar_functions} functions; zero scalar rejections, message mismatches or trap guards. Both constructions admit the same inlining population. These counts do not prove removable runtime cost.\n\n";
$md.="The audit joined all eight raw captures and verified all six full $oracle_bytes-byte outputs, the exact Git seed oracle, ".scalar(keys %tarfiles)." archive files, ".scalar(keys %$source_before)." frozen src/library files, $scope_totals{producer}{files} current candidate files and $scope_totals{main}{files} current main files. Input and output manifests, committed release blobs, passed validation summaries and current emitter bytes agree. Construction stderr contains exactly one statistics record each; runtime stdout/stderr are empty.\n\n";
$md.="The result is one fixed-order A1/B1/B2/A2/A3/B3 cohort, with no cache reset or randomized causal claim. Candidate CI remains pending until/if adoption; baseline CI is context only. GNU values have their recorded centisecond resolution and are not replaced by nanosecond wrapper durations. Source bundle retention was compared, without a fresh-repository reconstruction. No measured program, benchmark or compiler was rerun, and nothing was compressed, deleted or modified outside this audit directory.\n";
save('report.md',$md);
print "accepted: eight phases, six full output comparisons; decision=$decision\n";
