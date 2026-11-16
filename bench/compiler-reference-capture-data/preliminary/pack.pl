use strict;
use warnings;
use JSON::PP ();
use Digest::SHA qw(sha256_hex);
use File::Find ();
use Fcntl qw(:DEFAULT :mode);
use Archive::Tar ();
my $ROOT='/home/coder/workspace/lkjscript';
my $OUT="$ROOT/target/compiler-reference-capture-preliminary-retention-1";
my @ROOTS=map {"target/compiler-reference-capture-$_"} qw(preparation-1 desired-1 core-check-1 semantics-1 semantics-2 semantics-3 semantics-4);
my $ARCHIVE="$OUT/preliminary-roots.tar.gz";
my $JSON=JSON::PP->new->canonical->pretty;
my %expected=(
 $ROOTS[0]=>[42,2,11106447], $ROOTS[1]=>[1743,43,9381010], $ROOTS[2]=>[18,1,3419],
 $ROOTS[3]=>[15,1,215788], $ROOTS[4]=>[15,1,215247], $ROOTS[5]=>[18,1,428202], $ROOTS[6]=>[33,1,21524311]
);
sub need ($$) { my ($ok,$why)=@_; die "$why\n" unless $ok; }
sub same { my ($a,$b,$why)=@_; need(defined($a)&&defined($b)&&$a eq $b,$why); }
sub save {
 my ($name,$b)=@_; sysopen(my $f,"$OUT/$name",O_WRONLY|O_CREAT|O_EXCL,0600) or die "new $name: $!\n";
 binmode $f; print {$f} $b or die "write $name\n"; close $f or die "close $name\n";
}
sub read_bytes {open my $f,'<:raw',$_[0] or die "read $_[0]: $!\n"; local $/; my $b=<$f>; close $f or die "close $_[0]\n"; return defined($b)?$b:'';}
sub meta {
 my ($p)=@_; my @s=lstat("$ROOT/$p"); need(@s,"missing $p");
 need(S_ISDIR($s[2])||S_ISREG($s[2]),"symlink/special refused: $p");
 return {path=>$p,kind=>S_ISDIR($s[2])?'directory':'file',bytes=>$s[7],dev=>$s[0],ino=>$s[1],mode=>sprintf('%04o',$s[2]&07777),nlink=>$s[3],mtime=>$s[9],ctime=>$s[10]};
}
sub identity {
 my ($a,$b,$why)=@_;
 for my $k (qw(path kind dev ino mode nlink mtime ctime)) {same($a->{$k},$b->{$k},"$why $k");}
 same($a->{bytes},$b->{bytes},"$why bytes") if $a->{kind} eq 'file';
}
sub open_original {
 my ($e)=@_; identity(meta($e->{path}),$e,'original before open');
 sysopen(my $f,"$ROOT/$e->{path}",O_RDONLY|O_NOFOLLOW) or die "open $e->{path}: $!\n";
 binmode $f; my @s=stat($f);
 need(@s && S_ISREG($s[2]) && $s[0]==$e->{dev} && $s[1]==$e->{ino},'opened identity');
 return $f;
}
sub finish_original {
 my ($f,$e)=@_; my @s=stat($f);
 need(@s && $s[0]==$e->{dev} && $s[1]==$e->{ino} && $s[7]==$e->{bytes} && $s[9]==$e->{mtime} && $s[10]==$e->{ctime},'original changed while read');
 close $f or die "close original\n"; identity(meta($e->{path}),$e,'original after close');
}
sub enumerate {
 my @entries;
 for my $scope (@ROOTS) {
  my $m=meta($scope); same($m->{kind},'directory','owned root');
  File::Find::find({no_chdir=>1,follow=>0,wanted=>sub {
   my $p=substr($File::Find::name,length($ROOT)+1);
   need($p eq $scope || index($p,"$scope/")==0,'entry outside literal scope');
   need($p !~ /[\r\n\0]/ && $p !~ m{(?:^|/)\.\.?(/|$)},'unsupported path');
   my $e=meta($p); $e->{scope}=$scope;
   need($e->{nlink}==1,"multiply linked file refused: $p") if $e->{kind} eq 'file';
   push @entries,$e;
  }},"$ROOT/$scope");
 }
 return [sort {$a->{path} cmp $b->{path}} @entries];
}
my $entries=enumerate();
my (%files,%directories,%scope_totals);
for my $e (@$entries) {
 my $s=$scope_totals{$e->{scope}} //= {files=>0,dirs=>0,bytes=>0};
 if ($e->{kind} eq 'directory') {$directories{$e->{path}}=$e; $s->{dirs}++; next;}
 my $f=open_original($e); my $h=Digest::SHA->new(256); my $n=0;
 while(1) {my $got=read($f,my $b,65536); die "inventory read: $!\n" unless defined $got; last unless $got; $h->add($b); $n+=$got;}
 finish_original($f,$e); same($n,$e->{bytes},'inventory size'); $e->{sha256}=$h->hexdigest;
 $files{$e->{path}}=$e; $s->{files}++; $s->{bytes}+=$n;
}
for my $scope (@ROOTS) {my $s=$scope_totals{$scope}; same(join(',',@$s{qw(files dirs bytes)}),join(',',@{$expected{$scope}}),"frozen scope changed: $scope");}
same(scalar(keys %files),1884,'file count'); same(scalar(keys %directories),50,'directory count');
my $source_manifest="$ROOTS[6]/source-after.sha256";
same($files{$source_manifest}{sha256},'e34cdb687a9e7195e686f38bf53a6f55c1b38f7f1769f60605ba36361b6119b7','final source manifest pin');
same($files{"$ROOTS[6]/source-before.sha256"}{sha256},$files{$source_manifest}{sha256},'successful source before/after');
my $native="$ROOTS[6]/semantics";
same($files{$native}{sha256},'9da231621d686f79a870feee8d1ea04fe59da282783dc446dd527d9d34f7bac9','actual successful native binary');
same($files{$native}{bytes},21095338,'native bytes');
need(exists($files{"$ROOTS[0]/source.tar"}),'original source archive missing');
my @attempts;
for my $n (1..4) {
 my $scope="target/compiler-reference-capture-semantics-$n";
 my $status=read_bytes("$ROOT/$scope/build.status"); $status =~ s/\n\z//;
 need($status =~ /^\d+$/,'build status framing');
 need($status!=0,"failed attempt $n unexpectedly passed") if $n<4;
 same($status,0,'fourth build exit') if $n==4;
 my $run=exists($files{"$scope/run.status"})?read_bytes("$ROOT/$scope/run.status"):undef;
 $run =~ s/\n\z// if defined($run);
 same($run,0,'fourth native exit') if $n==4;
 need(!defined($run),'failed build unexpectedly has native result') if $n<4;
 push @attempts,{attempt=>$n,build_exit=>$status,run_exit=>$run,stderr_sha256=>$files{"$scope/build.stderr"}{sha256},build_time=>read_bytes("$ROOT/$scope/build.time"),load_before=>read_bytes("$ROOT/$scope/build.load-before"),load_after=>read_bytes("$ROOT/$scope/build.load-after")};
}
my $core_status=read_bytes("$ROOT/$ROOTS[2]/status"); same($core_status,"0\n",'core check exit');
my $controls={
 root_review_claim=>'728 contracts, zero failures; exact raw native stderr/stdout are retained in the sole archive',
 attempts=>\@attempts,
 native=>{path=>$native,sha256=>$files{$native}{sha256},bytes=>$files{$native}{bytes}},
 source_manifest=>{path=>$source_manifest,sha256=>$files{$source_manifest}{sha256}},
 core_check=>{status=>$core_status,time=>read_bytes("$ROOT/$ROOTS[2]/time"),load_before=>read_bytes("$ROOT/$ROOTS[2]/load-before"),load_after=>read_bytes("$ROOT/$ROOTS[2]/load-after")},
 fourth_run=>{status=>read_bytes("$ROOT/$ROOTS[6]/run.status"),time=>read_bytes("$ROOT/$ROOTS[6]/run.time"),load_before=>read_bytes("$ROOT/$ROOTS[6]/run.load-before"),load_after=>read_bytes("$ROOT/$ROOTS[6]/run.load-after"),stdout=>read_bytes("$ROOT/$ROOTS[6]/run.stdout"),stderr=>read_bytes("$ROOT/$ROOTS[6]/run.stderr")}
};
save('controls.json',$JSON->encode($controls));
save('inventory.json',$JSON->encode({schema=>'compiler-reference-capture-preliminary-inventory-1',root=>$ROOT,roots=>\@ROOTS,files=>1884,directories=>50,logical_bytes=>42874424,scopes=>\%scope_totals,entries=>$entries}));
save('archive-members.nul',join('',map {$_->{path}."\0"} @$entries));
save('original-files.sha256',join('',map {$files{$_}{sha256}."  $_\n"} sort keys %files));
open my $df,'-|','df','-Pk',$OUT or die "df: $!\n"; my $dfbytes; {local $/; $dfbytes=<$df>;} close $df or die "df failed\n"; save('space-before', $dfbytes);
my @dfrows=split /\n/,$dfbytes; my @columns=split /\s+/,$dfrows[1];
need(defined($columns[3]) && $columns[3]=~/^\d+$/ && $columns[3]>=3*int((42874424+1023)/1024)+65536,'insufficient free space');
save('archive-command.json',$JSON->encode({
 command=>['/bin/bash','-c','one fixed tar | gzip pipeline; exact source is embedded in pack.pl'],
 tar=>['tar','--sort=name','--mtime=@0','--owner=0','--group=0','--numeric-owner','--format=gnu','--no-recursion','--verbatim-files-from','--null','-C',$ROOT,'-cf','-','-T',"$OUT/archive-members.nul"],
 gzip=>['gzip','-n','-6'],output=>$ARCHIVE,compression_workers=>1,
 external_address_space_bytes=>4294967296,watchdog_seconds=>900,termination_grace_seconds=>10,
 contract=>'one deterministic archive; no retry or extraction; all original bytes retained in exactly one member per path'
}));
need(!-e $ARCHIVE && !-l $ARCHIVE,'archive destination already exists');
print "inventory-complete files=1884 directories=50 bytes=42874424; starting sole archive\n";
$|=1;
my $shell = <<'SH';
set -u
pack_out=$1
pack_root=$2
LC_ALL=C tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner --format=gnu --no-recursion --verbatim-files-from --null -C "$pack_root" -cf - -T "$pack_out/archive-members.nul" 2>"$pack_out/tar.stderr" |
    gzip -n -6 > "$pack_out/preliminary-roots.tar.gz" 2>"$pack_out/gzip.stderr"
