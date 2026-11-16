use strict;
use warnings;
use Fcntl qw(:DEFAULT :mode);
use File::Find;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use IPC::Open2;
use Encode qw(encode);
my $ROOT='/home/coder/workspace/lkjscript';
my $BASE='0b0b27803d27b932a5b91cde2d766bd3c8bb82b9';
my $PREP="$ROOT/target/compiler-reference-capture-integration-preparation-1";
my $DESIRED="$ROOT/target/compiler-reference-capture-desired-1";
my $CORE="$ROOT/target/compiler-reference-capture-core-operation-1";
my $TEST="$ROOT/target/compiler-reference-capture-tests-operation-1";
my $OUT="$ROOT/target/compiler-reference-capture-adoption-acceptance-1";
my @CORE_FILES=qw(src/driver.lkj src/elaborate.lkj src/elaborate/definition_types.lkj src/elaborate/definition_types/pattern_equations.lkj src/elaborate/diagnostic_inference.lkj src/elaborate/generic_check.lkj src/elaborate/incremental.lkj);
my @TEST_FILES=qw(src/semantic_tests.lkj src/semantic_main.lkj src/semantic_tests/reference_capture.lkj);
my %EXPECTED=map{$_=>1}(@CORE_FILES,@TEST_FILES);
my $J=JSON::PP->new->canonical->pretty->utf8;
my(%evidence,@commands);
$|=1;
sub raw {
 my($p)=@_;my@s=lstat($p);die"not regular $p\n"unless@s&&S_ISREG($s[2]);
 sysopen my$f,$p,O_RDONLY|O_NOFOLLOW or die"open $p: $!\n";binmode$f;local$/;my$b=<$f>;close$f or die"close $p: $!\n";return defined($b)?$b:'';
}
sub evidence {
 my($p)=@_;my$b=raw($p);$evidence{$p}={path=>$p,bytes=>length($b),sha256=>sha256_hex($b)};return$b;
}
sub pinned {my($p,$pin)=@_;my$b=evidence($p);die"pin $p\n"unless sha256_hex($b)eq$pin;return$b;}
sub save {
 my($name,$bytes)=@_;sysopen my$f,"$OUT/$name",O_WRONLY|O_CREAT|O_EXCL,0600 or die"new output $name: $!\n";
 binmode$f;print{$f}$bytes or die"write $name: $!\n";close$f or die"close $name: $!\n";
}
sub json_save {save($_[0],$J->encode($_[1]));}
sub command {
 my($argv,$allow_one)=@_;open my$f,'-|',@$argv or die"command open: $!\n";binmode$f;local$/;my$b=<$f>;close$f;my$code=$?>>8;my$signal=$?&127;
 push@commands,{argv=>$argv,exit=>$code,signal=>$signal};
 die"command refusal $code signal=$signal: @$argv\n"if$signal||($code!=0&&!($allow_one&&$code==1));
 return(defined($b)?$b:'',$code);
}
sub manifest {
 my($bytes)=@_;my%map;for my$l(split/\n/,$bytes){die"manifest framing\n"unless$l=~/\A([0-9a-f]{64})  ((?:src|library|tests)\/[A-Za-z0-9_.\/-]+)\z/;my($h,$p)=($1,$2);die"duplicate manifest $p\n"if exists$map{$p};$map{$p}=$h;}return\%map;
}
sub same_map {
 my($left,$right,$label)=@_;die"$label population differs\n"unless join("\0",sort keys%$left)eq join("\0",sort keys%$right);
 for my$p(keys%$left){die"$label hash differs $p\n"unless$left->{$p}eq$right->{$p};}
}
sub changes {
 my($before,$after)=@_;my%union=map{$_=>1}(keys%$before,keys%$after);return[sort grep{!exists$before->{$_}||!exists$after->{$_}||$before->{$_}ne$after->{$_}}keys%union];
}
sub need_set {
 my($actual,$want,$label)=@_;die"$label unexpected paths: ".join(',',@$actual)."\n"unless join("\0",sort@$actual)eq join("\0",sort@$want);
}
sub raw_manifest {my($map)=@_;return join('',map{"$map->{$_}  $_\n"}sort keys%$map);}
sub hash_regular {
 my($path)=@_;my@s=lstat($path);die"not regular $path\n"unless@s&&S_ISREG($s[2]);
 sysopen my$f,$path,O_RDONLY|O_NOFOLLOW or die"open $path: $!\n";binmode$f;my@opened=stat$f;
 die"open changed $path\n"unless@opened&&$s[0]==$opened[0]&&$s[1]==$opened[1];
 my$d=Digest::SHA->new(256);$d->addfile($f);my@end=stat$f;close$f or die$!;
 die"source changed while read $path\n"unless@end&&$end[7]==$opened[7]&&$end[9]==$opened[9]&&$end[10]==$opened[10];
 return$d->hexdigest;
}
sub scan_sources {
 my($root)=@_;my%map;
 for my$part(qw(src library tests)){find({no_chdir=>1,follow=>0,wanted=>sub{
  my$p=$File::Find::name;my@s=lstat$p;die"lstat $p\n"unless@s;return if S_ISDIR($s[2]);die"nonregular source $p\n"unless S_ISREG($s[2]);
  my$r=substr($p,length($root)+1);die"duplicate live path\n"if exists$map{$r};$map{$r}=hash_regular($p);
 }}, "$root/$part");}
 return\%map;
}
chdir$ROOT or die"cwd: $!\n";
my$freeze=decode_json(pinned("$PREP/freeze.json",'dfe7d72cd6ebf0ee7f42ac3553585585b84b05f27817dfc94980bb564dbb0a82'));
my$planned=decode_json(pinned("$PREP/commands.json",'537b1b999b17f40b96552a218e2a15f0f6156fce0766986b135612a6d69d7d2e'));
my$request_bytes=pinned("$PREP/tests-request.json",'3721803e8ed288cbe6687f35cee818f6bf403310bac9c234adf1eeb650acbd37');
my$request=decode_json($request_bytes);
my$desired_bytes=pinned("$PREP/desired-source.sha256",'e34cdb687a9e7195e686f38bf53a6f55c1b38f7f1769f60605ba36361b6119b7');
my$desired_manifest=manifest($desired_bytes);
die"frozen desired count\n"unless keys(%$desired_manifest)==1743;
my$core_before_bytes=evidence("$CORE/source-before.sha256");
my$core_after_bytes=evidence("$CORE/source-after.sha256");
my$test_before_bytes=evidence("$TEST/source-before.sha256");
my$test_after_bytes=evidence("$TEST/source-after.sha256");
die"unrecorded inter-transaction delta\n"unless$core_after_bytes eq$test_before_bytes;
my$core_before=manifest($core_before_bytes);my$core_after=manifest($core_after_bytes);
my$test_before=manifest($test_before_bytes);my$test_after=manifest($test_after_bytes);
die"source population counts\n"unless keys(%$core_before)==1742&&keys(%$core_after)==1742&&keys(%$test_after)==1743;
need_set(changes($core_before,$core_after),\@CORE_FILES,'core');
need_set(changes($test_before,$test_after),\@TEST_FILES,'tests');
for my$n(qw(desired-before.sha256 desired-after.sha256)){die"desired freeze at second transaction\n"unless evidence("$TEST/$n")eq$desired_bytes;}
for my$n(qw(request-before.sha256 request-after.sha256)){my$b=evidence("$TEST/$n");die"request record\n"unless$b=~/\A3721803e8ed288cbe6687f35cee818f6bf403310bac9c234adf1eeb650acbd37  (?:\Q$ROOT\E\/)?target\/compiler-reference-capture-integration-preparation-1\/tests-request\.json\n\z/;}
for my$dir($CORE,$TEST){
 for my$n(qw(producer-before.sha256 producer-after.sha256)){
  my$b=evidence("$dir/$n");
  die"operation producer record\n"unless$b eq"6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc  $ROOT/target/lkjscript\n";
 }
 die"operation exit\n"unless evidence("$dir/status")eq"0\n";
 die"operation cwd\n"unless evidence("$dir/cwd")eq"$ROOT\n";
 for my$n(qw(start.utc end.utc start.ns end.ns load-before load-after time limits base-commit stderr)){evidence("$dir/$n");}
}
print"captured transaction/source/request/producer chains verified\n";

