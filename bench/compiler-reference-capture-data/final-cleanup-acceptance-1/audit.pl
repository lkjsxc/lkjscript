#!/usr/bin/env perl
use strict; use warnings;
use Fcntl qw(:DEFAULT :mode O_NOFOLLOW);
use Errno qw(ENOENT);
use Digest::SHA qw(sha256_hex);
use JSON::PP;
my $repo='/home/coder/workspace/lkjscript';
my $rel='bench/compiler-reference-capture-data/final-cleanup-1';
my $base="$repo/$rel";
my $out="$repo/bench/compiler-reference-capture-data/final-cleanup-acceptance-1";
my $approval='6114aa74926b4257c8418e6fc4b915348c6100cb';
my $retention='9326dbaab1bf6832b9f35dc156e12d0d62eccd9e';
my $maprel='bench/compiler-reference-capture-data/final-recovery-1/recovery-map.json';
my $canonical='b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db';
my $j=JSON::PP->new->canonical;
my %inputs;
sub need ($$) { my($ok,$why)=@_; die "$why\n" unless $ok; }
sub eqv { my($a,$b,$why)=@_; need(defined($a)&&defined($b)&&!ref($a)&&!ref($b)&&$a eq $b,$why); }
sub same { my($a,$b,$why)=@_; eqv($j->encode($a),$j->encode($b),$why); }
sub read_bytes {
 my($p)=@_; sysopen my $fh,$p,O_RDONLY|O_NOFOLLOW or die "open $p: $!\n";
 binmode $fh; my @s=stat $fh; need(S_ISREG($s[2])&&$s[3]==1&&$s[7]<=67108864,"unsafe/big input $p");
 local $/; my $b=<$fh>; $b='' unless defined $b; my @after=stat $fh;
 close $fh or die "close $p: $!\n";
 same([@s[0,1,2,3,4,5,7,9,10]],[@after[0,1,2,3,4,5,7,9,10]],"input changed $p");
 need(length($b)==$s[7],"short input $p");
 $inputs{$p}={bytes=>length($b),sha256=>sha256_hex($b)}; return $b;
}
sub git_blob {
 my($rev,$path)=@_; need($rev=~/\A[0-9a-f]{40}\z/&&$path=~m{\Abench/[A-Za-z0-9_./-]+\z},"bad Git coordinate");
 open my $fh,'-|','/usr/bin/git','-C',$repo,'show',"$rev:$path" or die "git open: $!\n";
 binmode $fh; local $/; my $b=<$fh>; need(close($fh),"Git blob read failed"); $b='' unless defined $b;
 $inputs{"git:$rev:$path"}={bytes=>length($b),sha256=>sha256_hex($b)}; return $b;
}
sub save {
 my($name,$b)=@_; need($name=~/\A[A-Za-z0-9_.-]+\z/,"unsafe audit output");
 sysopen my $fh,"$out/$name",O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW,0600 or die "create $name: $!\n";
 print {$fh} $b or die "write $name: $!\n"; close $fh or die "close $name: $!\n";
}
sub pinned { my($name,$sha)=@_; my $b=read_bytes("$base/$name"); eqv(sha256_hex($b),$sha,"pin $name"); return $b; }
my %control_sha=(
 'guard.pl'=>'984c7e86d7cda2c9ffed7b20cb51d150b9c680ff0ec3218c9cce85563f4e2977',
 'expected-scope.json'=>'c93116cc86295a5f0e15a84821baa80a6061be5ae85d8d6335392aaa0a0c3e5a',
 'auxiliary-inventory.json'=>'4cf9f1145b9b6018075fff726d8aa1aec7e87d1e0da3ccf4878844b591e84939');
