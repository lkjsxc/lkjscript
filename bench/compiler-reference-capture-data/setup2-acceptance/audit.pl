use strict;
use warnings;
use Fcntl qw(:DEFAULT :mode);
use File::Find;
use Digest::SHA qw(sha1_hex sha256_hex);
use JSON::PP;
use IO::Uncompress::Gunzip;
my$REPO='/home/coder/workspace/lkjscript';
my$RUN="$REPO/target/compiler-reference-capture-measurement-2";
my$LAUNCH="$REPO/target/compiler-reference-capture-setup-launch-2";
my$OUT="$REPO/target/compiler-reference-capture-setup-acceptance-2";
my$RELEASE='5e1b82f3cf23654e04d0904e948c202c96723c0d';
my$PIN='fe8321b36a88864b33dd3c8bda9a71ab954f9dae14926d3ba9e45fbb564af847';
my$J=JSON::PP->new->canonical->pretty->utf8;
my(%pins,%observed,@commands);
$|=1;
sub read_bytes{
 my($p)=@_;sysopen my$f,$p,O_RDONLY|O_NOFOLLOW or die"open $p: $!\n";my@s=stat$f;
 die"not bounded regular $p\n"unless@s&&S_ISREG($s[2])&&$s[7]<=134217728&&$s[3]==1;
 binmode$f;local$/;my$b=<$f>;close$f or die$!;$b=''unless defined$b;die"short read $p\n"unless length($b)==$s[7];
 my$h=sha256_hex($b);if(exists$pins{$p}){die"input changed $p\n"unless$pins{$p}{sha256}eq$h&&$pins{$p}{bytes}==length($b);}
 else{$pins{$p}={path=>$p,sha256=>$h,bytes=>length($b)};}
 return$b;
}
sub save{my($p,$b)=@_;sysopen my$f,"$OUT/$p",O_WRONLY|O_CREAT|O_EXCL,0600 or die"new $p: $!\n";binmode$f;print{$f}$b or die$!;close$f or die$!;}
sub json_save{save($_[0],$J->encode($_[1]));}
sub same{my($x,$y,$label)=@_;die"$label mismatch\n"unless$x eq$y;}
sub git{
 my(@args)=@_;my@argv=('/usr/bin/git','-C',$REPO,@args);open my$f,'-|',@argv or die$!;binmode$f;local$/;my$b=<$f>;close$f or die"Git status $?\n";$b=''unless defined$b;
 push@commands,{argv=>\@argv,status=>0,bytes=>length$b,sha256=>sha256_hex($b)};return$b;
}
sub direct_names{
 my($p)=@_;my@s=lstat$p;die"not owned directory $p\n"unless@s&&S_ISDIR($s[2])&&$s[4]==$>;
 opendir my$d,$p or die$!;my@n=sort grep{$_ ne'.'&&$_ ne'..'}readdir$d;closedir$d;return\@n;
}
sub copy_metadata{
 my($from,$to)=@_;find({no_chdir=>1,wanted=>sub{
 my$p=$File::Find::name;return if$p eq$from;my@s=lstat$p;die"metadata stat\n"unless@s;
 return if S_ISDIR($s[2]);die"metadata special/large\n"unless S_ISREG($s[2])&&$s[7]<=2097152;
 my$rel=substr($p,length($from)+1);my$b=read_bytes($p);save("$to/$rel",$b);
 }},$from);
}
sub assert_pin{my($b,$p,$label)=@_;same(sha256_hex($b),$p->{sha256},"$label sha");die"$label size\n"if defined$p->{bytes}&&length($b)!=$p->{bytes};}
sub safe_path{
 my($p,$directory)=@_;$p=~s{\A\./}{};$p=~s{/\z}{} if $directory;
 return''if$directory&&($p eq''||$p eq'.');
 die"unsafe archive path\n"if$p eq''||$p=~m{\A/|\\|[\x00-\x1f\x7f]};
 die"unsafe path component\n"if grep{$_ eq''||$_ eq'.'||$_ eq'..'}split m{/},$p,-1;return$p;
}
sub octal{my($v)=@_;$v=~s/[\0 ]+\z//;$v=~s/\A +//;die"tar octal\n"unless$v=~/\A[0-7]+\z/;return oct$v;}
sub header_text{my($v)=@_;die"tar text padding\n"unless$v=~/\A([^\0]*)\0*\z/s;return$1;}
sub parse_tar{
 my($b,$source_revision)=@_;die"tar size alignment\n"unless length($b)%512==0;
 my(%files,%dirs,@pax);my($offset,$pending,$global,$root_seen,$ended)=(0,undef,0,0,0);
 while($offset+512<=length$b){
  my$h=substr($b,$offset,512);
  if($h eq"\0"x512){die"tar termination\n"unless length($b)-$offset>=1024&&substr($b,$offset)eq"\0"x(length($b)-$offset)&&!defined$pending;$ended=1;last;}
  die"tar magic\n"unless substr($h,257,8)eq"ustar\00000";my$check=$h;substr($check,148,8)=' 'x8;
  die"tar checksum\n"unless unpack('%32C*',$check)==octal(substr($h,148,8));
  my$name=header_text(substr($h,0,100));my$prefix=header_text(substr($h,345,155));my$raw=$prefix eq''?$name:"$prefix/$name";
  my$size=octal(substr($h,124,12));my$mode=octal(substr($h,100,8));my$type=substr($h,156,1);$type='0'if$type eq"\0";
  die"tar link or set-ID mode\n"unless header_text(substr($h,157,100))eq''&&($mode&07000)==0;
  my$end=$offset+512+512*int(($size+511)/512);die"tar extent\n"if$end>length$b;
  my$content=substr($b,$offset+512,$size);
  die"tar payload padding\n"unless substr($b,$offset+512+$size,$end-$offset-512-$size)eq"\0"x($end-$offset-512-$size);
  die"unconsumed PAX\n"if defined$pending&&$type ne'0';
  if($type eq'g'){
   die"unexpected global PAX\n"unless defined$source_revision&&!$global++&&$offset==0&&$raw eq'pax_global_header'&&$content eq"52 comment=$source_revision\n";
  }elsif($type eq'x'){
   die"unexpected per-file PAX\n"unless defined$source_revision&&$content=~/\A([1-9][0-9]*) path=([^\n]*)\n\z/s;
   my($length,$path)=($1,$2);die"PAX length\n"unless$length==length$content;$path=safe_path($path,0);
   $pending={path=>$path,record_offset=>$offset,record_bytes=>$size,record_sha256=>sha256_hex($content)};
  }elsif($type eq'0'||$type eq'5'){
   my$is_dir=$type eq'5';die"directory content\n"if$is_dir&&$size;
   my$path=safe_path(defined$pending?$pending->{path}:$raw,$is_dir);
   if($path eq''){die"unexpected root\n"if defined$source_revision||!$is_dir||$root_seen++;$offset=$end;next;}
   die"duplicate archive path\n"if exists$files{$path}||exists$dirs{$path};
   if($is_dir){$dirs{$path}=1;}else{
    $files{$path}={path=>$path,bytes=>$size,sha256=>sha256_hex($content),git_blob=>sha1_hex("blob $size\0$content"),archive_mode=>$mode,content=>$content};
    if(defined$pending){push@pax,{%$pending,following_raw_path=>$raw,following_header_offset=>$offset,payload_bytes=>$size,payload_sha256=>sha256_hex($content)};$pending=undef;}
   }
  }else{die"unsupported tar type\n";}
  $offset=$end;
 }
 die"tar incomplete\n"unless$ended;die"source global metadata\n"if defined$source_revision&&!$global;
 die"generated root missing\n"unless defined$source_revision||$root_seen==1;
 return{files=>\%files,directories=>\%dirs,pax=>\@pax};
}
sub tree_inventory{
 my($base)=@_;my(@files,@dirs);my@s=lstat$base;die"tree root\n"unless@s&&S_ISDIR($s[2])&&$s[4]==$>;
 find({no_chdir=>1,wanted=>sub{my$p=$File::Find::name;return if$p eq$base;my@st=lstat$p;die"tree ownership\n"unless@st&&$st[4]==$>;
  my$r=substr($p,length($base)+1);if(S_ISDIR($st[2])){push@dirs,safe_path($r,1);return;}
  die"tree special/hardlink\n"unless S_ISREG($st[2])&&$st[3]==1;$r=safe_path($r,0);my$b=read_bytes($p);
  push@files,{path=>$r,bytes=>length$b,sha256=>sha256_hex($b),mode=>$st[2]&07777};
 }},$base);
 return{files=>[sort{$a->{path}cmp$b->{path}}@files],directories=>[sort@dirs]};
}
sub same_inventory{same($J->encode($_[0]),$J->encode($_[1]),$_[2]);}
sub expand_gzip{
 my($encoded,$expected,$label)=@_;my$gz=IO::Uncompress::Gunzip->new(\$encoded,MultiStream=>0)or die"gzip open $label: $IO::Uncompress::Gunzip::GunzipError\n";
 my$out='';while(1){my$n=$gz->read(my$part,65536);die"gzip read $label\n"if$n<0;last if$n==0;die"gzip expansion cap $label\n"if length($out)+$n>length($expected);$out.=$part;}
 $gz->close or die"gzip finish $label\n";same($out,$expected,"complete decoded gzip $label");return{bytes=>length($out),sha256=>sha256_hex($out)};
}
same(join("\0",@{direct_names($RUN)}),join("\0",sort qw(artifacts artifacts.sha256 compiler-input control factory generated-100k generator producer-A records setup-summary.json)),'run top-level population');
copy_metadata("$RUN/control",'raw/control');copy_metadata("$RUN/records",'raw/records');copy_metadata($LAUNCH,'raw/launch');
my$manifest=read_bytes("$RUN/control/controls.manifest.sha256");same(sha256_hex($manifest),$PIN,'released control manifest');
my%cp;for my$l(split/\n/,$manifest){die"control manifest framing\n"unless$l=~/\A([0-9a-f]{64})  ([A-Za-z0-9_.-]+)\z/;die"duplicate control\n"if exists$cp{$2};$cp{$2}=$1;}
same(join("\0",sort keys%cp),join("\0",sort qw(README.md controls.json setup.pl setup.sh)),'control population');
for my$n(qw(README.md controls.json setup.pl setup.sh controls.manifest.sha256)){
 my$b=read_bytes("$RUN/control/$n");same($b,git('show',"$RELEASE:bench/compiler-reference-capture-data/setup-preparation-2/$n"),"committed control $n");
 same(sha256_hex($b),$cp{$n},"control hash $n")if$n ne'controls.manifest.sha256';
}
my$config=decode_json(read_bytes("$RUN/control/controls.json"));
same($config->{run_root},$RUN,'configured scope');die"A/B authorized by setup\n"if$config->{comparison_authorized};
die"original cohort reopened\n"unless$config->{original_cohort}{status}eq'CLOSED_AFTER_SETUP_FAILURE'&&$config->{original_cohort}{all_twelve_comparison_rows}eq'not-run'&&!$config->{original_cohort}{resumable}&&$config->{original_cohort}{native_commands_run}==0;
my$closed=git('show',"$RELEASE:bench/compiler-reference-capture-data/original-cohort-closed.json");save('original-cohort-closed.json',$closed);
same(read_bytes("$LAUNCH/release-commit"),"$RELEASE\n",'release commit');same(read_bytes("$LAUNCH/status"),"0\n",'outer success');
same(read_bytes("$LAUNCH/argv"),"bash target/compiler-reference-capture-setup-preparation-2/setup.sh --execute-after-root-release $PIN\n",'outer command');
same(read_bytes("$LAUNCH/cwd"),"$REPO\n",'outer cwd');same(read_bytes("$LAUNCH/stderr"),'','outer diagnostics');
same(read_bytes("$LAUNCH/controls-before.sha256"),read_bytes("$LAUNCH/controls-after.sha256"),'outer control pin stability');
die"outer control pin\n"unless read_bytes("$LAUNCH/controls-before.sha256")=~/\A\Q$PIN\E  target\/compiler-reference-capture-setup-preparation-2\/controls\.manifest\.sha256\n\z/;
same(read_bytes("$RUN/records/released-controls.sha256"),"$PIN\n",'copied release pin');
same(read_bytes("$RUN/records/status"),"0\n",'run status');same(read_bytes("$RUN/records/last-phase"),"complete\n",'last phase');
same(read_bytes("$RUN/records/completion"),"setup-complete; generated archive verified; A/B remains unreleased\n",'completion boundary');
my@phases=qw(recover-inputs prepare-factory generator-build generate-100k freeze-artifacts);
my$table="phase\toutcome\tcommand_status\tcapture_validation_status\n";$table.="$_\tok\t0\t0\n"for@phases;
same(read_bytes("$RUN/records/phases.tsv"),$table,'phase outcomes');
my%expected=(
'recover-inputs'=>['/usr/bin/perl',"$RUN/control/setup.pl",'recover',$RUN,"$RUN/control/controls.json"],
'prepare-factory'=>['/usr/bin/perl',"$RUN/control/setup.pl",'factory',$RUN,"$RUN/control/controls.json"],
'generator-build'=>["$RUN/producer-A",'build','src/main.lkj','--source-dir',"$RUN/factory",'--library-dir',"$RUN/factory/library",'--runtime-source',"$RUN/factory/library/runtime.lkj",'--inline-limit','0','--no-store','--stats','-o',"$RUN/generator"],
'generate-100k'=>["$RUN/generator",'generate',"$RUN/generated-100k",'100000','20261116'],
'freeze-artifacts'=>['/usr/bin/perl',"$RUN/control/setup.pl",'freeze',$RUN,"$RUN/control/controls.json"]);
my@phase_results;my$last_end=0;
sub timing{
 my($text,$outer)=@_;my%v;while($text=~/(?:\A|\s)(wall_s|user_s|system_s|max_rss_kib|maxrss_kib|exit|exit_status)=([0-9]+(?:\.[0-9]+)?)(?=\s|\z)/g){die"duplicate timing\n"if exists$v{$1};$v{$1}=$2;}
 my@names=$outer?qw(wall_s user_s system_s maxrss_kib exit_status):qw(wall_s user_s system_s max_rss_kib exit);
 same(join("\0",sort keys%v),join("\0",sort@names),'timing population');die"timing status\n"unless$v{$outer?'exit_status':'exit'}==0;return\%v;
}
for my$n(@phases){
 my$p="$RUN/records/$n";same(read_bytes("$p/status"),"0\n","$n status");same(read_bytes("$p/validation.status"),"0\n","$n validation");
 same_inventory(decode_json(read_bytes("$p/argv.json")),$expected{$n},"$n argv");
 my@wrapper=('/usr/bin/time','-f','wall_s=%e\nuser_s=%U\nsystem_s=%S\nmax_rss_kib=%M\nexit=%x','-o',"$p/time",'/usr/bin/timeout','--kill-after=10s','900s','/usr/bin/prlimit','--as=4294967296','--',@{$expected{$n}});
 same_inventory(decode_json(read_bytes("$p/wrapper-argv.json")),\@wrapper,"$n wrapper");
 my$cwd=$n eq'generator-build'?"$RUN/factory":$n eq'generate-100k'?$RUN:$REPO;
 same(read_bytes("$p/cwd"),"$cwd\n","$n cwd");same(read_bytes("$p/cwd.actual"),"$cwd\n","$n actual cwd");
 same(read_bytes("$p/stdin"),($n eq'generate-100k'?"$RUN/artifacts/shape":'/dev/null')."\n","$n stdin");
 my$free=read_bytes("$p/free-bytes");die"free-space capture\n"unless$free=~/\A[0-9]+\n\z/&&$free>=3221225472;
 my$t=timing(read_bytes("$p/time"),0);my$v=decode_json(read_bytes("$p/validation.stdout"));
 die"validation result\n"unless$v->{phase}eq$n&&$v->{status}==0&&$v->{complete_metrics};
 for my$k(keys%$t){same("$v->{timing}{$k}","$t->{$k}","$n validated timing $k");}
 same(read_bytes("$p/validation.stderr"),'','validation diagnostics');
 my$start=read_bytes("$p/start.ns");my$end=read_bytes("$p/end.ns");chomp($start,$end);
 die"phase sequencing\n"unless$start=~/\A[0-9]{19}\z/&&$end=~/\A[0-9]{19}\z/&&$start>=$last_end&&$end>=$start;$last_end=$end;
 same("$v->{outer_start_ns}","$start",'validated start');same("$v->{outer_end_ns}","$end",'validated end');
 for my$k(qw(start.utc end.utc)){die"UTC capture\n"unless read_bytes("$p/$k")=~/\A[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z\n\z/;}
 for my$k(qw(load-before load-after)){die"load capture\n"unless read_bytes("$p/$k")=~/\A[0-9]+(?:\.[0-9]+)? [0-9]+(?:\.[0-9]+)? [0-9]+(?:\.[0-9]+)?\n\z/;}
 if($n ne'generator-build'){same(read_bytes("$p/stderr"),'',"non-build stderr $n");same(read_bytes("$p/stats.raw"),'',"non-build stats $n");}
 push@phase_results,{phase=>$n,status=>0,capture_validation_status=>0,argv=>$expected{$n},cwd=>$cwd,timing=>$t,load_before=>read_bytes("$p/load-before"),load_after=>read_bytes("$p/load-after"),start_ns=>$start,end_ns=>$end};
}
my$outer_time=timing(read_bytes("$LAUNCH/time"),1);
my$summary_bytes=read_bytes("$RUN/setup-summary.json");my$summary=decode_json($summary_bytes);save('setup-summary.json',$summary_bytes);save('artifacts.sha256',read_bytes("$RUN/artifacts.sha256"));
die"setup summary boundary\n"unless$summary->{status}eq'setup-complete-not-an-ab-release'&&!$summary->{comparison_executed};
my%artifact;for my$e(@{$summary->{artifacts}}){die"artifact path/duplicate\n"unless$e->{path}=~/\Aartifacts\/[A-Za-z0-9_.-]+\z/&&!exists$artifact{$e->{path}};my$b=read_bytes("$RUN/$e->{path}");assert_pin($b,$e,$e->{path});$artifact{$e->{path}}=$e;}
same(join("\0",@{direct_names("$RUN/artifacts")}),join("\0",sort map{s{\Aartifacts/}{}r}keys%artifact),'artifact population');
my$expected_artifact_manifest=join('',map{"$artifact{$_}{sha256}  $_\n"}sort keys%artifact).sha256_hex($summary_bytes)."  setup-summary.json\n";
same(read_bytes("$RUN/artifacts.sha256"),$expected_artifact_manifest,'complete artifact manifest');
my$baseline=read_bytes("$RUN/producer-A");assert_pin($baseline,$config->{baseline},'actual A');assert_pin($baseline,$summary->{baseline},'summary A');
my$seed=read_bytes("$RUN/artifacts/baseline-A.gz");same($seed,git('show',"$config->{baseline}{recovery}{commit}:$config->{baseline}{recovery}{path}"),'exact seed Git bytes');
assert_pin($seed,{sha256=>$config->{baseline}{recovery}{archive_sha256},bytes=>$config->{baseline}{recovery}{archive_bytes}},'seed compressed pin');
my$a_recovery=expand_gzip($seed,$baseline,'A');
my$generator_source=read_bytes("$RUN/artifacts/scale-generator-source.lkj");assert_pin($generator_source,$config->{generator_source},'generator source');
same($generator_source,git('show',"$config->{generator_source}{revision}:$config->{generator_source}{path}"),'generator Git bytes');
my$shape=read_bytes("$RUN/artifacts/shape");assert_pin($shape,$config->{shape},'shape');same($shape,git('show',"$config->{shape}{revision}:$config->{shape}{path}"),'shape Git bytes');
my$tar=read_bytes("$RUN/artifacts/compiler-source.tar");assert_pin($tar,$config->{compiler_input},'compiler source tar');
my$source=parse_tar($tar,$config->{compiler_input}{revision});
die"exact source PAX count\n"unless@{$source->{pax}}==2;
my$git_tree=git('ls-tree','-r','-z','--full-tree',$config->{compiler_input}{revision},'--','src','library','tests');
my(%git,%git_dirs);for my$r(split/\0/,$git_tree){die"Git tree record\n"unless$r=~/\A(100644|100755) blob ([0-9a-f]{40})\t((?:src|library|tests)\/[A-Za-z0-9_.\/-]+)\z/;die"duplicate Git path\n"if exists$git{$3};$git{$3}={mode=>$1,blob=>$2};my@parts=split m{/},$3;pop@parts;while(@parts){$git_dirs{join('/',@parts)}=1;pop@parts;}}
die"source Git population\n"unless keys(%git)==1742;
same(join("\0",sort keys%{$source->{files}}),join("\0",sort keys%git),'source complete file set');
same(join("\0",sort keys%{$source->{directories}}),join("\0",sort keys%git_dirs),'source complete directory set');
my(@source_files,@archive_files);
for my$p(sort keys%git){
 my$f=$source->{files}{$p};same($f->{git_blob},$git{$p}{blob},"source Git content $p");die"source archive executable mode\n"unless!!($f->{archive_mode}&0111)==($git{$p}{mode}eq'100755');
 push@source_files,{path=>$p,bytes=>$f->{bytes},sha256=>$f->{sha256},mode=>$git{$p}{mode}eq'100755'?0700:0600};
 push@archive_files,{path=>$p,bytes=>$f->{bytes},sha256=>$f->{sha256},git_mode=>$git{$p}{mode},git_blob=>$git{$p}{blob},archive_mode=>$f->{archive_mode}};
}
my$source_inventory={files=>\@source_files,directories=>[sort keys%git_dirs]};
same_inventory(decode_json(read_bytes("$RUN/artifacts/compiler-archive-members.json")),\@archive_files,'recorded compiler archive members');
same_inventory(decode_json(read_bytes("$RUN/artifacts/factory-archive-members.json")),\@archive_files,'recorded factory archive members');
same_inventory(tree_inventory("$RUN/compiler-input"),$source_inventory,'all live compiler input bytes/modes');
same_inventory(decode_json(read_bytes("$RUN/artifacts/compiler-inputs.json")),$source_inventory,'recorded compiler input');
same_inventory(decode_json(read_bytes("$RUN/artifacts/factory-before.json")),$source_inventory,'factory before');
same(read_bytes("$RUN/artifacts/factory-main.original.lkj"),$source->{files}{'src/main.lkj'}{content},'original main');
my@factory_files=map{{%$_}}@source_files;for my$f(@factory_files){next unless$f->{path}eq'src/main.lkj';$f->{bytes}=length$generator_source;$f->{sha256}=sha256_hex($generator_source);}
my$factory_inventory={files=>\@factory_files,directories=>[sort keys%git_dirs]};
same_inventory(tree_inventory("$RUN/factory"),$factory_inventory,'live factory only-main overlay');
same_inventory(decode_json(read_bytes("$RUN/artifacts/factory-after.json")),$factory_inventory,'recorded factory after');
assert_pin(read_bytes("$RUN/compiler-input/library/runtime.lkj"),{sha256=>$config->{compiler_input}{runtime_sha256}},'runtime');
save('source-members.sha256',join('',map{"$_->{sha256}  $_->{path}\n"}@source_files));
json_save('source-pax-paths.json',$source->{pax});
print"Source archive: all 1,742 Git members, 42 directories and two PAX paths verified; factory differs only at main\n";
my$generator=read_bytes("$RUN/generator");assert_pin($generator,$summary->{generator},'actual generator');
my$generator_recovery=expand_gzip(read_bytes("$RUN/artifacts/generator.gz"),$generator,'generator');
my$build_stderr=read_bytes("$RUN/records/generator-build/stderr");
my@stats=$build_stderr=~/^LKJSCRIPT_STATS (.+)$/mg;die"generator stats population\n"unless@stats==1&&$build_stderr eq"LKJSCRIPT_STATS $stats[0]\n";
my$stats=decode_json($stats[0]);die"generator terminal stats\n"unless$stats->{route}eq'compiler'&&$stats->{stopped_stage}eq'done'&&$stats->{emitted_bytes}==length$generator;
for my$k(qw(front_reused checked_reused checked_local_reused emitted_reused build_reused inline_attempts)){die"generator reused/inline $k\n"unless exists$stats->{$k}&&$stats->{$k}==0;}
same_inventory(decode_json(read_bytes("$RUN/artifacts/generator-build-stats.json")),$stats,'stored generator stats');
same(read_bytes("$RUN/records/generator-build/stats.raw"),$build_stderr,'stats extraction');same(read_bytes("$RUN/records/generator-build/stdout"),'','generator build stdout');
my$generated_inventory=tree_inventory("$RUN/generated-100k");same_inventory($generated_inventory,decode_json(read_bytes("$RUN/artifacts/generated-members.json")),'live generated inventory');
my$workload=read_bytes("$RUN/generated-100k/workload.txt");same($workload,read_bytes("$RUN/artifacts/workload.txt"),'workload artifact');same($workload,read_bytes("$RUN/records/generate-100k/stdout"),'workload stdout');save('workload.txt',$workload);
my%fields;for my$l(split/\n/,$workload){die"workload framing\n"unless$l=~/\A([a-z_]+) (\S+)\z/;die"duplicate workload field\n"if exists$fields{$1};$fields{$1}=$2;}
die"workload input seed/lines\n"unless$fields{seed}eq'20261116'&&$fields{lines}eq'100000';
my($lines,$source_bytes,$files)=(0,0,0);
for my$f(@{$generated_inventory->{files}}){next if$f->{path}eq'workload.txt';die"unexpected generated extension\n"unless$f->{path}=~/\.lkj\z/;my$b=read_bytes("$RUN/generated-100k/$f->{path}");$lines+=($b=~tr/\n/\n/);++$lines if length($b)&&$b!~/\n\z/;$source_bytes+=length$b;++$files;}
die"generated full census\n"unless$lines==100000&&$files==169&&$source_bytes==3923070&&$fields{files}==$files&&$fields{source_bytes}==$source_bytes&&@{$generated_inventory->{files}}==170;
my$generated_tar=read_bytes("$RUN/artifacts/generated-100k.tar");
my$generated_gzip=read_bytes("$RUN/artifacts/generated-100k.tar.gz");assert_pin($generated_gzip,$summary->{generated_archive},'generated gzip summary');
my$generated_recovery=expand_gzip($generated_gzip,$generated_tar,'generated tar');
my$generated=parse_tar($generated_tar,undef);my%recorded=map{$_->{path}=>$_}@{$generated_inventory->{files}};
same(join("\0",sort keys%{$generated->{files}}),join("\0",sort keys%recorded),'generated archive complete file set');
same(join("\0",sort keys%{$generated->{directories}}),join("\0",@{$generated_inventory->{directories}}),'generated archive complete directories');
for my$p(sort keys%recorded){my$f=$generated->{files}{$p};same($f->{content},read_bytes("$RUN/generated-100k/$p"),"exact generated archive member $p");die"generated archive mode\n"unless$f->{archive_mode}==$recorded{$p}{mode};}
my$generated_manifest=join('',map{"$_->{sha256}  $_->{path}\n"}@{$generated_inventory->{files}});
same(read_bytes("$RUN/artifacts/generated-members.sha256"),$generated_manifest,'recorded generated hashes');save('generated-members.sha256',$generated_manifest);
same_inventory($summary->{generated},{lines=>$lines,source_bytes=>$source_bytes,source_files=>$files,total_regular_members=>170},'summary generated census');
json_save('factory-overlay.json',{changed_paths=>['src/main.lkj'],before_sha256=>$source->{files}{'src/main.lkj'}{sha256},after_sha256=>sha256_hex($generator_source),unchanged_file_count=>1741});
# Reread the exact inventoried inputs, including source trees; retain any drift instead of re-running setup.
for my$p(sort keys%pins){read_bytes($p);}
my$input_bytes=0;$input_bytes+=$_->{bytes}for values%pins;
my$result={
 schema=>1,status=>'accepted completed separate setup2; not an A/B release',release_commit=>$RELEASE,controls_manifest_sha256=>$PIN,
 original_cohort=>{status=>'CLOSED_AFTER_SETUP_FAILURE',all_twelve_ab_rows=>'not-run',resumable=>JSON::PP::false,closure_record_sha256=>sha256_hex($closed)},
 setup2=>{outer_status=>0,phase_count=>5,all_command_and_validation_statuses_zero=>JSON::PP::true,generator_build_commands=>1,generation_commands=>1,comparison_commands=>0},
 phases=>\@phase_results,outer_time=>$outer_time,outer_load_before=>read_bytes("$LAUNCH/load-before"),outer_load_after=>read_bytes("$LAUNCH/load-after"),
 source=>{revision=>$config->{compiler_input}{revision},archive=>$config->{compiler_input},regular_members=>1742,directories=>42,pax_paths=>$source->{pax},all_git_blob_identities_match=>JSON::PP::true,all_live_compiler_files_match=>JSON::PP::true},
 factory=>{only_changed_path=>'src/main.lkj',unchanged_files=>1741,complete_population_and_modes_match=>JSON::PP::true},
 baseline=>$summary->{baseline},generator=>$summary->{generator},generator_stats=>$stats,
 gzip_recovery=>{baseline=>$a_recovery,generator=>$generator_recovery,generated_tar=>$generated_recovery,method=>'bounded in-memory decoding of existing gzip bytes; no output gzip or reconstructed tree written'},
 generated=>{source_files=>$files,source_bytes=>$source_bytes,lines=>$lines,regular_members=>170,directories=>scalar(keys%{$generated->{directories}}),archive_sha256=>sha256_hex($generated_tar),archive_bytes=>length($generated_tar),gzip_sha256=>sha256_hex($generated_gzip),gzip_bytes=>length($generated_gzip),all_archive_members_byte_equal=>JSON::PP::true},
 inputs_checked=>scalar(keys%pins),input_bytes=>$input_bytes,all_reread_inputs_unchanged=>JSON::PP::true,
 recovery=>{existing_run_artifacts=>'Root retains the original compiler-source.tar, generated-100k.tar.gz, generator.gz, source/factory manifests, exact controls and original records before any comparison release. This audit copies bounded metadata only.',source_tar_sha256=>$config->{compiler_input}{sha256},baseline_git_seed=>"$config->{baseline}{recovery}{commit}:$config->{baseline}{recovery}{path}",generator_gzip_sha256=>sha256_hex(read_bytes("$RUN/artifacts/generator.gz"))},
 limitations=>['No claim of identity with cleaned historical generated sources.','Construction/generation timings are setup observations, not A/B performance evidence.','Full recorded metadata and bounded offline checks establish this result; no process/FD/syscall observation.'],
 no_original_mutation=>JSON::PP::true,no_compiler_native_generator_or_setup_execution=>JSON::PP::true,no_compression=>JSON::PP::true,no_key_or_store_read=>JSON::PP::true
};
json_save('acceptance.json',$result);json_save('input-pins.json',[map{$pins{$_}}sort keys%pins]);json_save('commands.json',\@commands);
print"PASS five phases; exact 1,742 inputs; generator=".sha256_hex($generator)."; generated=$files files/$lines lines/$source_bytes bytes\n";
print"Existing generator/seed/generated archive gzips decode exactly; all 170 generated archive members match\n";
print"Original cohort remains closed; no A/B commands or new workloads executed by this audit\n";
