#!/usr/bin/perl
use strict;
use warnings FATAL => 'all';
use Config;
use JSON::PP;
use Digest::SHA qw(sha256_hex sha1_hex);
use Fcntl qw(O_RDONLY O_NOFOLLOW O_WRONLY O_CREAT O_EXCL S_ISREG S_ISDIR);
use File::Find;
use POSIX qw(strftime);

my $M='/home/coder/workspace/lkjscript';
my $R="$M/target/compiler-reference-capture-comparison-2";
my $S="$M/target/compiler-reference-capture-measurement-2";
my $O="$M/target/compiler-reference-capture-comparison-launch-2";
my $D="$M/target/compiler-reference-capture-comparison-acceptance-2";
my $release_commit='0547c7041de0710dbe6e3e98b590a9231fc32329';
my $release_hash='6ea5952f0c0f366f86a829a85bb82292d8a9a656cd488d52305287eaae63d3ff';
my $summary_hash='b531ef397e7420418562125d44fadc6399139ac3cc8c3c58c37d1a45c415c33c';
my $J=JSON::PP->new->canonical->allow_nonref;
my (%observed,@git_calls);
sub need ($$) { die "$_[1]\n" unless $_[0]; }
need($Config{ivsize}>=8,'64-bit exact integer reader required');
sub read_bytes {
 my($p)=@_; my @a=lstat($p); need(@a && S_ISREG($a[2]) && $a[3]==1,"non-regular/multiple-link input: $p");
 sysopen(my $f,$p,O_RDONLY|O_NOFOLLOW) or die "open $p: $!";
 binmode $f; my @b=stat($f); need($a[0]==$b[0]&&$a[1]==$b[1]&&$a[7]==$b[7],"identity changed: $p");
 local $/;my $s=<$f>; $s='' unless defined $s; close $f or die "close $p: $!";
 need(length($s)==$a[7],"short input: $p");
 $observed{$p}={bytes=>length($s),sha256=>sha256_hex($s)};
 return $s;
}
sub line { my $v=read_bytes($_[0]);need($v =~ s/\n\z//,"missing newline $_[0]");need($v !~ /\n/,"multiple lines $_[0]");return $v; }
sub json { return decode_json(read_bytes($_[0])); }
sub same ($$$) {need($J->encode($_[0]) eq $J->encode($_[1]),$_[2]);}
sub pin ($$) {my($p,$h)=@_;my $b=read_bytes($p);need(sha256_hex($b) eq $h,"hash mismatch $p");return $b;}
sub save {
 my($name,$bytes)=@_;sysopen(my $f,"$D/$name",O_WRONLY|O_CREAT|O_EXCL,0600) or die "claim $name: $!";
 binmode $f;print $f $bytes or die "write $name: $!";close $f or die "close $name: $!";
}
sub save_json {save($_[0],JSON::PP->new->canonical->pretty->encode($_[1]));}
sub git {
 my @args=@_;push @git_calls,[@args];open my $f,'-|','/usr/bin/git','-C',$M,@args or die "git start: $!";
 binmode $f;local $/;my $b=<$f>;need(close($f),"git failed @args");return defined($b)?$b:'';
}
sub uint ($$) {my($n,$where)=@_;need(defined($n)&&!ref($n)&&$J->encode($n)=~/\A(?:0|[1-9][0-9]*)\z/,"unsigned JSON number $where");return $n;}
sub cents { my($s)=@_;need($s=~/\A(\d+)\.(\d{2})\z/,"bad GNU decimal $s");return $1*100+$2;}
sub raw_time {
 my($p)=@_;my $b=read_bytes($p);
 need($b =~ /\Awall_s=(\d+\.\d{2})\nuser_s=(\d+\.\d{2})\nsystem_s=(\d+\.\d{2})\nmax_rss_kib=(\d+)\nexit=(\d+)\n\z/,"bad complete GNU timing $p");
 my($w,$u,$s,$rss,$exit)=($1,$2,$3,$4,$5);need($exit==0,"GNU nonzero $p");
 return {wall_s=>$w,user_s=>$u,system_s=>$s,max_rss_kib=>0+$rss,exit=>0+$exit,
 wall_cs=>cents($w),user_cs=>cents($u),system_cs=>cents($s),cpu_cs=>cents($u)+cents($s)};
}
sub capture {
 my($p,$argv,$cwd)=@_;
 same(json("$p/argv.json"),$argv,"payload argv $p");
 same(json("$p/wrapper-argv.json"),['/usr/bin/time','-f','wall_s=%e\nuser_s=%U\nsystem_s=%S\nmax_rss_kib=%M\nexit=%x','-o',"$p/time",
 '/usr/bin/timeout','--kill-after=10s','900s','/usr/bin/prlimit','--as=4294967296','--',@$argv],"wrapper argv $p");
 need(line("$p/status") eq '0',"status $p");need(line("$p/cwd") eq $cwd&&line("$p/cwd.actual") eq $cwd,"cwd $p");
 need(line("$p/stdin") eq '/dev/null',"stdin $p");my $free=line("$p/free-bytes");need($free=~/\A\d+\z/&&$free>=3221225472,"free bytes $p");
 my %c;for my $n(qw(start.ns end.ns start.utc end.utc load-before load-after)){$c{$n}=line("$p/$n");}
 for my $n(qw(start.ns end.ns)){need($c{$n}=~/\A\d{19}\z/,"nanoseconds $p/$n");}
 need($c{'start.ns'}<$c{'end.ns'},"interval $p");
 for my $n(qw(load-before load-after)){need($c{$n}=~/\A\d+\.\d+ \d+\.\d+ \d+\.\d+\z/,"load $p/$n");}
 for my $stem(qw(start end)){
  my $sec=substr($c{"$stem.ns"},0,-9);my $match=0;
  for my $delta(-1,0,1){$match=1 if strftime('%Y-%m-%dT%H:%M:%SZ',gmtime($sec+$delta)) eq $c{"$stem.utc"};}
  need($match,"coarse UTC $p/$stem");
 }
 my $t=raw_time("$p/time");need($c{'end.ns'}-$c{'start.ns'}+10000000 >= $t->{wall_cs}*10000000,"GNU/capture interval $p");
 return {path=>$p,argv=>$argv,cwd=>$cwd,timing=>$t,capture=>\%c};
}
sub close_float ($$$) {my($a,$b,$where)=@_;need(defined($a)&&abs($a-$b)<=1e-8*(1+abs($b)),"summary arithmetic $where");}

my %frozen=(
 'controls.manifest.sha256'=>'3c3bb9864c1d57cbb44d5f1f56923a3d230971d9121bac957ca0861c49fe1fc3',
 'controls.json'=>'1e8dbcb3b39d216ecebaa10b666793c770138bcfbe13c8e5d794615aba8856f0',
 'trial.pl'=>'3b2be33e636e0893b2838e8e12a14e8783024909877341f4cec3be1dbe1c0699',
 'run.sh'=>'c768836db5339bb9f007ccc96ee783b30a76c8561e40d9124c1d873188ee0a40',
 'order.json'=>'c3d13ecf4e075c3a59c8d263a4986a0525614bd119718333d1d5fe254aec739d',
 'protocol.md'=>'b7679151c15856b072f76d295d3fd5e773cb6a107183fbc681870d4ffc10c479',
 'release.json'=>$release_hash);
for my $n(sort keys %frozen){pin("$R/control/$n",$frozen{$n});}
my $manifest=read_bytes("$R/control/controls.manifest.sha256");my @ml=split /\n/,$manifest;need(@ml==5,'five frozen controls');
for(@ml){need(/\A([0-9a-f]{64})  ([A-Za-z0-9_.-]+)\z/,'control manifest format');need(exists($frozen{$2})&&$frozen{$2} eq $1,'control manifest pin');}
my $C=json("$R/control/controls.json");my $release=json("$R/control/release.json");my $order=json("$R/control/order.json");
my $summary=decode_json(pin("$R/summary.json",$summary_hash));my $pre=json("$R/records/preflight.json");
need($release->{released}&&$release->{status} eq 'ROOT_RELEASED_ONCE','actual release');
need($release->{controls_manifest_sha256} eq $frozen{'controls.manifest.sha256'},'release control pin');
need($C->{repository} eq $M&&$C->{run_root} eq $R&&$C->{setup_root} eq $S,'literal roots');
need(!defined($summary->{adoption_threshold})&&$summary->{status} eq 'complete','complete/no threshold');
same($summary->{source_ci},$release->{candidate_ci},'recorded release CI');
same($pre->{candidate_ci},$release->{candidate_ci},'preflight CI');

my %committed_control;
for my $n(sort keys %frozen){
 my $p="bench/compiler-reference-capture-data/trial-preparation-2/$n";
 my $b=git('show',"$release_commit:$p");need(sha256_hex($b) eq $frozen{$n},"committed control $n");$committed_control{$n}=$p;
}
my @retained_proofs;
for my $p(@{$release->{retained_records}}){
 my $b=git('show',"$p->{commit}:$p->{path}");need(sha256_hex($b) eq $p->{sha256},"committed proof $p->{path}");
 need(length($b)==$p->{bytes},"proof bytes $p->{path}") if exists$p->{bytes};
 push @retained_proofs,{%$p,verified_bytes=>length($b)};
}
for my $p(@{$C->{setup_files}}){my $b=pin("$S/$p->{path}",$p->{sha256});need(length($b)==$p->{bytes},"setup bytes $p->{path}");}
my @producer_proof;
for my $which(qw(A B)){
 my $p=$C->{producers}{$which};my $b=pin("$R/producer-$which",$p->{sha256});need(length($b)==$p->{bytes},"producer bytes $which");
 my $src=git('rev-parse',"$p->{source_commit}:src");chomp$src;my $lib=git('rev-parse',"$p->{source_commit}:library");chomp$lib;
 need($src eq $p->{src_tree}&&$lib eq $p->{library_tree},"producer source trees $which");
 my $seed=git('show',"$p->{seed_commit}:seed/lkjscript.gz");need(sha256_hex($seed) eq $p->{seed_sha256},"producer seed commit $which");
 if($which eq 'A'){need(read_bytes("$S/producer-A") eq $b,'A full byte copy');}
 else{need(read_bytes("$R/producer-B.gz") eq $seed,'B retained seed full bytes');}
 my($before)=grep{$_->{producer} eq $which}@{$pre->{producers}};my($after)=grep{$_->{producer} eq $which}@{$summary->{producers_after}};
 same($before,{producer=>$which,sha256=>$p->{sha256},bytes=>$p->{bytes}},"producer before $which");
 same($after,$before,"producer after $which");
 push @producer_proof,{producer=>$which,sha256=>$p->{sha256},bytes=>$p->{bytes},source_commit=>$p->{source_commit},src_tree=>$src,library_tree=>$lib,seed_commit=>$p->{seed_commit},seed_sha256=>$p->{seed_sha256}};
}
pin($C->{runtime_path},$C->{runtime_sha256});
my @input_proof;
my $git_members=git('ls-tree','-r','-z','c887b7c5aca7bb80d70ca26d7dfdf1a44213a163');
my %git_blob;for(split/\0/,$git_members){need(/\A([0-7]+) blob ([0-9a-f]{40})\t(.+)\z/s,'Git tree record');$git_blob{$3}=$2;}
for my $input(@{$C->{inputs}}){
 my $name=$input->{name};my $before=read_bytes("$R/records/before-$name.json");my $after=read_bytes("$R/records/after-$name.json");
 need($before eq $after&&$before eq read_bytes("$S/$input->{inventory}"),"before/after/source inventory $name");
 my $inv=decode_json($before);need(@{$inv->{files}}==$input->{regular_members},"member count $name");
 my %expected=map{$_->{path}=>$_}@{$inv->{files}};my %dirs=map{$_=>1}@{$inv->{directories}};
 my(%seen,%seen_dir);my($bytes,$sourcebytes,$lines,$sourcefiles,$git_count)=(0,0,0,0,0);
 find({no_chdir=>1,wanted=>sub{
  my $p=$File::Find::name;return if $p eq $input->{cwd};my $rel=substr($p,length($input->{cwd})+1);my @st=lstat$p;
  need(@st,'input lstat');if(S_ISDIR($st[2])){need(exists$dirs{$rel},"extra directory $name/$rel");$seen_dir{$rel}=1;return;}
  need(exists$expected{$rel},"extra file $name/$rel");need(!$seen{$rel}++,"duplicate file $rel");
  my $b=read_bytes($p);my $e=$expected{$rel};
  need(length($b)==$e->{bytes}&&sha256_hex($b) eq $e->{sha256}&&($st[2]&07777)==$e->{mode},"live input $name/$rel");
  $bytes+=length$b;
  if($rel=~/\.lkj\z/){$sourcefiles++;$sourcebytes+=length$b;$lines+=($b=~tr/\n/\n/);}
  if($name eq 'compiler'){need(exists$git_blob{$rel}&&sha1_hex('blob '.length($b)."\0".$b) eq $git_blob{$rel},"Git blob $rel");$git_count++;}
 }},$input->{cwd});
 need(keys(%seen)==keys(%expected)&&keys(%seen_dir)==keys(%dirs),"complete live set $name");
 if($name eq 'new-100k'){need($sourcefiles==169&&$sourcebytes==3923070&&$lines==100000,'generated frozen shape counts');}
 my($pb)=grep{$_->{name} eq $name}@{$pre->{inputs}};my($pa)=grep{$_->{name} eq $name}@{$summary->{inputs_after}};
 same($pb,{name=>$name,inventory_sha256=>sha256_hex($before),regular_members=>scalar(keys%seen)},"preflight input $name");
 same($pa,$pb,"postflight input $name");
 push @input_proof,{name=>$name,regular_files=>scalar(keys%seen),directories=>scalar(keys%seen_dir),bytes=>$bytes,source_files=>$sourcefiles,source_bytes=>$sourcebytes,source_lines=>$lines,git_blob_matches=>$git_count,inventory_sha256=>sha256_hex($before)};
}

my @labels=qw(A1 B1 B2 A2 A3 B3);my(@expected_order,@rows,@timeline,%group,%refs,@outputs);
my $ordinal=0;my $phase_table="ordinal\tinput\tlabel\tproducer\tphase\toutcome\tcommand_status\tvalidation_status\n";
my @base_meta=('/usr/bin/perl',"$R/control/trial.pl");
my @tail_meta=($R,"$R/control/controls.json","$R/control/release.json");
my $pc=capture("$R/records/preflight",[@base_meta,'prepare',@tail_meta],$M);need(read_bytes("$R/records/preflight/stderr") eq '','preflight stderr');read_bytes("$R/records/preflight/stdout");push@timeline,$pc;
for my $input(@{$C->{inputs}}){
 my $name=$input->{name};my $ref;
 for my $label(@labels){
  $ordinal++;my $which=substr($label,0,1);my $phase=sprintf('%02d-%s-%s',$ordinal,$name,$label);
  $ref="$R/images/$phase.image" if $label eq 'A1';
  my $output="$R/outputs/$name.image";my $retained="$R/images/$phase.image";
  my @argv=("$R/producer-$which",'build',$input->{entry},'--source-dir',$input->{cwd},'--library-dir',"$S/compiler-input/library",'--runtime-source',"$S/compiler-input/library/runtime.lkj",'--no-store','--inline-limit','0','--stats','-o',$output);
  my $expected={ordinal=>$ordinal,phase=>$phase,input=>$name,label=>$label,producer=>$which,cwd=>$input->{cwd},entry=>$input->{entry},output=>$output,retained_output=>$retained,reference=>$ref,argv=>\@argv};
  push@expected_order,$expected;
  my $p="$R/records/$phase";my $cap=capture($p,\@argv,$input->{cwd});push@timeline,$cap;
  need(read_bytes("$p/stdout") eq '',"compiler stdout $phase");my $stderr=read_bytes("$p/stderr");
  need($stderr=~/\ALKJSCRIPT_STATS (\{[^\n]+\})\n\z/,"unexpected compiler diagnostic $phase");my $stats=decode_json($1);
  need($stats->{route} eq 'compiler'&&$stats->{stopped_stage} eq 'done',"stats route $phase");
  for my $k(qw(build_reused checked_local_reused checked_reused emitted_reused front_reused inline_attempts inline_accepted inline_calls inline_functions inline_rejected)){
   need(uint($stats->{$k},"$phase/$k")==0,"reuse/inline $phase/$k");
  }
  for my $k(qw(store_get_attempts store_put_attempts)){need(uint($stats->{$k},"$phase/$k")==0,"store count $phase/$k") if exists$stats->{$k};}
  uint($stats->{emitted_bytes},"$phase/emitted_bytes");
  my $row=json("$p/row.json");
  # The frozen completer converts the raw digit-string RSS to a JSON number.
  # Normalize only this one documented integer field; all other fields stay exact.
  my $rss=$row->{timing}{gnu}{max_rss_kib};
  need(defined($rss)&&!ref($rss)&&"$rss"=~/\A(?:0|[1-9][0-9]*)\z/,"row unsigned RSS $phase");
  $row->{timing}{gnu}{max_rss_kib}=0+$rss;
  same($row,$summary->{rows}[$ordinal-1],"row versus summary $phase");
  for my $k(keys%$expected){same($row->{$k},$expected->{$k},"row plan $phase/$k");}
  same($row->{stats},$stats,"raw stats $phase");
  need($row->{valid}&&$row->{full_byte_equal}&&$row->{output_slot_retained}&&$row->{command_status}==0,"validated row $phase");
  for my $k(qw(start.ns end.ns start.utc end.utc load-before load-after)){need("$row->{timing}{capture}{$k}" eq $cap->{capture}{$k},"row raw capture $phase/$k");}
  my $g=$row->{timing}{gnu};for my $k(qw(wall_s user_s system_s max_rss_kib exit)){need("$g->{$k}" eq "$cap->{timing}{$k}","row GNU $phase/$k");}
  for my $k(qw(wall user system)){need($g->{$k.'_s_cs'}==$cap->{timing}{$k.'_cs'},"row exact centiseconds $phase/$k");}
  need(line("$p/validation.status") eq '0',"validation status $phase");
  my $b=read_bytes($retained);$refs{$name}=$b if $label eq 'A1';need($b eq $refs{$name},"retained full byte comparison $phase");
  my $id={bytes=>length($b),sha256=>sha256_hex($b)};
  need(length($b)==$stats->{emitted_bytes},"stats image size $phase");same($row->{output_identity},$id,"output identity $phase");same($row->{reference_identity},$id,"reference identity $phase");
  push@outputs,{path=>$retained,%$id,full_byte_equal_to=>$ref};
  push@rows,{%$expected,timing=>$cap->{timing},capture=>$cap->{capture},image=>$id,stats_sha256=>sha256_hex($stderr)};
  push@{$group{$name}{$which}{wall_s}},$cap->{timing}{wall_cs};
  push@{$group{$name}{$which}{cpu_s}},$cap->{timing}{cpu_cs};
  push@{$group{$name}{$which}{max_rss_kib}},$cap->{timing}{max_rss_kib};
  my $v="$R/records/validation-$phase";my $vc=capture($v,[@base_meta,'row',@tail_meta,$phase],$M);
  need(read_bytes("$v/stderr") eq '',"validation stderr $phase");read_bytes("$v/stdout");push@timeline,$vc;
  $phase_table.=join("\t",$ordinal,$name,$label,$which,$phase,'passed',0,0)."\n";
 }
 my $b=read_bytes("$R/outputs/$name.image");need($b eq $refs{$name},"final slot full bytes $name");
 push@outputs,{path=>"$R/outputs/$name.image",bytes=>length($b),sha256=>sha256_hex($b),full_byte_equal_to=>$ref};
}
need(@{$summary->{rows}}==12,'exact summary rows');same($order,\@expected_order,'fixed twelve-command order');
need(read_bytes("$R/records/phases.tsv") eq $phase_table,'complete raw phase table');
my $post=capture("$R/records/postflight",[@base_meta,'complete',@tail_meta],$M);need(read_bytes("$R/records/postflight/stderr") eq '','postflight stderr');read_bytes("$R/records/postflight/stdout");push@timeline,$post;
opendir(my $rd,"$R/records") or die$!;my @actual_dirs=sort grep{my @s=lstat("$R/records/$_");$_ ne'.'&&$_ ne'..'&&S_ISDIR($s[2])}readdir$rd;closedir$rd;
my @expected_dirs=sort map{my $p=$_->{path};$p=~s{.*/}{};$p}@timeline;same(\@actual_dirs,\@expected_dirs,'26 and only 26 command directories');
need(line("$R/records/status") eq '0'&&read_bytes("$R/records/completion") eq "complete: exactly twelve builds; all byte/input/producer controls passed\n",'completion controls');
need(line("$R/records/release.sha256") eq $release_hash,'held release hash');
need(read_bytes("$R/records/environment") eq "PATH=/usr/bin:/bin\nLC_ALL=C\nTZ=UTC\nLKJSCRIPT_STORE_IO unset\nno store or key; no cache reset\n",'environment');
need(line("$O/release-commit") eq $release_commit&&line("$O/status") eq '0'&&read_bytes("$O/stderr") eq '','outer release/status');
my $outer_time=raw_time("$O/time");my %outer;
for my $k(qw(start.ns end.ns start.utc end.utc load-before load-after)){$outer{$k}=line("$O/$k");}
need(line("$O/cwd") eq $M,'outer cwd');
my $outer_argv="bash target/compiler-reference-capture-trial-preparation-2/run.sh --execute-after-root-release $release_hash";
need(line("$O/argv") eq $outer_argv,'outer exact invocation');
my $control_lines="$release_hash  target/compiler-reference-capture-trial-preparation-2/release.json\n$frozen{'controls.manifest.sha256'}  target/compiler-reference-capture-trial-preparation-2/controls.manifest.sha256\n";
need(read_bytes("$O/controls-before.sha256") eq $control_lines&&read_bytes("$O/controls-after.sha256") eq $control_lines,'outer immutable controls');
read_bytes("$O/stdout");
my $last=$outer{'start.ns'};my @intervals;
for my $c(@timeline){need($c->{capture}{'start.ns'}>=$last,"non-overlap/order $c->{path}");$last=$c->{capture}{'end.ns'};push@intervals,{path=>$c->{path},start_ns=>"$c->{capture}{'start.ns'}",end_ns=>"$last"};}
need($last<=$outer{'end.ns'},'outer contains all commands');
my @computed;
for my $ix(0,1){
 my $name=$C->{inputs}[$ix]{name};my $published=$summary->{summaries}[$ix];need($published->{input} eq $name,'summary input order');my %metrics;
 for my $metric(qw(wall_s cpu_s max_rss_kib)){
  my $unit=$metric eq 'max_rss_kib'?1:100;my %out;
  for my $producer(qw(A B)){
   my @v=@{$group{$name}{$producer}{$metric}};need(@v==3,'three samples');my @sorted=sort{$a<=>$b}@v;my $sum=0;$sum+=$_ for@v;
   my $p=$published->{metrics}{$metric}{$producer};need(@{$p->{samples}}==3,'published three samples');
   for my $i(0..2){close_float($p->{samples}[$i],$v[$i]/$unit,"$name/$metric/$producer/sample$i");}
   my %numeric=(sum=>$sum/$unit,mean=>$sum/(3*$unit),median=>$sorted[1]/$unit,minimum=>$sorted[0]/$unit,maximum=>$sorted[-1]/$unit);
   for my $k(keys%numeric){close_float($p->{$k},$numeric{$k},"$name/$metric/$producer/$k");}
   $out{$producer}={integer_unit=>$unit==1?'KiB':'centisecond',integer_samples=>\@v,integer_sum=>$sum,mean_numerator=>$sum,mean_denominator=>3*$unit,%numeric};
  }
  my $ratio=$out{B}{integer_sum}/$out{A}{integer_sum};close_float($published->{metrics}{$metric}{mean_B_over_A},$ratio,"$name/$metric/ratio");
  $metrics{$metric}={%out,mean_ratio_numerator=>$out{B}{integer_sum},mean_ratio_denominator=>$out{A}{integer_sum},mean_B_over_A=>$ratio,percent_B_minus_A=>100*($ratio-1)};
 }
 push@computed,{input=>$name,metrics=>\%metrics};
}

# Inventory only the two authorized frozen evidence roots, without executing payloads.
my %raw_inventory;
for my $root($R,$O){
 find({no_chdir=>1,wanted=>sub{my $p=$File::Find::name;my @s=lstat$p;return if S_ISDIR($s[2]);read_bytes($p) unless exists$observed{$p};$raw_inventory{$p}=$observed{$p};}},$root);
}
my $result={
 schema=>'compiler-reference-capture-independent-comparison-acceptance-2',status=>'accepted',release_commit=>$release_commit,release_sha256=>$release_hash,summary_sha256=>$summary_hash,
 measured_builds=>12,metadata_commands=>14,ordered_nonoverlapping_commands=>26,retained_images=>12,final_slots=>2,full_byte_comparisons=>14,
 source_regular_members=>1912,compiler_git_blob_matches=>1742,setup_pins_verified=>scalar(@{$C->{setup_files}}),
 producer_proof=>\@producer_proof,input_proof=>\@input_proof,outputs=>\@outputs,arithmetic=>\@computed,
 outer=>{timing=>$outer_time,capture=>\%outer},source_ci=>$release->{candidate_ci},committed_controls=>\%committed_control,retained_release_proofs=>\@retained_proofs,
 interpretation=>[
 'Each input has one fixed A1 B1 B2 A2 A3 B3 sequence; all three samples enter each mean.',
 'Both ordinary compiler producers emitted byte-identical output per input at explicit inline0/no-store.',
 'GNU centiseconds and integer RSS are authoritative; means and ratios are descriptive, not significance estimates.',
 'Operating-system caches were not reset; preflight hashing and validation can warm caches, and shared-host load varies.',
 'No allocation-site attribution, 1M-wall inference, generated-code speed claim, or 10-percent adoption threshold is supported or applied.',
 'Archive recovery was already admitted by the pinned setup acceptance; this audit checks exact retained archive bytes and all live compiler members against Git blobs, without decompression.',
 'No compiler, native image, original trial helper, compression, observer, credential or store payload operation ran during this audit.'
 ]};
save_json('acceptance.json',$result);save_json('rows.json',\@rows);save_json('timeline.json',\@intervals);
save_json('raw-inventory.json',\%raw_inventory);save_json('observed-inputs.json',\%observed);save_json('git-read-commands.json',\@git_calls);
my $tsv="phase\twall_s\tcpu_s\tmax_rss_kib\tload_before\tload_after\timage_bytes\timage_sha256\n";
for(@rows){$tsv.=join("\t",$_->{phase},$_->{timing}{wall_s},sprintf('%.2f',$_->{timing}{cpu_cs}/100),$_->{timing}{max_rss_kib},$_->{capture}{'load-before'},$_->{capture}{'load-after'},$_->{image}{bytes},$_->{image}{sha256})."\n";}
save('rows.tsv',$tsv);
print JSON::PP->new->canonical->encode({status=>'accepted',measured_builds=>12,metadata_commands=>14,source_members=>1912,output_paths=>14,raw_evidence_files=>scalar(keys%raw_inventory),acceptance_sha256=>sha256_hex(JSON::PP->new->canonical->pretty->encode($result))})."\n";
