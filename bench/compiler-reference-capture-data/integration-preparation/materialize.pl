use strict;
use warnings;
use Fcntl qw(:DEFAULT :mode);
use Errno qw(ENOENT);
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use Encode qw(decode encode FB_CROAK LEAVE_SRC);
my $ROOT='/home/coder/workspace/lkjscript';
my $DESIRED="$ROOT/target/compiler-reference-capture-desired-1";
my $PREP="$ROOT/target/compiler-reference-capture-integration-preparation-1";
my $BASE='0b0b27803d27b932a5b91cde2d766bd3c8bb82b9';
my $CHECK="$ROOT/target/compiler-reference-capture-semantics-4";
my @CORE=qw(src/driver.lkj src/elaborate.lkj src/elaborate/definition_types.lkj
    src/elaborate/definition_types/pattern_equations.lkj src/elaborate/diagnostic_inference.lkj
    src/elaborate/generic_check.lkj src/elaborate/incremental.lkj);
my @TEST=qw(src/semantic_tests/reference_capture.lkj src/semantic_tests.lkj src/semantic_main.lkj);
my $J=JSON::PP->new->canonical->pretty->utf8;
sub raw {
    my($p)=@_;my@s=lstat($p);die"not regular $p\n"unless@s&&S_ISREG($s[2]);
    sysopen my$f,$p,O_RDONLY|O_NOFOLLOW or die"open $p: $!\n";binmode$f;local$/;
    my$b=<$f>;close$f or die"close $p: $!\n";return defined($b)?$b:'';
}
sub save {my($n,$b)=@_;sysopen my$f,"$PREP/$n",O_WRONLY|O_CREAT|O_EXCL,0600 or die"new $n: $!\n";binmode$f;print{$f}$b or die$!;close$f or die$!;}
sub git_blob {
    my($p)=@_;open my$f,'-|','/usr/bin/git','-C',$ROOT,'show',"$BASE:$p" or die"git $p: $!\n";
    binmode$f;local$/;my$b=<$f>;close$f or die"git show $p failed: $?\n";return$b;
}
my$m=raw("$CHECK/source-before.sha256");
die"desired freeze manifest pin\n"unless sha256_hex($m)eq'e34cdb687a9e7195e686f38bf53a6f55c1b38f7f1769f60605ba36361b6119b7';
die"desired changed during preliminary validation\n"unless raw("$CHECK/source-after.sha256")eq$m;
my$pm=raw("$CHECK/producer-before.sha256");
die"producer record pin\n"unless sha256_hex($pm)eq'180a4bf01f9f8b8b141604133ca790c46b91d9041f1d888f68e0c582beb2cc84'
    &&raw("$CHECK/producer-after.sha256")eq$pm;
my%pins;for my$l(split/\n/,$m){die"manifest syntax\n"unless$l=~/\A([0-9a-f]{64})  ((?:src|library|tests)\/[A-Za-z0-9_.\/-]+)\z/;die"duplicate manifest path\n"if exists$pins{$2};$pins{$2}=$1;}
die"freeze population\n"unless keys(%pins)==1743;
my(%bytes,@files);
for my$p(@CORE,@TEST){
    die"unfrozen path $p\n"unless exists$pins{$p};my$b=raw("$DESIRED/$p");
    die"desired changed $p\n"unless sha256_hex($b)eq$pins{$p};$bytes{$p}=$b;
    push@files,{path=>$p,bytes=>length($b),sha256=>$pins{$p},stage=>(grep{$_ eq$p}@CORE)?'core-replay':'explicit-test-request'};
}
die"final private assertion pin\n"unless$pins{'src/elaborate/definition_types.lkj'}eq'35aede47461e4b7204ae257d17bdfcd3476dbfb0e8335893818b584a665b418b';
die"final public module pin\n"unless$pins{'src/semantic_tests/reference_capture.lkj'}eq'694b8163ad7a1216b22941187b0f740b5f46d46c37500f84017b6cb9742091c5';
my@baseline;
for my$p(@CORE,'src/semantic_tests.lkj','src/semantic_main.lkj'){
    my$b=git_blob($p);die"current source not baseline $p\n"unless raw("$ROOT/$p")eq$b;
    push@baseline,{path=>$p,bytes=>length($b),sha256=>sha256_hex($b)};
}
$!=0;my@new=lstat("$ROOT/src/semantic_tests/reference_capture.lkj");my$errno=0+$!;
die"new module path is not absent\n"unless!@new&&$errno==ENOENT;
my$base_tests=git_blob('src/semantic_tests.lkj');my$want_tests=$base_tests;
die"module insertion anchor\n"unless$want_tests=~s/^mod library_demand;\n/mod library_demand;\nmod reference_capture;\n/m==1;
die"core run anchor\n"unless$want_tests=~s/^    count \+= library_demand::run\(\);\n/    count += library_demand::run();\n    count += reference_capture::run();\n/m==1;
die"additional semantic_tests root change\n"unless$bytes{'src/semantic_tests.lkj'}eq$want_tests;
my($body)=$want_tests=~/^pub fn core\(\) -> usize (\{.*?^\})/ms;
die"core body extraction\n"unless defined($body)&&$body=~/reference_capture::run\(\)/;
my$base_main=git_blob('src/semantic_main.lkj');my$want_main=$base_main;
die"closed_core anchor\n"unless$want_main=~s/^pub fn closed_core\(\) \{ assert_eq!\(semantic_tests::core\(\), 185usize\); \}/pub fn closed_core() { assert_eq!(semantic_tests::core(), 192usize); }/m==1;
die"additional semantic_main root change\n"unless$bytes{'src/semantic_main.lkj'}eq$want_main;
my$module=decode('UTF-8',$bytes{'src/semantic_tests/reference_capture.lkj'},FB_CROAK|LEAVE_SRC);
my$core=decode('UTF-8',$body,FB_CROAK|LEAVE_SRC);
my$request={version=>1,operations=>[
    {op=>'add-item',id=>'semantic_tests',before=>'semantic_tests::optimizer',
     source=>'mod reference_capture;',projection=>{path=>'src/semantic_tests/reference_capture.lkj',source=>$module}},
    {op=>'replace-body',id=>'semantic_tests::core',source=>$core},
    {op=>'replace-body',id=>'closed_core',source=>'{ assert_eq!(semantic_tests::core(), 192usize); }'}
]};
my$rb=$J->encode($request);
die"projection UTF-8 round trip\n"unless encode('UTF-8',decode_json($rb)->{operations}[0]{projection}{source})eq$bytes{'src/semantic_tests/reference_capture.lkj'};
my$key='/home/coder/.local/state/lkjscript/store-key'; # path only, never opened here
my$producer="$ROOT/target/lkjscript";
my@core_argv=($producer,'edit','src/main.lkj','--source-dir',$ROOT,'--library-dir',"$ROOT/library",
    '--against','src/main.lkj','--against-source-dir',$DESIRED,'--against-library-dir',"$DESIRED/library",'--store-key',$key);
