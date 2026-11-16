#!/usr/bin/perl
# Finite, cooperative cleanup. No retries, workload, key reads or archive expansion.
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use IPC::Open2;
use File::Find;
use Fcntl qw(:DEFAULT :mode O_DIRECTORY O_NOFOLLOW O_NONBLOCK);
use IO::Handle;
use Cwd qw(abs_path);
my $REPO='/home/coder/workspace/lkjscript';
my $BASE="$REPO/bench/compiler-fingerprint-index-data";
my $PREP="$BASE/cleanup-preparation-1";
my $REC="$BASE/recovery-1";
my $OUT="$BASE/cleanup-execution-1";
my %ROOT=(
 P=>"$REPO/target/compiler-fingerprint-index-preparation-1",
 G=>"$REPO/target/compiler-fingerprint-index-golden-1",
 I=>"$REPO/target/compiler-fingerprint-index-integration-1"
);
my %COUNT=(P=>[9,39730],G=>[1816,45498446],I=>[49,1382925]);
my $REC_MANIFEST='f7f88ed9b02362c2d3ca9b80f9fcad3a9787db2761b90bbe53a17c9b910875d7';
my $SEED_MANIFEST='5fb0e5e5d19b0f8379565fb4567c81f52bdb4e281e2eea078d8bc55baa2ebe05';
my $A='b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db';
my $json=JSON::PP->new->canonical;
my($log,$git_in,$git_out,$git_pid);my(%git_cache,%held,%dir_by);
my($seq,$removed_files,$removed_dirs,$removed_bytes)=(0,0,0,0);my $phase='release';
sub fail { die "$_[0]\n"; }
sub raw {
 my($p)=@_;my @s=lstat($p);fail("not regular single-link: $p") unless @s&&S_ISREG($s[2])&&$s[3]==1;
 sysopen my $f,$p,O_RDONLY|O_NOFOLLOW|O_NONBLOCK or fail("open $p: $!");
 my @t=stat($f);fail("opened identity drift: $p") unless @t&&join(':',@s[0,1,2,3,4,5,7]) eq join(':',@t[0,1,2,3,4,5,7]);
 local $/;my $b=<$f>;close $f or fail("close $p");return defined($b)?$b:'';
}
sub put {my($p,$b)=@_;sysopen my $f,$p,O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW,0600 or fail("create $p: $!");binmode $f;print {$f}$b or fail("write $p");close $f or fail("close $p");}
sub save {put($_[0],JSON::PP->new->canonical->pretty->encode($_[1]));}
sub event {my($kind,$fields)=@_;my $row={sequence=>$seq++,event=>$kind,%{$fields//{}}};print {$log}$json->encode($row)."\n" or fail('progress write');}
sub safe_rel {my($p,$dot)=@_;return 1 if $dot&&$p eq '.';return $p=~/\A[\x20-\x7e]+\z/&&$p!~m{\A/|//|(?:\A|/)\.\.?(?:/|\z)|/\z};}
sub exact_read {my($f,$n)=@_;my $b='';while(length($b)<$n){my $r=read($f,my $p,$n-length($b));fail('short Git read')unless defined($r)&&$r>0;$b.=$p;}return $b;}
sub blob {
 my($commit,$path)=@_;fail('invalid Git provider')unless $commit=~/\A[0-9a-f]{40}\z/&&safe_rel($path,0);
 my $key="$commit:$path";return $git_cache{$key} if exists $git_cache{$key};
 print {$git_in}"$key\n" or fail('Git request');my $h=<$git_out>;
 fail("missing committed recovery: $key")unless defined($h)&&$h=~/\A([0-9a-f]{40}) blob (\d+)\n\z/;
 my($oid,$n)=($1,0+$2);fail('unexpected recovery blob size')if $n>67108864;
 my $b=exact_read($git_out,$n);fail('Git framing')unless exact_read($git_out,1) eq "\n";
 return $git_cache{$key}={oid=>$oid,bytes=>$b};
}
sub committed {
 my($commit,$rel,$sha)=@_;fail('unsafe retained relative path')unless safe_rel($rel,0);
 my $b=raw("$BASE/$rel");fail("retained hash mismatch: $rel")unless sha256_hex($b) eq $sha;
 fail("retained bytes not committed: $rel")unless blob($commit,"bench/compiler-fingerprint-index-data/$rel")->{bytes} eq $b;
 return $b;
}
sub manifest {
 my($bytes)=@_;my %m;
 for my $line(split /\n/,$bytes){fail('manifest framing')unless $line=~/\A([0-9a-f]{64})  (.+)\z/;my($sha,$path)=($1,$2);fail('manifest path/duplicate')unless safe_rel($path,0)&&!exists $m{$path};$m{$path}=$sha;}
 return \%m;
}
sub table {
 my($bytes,$header)=@_;my @l=split /\n/,$bytes;fail('inventory header')unless shift(@l) eq $header;
 my @cols=split /\t/,$header;my @out;
 for my $line(@l){my @v=split /\t/,$line,-1;fail('inventory width')unless @v==@cols;my %r;@r{@cols}=@v;push @out,\%r;}
 return \@out;
}
sub meta {
 my($s,$r,$dir,$links)=@_;
 return @$s&&($dir?S_ISDIR($s->[2]):S_ISREG($s->[2]))&&sprintf('%04o',$s->[2]&07777) eq $r->{mode}
  &&$s->[0]==$r->{dev}&&$s->[1]==$r->{inode}&&$s->[4]==$r->{uid}&&$s->[5]==$r->{gid}
  &&(!$links||$s->[3]==$r->{nlink})&&($dir||($s->[7]==$r->{bytes}&&$s->[3]==1));
}
sub absolute {my($r)=@_;return $r->{path} eq '.'?$ROOT{$r->{root}}:"$ROOT{$r->{root}}/$r->{path}";}
sub check_dir {
 my($key,$links)=@_;my $r=$dir_by{$key};fail('unlisted parent')unless $r;
 my @s=lstat(absolute($r));fail("directory path drift: $key")unless meta(\@s,$r,1,$links);
 if(exists $held{$key}){my @t=stat($held{$key});fail("held directory drift: $key")unless meta(\@t,$r,1,0);}
}
sub parents {
 my($r)=@_;my @parts=split m{/},$r->{path};pop @parts;my $rel='.';
 check_dir("$r->{root}/.",0);
 for my $part(@parts){$rel=$rel eq '.'?$part:"$rel/$part";check_dir("$r->{root}/$rel",0);}
}
sub check_file {
 my($r)=@_;parents($r);my $p=absolute($r);my @s=lstat($p);fail("file metadata drift: $p")unless meta(\@s,$r,0,1);
 sysopen my $f,$p,O_RDONLY|O_NOFOLLOW|O_NONBLOCK or fail("open $p: $!");my @t=stat($f);
 fail("opened file drift: $p")unless meta(\@t,$r,0,1);
 my $sha=Digest::SHA->new(256)->addfile($f)->hexdigest;
 @t=stat($f);fail("file changed during hash: $p")unless meta(\@t,$r,0,1)&&$sha eq $r->{sha256};
 close $f or fail("close $p");@s=lstat($p);fail("file name drift after hash: $p")unless meta(\@s,$r,0,1);
}
fail('wrong helper path')unless abs_path($0) eq "$PREP/guard.pl";
fail("usage: perl $PREP/guard.pl $PREP/release.json")unless @ARGV==1&&$ARGV[0] eq "$PREP/release.json";
my $release_bytes=raw($ARGV[0]);my $release=$json->decode($release_bytes);
fail('not released')unless ($release->{schema}//'') eq 'fingerprint-three-root-cleanup-release-1'
 &&($release->{status}//'') eq 'RELEASED_AFTER_COMPARISON_OWNS_A'
 &&$release->{comparison_owns_producer_a}&&($release->{comparison_producer_a_sha256}//'') eq $A
 &&($release->{comparison_copy_evidence}//'')=~/\S/&&$release->{comparison_copy_evidence}!~/ROOT_TO_FILL/;
my $commit=$release->{retention_commit}//'';fail('retention commit required')unless $commit=~/\A[0-9a-f]{40}\z/;
fail('wrong release scope/pins')unless ($release->{recovery_manifest_sha256}//'') eq $REC_MANIFEST
 &&($release->{seed_proof_manifest_sha256}//'') eq $SEED_MANIFEST
 &&($release->{guard_sha256}//'') eq sha256_hex(raw("$PREP/guard.pl"));
mkdir $OUT,0700 or fail("attempt already exists or output mkdir failed: $!");
sysopen $log,"$OUT/progress.jsonl",O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW,0600 or fail('progress create');$log->autoflush(1);
save("$OUT/release.json",$release);event('begin',{retention_commit=>$commit,release_sha256=>sha256_hex($release_bytes)});
my $ok=eval {
 $phase='retention-preflight';$git_pid=open2($git_out,$git_in,'git','-C',$REPO,'cat-file','--batch');binmode $git_out;binmode $git_in;my $old=select $git_in;$|=1;select $old;
 committed($commit,'cleanup-preparation-1/guard.pl',$release->{guard_sha256});
 my $rm=committed($commit,'recovery-1/manifest.sha256',$REC_MANIFEST);my $rmap=manifest($rm);fail('recovery record count')unless keys(%$rmap)==32;
 for my $rel(sort keys %$rmap){committed($commit,"recovery-1/$rel",$rmap->{$rel});}
 my $sm=committed($commit,'cleanup-preparation-1/seed-proof.sha256',$SEED_MANIFEST);my $smap=manifest($sm);fail('seed proof count')unless keys(%$smap)==18;
 for my $rel(sort keys %$smap){committed($commit,$rel,$smap->{$rel});}
 my $seed_base="$BASE/baseline-seed-recovery-1";
 fail('seed proof did not pass')unless raw("$seed_base/status") eq "0\n"&&raw("$seed_base/git-seed-comparison.status") eq "0\n"&&raw("$seed_base/stderr") eq ''&&raw("$seed_base/stdout") eq '';
 my $seed_inputs=raw("$seed_base/inputs-before.sha256");
 fail('seed proof inputs drift')unless $seed_inputs eq raw("$seed_base/inputs-after.sha256")
  &&$seed_inputs eq "c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae  seed/lkjscript.gz\n$A  $ROOT{G}/bin/compiler-baseline\n";
 my $files=table(raw("$REC/files.tsv"),"root\tpath\tmode\tuid\tgid\tdev\tinode\tnlink\tbytes\tsha256\tprovider\tmember\tprovider_oid\tbyte_comparison");
 my $dirs=table(raw("$REC/directories.tsv"),"root\tpath\tmode\tuid\tgid\tdev\tinode\tnlink");
 fail('fixed population')unless @$files==1874&&@$dirs==57;
 my(%file_by,%counts,%sizes);my $total=0;
 for my $r(@$dirs){fail('directory scope/path')unless exists($ROOT{$r->{root}})&&safe_rel($r->{path},1);my $k="$r->{root}/$r->{path}";fail('duplicate directory')if exists $dir_by{$k};$dir_by{$k}=$r;}
 for my $r(@$files){
  fail('file scope/path/hash')unless exists($ROOT{$r->{root}})&&safe_rel($r->{path},0)&&$r->{sha256}=~/\A[0-9a-f]{64}\z/&&$r->{bytes}=~/\A[0-9]+\z/&&$r->{nlink}==1;
  my $k="$r->{root}/$r->{path}";fail('duplicate/overlapping file')if exists($file_by{$k})||exists($dir_by{$k});$file_by{$k}=$r;++$counts{$r->{root}};$sizes{$r->{root}}+=$r->{bytes};$total+=$r->{bytes};
 }
 for my $id(qw(P G I)){fail('per-root counts')unless $counts{$id}==$COUNT{$id}->[0]&&$sizes{$id}==$COUNT{$id}->[1]&&exists $dir_by{"$id/."};}
 fail('fixed byte total')unless $total==46921101;
 my $providers=$json->decode(raw("$REC/providers.json"));
 for my $r(@$files){
  my $p=$providers->{$r->{provider}};fail('missing provider')unless $p;
  if($p->{kind} eq 'git-blob'){
   my $path=$p->{prefix}.(exists($p->{fixed})?$p->{fixed}:$r->{member});my $b=blob($p->{commit},$path);
   fail('original Git recovery mismatch')unless $b->{oid} eq $r->{provider_oid}&&length($b->{bytes})==$r->{bytes}&&sha256_hex($b->{bytes}) eq $r->{sha256};
  }elsif($r->{provider} eq 'seed'){
   fail('seed row mismatch')unless $r->{root} eq 'G'&&$r->{path} eq 'bin/compiler-baseline'&&$r->{sha256} eq $A&&$r->{bytes}==32588975;
   my $b=blob($p->{commit},$p->{path});fail('seed Git blob mismatch')unless $b->{oid} eq $p->{git_blob}&&sha256_hex($b->{bytes}) eq $p->{gzip_sha256};
  }elsif($r->{provider} eq 'image'){
   fail('golden image row mismatch')unless $r->{root} eq 'G'&&$r->{path} eq 'bin/fingerprint-golden'&&$r->{sha256} eq $p->{expanded_sha256}&&$r->{bytes}==$p->{expanded_bytes};
   my $proof=$json->decode(raw("$REC/records/expansion-comparison.json"));
   fail('image expansion proof mismatch')unless $proof->{status} eq 'exact'&&$proof->{compared_every_byte}&&$proof->{expanded_sha256} eq $r->{sha256}&&$proof->{expanded_bytes}==$r->{bytes}&&$proof->{gzip_sha256} eq $p->{gzip_sha256};
  }else{fail('unsupported provider');}
 }
 close $git_in or fail('Git input close');close $git_out or fail('Git output close');waitpid($git_pid,0)==$git_pid&&$?==0 or fail('Git provider exit');$git_pid=undef;
 event('retention-preflight-complete',{files=>1874,seed_supplement=>'direct proof verified; old pending map unchanged',comparison_ownership=>'root release attestation; no comparison-root inspection'});
 $phase='original-preflight';
 for my $r(@$dirs){
  check_dir("$r->{root}/$r->{path}",1);my $p=absolute($r);sysopen my $h,$p,O_RDONLY|O_DIRECTORY|O_NOFOLLOW or fail("hold $p: $!");
  my @s=stat($h);fail('held directory identity mismatch')unless meta(\@s,$r,1,1);$held{"$r->{root}/$r->{path}"}=$h;
 }
 for my $id(qw(P G I)){
  my(%seen_f,%seen_d);
  find({no_chdir=>1,follow=>0,wanted=>sub{
   my $p=$File::Find::name;my $rel=$p eq $ROOT{$id}?'.':substr($p,length($ROOT{$id})+1);my $k="$id/$rel";my @s=lstat($p);fail('population lstat')unless @s;
   if(S_ISDIR($s[2])){fail("unlisted/different directory: $k")unless exists($dir_by{$k})&&meta(\@s,$dir_by{$k},1,1);$seen_d{$k}=1;}
   elsif(S_ISREG($s[2])){fail("unlisted/different file: $k")unless exists($file_by{$k})&&meta(\@s,$file_by{$k},0,1);$seen_f{$k}=1;}
   else{fail("symlink/special original: $k");}
  }},$ROOT{$id});
  fail('missing original paths')unless keys(%seen_f)==$COUNT{$id}->[0]&&keys(%seen_d)==scalar(grep{$_->{root} eq $id}@$dirs);
  for my $r(grep{$_->{root} eq $id}@$files){check_file($r);}
  event('root-preflight-complete',{root=>$id,files=>$COUNT{$id}->[0],bytes=>$COUNT{$id}->[1],directories=>scalar(keys %seen_d)});
 }
 event('all-preflight-complete',{files=>1874,directories=>57,bytes=>46921101});
 $phase='file-removal';
 for my $r(@$files){
  check_file($r);event('unlink-intent',{root=>$r->{root},path=>$r->{path},sha256=>$r->{sha256},bytes=>0+$r->{bytes}});
  unlink(absolute($r))==1 or fail("unlink ".absolute($r).": $!");
  ++$removed_files;$removed_bytes+=$r->{bytes};event('unlink-done',{root=>$r->{root},path=>$r->{path}});
 }
 $phase='directory-removal';
 for my $r(sort{my $ad=$a->{path} eq '.'?0:scalar(split m{/},$a->{path});my $bd=$b->{path} eq '.'?0:scalar(split m{/},$b->{path});$bd<=>$ad||$a->{root}cmp$b->{root}||$a->{path}cmp$b->{path}}@$dirs){
  parents($r)unless $r->{path} eq '.';check_dir("$r->{root}/$r->{path}",0);
  opendir my $d,absolute($r) or fail('directory open before removal');my @left=grep{$_ ne '.'&&$_ ne '..'}readdir($d);closedir $d or fail('directory close');fail('unexpected nonempty directory')if @left;
  event('rmdir-intent',{root=>$r->{root},path=>$r->{path}});rmdir(absolute($r)) or fail("rmdir ".absolute($r).": $!");
  ++$removed_dirs;event('rmdir-done',{root=>$r->{root},path=>$r->{path}});
 }
 $phase='absence';
 for my $id(qw(P G I)){my @s=lstat($ROOT{$id});fail("root remains: $id")if @s;fail("absence is not ENOENT: $id")unless $!{ENOENT};event('root-absent',{root=>$id,errno=>'ENOENT'});}
 fail('completion counts')unless $removed_files==1874&&$removed_dirs==57&&$removed_bytes==46921101;
 event('complete',{files=>$removed_files,directories=>$removed_dirs,bytes=>$removed_bytes});1;
};
my $error=$@;
eval{event($ok?'accepted':'stopped',{phase=>$phase,removed_files=>$removed_files,removed_directories=>$removed_dirs,removed_bytes=>$removed_bytes,error=>$ok?undef:$error});};
save("$OUT/result.json",{schema=>'fingerprint-three-root-cleanup-result-1',status=>$ok?'complete':'stopped',phase=>$phase,removed_files=>$removed_files,removed_directories=>$removed_dirs,removed_bytes=>$removed_bytes,error=>$ok?undef:$error,scope=>\%ROOT,retention_commit=>$commit,
 contract=>'Cooperative finite non-atomic cleanup. All originals and recoveries preflight before first unlink; per-leaf/path checks repeat. A failure stops without retry or rollback. No guarantee against a hostile concurrent rename between final check and unlink.'});
close $log or fail('progress close');
die $error unless $ok;
print "removed exactly 1874 files / 57 directories / 46921101 bytes; all three roots ENOENT\n";
