use strict;
use warnings;
use Fcntl qw(:DEFAULT :mode);
use Errno qw(ENOENT);
use File::Find;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
my$REPO='/home/coder/workspace/lkjscript';
my$RUN="$REPO/target/compiler-reference-capture-measurement-1";
my$LAUNCH="$REPO/target/compiler-reference-capture-setup-launch-1";
my$OUT="$REPO/target/compiler-reference-capture-setup-acceptance-1";
my$COMMIT='a276bf3748810dbe7b4dc461ecee4ad17e6ec51b';
my$MANIFEST='edb63ca8c50e2659c12e057225f3adb7a774aa3fa9091ee277e872770b4d2d58';
my$J=JSON::PP->new->canonical->pretty->utf8;
my(@commands,%bytes,%inventory);
$|=1;
sub raw{
 my($p)=@_;sysopen my$f,$p,O_RDONLY|O_NOFOLLOW or die"open $p: $!\n";my@s=stat$f;
 die"not bounded regular $p\n"unless@s&&S_ISREG($s[2])&&$s[7]<=134217728;
 binmode$f;local$/;my$b=<$f>;close$f or die$!;$b=''unless defined$b;die"short $p\n"unless length($b)==$s[7];return$b;
}
sub save{my($p,$b)=@_;sysopen my$f,"$OUT/$p",O_WRONLY|O_CREAT|O_EXCL,0600 or die"new $p: $!\n";binmode$f;print{$f}$b or die$!;close$f or die$!;}
sub same{my($a,$b,$n)=@_;die"$n mismatch\n"unless$a eq$b;}
sub git{
 my(@a)=@_;my@v=('/usr/bin/git','-C',$REPO,@a);open my$f,'-|',@v or die$!;binmode$f;local$/;my$b=<$f>;close$f or die"git failure $?\n";$b=''unless defined$b;
 push@commands,{argv=>\@v,exit=>0,bytes=>length$b,sha256=>sha256_hex($b)};return$b;
}
sub inventory{
 my($root)=@_;my(@files,@dirs);my@rs=lstat$root;die"unsafe root\n"unless@rs&&S_ISDIR($rs[2])&&$rs[4]==$>;
 find({no_chdir=>1,wanted=>sub{my$p=$File::Find::name;return if$p eq$root;my@s=lstat$p;die"unsafe owned entry $p\n"unless@s&&$s[4]==$>;
  my$r=substr($p,length($root)+1);die"unsafe path\n"unless$r=~/\A[A-Za-z0-9_.\/-]+\z/&&$r!~m{(?:\A|/)\.\.(?:/|\z)};
  if(S_ISDIR($s[2])){push@dirs,$r;return;}die"special/hardlinked entry $p\n"unless S_ISREG($s[2])&&$s[3]==1;
  my$b=raw($p);$bytes{$p}=$b;$inventory{$p}={path=>$p,relative=>$r,scope=>$root,bytes=>length$b,sha256=>sha256_hex($b),mode=>sprintf('%04o',$s[2]&07777)};
  push@files,$r;
 }},$root);
 return{files=>[sort@files],directories=>[sort@dirs]};
}
my$run=inventory($RUN);my$launch=inventory($LAUNCH);
my@controls=qw(README.md controls.json controls.manifest.sha256 setup.pl setup.sh);
my@record_top=qw(completion end.ns end.utc environment last-phase launch.argv-lines launch.cwd load-after load-before metadata-subcommands.jsonl phases.tsv released-controls.sha256 start.ns start.utc status);
my@record_recover=qw(argv.json cwd cwd.actual end.ns end.utc free-bytes load-after load-before start.ns start.utc stats.raw status stderr stdin stdout time validation.argv-lines validation.status validation.stderr validation.stdout wrapper-argv.json);
my@outer=qw(argv controls-after.sha256 controls-before.sha256 cwd end.ns end.utc load-after load-before release-commit start.ns start.utc status stderr stdout time);
my@artifacts=qw(baseline-A.gz compiler-source.tar scale-generator-source.lkj shape);
my@expected_run=('producer-A',(map{"control/$_"}@controls),(map{"records/$_"}@record_top),(map{"records/recover-inputs/$_"}@record_recover),(map{"artifacts/$_"}@artifacts));
same(join("\0",@{$run->{files}}),join("\0",sort@expected_run),'exact failure file population');
same(join("\0",@{$run->{directories}}),join("\0",sort qw(artifacts control records records/recover-inputs)),'exact failure directory population');
same(join("\0",@{$launch->{files}}),join("\0",sort@outer),'exact outer population');
die"outer unexpected directory\n"if@{$launch->{directories}};
for my$n(@outer){save("raw/launch/$n",$bytes{"$LAUNCH/$n"});}
for my$n(@controls){save("raw/control/$n",$bytes{"$RUN/control/$n"});}
for my$n(@record_top){save("raw/records/$n",$bytes{"$RUN/records/$n"});}
for my$n(@record_recover){save("raw/records/recover-inputs/$n",$bytes{"$RUN/records/recover-inputs/$n"});}
my$control_manifest=$bytes{"$RUN/control/controls.manifest.sha256"};
same(sha256_hex($control_manifest),$MANIFEST,'released manifest');
my%control_pin;for my$l(split/\n/,$control_manifest){die"control manifest framing\n"unless$l=~/\A([0-9a-f]{64})  ([A-Za-z0-9_.-]+)\z/;die"duplicate control\n"if exists$control_pin{$2};$control_pin{$2}=$1;}
same(join("\0",sort keys%control_pin),join("\0",sort qw(setup.sh setup.pl controls.json README.md)),'exact four controls');
for my$n(@controls){
 same($bytes{"$RUN/control/$n"},git('show',"$COMMIT:bench/compiler-reference-capture-data/setup-preparation/$n"),"committed control $n");
 same(sha256_hex($bytes{"$RUN/control/$n"}),$control_pin{$n},"control pin $n")if$n ne'controls.manifest.sha256';
}
my$c=decode_json($bytes{"$RUN/control/controls.json"});
same($c->{run_root},$RUN,'run root');die"comparison authorized\n"if$c->{comparison_authorized};
same($bytes{"$LAUNCH/release-commit"},"$COMMIT\n",'release commit');
same($bytes{"$LAUNCH/controls-before.sha256"},$bytes{"$LAUNCH/controls-after.sha256"},'outer control before/after');
same($bytes{"$LAUNCH/controls-before.sha256"},"$MANIFEST  target/compiler-reference-capture-setup-preparation-1/controls.manifest.sha256\n",'outer manifest pin');
same($bytes{"$RUN/records/released-controls.sha256"},"$MANIFEST\n",'run manifest pin');
same($bytes{"$LAUNCH/argv"},"bash target/compiler-reference-capture-setup-preparation-1/setup.sh --execute-after-root-release $MANIFEST\n",'outer launch');
same($bytes{"$LAUNCH/cwd"},"$REPO\n",'outer cwd');
same($bytes{"$RUN/records/launch.cwd"},"$REPO\n",'launch cwd');
my$phase="$RUN/records/recover-inputs";
same($J->encode(decode_json($bytes{"$phase/argv.json"})),$J->encode(['/usr/bin/perl',"$RUN/control/setup.pl",'recover',$RUN,"$RUN/control/controls.json"]),'phase argv');
my@wrapper=('/usr/bin/time','-f','wall_s=%e\nuser_s=%U\nsystem_s=%S\nmax_rss_kib=%M\nexit=%x','-o',"$phase/time",'/usr/bin/timeout','--kill-after=10s','900s','/usr/bin/prlimit','--as=4294967296','--','/usr/bin/perl',"$RUN/control/setup.pl",'recover',$RUN,"$RUN/control/controls.json");
same($J->encode(decode_json($bytes{"$phase/wrapper-argv.json"})),$J->encode(\@wrapper),'phase wrapper');
for my$p("$LAUNCH/status","$RUN/records/status","$phase/status"){same($bytes{$p},"255\n","original failure status $p");}
same($bytes{"$phase/validation.status"},"0\n",'capture validation success');
same($bytes{"$phase/stderr"},"unexpected/duplicate archive file\n",'first failure diagnostic');
same($bytes{"$phase/stdout"},'','recover stdout');same($bytes{"$phase/stats.raw"},'','no compiler stats');
same($bytes{"$RUN/records/last-phase"},"recover-inputs\n",'last phase');
same($bytes{"$RUN/records/completion"},"setup-stopped; preserve all partial files; no retry or cleanup\n",'stopped completion');
my$phases="phase\toutcome\tcommand_status\tcapture_validation_status\nrecover-inputs\tfailed\t255\t0\n";
$phases.="$_\tnot-run\tunavailable\tunavailable\n"for qw(prepare-factory generator-build generate-100k freeze-artifacts);
same($bytes{"$RUN/records/phases.tsv"},$phases,'all five phase outcomes');
same($bytes{"$phase/cwd"},"$REPO\n",'declared phase cwd');same($bytes{"$phase/cwd.actual"},"$REPO\n",'actual phase cwd');same($bytes{"$phase/stdin"},"/dev/null\n",'phase stdin');
die"free-space contract\n"unless$bytes{"$phase/free-bytes"}=~/\A[0-9]+\n\z/&&$bytes{"$phase/free-bytes"}>=3221225472;
sub time_fields{
 my($b,$outer)=@_;my%t;while($b=~/(?:\A|\s)(wall_s|user_s|system_s|max_rss_kib|maxrss_kib|exit_status|exit)=([0-9]+(?:\.[0-9]+)?)(?=\s|\z)/g){die"duplicate time key\n"if exists$t{$1};$t{$1}=$2;}
 my@keys=$outer?qw(wall_s user_s system_s maxrss_kib exit_status):qw(wall_s user_s system_s max_rss_kib exit);
 same(join("\0",sort keys%t),join("\0",sort@keys),'complete timing fields');die"time exit\n"unless$t{$outer?'exit_status':'exit'}==255;return\%t;
}
my$inner_time=time_fields($bytes{"$phase/time"},0);my$outer_time=time_fields($bytes{"$LAUNCH/time"},1);
my$validation=decode_json($bytes{"$phase/validation.stdout"});
die"validation record result\n"unless$validation->{status}==255&&$validation->{phase}eq'recover-inputs'&&$validation->{complete_metrics};
for my$k(keys%$inner_time){same("$validation->{timing}{$k}","$inner_time->{$k}","validated time $k");}
same($bytes{"$phase/validation.stderr"},'','capture validation stderr');
for my$r($LAUNCH,"$RUN/records",$phase){
 for my$k(qw(load-before load-after)){die"load capture\n"unless$bytes{"$r/$k"}=~/\A[0-9]+(?:\.[0-9]+)? [0-9]+(?:\.[0-9]+)? [0-9]+(?:\.[0-9]+)?\n\z/;}
 for my$k(qw(start.ns end.ns)){die"timestamp capture\n"unless$bytes{"$r/$k"}=~/\A[0-9]{19}\n\z/;}
 die"backwards timestamp\n"unless$bytes{"$r/end.ns"}>=$bytes{"$r/start.ns"};
}
my@subcommands=map{decode_json($_)}split/\n/,$bytes{"$RUN/records/metadata-subcommands.jsonl"};
my@expected_sub=(
 ['git','-C',$REPO,'show',"$c->{baseline}{recovery}{commit}:$c->{baseline}{recovery}{path}"],
 ['git','-C',$REPO,'show',"$c->{generator_source}{revision}:$c->{generator_source}{path}"],
 ['git','-C',$REPO,'show',"$c->{shape}{revision}:$c->{shape}{path}"],
 ['git','-C',$REPO,'ls-tree','-r','-z','--full-tree',$c->{compiler_input}{revision},'--','src','library','tests']);
