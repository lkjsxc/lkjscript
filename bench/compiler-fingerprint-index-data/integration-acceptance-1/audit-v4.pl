#!/usr/bin/env perl
use strict; use warnings;
use Fcntl qw(:DEFAULT :mode O_NOFOLLOW); use File::Find; use Digest::SHA qw(sha256_hex sha1_hex);
use JSON::PP; use Encode qw(encode); use IPC::Open2; use IO::Handle;
my $repo='/home/coder/workspace/lkjscript';
my $data='bench/compiler-fingerprint-index-data';
my $prep="$repo/$data/integration-preparation-1";
my $exe="$repo/$data/integration-execution-1";
my $out="$repo/$data/integration-acceptance-1";
my $commit='88a06df2b212d5856b62aa9bcb49e9092cad1d10';
my $new='src/semantic_tests/incremental_fingerprint.lkj';
my $j=JSON::PP->new->canonical;
my(%cache,%read_pins);
sub need ($$) {my($ok,$why)=@_;die "$why\n" unless $ok;}
sub eqv {my($a,$b,$why)=@_;need(defined($a)&&defined($b)&&!ref($a)&&!ref($b)&&$a eq $b,$why);}
sub same {my($a,$b,$why)=@_;eqv($j->encode($a),$j->encode($b),$why);}
sub raw {
 my($p)=@_;return $cache{$p} if exists $cache{$p};
 sysopen my $f,$p,O_RDONLY|O_NOFOLLOW or die "open $p: $!\n";binmode $f;my @s=stat $f;
 need(@s&&S_ISREG($s[2])&&$s[3]==1&&$s[7]<=134217728,"unsafe input $p");
 local $/;my $b=<$f>;$b='' unless defined $b;my @t=stat $f;close $f or die "close $p\n";
 same([@s[0,1,2,3,4,5,7,9,10]],[@t[0,1,2,3,4,5,7,9,10]],"read drift $p");need(length($b)==$s[7],"length $p");
 $read_pins{$p}={bytes=>length($b),sha256=>sha256_hex($b)};$cache{$p}=$b;return $b;
}
sub run_read {
 my($ok,@args)=@_;open my $f,'-|',@args or die "read command: $!\n";binmode $f;local $/;my $b=<$f>;$b='' unless defined $b;
 close $f;my $status=$?;need(($status&127)==0&&($status>>8)==$ok,'read command exit');return $b;
}
sub blob {my($rev,$p)=@_;return run_read(0,'/usr/bin/git','-c','core.packedGitWindowSize=1m','-c','core.packedGitLimit=64m','-C',$repo,'show',"$rev:$p");}
sub json {return JSON::PP->new->utf8->decode($_[0]);}
sub utf8 {return encode('UTF-8',$_[0]);}
sub manifest {
 my($b)=@_;my %m;for my $line(split /\n/,$b){
  need($line=~/\A([0-9a-f]{64})  ([\x20-\x7e]+)\z/,'manifest framing');my($sha,$p)=($1,$2);
  need($p!~m{\A/|(?:\A|/)\.\.?(?:/|\z)}&&!exists($m{$p}),'manifest path');$m{$p}=$sha;
 }return \%m;
}
sub save {
 my($name,$b)=@_;need($name=~/\A[A-Za-z0-9_.-]+\z/,'output name');
 sysopen my $f,"$out/$name",O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW,0600 or die "create $name: $!\n";
 print {$f} $b or die "write $name\n";close $f or die "close $name\n";
}
my $pm=raw("$prep/manifest.sha256");
eqv(sha256_hex($pm),'3bd8cf979c22f3a64143870e27c395cc52d1b6e6a95e83e6b541dddb80abbca3','preparation manifest pin');
eqv($pm,blob($commit,"$data/integration-preparation-1/manifest.sha256"),'committed preparation manifest');
my $pm_map=manifest($pm);need(keys(%$pm_map)==48,'preparation population');
for my $p(sort keys %$pm_map) {
 my $b=raw("$prep/$p");eqv(sha256_hex($b),$pm_map->{$p},"preparation hash $p");
 eqv($b,blob($commit,"$data/integration-preparation-1/$p"),"preparation Git bytes $p");
}
my $pins=json(raw("$prep/pins.json"));my $corr=json(raw("$prep/correspondence.json"));my $req=json(raw("$prep/request.json"));
need($req->{version}==1&&@{$req->{operations}}==4,'request shape');
eqv(sha256_hex(raw("$prep/request.json")),'47ceb4ed8e8e7fdc3746537d325ae48da9520142f3845b0527baecfe61d3927c','request pin');
my %ex;
opendir my $ed,$exe or die "execution directory\n";my @ef=sort grep{$_ ne'.'&&$_ ne'..'}readdir $ed;closedir $ed;
need(@ef==32,'execution capture population');
for my $f(@ef){$ex{$f}=raw("$exe/$f");}
eqv($ex{'base-commit'},"$commit\n",'operation base');
eqv($ex{'inputs-before.sha256'},raw("$prep/before/inputs.sha256"),'input preimage manifest');
eqv($ex{'language-before.sha256'},raw("$prep/before/tracked-language.sha256"),'language preimage manifest');
my $ib=manifest($ex{'inputs-before.sha256'});my $ia=manifest($ex{'inputs-after.sha256'});
my $lb=manifest($ex{'language-before.sha256'});my $la=manifest($ex{'language-after.sha256'});
need(keys(%$ib)==1743&&keys(%$ia)==1744&&keys(%$lb)==1221&&keys(%$la)==1222,'captured input/language counts');
same([sort keys %$ia],[sort(keys(%$ib),$new)],'input one addition');same([sort keys %$la],[sort(keys(%$lb),$new)],'language paths plus new module');
my %before=(%$ib,%$lb);my %after=(%$ia,%$la);
for my $p(keys %$ib){eqv($ib->{$p},$lb->{$p},"overlapping before manifest $p") if exists $lb->{$p};}
for my $p(keys %$ia){eqv($ia->{$p},$la->{$p},"overlapping after manifest $p") if exists $la->{$p};}
my %changed=map{$_=>1}qw(src/elaborate/incremental.lkj src/semantic_main.lkj src/semantic_tests.lkj);
for my $p(sort keys %before){need(exists($after{$p}),"missing prior input $p");need($before{$p}eq$after{$p}||$changed{$p},"unexpected changed input $p");}
my %physical;
for my $sub(qw(src library tests)){
 find({no_chdir=>1,follow=>0,wanted=>sub{
  my $p=$File::Find::name;my @s=lstat $p;need(@s,'source lstat');return if S_ISDIR($s[2]);
  need(S_ISREG($s[2])&&$s[3]==1,'source special/linked');my $r=substr($p,length($repo)+1);$physical{$r}=1;
 }}, "$repo/$sub");
}
same([sort keys %physical],[sort keys %$ia],'current complete physical input set');
my @tracked=grep{/\.lkj\z/}split /\0/,run_read(0,'/usr/bin/git','-c','core.packedGitWindowSize=1m','-c','core.packedGitLimit=64m','-C',$repo,'ls-files','-z');
my %current_tracked=map{$_=>1}@tracked;delete $current_tracked{$new};
same([sort keys %current_tracked],[sort keys %$lb],'current original tracked language set');
for my $p(sort keys %after){eqv(sha256_hex(raw("$repo/$p")),$after{$p},"current source bytes $p");}
my @base_paths=sort keys %before;
my $tree=run_read(0,'/usr/bin/git','-c','core.packedGitWindowSize=1m','-c','core.packedGitLimit=64m','-C',$repo,'ls-tree','-rz','--full-tree',$commit,'--',@base_paths);
my %tree;
for my $record(split /\0/,$tree){need($record=~/\A(100644|100755) blob ([0-9a-f]{40})\t(.+)\z/s,'baseline tree framing');need(!exists($tree{$3}),'tree duplicate');$tree{$3}={mode=>$1,oid=>$2};}
same([sort keys %tree],\@base_paths,'baseline Git path population');
my $g=json(raw("$prep/before/git-correspondence.json"));my %g=map{$_->{path}=>$_}@{$g->{entries}};
same([sort keys %g],\@base_paths,'preparation Git correspondence population');
my($gr,$gw);my $pid=open2($gr,$gw,'/usr/bin/git','-c','core.packedGitWindowSize=1m','-c','core.packedGitLimit=64m','-C',$repo,'cat-file','--batch');binmode $gr;binmode $gw;$gw->autoflush(1);
my $original_bytes=0;
for my $p(@base_paths){
 eqv($tree{$p}{oid},$g{$p}{git_oid},"baseline oid $p");eqv($tree{$p}{mode},$g{$p}{mode},"baseline mode $p");
 my $oid=$tree{$p}{oid};print {$gw} "$oid\n" or die "Git batch write\n";my $header=<$gr>;
 need(defined($header)&&$header=~/\A([0-9a-f]{40}) blob ([0-9]+)\n\z/,'Git batch header');eqv($1,$oid,'Git returned oid');my $size=0+$2;
 my $b='';while(length($b)<$size){my $n=read($gr,my $part,$size-length($b));need(defined($n)&&$n>0,'short Git blob');$b.=$part;}
 my $lf; need(read($gr,$lf,1)==1&&$lf eq"\n",'Git blob boundary');eqv(sha1_hex("blob $size\0$b"),$oid,'Git object identity');
 eqv(sha256_hex($b),$before{$p},"baseline original bytes $p");need(length($b)==$g{$p}{bytes},"baseline extent $p");$original_bytes+=$size;
}
close $gw or die "close Git writer\n";close $gr or die "close Git reader\n";waitpid($pid,0);need($?==0,'Git batch status');
my %base_names=('src/elaborate/incremental.lkj'=>'incremental.lkj.txt','src/semantic_main.lkj'=>'semantic_main.lkj.txt','src/semantic_tests.lkj'=>'semantic_tests.lkj.txt');
my @splice_proof;
for my $n(0,2,3){
 my $op=$corr->{operations}[$n];my $request=$req->{operations}[$n];
 eqv($request->{op},'replace-body','body operation');eqv($request->{id},$op->{id},'body identity');
 my $p=$op->{file};my $old=raw("$prep/before/$base_names{$p}");eqv(sha256_hex($old),$before{$p},'body whole preimage');
 my($start,$end)=@{$op->{source_span}}{qw(start end)};need($start<$end&&$end<=length($old),'body span');
 eqv(sha256_hex(substr($old,$start,$end-$start)),$op->{before_body_sha256},'body preimage digest');
 my $payload=utf8($request->{source});eqv(sha256_hex($payload),$op->{after_payload_sha256},'request body digest');need(length($payload)==$op->{after_payload_bytes},'payload extent');
 my $expected=substr($old,0,$start).$payload.substr($old,$end);
 my $tail='';
 if($n==2){$tail="\n".utf8($req->{operations}[1]{source})."\n";$expected.=$tail;}
 eqv($expected,raw("$repo/$p"),"exact whole-file splice $p");
 push @splice_proof,{identity=>$op->{id},path=>$p,original_span=>{start=>$start,end=>$end},before_sha256=>sha256_hex($old),payload_sha256=>sha256_hex($payload),after_sha256=>sha256_hex($expected),declaration_tail=>$tail};
}
my $add=$req->{operations}[1];
eqv($add->{op},'add-item','add operation');eqv($add->{id},'semantic_tests','add parent');eqv($add->{source},'mod incremental_fingerprint;','declaration exact');
eqv($add->{projection}{path},$new,'projection path');my $module=raw("$repo/$new");
eqv($module,utf8($add->{projection}{source}),'projection actual bytes');eqv($module,raw("$prep/proposals/incremental-fingerprint-module.txt"),'projection retained proposal');
eqv(sha256_hex($module),'ccaad8644eddaab9f4ae344648922ef7e3df032751a732e8923dcdbe1df7d956','module pin');need(length($module)==15640,'module bytes');
my $frozen=raw("$repo/$data/preparation-1/golden-preparation-1/incremental-fingerprint-module.txt");
eqv(sha256_hex($frozen),'5b5a7c739cb6c45c5fd88bf366b46a04c196c1ea88d58997484623a6dee1c7b5','frozen module pin');
my $fc=$corr->{fixture};my $expected_item=raw("$prep/proposals/expected-item.txt");
eqv(substr($frozen,0,$fc->{old_item_start}).$expected_item.substr($frozen,$fc->{old_item_end}),$module,'only expected item filled');
my $body=raw("$repo/$data/preparation-1/replacement-body.txt");
eqv(utf8($req->{operations}[0]{source})."\n",$body,'fingerprint LF-only normalization');
my $dump=raw("$prep/before/baseline-dump.txt");eqv(sha256_hex($dump),'346340880cf8b9bd247c0caf30f75ed534a702c8b0257dc81fcea2cb5849e4cd','baseline golden dump');
my @golden;my $joined="lkjscript-fingerprint-goldens 1\n";
while($expected_item=~/^\s+([0-9]+)usize => ("(?:\\.|[^"\\])*")\.to_string\(\),$/mg){
 my($n,$literal)=(0+$1,$2);need($n==@golden,'golden arm order');my $record=utf8(json($literal));my $c=$fc->{records}[$n];
 need($record=~/\Acase $n ([a-z0-9-]+)\n/,'golden record header');eqv($1,$c->{id},'golden name');
 need(length($record)==$c->{record_bytes},'golden record extent');eqv(sha256_hex($record),$c->{record_sha256},'golden record digest');
 my @lines=split /\n/,$record;eqv(pop(@lines),'end','golden terminator');
 my $outcome=$lines[1];my $slots=0;
 if($outcome eq'none'){need(@lines==2,'None golden extent');}
 else{need($outcome=~/\Asome [0-9a-f]{64} ([0-9]+)\z/,'Some golden');$slots=0+$1;need(@lines==2+$slots,'golden body count');for my $k(0..$slots-1){need($lines[2+$k]=~/\Abody $k (?:empty|[0-9a-f]{64})\z/,'golden body position');}}
 push @golden,{ordinal=>$n,id=>$c->{id},sha256=>sha256_hex($record),bytes=>length($record),body_slots=>$slots,outcome=>$outcome eq'none'?'None':'Some'};$joined.=$record;
}
need(@golden==17,'golden case population');$joined.="complete 17\n";eqv($joined,$dump,'complete golden dump roundtrip');
my $runner_before=raw("$prep/before/closed-semantics.sh.txt");my $runner_after=$runner_before;
my $runner_old='if [[ "$entry" == main ]]; then count=192; fi';
my $runner_new='if [[ "$entry" == main ]]; then count=196; fi';
my $changed_count=($runner_after=~s/\Q$runner_old\E/$runner_new/);
need($changed_count==1,'host runner one substitution');
eqv($runner_after,raw("$prep/proposals/closed-semantics.sh.txt"),'runner proposal');eqv($runner_after,raw("$repo/tools/closed-semantics.sh"),'runner current');
eqv(sha256_hex($runner_after),'ebbd18fb4edb6fa98b61efc458b38e9d94f3ace1ebf45c362db0b48fa34deb25','runner pin');
eqv($ex{status},"0\n",'operation exit');eqv($ex{stderr},'','operation stderr');eqv($ex{'guard.status'},"0\n",'prelaunch guard exit');eqv($ex{'guard.stderr'},'','prelaunch guard stderr');
eqv($ex{'preparation-check.stderr'},'','preparation validation stderr');eqv($ex{cwd},"$repo\n",'operation cwd');
eqv($ex{'command-plan.json'},raw("$prep/commands.json"),'actual command plan');my $commands=json($ex{'command-plan.json'});
same([split /\0/,$ex{'edit.argv'}],$commands->{argv},'actual edit argv');
eqv($commands->{argv}[8],"$repo/target/compiler-fingerprint-index-integration-1/request.json",'named request path');
eqv(raw("$repo/target/compiler-fingerprint-index-integration-1/request.json"),raw("$prep/request.json"),'named actual request bytes');
my @wrap=('/usr/bin/time','-o',"$exe/time",'-f','wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x','/usr/bin/timeout','--signal=TERM','--kill-after=10s','900s','/usr/bin/prlimit','--as=4294967296','--',@{$commands->{argv}});
same([split /\0/,$ex{'wrapped.argv'}],\@wrap,'wrapped edit argv');eqv($ex{'wrapper-before.sha256'},$ex{'wrapper-after.sha256'},'wrapper stable');
eqv($ex{'producer-seed-before.sha256'},$ex{'producer-seed-after.sha256'},'captured producer/seed stable');
eqv($ex{'producer-seed-before.sha256'},"b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db  target/lkjscript\nc7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae  seed/lkjscript.gz\n",'captured operation producer pin');
my %facts;
while($ex{stdout}=~/^"([^"]+)" level=source-claim file before=([a-z0-9]+) after=([0-9a-f]{64})$/mg){
 my($p,$a,$b)=($1,$2,$3);$p=~s{\A\Q$repo/\E}{};need(!exists($facts{$p}),'duplicate file fact');$facts{$p}={before=>$a,after=>$b};
 eqv($a,exists($before{$p})?$before{$p}:'absent','file fact before');eqv($b,$after{$p},'file fact after');
}
same([sort keys %facts],[sort(keys(%changed),$new)],'exact four reported files');
need($ex{stdout}=~/^transaction sites=4 files=4 identities=5$/m,'operation counts');
my @targets=$ex{stdout}=~/^"([^"]+)" level=source-claim operation-target$/mg;
same(\@targets,[qw(closed_core elaborate::incremental::fingerprint semantic_tests semantic_tests::core semantic_tests::incremental_fingerprint)],'five operation targets');
need($ex{stdout}=~/^program level=checked-ir before=([0-9a-f]{64}) after=(cd70ead245f91bf3d1a89e7b4496b8a0fe110272ac337068c5815d280def4b71) checked-instances=10559 fresh-checked=10559 proof-reused=0 frontend-resumed=0 library-validation=demanded$/m,'checked admission record');
my($meaning_before,$meaning_after)=($1,$2);
need($ex{stdout}=~/^transaction source-before=([0-9a-f]{64}) source-after=([0-9a-f]{64}) mode=apply$/m,'applied source record');
my($source_before,$source_after)=($1,$2);
my(@numstat,$added,$deleted);$added=0;$deleted=0;
for my $p(sort keys %changed){
 my $ns=run_read(1,'/usr/bin/git','diff','--no-index','--no-ext-diff','--no-textconv','--numstat',"$prep/before/$base_names{$p}","$repo/$p");
 need($ns=~/\A([0-9]+)\t([0-9]+)\t[^\n]+\n\z/,'actual diff numstat');
 my($a,$d)=(0+$1,0+$2);push @numstat,{path=>$p,added=>$a,deleted=>$d};$added+=$a;$deleted+=$d;
}
my $reconstructed=join('',map{"$_->{added}\t$_->{deleted}\t$_->{path}\n"}@numstat);
eqv($reconstructed,$ex{'tracked-numstat'},'captured versus independently derived numstat');
my $module_lines=()=$module=~/\n/g;need($module=~/\n\z/&&$module_lines==301,'new module lines');
push @numstat,{path=>$new,added=>$module_lines,deleted=>0};$added+=$module_lines;need($added==331&&$deleted==6&&$added+$deleted==337,'retained language line count');
need($ex{time}=~/\Awall_s=([0-9]+\.[0-9]+) user_s=([0-9]+\.[0-9]+) system_s=([0-9]+\.[0-9]+) maxrss_kib=([0-9]+) exit_status=0\n\z/,'operation GNU time');
my($wall,$user,$sys,$rss)=("$1","$2","$3","$4");my %load;
for my $n(qw(load-before load-after)){need($ex{$n}=~/\A([0-9.]+) ([0-9.]+) ([0-9.]+)\n\z/,'load record');$load{$n}=[$1,$2,$3];}
need($ex{'start.ns'}=~/\A([0-9]{19})\n\z/,'start ns');my $start=0+$1;need($ex{'end.ns'}=~/\A([0-9]{19})\n\z/,'end ns');my $end=0+$1;need($end>$start,'positive interval');
my $answer={
 schema=>'compiler-fingerprint-index-integration-independent-acceptance-1',status=>'accepted',preparation_commit=>$commit,
 preparation_manifest_sha256=>sha256_hex($pm),verified_preparation_files=>49,operation_request_sha256=>sha256_hex(raw("$prep/request.json")),
 input_files_before=>1743,input_files_after=>1744,baseline_tracked_language_files=>1221,new_language_files=>1,
 baseline_git_union_files=>scalar(@base_paths),baseline_git_union_bytes=>$original_bytes,current_verified_union_files=>scalar(keys %after),
 existing_changed_language_files=>3,unchanged_original_union_files=>scalar(@base_paths)-3,all_other_original_bytes_identical=>JSON::PP::true,
 exact_splices=>\@splice_proof,new_module=>{path=>$new,bytes=>length($module),lines=>$module_lines,sha256=>sha256_hex($module)},
 golden_dump_sha256=>sha256_hex($dump),golden_records=>\@golden,host_runner=>{path=>'tools/closed-semantics.sh',sha256=>sha256_hex($runner_after),only_change=>'count=192 -> count=196',language_credit=>JSON::PP::false},
 language_lines=>{added=>$added,deleted=>$deleted,changed=>$added+$deleted,operation_accounted=>$added+$deleted,share=>1,files=>\@numstat},
 operation=>{sites=>4,files=>4,targets=>5,checked_instances=>10559,fresh_checked=>10559,proof_reused=>0,frontend_resumed=>0,meaning_before=>$meaning_before,meaning_after=>$meaning_after,source_before=>$source_before,source_after=>$source_after,
 stdout_sha256=>sha256_hex($ex{stdout}),exit_status=>0,wall_s=>$wall,user_s=>$user,system_s=>$sys,peak_rss_kib=>$rss,load_before=>$load{'load-before'},load_after=>$load{'load-after'},start_ns=>"$start",end_ns=>"$end"},
 qualifications=>[
 'This is offline adoption/correspondence verification, not a new compiler or semantic execution. Checked admission values are the retained successful transaction verdict.',
 'Operation producer b01 and seed c791 are bound through captured before/after records. The live canonical was intentionally not read or constrained because the separately released gate bootstrap may replace it.',
 'The separate shell count edit has no language-operation credit. All 337 added/deleted retained language lines match the four-operation request, including compiler-selected declaration framing.',
 'No compressed archive, key, process/FD observer, Python or native workload was accessed or executed.'
 ]};