my %controls;
for my $name(sort keys %control_sha) {
 $controls{$name}=pinned($name,$control_sha{$name});
 eqv($controls{$name},git_blob($approval,"$rel/$name"),"committed control $name");
}
pinned('run.sh','b4c2ea08688619c740f12b16958c987348bc473a366cae2d46e3b1b2eb199e10');
my $release_bytes=pinned('release.json','fa5921bacda8db2964ef291c83fe4af1f6c6c873eaa48b553180b6b3da3325e9');
my $scope=decode_json($controls{'expected-scope.json'});
my $aux=decode_json($controls{'auxiliary-inventory.json'});
my $release=decode_json($release_bytes);
eqv($release->{approval_commit},$approval,'approval'); eqv($release->{retention_commit},$retention,'retention');
need($release->{execute}&&$release->{cooperative_quiescence_confirmed}&&!$release->{prior_python_verdict_trusted},'release policy');
my @roots=map{"compiler-reference-capture-$_"} qw(comparison-2 comparison-launch-2 measurement-2 trial-preparation-2 comparison-acceptance-2 comparison-recovery-2 measurement-preparation-1 cleanup-preparation-2 cleanup-execution-2 final-recovery-1);
my @top=map{"compiler-reference-capture-$_"} qw(cleanup-inventory-1.json recovery-guard-proposal-1.md recovery-map-1.json recovery-proof-1.sha256 recovery-proposal-1.py recovery-run-1.json recovery-syntax-1.json recovery-time-1.txt source-commit-message.txt);
same($scope->{directory_roots},\@roots,'scope roots'); same($scope->{top_level_files},\@top,'scope literal files');
same($release->{directory_roots},\@roots,'release roots'); same($release->{top_level_files},\@top,'release literals');
my $map_bytes=git_blob($retention,$maprel);
eqv(sha256_hex($map_bytes),'036832967ed5d4cc55e27c109bea315bac79877ad80636bfea5f81de3cca7bc1','committed recovery map');
need(length($map_bytes)==5318396,'map size'); my $map=decode_json($map_bytes);
need(ref($map->{unresolved})eq'ARRAY'&&!@{$map->{unresolved}},'unresolved recovery entries');
same($map->{scopes},[@roots[0..6]],'map seven roots');
my %entries; my %root_ok=map{$_=>1}@roots; my %top_ok=map{$_=>1}@top;
my %aux_recovery=map{$_->{source}=>$_}@{$scope->{auxiliary_recoveries}};
need(keys(%aux_recovery)==64,'auxiliary recovery population');
for my $e(@{$map->{source_entries}},@{$aux->{entries}}) {
 my $p=$e->{path}; need(defined($p)&&$p=~m{\A[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*\z},'bad inventory path');
 my($head,$tail)=split m{/},$p,2; need($root_ok{$head}||(!defined($tail)&&$top_ok{$head}),'inventory outside scope');
 need(!exists($entries{$p}),'duplicate inventory entry'); need($e->{type}eq'file'||$e->{type}eq'directory','inventory type');
 if($e->{type}eq'file') {
  need($e->{nlink}==1&&$e->{sha256}=~/\A[0-9a-f]{64}\z/,'file metadata'); my $d;
  if(exists $aux_recovery{$p}) { $d=$aux_recovery{$p}; }
  else { need($e->{recovery}{kind}eq'provider','missing provider reference'); $d=$map->{providers}{$e->{recovery}{id}}; }
  need(ref($d)eq'HASH'&&$d->{bytes}==$e->{bytes},'provider extent'); eqv($d->{sha256},$e->{sha256},'provider digest');
 }
 $entries{$p}=$e;
}
my @files=grep{$entries{$_}{type}eq'file'}keys %entries;
my @dirs=grep{$entries{$_}{type}eq'directory'}keys %entries;
my $total=0; $total+=$entries{$_}{bytes} for @files;
need(@files==4532&&@dirs==145&&$total==354054095,'inventory totals');
my %outer;
for my $f(qw(argv.json canonical.before canonical.after controls.before controls.after cwd free-space start.utc start.ns load-before stdout stderr time end.ns end.utc load-after status)) { $outer{$f}=read_bytes("$base/outer-1/$f"); }
eqv($outer{status},"0\n",'outer exit'); eqv($outer{stderr},'','outer diagnostic'); eqv($outer{cwd},"$repo\n",'outer cwd');
eqv($outer{'canonical.before'},$outer{'canonical.after'},'canonical captured drift');
eqv($outer{'canonical.before'},"$canonical  target/lkjscript\n",'canonical capture pin');
eqv($outer{'controls.before'},$outer{'controls.after'},'captured control drift');
my %expected_control=(%control_sha,'release.json'=>'fa5921bacda8db2964ef291c83fe4af1f6c6c873eaa48b553180b6b3da3325e9');
my %seen_control;
for my $line(split /\n/,$outer{'controls.before'}) {
 need($line=~/\A([0-9a-f]{64})  (.+)\z/,'control capture framing'); my($sha,$path)=($1,$2);
 $path=~s{\A\Q$repo/\E}{}; need($path=~m{\A\Q$rel/\E([^/]+)\z},'captured control path'); my $name=$1;
 need(exists($expected_control{$name})&&!$seen_control{$name}++,'captured control population'); eqv($sha,$expected_control{$name},'captured control hash');
}
need(keys(%seen_control)==4,'captured control count');
my @guard_argv=('/usr/bin/perl',"$base/guard.pl",'--release',"$base/release.json",'--execute-after-root-release');
my @wrapper=('/usr/bin/time','-o',"$rel/outer-1/time",'-f','wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x','/usr/bin/timeout','--signal=TERM','--kill-after=10s','900s','/usr/bin/prlimit','--as=4294967296','--',@guard_argv);
same(decode_json($outer{'argv.json'})->{argv},\@wrapper,'outer argv');
my %exe;
for my $f(qw(invocation.json result.json release.json progress.jsonl status)) { $exe{$f}=read_bytes("$base/execution-1/$f"); }
eqv($exe{status},"0\n",'guard exit'); eqv($exe{'release.json'},$release_bytes,'executed release bytes');
my $inv=decode_json($exe{'invocation.json'}); same($inv->{argv},\@guard_argv,'guard actual argv');
eqv($inv->{cwd},$repo,'guard cwd'); eqv($inv->{approval_commit},$approval,'invocation approval'); eqv($inv->{retention_commit},$retention,'invocation retention');
same($inv->{roots},\@roots,'invocation roots'); same($inv->{top_level_files},\@top,'invocation literals');
need(!$inv->{prior_python_verdict_trusted},'invocation Python verdict');
eqv($inv->{guard_sha256},$control_sha{'guard.pl'},'invoked guard'); eqv($inv->{scope_sha256},$control_sha{'expected-scope.json'},'invoked scope');
eqv($inv->{auxiliary_sha256},$control_sha{'auxiliary-inventory.json'},'invoked auxiliary');
my @expected=({event=>'begin'},{event=>'complete_preflight'});
for my $root(@roots) {
 my @rf=sort grep{$entries{$_}{type}eq'file'&&index($_,"$root/")==0}keys %entries;
 for my $p(@rf) { push @expected,{event=>'unlink_intent',relative=>$p},{event=>'unlinked',relative=>$p}; }
 my @rd=sort{scalar(split m{/},$b)<=>scalar(split m{/},$a)||$b cmp $a} grep{$entries{$_}{type}eq'directory'&&($_ eq$root||index($_,"$root/")==0)}keys %entries;
 for my $p(@rd) { push @expected,{event=>'rmdir_intent',relative=>$p},{event=>'removed_directory',relative=>$p}; }
 push @expected,{event=>'root_absent',relative=>$root};
}
for my $p(@top) { push @expected,{event=>'unlink_intent',relative=>$p},{event=>'unlinked',relative=>$p}; }
push @expected,{event=>'complete'};
my @lines=split /\n/,$exe{'progress.jsonl'}; need(@lines==@expected&&@lines==9367,'progress event population');
my($pending,$prior_epoch,$preflight_epoch,$first_mutation_epoch,$last_epoch); my %counts;
for my $i(0..$#lines) {
 my $e=decode_json($lines[$i]); my $want=$expected[$i]; eqv($e->{event},$want->{event},"event order at $i"); $counts{$e->{event}}++;
 need(defined($e->{epoch})&&!ref($e->{epoch})&&$e->{epoch}=~/\A[0-9]+(?:\.[0-9]+)?\z/,'event time');
 need(!defined($prior_epoch)||$e->{epoch}>=$prior_epoch,'event times regress'); $prior_epoch=$e->{epoch}; $last_epoch=$e->{epoch};
 if(exists $want->{relative}) { eqv($e->{path},"$repo/target/$want->{relative}","event path $i"); }
 if($e->{event}eq'begin') { need($i==0&&$e->{expected_regular_files}==4532&&$e->{expected_directories}==145&&$e->{expected_bytes}==354054095&&$e->{directory_roots}==10&&$e->{literal_top_level_files}==9,'begin totals'); }
 elsif($e->{event}eq'complete_preflight') {
  need($i==1&&$e->{regular_files}==4532&&$e->{directories}==145&&$e->{logical_bytes}==354054095&&$e->{independent_full_byte_recovery_checks}&&$e->{all_populations_rechecked}&&$e->{no_python_verdict_used},'complete preflight'); $preflight_epoch=$e->{epoch};
 } elsif($e->{event}eq'unlink_intent'||$e->{event}eq'rmdir_intent') {
  need(defined($preflight_epoch)&&!defined($pending),'intent before preflight/overlap'); $first_mutation_epoch=$e->{epoch} unless defined $first_mutation_epoch;
  $pending={event=>$e->{event},path=>$e->{path}};
  if($e->{event}eq'unlink_intent') { my $entry=$entries{$want->{relative}}; need($e->{bytes}==$entry->{bytes},'intent extent'); eqv($e->{sha256},$entry->{sha256},'intent content identity'); }
 } elsif($e->{event}eq'unlinked'||$e->{event}eq'removed_directory') {
  need(defined($pending),'completion without intent'); eqv($pending->{path},$e->{path},'intent/completion path');
  eqv($pending->{event},$e->{event}eq'unlinked'?'unlink_intent':'rmdir_intent','intent/completion kind'); eqv($e->{absence},'ENOENT','completion absence'); $pending=undef;
 } elsif($e->{event}eq'root_absent') { need(!defined($pending),'root pending mutation'); eqv($e->{absence},'ENOENT','root logged absence'); }
 elsif($e->{event}eq'complete') { need(!defined($pending)&&$e->{regular_files}==4532&&$e->{directories}==145&&$e->{directory_roots_absent}==10&&$e->{literal_files_absent}==9,'complete totals'); }
}
my $result=decode_json($exe{'result.json'});
eqv($result->{status},'complete','result status'); eqv($result->{retention_commit},$retention,'result retention');
need($result->{removed_regular_files}==4532&&$result->{removed_directories}==145&&$result->{logical_bytes}==354054095,'result totals');
same($result->{directory_roots_absent},[map{"$repo/target/$_"}@roots],'result roots');
same($result->{literal_top_level_files_absent},[map{"$repo/target/$_"}@top],'result literals');
need(!$result->{atomic_multi_root_guarantee}&&$result->{cooperative_quiescence_contract}&&!$result->{python_executed},'result qualifications');
my @absences;
for my $p(@roots,@top) { my $full="$repo/target/$p"; my @s=lstat $full; need(!@s&&$!==ENOENT,"not ENOENT: $full"); push @absences,{path=>$full,result=>'ENOENT'}; }
my $canonical_bytes=read_bytes("$repo/target/lkjscript");
need(length($canonical_bytes)==32588975,'current canonical length'); eqv(sha256_hex($canonical_bytes),$canonical,'current canonical identity');
need($outer{time}=~/\Awall_s=([0-9]+\.[0-9]+) user_s=([0-9]+\.[0-9]+) system_s=([0-9]+\.[0-9]+) maxrss_kib=([0-9]+) exit_status=0\n\z/,'GNU time framing');
my($wall,$user,$system,$rss)=("$1","$2","$3","$4"); my $cpu=sprintf('%.2f',$user+$system);
my %load;
for my $k(qw(load-before load-after)) { need($outer{$k}=~/\A([0-9.]+) ([0-9.]+) ([0-9.]+) [0-9]+\/[0-9]+ [0-9]+\n\z/,'load framing'); $load{$k}=[$1,$2,$3]; }
need($outer{'start.ns'}=~/\A([0-9]{19})\n\z/,'start ns'); my $start=0+$1;
need($outer{'end.ns'}=~/\A([0-9]{19})\n\z/,'end ns'); my $end=0+$1;
need($end>$start&&($end-$start)<900000000000,'outer interval');
need($inv->{started_epoch}>=$start/1e9-0.01&&$last_epoch<=$end/1e9+0.01,'event interval outside wrapper');
need($result->{elapsed_seconds}>0&&$result->{elapsed_seconds}<=($end-$start)/1e9,'guard elapsed outside wrapper');
my $answer={
 schema=>'compiler-reference-capture-final-cleanup-independent-acceptance-1',status=>'accepted',
 approval_commit=>$approval,retention_commit=>$retention,release_sha256=>sha256_hex($release_bytes),
 guard_sha256=>$control_sha{'guard.pl'},scope_sha256=>$control_sha{'expected-scope.json'},auxiliary_sha256=>$control_sha{'auxiliary-inventory.json'},
 recovery_map_sha256=>sha256_hex($map_bytes),progress_sha256=>sha256_hex($exe{'progress.jsonl'}),result_sha256=>sha256_hex($exe{'result.json'}),
 regular_files=>4532,directories=>145,logical_bytes=>354054095,progress_records=>scalar(@lines),event_counts=>\%counts,
 complete_preflight_before_first_mutation=>JSON::PP::true,all_intents_have_exact_adjacent_completions=>JSON::PP::true,
 residual_pending_intent=>undef,preflight_epoch=>$preflight_epoch,first_mutation_epoch=>$first_mutation_epoch,
 current_literal_absences=>\@absences,canonical=>{sha256=>$canonical,bytes=>32588975,captured_before_after_equal=>JSON::PP::true,current_bytes_equal=>JSON::PP::true},
 cleanup_timing=>{gnu_wall_seconds=>$wall,user_seconds=>$user,system_seconds=>$system,cpu_seconds=>$cpu,peak_rss_kib=>$rss,
 wrapper_start_ns=>"$start",wrapper_end_ns=>"$end",wrapper_elapsed_ns=>"".($end-$start),start_utc=>$outer{'start.utc'},end_utc=>$outer{'end.utc'},
 load_before=>$load{'load-before'},load_after=>$load{'load-after'}},
 qualifications=>[
 'Post-removal audit of committed scope/recovery identities, exact log sequence, literal absences and current canonical bytes; deleted payloads were not reconstructed or decompressed again.',
 'Full original-byte recovery checks are evidenced by the pinned reviewed guard and its complete-preflight event; this audit does not claim a new comparison with deleted originals.',
 'Cooperative partial-stop cleanup only. Reviewed guard records pending intent on failure and performs no automatic retry; successful execution left no pending intent. No failure was injected.',
 'File mode/timestamp/nlink identity checks precede unlink; directory full metadata checks precede mutation and later checks permit child-removal timestamp/link changes while retaining named/held identity.',
 'No compiler/native workload, cleanup rerun, decompression, Python, process/FD observer or credential read was performed.'
 ]};
save('acceptance.json',JSON::PP->new->canonical->pretty->encode($answer));
save('inputs.json',JSON::PP->new->canonical->pretty->encode(\%inputs));
save('README.md',"# Final finite cleanup: independent acceptance\n\nAccepted once-only root1600/final1602 against controls at $approval and recovery map at $retention.\n\nVerified all 9,367 progress records: begin, complete preflight, 4,532 exact file intent/completion pairs, 145 directory intent/completion pairs, ten root-absence events, and completion. Every intended file matches committed path, byte count and digest. Preflight precedes the first mutation; no pending intent remains. All ten roots and nine literal files are currently ENOENT.\n\nCanonical remains $canonical (32,588,975 bytes), matching both captured identities. Actual outer/guard argv, control pins, zero wrapper/guard/GNU exits and empty stderr agree.\n\nCleanup: wall $wall s, CPU $cpu s (user $user / system $system), peak RSS $rss KiB; load ".join('/',@{$load{'load-before'}})." -> ".join('/',@{$load{'load-after'}}).". This is cleanup timing, not compiler performance.\n\nThis reads retained metadata/logs and current canonical bytes. It does not reconstruct deleted payloads, rerun recovery checks, execute the cleanup guard, decompress an archive, run a compiler, inspect credentials, or use Python/process/FD observers. Full-byte preflight is established through the exact reviewed guard and its successful event, not through a second read of deleted originals. Cleanup is cooperative and can stop partially, not an atomic multi-root transaction. No failure was injected; success left no residual intent.\n\nacceptance.json records exact pins/counts/order/absences/timing; inputs.json records audit input hashes. The audit and its sole execution capture are retained. No original evidence changed. One earlier tool-publication attempt failed in JavaScript parsing before any remote write or audit execution; that failure remains in the tool transcript.\n");
print "accepted: 4532 file pairs, 145 directory pairs, 9367 events, 19 literal absences, canonical b01 unchanged\n";
