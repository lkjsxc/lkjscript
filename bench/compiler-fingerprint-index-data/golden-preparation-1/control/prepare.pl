#!/usr/bin/perl
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use File::Path qw(make_path);
use File::Find;
use File::Copy qw(copy);
use IPC::Open2;
use Fcntl qw(:DEFAULT :mode);

my $repo='/home/coder/workspace/lkjscript';
my $root="$repo/target/compiler-fingerprint-index-golden-1";
my $source="$root/source";
my $commit='9326dbaab1bf6832b9f35dc156e12d0d62eccd9e';
my $producer='b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db';
my $incremental='760485a5a12d97ce320ba9113797370df584c154084ee4a1f9689da28c7d1127';
my $frozen="$repo/target/compiler-fingerprint-index-preparation-1";
my @controls=(
 ['golden-preparation-1/golden-main.txt','golden-main.txt','7132e34bfb2da3bcfe033b5b99800198b24baf5db807a8ca947df304ba886e85'],
 ['golden-preparation-1/incremental-fingerprint-module.txt','incremental-fingerprint-module.txt','5b5a7c739cb6c45c5fd88bf366b46a04c196c1ea88d58997484623a6dee1c7b5'],
 ['fixtures.json','fixtures.json','4533bf85ddb41fdca92aca085dd5f961e0edd62bf9299cd74ec060ed5bac7295'],
 ['golden-preparation-1/integration-proposal.json','integration-proposal.json','d979c680eb6afcc3d7ae369c1c3b2843920eb5420ce40c8d1155e9414eea43a9'],
 ['golden-preparation-1/README.md','golden-preparation-readme.md','25b3a85a8de5f2112ba73aaa9b7874e26b84791b4b3172835485ec29b4896ba4']
);
sub raw {
 my($p)=@_; my @s=lstat($p);
 die "not a regular single-link file: $p\n" unless @s && S_ISREG($s[2]) && $s[3]==1;
 open my $f,'<:raw',$p or die "$p: $!\n"; local $/; my $b=<$f>; close $f or die "$p close: $!\n"; return $b;
}
sub put {
 my($p,$b)=@_; sysopen my $f,$p,O_WRONLY|O_CREAT|O_EXCL,0600 or die "$p: $!\n";
 binmode $f; print {$f} $b or die "$p write: $!\n"; close $f or die "$p close: $!\n";
}
sub json { put($_[0],JSON::PP->new->canonical->pretty->encode($_[1])); }
sub command {
 my(@a)=@_; open my $f,'-|',@a or die "open command: $!\n"; binmode $f;
 local $/; my $b=<$f>; close $f or die "command failed: @a\n"; return $b;
}
sub read_exact {
 my($f,$n)=@_; my $b='';
 while(length($b)<$n) { my $got=read($f,my $part,$n-length($b)); die "short Git blob\n" unless defined($got) && $got>0; $b.=$part; }
 return $b;
}
die "source already exists\n" if lstat($source);
die "owned producer already exists\n" if lstat("$root/bin/compiler-baseline");
my %frozen_bytes;
for my $c(@controls) {
 my $b=raw("$frozen/$c->[0]"); die "frozen control changed: $c->[0]\n" unless sha256_hex($b) eq $c->[2];
 $frozen_bytes{$c->[1]}=$b;
}
my $fixture=JSON::PP->new->decode($frozen_bytes{'fixtures.json'});
die "wrong case population\n" unless @{$fixture->{cases}}==17;
die "wrong baseline fixture pin\n" unless $fixture->{baseline_producer_sha256} eq $producer && $fixture->{original_function_file_sha256} eq $incremental;
my $git_listing=command('git','-C',$repo,'ls-tree','-r','-l','-z',$commit,'--','src','library','tests');
my @entries;
for my $record(split /\0/,$git_listing) {
 die "unexpected Git tree entry\n" unless $record =~ /\A100644 blob ([0-9a-f]{40})\s+(\d+)\t((?:src|library|tests)\/[^\0\r\n\t]+)\z/;
 my($oid,$size,$path)=($1,0+$2,$3);
 die "unsafe Git path: $path\n" if $path =~ m{(?:\A|/)\.\.?(?:/|\z)};
 push @entries,{path=>$path,git_oid=>$oid,bytes=>$size,mode=>'100644'};
}
die "unexpected Git population\n" unless @entries==1743;
my @tree=split /\n/,command('git','-C',$repo,'rev-parse',"$commit:src","$commit:library","$commit:tests");
die "source tree mismatch\n" unless $tree[0] eq '626c9c1f234f4d003a08820d9b69840535eff8b8';
die "library tree mismatch\n" unless $tree[1] eq '25bdb8ff8567b941790d3951c78e896b23e26e8e';
die "tests tree mismatch\n" unless $tree[2] eq 'e2b30ac0f2fa8f897f366293189d6af552a41980';
my $canonical_before=raw("$repo/target/lkjscript");
die "wrong canonical bytes\n" unless length($canonical_before)==32588975 && sha256_hex($canonical_before) eq $producer;
mkdir $source,0700 or die "mkdir source: $!\n";
my($git_out,$git_in);
my $pid=open2($git_out,$git_in,'git','-C',$repo,'cat-file','--batch');
binmode $git_out; binmode $git_in;
my $old=select $git_in; $|=1; select $old;
my %by_path;
for my $e(@entries) {
 print {$git_in} "$e->{git_oid}\n" or die "Git request: $!\n";
 my $h=<$git_out>;
 die "Git blob header mismatch\n" unless defined($h) && $h eq "$e->{git_oid} blob $e->{bytes}\n";
 my $b=read_exact($git_out,$e->{bytes}); die "Git separator mismatch\n" unless read_exact($git_out,1) eq "\n";
 $e->{git_sha256}=sha256_hex($b);
 my $p="$source/$e->{path}"; (my $parent=$p)=~s{/[^/]+\z}{};
 make_path($parent,{mode=>0755}); put($p,$b); chmod 0644,$p or die "chmod: $!\n";
 die "copied Git bytes mismatch\n" unless raw($p) eq $b;
 $by_path{$e->{path}}=$e;
}
close $git_in or die "Git stdin close\n"; close $git_out or die "Git stdout close\n";
waitpid($pid,0)==$pid && $?==0 or die "Git batch failed\n";
die "wrong original fingerprint\n" unless $by_path{'src/elaborate/incremental.lkj'}->{git_sha256} eq $incremental;
put("$root/control/source.git.sha256",join('',map{"$_->{git_sha256}  $_->{path}\n"} sort{$a->{path}cmp$b->{path}}@entries));
for my $overlay(
 ['src/main.lkj','golden-main.txt'],
 ['src/fingerprint_fixture.lkj','incremental-fingerprint-module.txt']
) {
 my($path,$control)=@$overlay; my $p="$source/$path";
 if($path eq 'src/main.lkj') {
  die "main copy drift\n" unless sha256_hex(raw($p)) eq $by_path{$path}->{git_sha256};
  open my $f,'>:raw',$p or die "overlay main: $!\n"; print {$f} $frozen_bytes{$control} or die "overlay main write\n"; close $f or die "overlay close\n";
 } else { put($p,$frozen_bytes{$control}); chmod 0644,$p or die "overlay chmod\n"; }
}
my @final;
find({no_chdir=>1,wanted=>sub {
 my $p=$File::Find::name; my @s=lstat($p); die "source lstat: $p\n" unless @s;
 return if S_ISDIR($s[2]);
 die "source special: $p\n" unless S_ISREG($s[2]) && $s[3]==1;
 (my $rel=$p)=~s/^\Q$source\E\///;
 my $b=raw($p); my $sha=sha256_hex($b);
 my $overlay=$rel eq 'src/main.lkj' ? 'golden-main.txt' : $rel eq 'src/fingerprint_fixture.lkj' ? 'incremental-fingerprint-module.txt' : undef;
 die "unrepresented source: $rel\n" unless defined($overlay)||exists($by_path{$rel});
 die "non-overlay drift: $rel\n" if !defined($overlay) && $sha ne $by_path{$rel}->{git_sha256};
 die "overlay drift: $rel\n" if defined($overlay) && $b ne $frozen_bytes{$overlay};
 die "source mode drift: $rel\n" unless ($s[2]&0777)==0644;
 push @final,{path=>$rel,sha256=>$sha,bytes=>length($b),mode=>'100644',git_oid=>exists($by_path{$rel})?$by_path{$rel}->{git_oid}:undef,overlay=>$overlay};
}},$source);
@final=sort{$a->{path}cmp$b->{path}}@final;
die "wrong overlaid population\n" unless @final==1744;
put("$root/control/source.overlay.sha256",join('',map{"$_->{sha256}  $_->{path}\n"}@final));
json("$root/control/source-correspondence.json",{schema=>'fingerprint-golden-source-correspondence-1',commit=>$commit,trees=>{src=>$tree[0],library=>$tree[1],tests=>$tree[2]},copied_git_files=>scalar(@entries),final_files=>scalar(@final),files=>\@final});
for my $c(@controls) { put("$root/control/$c->[1]",$frozen_bytes{$c->[1]}); }
put("$root/bin/compiler-baseline",$canonical_before);
my @cs=lstat("$repo/target/lkjscript"); chmod($cs[2]&0777,"$root/bin/compiler-baseline") or die "producer chmod\n";
die "owned producer mismatch\n" unless raw("$root/bin/compiler-baseline") eq $canonical_before;
die "canonical changed during copy\n" unless raw("$repo/target/lkjscript") eq $canonical_before;
my $bytes=0; $bytes+=$_->{bytes} for @final;
my %pin=map{$_->{path}=>$_->{sha256}}@final;
json("$root/control/pins.json",{
 schema=>'fingerprint-baseline-golden-pins-1',status=>'PREPARED_NOT_EXECUTED',root=>$root,source=>$source,commit=>$commit,
 trees=>{src=>$tree[0],library=>$tree[1],tests=>$tree[2]},source_files=>scalar(@final),source_bytes=>$bytes,
 source_manifest_sha256=>sha256_hex(raw("$root/control/source.overlay.sha256")),
 original_manifest_sha256=>sha256_hex(raw("$root/control/source.git.sha256")),
 correspondence_sha256=>sha256_hex(raw("$root/control/source-correspondence.json")),
 producer=>{original=>"$repo/target/lkjscript",owned=>"$root/bin/compiler-baseline",sha256=>$producer,bytes=>32588975},
 incremental_module_sha256=>$pin{'src/elaborate/incremental.lkj'},runtime=>{mode=>'explicit pinned source',path=>'library/runtime.lkj',sha256=>$pin{'library/runtime.lkj'}},
 overlays=>{'src/main.lkj'=>$pin{'src/main.lkj'},'src/fingerprint_fixture.lkj'=>$pin{'src/fingerprint_fixture.lkj'}},
 cases=>[map{$_->{id}}@{$fixture->{cases}}],none_ordinals=>[4,5,7,8,9,10],
 source_boundary=>'Only tracked src/library/tests from the fixed Git tree plus the two named overlays; no bench, seed, tools, .git, key or store is copied.'
});
print JSON::PP->new->canonical->encode({status=>'prepared_not_executed',git_files=>scalar(@entries),source_files=>scalar(@final),source_bytes=>$bytes,producer_sha256=>$producer,incremental_sha256=>$incremental}),"\n";