save('acceptance.json',JSON::PP->new->canonical->pretty->encode($answer));
save('read-inputs.json',JSON::PP->new->canonical->pretty->encode(\%read_pins));
save('README.md',"# Fingerprint indexing adoption: independent acceptance\n\nAccepted the one successful four-operation transaction against all 49 committed preparation files at $commit. The 1,743 source/library/test inputs became exactly 1,744; all 1,221 original tracked language files and the new module were checked. The union of original inputs and tracked language files was also compared to exact Git blobs. Only three existing language files changed; all others remain byte-identical.\n\nThree body splices, the complete new module and the compiler-selected EOF declaration framing equal the request bytes. The golden module differs from its original only in expected(case); all 17 decoded literals reconstruct the complete retained baseline dump, including every body slot and every None case.\n\nThe independently recomputed retained language delta is $added added + $deleted deleted = ".($added+$deleted)." lines, all accounted for by the structural transaction. The separate tools/closed-semantics.sh count change has no language credit. Structural counts are core 196 plus four 134-contract partitions, total 732.\n\nCaptured operation: four sites/four files/five targets, 10,559 fresh checked instances, zero proof reuse/frontend resume, checked meaning $meaning_after. Exit0, wall $wall s, CPU ".sprintf('%.2f',$user+$sys)." s, RSS $rss KiB, load ".join('/',@{$load{'load-before'}})." -> ".join('/',@{$load{'load-after'}}).". This is operation timing, not an optimization performance result.\n\nThe captured producer/seed were unchanged b01/c791 during the operation. This audit deliberately does not require the live canonical to remain b01 while the separately released full-gate bootstrap runs. No compiler/native/semantic rerun, source write, archive decompression/compression, key read, process/FD observer or Python was used. acceptance.json gives exact file/record identities and read-inputs.json the audit input pins. No original evidence changed. The original audit.pl stopped at Perl compile time before any input read because its one-byte Git-boundary variable was declared inside a compound expression. audit-v2.pl corrected that declaration, then stopped after validating all committed preparation bytes because its after-language count incorrectly excluded the new module. audit-v3.pl requires the captured 1,222-entry language manifest and the exact one added path. V3 passed the current source/manifest checks, then its Git batch reader refused an existing pack mapping under the fixed 1 GiB audit address-space cap. audit-v4.pl adds only per-command Git pack-window/cache limits of 1 MiB/64 MiB; it does not change repository configuration or increase the cap. All prior scripts and captures remain unchanged.\n");
print "accepted: 1743 -> 1744 inputs, 1221 original language files + new module, 17 golden records, 331 added + 6 deleted = 337 operation lines\n";
