#!/usr/bin/env perl
use strict;use warnings;no warnings 'once';
use Digest::SHA qw(sha256_hex);use Fcntl qw(O_RDONLY O_NOFOLLOW S_ISREG);
use JSON::PP;use Errno qw(ENOENT);use Time::HiRes ();use IO::Handle;
my$main='/home/coder/workspace/lkjscript';
my$bundle='bench/compiler-completed-evidence-closure-data';
die "usage: absolute guard.pl --execute EXACT_RELEASE_COMMIT\n"
 unless @ARGV==2 && $ARGV[0]eq'--execute' && $ARGV[1]=~/\A[0-9a-f]{40}\z/;
my$release=$ARGV[1];
die "guard must run from its retained main path\n"unless$0 eq"$main/$bundle/guard.pl";
sub boot_bytes {
 my($p)=@_;sysopen(my$f,$p,O_RDONLY|O_NOFOLLOW)or die"open retained $p\n";binmode$f;my@s=stat$f;
 die"retained file type/link\n"unless@s&&S_ISREG($s[2])&&$s[3]==1;local$/;my$b=<$f>;close$f or die"retained close\n";return$b//'';
}
for my$name(qw(guard.pl closure-lib.pl manifest.json)){
 open(my$f,'-|','git','-C',$main,'cat-file','blob',"$release:$bundle/$name")or die"retained Git open\n";
 binmode$f;local$/;my$b=<$f>;close$f or die"retained Git failed\n";
 die"retained code/manifest differs $name\n"unless($b//'')eq boot_bytes("$main/$bundle/$name");
}
require "$main/$bundle/closure-lib.pl";
$CompilerClosure::RELEASE=$release;
my$result=$CompilerClosure::RESULT;
die "wrapper result claim missing\n"unless -d$result;
die "release receipt differs\n"unless CompilerClosure::bytes("$result/release.commit")eq"$release\n";
my$manifest=decode_json(CompilerClosure::git_bytes($release,"$bundle/manifest.json"));
open(my$head,'-|','git','-C',$main,'rev-parse','HEAD')or die"HEAD open\n";my$actual=<$head>;close$head or die"HEAD read\n";chomp$actual;
die"HEAD is not exact release\n"unless$actual eq$release;
my@expected_roots=(map{{path=>$_,kind=>'d',group=>'old'}}@CompilerClosure::OLD_ROOTS);
push@expected_roots,map{{path=>$_->{path},kind=>$_->{kind},group=>'mirror'}}@CompilerClosure::MIRRORS;
die"literal root manifest differs\n"unless CompilerClosure::json($manifest->{roots})eqCompilerClosure::json(\@expected_roots);
die"self mirror differs\n"unless CompilerClosure::json($manifest->{self_mirror})eq CompilerClosure::json({path=>$CompilerClosure::PREP,kind=>'d',committed_prefix=>$bundle});
die"declared old/disposable totals differ\n"unless
 $manifest->{counts}{old_files}==2175&&$manifest->{counts}{old_directories}==201&&$manifest->{counts}{old_logical_bytes}==480049506&&
 $manifest->{counts}{disposable_files}==34&&$manifest->{counts}{disposable_bytes}==141239847;
my$events="$result/events.jsonl";
sysopen(my$log,$events,Fcntl::O_WRONLY()|Fcntl::O_CREAT()|Fcntl::O_EXCL()|O_NOFOLLOW,0600)or die"claim events\n";
$log->autoflush(1);
sub event {my($r)=@_;print{$log} CompilerClosure::json($r),"\n"or die"event write\n";}
my($deleted_files,$deleted_dirs,$deleted_bytes)=(0,0,0);
my$phase='preflight';
my$outcome;my$error;
eval {
 my@roots=(@expected_roots,{path=>$CompilerClosure::PREP,kind=>'d',group=>'self'});
 my$live=CompilerClosure::scan(\@roots);
 my(%files,%dirs);
 for my$f(@{$manifest->{files}}){die"duplicate file record\n"if exists$files{$f->{path}};$files{$f->{path}}=$f;}
 for my$d(@{$manifest->{directories}}){die"duplicate directory record\n"if exists$dirs{$d->{path}};$dirs{$d->{path}}=$d->{metadata};}
 my%self_expected_dirs=($CompilerClosure::PREP=>1);
 for my$git_path(CompilerClosure::release_tree($bundle)){
  my$rel=substr($git_path,length($bundle)+1);my$p="$CompilerClosure::PREP/$rel";
  die"self mirror file absent $p\n"unless exists$live->{files}{$p};
  my$bytes=CompilerClosure::git_bytes($release,$git_path);
  $files{$p}={path=>$p,metadata=>$live->{files}{$p},bytes=>length($bytes),sha256=>sha256_hex($bytes),recovery=>{kind=>'blob',revision=>'release',path=>$git_path}};
  for(my$d=File::Basename::dirname($p);;$d=File::Basename::dirname($d)){$self_expected_dirs{$d}=1;last if$d eq$CompilerClosure::PREP;}
 }
 for my$d(keys%self_expected_dirs){die"self mirror directory absent\n"unless exists$live->{dirs}{$d};$dirs{$d}=$live->{dirs}{$d};}
 die"complete live file set differs\n"unless join("\0",sort keys%files)eq join("\0",sort keys%{$live->{files}});
 die"complete live directory set differs\n"unless join("\0",sort keys%dirs)eq join("\0",sort keys%{$live->{dirs}});
 for my$p(keys%dirs){CompilerClosure::same_metadata($live->{dirs}{$p},$dirs{$p},$p);}
 for my$p(keys%{$manifest->{parents}}){die"missing parent\n"unless exists$live->{parents}{$p};CompilerClosure::same_identity($live->{parents}{$p},$manifest->{parents}{$p},$p);}
 my($verified,$bytes,$disp,$dispbytes)=(0,0,0,0);
 for my$p(sort keys%files){my$f=$files{$p};
  CompilerClosure::same_metadata($live->{files}{$p},$f->{metadata},$p);
  $bytes+=CompilerClosure::verify_record($f,$live->{parents});$verified++;
  if($f->{recovery}{kind}eq'disposable'){
   die"unknown disposable classification\n"unless$f->{recovery}{class}=~/\Areviewed_(?:native_preparation_intermediate|generated_cohort_image)\z/;
   $disp++;$dispbytes+=$f->{bytes};
  }
 }
 die"actual disposable totals differ\n"unless$disp==34&&$dispbytes==141239847;
 my$again=CompilerClosure::scan(\@roots);
 for my$kind(qw(files dirs)){
  die"paths changed during recovery checks\n"unless join("\0",sort keys%{$live->{$kind}})eq join("\0",sort keys%{$again->{$kind}});
  for my$p(keys%{$live->{$kind}}){CompilerClosure::same_metadata($again->{$kind}{$p},$live->{$kind}{$p},$p);}
 }
 CompilerClosure::save("$result/preflight.json",CompilerClosure::pretty({release_commit=>$release,all_recoveries_verified=>JSON::PP::true,
  files=>$verified,directories=>scalar(keys%dirs),logical_bytes=>$bytes,disposable_files=>$disp,disposable_bytes=>$dispbytes,
  self_mirror_metadata=>{files=>[map{$files{$_}}sort grep{index($_,"$CompilerClosure::PREP/")==0}keys%files],
   directories=>[map{{path=>$_,metadata=>$dirs{$_}}}sort keys%self_expected_dirs]}}));
 event({stage=>'all_preflight_complete',files=>$verified,directories=>scalar(keys%dirs),logical_bytes=>$bytes});
 $phase='unlink';
 for my$p(sort keys%files){
  my$f=$files{$p};CompilerClosure::check_parents($p,$live->{parents});
  my($fh)=CompilerClosure::open_file($p,$f->{metadata});local$/;my$data=<$fh>;$data=''unless defined$data;
  die"file changed before unlink $p\n"unless length($data)==$f->{bytes}&&sha256_hex($data)eq$f->{sha256};
  CompilerClosure::same_metadata(CompilerClosure::metadata($p,$fh),$f->{metadata},$p);
  CompilerClosure::same_metadata(CompilerClosure::metadata($p),$f->{metadata},$p);CompilerClosure::check_parents($p,$live->{parents});
  event({stage=>'before_unlink',path=>$p,sha256=>$f->{sha256},bytes=>$f->{bytes}});
  unlink($p)==1 or die"unlink $p: $!\n";$deleted_files++;$deleted_bytes+=$f->{bytes};my@after=Time::HiRes::stat($fh);
  die"unlinked inode still linked $p\n"unless@after&&$after[3]==0;close($fh)or die"close removed $p\n";
  event({stage=>'unlinked',path=>$p});
 }
 $phase='rmdir';
 for my$p(sort{length($b)<=>length($a)||$b cmp$a}keys%dirs){
  CompilerClosure::check_parents($p,$live->{parents});CompilerClosure::same_identity(CompilerClosure::metadata($p),$dirs{$p},$p);
  sysopen(my$pin,$p,O_RDONLY|Fcntl::O_DIRECTORY()|O_NOFOLLOW)or die"open directory before rmdir\n";
  CompilerClosure::same_identity(CompilerClosure::metadata($p,$pin),$dirs{$p},$p);
  opendir(my$d,$p)or die"opendir before rmdir\n";my@entries=grep{$_ ne'.'&&$_ ne'..'}readdir$d;closedir$d or die"closedir\n";
  die"directory not empty $p\n"if@entries;CompilerClosure::check_parents($p,$live->{parents});
  CompilerClosure::same_identity(CompilerClosure::metadata($p),$dirs{$p},$p);
  event({stage=>'before_rmdir',path=>$p});rmdir($p)or die"rmdir $p: $!\n";$deleted_dirs++;close($pin)or die"directory pin close\n";
  event({stage=>'removed_directory',path=>$p});
 }
 $phase='absence';
 my@absent;
 for my$r(@roots){my@s=Time::HiRes::lstat($r->{path});die"root remains $r->{path}\n"if@s;die"root error is not ENOENT $r->{path}: $!\n"unless$!==ENOENT;push@absent,$r->{path};}
 $outcome={success=>JSON::PP::true,release_commit=>$release,absent_roots=>\@absent,deleted_files=>$deleted_files,deleted_directories=>$deleted_dirs,deleted_logical_bytes=>$deleted_bytes};
 event({stage=>'complete',%$outcome});1;
}or$error=$@||'unknown guard failure';
if(defined$error){$outcome={success=>JSON::PP::false,release_commit=>$release,phase=>$phase,error=>$error,
 deleted_files=>$deleted_files,deleted_directories=>$deleted_dirs,deleted_logical_bytes=>$deleted_bytes};eval{event({stage=>'stopped',%$outcome});};}
CompilerClosure::save("$result/result.json",CompilerClosure::pretty($outcome));
close($log)or die"event close\n";
print CompilerClosure::pretty($outcome);
exit(defined($error)?1:0);
