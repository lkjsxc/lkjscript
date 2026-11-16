use strict;
use warnings;
use File::Find;
use Fcntl qw(:mode :DEFAULT);
use Digest::SHA qw(sha256_hex);
use JSON::PP;
my $root='/home/coder/workspace/lkjscript';
my $out="$root/target/compiler-scalar-cohort-retention-proposal-1";
my @scopes=('target/compiler-scalar-simplify.20261116-1','target/compiler-scalar-trial-capture.20261116-1');
my $cohort=$scopes[0];my $outer=$scopes[1];
my $source_sha='cf3cd4568e4b7581aefff46cc92f93a6f145d861f46481df76d1e861fd0d0c64';
my $seed_sha='8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132';
my $seed_gz_sha='85b4e70de34dfa759aaf3cfb7e1009ba6b49b57499e01e4a3d49ae9182a6193c';
my $emitter_sha='f85624e3cc694f6a3a3e10c33f822789ae6084e2d5a5b41c3b1fc8fcc9bd2d64';
my $emitter_gz='bench/compiler-scalar-simplify-data/preparation/artifacts/'.$emitter_sha.'.compiler.gz';
my $source_recovery='artifacts/'.$source_sha.'.source.tar.gz';
my %producer=(
 'fixture/producer-A'=>['c7b8d970ace8f836a559f21fd7d95867fd718b9e95160d686442380afa330db1',22937783,'A'],
 'fixture/producer-B'=>['12b1f645c3ae4085c998cde9b6d3c920bc6c4ca547839654586c196e9aaaad11',22929591,'B']
);
my %seed_paths=map {$_=>1} ('fixture/expected',map {"fixture/output-$_"} qw(A1 B1 B2 A2 A3 B3));
my @rows;my @scope_rows;my %class;my %seen;my($count,$bytes)=(0,0);
my($source_count,$seed_count,$emitter_count,$producer_count,$archive_count)=(0,0,0,0,0);
sub save {my($p,$b)=@_;sysopen my$f,$p,O_WRONLY|O_CREAT|O_EXCL,0600 or die "$p: $!";binmode$f;print{$f}$b or die "$p: $!";close$f or die "$p close: $!";}
for my $scope(@scopes){
 my($files,$size,$dirs)=(0,0,0);my $base="$root/$scope";my@top=lstat($base);
 die "not a literal real scope: $base\n" unless @top&&S_ISDIR($top[2]);
 find({no_chdir=>1,follow=>0,wanted=>sub {
  my$p=$File::Find::name;my@s=lstat($p);die"lstat $p\n"unless@s;
  my$rel=substr($p,length($base)+1);my$full=substr($p,length($root)+1);
  if(S_ISDIR($s[2])){
   if($p ne $base && index($rel,'fixture/source/')!=0 && $rel =~ m{(?:^|/)(?:keys?|store|cold-store|private-keys?)$}){die"excluded key/store directory encountered: $full\n";}
   $dirs++;return;
  }
  die"nonregular file: $full\n"unless S_ISREG($s[2]);
  die"key path encountered: $full\n"if $rel =~ m{(?:^|/)(?:store-key|private-key|[^/]+\.key)$};
  die"unsafe inventory name\n"if $full =~ /[\n\r\t\0]/;
  $count++;$bytes+=$s[7];die"bounded inventory exceeded\n"if$count>3000||$bytes>536870912;
  sysopen my$f,$p,O_RDONLY|O_NOFOLLOW or die"$p: $!";my@opened=stat($f);
  die"changed file identity: $full\n"unless@opened&&S_ISREG($opened[2])&&$opened[0]==$s[0]&&$opened[1]==$s[1]&&$opened[7]==$s[7];
  my$d=Digest::SHA->new(256);my$first='';my$readbytes=0;
  while(1){my$n=read($f,my$b,65536);die"read $p: $!"unless defined$n;last unless$n;$first=substr($b,0,4)unless$readbytes;$d->add($b);$readbytes+=$n;}
  close$f or die"close $p: $!";die"size changed: $full\n"unless$readbytes==$s[7];
  my$sha=$d->hexdigest;
  my($kind,$recovery,$disposition);
  if($scope eq $cohort && $rel eq 'fixture/source.tar'){
   die"frozen source archive differs\n"unless$sha eq$source_sha;
   ($kind,$recovery,$disposition)=('retain-source-archive',$source_recovery,'retain exact original via gzip -n -9 after release');$archive_count++;
  }elsif($scope eq $cohort && index($rel,'fixture/source/')==0){
   my$member=substr($rel,length('fixture/source/'));
   ($kind,$recovery,$disposition)=('source-tree-duplicate',$source_recovery.'::'.$member,'disposable only after full source archive member verification and committed recovery');
   if($member eq 'seed/lkjscript.gz'){die"frozen embedded seed differs\n"unless$sha eq$seed_gz_sha;}
   $source_count++;
  }elsif($scope eq $cohort && exists$producer{$rel}){
   my$e=$producer{$rel};die"producer pin mismatch: $rel\n"unless$sha eq$e->[0]&&$s[7]==$e->[1]&&$first eq"\x7fELF";
   ($kind,$recovery,$disposition)=('retain-producer-'.$e->[2],'artifacts/'.$sha.'.producer-'.$e->[2].'.gz','retain actual executed native bytes via gzip -n -9 after release');$producer_count++;
  }elsif($scope eq $cohort && exists$seed_paths{$rel}){
   die"output/expected seed mismatch: $rel\n"unless$sha eq$seed_sha&&$s[7]==16228028&&$first eq"\x7fELF";
   ($kind,$recovery,$disposition)=('seed-equal-output-duplicate',$source_recovery.'::seed/lkjscript.gz -> gunzip -> '.$seed_sha,'disposable only after accepted byte-equality proof and committed source archive/embedded seed recovery');$seed_count++;
  }elsif($sha eq$emitter_sha){
   die"copied emitter shape/size\n"unless$scope eq$cohort&&$s[7]==32761734&&$first eq"\x7fELF";
   ($kind,$recovery,$disposition)=('existing-emitter-duplicate',$emitter_gz,'disposable only after committed existing gzip pin/decompression verification; do not duplicate or remove existing artifact');$emitter_count++;
  }else{
   die"unclassified native binary: $full\n"if$first eq"\x7fELF";
   ($kind,$recovery,$disposition)=('retain-raw-record','raw-cohort-records.tar.gz::'.$full,'retain exact bytes and original member path; preserve failure and timing records');
  }
  push@rows,{path=>$full,scope=>$scope,relative=>$rel,bytes=>$s[7],sha256=>$sha,mode=>sprintf('%04o',$s[2]&07777),dev=>$s[0],ino=>$s[1],mtime=>$s[9],classification=>$kind,recovery=>$recovery,disposition=>$disposition};
  $class{$kind}{files}++;$class{$kind}{bytes}+=$s[7];$files++;$size+=$s[7];
  die"duplicate path\n"if$seen{$full}++;
 }},$base);
 push@scope_rows,{path=>$scope,files=>$files,bytes=>$size,directories_including_root=>$dirs};
}
die"unexpected fixed cohort population\n"unless$source_count==172&&$seed_count==7&&$emitter_count==1&&$producer_count==2&&$archive_count==1;
@rows=sort{$a->{path}cmp$b->{path}}@rows;
my$json=JSON::PP->new->canonical->pretty;
my$proof={acceptance_directory=>'target/compiler-scalar-cohort-acceptance-1',
 acceptance_sha256=>'87667ea276c690cd43bc985ac0d439b6ae6aa3d8ceadcbd170213dcfff457f59',
 report_sha256=>'16a3da66cb239ffb4d3ae266a420e99bc637547bf9777cd1b4d12749d4f0c1bf',
 source_comparisons_sha256=>'9e033a650b89560633204a16d7f0025a324aadc79235d181c567f66018ba2f6b',
 manifest_sha256=>'50fc48af70fb957ef7040bb28a55e0f038dee6195a1295554ff2400fcc34ca59',
 source=>'pins supplied by independent reviewer; not reread or modified by this inventory'};