printf '%s\n' "${PIPESTATUS[@]}" > "$pack_out/archive-component-statuses"
SH
my $rc=system('/bin/bash','-c',$shell,'reference-capture-pack',$OUT,$ROOT);
save('archive-shell-status',"$rc\n"); same($rc,0,'archive shell status');
same(read_bytes("$OUT/archive-component-statuses"),"0\n0\n",'tar/gzip exits');
same(read_bytes("$OUT/tar.stderr"),'','tar stderr'); same(read_bytes("$OUT/gzip.stderr"),'','gzip stderr');
my $tar=Archive::Tar->new;
$tar->read($ARCHIVE,1) or die "read retained archive: ".$tar->error."\n";
my (%seen,@verified);
my $verified_bytes=0;
for my $member ($tar->get_files) {
 my $p=$member->full_path; $p =~ s{/\z}{} if $member->is_dir;
 need(!exists($seen{$p}),'duplicate archive member'); $seen{$p}=1;
 my $e=$member->is_dir?$directories{$p}:$files{$p};
 need(defined($e),"unexpected archive member: $p");
 same($member->uid,0,'archive uid'); same($member->gid,0,'archive gid'); same($member->mtime,0,'archive mtime');
 same(sprintf('%04o',$member->mode&07777),$e->{mode},'archive mode');
 if($member->is_dir) {same($e->{kind},'directory','archive directory'); next;}
 need($member->is_file,'nonregular archive entry');
 my $content=$member->get_content; same(length($content),$e->{bytes},'archive size');
 my $f=open_original($e); my $h=Digest::SHA->new(256); my $at=0;
 while(1) {
  my $n=read($f,my $b,65536); die "verification read: $!\n" unless defined $n; last unless $n;
  same(substr($content,$at,$n),$b,"archive/original full bytes differ: $p");
  $h->add($b); $at+=$n;
 }
 finish_original($f,$e); same($at,$e->{bytes},'verified length'); my $sha=$h->hexdigest; same($sha,$e->{sha256},'original digest unchanged');
 $verified_bytes+=$at;
 push @verified,{path=>$p,bytes=>$at,sha256=>$sha,byte_equal=>JSON::PP::true,recovery=>"preliminary-roots.tar.gz::$p"};
}
same(scalar(keys %seen),1934,'archive complete member population');
same(scalar(@verified),1884,'archive regular members'); same($verified_bytes,42874424,'verified payload bytes');
my $after=enumerate(); same(scalar(@$after),scalar(@$entries),'final path count');
for my $i (0..$#$entries) {identity($after->[$i],$entries->[$i],'final original identity');}
open my $af,'<:raw',$ARCHIVE or die "archive hash open\n"; my $ah=Digest::SHA->new(256); $ah->addfile($af); close $af or die "archive close\n";
my $sha=$ah->hexdigest; my $size=-s $ARCHIVE;
save('verification.json',$JSON->encode({
 schema=>'compiler-reference-capture-preliminary-retention-1',status=>'verified',root=>$ROOT,roots=>\@ROOTS,
 archive=>'preliminary-roots.tar.gz',archive_sha256=>$sha,archive_bytes=>$size,archive_regular_members=>1884,archive_directories=>50,
 original_logical_bytes=>42874424,all_members_byte_equal=>JSON::PP::true,all_original_paths_and_metadata_unchanged=>JSON::PP::true,
 rows=>\@verified,native=>$controls->{native},source_manifest=>$controls->{source_manifest},
 failed_attempts_preserved=>[1,2,3],fourth_build_and_native_exits=>[0,0],
 qualifications=>[
 'One tar/gzip archive was created. Every regular member was compared byte-for-byte with its original exactly once during verification; originals were also hashed for the complete inventory.',
 'Tar ordering is byte-sorted, ownership and mtime normalized, and gzip stores no timestamp or source filename. Determinism follows the frozen invocation; no second compression was run.',
 'No member was extracted or executed. There are no extra native-image copies or per-image gzips.',
 'The exact seven roots are the complete scope; the concurrently active compiler-reference-capture-core-operation-1 directory is excluded.',
 'All three failed compiler attempts and replacement/fix provenance remain in the archive and at their original paths. No semantic workload, compiler, generator, profiler or observer was rerun.',
 'Recovery is not a cleanup release. No original, bench file, key, service or other worktree was changed or deleted.'
 ]
}));
print "verified archive=$sha bytes=$size members=1934 files=1884 directories=50 payload=42874424\n";