my@test_argv=($producer,'edit','src/semantic_main.lkj','--source-dir',$ROOT,'--library-dir',"$ROOT/library",
    '--request','target/compiler-reference-capture-integration-preparation-1/tests-request.json','--store-key',$key);
my@private=qw(elaborate::assert_reference_capture elaborate::definition_types::assert_reference_capture_recording elaborate::definition_types::assert_reference_capture_deferred);
my$freeze={schema=>1,status=>'frozen desired; requests prepared only; no compiler or publication',
 base_commit=>$BASE,desired_root=>$DESIRED,source_manifest=>{records=>1743,sha256=>sha256_hex($m),copied_as=>'desired-source.sha256'},
 producer_record=>{sha256=>sha256_hex($pm),copied_as=>'producer.sha256'},
 producer=>{path=>$producer,sha256=>'6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc',bytes=>32584542},
 desired_changed_files=>\@files,baseline_changed_files=>\@baseline,new_path_before=>'absent',
 private_assertion_items=>\@private,test_request=>{operations=>3,sha256=>sha256_hex($rb),projection_bytes=>length($bytes{'src/semantic_tests/reference_capture.lkj'}),
 projection_sha256=>$pins{'src/semantic_tests/reference_capture.lkj'},core_body_sha256=>sha256_hex($body)},
 key_handling=>'existing external path from prior accepted operation argv; no contents read, generated, replaced or removed',
 preliminary_scope=>'root-reported canonical-emitter closed-report 728 contracts, zero failures; this preparation does not rerun or independently relabel that observation'};
my$commands={schema=>1,status=>'prospective only; root release required',cwd=>$ROOT,
 limits=>{address_space_bytes=>4294967296,timeout_seconds=>900,kill_after_seconds=>10,compiler_workers=>1},
 stages=>[
  {name=>'core-replay',selected_root=>'src/main.lkj',expected_changed_files=>\@CORE,argv=>\@core_argv,
   optional_dry_run_argv=>[@core_argv,'--dry-run'],note=>'No --request. Actual emitted operation count and file facts are acceptance evidence, not guessed here.'},
  {name=>'explicit-test-root',selected_root=>'src/semantic_main.lkj',expected_changed_files=>\@TEST,argv=>\@test_argv,
   optional_dry_run_argv=>[@test_argv,'--dry-run'],request_sha256=>sha256_hex($rb),
   note=>'Only after core apply is accepted. No --against on the test root.'}
 ],dry_run_policy=>'Optional arrays are for root review, not an authorization or requirement to launch extra commands.',
 no_source_apply=>JSON::PP::true};
# Check every source used to create this request again before publishing proposal files.
for my$p(@CORE,@TEST){die"desired changed while preparing $p\n"unless raw("$DESIRED/$p")eq$bytes{$p};}
save('desired-source.sha256',$m);save('producer.sha256',$pm);save('tests-request.json',$rb);
save('freeze.json',$J->encode($freeze));save('commands.json',$J->encode($commands));
print"request_operations=3\nrequest_sha256=".sha256_hex($rb)."\n";
print"frozen_changed_files=".scalar(@files)." desired_manifest_records=1743\n";
for my$f(@files){print"$f->{sha256}  $f->{path}\n";}