my$inventory={schema=>1,status=>'prospective packaging/closure; not a cleanup release',root=>$root,
 total_files=>$count,total_bytes=>$bytes,scopes=>\@scope_rows,classes=>\%class,files=>\@rows,
 independent_acceptance=>$proof,limits=>{files=>3000,logical_bytes=>536870912},
 excluded=>['keys','stores','all paths outside the two scopes except this new proposal directory'],
 no_compiler_or_native=>JSON::PP::true,no_compression=>JSON::PP::true,no_deletion=>JSON::PP::true};
save("$out/inventory.json",$json->encode($inventory));
save("$out/inventory.sha256",join('',map{"$_->{sha256}  $_->{path}\n"}@rows));
save("$out/recovery-map.tsv","path\tbytes\tsha256\tclassification\trecovery\tdisposition\n".join('',map{join("\t",@{$_}{qw(path bytes sha256 classification recovery disposition)})."\n"}@rows));
my@raw=grep{$_->{classification}eq'retain-raw-record'}@rows;
save("$out/raw-members.nul",join('',map{"$_->{path}\0"}@raw));
save("$out/raw-members.sha256",join('',map{"$_->{sha256}  $_->{path}\n"}@raw));
my@future=grep{$_->{classification}=~/\Aretain-(?:source-archive|producer-[AB])\z/}@rows;
save("$out/later-artifacts.json",$json->encode({new_gzips=>[map{{source=>$_->{path},original_sha256=>$_->{sha256},original_bytes=>$_->{bytes},artifact=>$_->{recovery}}}@future],
 existing_emitter_gzip=>{path=>$emitter_gz,sha256=>'ec4d686eb3a296c97757c479ce308a7c6388f3da8d30dca0079af9cb5684e7c2',bytes=>6582463,original_sha256=>$emitter_sha,original_bytes=>32761734,identity_source=>'already verified preparation package; reverify committed recovery before cleanup'},
 shared_output_recovery=>{original_sha256=>$seed_sha,original_bytes=>16228028,paths=>[map{$_->{path}}grep{$_->{classification}eq'seed-equal-output-duplicate'}@rows],artifact=>$source_recovery,member=>'seed/lkjscript.gz',member_sha256=>$seed_gz_sha},
 acceptance=>$proof}));
