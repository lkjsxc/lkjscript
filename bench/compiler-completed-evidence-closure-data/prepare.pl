#!/usr/bin/env perl
use strict;use warnings;no warnings 'once';
use FindBin qw($Bin);use Digest::SHA qw(sha256_hex);use JSON::PP;
require "$Bin/closure-lib.pl";
my($M,$O,$Q,$N,$P,$B)=($CompilerClosure::MAIN,$CompilerClosure::OWN,$CompilerClosure::QUERY,$CompilerClosure::NATIVE,$CompilerClosure::PREP,$CompilerClosure::BUNDLE);
die "preparation location\n"unless$Bin eq$P;
$CompilerClosure::PREPARING=1;
my@roots=(map {{path=>$_,kind=>'d',group=>'old'}} @CompilerClosure::OLD_ROOTS);
push@roots,map {{path=>$_->{path},kind=>$_->{kind},group=>'mirror'}} @CompilerClosure::MIRRORS;
my$snapshot=CompilerClosure::scan(\@roots);
my(%maps,%prior);
sub blob {my($p,$r)=@_;return{kind=>'blob',revision=>$r//'release',path=>$p};}
sub archive {my($p,$member,$sha)=@_;my$r={kind=>'archive',revision=>'release',path=>$p,member=>$member};$r->{archive_sha256}=$sha if defined$sha;return$r;}
sub add {my($p,$r,$sha,$bytes)=@_;die"duplicate map $p\n"if exists$maps{$p};$maps{$p}=$r;$prior{$p}={sha256=>$sha,bytes=>$bytes} if defined$sha;}
my$qbase='bench/compiler-query-temp-closure-data';
my$q=CompilerClosure::load_json("$M/$qbase/inventory-1/full-inventory.json");
my$qg=CompilerClosure::load_json("$M/$qbase/inventory-1/retention-gaps.json");
my%qg=map {$_->{path}=>$_} @$qg;
for my$f(@{$q->{files}}){
 my$r=$f->{recovery};my$mapped;
 if($r->{kind}eq'committed-git-archive'){$mapped={kind=>'archive',revision=>$r->{git_revision},path=>$r->{git_path},member=>$r->{member},archive_sha256=>$r->{archive_sha256}};}
 elsif($r->{kind}eq'committed-git-file'){$mapped=blob($r->{git_path},$r->{git_revision});}
 else{my$g=$qg{$f->{path}}or die"unknown query recovery $f->{path}\n";$mapped=blob("$qbase/inventory-1/$g->{proposal_copy}");}
 add("$Q/target/$f->{path}",$mapped,$f->{sha256},$f->{bytes});
}
my$nbase='bench/compiler-native-temp-closure-data';
my$nr=CompilerClosure::load_json("$M/$nbase/new-retention.json");my%nr=map {$_->{source}=>$_} @$nr;
my@rows=split/\n/,CompilerClosure::bytes("$M/$nbase/disposition.tsv");die"native header\n"unless shift(@rows)eq"class\tbytes\tsha256\treference\tpath";
my($nd,$nb)=(0,0);
for my$row(@rows){
 my($class,$bytes,$sha,$ref,$rel)=split/\t/,$row,-1;die"native row\n"unless defined$rel;
 my$p="$N/target/$rel";my$r;
 if($class eq'exact_committed_counterpart'){die"native committed reference\n"unless$ref=~/\A([0-9a-f]{40}):(.+)\z/;$r=blob($2,$1);}
 elsif($class eq'preparation_intermediate'){$r={kind=>'disposable',class=>'reviewed_native_preparation_intermediate'};$nd++;$nb+=$bytes;}
 else{my$copy=$nr{$p}or die"missing native retention $p\n";$r=blob("$nbase/$copy->{retained}");}
 add($p,$r,$sha,$bytes);
}
die"native disposable set changed\n"unless$nd==32&&$nb==39832656;
my$original='bench/compiler-profile-copy-sites-cohort-1-refusal-data/diagnostic/compiler-profile-copy-sites.original.sh';
my%ranges=(INLINE_1=>[853,642],INLINE_2=>[12107,248],INLINE_3=>[16540,52],INLINE_4=>[16540,52],
 PERL_CAPTURE=>[17630,1833],PERL_INVENTORY=>[14594,607],PERL_PREFLIGHT=>[3644,8413],PERL_STATS=>[19850,1394]);
my%measurement=(
 'compiler-profile-copy-sites-measurement-plan.md'=>'bench/compiler-profile-copy-sites-measurement-plan.md',
 'compiler-profile-copy-sites-release.example.json'=>'bench/compiler-profile-copy-sites-release.example.json',
 'compiler-profile-copy-sites.sh'=>$original,
 'launcher-preparation.md'=>'bench/compiler-profile-copy-sites-preparation-data/launcher-preparation.md');
my%ordinary=(
 'accounting.pl'=>blob('bench/compiler-profile-copy-account.pl'),
 'owner-cwd'=>blob('bench/compiler-profile-copy-sites-cohort-2-data/cwd'),
 'owner-pid'=>blob('bench/compiler-profile-copy-sites-cohort-2-data/owner-pid'),
 'owner-repository'=>blob('bench/compiler-profile-copy-sites-cohort-2-data/owner-repository'),
 'owner-start'=>blob('bench/compiler-profile-copy-sites-cohort-2-data/owner-start'),
 'owner-evidence'=>archive('bench/compiler-profile-copy-sites-cohort-2-acceptance/raw-audit.tar.gz','./evidence.path','2e428b8a01bd00d8780c04c3ab22dbb1b6782b3091a591be63a9bb3f65813ac0'),
 'producer'=>{kind=>'gunzip',revision=>'7d0731e99aeaf241c4334fc5ff065a341fb388c8',path=>'seed/lkjscript.gz'},
 'producer-profile64.samples.00000000000002892325'=>{kind=>'gunzip',revision=>'release',path=>'bench/compiler-profile-copy-sites-cohort-2-data/artifacts/7849d4dd69906491d07664f493eb0f4fd8340cfde3760f6e4f168dbbde95f302.profile.lkjprof.gz'},
 'selected-native.lkjir'=>{kind=>'gunzip',revision=>'release',path=>'bench/compiler-profile-copy-sites-cohort-2-data/artifacts/32c0f43dbde125e43f2ca5a0c6d8548ba0f1774ce3b51bc62d94a6488af0fdca.selected.lkjir.gz'});
for my$n(qw(expected output-ordinary output-profile)){$ordinary{$n}={kind=>'gunzip',revision=>'26bf2c3c021dd79810a81fa1d9054d59f3768b06',path=>'seed/lkjscript.gz'};}
my%images=(
 'producer-ordinary64'=>['20cfe7aae7dddba36da8edca9b56e17f8983bed7e48eed965ce0a01d43b40856',22936514],
 'producer-profile64'=>['eb4ab30d00979292937e2f7e25ec61c1d43bbea49c8b1a86185d8b6a6799c95c',78470677]);
my%own_direct=(
 'compiler-copy-sites-preflight-diagnostic.kGyrqc'=>'bench/compiler-profile-copy-sites-cohort-1-refusal-data/diagnostic',
 'compiler-copy-sites-census-audit.20261116-1.fIdBSp'=>'bench/compiler-copy-sites-census-acceptance',
 'compiler-copy-sites-fn5719-review.20261116-1.bcni8c'=>'bench/compiler-copy-sites-census-fn5719-data');
my%own_archives=(
 'compiler-copy-sites-docs'=>['bench/compiler-copy-sites-data/proposals/docs.tar.gz','compiler-copy-sites-docs/','ce532246b87b69fc81af8b8edebfe4a55e3f18c298f8bdb806cd9189cdced926'],
 'compiler-copy-sites-ci.43872738366.M8M145'=>['bench/compiler-copy-sites-data/ci-43872738366/raw-ci.tar.gz','./','12197424fe9194642cae806776e8100498dd8c19d50470ce7784416c0b2f4ab5'],
 'compiler-copy-sites-acceptance.20261116-2.FFPpPx'=>['bench/compiler-profile-copy-sites-cohort-2-acceptance/raw-audit.tar.gz','./','2e428b8a01bd00d8780c04c3ab22dbb1b6782b3091a591be63a9bb3f65813ac0']);
my$copy=CompilerClosure::bytes("$O/target/compiler-copy-sites-closure-proposal.md");
if(-e"$P/eleven-scope-proposal.md"){die"proposal copy differs\n"unless CompilerClosure::bytes("$P/eleven-scope-proposal.md")eq$copy;}
else{CompilerClosure::save("$P/eleven-scope-proposal.md",$copy);}
for my$p(sort keys%{$snapshot->{files}}){
 next if exists$maps{$p};
 if($p=~m{\A\Q$M\E/target/compiler-copy-sites-evidence\.20261116-2/(.+)\z}){add($p,blob("bench/compiler-profile-copy-sites-cohort-2-data/$1"));next;}
 if($p=~m{\A\Q$M\E/target/(compiler-copy-sites-census-(?:output|evidence)\.20261116-1/.+)\z}){add($p,archive('bench/compiler-copy-sites-census-extraction-data/raw-extraction.tar.gz',$1,'babfaf4cbcaebb16fb146e142d96b98890907bb47a6f0abe576df8bcb9d1ab46'));next;}
 if($p=~m{\A\Q$M\E/target/compiler-copy-sites-run\.20261116-2/(.+)\z}){
  my$rel=$1;if($rel=~m{\Asource/(.+)\z}){add($p,blob($1,'26bf2c3c021dd79810a81fa1d9054d59f3768b06'));}
  elsif(exists$ordinary{$rel}){add($p,$ordinary{$rel});}
  elsif(exists$images{$rel}){add($p,{kind=>'disposable',class=>'reviewed_generated_cohort_image'},@{$images{$rel}});}
  else{die"unmapped run file $p\n";}next;
 }
 if($p=~m{\A\Q$O\E/target/([^/]+)/(.+)\z}){
  my($scope,$rel)=($1,$2);
  if(exists$own_direct{$scope}){add($p,blob("$own_direct{$scope}/$rel"));next;}
  if(exists$own_archives{$scope}){my($a,$prefix,$sha)=@{$own_archives{$scope}};add($p,archive($a,"$prefix$rel",$sha));next;}
  if($scope eq'compiler-copy-sites-measurement-plan'){
   if(exists$measurement{$rel}){add($p,blob($measurement{$rel}));}
   elsif($rel=~m{\A(?:static|static-v2)/([A-Z0-9_]+)\.pl\z}&&exists$ranges{$1}){my($offset,$length)=@{$ranges{$1}};add($p,{kind=>'range',revision=>'release',path=>$original,offset=>$offset,length=>$length,container_sha256=>'47d085bc0cf1b0a48be56fcfda8efcec5cba85162dcdf9031f5da5ba7e6140e6'});}
   else{die"unmapped helper $p\n";}next;
  }
 }
 my$found=0;for my$m(@CompilerClosure::MIRRORS){
  if($m->{kind}eq'f'&&$p eq$m->{path}){add($p,blob($m->{prefix}));$found=1;last;}
  if($m->{kind}eq'd'&&index($p,"$m->{path}/")==0){my$rel=substr($p,length($m->{path})+1);add($p,blob("$m->{prefix}/$rel"));$found=1;last;}
 }
 die"unmapped live file $p\n"unless$found;
}
die "mapping/live path set differs\n" unless join("\0",sort keys%maps)eq join("\0",sort keys%{$snapshot->{files}});
my(@files,$old_files,$old_bytes,$old_dirs,$disposable_files,$disposable_bytes);
($old_files,$old_bytes,$old_dirs,$disposable_files,$disposable_bytes)=(0,0,0,0,0);
sub in_old {my($p)=@_;for my$r(@CompilerClosure::OLD_ROOTS){return 1 if$p eq$r||index($p,"$r/")==0;}return 0;}
for my$p(keys%{$snapshot->{dirs}}){$old_dirs++ if in_old($p);}
for my$p(sort keys%maps){
 my$m=$snapshot->{files}{$p};my($data)=CompilerClosure::read_checked($p,$m);my$sha=sha256_hex($data);
 if(exists$prior{$p}){die"prior inventory differs $p\n"unless$sha eq$prior{$p}{sha256}&&length($data)==$prior{$p}{bytes};}
 my$record={path=>$p,metadata=>$m,bytes=>length($data),sha256=>$sha,recovery=>$maps{$p}};
 if($record->{recovery}{kind}eq'disposable'){$disposable_files++;$disposable_bytes+=length($data);}
 else{my$recovered=CompilerClosure::recovery_bytes($record->{recovery});die"recovery differs $p\n"unless$data eq$recovered;}
 push@files,$record;if(in_old($p)){$old_files++;$old_bytes+=length($data);}
}
die "old totals changed\n"unless$old_files==2175&&$old_dirs==201&&$old_bytes==480049506;
die "disposable totals changed\n"unless$disposable_files==34&&$disposable_bytes==141239847;
my$after=CompilerClosure::scan(\@roots);
for my$kind(qw(files dirs)){
 die"paths changed during preparation\n"unless join("\0",sort keys%{$snapshot->{$kind}})eq join("\0",sort keys%{$after->{$kind}});
 for my$p(keys%{$snapshot->{$kind}}){CompilerClosure::same_metadata($snapshot->{$kind}{$p},$after->{$kind}{$p},$p);}
}
my$manifest={schema=>'compiler-completed-evidence-closure-1',roots=>\@roots,files=>\@files,
 directories=>[map {{path=>$_,metadata=>$snapshot->{dirs}{$_}}} sort keys%{$snapshot->{dirs}}],
 parents=>$snapshot->{parents},
 counts=>{old_files=>$old_files,old_directories=>$old_dirs,old_logical_bytes=>$old_bytes,
  disposable_files=>$disposable_files,disposable_bytes=>$disposable_bytes,
  frozen_files=>scalar@files,frozen_directories=>scalar keys%{$snapshot->{dirs}}},
 self_mirror=>{path=>$P,kind=>'d',committed_prefix=>$B},
 pending_release_bundles=>['bench/compiler-query-temp-closure-data','bench/compiler-native-temp-closure-data',$B],
 no_removal_performed=>JSON::PP::true};
CompilerClosure::save("$P/manifest.json",CompilerClosure::pretty($manifest));
print CompilerClosure::pretty({status=>'prepared',counts=>$manifest->{counts},manifest_sha256=>sha256_hex(CompilerClosure::pretty($manifest)),recovery_verified=>'all current file bytes; pending bundles require final committed release'});
