use strict; use warnings;
use JSON::PP (); use Digest::SHA qw(sha256_hex); use File::Find ();
use Fcntl qw(:DEFAULT :mode); use Archive::Tar (); use Time::Local qw(timegm);
my $R='/home/coder/workspace/lkjscript';
my $O="$R/target/compiler-reference-capture-gate-acceptance-1";
my $G='target/compiler-reference-capture-gate-1';
my $Q='target/compiler-reference-capture-query-1';
my $S='target/compiler-reference-capture-seed-1';
my $J=JSON::PP->new->canonical->pretty;
sub need ($$) {die "$_[1]\n" unless $_[0];}
sub same {need(defined($_[0]) && defined($_[1]) && "$_[0]" eq "$_[1]",$_[2]);}
sub save {my($n,$b)=@_;sysopen(my$f,"$O/$n",O_WRONLY|O_CREAT|O_EXCL,0600)or die"new $n: $!\n";binmode$f;print{$f}$b or die$!;close$f or die$!;}
sub path_ok {my($p)=@_;need($p!~m{(?:^|/)\.\.?(/|$)|[\x00-\x1f\x7f]|^/},'unsafe relative path');}
sub meta {
 my($p)=@_;path_ok($p);my@s=lstat("$R/$p");need(@s,"missing $p");
 need(S_ISREG($s[2])||S_ISDIR($s[2]),"special/link refused: $p");
 need($s[3]==1,"hardlink refused: $p") if S_ISREG($s[2]);
 return {path=>$p,kind=>S_ISREG($s[2])?'file':'directory',dev=>$s[0],ino=>$s[1],mode=>sprintf('%04o',$s[2]&07777),nlink=>$s[3],bytes=>$s[7],mtime=>$s[9],ctime=>$s[10]};
}
sub unchanged {my($a,$b)=@_;for my$k(qw(path kind dev ino mode nlink mtime ctime)){same($a->{$k},$b->{$k},"changed $a->{path}: $k");}same($a->{bytes},$b->{bytes},"changed $a->{path}: bytes")if$a->{kind}eq'file';}
sub raw {
 my($p)=@_;my$m=meta($p);same($m->{kind},'file',"not regular $p");need($m->{bytes}<2_000_000,"oversized text input $p");
 sysopen(my$f,"$R/$p",O_RDONLY|O_NOFOLLOW)or die$!;binmode$f;local$/;my$b=<$f>;$b=''unless defined$b;close$f or die$!;
 same(length$b,$m->{bytes},"short read $p");unchanged(meta($p),$m);return$b;
}
sub hashed {
 my($p)=@_;my$m=meta($p);same($m->{kind},'file',"not regular $p");
 sysopen(my$f,"$R/$p",O_RDONLY|O_NOFOLLOW)or die$!;binmode$f;my$h=Digest::SHA->new(256);$h->addfile($f);close$f or die$!;unchanged(meta($p),$m);
 return {%$m,sha256=>$h->hexdigest};
}
sub millis {my($s)=@_;need($s=~/\A(\d+)\.(\d{3})\z/,"bad millisecond field $s");return $1*1000+$2;}
sub utc_ns {my($s)=@_;need($s=~/\A(\d{4})-(\d\d)-(\d\d)T(\d\d):(\d\d):(\d\d)\.(\d{9})Z\n\z/,'bad UTC');return timegm($6,$5,$4,$3,$2-1,$1)*1_000_000_000+$7;}
sub collect {
 my(%files,%dirs);
 for my$scope($G,$Q,'target/gate',$S){
  File::Find::find({no_chdir=>1,follow=>0,wanted=>sub {
   my$p=substr($File::Find::name,length($R)+1);
   need($p eq$scope||index($p,"$scope/")==0,'out of scope');
   my@s=lstat($File::Find::name);need(@s,'missing enumeration entry');
   if(S_ISDIR($s[2])){
    if($p ne$scope && $p=~m{/(?:store|cache|keys)(?:/|$)}){die"unexpected protected subdirectory in scope\n";}
    $dirs{$p}=meta($p) if$scope ne'target/gate'; return;
   }
   return if$scope eq'target/gate' && $p!~/\.log\z/;
   return if$scope eq$S && $p=~m{/(?:candidate\.gz|repeated\.gz|expanded)\z};
   my$m=meta($p);same($m->{kind},'file','selected non-file');need($m->{bytes}<2_000_000,"selected non-log-sized file $p");
   $m->{scope}=$scope;$files{$p}=$m;
   if($scope eq'target/gate'){my$parent=$p;$parent=~s{/[^/]+\z}{};while(index($parent,'target/gate')==0){$dirs{$parent}=meta($parent);last if$parent eq'target/gate';$parent=~s{/[^/]+\z}{};}}
  }},"$R/$scope");
 }
 return(\%files,\%dirs);
}
my($files,$dirs)=collect();my%contents;my$total=0;
for my$p(sort keys%$files){my$b=raw($p);need(substr($b,0,4)ne"\x7fELF","executable refused $p");$contents{$p}=$b;$files->{$p}{sha256}=sha256_hex($b);$total+=length$b;}
same($contents{"$G/status"},"0\n",'gate status');same($contents{"$G/stderr"},'','gate stderr');
same($contents{"$G/cwd"},"$R\n",'gate cwd');
same($contents{"$G/inputs-before.sha256"},$contents{"$G/inputs-after.sha256"},'source before/after');
same($contents{"$G/producer-seed-before.sha256"},$contents{"$G/producer-seed-after.sha256"},'producer/seed before/after');
my@expected_argv=('/usr/bin/time','-o',"$R/$G/time",'-f','wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x','/usr/bin/timeout','--signal=TERM','--kill-after=10s','2400s','/usr/bin/prlimit','--as=4294967296','--','bash','tools/check.sh');
same($contents{"$G/argv"},join('',map{$_."\0"}@expected_argv),'gate argv');
need($contents{"$G/environment"}=~/\ALKJSCRIPT_GATE_JOBS=4\n/,'four worker environment');
my($start,$end)=map{my$x=$contents{"$G/$_"};need($x=~/\A\d{19}\n\z/,'nanosecond framing');chomp$x;0+$x}qw(start.ns end.ns);
need($end>$start,'outer time order');my$wall_ns=$end-$start;
my$utc_start=utc_ns($contents{"$G/start.utc"});my$utc_end=utc_ns($contents{"$G/end.utc"});
need($utc_start<=$start && $start-$utc_start<1_000_000_000 && $utc_end>=$end && $utc_end-$end<1_000_000_000,'UTC/capture adjacency');
my%time;for(split/ /,$contents{"$G/time"}){chomp;need(/\A([a-z_]+)=([\d.]+)\z/,'GNU time framing');need(!exists$time{$1},'duplicate GNU field');$time{$1}=$2;}
same(scalar keys%time,5,'GNU field population');same($time{exit_status},0,'GNU exit');
need(abs($time{wall_s}*1_000_000_000-$wall_ns)<20_000_000,'GNU/outer clock agreement');
for my$n(qw(load-before load-after)){need($contents{"$G/$n"}=~/\A\d+\.\d+ \d+\.\d+ \d+\.\d+\n\z/,'load capture');}
my(@source_proof,%source);
for my$row(split/\n/,$contents{"$G/inputs-before.sha256"}){
 need($row=~/\A([0-9a-f]{64})  ((?:src|library|tests|tools|examples|witness|\.github)\/.+)\z/,'source manifest framing/scope');
 my($sha,$p)=($1,$2);need(!exists$source{$p},'duplicate source path');my$h=hashed($p);same($h->{sha256},$sha,"current source differs $p");$source{$p}=$sha;push@source_proof,{path=>$p,sha256=>$sha,bytes=>$h->{bytes}};
}
same(scalar@source_proof,1939,'source file population');
my@producer_proof;
for my$row(split/\n/,$contents{"$G/producer-seed-before.sha256"}){
 need($row=~/\A([0-9a-f]{64})  (target\/lkjscript|seed\/lkjscript\.gz)\z/,'producer manifest framing');
 my($sha,$p)=($1,$2);my$h=hashed($p);
 if($p eq'seed/lkjscript.gz'){same($h->{sha256},'c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae','current post-gate seed');$h->{gate_before_after_sha256}=$sha;}
 else{same($h->{sha256},$sha,"current producer differs $p");}
 push@producer_proof,$h;
}
same(scalar@producer_proof,2,'producer/seed count');
same($producer_proof[0]{sha256},'b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db','candidate pin');
same($producer_proof[1]{gate_before_after_sha256},'505be947525d53b4db1bff6c85cb77622cb2f5dc7e7d7fe92b772ee3b8ffc3e2','historical gate seed pin');
same($producer_proof[1]{bytes},6551428,'current seed size');
same($contents{"$S/status"},"0\n",'post-gate seed capture status');same($contents{"$S/stderr"},'','seed capture stderr');
same($contents{"$S/git-verification.status"},"0\n",'recorded seed Git roundtrip status');
same($contents{"$S/git-verification.stdout"},'','recorded Git verification stdout');same($contents{"$S/git-verification.stderr"},'','recorded Git verification stderr');
same($contents{"$S/seed-commit"},"e5294c339d9aefbbafc2e951c997f4fefabcc9ad\n",'seed commit pin');
same($contents{"$S/source-commit"},"bd8a9050a3786de9f227c6c4037f5acd43205f71\n",'seed source commit');
same($contents{"$S/sizes"},"seed/lkjscript.gz 6551428\ntarget/lkjscript 32588975\n",'seed size capture');
need($contents{"$S/start.ns"}=~/\A(\d{19})\n\z/ && $1>$end,'seed transition follows gate');
need(index($contents{"$S/inputs.sha256"},$contents{"$G/producer-seed-after.sha256"})==0,'transition input matches gate output');
my$expected_seed_outputs=join('',map{$_->[0]."  ".$_->[1]."\n"}(
 ['c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae',"$R/$S/candidate.gz"],
 ['c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae',"$R/$S/repeated.gz"],
 ['b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db',"$R/$S/expanded"],
 ['c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae','seed/lkjscript.gz'],
 ['b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db','target/lkjscript']));