my$doc="# Scalar cohort retention and later closure proposal\n\n";
$doc.="Prospective only. The measured scalar gain was 0.702679%, below the fixed 10% rule; the experimental production candidate is discarded with no main adoption. Retention preserves this miss and every original capture. No compression, cleanup, compiler/native run, benchmark or source change occurs in this preparation.\n\n";
$doc.="## Exact scopes\n\nOnly the following two frozen main-worktree directories were inventoried. Every regular file has a named recovery and disposition in recovery-map.tsv; no symlink or special file was accepted.\n\n| Scope | Files | Logical bytes |\n| --- | ---: | ---: |\n";
for(@scope_rows){$doc.="| $_->{path} | $_->{files} | $_->{bytes} |\n";}
$doc.="| Total | $count | $bytes |\n\n";
$doc.="## Retention classes\n\n| Class | Files | Bytes |\n| --- | ---: | ---: |\n";
for my$k(sort keys%class){$doc.="| $k | $class{$k}{files} | $class{$k}{bytes} |\n";}
$doc.="\nRetain all raw cohort and outer capture records with original member paths in one deterministic raw-cohort-records.tar.gz made only from raw-members.nul. Keep the exact per-member hashes, full argv/environment metadata, time/load/status/stdout/stderr, inventories, original pin and result records. Do not rewrite a record because the threshold was missed.\n\n";
$doc.="Retain the two actual A/B native binaries as separate content-addressed gzip -n -9 artifacts. Retain exact fixture/source.tar ($source_sha) as $source_recovery, without regenerating it from a different checkout. The archive's 172 extracted files are duplicate recovery targets, not a second source archive to retain. Verify every original archive member against this inventory before closure.\n\n";
$doc.="The copied E emitter maps to the already byte-verified existing preparation artifact $emitter_gz (gzip SHA ec4d686eb3a296c97757c479ce308a7c6388f3da8d30dca0079af9cb5684e7c2). Do not make another E gzip or remove that committed artifact. This proposal references its existing verification; it did not read another scope to reverify it.\n\n";
$doc.="All six outputs and fixture/expected are $seed_sha, 16228028 bytes each. Their one shared recoverable representation is the exact frozen seed/lkjscript.gz member ($seed_gz_sha) already inside the retained source archive. There is no need for seven output gzips or a second seed gzip. The independent acceptance proves their complete byte equality to the exact Git seed; this inventory additionally checks each current size/hash.\n\n";
$doc.="## Acceptance and boundaries\n\nIndependent acceptance is frozen at target/compiler-scalar-cohort-acceptance-1: acceptance.json $proof->{acceptance_sha256}; source-comparisons.json $proof->{source_comparisons_sha256}; 52-record manifest $proof->{manifest_sha256}. These are reviewer-supplied pins, not a new acceptance execution. Root retains that proof separately. Preserve all earlier auditor-only failures there.\n\n";
$doc.="The initial directory-read attempts received Coder HTTP 429 before any inventory or mutation. That is a preparation access limitation, not a cohort failure or incomplete result. Reads were paused until root explicitly released this one batched bounded inventory/proposal call.\n\n";
$doc.="## Future execution sequence, not released here\n\n1. Root commits and verifies this proposal plus accepted raw/proof evidence, then explicitly releases packaging.\n2. Produce the explicit-list raw archive and the three new gzip artifacts (A, B, exact source tar). Record original/compressed lengths and SHA-256; fully verify every raw member and decompress/byte-compare each new artifact. Verify the frozen source archive member mapping, including the shared seed representation. Preserve any failure without overwriting the originals.\n3. Verify the existing committed E gzip and its original identity, and ensure independent acceptance remains exactly pinned. Commit all recovery artifacts and inventories before considering cleanup.\n4. A later separately released guard must require exact two literal root identities, every current file name/type/length/hash and no additional leaf. It must reject missing/changed recovery or unknown files, remove only enumerated files and empty directories, then require ENOENT for those two scopes. No native rerun is part of cleanup.\n\n";
$doc.="No keys, stores, candidate/source-worktree cleanup, validation-package cleanup, preparation artifact deletion, main source, gate data or unrelated target path belongs to this closure. Original files remain present. Packaging and deletion are both pending explicit root release.\n";
save("$out/proposal.md",$doc);
print"proposal=$out\nfiles=$count bytes=$bytes\n";
for(@scope_rows){print"$_->{path}\tfiles=$_->{files}\tbytes=$_->{bytes}\n";}
for my$k(sort keys%class){print"$k\tfiles=$class{$k}{files}\tbytes=$class{$k}{bytes}\n";}
for my$r(grep{$_->{classification}ne'retain-raw-record'&&$_->{classification}ne'source-tree-duplicate'&&$_->{classification}ne'seed-equal-output-duplicate'}@rows){print"artifact\t$r->{path}\t$r->{sha256}\t$r->{bytes}\t$r->{recovery}\n";}
print"proposal_sha256=".sha256_hex($doc)."\n";