die"metadata subcommand population\n"unless@subcommands==4;
for my$i(0..3){same($subcommands[$i]{mode},'recover','metadata mode');same($J->encode($subcommands[$i]{argv}),$J->encode($expected_sub[$i]),"metadata command $i");}
my@absent;
for my$r(qw(compiler-input factory generator generated-100k setup-summary.json artifacts.sha256 records/prepare-factory records/generator-build records/generate-100k records/freeze-artifacts)){
 my$p="$RUN/$r";my@s=lstat$p;die"unexpected produced path $p\n"if@s;die"absence is not ENOENT $p\n"unless$!==ENOENT;push@absent,$r;
}
my@recovery;
sub artifact_pin{my($path,$pin)=@_;same(sha256_hex($bytes{$path}),$pin->{sha256},"artifact digest $path");die"artifact size $path\n"if defined$pin->{bytes}&&length($bytes{$path})!=$pin->{bytes};}
artifact_pin("$RUN/producer-A",$c->{baseline});
artifact_pin("$RUN/artifacts/baseline-A.gz",{sha256=>$c->{baseline}{recovery}{archive_sha256},bytes=>$c->{baseline}{recovery}{archive_bytes}});
artifact_pin("$RUN/artifacts/compiler-source.tar",$c->{compiler_input});
artifact_pin("$RUN/artifacts/scale-generator-source.lkj",$c->{generator_source});
artifact_pin("$RUN/artifacts/shape",$c->{shape});
same($bytes{"$RUN/artifacts/baseline-A.gz"},git('show',"$c->{baseline}{recovery}{commit}:$c->{baseline}{recovery}{path}"),'exact committed seed gzip');
for my$pair(['scale-generator-source.lkj','generator_source'],['shape','shape']){
 my($name,$key)=@$pair;my$p=$c->{$key};same($bytes{"$RUN/artifacts/$name"},git('show',"$p->{revision}:$p->{path}"),"exact committed $name");
 push@recovery,{path=>"artifacts/$name",recovery=>"$p->{revision}:$p->{path}",verification=>'byte-equal to exact Git blob',sha256=>sha256_hex($bytes{"$RUN/artifacts/$name"}),bytes=>length($bytes{"$RUN/artifacts/$name"})};
}
push@recovery,{path=>'artifacts/baseline-A.gz',recovery=>"$c->{baseline}{recovery}{commit}:$c->{baseline}{recovery}{path}",verification=>'byte-equal to exact Git blob',sha256=>$c->{baseline}{recovery}{archive_sha256},bytes=>$c->{baseline}{recovery}{archive_bytes}};
push@recovery,{path=>'producer-A',recovery=>"$c->{baseline}{recovery}{commit}:$c->{baseline}{recovery}{path} (decompressed)",verification=>'actual complete bytes hash/size match frozen decompressed baseline identity; setup performed recovery; this audit does not repeat decompression',sha256=>$c->{baseline}{sha256},bytes=>$c->{baseline}{bytes}};
my$proof_bytes=git('show',"$COMMIT:bench/compiler-reference-capture-data/preliminary/verification.json");
my$proof=decode_json($proof_bytes);die"prior recovery proof incomplete\n"unless$proof->{all_members_byte_equal}&&$proof->{archive_regular_members}==1884;
my@matches;sub walk{my($v)=@_;if(ref$v eq'HASH'){push@matches,$v if($v->{path}//'')eq'target/compiler-reference-capture-preparation-1/source.tar';walk($_)for values%$v;}elsif(ref$v eq'ARRAY'){walk($_)for@$v;}}
walk($proof);die"source recovery mapping\n"unless@matches==1&&$matches[0]{byte_equal}&&$matches[0]{bytes}==$c->{compiler_input}{bytes}&&$matches[0]{sha256}eq$c->{compiler_input}{sha256};
same($matches[0]{recovery},'preliminary-roots.tar.gz::target/compiler-reference-capture-preparation-1/source.tar','source archive recovery member');
my$archive=git('show',"$COMMIT:bench/compiler-reference-capture-data/preliminary/preliminary-roots.tar.gz");
same(sha256_hex($archive),$proof->{archive_sha256},'committed recovery archive digest');die"committed archive size\n"unless length($archive)==$proof->{archive_bytes};
push@recovery,{path=>'artifacts/compiler-source.tar',recovery_commit=>$COMMIT,recovery_archive=>'bench/compiler-reference-capture-data/preliminary/preliminary-roots.tar.gz',recovery_archive_sha256=>$proof->{archive_sha256},recovery_archive_bytes=>$proof->{archive_bytes},recovery_member=>'target/compiler-reference-capture-preparation-1/source.tar',sha256=>$c->{compiler_input}{sha256},bytes=>$c->{compiler_input}{bytes},verification=>'actual hash/size matches existing committed full-byte member proof; exact committed archive digest checked; no duplicate extraction/compression in this audit'};
save('source-archive-recovery-record.json',$J->encode({proof_commit=>$COMMIT,proof_path=>'bench/compiler-reference-capture-data/preliminary/verification.json',proof_sha256=>sha256_hex($proof_bytes),member=>$matches[0],archive_sha256=>$proof->{archive_sha256},archive_bytes=>$proof->{archive_bytes}}));
for my$p(sort keys%inventory){my$b=raw($p);same(sha256_hex($b),$inventory{$p}{sha256},"original unchanged $p");die"original size changed\n"unless length($b)==$inventory{$p}{bytes};}
my$total=0;$total+=$_->{bytes}for values%inventory;
my$result={
 schema=>1,status=>'accepted observed preparation failure; setup did not complete',
 release_commit=>$COMMIT,controls_manifest_sha256=>$MANIFEST,
 original_command_status=>255,capture_validation_status=>0,first_failure=>'unexpected/duplicate archive file',failed_phase=>'recover-inputs',
 later_phases=>[map{{phase=>$_,outcome=>'not-run',command_status=>undef}}qw(prepare-factory generator-build generate-100k freeze-artifacts)],
 outer_time=>$outer_time,recovery_time=>$inner_time,outer_load_before=>$bytes{"$LAUNCH/load-before"},outer_load_after=>$bytes{"$LAUNCH/load-after"},
 recovery_load_before=>$bytes{"$phase/load-before"},recovery_load_after=>$bytes{"$phase/load-after"},
 input_files=>scalar(keys%inventory),input_bytes=>$total,run_files=>scalar@{$run->{files}},launch_files=>scalar@{$launch->{files}},
 copied_raw_control_records=>56,artifacts=>\@recovery,absent_outputs=>\@absent,
 no_source_extraction=>JSON::PP::true,no_generator_construction_or_execution=>JSON::PP::true,no_ab_executed=>JSON::PP::true,no_performance_data=>JSON::PP::true,
 proof_boundary=>'Fixed controls plus saved phase/command records and exact existing/absent artifact population establish the observed failure boundary; this is not syscall observation or a diagnosis of archive validity.',
 source_recovery_qualification=>'Uses the committed previously accepted complete byte-equality member proof and exact archive digest; no new archive reconstruction or duplicate gzip is performed.',
 root_inspection_qualification=>'compiler-reference-capture-setup-refusal-v1509 separately exited 1 while reading a guessed missing argv filename; actual argv.json is present and verified. This read-only inspection did not cause or retry the original status-255 setup failure.',
 no_original_mutation=>JSON::PP::true,no_compiler_native_or_setup_rerun=>JSON::PP::true,no_key_or_store_read=>JSON::PP::true,no_decompression_or_compression=>JSON::PP::true
};
save('acceptance.json',$J->encode($result));save('original-inventory.json',$J->encode([map{$inventory{$_}}sort keys%inventory]));save('commands.json',$J->encode(\@commands));
print"PASS observed failure preserved: setup/recover=255 capture_validation=0 later_phases=4 not-run\n";
print"original_files=".scalar(keys%inventory)." original_bytes=$total raw_copies=56 artifacts=5\n";
print"No compiler/generator/A-B execution or extracted source tree; actual A and copied artifacts match fixed recovery pins\n";
