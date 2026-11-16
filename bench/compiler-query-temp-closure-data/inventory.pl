#!/usr/bin/env perl
use strict;use warnings;
use Archive::Tar;
use Cwd qw(abs_path);
use Digest::SHA qw(sha256_hex);
use Fcntl qw(O_RDONLY O_WRONLY O_CREAT O_EXCL O_NOFOLLOW :mode);
use File::Find;
use File::Path qw(make_path);
use IO::Uncompress::Gunzip qw(gunzip $GunzipError);
use JSON::PP;
my $target='/home/coder/workspace/lkjscript-compiler-query-container-20261116/target';
my $main='/home/coder/workspace/lkjscript';
my $revision='61049544e4461e3365d239191a6cbe704407a85e';
my $out="$target/compiler-query-worktree-closure-proposal-20261116/inventory-1";
my @scopes=qw(
 compiler-copy-sites-profiler-candidate
 compiler-copy-sites-accounting-candidate
 compiler-copy-sites-extractor-candidate
 compiler-copy-sites-extractor-package-20261116
 compiler-copy-sites-census-join-20261116
 compiler-copy-sites-census-join-package-20261116
 compiler-copy-sites-family-census-20261116
 compiler-copy-sites-family-census-package-20261116
);
die "output already exists\n" if lstat($out);mkdir($out,0700)or die "claim output\n";
my $J=JSON::PP->new->canonical->utf8->pretty;
sub save{my($name,$bytes)=@_;die "unsafe output\n" unless $name=~/\A[A-Za-z0-9_.\/-]+\z/&&$name!~m{(?:\A|/)\.\.?(?:/|\z)};
 my $path="$out/$name";my $parent=$path;$parent=~s{/[^/]+$}{};make_path($parent,{mode=>0700}) unless -d $parent;
 sysopen(my $f,$path,O_WRONLY|O_CREAT|O_EXCL,0600)or die "write $path\n";binmode$f;print$f $bytes or die"write";close$f or die"close";
}
sub bytes{my($path)=@_;sysopen(my$f,$path,O_RDONLY|O_NOFOLLOW)or die"open $path\n";binmode$f;my @a=stat($f);
 die "nonregular\n" unless @a&&S_ISREG($a[2]);local$/;my$b=<$f>;$b//='';my@z=stat($f);close$f or die"close";
 for my$i(0,1,2,7,9,10){die"read changed\n"unless$a[$i]==$z[$i];}die"length\n"unless length($b)==$a[7];return$b;
}
sub git_blob{my($path)=@_;open(my$f,'-|','git','-C',$main,'show',"$revision:$path")or die"git open";binmode$f;local$/;my$b=<$f>;close$f or die"git show failed: $path\n";return$b//'';}
my(%recovery,@archives,@direct);
my %archive_pins=(
 'bench/compiler-copy-sites-data/proposals/profiler.tar.gz'=>'306377afa71b52f8121f6f6447460224be493750daf36c6e3aa10266320e17f8',
 'bench/compiler-profile-copy-sites-preparation-data/accounting-proposal.tar.gz'=>'ff8e03d6bdbc49d6c97f7fdb79f223e71e9bc619885df273d9e072db328e8e90',
 'bench/compiler-copy-sites-extractor-data/candidate.tar.gz'=>'39e748fc1364a2ab1a3120ff8f0372d2c0885faf083e8b21e5fc77e6a62f76ad',
 'bench/compiler-copy-sites-census-join-data/evidence.tar.gz'=>'2656c2abff9149705e482d78bef8f3aec4c21781bc8114ffe77b4f35841bb29a',
 'bench/compiler-copy-sites-census-family-data/raw-family-census.tar.gz'=>'6f354d1d4ce488343cdc99a5f9a070c6ff6309175416e015e128315dadfb1cee'
);
sub index_bytes{my($b,$record)=@_;my$key=length($b).':'.sha256_hex($b);push@{$recovery{$key}},$record;}
sub index_archive{my($raw,$record)=@_;my$unpacked='';gunzip \$raw=>\$unpacked or die"gunzip $GunzipError\n";
 open(my$fh,'<',\$unpacked)or die"scalar open";binmode$fh;my$tar=Archive::Tar->new;$tar->read($fh,0)or die"tar read\n";close$fh or die"tar close";
 my($files,$logical)=(0,0);my%seen;
 for my$f($tar->get_files){my$name=$f->full_path;die"unsafe archive path $name\n" if$name=~m{(?:\A/|(?:\A|/)\.\.(?:/|\z))};
  next if$f->is_dir;die"archive special entry\n"unless$f->is_file;die"duplicate archive name\n"if$seen{$name}++;
  my$b=$f->get_content;$files++;$logical+=length$b;index_bytes($b,{%$record,member=>$name});
 }
 push@archives,{%$record,archive_sha256=>sha256_hex($raw),archive_bytes=>"".length($raw),regular_files=>"$files",logical_bytes=>"$logical"};
}
for my$path(
 'bench/compiler-copy-sites-data/proposals/profiler.tar.gz',
 'bench/compiler-profile-copy-sites-preparation-data/accounting-proposal.tar.gz',
 'bench/compiler-copy-sites-extractor-data/candidate.tar.gz',
 'bench/compiler-copy-sites-census-join-data/evidence.tar.gz',
 'bench/compiler-copy-sites-census-family-data/raw-family-census.tar.gz'
){my$raw=git_blob($path);die "committed archive pin $path\n" unless sha256_hex($raw) eq $archive_pins{$path};my$r={kind=>'committed-git-archive',git_revision=>$revision,git_path=>$path,archive_sha256=>sha256_hex($raw)};
 index_archive($raw,$r);index_bytes($raw,{kind=>'committed-git-file',git_revision=>$revision,git_path=>$path});
 push@direct,{git_path=>$path,sha256=>sha256_hex($raw),bytes=>"".length($raw)};
}
for my$path(
 'bench/compiler-copy-sites-extractor-data/candidate-inventory.sha256',
 'bench/compiler-copy-sites-extractor-data/candidate-inventory.json',
 'bench/compiler-profile-copy-sites-preparation-data/accounting-validated.sha256',
 'bench/compiler-copy-sites-census-join-data/inventory.sha256',
 'bench/compiler-copy-sites-census-join-data/inventory.json',
 'bench/compiler-copy-sites-census-family-data/inventory.sha256',
 'bench/compiler-copy-sites-census-family-data/inventory.json'
){my$raw=git_blob($path);index_bytes($raw,{kind=>'committed-git-file',git_revision=>$revision,git_path=>$path});
 push@direct,{git_path=>$path,sha256=>sha256_hex($raw),bytes=>"".length($raw)};
}
my(@files,@directories,%by_scope,%disposition,@gaps);
for my$scope(@scopes){
 my$root="$target/$scope";die"wrong literal scope\n"unless abs_path($root)eq$root;my$sum={scope=>$root,files=>0,directories=>0,bytes=>0,dispositions=>{}};
 find({no_chdir=>1,follow=>0,wanted=>sub{
  my$path=$File::Find::name;die"scope escape\n"unless$path=~/\A\Q$root\E(?:\/[A-Za-z0-9_.-]+)*\z/;
  my@a=lstat($path);die"missing entry\n"unless@a;my$rel=substr($path,length($target)+1);
  if(S_ISDIR($a[2])){push@directories,{path=>$rel,mode=>sprintf('%o',$a[2]&07777),device=>"$a[0]",inode=>"$a[1]"};$sum->{directories}++;return;}
  die"nonregular or multiply-linked source $path\n"unless S_ISREG($a[2])&&$a[3]==1;
  my$content=bytes($path);my@again=lstat($path);die "entry disappeared\n" unless @again;
  for my$i(0,1,2,3,7,9,10){die "inventory metadata changed\n" unless $a[$i]==$again[$i];}
  my$sha=sha256_hex($content);my$key=length($content).":$sha";my@options=@{$recovery{$key}//[]};
  my%priority=('committed-git-file'=>0,'committed-git-archive'=>1,'pending-git-archive'=>2,'pending-git-archive-artifact'=>3);
  @options=sort{($priority{$a->{kind}}//9)<=>($priority{$b->{kind}}//9)||($a->{git_path}//$a->{owned_archive}//'')cmp($b->{git_path}//$b->{owned_archive}//'')||($a->{member}//'')cmp($b->{member}//'')}@options;
  my$r;
  if(@options){$r=$options[0];}
  else{my$gap="pending-retention/$rel";save($gap,$content);$r={kind=>'new-retention-required',proposal_copy=>$gap,release_condition=>'commit this exact retained copy before any removal'};push@gaps,{path=>$rel,bytes=>"".length($content),sha256=>$sha,proposal_copy=>$gap};}
  my$rec={path=>$rel,bytes=>"".length($content),sha256=>$sha,mode=>sprintf('%o',$a[2]&07777),device=>"$a[0]",inode=>"$a[1]",links=>"$a[3]",mtime=>"$a[9]",ctime=>"$a[10]",recovery=>$r};
  push@files,$rec;$sum->{files}++;$sum->{bytes}+=length$content;$sum->{dispositions}{$r->{kind}}++;$disposition{$r->{kind}}{files}++;$disposition{$r->{kind}}{bytes}+=length$content;
 }},$root);$by_scope{$scope}=$sum;
}
@files=sort{$a->{path}cmp$b->{path}}@files;@directories=sort{$a->{path}cmp$b->{path}}@directories;
my$bytes=0;$bytes+=$_->{bytes}for@files;
my$summary={schema=>'owned-query-worktree-closure-proposal-1',removal_performed=>JSON::PP::false,cleanup_authorized=>JSON::PP::false,
 git_recovery_revision=>$revision,target=>$target,scopes=>[map{$by_scope{$_}}@scopes],files=>"".scalar(@files),directories=>"".scalar(@directories),logical_bytes=>"$bytes",dispositions=>\%disposition,
 pending_retention_files=>"".scalar(@gaps),disposal_policy=>'Only byte-exact duplicate copies after every pending recovery is committed; no unique input is classified as disposable.',
 exclusions=>['all other target paths, especially compiler-container-* evidence and key locator','worktree source and durable .lkjscript-edit.* metadata','/tmp/compiler-container-key.m4u6F6 and every other credential','main and every other worktree','this closure proposal directory'],
 notes=>['Archive content was read, not extracted or executed.','This frozen metadata inventory is a proposal; any later cleanup must recheck all literal scopes, path sets and hashes.','No process/FD scan, compiler, profile, official reader, extractor or prior census was run.']};
save('full-inventory.json',$J->encode({%$summary,files=>\@files,directories=>\@directories}));
my$tsv="path\tbytes\tsha256\tmode\tdevice\tinode\tlinks\tmtime\tctime\trecovery_json\n";
my$compact=JSON::PP->new->canonical->utf8;
for my$r(@files){$tsv.=join("\t",@$r{qw(path bytes sha256 mode device inode links mtime ctime)},$compact->encode($r->{recovery}))."\n";}
save('full-inventory.tsv',$tsv);save('inventory.sha256',join('',map{"$_->{sha256}  $_->{path}\n"}@files));
save('recovery-archives.json',$J->encode(\@archives));save('recovery-direct-files.json',$J->encode(\@direct));save('retention-gaps.json',$J->encode(\@gaps));save('summary.json',$J->encode($summary));
print$J->encode($summary);
