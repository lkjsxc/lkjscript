use strict;
use warnings;
use Fcntl qw(:DEFAULT :mode);
use File::Find;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
my $root='/home/coder/workspace/lkjscript-compiler-scalar-20261116';
my $pkg="$root/target/compiler-scalar-validation-package-1";
chdir $root or die "cwd: $!";
my $json=JSON::PP->new->canonical->pretty;
sub raw {
 my($p)=@_;
 sysopen my $f,$p,O_RDONLY|O_NOFOLLOW or die "open $p: $!";
 my @s=stat($f); die "not regular: $p\n" unless @s && S_ISREG($s[2]);
 local $/; my $b=<$f>; close $f or die "close $p: $!"; return defined($b)?$b:'';
}
sub hash_file {
 my($p)=@_;
 sysopen my $f,$p,O_RDONLY|O_NOFOLLOW or die "open $p: $!";
 my @s=stat($f); die "not regular: $p\n" unless @s && S_ISREG($s[2]);
 my $h=Digest::SHA->new(256); $h->addfile($f);
 close $f or die "close $p: $!"; return ($h->hexdigest,$s[7],$s[2]&07777);
}
sub save {
 my($p,$bytes,$mode)=@_; $mode=0600 unless defined $mode;
 sysopen my $f,$p,O_WRONLY|O_CREAT|O_EXCL,$mode or die "create $p: $!";
 binmode $f; print {$f} $bytes or die "write $p: $!"; close $f or die "close $p: $!";
 chmod $mode,$p or die "chmod $p: $!";
}
sub save_json { my($p,$v)=@_; save($p,$json->encode($v)); }
sub sq { my $s=shift; $s =~ s/'/'\\''/g; return "'$s'"; }
sub exact {
 my($p,$h,$n)=@_; my($a,$b)=hash_file($p);
 die "input hash/size mismatch: $p\n" unless $a eq $h && (!defined($n)||$b==$n);
 return $b;
}
my $inventory_path="$root/target/compiler-scalar-retention-proposal-1/closed-inventory.json";
exact($inventory_path,'bb18dc804c6591917118d4184c4a87a6f78b9fba5a48899d64709d39443ec8b8',undef);
my $inv=decode_json(raw($inventory_path));
die "inventory declaration mismatch\n" unless $inv->{total_files}==388 && $inv->{retained_record_files}==386 && $inv->{total_bytes}==30958886;
my %all=map { $_->{path}=>$_ } @{$inv->{files}};
die "duplicate inventory paths\n" unless keys(%all)==388;
my %records=map { $_->{path}=>$_ } grep { $_->{class} eq 'retained-record' } @{$inv->{files}};
my @binary=(
 {source=>'target/lkjscript',sha256=>'f85624e3cc694f6a3a3e10c33f822789ae6084e2d5a5b41c3b1fc8fcc9bd2d64',bytes=>32761734,suffix=>'compiler.gz'},
 {source=>'target/compiler-scalar-live-preliminary-1/live',sha256=>'fff89090526d3c9f0ed6081f57bd54121e51773912b833bff62db31e531a1b85',bytes=>21251056,suffix=>'live.gz'},
 {source=>'target/compiler-scalar-proposals/desired-tests-input.tar',sha256=>'556b02937776b789adc51e04127ba4ff919400c8be047509970f28418f47e156',bytes=>5376000,suffix=>'tests-input.tar.gz'}
);
my %binary_by_path=map { $_->{source}=>$_ } @binary;
sub check_closed {
 my %seen;
 for my $s(@{$inv->{scopes}}){
  my $d="$root/$s->{scope}"; my @top=lstat($d);
  die "scope not directory: $d\n" unless @top&&S_ISDIR($top[2]);
  find({no_chdir=>1,follow=>0,wanted=>sub{
   my $p=$File::Find::name; my @s=lstat($p); die "lstat $p: $!" unless @s;
   return if S_ISDIR($s[2]);
   die "nonregular $p\n" unless S_ISREG($s[2]);
   my $rel=substr($p,length($root)+1);
   die "unlisted or duplicate $p\n" unless exists $all{$rel} && !$seen{$rel}++;
   die "forbidden key path: $p\n" if $rel =~ m{(?:^|/)(?:store-key|key|private-key|[^/]+\.key)$};
   my $e=$all{$rel}; die "size/mode changed: $p\n" unless $s[7]==$e->{bytes} && sprintf('%04o',$s[2]&07777) eq $e->{mode};
   my $hash=$e->{sha256};
   $hash=$binary_by_path{$rel}{sha256} if !defined($hash)&&exists $binary_by_path{$rel};
   die "unbound input $p\n" unless defined $hash;
   exact($p,$hash,$e->{bytes});
  }},$d);
 }
 die "closed scope population mismatch\n" unless keys(%seen)==388;
}
check_closed();
exact("$root/target/lkjscript",$binary[0]{sha256},$binary[0]{bytes});
for my $d(qw(artifacts copied-proposals)){mkdir "$pkg/$d",0700 or die "$pkg/$d: $!";}
my @copies;
for my $name(qw(compiler-scalar-retention-proposal-1 compiler-scalar-release-proposal-1)){
 my $source="$root/target/$name";
 my @paths;
 find({no_chdir=>1,follow=>0,wanted=>sub {
  my $p=$File::Find::name; my @s=lstat($p); die "lstat $p\n" unless @s;
  die "not regular/dir: $p\n" unless S_ISDIR($s[2])||S_ISREG($s[2]);
  push @paths,[$p,\@s];
 }},$source);
 @paths=sort{$a->[0] cmp $b->[0]}@paths;
 for my $p(@paths){
  my($from,$st)=@$p; my $rel=substr($from,length($source));
  my $to="$pkg/copied-proposals/$name$rel";
  if(S_ISDIR($st->[2])){mkdir $to,$st->[2]&07777 or die "mkdir $to: $!";next;}
  my $bytes=raw($from); save($to,$bytes,$st->[2]&07777);
  my $h=sha256_hex($bytes); exact($to,$h,length($bytes)); exact($from,$h,length($bytes));
  push @copies,{source=>"target/$name$rel",copy=>"copied-proposals/$name$rel",sha256=>$h,bytes=>length($bytes)};
 }
}
die "proposal copy count mismatch\n" unless @copies==14;
save_json("$pkg/proposal-copies.json",\@copies);
save("$pkg/closed-members.nul",join('',map{"$_\0"}sort keys %records));
save("$pkg/closed-members.sha256",join('',map{"$records{$_}{sha256}  $_\n"}sort keys %records));
my %overlay=(
 'src/driver.lkj'=>{sha256=>'c3a1c555f9b14e0c09ee1fb8d2ab572dd901b93401bffe77e4dc960f9876a095'},
 'src/reuse.lkj'=>{sha256=>'e12f48ed1ee75176125697f5a3f882dacb251108030127195d7520c0d6c2a1aa'},
 'src/semantic_tests/optimizer.lkj'=>{sha256=>'bb282ac7e08d92c35466ce607f1d68dd3303a223627d34e1be1f26ef5da40cb3'}
);
my $desired="$root/target/compiler-scalar-desired";
for my $p(sort keys %overlay){
 my($h,$n,$m)=hash_file("$desired/$p"); die "overlay mismatch $p\n" unless $h eq $overlay{$p}{sha256};
 $overlay{$p}{bytes}=$n; $overlay{$p}{mode}=sprintf('%04o',$m);
}
save("$pkg/desired-overlay.members.nul",join('',map{"$_\0"}sort keys %overlay));
save("$pkg/desired-overlay.sha256",join('',map{"$overlay{$_}{sha256}  $_\n"}sort keys %overlay));
sub tar_command {
 my($base,$list,$out)=@_;
 return join(' ','/usr/bin/tar','--format=ustar','--mtime=@0','--owner=0','--group=0','--numeric-owner',
 '--hard-dereference','--no-recursion','--null','--verbatim-files-from','-C',sq($base),'-T',sq($list),'-cf','-',
 '|','/usr/bin/gzip','-n','-9','>',sq($out));
}
my @commands=(
 {kind=>'closed-records',shell=>tar_command($root,"$pkg/closed-members.nul","$pkg/closed-records.tar.gz")},
 {kind=>'desired-overlay',shell=>tar_command($desired,"$pkg/desired-overlay.members.nul","$pkg/desired-overlay.tar.gz")}
);
for my $b(@binary){
 $b->{artifact}="artifacts/$b->{sha256}.$b->{suffix}";
 push @commands,{kind=>$b->{source},shell=>'/usr/bin/gzip -n -9 -c -- '.sq("$root/$b->{source}").' > '.sq("$pkg/$b->{artifact}")};
}
save_json("$pkg/commands.json",\@commands);
for my $c(@commands){
 print "create $c->{kind}\n";
 my $s=system('/bin/bash','-o','pipefail','-o','noclobber','-c',$c->{shell});
 die "command failed ($s): $c->{kind}\n" unless $s==0;
}
sub take {
 my($f,$n)=@_; my $b='';
 while(length($b)<$n){
  my $r=read($f,my $part,$n-length($b));
  die "read failure: $!" unless defined $r;
  die "unexpected EOF\n" if !$r;
  $b.=$part;
 }
 return $b;
}
sub octal {
 my($v)=@_; $v =~ s/\0.*\z//s; $v =~ s/^\s+|\s+$//g;
 die "bad octal tar field\n" unless $v =~ /\A[0-7]{1,12}\z/;
 return oct($v);
}
sub tar_string {
 my $v=shift; $v =~ s/\0.*\z//s; return $v;
}
sub verify_tar {
 my($archive,$expected,$manifest)=@_;
 open my $z,'-|','/usr/bin/gzip','-dc','--',$archive or die "gzip pipe: $!";
 binmode $z;
 my %seen; my @verified; my $total=0;
 while(1){
  my $h=take($z,512);
  if($h eq "\0"x512){
   die "missing second tar end record\n" unless take($z,512) eq "\0"x512;
   while(1){my $n=read($z,my $b,65536);die "tail read: $!" unless defined $n;last unless $n;die "nonzero tar tail\n" if $b =~ /[^\0]/;}
   last;
  }
  my $chk=octal(substr($h,148,8)); my $copy=$h; substr($copy,148,8)=' 'x8;
  die "tar header checksum\n" unless unpack('%32C*',$copy)==$chk;
  die "tar format\n" unless substr($h,257,6) eq "ustar\0";
  my $type=substr($h,156,1);die "nonregular tar member\n" unless $type eq '0'||$type eq "\0";
  my $name=tar_string(substr($h,0,100));my $prefix=tar_string(substr($h,345,155));
  $name="$prefix/$name" if length $prefix;
  die "unexpected/duplicate tar member $name\n" unless exists $expected->{$name} && !$seen{$name}++;
  my $e=$expected->{$name}; my $n=octal(substr($h,124,12)); my $mode=octal(substr($h,100,8));
  die "tar size/mode mismatch $name\n" unless $n==$e->{bytes} && sprintf('%04o',$mode&07777) eq $e->{mode};
  die "tar normalized metadata mismatch\n" unless octal(substr($h,108,8))==0 && octal(substr($h,116,8))==0 && octal(substr($h,136,12))==0;
  my $sha=Digest::SHA->new(256);my $left=$n;
  while($left){my $want=$left>65536?65536:$left;$sha->add(take($z,$want));$left-=$want;}
  my $pad=(512-($n%512))%512;die "tar padding nonzero\n" if $pad&&take($z,$pad) ne "\0"x$pad;
  my $actual=$sha->hexdigest; die "tar payload mismatch $name\n" unless $actual eq $e->{sha256};
  push @verified,{path=>$name,sha256=>$actual,bytes=>$n};$total+=$n;
 }
 close $z or die "gzip decompression/CRC failed: $?";
 die "tar member population mismatch\n" unless keys(%seen)==keys(%$expected);
 @verified=sort{$a->{path}cmp$b->{path}}@verified;
 save($manifest,join('',map{"$_->{sha256}  $_->{path}\n"}@verified));
 my($h,$n)=hash_file($archive);
 return {archive=>substr($archive,length($pkg)+1),sha256=>$h,compressed_bytes=>$n,members=>scalar(@verified),member_bytes=>$total,
 verification=>'every member name, regular type, mode, length, SHA-256, header checksum, padding, normalized metadata, end framing and gzip CRC'};
}
my @archives;
push @archives,verify_tar("$pkg/closed-records.tar.gz",\%records,"$pkg/verified-closed-members.sha256");
push @archives,verify_tar("$pkg/desired-overlay.tar.gz",\%overlay,"$pkg/verified-desired-overlay.sha256");
die "closed manifest mismatch\n" unless raw("$pkg/closed-members.sha256") eq raw("$pkg/verified-closed-members.sha256");
for my $b(@binary){
 open my $z,'-|','/usr/bin/gzip','-dc','--',"$pkg/$b->{artifact}" or die "gzip: $!";
 sysopen my $orig,"$root/$b->{source}",O_RDONLY|O_NOFOLLOW or die "original: $!";
 binmode $z;binmode $orig;
 my $sha=Digest::SHA->new(256);my $total=0;
 while(1){
  my $n=read($z,my $part,65536);die "gzip read: $!" unless defined $n;last unless $n;
  die "gzip source byte mismatch: $b->{source}\n" unless take($orig,$n) eq $part;
  $sha->add($part);$total+=$n;
 }
 my $extra=read($orig,my $tail,1);die "original final read: $!" unless defined $extra;
 die "original extra bytes\n" if $extra;
 close $orig or die "original close: $!";
 close $z or die "gzip CRC/exit: $?";
 die "gzip identity mismatch\n" unless $total==$b->{bytes} && $sha->hexdigest eq $b->{sha256};
 my($h,$n)=hash_file("$pkg/$b->{artifact}");
 $b->{compressed_sha256}=$h;$b->{compressed_bytes}=$n;$b->{verified_byte_for_byte}=JSON::PP::true;
 print "verified $b->{source} bytes=$total compressed=$n\n";
}
check_closed();
exact("$root/target/lkjscript",$binary[0]{sha256},$binary[0]{bytes});
for my $c(@copies){exact("$root/$c->{source}",$c->{sha256},$c->{bytes});exact("$pkg/$c->{copy}",$c->{sha256},$c->{bytes});}
for my $p(keys %overlay){exact("$desired/$p",$overlay{$p}{sha256},$overlay{$p}{bytes});}
for my $id(qw(compiler-scalar-core-audit-1309 compiler-scalar-publications-audit-1320)){
 my $r=decode_json(raw("$pkg/gateway/$id.result.json"));
 die "gateway capture not complete\n" unless $r->{capture_status} eq 'complete' && $r->{finished} && !$r->{stdout_capture_truncated} && !$r->{stderr_capture_truncated};
 for my $stream(qw(stdout stderr)){
  my $b=raw("$pkg/gateway/$id.$stream");
  die "gateway payload mismatch\n" unless $b eq $r->{$stream} && length($b)==$r->{$stream.'_ref'}{total_bytes};
 }
}
save_json("$pkg/verification.json",{
 schema=>1,source_commit=>'5b5ce7759c035ae0437e3512e18efc440aa50d2e',
 base=>'e0b21e66659dc1830a5bebddf0c1f30fd2d8fa94',
 closed_scopes=>13,closed_files=>388,closed_bytes=>30958886,
 archived_records=>386,archived_record_bytes=>4331830,
 archives=>\@archives,binaries=>\@binary,copied_proposal_records=>14,desired_overlay=>\%overlay,
 source_inputs_unchanged=>JSON::PP::true,exact_gateway_exports=>2,
 compiler_or_native_executed=>JSON::PP::false,deletion=>JSON::PP::false,
 git_bundle=>'owned by root; not created here',
 excluded=>['target/gate','target/compiler-scalar-cold-gate-1','target/compiler-scalar-cold-store-1','target/compiler-scalar-bootstrap-store-1','all keys','durable edit metadata','main worktree']
});
print "PASS closed_records=386 overlay=3 binary_gzips=3 copied_proposals=14 gateway_outcomes=2\n";
