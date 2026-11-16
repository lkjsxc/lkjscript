#!/usr/bin/env perl
use strict;
use warnings;
use Fcntl qw(:DEFAULT :mode);
use Errno qw(ENOENT);
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use Math::BigInt;
use File::Find;
my$REPO='/home/coder/workspace/lkjscript';
my$OUT="$REPO/bench/compiler-reference-capture-data/cleanup-acceptance-2";
my$OUTER="$REPO/bench/compiler-reference-capture-data/cleanup-execution-2";
my$INNER="$REPO/target/compiler-reference-capture-cleanup-execution-2";
my$PREP="$REPO/target/compiler-reference-capture-cleanup-preparation-2";
my$COMMIT='d84cac6edcee59e2c2ad631fb6ca15d63340a1c8';
my$GUARD_COMMIT='ce05f3a1536b3759b25a4a787873ced64e1e9be7';
my$GUARD_SHA='e182101cf27ff0f2bed93a015ecfd2cc93aef1d13234eb2809eccbe361042e64';
my$RELEASE_SHA='2a6d1df2af2a004b516d04b58508c218f3ebc234766080f91877eeb4bbe32ce2';
my$MAP_SHA='ff159c384304fb66cd5aa644a06485a5cec2bcc4aab393ccd679a3b7f6faa4bd';
my$CANONICAL_SHA='b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db';
my@KEEP=map{'compiler-reference-capture-'.$_}qw(measurement-preparation-1 measurement-2 trial-preparation-2 comparison-2 comparison-launch-2);
my$J=JSON::PP->new->canonical->pretty->utf8;
my(%pins,@gitreads);
sub need{die"$_[1]\n"unless$_[0];}
sub eqtext{need($_[0]eq$_[1],$_[2]);}
sub jsonsame{eqtext($J->encode($_[0]),$J->encode($_[1]),$_[2]);}
sub read_bytes{
 my($p,$cap)=@_;$cap=67108864 unless defined$cap;sysopen my$f,$p,O_RDONLY|O_NOFOLLOW or die"read $p: $!\n";my@s=stat$f;
 need(@s&&S_ISREG($s[2])&&$s[3]==1&&$s[4]==$>&&$s[7]<=$cap,"unsafe input $p");binmode$f;local$/;my$b=<$f>;close$f or die$!;$b=''unless defined$b;need(length($b)==$s[7],"short read $p");
 my$h=sha256_hex($b);if(exists$pins{$p}){eqtext($pins{$p}{sha256},$h,"changed $p");need($pins{$p}{bytes}==length$b,"size changed $p");}else{$pins{$p}={path=>$p,sha256=>$h,bytes=>length$b};}return$b;
}
sub write_new{my($n,$b)=@_;sysopen my$f,"$OUT/$n",O_WRONLY|O_CREAT|O_EXCL,0600 or die"new output $n: $!\n";binmode$f;print{$f}$b or die$!;close$f or die$!;}
sub savejson{write_new($_[0],$J->encode($_[1]));}
sub gitread{
 my($rev,$p)=@_;open my$f,'-|','/usr/bin/git','-C',$REPO,'show',"$rev:$p"or die$!;binmode$f;local$/;my$b=<$f>;close$f or die"git failure\n";$b=''unless defined$b;
 push@gitreads,{argv=>['/usr/bin/git','-C',$REPO,'show',"$rev:$p"],status=>0,bytes=>length$b,sha256=>sha256_hex($b)};return$b;
}
sub names{my($p)=@_;opendir my$d,$p or die$!;my@n=sort grep{$_ ne'.'&&$_ ne'..'}readdir$d;closedir$d or die$!;return\@n;}
jsonsame(names($INNER),[sort qw(progress.jsonl release.json invocation.json result.json status)],'inner capture population');
jsonsame(names($OUTER),[sort qw(run.sh release.json cwd canonical.before controls.before free-space argv.json start.utc start.ns load-before stdout stderr time end.ns end.utc load-after status canonical.after controls.after)],'outer capture population');
for my$n(@{names($OUTER)}){read_bytes("$OUTER/$n",1048576);}
for my$n(@{names($INNER)}){read_bytes("$INNER/$n",8388608);}
my$guard=read_bytes("$PREP/guard.pl",1048576);eqtext(sha256_hex($guard),$GUARD_SHA,'guard pin');
eqtext($guard,gitread($GUARD_COMMIT,'bench/compiler-reference-capture-data/cleanup-preparation-2/guard.pl'),'committed executed guard');
my$release_bytes=read_bytes("$OUTER/release.json");eqtext(sha256_hex($release_bytes),$RELEASE_SHA,'release pin');
eqtext(read_bytes("$INNER/release.json"),$release_bytes,'inner release');
eqtext(read_bytes("$PREP/release.json"),$release_bytes,'live release');
my$release=decode_json($release_bytes);need($release->{execute}&&$release->{cooperative_quiescence_confirmed}&&$release->{python_violation_acknowledged},'release confirmations');
eqtext($release->{approval_commit},$GUARD_COMMIT,'release guard commit');eqtext($release->{guard_sha256},$GUARD_SHA,'release guard');
eqtext($release->{map_sha256},$MAP_SHA,'release map');eqtext($release->{map_commit},$COMMIT,'release map commit');
need($release->{expected_files}==3050&&$release->{expected_logical_bytes}==160418647,'release scope counts');
my@roots=@{$release->{roots}};need(@roots==29,'root count');my(%roots,%kept);$kept{$_}=1 for@KEEP;
for my$n(@roots){need($n=~/\Acompiler-reference-capture-[a-z0-9-]+\z/&&!$kept{$n}&&!$roots{$n}++,'literal root scope');}
my$map_bytes=gitread($COMMIT,'bench/compiler-reference-capture-data/cleanup-mapping/compiler-reference-capture-recovery-map-1.json');eqtext(sha256_hex($map_bytes),$MAP_SHA,'committed map pin');
my$map=decode_json($map_bytes);my%mapped=map{$_->{name}=>$_}@{$map->{roots}};
my$inv=decode_json(read_bytes("$INNER/invocation.json"));jsonsame($inv->{roots},\@roots,'invocation roots');jsonsame($inv->{excluded},\@KEEP,'invocation exclusions');
eqtext($inv->{guard_sha256},$GUARD_SHA,'invocation guard');eqtext($inv->{map_sha256},$MAP_SHA,'invocation map');eqtext($inv->{map_commit},$COMMIT,'invocation map commit');eqtext($inv->{cwd},$REPO,'invocation cwd');
jsonsame($inv->{argv},['/usr/bin/perl',"$PREP/guard.pl",'--release',"$PREP/release.json",'--execute-after-root-release'],'inner argv');
my$expected_argv=['/usr/bin/time','-o','bench/compiler-reference-capture-data/cleanup-execution-2/time','-f','wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x','/usr/bin/timeout','--signal=TERM','--kill-after=10s','900s','/usr/bin/prlimit','--as=4294967296','--','/usr/bin/perl','target/compiler-reference-capture-cleanup-preparation-2/guard.pl','--release',"$PREP/release.json",'--execute-after-root-release'];
jsonsame(decode_json(read_bytes("$OUTER/argv.json"))->{argv},$expected_argv,'outer argv and bounds');eqtext(read_bytes("$OUTER/cwd"),"$REPO\n",'outer cwd');
eqtext(read_bytes("$INNER/status"),"0\n",'inner status');eqtext(read_bytes("$OUTER/status"),"0\n",'outer status');eqtext(read_bytes("$OUTER/stderr"),'','outer stderr');
eqtext(read_bytes("$OUTER/stdout"),"cleanup complete: 29 roots, 3050 files, 160418647 bytes; enumerated only; all roots ENOENT\n",'exact final stdout');
my$expected_controls="$GUARD_SHA  target/compiler-reference-capture-cleanup-preparation-2/guard.pl\n$RELEASE_SHA  $PREP/release.json\n";
eqtext(read_bytes("$OUTER/controls.before"),$expected_controls,'before controls');eqtext(read_bytes("$OUTER/controls.after"),$expected_controls,'after controls');
my$canonical_line="$CANONICAL_SHA  target/lkjscript\n";
eqtext(read_bytes("$OUTER/canonical.before"),$canonical_line,'canonical before record');eqtext(read_bytes("$OUTER/canonical.after"),$canonical_line,'canonical after record');
my$canonical=read_bytes("$REPO/target/lkjscript");need(length($canonical)==32588975,'canonical size');eqtext(sha256_hex($canonical),$CANONICAL_SHA,'current canonical bytes');
my$times=read_bytes("$OUTER/time");need($times=~/\Awall_s=([0-9.]+) user_s=([0-9.]+) system_s=([0-9.]+) maxrss_kib=([0-9]+) exit_status=0\n\z/,'GNU time framing');
my$timing={wall_s=>$1,user_s=>$2,system_s=>$3,maxrss_kib=>$4,exit_status=>0};
jsonsame($timing,{wall_s=>'29.55',user_s=>'25.82',system_s=>'3.24',maxrss_kib=>'303264',exit_status=>0},'reported execution timing');
my%load;for my$part(qw(before after)){my$v=read_bytes("$OUTER/load-$part");need($v=~/\A([0-9.]+) ([0-9.]+) ([0-9.]+) [0-9]+\/[0-9]+ [0-9]+\n\z/,'load framing');$load{$part}=[$1,$2,$3];}
jsonsame($load{before},['1.11','0.99','1.11'],'before load');jsonsame($load{after},['1.69','1.14','1.16'],'after load');
my$start=read_bytes("$OUTER/start.ns");my$end=read_bytes("$OUTER/end.ns");chomp($start,$end);need($start=~/\A[0-9]{19}\z/&&$end=~/\A[0-9]{19}\z/,'exact ns framing');
my$duration=Math::BigInt->new($end)->bsub(Math::BigInt->new($start));need(!$duration->is_neg,'ns order');
for my$n(qw(start.utc end.utc)){need(read_bytes("$OUTER/$n")=~/\A[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\.[0-9]{9}Z\n\z/,'UTC format');}
my@expected;sub add{my($type,$fields)=@_;push@expected,{event=>$type,%$fields};}
add('begin',{files=>3050,logical_bytes=>160418647,prior_python_acceptance_trusted=>JSON::PP::false,selected_roots=>29});
my(@chosen,%file_paths,%directory_paths,$file_count,$byte_count,$directory_count);($file_count,$byte_count,$directory_count)=(0,0,0);
for my$name(@roots){
 my$r=$mapped{$name};need(defined$r,'missing mapped root');eqtext($r->{path},"$REPO/target/$name",'mapped absolute root');
 my@rows=@{$map->{file_rows}}[$r->{file_row_start}..$r->{file_row_start}+$r->{file_row_count}-1];
 @rows=sort{$a->[0]cmp$b->[0]}@rows;
 add('root_preflight_verified',{root=>$name,files=>scalar(@rows)});
 for my$row(@rows){need($row->[0]!~m{\A/|(?:\A|/)\.\.(?:/|\z)}&&!$file_paths{"$r->{path}/$row->[0]"}++,'duplicate/unsafe expected file');$file_count++;$byte_count+=$row->[4];}
 my@dirs=sort{scalar(split m{/},$b->[0])<=>scalar(split m{/},$a->[0])||$b->[0]cmp$a->[0]}@{$r->{directories}};
 for my$d(@dirs){need(!$directory_paths{"$r->{path}/$d->[0]"}++,'duplicate directory');$directory_count++;}
 need(!$directory_paths{$r->{path}}++,'duplicate root directory');$directory_count++;
 push@chosen,{name=>$name,path=>$r->{path},rows=>\@rows,dirs=>\@dirs};
}
need($file_count==3050&&$byte_count==160418647&&$directory_count==119,'independently recomputed scope');
add('complete_preflight',{roots=>29,files=>3050,logical_bytes=>160418647,independent_perl_recovery_and_byte_checks=>JSON::PP::true});
for my$r(@chosen){
 for my$row(@{$r->{rows}}){my$p="$r->{path}/$row->[0]";add('unlink_intent',{path=>$p,sha256=>$row->[8],bytes=>$row->[4]});add('unlinked',{path=>$p});}
 for my$d(@{$r->{dirs}}){my$p="$r->{path}/$d->[0]";add('rmdir_intent',{path=>$p});add('removed_directory',{path=>$p});}
 add('rmdir_intent',{path=>$r->{path}});add('root_absent',{path=>$r->{path},errno=>'ENOENT'});
}
add('complete',{removed_files=>3050,removed_directories=>119,roots_absent=>29});
my$progress=read_bytes("$INNER/progress.jsonl",8388608);my@lines=split/\n/,$progress;need($progress=~/\n\z/&&@lines==@expected&&@lines==6370,'complete exact progress population');
my(%events,$last_elapsed);$last_elapsed=-1;
for my$i(0..$#expected){
 my$actual=decode_json($lines[$i]);need(ref($actual)eq'HASH'&&exists$actual->{elapsed_seconds},"progress record $i");my$t=delete$actual->{elapsed_seconds};
 need(defined$t&&!ref$t&&$t>=0&&$t>=$last_elapsed,"progress time order $i");$last_elapsed=$t;
 jsonsame($actual,$expected[$i],"progress exact event $i");$events{$actual->{event}}++;
}
need($expected[30]{event}eq'complete_preflight'&&$expected[31]{event}eq'unlink_intent','preflight/mutation boundary');
my$result=decode_json(read_bytes("$INNER/result.json"));need($result->{status}eq'complete'&&$result->{removed_files}==3050&&$result->{removed_directories}==119&&$result->{no_atomic_multi_root_guarantee},'inner completed result');
jsonsame($result->{roots_absent},[map{"$REPO/target/$_"}@roots],'result absent roots');jsonsame($result->{excluded},\@KEEP,'result keep roots');
need($result->{elapsed_seconds}>=$last_elapsed&&$result->{elapsed_seconds}<30,'guard elapsed boundary');
my@absence;for my$name(@roots){my$p="$REPO/target/$name";my@s=lstat$p;need(!@s&&$!==ENOENT,"current absence is not ENOENT $p");push@absence,{path=>$p,absent=>JSON::PP::true,errno=>'ENOENT'};}
my$old_bytes=gitread($COMMIT,'bench/compiler-reference-capture-data/cleanup-mapping/compiler-reference-capture-cleanup-inventory-1.json');
eqtext(sha256_hex($old_bytes),'c6784f5da5078be87ee4c6f20401477e2096eaa7a14adb85b17b370896347da7','prior root inventory');
my$old=decode_json($old_bytes);my%prior=map{$_->{name}=>$_}@{$old->{roots}};my@exclusions;
for my$name(@KEEP){
 my$p="$REPO/target/$name";my@s=lstat$p;need(@s&&S_ISDIR($s[2])&&$s[4]==$>,"excluded root presence/owner $name");
 my$now={uid=>0+$s[4],gid=>0+$s[5],mode=>sprintf('%04o',$s[2]&07777),dev=>0+$s[0],inode=>0+$s[1]};
 if(exists$prior{$name}){jsonsame($now,$prior{$name}{root_metadata},"preserved excluded root identity $name");push@exclusions,{path=>$p,current=>$now,prior_identity_verified=>JSON::PP::true,prior_source=>'committed original prefix inventory; root identity fields only'};}
 else{push@exclusions,{path=>$p,current=>$now,prior_identity_verified=>JSON::PP::false,qualification=>'Historical inode/device/owner identity not recorded in the available comparison inventory; current owned-directory presence and the recorded population/modes/sizes are checked separately.'};}
}
# Six excluded preparation files have full recorded hashes; verify without executing them.
my$dep=$mapped{'compiler-reference-capture-measurement-preparation-1'};my@dep_rows=@{$map->{file_rows}}[$dep->{file_row_start}..$dep->{file_row_start}+$dep->{file_row_count}-1];need(@dep_rows==6,'dependency file count');
for my$row(@dep_rows){my$p="$dep->{path}/$row->[0]";my$b=read_bytes($p);eqtext(sha256_hex($b),$row->[8],'kept preparation content');need(length$b==$row->[4],'kept preparation size');}
# Compare protected comparison-root metadata only; never read their file contents.
my$comparison_bytes=gitread($COMMIT,'bench/compiler-reference-capture-data/comparison-recovery/inventory.json');my$comparison=decode_json($comparison_bytes);
jsonsame($comparison->{scopes},['compiler-reference-capture-comparison-2','compiler-reference-capture-comparison-launch-2'],'comparison inventory scope');
my%cmp_expected;for my$e(@{$comparison->{entries}}){need(!exists$cmp_expected{$e->{path}},'duplicate comparison inventory');$cmp_expected{$e->{path}}=$e;}
my%cmp_seen;my($cmp_files,$cmp_dirs,$cmp_bytes)=(0,0,0);
for my$n(@{$comparison->{scopes}}){
 my$root="$REPO/target/$n";find({no_chdir=>1,follow=>0,wanted=>sub{
 my$p=$File::Find::name;my$rel=substr($p,length("$REPO/target/"));my$e=$cmp_expected{$rel};need(defined$e&&!$cmp_seen{$rel}++,'unexpected comparison path');my@s=lstat$p;need(@s&&$s[4]==$>,'comparison current ownership');
 need(($s[2]&07777)==$e->{mode},'comparison recorded mode');
 if($e->{type}eq'directory'){need(S_ISDIR($s[2]),'comparison directory type');$cmp_dirs++;}
 else{need(($e->{type}eq'file'||$e->{type}eq'regular')&&S_ISREG($s[2])&&$s[3]==1&&$s[7]==$e->{bytes},'comparison file metadata');$cmp_files++;$cmp_bytes+=$s[7];}
 }},$root);
}
need(keys(%cmp_seen)==keys(%cmp_expected)&&$cmp_files==501&&$cmp_dirs==32&&$cmp_bytes==254521215,'complete protected comparison population');
# Reread the captured controls and progress, not the measured workload or archived payloads.
for my$p(sort keys%pins){next if$p eq"$REPO/target/lkjscript";read_bytes($p);}
my$accept={
 schema=>1,status=>'accepted completed Perl cleanup',execution_guard_commit=>$GUARD_COMMIT,guard_sha256=>$GUARD_SHA,release_sha256=>$RELEASE_SHA,map_commit=>$COMMIT,map_sha256=>$MAP_SHA,
 original_python_violation=>{preserved=>JSON::PP::true,actual_interpreter=>'/usr/bin/python3',qualification=>'The earlier mapper and its syntax run violated the no-Python instruction. Their original captures and .py source are retained truthfully; the source is stored as inert .py.txt with original-path mapping. This outcome audit does not retroactively authorize that run.',distinct_perl_execution=>JSON::PP::true},
 execution=>{outer_status=>0,inner_status=>0,gnu_time=>$timing,load_before=>$load{before},load_after=>$load{after},start_ns=>"$start",end_ns=>"$end",wrapper_elapsed_ns=>$duration->bstr(),guard_elapsed_seconds=>$result->{elapsed_seconds}},
 progress=>{records=>scalar(@lines),event_counts=>\%events,expected_unique_files=>$file_count,expected_logical_bytes=>$byte_count,expected_directories_including_roots=>$directory_count,complete_preflight_event_index_zero_based=>30,first_mutation_intent_index_zero_based=>31,full_exact_sequence_matches=>JSON::PP::true,all_29_preflights_before_first_mutation=>JSON::PP::true,progress_sha256=>sha256_hex($progress)},
 current_absence=>\@absence,canonical=>{sha256=>$CANONICAL_SHA,bytes=>length($canonical),before_after_records_equal=>JSON::PP::true,current_bytes_match=>JSON::PP::true},
 excluded_roots=>\@exclusions,excluded_preparation_files_hash_verified=>6,protected_comparison_metadata=>{regular_files=>$cmp_files,directories_including_roots=>$cmp_dirs,logical_file_bytes=>$cmp_bytes,complete_population_modes_sizes_match=>JSON::PP::true,payloads_read=>JSON::PP::false},
 limits=>['This is an independent saved-record, source-pin, ordering and current-state audit; the cleanup guard was not rerun, and archived recovery payloads were not decoded again.','No claim of historical nanosecond filesystem metadata: the frozen map has none. Exact outer capture nanoseconds remain decimal strings.','Three kept roots have prior inode/device/owner/mode identities and match; the two comparison roots lack historical inode identities in the examined inventory, so only current presence plus complete recorded population/modes/sizes are asserted.','Unrecorded contents of other excluded roots and credentials were not inspected; there is no blanket claim of unchanged credential bytes.'],
 no_guard_rerun=>JSON::PP::true,no_workload=>JSON::PP::true,no_compression=>JSON::PP::true,no_key_read=>JSON::PP::true,no_original_mutation=>JSON::PP::true
};
savejson('acceptance.json',$accept);savejson('input-pins.json',[map{$pins{$_}}sort keys%pins]);savejson('git-reads.json',\@gitreads);
print"PASS: 6370 exact progress records; 3050 unique unlink intents/completions; 119 exact directory intents/removals; 29 current ENOENT\n";
print"All 29 preflights and complete_preflight precede the first mutation; canonical b01eabfa unchanged\n";
print"Five excluded roots exist; three recorded identities match; six kept preparation hashes and 501-file/32-directory comparison metadata match\n";
print"No cleanup rerun, workload, compression, key read or original mutation\n";
