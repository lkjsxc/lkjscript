#!/usr/bin/env perl
# Independent read-only acceptance of the frozen current-producer scale cohort.
use strict; use warnings;
use JSON::PP; use Digest::SHA; use File::Find; use Text::ParseWords qw(shellwords);
use POSIX qw(ceil);
@ARGV == 1 or die "usage: audit REPOSITORY\n";
my $repo=$ARGV[0];
my $raw="$repo/bench/compiler-scale-refusal-data/measurement-1";
my $fixture="$repo/target/compiler-scale-refusal-run.tCSZMe";
sub bytes { my($p)=@_; open my $f,'<:raw',$p or die "$p: $!"; local $/; my $v=<$f>//''; close $f or die $!; return $v; }
sub digest { my($p)=@_; open my $f,'<:raw',$p or die "$p: $!"; my $v=Digest::SHA->new(256)->addfile($f)->hexdigest; close $f or die $!; return $v; }
sub object { my($p)=@_; my $v=decode_json(bytes($p)); ref($v) eq 'HASH' or die "object $p"; return $v; }
sub kv { my($p)=@_; my %v; for(split /\n/,bytes($p)) { my($k,$x)=split /=/,$_,2; die "field $p" unless defined $x && !exists $v{$k}; $v{$k}=$x; } return \%v; }
sub same { my($a,$b,$why)=@_; die "$why\n" unless $a eq $b; }
my $manifest="$repo/bench/compiler-scale-refusal-results.sha256";
same(digest($manifest),'24574f4eb2e85e21e5a8f3d190c7aa143c45012af140a5773e1230fa99bf3085','result manifest');
my $records=0;
for(split /\n/,bytes($manifest)) {
 m{\A([0-9a-f]{64})  (bench/[A-Za-z0-9_./-]+)\z} or die "manifest framing";
 my($want,$path)=($1,$2); die "manifest traversal" if $path =~ m{(?:\A|/)\.\.(?:/|\z)};
 same(digest("$repo/$path"),$want,"manifest $path"); $records++;
}
same("$records",'106','manifest count');
my @labels=qw(generator-build 100000-generate 100000-cold 1000000-generate 1000000-cold 1000000-check);
my @phases;
for my $i (0..$#labels) {
 my $label=$labels[$i]; my $m=kv("$raw/$label.meta"); my $j=object("$raw/$label.result.json");
 my $line=bytes("$raw/$label.time"); chomp $line;
 $line =~ /\A([0-9.]+)\t([0-9.]+)\t([0-9.]+)\t([0-9]+)\t([0-9]+)\z/ or die "time $label";
 my @time=($1,$2,$3,$4,$5); my @fields=qw(wall_s user_s kernel_s rss_kib gnu_exit);
 for my $n (0..4) { die "time mismatch $label/$fields[$n]" unless $j->{$fields[$n]} == $time[$n]; }
 my $exit=bytes("$raw/$label.status"); chomp $exit;
 same($exit,$m->{wrapper_exit},"wrapper $label");
 die "exit $label" unless $exit == $time[4] && $exit == ($i<4?0:1) && $time[3]>0;
 my $capture=bytes("$raw/$label.capture-status"); chomp $capture; same($capture,'0',"capture $label");
 same($j->{capture_status},'complete',"complete $label");
 for my $k (qw(load_before load_after)) {
  $m->{$k} =~ /\A[0-9.]+,[0-9.]+,[0-9.]+\z/ or die "load $label";
  same($m->{$k},$j->{$k},"load $label/$k");
 }
 my $memory=$i<4?4294967296:8589934592; my $deadline=$i<4?900:456;
 die "bound $label" unless $m->{memory_limit_bytes}==$memory && $j->{memory_limit_bytes}==$memory
  && $m->{timeout_seconds}==$deadline && $j->{timeout_seconds}==$deadline && $m->{kill_grace_seconds}==10;
 if($i>=4) {
  same(bytes("$raw/$label.stderr"),"src/elaborate.lkj:2618: R0008 stack exhausted\n","located refusal $label");
  same($j->{outcome},'located_resource_refusal',"outcome $label");
  die "invented stats $label" if exists $j->{stats} || exists $j->{store_io};
 }
 my @argv=shellwords(bytes("$raw/$label.command"));
 my @expected;
 my $cwd;
 if($i==0) {
  $cwd="$fixture/factory";
  @expected=("$fixture/factory-producer",'build','src/main.lkj','--source-dir',$cwd,
   '--library-dir',"$cwd/library",'--runtime-source',"$cwd/library/runtime.lkj",'--inline-limit','0','--no-store','-o',"$fixture/generator");
 } elsif($i==1 || $i==3) {
  my $rung=$i==1?'100000':'1000000'; $cwd="$fixture/source-$rung";
  @expected=("$fixture/generator",'generate',$cwd,$rung,'20261116');
 } else {
  my $rung=$i==2?'100000':'1000000'; my $kind=$i==5?'check':'build'; $cwd="$fixture/source-$rung";
  @expected=('env','LKJSCRIPT_STORE_IO=1',"$fixture/producer",$kind,'main.lkj','--source-dir',$cwd,
   '--library-dir',"$fixture/frozen/library",'--runtime-source',"$fixture/frozen/library/runtime.lkj",
   '--store',"$fixture/store-$rung-$kind",'--store-key','/tmp/compiler-scale-refusal-key.aD8jW6/key',
   '--store-bytes','2147483648','--inline-limit','0','--stats');
  push @expected,('-o',"$fixture/image-$rung-build") if $kind eq 'build';
 }
 same(join("\0",@argv),join("\0",@expected),"argv $label");
 same(bytes("$raw/$label.cwd"),"$cwd\n","cwd $label");
 push @phases,{label=>$label,wall_s=>0+$time[0],user_s=>0+$time[1],system_s=>0+$time[2],
  rss_kib=>0+$time[3],exit=>0+$exit,load_before=>$m->{load_before},load_after=>$m->{load_after}};
}
my $cold=object("$raw/100000-cold.result.json"); my $stats=$cold->{stats}; my $io=$cold->{store_io};
die "control records" unless $cold->{stats_records}==1 && $cold->{store_io_summary_records}==1
 && $stats->{route} eq 'compiler' && $stats->{stopped_stage} eq 'done';
for(qw(front_reused checked_reused checked_local_reused emitted_reused build_reused inline_calls inline_attempts store_hits)) {
 die "control reuse $_" unless exists($stats->{$_}) && $stats->{$_}==0;
}
for(qw(global_timing_failures global_counter_overflow pending_checkpoint)) { die "store $_" unless exists($io->{$_}) && $io->{$_}==0; }
die "control counts" unless $stats->{ir_functions}==4209 && $stats->{ir_instructions}==896361
 && $stats->{emitted_bytes}==4472259 && $stats->{store_retained_bytes}==26716768;
my $bound=object("$raw/1m-bound.json");
my $desired=ceil((2*$cold->{rss_kib}*10+262144)/1024);
my $deadline=ceil($cold->{wall_s}*30+120); $deadline=900 if $deadline>900;
die "derived bound" unless $desired==$bound->{desired_limit_mib} && $bound->{memory_limit_mib}==($desired>8192?8192:$desired)
 && $deadline==$bound->{timeout_seconds};
my @forests=(
 ['factory-inputs.sha256',"$fixture/factory",215,undef,undef],
 ['specimen-library-inputs.sha256',"$fixture/frozen",28,undef,undef],
 ['100000-inputs.sha256',"$fixture/source-100000",170,100000,3923070],
 ['1000000-inputs.sha256',"$fixture/source-1000000",1881,1000000,40327556]);
my @inputs;
for my $row (@forests) {
 my($name,$tree,$count,$want_lines,$want_bytes)=@$row; my %expected; my($lines,$source_bytes)=(0,0);
 for(split /\n/,bytes("$raw/$name")) {
  /\A([0-9a-f]{64})  (\.\/[^\r\n]+)\z/ or die "source manifest $name";
  my($hash,$relative)=($1,$2); die "source path" if $relative =~ m{(?:\A|/)\.\.(?:/|\z)} || exists $expected{$relative};
  $expected{$relative}=$hash;
 }
 my %actual;
 File::Find::find({no_chdir=>1,wanted=>sub {
  my $path=$File::Find::name; my @s=lstat($path); die "input kind" unless @s && !-l _;
  return if -d _; die "input regular" unless -f _;
  my $rel='./'.substr($path,length($tree)+1); $actual{$rel}=1;
  die "extra input $rel" unless exists $expected{$rel};
  my $data=bytes($path); same(Digest::SHA::sha256_hex($data),$expected{$rel},"live input $rel");
  if($rel =~ /\.lkj\z/) { $source_bytes+=length($data); $lines+=($data =~ tr/\n/\n/); }
 }},$tree);
 die "input set $name" unless keys(%actual)==$count && keys(%expected)==$count;
 die "source size $name" if defined($want_lines) && ($lines!=$want_lines || $source_bytes!=$want_bytes);
 push @inputs,{manifest=>$name,files=>$count,source_lines=>$lines,source_bytes=>$source_bytes};
}
same(digest("$fixture/producer"),'34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377','producer');
same(digest("$fixture/image-100000-build"),'cf6568cea58777168705a5d6ed11aa0eb57c0ca0f811eebd55f5b3c3e9370bea','control image');
die "unexpected refused image" if lstat("$fixture/image-1000000-build");
same(bytes("$raw/outcome"),"primary_resource_refusal_check_exit_1\n",'outcome');
my $done=kv("$raw/completion"); same($done->{supervisor_exit},'1','supervisor');
print JSON::PP->new->canonical->pretty->encode({result=>'accepted',manifest_records=>$records,
 phases=>\@phases,live_inputs=>\@inputs,live_input_total=>2294,control_instances=>4209,control_instructions=>896361,
 desired_limit_mib=>$desired,applied_limit_mib=>$bound->{memory_limit_mib},deadline_seconds=>$deadline,
 compiler_executions=>0,key_contents_read=>JSON::PP::false,meaning=>'one successful control; two located refusals; no new build exponent'});