# Verify the complete initial population directly against exact Git blobs.
my($tree)=command(['/usr/bin/git','-C',$ROOT,'ls-tree','-r','-z','--full-tree',$BASE,'--','src','library','tests'],0);
my%blobs;
for my$row(split/\0/,$tree){die"Git tree framing\n"unless$row=~/\A(100644|100755) blob ([0-9a-f]{40})\t((?:src|library|tests)\/[A-Za-z0-9_.\/-]+)\z/;die"duplicate Git path\n"if exists$blobs{$3};$blobs{$3}=$2;}
die"Git source population differs\n"unless join("\0",sort keys%blobs)eq join("\0",sort keys%$core_before);
my($reader,$writer);my$pid=open2($reader,$writer,'/usr/bin/git','-C',$ROOT,'cat-file','--batch');
binmode$reader;binmode$writer;my$previous=select($writer);$|=1;select$previous;
sub take {
 my($f,$n)=@_;my$b='';while(length($b)<$n){my$r=read($f,my$v,$n-length($b));die"batch read\n"unless defined$r&&$r;$b.=$v;}return$b;
}
my%git_hashes;my%baseline_changed;
for my$p(sort keys%blobs){
 print{$writer}"$blobs{$p}\n"or die"batch input\n";my$h=<$reader>;
 die"batch header\n"unless defined$h&&$h=~/\A([0-9a-f]{40}) blob ([0-9]+)\n\z/&&$1 eq$blobs{$p};my$n=$2;
 my$b=take($reader,$n);die"batch framing\n"unless take($reader,1)eq"\n";
 my$sha=sha256_hex($b);die"baseline not exact Git $p\n"unless$sha eq$core_before->{$p};
 $git_hashes{$p}=$sha;$baseline_changed{$p}=$b if$EXPECTED{$p};
}
close$writer or die"batch input close\n";close$reader or die"batch output close\n";waitpid($pid,0);die"batch status $?\n"if$?;
push@commands,{argv=>['/usr/bin/git','-C',$ROOT,'cat-file','--batch'],exit=>0,objects=>scalar(keys%blobs)};
save('baseline-git.sha256',raw_manifest(\%git_hashes));
print"all ".scalar(keys%blobs)." initial source/library/test files match baseline Git\n";