same($contents{"$S/outputs.sha256"},$expected_seed_outputs,'post-gate seed output pins');
my@git_seed_proof;
for my$pair(['7d0731e99aeaf241c4334fc5ff065a341fb388c8','505be947525d53b4db1bff6c85cb77622cb2f5dc7e7d7fe92b772ee3b8ffc3e2'],
 ['e5294c339d9aefbbafc2e951c997f4fefabcc9ad','c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae']){
 my@argv=('git','-C',$R,'show',"$pair->[0]:seed/lkjscript.gz");
 open my$pipe,'-|',@argv or die$!;binmode$pipe;my$gh=Digest::SHA->new(256);$gh->addfile($pipe);close$pipe or die'Git seed read failed';my$sha=$gh->hexdigest;same($sha,$pair->[1],'Git historical/current seed bytes');push@git_seed_proof,{argv=>\@argv,sha256=>$sha};
}
my$check=raw('tools/check.sh');same(sha256_hex($check),$source{'tools/check.sh'},'gate script belongs to captured source');
my@expected_labels=($check=~/^(?:step|parallel_step\s+(?:\d+|"\$check_suite_jobs"))\s+([a-z][a-z0-9-]*)\s/gm);
my%expected_labels=map{$_=>1}@expected_labels;same(scalar keys%expected_labels,60,'gate script phase population');
my(@phases,%seen);my($duration_sum,$rerun_sum,$reuse_sum)=(0,0,0);my@other;
for my$line(split/\n/,$contents{"$G/stdout"}){
 if($line=~/\Aok ([a-z0-9-]+) \((\d+\.\d{3}) s, load ([\d.]+) -> ([\d.]+); gate\+(\d+\.\d{3})\.\.(\d+\.\d{3}) s\); (.+)\z/){
  my($name,$dur,$lb,$la,$lo,$hi,$result)=($1,$2,$3,$4,$5,$6,$7);
  need($expected_labels{$name}&&!$seen{$name}++,'unexpected/duplicate phase');
  my($dm,$lm,$hm)=map{millis($_)}($dur,$lo,$hi);need($lm>=0&&$hm>=$lm&&abs(($hm-$lm)-$dm)<=1,'phase rounded timing mismatch');$duration_sum+=$dm;
  my$detail=$contents{"target/gate/details-$name.log"};need(defined$detail,"missing detail $name");
  my($rr,$ru,$counted)=(0,0,0);my@stats;
  for my$d(split/\n/,$detail){
   if($d=~/\ALKJSCRIPT_STATS (.+)\z/){my$x=JSON::PP::decode_json($1);push@stats,$x;if(exists$x->{test_reused}){$ru+=$x->{test_reused};$counted=1;}$rr+=$x->{test_rerun}//0;}
   if($d=~/\ALKJSCRIPT_CLOSED_GATE reused=(\d+) rerun=(\d+) /){$ru+=$1;$rr+=$2;$counted=1;}
  }
  if($result=~/\Acases reused=(\d+) rerun=(\d+) /){same($ru,$1,"reuse detail $name");same($rr,$2,"fresh detail $name");need($counted,"missing case evidence $name");$reuse_sum+=$ru;$rerun_sum+=$rr;}
  else{need($result eq'rerun suite (live host observations or fresh admission)','unexpected phase result');same($rr+$ru,0,"unreported cases $name");}
  my$log="target/gate/$name.log";same($contents{$log},"$line\n","parallel summary differs $name")if exists$contents{$log};
  push@phases,{name=>$name,duration_ms=>$dm,gate_start_ms=>$lm,gate_end_ms=>$hm,load_before=>$lb,load_after=>$la,reused=>$ru,rerun=>$rr,detail_sha256=>$files->{"target/gate/details-$name.log"}{sha256},stats_records=>scalar@stats};
 }else{push@other,$line;}
}
same(scalar@phases,60,'sixty phase results');same($rerun_sum,5455,'fresh request sum');same($reuse_sum,0,'reuse sum');same(scalar@other,2,'unexpected gate stdout');
same($other[0],'ok gate selection: skipped 0 case requests on authenticated exact compiler/input closure or stable runner/sealed ELF/checked closure/closed contract equality; reran 5455 misses/open effects; host suites rerun','gate selection');
need($other[1]=~/\Aok gate \((\d+\.\d{3}) s, load ([\d.]+) -> ([\d.]+)\)\z/,'gate final framing');
my($gate_ms,$glb,$gla)=(millis($1),$2,$3);same($gate_ms,611802,'reported gate time');same($glb,(split/ /,$contents{"$G/load-before"})[0],'gate starting load');same($gla,(split/ /,$contents{"$G/load-after"})[0],'gate ending load');
need(abs($gate_ms*1_000_000-$wall_ns)<20_000_000,'outer/gate duration');
for my$p(@phases){need($p->{gate_end_ms}<=$gate_ms,'phase outside gate');}
my@intervals=sort{$a->[0]<=>$b->[0]}map{[$_->{gate_start_ms},$_->{gate_end_ms}]}@phases;
my($union,$lo,$hi)=(0,@{$intervals[0]});for my$i(@intervals[1..$#intervals]){if($i->[0]>$hi){$union+=$hi-$lo;($lo,$hi)=@$i;}elsif($i->[1]>$hi){$hi=$i->[1];}}$union+=$hi-$lo;
my$struct=$contents{'target/gate/details-semantic-structure.log'};
my@struct_counts=($struct=~/^LKJSCRIPT_CLOSED_GATE reused=0 rerun=(\d+) /gm);same(join(',',@struct_counts),'192,134,134,134,134','structural runners');
my$concat=join('',map{$contents{"target/gate/closed/structure-$_.log"}}qw(main source_0 source_1 source_2 source_3));same($concat,$struct,'structural raw child logs');
my$mut=$contents{'target/gate/details-ir-mutation.log'};
need($mut=~/\Aok mutation: (\d+) mutants, (\d+) rejected, (\d+) values, (\d+) traps, (\d+) did not terminate, (\d+) resources, (\d+) findings; (\d+) mutants\/minute \((\d+\.\d{3}) s, (\d+) workers\); (\d+) checked reuses, (\d+) emitted reuses, (\d+) store rejects; inline default\(0\), 0 attempts, 0 accepted, 0 executed, 0 rejected, 0 candidate calls in 0 bodies, 0 local body proofs\n\z/,'mutation summary framing');
my@mc=($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13);
same(join(',',@mc[0..6]),'128,104,22,1,1,0,0','mutation outcomes');same($mc[9],4,'mutation workers');same($mc[0],$mc[1]+$mc[2]+$mc[3]+$mc[4]+$mc[5]+$mc[6],'mutation partition');
opendir my$md,"$R/target/mutation-1701-128"or die$!;my@mutation_files=grep{$_ ne'.'&&$_ ne'..'}readdir$md;closedir$md;same(scalar@mutation_files,0,'unexpected retained individual mutation result');
my$base=$contents{"$G/base-commit"};chomp$base;same($base,'d03ad66bf672b3338ec7dd073560cf599e182303','gate base record');
open my$git,'-|','git','-C',$R,'cat-file','-t',$base or die$!;my$git_type=do{local$/;<$git>};close$git or die'Git base lookup failed';same($git_type,"commit\n",'base Git identity');
same($contents{"$Q/status"},"0\n",'focused capture retained status');same($contents{"$Q/stderr"},'','focused captured stderr');
my@focused=grep{index($_,"$Q/evidence/")==0}sort keys%$files;
my@focused_capture=grep{m{/evidence/capture-(?:cold-before|from-ordinary|unchanged|from-body|cold-after)-(?:generic|held-type|outline|shadow-type|shadowed)\z}}@focused;
same(scalar@focused_capture,25,'focused 25 replies retained');
my@focused_sources=grep{m{/evidence/capture-(?:before|after)\.lkj\z}}@focused;same(scalar@focused_sources,2,'focused two projections retained');
my%scope_counts;for my$p(sort keys%$files){my$s=$files->{$p}{scope};$scope_counts{$s}{files}++;$scope_counts{$s}{bytes}+=$files->{$p}{bytes};}
my$report={
 schema=>'compiler-reference-capture-gate-independent-acceptance-1',status=>'accepted',base_commit=>$base,
 input_manifest_sha256=>sha256_hex($contents{"$G/inputs-before.sha256"}),current_input_files=>scalar@source_proof,inputs_before_after_byte_equal=>JSON::PP::true,current_inputs_match=>JSON::PP::true,
 producer_seed=>\@producer_proof,gate_argv=>\@expected_argv,
 post_gate_seed_transition=>{scope=>$S,seed_commit=>'e5294c339d9aefbbafc2e951c997f4fefabcc9ad',historical_sha256=>$producer_proof[1]{gate_before_after_sha256},current_sha256=>$producer_proof[1]{sha256},git_blob_proof=>\@git_seed_proof,time=>$contents{"$S/time"},load_before=>$contents{"$S/load-before"},load_after=>$contents{"$S/load-after"},omitted_recoverable_images=>['candidate.gz','repeated.gz','expanded'],qualification=>'Transition metadata and original successful Git roundtrip capture retained. This audit hashes compressed old/new Git blobs; it does not rerun gzip or binary comparison.'},
 gate=>{reported_wall_ms=>$gate_ms,reported_load_before=>$glb,reported_load_after=>$gla,outer_wall_ns=>"$wall_ns",gnu=>\%time,load_before=>$contents{"$G/load-before"},load_after=>$contents{"$G/load-after"},successful_phases=>scalar@phases,phase_duration_sum_ms=>$duration_sum,rounded_interval_union_ms=>$union,rounded_interval_gap_ms=>$gate_ms-$union,case_rerun=>$rerun_sum,case_reused=>$reuse_sum,structural_contracts=>728,structural_runner_counts=>\@struct_counts},
 mutation=>{total=>0+$mc[0],rejected=>0+$mc[1],values=>0+$mc[2],traps=>0+$mc[3],nontermination=>0+$mc[4],resources=>0+$mc[5],findings=>0+$mc[6],reported_per_minute=>0+$mc[7],reported_wall_seconds=>$mc[8],workers=>0+$mc[9],checked_reuses=>0+$mc[10],emitted_reuses=>0+$mc[11],store_rejects=>0+$mc[12],retained_individual_result_files=>0,summary_path=>'target/gate/details-ir-mutation.log'},
 phases=>\@phases,raw_scopes=>\%scope_counts,retained_file_count=>scalar(keys%$files),retained_directory_count=>scalar(keys%$dirs),retained_payload_bytes=>$total,
 focused_query=>{scope=>$Q,status=>0,capture_replies_retained=>25,source_projections_retained=>2,time=>$contents{"$Q/time"},load_before=>$contents{"$Q/load-before"},load_after=>$contents{"$Q/load-after"},scope_qualification=>'Full raw retention only; the separate semantic-query lifecycle audit is outside this gate acceptance.'},
 limitations=>[
 'Gate source/producer/seed manifests agree before/after. All 1939 current inputs and the producer match. The later seed transition is explicit and bound to its capture plus exact old/new Git gzip bytes; current seed is not falsely claimed unchanged since the gate. The base commit is the recorded working base.',
 'Per-phase wall durations overlap under the four-worker gate; their sum is not total wall or CPU. Printed phase interval endpoints have millisecond rounding.',
 'The existing aggregate mutation summary is retained. Individual workers used temporary captures and the persistent mutation directory is empty; no per-mutant results are reconstructed.',
 'The 25 focused capture replies and two source projections are retained as a distinct original root; no focused lifecycle validation is inferred from packaging.',
 'No compiler, executable, official reader, test, observer or source transformation was run. No store/cache/key contents or executable copies are included.'
 ]};
save('source-recheck.json',$J->encode(\@source_proof));
save('audit.json',$J->encode($report));
save('phases.tsv',"phase\twall_ms\tstart_ms\tend_ms\tload_before\tload_after\trerun\treused\n".join('',map{join("\t",@$_{qw(name duration_ms gate_start_ms gate_end_ms load_before load_after rerun reused)})."\n"}@phases));
my@entries=sort{$a->{path}cmp$b->{path}}(values%$files,values%$dirs);
save('raw-inventory.json',$J->encode({files=>scalar(keys%$files),directories=>scalar(keys%$dirs),logical_bytes=>$total,entries=>\@entries}));
save('raw-members.nul',join('',map{$_->{path}."\0"}@entries));
save('raw-files.sha256',join('',map{$files->{$_}{sha256}."  $_\n"}sort keys%$files));
open my$df,'-|','df','-Pk',$O or die$!;my$dfb=do{local$/;<$df>};close$df or die'df failed';save('space-before',$dfb);my@df=split/\n/,$dfb;my@cols=split/\s+/,$df[1];need($cols[3]>65536+3*int(($total+1023)/1024),'insufficient archive space');
print "audit passed phases=60 fresh=5455 reused=0 structural=728 mutation=128/0; raw files=",scalar(keys%$files)," bytes=$total\n";$|=1;
my$shell=<<'SH';
set -u
out=$1
repo=$2
LC_ALL=C tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner --format=gnu --no-recursion --verbatim-files-from --null -C "$repo" -cf - -T "$out/raw-members.nul" 2>"$out/tar.stderr" |
 gzip -n -6 > "$out/raw-logs.tar.gz" 2>"$out/gzip.stderr"
printf '%s\n' "${PIPESTATUS[@]}" > "$out/archive-statuses"
SH
my$rc=system('/bin/bash','-c',$shell,'gate-retention',$O,$R);save('archive-shell-status',"$rc\n");same($rc,0,'archive shell');
open my$st,'<',"$O/archive-statuses"or die$!;my$statuses=do{local$/;<$st>};close$st;same($statuses,"0\n0\n",'archive components');
my$tar=Archive::Tar->new;$tar->read("$O/raw-logs.tar.gz",1)or die$tar->error;
my(%archive_seen,@verified);for my$m($tar->get_files){my$p=$m->full_path;$p=~s{/\z}{} if $m->is_dir;need(!$archive_seen{$p}++,'duplicate member');my$e=$m->is_dir?$dirs->{$p}:$files->{$p};need(defined$e,'unexpected member');same($m->mtime,0,'member mtime');same($m->uid,0,'member uid');same($m->gid,0,'member gid');same(sprintf('%04o',$m->mode&07777),$e->{mode},'member mode');if($m->is_dir){unchanged(meta($p),$e);next;}need($m->is_file,'nonregular member');my$b=raw($p);same($m->get_content,$b,"raw archive mismatch $p");same(sha256_hex($b),$e->{sha256},'retained input changed');push@verified,{path=>$p,bytes=>length$b,sha256=>$e->{sha256},byte_equal=>JSON::PP::true,recovery=>"raw-logs.tar.gz::$p"};}
same(scalar keys%archive_seen,scalar@entries,'archive member population');same(scalar@verified,scalar keys%$files,'regular member population');
my($af,$ad)=collect();same(join("\n",sort keys%$af),join("\n",sort keys%$files),'raw file paths changed');same(join("\n",sort keys%$ad),join("\n",sort keys%$dirs),'raw directories changed');for my$p(keys%$files){unchanged($af->{$p},$files->{$p});}for my$p(keys%$dirs){unchanged($ad->{$p},$dirs->{$p});}
open my$z,'<:raw',"$O/raw-logs.tar.gz"or die$!;my$h=Digest::SHA->new(256);$h->addfile($z);close$z;my$sha=$h->hexdigest;my$size=-s"$O/raw-logs.tar.gz";
save('archive-verification.json',$J->encode({status=>'verified',archive=>'raw-logs.tar.gz',sha256=>$sha,bytes=>$size,files=>scalar@verified,directories=>scalar keys%$dirs,logical_bytes=>$total,all_bytes_equal=>JSON::PP::true,originals_unchanged=>JSON::PP::true,members=>\@verified}));
print "archive verified sha256=$sha bytes=$size files=",scalar(@verified)," directories=",scalar(keys%$dirs),"\n";
