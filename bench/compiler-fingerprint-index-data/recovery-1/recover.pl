#!/usr/bin/perl
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use IPC::Open2;
use File::Find;
use Fcntl qw(:DEFAULT :mode);
use IO::Uncompress::Gunzip qw($GunzipError);
my $repo='/home/coder/workspace/lkjscript';
my $dest="$repo/bench/compiler-fingerprint-index-data/recovery-1";
my $json=JSON::PP->new->canonical->pretty;
my @root_specs=(
 ['P',"$repo/target/compiler-fingerprint-index-preparation-1",9,39730],
 ['G',"$repo/target/compiler-fingerprint-index-golden-1",1816,45498446],
 ['I',"$repo/target/compiler-fingerprint-index-integration-1",49,1382925]
);
my %providers=(
 prep=>{kind=>'git-blob',commit=>'1f74338b332bb6750f7b29c0cd9f25ff41e9ad5f',prefix=>'bench/compiler-fingerprint-index-data/preparation-1/'},
 golden_prep=>{kind=>'git-blob',commit=>'042bce952af30959b1ecdff2fc686033f789c497',prefix=>'bench/compiler-fingerprint-index-data/golden-preparation-1/'},
 raw=>{kind=>'git-blob',commit=>'88a06df2b212d5856b62aa9bcb49e9092cad1d10',prefix=>'bench/compiler-fingerprint-index-data/golden-acceptance-1/results-v2/raw/'},
 integration=>{kind=>'git-blob',commit=>'88a06df2b212d5856b62aa9bcb49e9092cad1d10',prefix=>'bench/compiler-fingerprint-index-data/integration-preparation-1/'},
 source=>{kind=>'git-blob',commit=>'9326dbaab1bf6832b9f35dc156e12d0d62eccd9e',prefix=>''},
 overlay_main=>{kind=>'git-blob',commit=>'042bce952af30959b1ecdff2fc686033f789c497',prefix=>'bench/compiler-fingerprint-index-data/golden-preparation-1/control/',fixed=>'golden-main.txt'},
 overlay_fixture=>{kind=>'git-blob',commit=>'042bce952af30959b1ecdff2fc686033f789c497',prefix=>'bench/compiler-fingerprint-index-data/golden-preparation-1/control/',fixed=>'incremental-fingerprint-module.txt'},
 release=>{kind=>'git-blob',commit=>'88a06df2b212d5856b62aa9bcb49e9092cad1d10',prefix=>'bench/compiler-fingerprint-index-data/',fixed=>'golden-release-1.json'},
 seed=>{kind=>'git-gzip',commit=>'e5294c339d9aefbbafc2e951c997f4fefabcc9ad',path=>'seed/lkjscript.gz',gzip_sha256=>'c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae',expanded_sha256=>'b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db',expanded_bytes=>32588975},
 image=>{kind=>'new-gzip',path=>'artifacts/golden-image-99a179be.gz',expanded_sha256=>'99a179be5fd5f6cd9c8e7d3e4466468ff4b519a406303e77e935c1d27dcade89',expanded_bytes=>1568827}
);
sub raw {
 my($p)=@_; my @s=lstat($p); die "not regular single-link: $p\n" unless @s&&S_ISREG($s[2])&&$s[3]==1;
 open my $f,'<:raw',$p or die "$p: $!"; local $/; my $b=<$f>; close $f or die "close";
 my @t=lstat($p); die "file metadata changed during read: $p\n" unless @t && join(':',@s[0,1,2,3,4,5,7]) eq join(':',@t[0,1,2,3,4,5,7]);
 return $b;
}
sub put { my($p,$b)=@_; sysopen my $f,$p,O_WRONLY|O_CREAT|O_EXCL,0600 or die "$p: $!"; binmode $f; print {$f} $b or die "write"; close $f or die "close"; }
sub save { put($_[0],$json->encode($_[1])); }
sub hash_file { open my $f,'<:raw',$_[0] or die "$_[0]: $!"; my $h=Digest::SHA->new(256)->addfile($f)->hexdigest; close $f or die "close"; return $h; }
sub exact_read { my($f,$n)=@_; my $b=''; while(length($b)<$n){my $r=read($f,my $part,$n-length($b)); die "short read" unless defined($r)&&$r>0; $b.=$part;} return $b; }
sub cmd { open my $f,'-|',@_ or die "command"; local $/; my $b=<$f>; close $f or die "command failed: @_"; $b=~s/\n\z//; return $b; }
sub load3 { open my $f,'<','/proc/loadavg' or die $!; my $line=<$f>; close $f; die "load" unless $line=~/\A(\d+\.\d+) (\d+\.\d+) (\d+\.\d+) /; return [$1,$2,$3]; }
my($git_out,$git_in); my $git_pid=open2($git_out,$git_in,'git','-C',$repo,'cat-file','--batch');
binmode $git_out; binmode $git_in; my $old=select $git_in; $|=1; select $old;
my %cache;
sub git_blob {
 my($commit,$path)=@_; my $spec="$commit:$path"; return $cache{$spec} if exists $cache{$spec};
 print {$git_in} "$spec\n" or die "Git request";
 my $h=<$git_out>; die "missing/nonblob committed provider: $spec\n" unless defined($h)&&$h=~/\A([0-9a-f]{40}) blob (\d+)\n\z/;
 my($oid,$n)=($1,0+$2); my $b=exact_read($git_out,$n); die "Git separator" unless exact_read($git_out,1) eq "\n";
 return $cache{$spec}={oid=>$oid,bytes=>$b};
}
my @files; my @directories; my @root_info;
for my $spec(@root_specs) {
 my($id,$base,$want_files,$want_bytes)=@$spec; my($count,$bytes)=(0,0); my @rs=lstat($base);
 die "root missing/special: $base\n" unless @rs&&S_ISDIR($rs[2]);
 find({no_chdir=>1,wanted=>sub{
  my $path=$File::Find::name; my @s=lstat($path); die "lstat $path" unless @s;
  my $rel=$path eq $base ? '.' : substr($path,length($base)+1);
  die "unsafe/non-ASCII original path\n" if $rel=~/[^\x20-\x7e]/ || $rel=~m{\A/|(?:\A|/)\.\.(?:/|\z)};
  my $row={root=>$id,path=>$rel,mode=>sprintf('%04o',$s[2]&07777),uid=>$s[4],gid=>$s[5],dev=>$s[0],inode=>$s[1],nlink=>$s[3],bytes=>$s[7]};
  if(S_ISDIR($s[2])) { push @directories,$row; return; }
  die "symlink/special/hardlinked leaf: $path\n" unless S_ISREG($s[2])&&$s[3]==1;
  $row->{absolute}=$path; push @files,$row; ++$count; $bytes+=$s[7];
 }},$base);
 die "root population/size drift: $base\n" unless $count==$want_files&&$bytes==$want_bytes;
 push @root_info,{id=>$id,path=>$base,files=>$count,bytes=>$bytes,mode=>sprintf('%04o',$rs[2]&07777),uid=>$rs[4],gid=>$rs[5],dev=>$rs[0],inode=>$rs[1]};
}
@files=sort{$a->{root}cmp$b->{root}||$a->{path}cmp$b->{path}}@files;
@directories=sort{$a->{root}cmp$b->{root}||$a->{path}cmp$b->{path}}@directories;
my %counts; my %byte_counts; my $image_path;
for my $f(@files) {
 my($id,$rel)=@$f{qw(root path)}; my($provider,$member);
 if($id eq 'P') {($provider,$member)=('prep',$rel);}
 elsif($id eq 'I') {($provider,$member)=('integration',$rel);}
 elsif($rel eq 'source/src/main.lkj') {($provider,$member)=('overlay_main','-');}
 elsif($rel eq 'source/src/fingerprint_fixture.lkj') {($provider,$member)=('overlay_fixture','-');}
 elsif($rel=~m{\Asource/((?:src|library|tests)/.+)\z}) {($provider,$member)=('source',$1);}
 elsif($rel=~m{\Acontrol/}||$rel=~m{\Arecords/preparation/}||$rel eq 'preparation-manifest.sha256') {($provider,$member)=('golden_prep',$rel);}
 elsif($rel=~m{\Arecords/execution/(.+)\z}) {($provider,$member)=('raw',$1);}
 elsif($rel eq 'release.json') {($provider,$member)=('release','-');}
 elsif($rel eq 'bin/compiler-baseline') {($provider,$member)=('seed','-');}
 elsif($rel eq 'bin/fingerprint-golden') {($provider,$member)=('image','-');$image_path=$f->{absolute};}
 else {die "unrepresented original path, contents not read: $id/$rel\n";}
 my $p=$providers{$provider}; my $provider_blob;
 if($p->{kind} eq 'git-blob') {
  my $path=$p->{prefix}.(exists($p->{fixed})?$p->{fixed}:$member);
  $provider_blob=git_blob($p->{commit},$path);
  $f->{provider_oid}=$provider_blob->{oid};
 }
 my $live=raw($f->{absolute});
 $f->{sha256}=sha256_hex($live); die "original length changed\n" unless length($live)==$f->{bytes};
 if($p->{kind} eq 'git-blob') {
  die "original/committed provider bytes differ: $id/$rel\n" unless $live eq $provider_blob->{bytes};
  $f->{byte_comparison}='direct-git-bytes-equal';
 } else {
  die "binary input digest/size mismatch: $id/$rel\n" unless $f->{sha256} eq $p->{expanded_sha256}&&$f->{bytes}==$p->{expanded_bytes};
  $f->{provider_oid}='-'; $f->{byte_comparison}=$provider eq 'seed' ? 'pending-seed-expansion-comparison' : 'pending-authorized-gzip-comparison';
 }
 if($rel=~m{\Asource/}) {die "copied source mode changed\n" unless $f->{mode} eq '0644';}
 $f->{provider}=$provider; $f->{member}=$member;
 ++$counts{$provider}; $byte_counts{$provider}+=$f->{bytes};
}
my $seed=git_blob($providers{seed}->{commit},$providers{seed}->{path});
die "retained seed gzip digest\n" unless sha256_hex($seed->{bytes}) eq $providers{seed}->{gzip_sha256};
$providers{seed}->{git_blob}=$seed->{oid};$providers{seed}->{gzip_bytes}=length($seed->{bytes});
$providers{seed}->{verification}='PENDING direct expanded-byte comparison. Original owned compiler hash/size and compressed Git seed hash are verified. This task did not expand the seed or read canonical.';
$providers{seed}->{known_expanded_pin_source}='042bce952af30959b1ecdff2fc686033f789c497:bench/compiler-fingerprint-index-data/golden-preparation-1/control/pins.json';
close $git_in or die "Git input close"; close $git_out or die "Git output close"; waitpid($git_pid,0)==$git_pid&&$?==0 or die "Git batch exit";
my @columns=qw(root path mode uid gid dev inode nlink bytes sha256 provider member provider_oid byte_comparison);
# The complete file map is published only after the new image comparison succeeds.
my @dcolumns=qw(root path mode uid gid dev inode nlink);
put("$dest/directories.tsv",join("\t",@dcolumns)."\n".join('',map{my $f=$_;join("\t",map{$f->{$_}}@dcolumns)."\n"}@directories));
save("$dest/roots.json",\@root_info);
save("$dest/records/preflight.json",{status=>'all_originals_enumerated_and_git_providers_byte_compared',files=>scalar(@files),directories=>scalar(@directories),provider_files=>\%counts,provider_bytes=>\%byte_counts,seed_comparison=>'pending_no_expansion_authorized_here',new_image_comparison=>'pending_single_gzip_and_full_expansion'});
die "missing golden image\n" unless defined($image_path);
my $artifact="$dest/$providers{image}->{path}";
die "image compression already attempted\n" if lstat($artifact);
my $gzip_record="$dest/records/gzip";
my $format="elapsed_seconds\t%e\nuser_seconds\t%U\nsystem_seconds\t%S\nmax_rss_kib\t%M\nexit\t%x";
my @argv=('/usr/bin/time','-o',"$gzip_record.time.tsv",'-f',$format,
 '/usr/bin/timeout','--signal=TERM','--kill-after=10s','900s',
 '/usr/bin/prlimit','--as=4294967296','--','/usr/bin/gzip','-n','-9','-c',$image_path);
save("$gzip_record.argv.json",{argv=>\@argv,cwd=>$repo,stdout_artifact=>$providers{image}->{path},stdin=>'/dev/null',policy=>'Single deterministic gzip creation, no retry/recompression.'});
my $before_utc=cmd('/usr/bin/date','-u','+%Y-%m-%dT%H:%M:%S.%NZ');my $before_ns=cmd('/usr/bin/date','+%s%N');my $load_before=load3();
my $pid=fork();die "fork" unless defined($pid);
if($pid==0) {
 chdir $repo or die "chdir";open STDIN,'<','/dev/null' or die "stdin";
 sysopen STDOUT,$artifact,O_WRONLY|O_CREAT|O_EXCL,0600 or die "artifact: $!";
 sysopen STDERR,"$gzip_record.stderr",O_WRONLY|O_CREAT|O_EXCL,0600 or die "stderr: $!";
 exec {$argv[0]} @argv or die "gzip exec";
}
waitpid($pid,0)==$pid or die "gzip wait";my $wait_status=$?;
my $after_ns=cmd('/usr/bin/date','+%s%N');my $after_utc=cmd('/usr/bin/date','-u','+%Y-%m-%dT%H:%M:%S.%NZ');my $load_after=load3();
my $status=($wait_status&127)?128+($wait_status&127):$wait_status>>8;
save("$gzip_record.status.json",{exit_code=>$status,wait_status=>$wait_status,before_utc=>$before_utc,after_utc=>$after_utc,before_ns=>"$before_ns",after_ns=>"$after_ns",load_before=>$load_before,load_after=>$load_after});
die "gzip failed; preserve artifact and stop\n" unless $status==0;
# Exactly one expansion; compare every produced byte directly to the owned image.
my $expand_start=cmd('/usr/bin/date','+%s%N');my $expand_load_before=load3();
my $z=IO::Uncompress::Gunzip->new($artifact,MultiStream=>0,Strict=>1) or die "gunzip open: $GunzipError";
open my $original,'<:raw',$image_path or die "image open";
my $expanded=Digest::SHA->new(256);my $total=0;
while(1) {
 my $n=$z->read(my $chunk,65536);die "gunzip read: $GunzipError" unless defined($n)&&$n>=0;last if $n==0;
 my $same=exact_read($original,$n);die "expanded/current golden bytes differ\n" unless $same eq $chunk;
 $expanded->add($chunk);$total+=$n;
}
my $extra=read($original,my $tail,1);die "image length/read mismatch\n" unless defined($extra)&&$extra==0;
close $original or die "image close";$z->close or die "gunzip close: $GunzipError";
my $expanded_sha=$expanded->hexdigest;
die "expanded golden identity mismatch\n" unless $total==$providers{image}->{expanded_bytes}&&$expanded_sha eq $providers{image}->{expanded_sha256};
$providers{image}->{gzip_sha256}=hash_file($artifact);$providers{image}->{gzip_bytes}=-s $artifact;
$providers{image}->{verification}='One full expansion compared byte-for-byte to the exact owned image; expanded identity matches the pinned original. New artifact is pending root commit.';
save("$dest/records/expansion-comparison.json",{status=>'exact',implementation=>'IO::Uncompress::Gunzip, Strict=1, one streaming expansion',artifact=>$providers{image}->{path},gzip_sha256=>$providers{image}->{gzip_sha256},gzip_bytes=>$providers{image}->{gzip_bytes},expanded_sha256=>$expanded_sha,expanded_bytes=>$total,original=>$image_path,compared_every_byte=>JSON::PP::true,
 before_ns=>"$expand_start",after_ns=>cmd('/usr/bin/date','+%s%N'),load_before=>$expand_load_before,load_after=>load3(),new_expanded_file_created=>JSON::PP::false});
for my $f(@files) { $f->{byte_comparison}='direct-gzip-expanded-bytes-equal' if $f->{provider} eq 'image'; }
put("$dest/files.tsv",join("\t",@columns)."\n".join('',map{my $f=$_;join("\t",map{$f->{$_}}@columns)."\n"}@files));
for my $id(keys %providers) {
 $providers{$id}->{files}=$counts{$id}//0;$providers{$id}->{original_bytes}=$byte_counts{$id}//0;
 $providers{$id}->{verification}='Every mapped original compared directly to the immutable Git blob bytes.' if $providers{$id}->{kind} eq 'git-blob';
}
save("$dest/providers.json",\%providers);
my $bytes=0;$bytes+=$_->{bytes} for @files;
save("$dest/summary.json",{
 schema=>'fingerprint-three-root-recovery-1',status=>'recovery_prepared_roots_retained',
 roots=>scalar(@root_specs),files=>scalar(@files),directories=>scalar(@directories),logical_bytes=>$bytes,
 committed_git_byte_comparisons=>scalar(@files)-2,new_gzip_full_byte_comparisons=>1,pending_seed_expansion_comparisons=>1,
 new_raw_record_copies=>0,new_gzip_files=>1,all_originals_still_required=>JSON::PP::true,
 original_modes=>'Exact current octal modes/uid/gid/dev/inode/nlink are recorded for every original file and directory. Providers recover bytes; restore mode from this inventory. No historical nanosecond metadata claim.',
 pending=>'Owned baseline A remains in place. Its b01 digest and the e529 compressed seed digest match known committed pins; no new seed expansion comparison was performed.',
 exclusions=>'No deletion, key read, canonical read, gate-store/measurement-preparation access, unrelated archive expansion, source generation, compiler/native execution, observer or Git commit.',
 files_sha256=>hash_file("$dest/files.tsv"),directories_sha256=>hash_file("$dest/directories.tsv"),providers_sha256=>hash_file("$dest/providers.json"),
 image=>$providers{image}
});
print $json->encode({status=>'recovery_prepared_no_cleanup',files=>scalar(@files),directories=>scalar(@directories),logical_bytes=>$bytes,new_raw_record_copies=>0,pending_seed_expansion_comparisons=>1,image_gzip_sha256=>$providers{image}->{gzip_sha256},image_gzip_bytes=>$providers{image}->{gzip_bytes},summary_sha256=>hash_file("$dest/summary.json")});