my@phase_results;
for my$i(0,1){
 my$dir=$i?$TEST:$CORE;my$before=$i?$test_before:$core_before;my$after=$i?$test_after:$core_after;my$expected=$i?\@TEST_FILES:\@CORE_FILES;
 my$out=evidence("$dir/stdout");my%facts;
 while($out=~/^"([^"\r\n]+)" level=source-claim file before=(absent|[0-9a-f]{64}) after=([0-9a-f]{64})$/mg){
  my($p,$bh,$ah)=($1,$2,$3);die"file fact outside main\n"unless$p=~s{\A\Q$ROOT\E/}{};die"duplicate file fact\n"if exists$facts{$p};
  die"file before claim $p\n"unless(defined$before->{$p}?$before->{$p}:'absent')eq$bh;
  die"file after claim $p\n"unless defined$after->{$p}&&$after->{$p}eq$ah;
  $facts{$p}={path=>$p,before=>$bh,after=>$ah};
 }
 need_set([keys%facts],$expected,'reported files');
 my($sites,$file_count,$identities)=$out=~/^transaction sites=(\d+) files=(\d+) identities=(\d+)$/m;
 die"transaction counts\n"unless defined$sites&&$file_count==@$expected&&$sites==($i?3:20)&&$identities==($i?4:18);
 my($meaning_before,$meaning_after,$checked,$fresh,$reused,$resumed,$library)=$out=~/^program level=checked-ir before=([0-9a-f]{64}) after=([0-9a-f]{64}) checked-instances=(\d+) fresh-checked=(\d+) proof-reused=(\d+) frontend-resumed=(\d+) library-validation=(\S+)$/m;
 die"checked program record\n"unless defined$meaning_after&&$checked==($i?10532:12924)&&$fresh==$checked&&$reused==0&&$resumed==0&&$library eq'demanded';
 die"apply publication missing\n"unless$out=~/^transaction source-before=[0-9a-f]{64} source-after=[0-9a-f]{64} mode=apply$/m;
 my$operation_count;
 if(!$i){
  my($d,$c,$n)=$out=~/^replay desired=([0-9a-f]{64}) candidate=([0-9a-f]{64}) verified=true operations=(\d+)$/m;
  die"replay equality\n"unless defined$d&&$d eq$c&&$c eq$meaning_after&&$n==21
      &&$c eq'59681227ba0d1ace14608e4363d8901ccf2466c2d6e7e49e19102efe8bb31c16';
  $operation_count=$n;
  for my$id(@{$freeze->{private_assertion_items}}){die"private assertion omitted $id\n"unless$out=~/^"\Q$id\E" level=source-claim changed=added$/m;}
 }else{
  die"explicit request has unexpected replay marker\n"if$out=~/^replay /m;
  die"test checked meaning\n"unless$meaning_after eq'0b770ac6868af0bd5a3647b4bb003af31ba1bafdb45a0934a98d8b402854dc4d';
  $operation_count=scalar@{$request->{operations}};
 }
 my$argv_bytes=evidence("$dir/argv");die"argv terminator\n"unless$argv_bytes=~/\0\z/;my@argv=split/\0/,$argv_bytes;
 my@expected_argv=@{$planned->{stages}[$i]{argv}};my$request_path_spelling='exact';
 if($i){for my$at(0..$#argv-1){if($argv[$at]eq'--request'&&$argv[$at+1]eq"$ROOT/$expected_argv[$at+1]"){$argv[$at+1]=$expected_argv[$at+1];$request_path_spelling='absolute equivalent of the planned relative request path';}}}
 die"actual argv differs from prepared command\n"unless join("\0",@argv)eq join("\0",@expected_argv);
 my@authority=grep{/ level=checked-ir authority-gained=/}split/\n/,$out;
 push@phase_results,{phase=>$i?'explicit-tests':'core-replay',capture=>$dir,operations=>0+$operation_count,sites=>0+$sites,
  files=>[map{$facts{$_}}sort keys%facts],identities=>0+$identities,meaning_before=>$meaning_before,meaning_after=>$meaning_after,
  fresh_checked=>0+$fresh,proof_reused=>0+$reused,frontend_resumed=>0+$resumed,library_validation=>$library,
  authority_rows=>\@authority,argv_verified=>JSON::PP::true,request_path_spelling=>$request_path_spelling,
  time=>evidence("$dir/time"),load_before=>evidence("$dir/load-before"),load_after=>evidence("$dir/load-after")};
}
die"request structure\n"unless$request->{version}==1&&@{$request->{operations}}==3;
my($add,$core,$closed)=@{$request->{operations}};
die"add request\n"unless$add->{op}eq'add-item'&&$add->{id}eq'semantic_tests'&&$add->{before}eq'semantic_tests::optimizer'
 &&$add->{source}eq'mod reference_capture;'&&$add->{projection}{path}eq'src/semantic_tests/reference_capture.lkj'
 &&sha256_hex(encode('UTF-8',$add->{projection}{source}))eq'694b8163ad7a1216b22941187b0f740b5f46d46c37500f84017b6cb9742091c5';
die"core request\n"unless$core->{op}eq'replace-body'&&$core->{id}eq'semantic_tests::core'
 &&sha256_hex(encode('UTF-8',$core->{source}))eq$freeze->{test_request}{core_body_sha256};
die"closed request\n"unless$closed->{op}eq'replace-body'&&$closed->{id}eq'closed_core'
 &&$closed->{source}eq'{ assert_eq!(semantic_tests::core(), 192usize); }';
print"all 10 reported publication hashes and 24 operations mapped to the captured chain\n";

my$live=scan_sources($ROOT);my$desired_live=scan_sources($DESIRED);
same_map($live,$test_after,'current final sources');
same_map($desired_live,$desired_manifest,'current frozen desired');
save('live-actual.sha256',raw_manifest($live));save('live-desired.sha256',raw_manifest($desired_live));
my@projection;my$exact=0;
for my$p(sort keys%$live){
 if($live->{$p}eq$desired_live->{$p}){$exact++;next;}
 my$actual=raw("$ROOT/$p");my$wanted=raw("$DESIRED/$p");
 my$stem=$p;$stem=~s{/}{__}g;
 my($diff,$exit)=command(['/usr/bin/git','-C',$ROOT,'diff','--no-index','--no-ext-diff','--no-renames','--',"$DESIRED/$p","$ROOT/$p"],1);
 die"projection difference not observed\n"unless$exit==1;
 save("projection-diffs/$stem.patch",$diff);
 my@actual_nonblank=grep{$_!~/\A[ \t\r]*\z/}split/\n/,$actual,-1;
 my@desired_nonblank=grep{$_!~/\A[ \t\r]*\z/}split/\n/,$wanted,-1;
 push@projection,{path=>$p,actual_sha256=>$live->{$p},desired_sha256=>$desired_live->{$p},
  actual_bytes=>length($actual),desired_bytes=>length($wanted),diff=>"projection-diffs/$stem.patch",
  same_nonblank_lines=>(join("\n",@actual_nonblank)eq join("\n",@desired_nonblank)?JSON::PP::true:JSON::PP::false),
  qualification=>'Descriptive comparison only. All diff hunks require bounded human review; no general whitespace/comment equivalence rule is inferred.'};
}
# Line accounting uses final retained deltas, not whole replacement source sizes.
my($ns)=command(['/usr/bin/git','-C',$ROOT,'diff','--no-ext-diff','--no-renames','--numstat','-z',$BASE,'--','*.lkj'],0);
save('retained-numstat-tracked.raw',$ns);
my%numstat;
for my$row(split/\0/,$ns){die"numstat framing\n"unless$row=~/\A(\d+)\t(\d+)\t(.+)\z/s;die"duplicate numstat\n"if exists$numstat{$3};$numstat{$3}={path=>$3,added=>0+$1,deleted=>0+$2};}
my($untracked)=command(['/usr/bin/git','-C',$ROOT,'ls-files','--others','--exclude-standard','-z','--','*.lkj'],0);
my@untracked=split/\0/,$untracked;
for my$p(@untracked){
 die"uncovered untracked language file $p\n"unless$p eq'src/semantic_tests/reference_capture.lkj'&&!exists$numstat{$p};
 my($added,$exit)=command(['/usr/bin/git','-C',$ROOT,'diff','--no-index','--no-ext-diff','--no-renames','--numstat','--','/dev/null',"$ROOT/$p"],1);
 save('retained-numstat-new.raw',$added);
 die"new file numstat\n"unless$exit==1&&$added=~/\A(\d+)\t0\t[^\n]+\n\z/;
 $numstat{$p}={path=>$p,added=>0+$1,deleted=>0,untracked_at_audit=>JSON::PP::true};
}
need_set([keys%numstat],[keys%EXPECTED],'retained language changes');
my($added,$deleted,$core_lines,$test_lines)=(0,0,0,0);
for my$p(keys%numstat){my$r=$numstat{$p};$added+=$r->{added};$deleted+=$r->{deleted};my$n=$r->{added}+$r->{deleted};
 if(grep{$_ eq$p}@CORE_FILES){$core_lines+=$n;}else{$test_lines+=$n;}
 die"final retained file not publication-backed $p\n"unless$live->{$p}eq$test_after->{$p};
}
my%groups;
for my$p(keys%$core_before){my($group)=$p=~/\A([^\/]+)/;$groups{$group}{baseline}++;$groups{$group}{unchanged}++if$core_before->{$p}eq$live->{$p};}
for my$p(keys%$live){my($group)=$p=~/\A([^\/]+)/;$groups{$group}{final}++;}
my$unchanged=grep{$core_before->{$_}eq$live->{$_}}keys%$core_before;
my$result={
 schema=>1,status=>'publication and retained-line chains verified; desired text discrepancies retained for bounded review',
 baseline_commit=>$BASE,operation_producer=>'6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc',
 producer_scope=>'Captured before/after operation records only; live target/lkjscript deliberately not read because root bootstrap may replace it.',
 original_population=>scalar(keys%$core_before),final_population=>scalar(keys%$live),all_baseline_files_match_git=>JSON::PP::true,
 source_groups=>\%groups,unchanged_original_files=>0+$unchanged,changed_existing_files=>9,new_files=>1,
 core_to_tests_manifest_bytes_equal=>JSON::PP::true,desired_manifest_records=>1743,
 desired_current_and_second_transaction_pin=>'e34cdb687a9e7195e686f38bf53a6f55c1b38f7f1769f60605ba36361b6119b7',
 core_desired_before_qualification=>'Core wrapper used a successful preflight manifest check, not a separately retained desired-before copy. The frozen preliminary manifest and the second transaction desired-before provide the retained before/after-core checkpoints; no missing copy is invented.',
 phases=>\@phase_results,requested_operations=>24,source_sites=>23,projection_exact_files=>$exact,projection_differences=>\@projection,
 retained_line_accounting=>{files=>[map{$numstat{$_}}sort keys%numstat],added=>$added,deleted=>$deleted,total=>$added+$deleted,
 operation_backed=>$added+$deleted,core=>$core_lines,explicit_tests=>$test_lines,uncovered=>0,
 src_library_denominator=>$added+$deleted,all_retained_lkj_denominator=>$added+$deleted,
 boundary=>'Final baseline-to-current added+deleted .lkj lines only. No transient desired edits, refused attempts, whole replacement sizes, shell/JSON/report/Rust/archive lines credited.'},
 host_tools=>'Two separately recorded shell edits are outside language adoption and do not invalidate the language baseline.',
 no_compiler_or_native=>JSON::PP::true,no_source_mutation=>JSON::PP::true,no_key_contents=>JSON::PP::true,
 final_commit_qualification=>'Working source after publication audited now. Root must match these hashes to eventual committed blobs; no future commit is claimed.'};
json_save('acceptance.json',$result);json_save('commands.json',\@commands);
json_save('input-pins.json',[map{$evidence{$_}}sort keys%evidence]);
print"PASS baseline=".scalar(keys%$core_before)." final=".scalar(keys%$live)." unchanged=$unchanged\n";
print"retained_lines=".($added+$deleted)." credited=".($added+$deleted)." core=$core_lines tests=$test_lines added=$added deleted=$deleted\n";
print"desired_exact=$exact differences=".scalar(@projection)."\n";
for my$d(@projection){print"desired_difference=$d->{path} nonblank_equal=$d->{same_nonblank_lines}\n";}
