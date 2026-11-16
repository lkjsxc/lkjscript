#!/usr/bin/env perl
use strict;
use warnings;
use Fcntl qw(:DEFAULT :mode);
use Errno qw(ENOENT);
use Cwd qw(getcwd);
use File::Find;
use Digest::SHA qw(sha1_hex sha256_hex);
use JSON::PP;
use IO::Uncompress::Gunzip;
use IO::Handle;
use Time::HiRes qw(time);

my $REPO='/home/coder/workspace/lkjscript';
my $TARGET="$REPO/target";
my $BLOB_BASE='bench/compiler-reference-capture-data/';
my $BUNDLE_REL=$BLOB_BASE.'final-cleanup-1';
my $BUNDLE="$REPO/$BUNDLE_REL";
my $SELF="$BUNDLE/guard.pl";
my $SCOPE="$BUNDLE/expected-scope.json";
my $AUX="$BUNDLE/auxiliary-inventory.json";
my $RELEASE="$BUNDLE/release.json";
my $CAPTURE="$BUNDLE/execution-1";
my $RETENTION='9326dbaab1bf6832b9f35dc156e12d0d62eccd9e';
my $MAP_PATH=$BLOB_BASE.'final-recovery-1/recovery-map.json';
my $MAP_SHA='036832967ed5d4cc55e27c109bea315bac79877ad80636bfea5f81de3cca7bc1';
my $SCOPE_SHA='c93116cc86295a5f0e15a84821baa80a6061be5ae85d8d6335392aaa0a0c3e5a';
my $AUX_SHA='4cf9f1145b9b6018075fff726d8aa1aec7e87d1e0da3ccf4878844b591e84939';
my $OLD='d84cac6edcee59e2c2ad631fb6ca15d63340a1c8';
my @ROOTS=map {'compiler-reference-capture-'.$_} qw(
 comparison-2 comparison-launch-2 measurement-2 trial-preparation-2
 comparison-acceptance-2 comparison-recovery-2 measurement-preparation-1
 cleanup-preparation-2 cleanup-execution-2 final-recovery-1
);
my @TOP=map {'compiler-reference-capture-'.$_} qw(
 cleanup-inventory-1.json recovery-guard-proposal-1.md recovery-map-1.json
 recovery-proof-1.sha256 recovery-proposal-1.py recovery-run-1.json
 recovery-syntax-1.json recovery-time-1.txt source-commit-message.txt
);
my @OLD_ROOTS=@ROOTS[0..6];
my @AUX_ROOTS=@ROOTS[7..9];
my %ROOT=map {$_=>1} @ROOTS;
my %TOP=map {$_=>1} @TOP;
my %SEED=(
 '7d0731e99aeaf241c4334fc5ff065a341fb388c8'=>[
  '505be947525d53b4db1bff6c85cb77622cb2f5dc7e7d7fe92b772ee3b8ffc3e2',
  '6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc',32584542],
 'e5294c339d9aefbbafc2e951c997f4fefabcc9ad'=>[
  'c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae',
  'b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db',32588975]
);
my $J=JSON::PP->new->canonical->utf8;
my (%git_cache,%archive_cache,%expanded_cache,%recover_cache,%held_dirs,%entries,%descriptors);
my ($log,$capture_started,$mutation_started)=(undef,0,0);
my (@removed_files,@removed_dirs,@completed_roots,@completed_top);
my $pending_mutation;
my $start=time;
$|=1;
sub fail { die "$_[0]\n"; }
sub equal { fail($_[2]) unless $_[0] eq $_[1]; }
sub safe_relative {
 my($p)=@_;fail('unsafe relative path')unless defined($p)&&!ref($p)&&$p ne''&&$p=~m{\A[A-Za-z0-9_./-]+\z}&&$p!~m{\A/};
 fail('unsafe relative component')if grep {$_ eq''||$_ eq'.'||$_ eq'..'}split m{/},$p,-1;
 return $p;
}
sub uint {
 my($n,$max,$why)=@_;fail($why)unless defined($n)&&!ref($n)&&"$n"=~/\A(?:0|[1-9][0-9]*)\z/&&length("$n")<=16&&$n<=$max;return 0+$n;
}
sub sha_field { fail('bad SHA-256 field')unless defined($_[0])&&!ref($_[0])&&$_[0]=~/\A[0-9a-f]{64}\z/;return $_[0]; }
sub oid_field { fail('bad Git object field')unless defined($_[0])&&!ref($_[0])&&$_[0]=~/\A[0-9a-f]{40}\z/;return $_[0]; }
sub array_equal {
 my($a,$b,$why)=@_;fail($why)unless ref($a)eq'ARRAY'&&ref($b)eq'ARRAY'&&@$a==@$b;
 for my$i(0..$#$a){fail($why)unless defined($a->[$i])&&defined($b->[$i])&&!ref($a->[$i])&&!ref($b->[$i])&&"$a->[$i]"eq"$b->[$i]";}
}

sub bytes {
 my($p,$cap)=@_;$cap=134217728 unless defined$cap;
 sysopen my$f,$p,O_RDONLY|O_NOFOLLOW or fail("open $p: $!");
 my@s=stat$f;fail("unsafe input $p")unless@s&&S_ISREG($s[2])&&$s[4]==$>&&$s[3]==1&&$s[7]<=$cap;
 binmode$f;my$b='';while(1){my$n=sysread($f,my$part,65536);fail("read $p: $!")unless defined$n;last unless$n;$b.=$part;fail("input cap $p")if length($b)>$cap;}
 my@e=stat$f;close$f or fail("close $p: $!");
 fail("input changed while read $p")unless length($b)==$s[7]&&join(',',@s[0,1,2,3,4,5,7,9,10])eq join(',',@e[0,1,2,3,4,5,7,9,10]);
 return($b,\@s);
}

sub save_new {
 my($name,$b)=@_;safe_relative($name);fail('capture filename nesting')if$name=~m{/};
 sysopen my$f,"$CAPTURE/$name",O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW,0600 or fail("capture create $name: $!");
 binmode$f;print{$f}$b or fail("capture write $name: $!");close$f or fail("capture close $name: $!");
}
sub event {
 my($kind,$v)=@_;return unless$log;
 $pending_mutation={event=>$kind,%$v} if $kind eq'unlink_intent'||$kind eq'rmdir_intent';
 print{$log}$J->encode({event=>$kind,epoch=>time,%$v})."\n" or fail('progress write');
 undef $pending_mutation if $kind eq'unlinked'||$kind eq'removed_directory';
}
sub git {
 my(@args)=@_;open my$f,'-|','/usr/bin/git','-C',$REPO,@args or fail('Git launch');
 binmode$f;local$/;my$b=<$f>;close$f or fail('Git refusal');return defined($b)?$b:'';
}
sub git_blob {
 my($commit,$path)=@_;oid_field($commit);safe_relative($path);
 my$k="$commit:$path";return @{$git_cache{$k}}if exists$git_cache{$k};
 my$id=git('rev-parse','--verify',$k);chomp$id;oid_field($id);
 my$b=git('cat-file','blob',$id);fail('Git blob cap')if length($b)>134217728;
 equal(sha1_hex('blob '.length($b)."\0".$b),$id,'Git blob contents disagree');
 $git_cache{$k}=[$b,$id];return($b,$id);
}
sub inflate {
 my($compressed,$cap)=@_;my$z=IO::Uncompress::Gunzip->new(\$compressed,MultiStream=>0)or fail('gzip open');
 my$out='';while(1){my$n=$z->read(my$part,65536);fail('gzip read')if$n<0;last unless$n;fail('gzip extent cap')if length($out)+$n>$cap;$out.=$part;}
 $z->close or fail('gzip close');return$out;
}
sub tar_text {
 my($x)=@_;fail('tar NUL padding')unless$x=~/\A([^\0]*)\0*\z/s;return$1;
}
sub octal {
 my($x)=@_;$x=~s/[\0 ]+\z//;$x=~s/\A +//;fail('tar octal')unless$x=~/\A[0-7]+\z/&&length($x)<=12;my$n=oct$x;fail('tar integer cap')if$n>134217728;return$n;
}
sub pax {
 my($b)=@_;my(%p,$at);
 $at=0;while($at<length$b){my$tail=substr($b,$at);fail('PAX framing')unless$tail=~/\A([1-9][0-9]*) /;my$n=uint($1,1048576,'PAX length');fail('PAX extent')if$n>length($b)-$at;
 my$record=substr($b,$at,$n);fail('PAX record')unless$record=~/\A[1-9][0-9]* ([A-Za-z0-9_.-]+)=([^\n]*)\n\z/s;
 my($key,$value)=($1,$2);fail('duplicate PAX field')if exists$p{$key};
 fail('unsupported PAX field')unless$key=~/\A(?:path|mtime|atime|ctime|uid|gid|uname|gname|comment)\z/;
 $p{$key}=$value;$at+=$n;}return\%p;
}
sub tar_members {
 my($raw)=@_;fail('tar alignment')unless length($raw)%512==0;my(%members,%names,$pending,$ended);my$at=0;
 while($at+512<=length$raw){
  my$h=substr($raw,$at,512);
  if($h eq"\0"x512){fail('tar terminator')unless length($raw)-$at>=1024&&!defined$pending&&substr($raw,$at)eq"\0"x(length($raw)-$at);$ended=1;last;}
  my$magic=substr($h,257,8);fail('tar format')unless$magic eq"ustar\00000"||$magic eq"ustar  \0";
  my$check=$h;substr($check,148,8)=' 'x8;fail('tar checksum')unless unpack('%32C*',$check)==octal(substr($h,148,8));
  my$name=tar_text(substr($h,0,100));if($magic eq"ustar\00000"){my$p=tar_text(substr($h,345,155));$name="$p/$name"if$p ne'';}
  my$size=octal(substr($h,124,12));my$mode=octal(substr($h,100,8));fail('privileged archive mode')if$mode&07000;my$raw_name=$name;my$type=substr($h,156,1);$type='0'if$type eq"\0";
  fail('tar link target')unless tar_text(substr($h,157,100))eq'';
  my$end=$at+512+512*int(($size+511)/512);fail('tar extent')if$end>length$raw;
  my$content=substr($raw,$at+512,$size);fail('tar payload padding')unless substr($raw,$at+512+$size,$end-$at-512-$size)eq"\0"x($end-$at-512-$size);
  if($type eq'L'){
   fail('nested GNU long path')if defined$pending;fail('GNU long path')unless$content=~/\A([^\0]+)\0\z/s;$pending=$1;
  }elsif($type eq'x'||$type eq'g'){
   fail('nested path extension')if defined$pending;my$p=pax($content);
   fail('global path extension')if$type eq'g'&&exists$p->{path};
   $pending=$p->{path}if exists$p->{path};
  }elsif($type eq'0'||$type eq'5'){
   $name=$pending if defined$pending;$pending=undef;
   $name=~s{\A\./}{};$name=~s{/\z}{} if$type eq'5';
   if($name eq''||$name eq'.'){fail('non-directory root header')unless$type eq'5';$at=$end;next;}
   safe_relative($name);fail('duplicate tar member')if$names{$name}++;
   if($type eq'5'){fail('directory payload')if$size;}else{$members{$name}={data=>$content,mode=>$mode,header_offset=>$at,raw_path=>$raw_name};}
  }else{fail("unsupported tar member type $type");}
  $at=$end;
 }
 fail('unterminated archive')unless$ended;return\%members;
}

sub recovery_bytes {
 my($d,$depth)=@_;$depth=0 unless defined$depth;
 fail('recovery object/depth')unless ref($d)eq'HASH'&&$depth<=3;
 my$key=$J->encode($d);return$recover_cache{$key}if exists$recover_cache{$key};
 my$len=uint($d->{bytes},134217728,'recovery byte count');my$sha=sha_field($d->{sha256});my$out;
 if($d->{kind}eq'git-blob'){
  my$rev=oid_field($d->{revision});my$path=safe_relative($d->{path});
  fail('unapproved Git recovery')unless(($rev eq$RETENTION||$rev eq$OLD)&&index($path,$BLOB_BASE)==0)
   ||(exists($SEED{$rev})&&$path eq'seed/lkjscript.gz');
  my($b,$oid)=git_blob($rev,$path);equal($oid,oid_field($d->{git_blob}),'recovery Git object');$out=$b;
  equal(sha256_hex($b),$SEED{$rev}[0],'fixed compressed seed')if exists$SEED{$rev};
 } elsif($d->{kind}eq'gunzip-git-blob'){
  fail('gzip source descriptor')unless ref($d->{source})eq'HASH'&&$d->{source}{kind}eq'git-blob';
  my$s=$d->{source};my$p=safe_relative($s->{path});my$r=oid_field($s->{revision});
  fail('unapproved gzip expansion')unless(exists($SEED{$r})&&$p eq'seed/lkjscript.gz')
   ||(($r eq$RETENTION||$r eq$OLD)&&($p eq$BLOB_BASE.'setup2/artifacts/generator.gz'||$p eq$BLOB_BASE.'setup2/artifacts/generated-100k.tar.gz'));
  my$b=recovery_bytes($s,$depth+1);my$k=$J->encode($s);
  $expanded_cache{$k}=inflate($b,$len)unless exists$expanded_cache{$k};$out=$expanded_cache{$k};
  if(exists$SEED{$r}){fail('fixed seed expansion length')unless length($out)==$SEED{$r}[2];equal(sha256_hex($out),$SEED{$r}[1],'fixed expanded seed');}
 } elsif($d->{kind}eq'archive-member'){
  fail('archive descriptor')unless ref($d->{archive})eq'HASH'&&$d->{archive}{kind}eq'git-blob';
  my$a=$d->{archive};my$r=oid_field($a->{revision});my$p=safe_relative($a->{path});my$compression=$d->{compression};
  fail('unapproved archive revision')unless$r eq$RETENTION||$r eq$OLD;
  my$expected=$p eq$BLOB_BASE.'comparison-recovery/recovery.tar.gz'?'gzip':
   $p eq$BLOB_BASE.'setup2/artifacts/generated-100k.tar.gz'?'gzip':
   $p eq$BLOB_BASE.'setup2/artifacts/compiler-source.tar'?'none':'';
  fail('unapproved archive/encoding')unless$expected ne''&&defined($compression)&&!ref($compression)&&$compression eq$expected;
  my$b=recovery_bytes($a,$depth+1);my$k=$J->encode($a);
  $archive_cache{$k}=tar_members($compression eq'gzip'?inflate($b,134217728):$b)unless exists$archive_cache{$k};
  my$m=safe_relative($d->{member});fail('missing archive member')unless exists$archive_cache{$k}{$m};
  my$record=$archive_cache{$k}{$m};$out=$record->{data};
  fail('archive member mode')unless uint($d->{archive_mode},0777,'archive mode')==$record->{mode};
  if(exists$d->{raw_header_offset}){fail('archive header offset')unless uint($d->{raw_header_offset},134217728,'archive offset')==$record->{header_offset};}
  if(exists$d->{raw_header_path}){equal($d->{raw_header_path},$record->{raw_path},'archive raw header path');}
 } else {fail('unsupported recovery recipe');}
 fail('recovered byte count')unless length($out)==$len;equal(sha256_hex($out),$sha,'recovered digest');
 $recover_cache{$key}=$out;return$out;
}
sub identity {
 my($s)=@_;return[0+$s->[4],0+$s->[5],0+($s->[2]&07777),0+$s->[0],0+$s->[1]];
}
sub expected_identity {
 my($e)=@_;return[@$e{qw(uid gid mode device inode)}];
}
sub hold_directory {
 my($p,$e)=@_;return if exists$held_dirs{$p};
 my@s=lstat$p;fail("unsafe directory $p")unless@s&&S_ISDIR($s[2])&&$s[4]==$>;
 array_equal(identity(\@s),expected_identity($e),'directory identity mismatch');
 sysopen my$f,$p,O_RDONLY|O_DIRECTORY|O_NOFOLLOW or fail("hold directory $p: $!");
 my@h=stat$f;array_equal(identity(\@h),expected_identity($e),'held directory mismatch');
 $held_dirs{$p}={handle=>$f,identity=>expected_identity($e)};
}
sub held_identity {
 my($p)=@_;fail('unheld parent')unless exists$held_dirs{$p};
 my@s=lstat$p;fail('held named directory unavailable')unless@s&&S_ISDIR($s[2]);
 array_equal(identity(\@s),$held_dirs{$p}{identity},'named directory drift');
 my@h=stat($held_dirs{$p}{handle});array_equal(identity(\@h),$held_dirs{$p}{identity},'held descriptor drift');
}
sub parents_unchanged {
 my($relative)=@_;safe_relative($relative);held_identity($REPO);held_identity($TARGET);
 my@parts=split m{/},$relative;pop@parts;my$p=$TARGET;
 for my$n(@parts){$p.="/$n";held_identity($p);}
}
sub metadata {
 my($path)=@_;my@s=lstat$path;fail("missing metadata $path")unless@s;
 my$t=S_ISDIR($s[2])?'directory':S_ISREG($s[2])?'file':'special';
 return({type=>$t,uid=>0+$s[4],gid=>0+$s[5],mode=>0+($s[2]&07777),device=>0+$s[0],inode=>0+$s[1],
 nlink=>0+$s[3],bytes=>$t eq'file'?0+$s[7]:0,mtime=>0+$s[9],ctime=>0+$s[10]},\@s);
}
sub metadata_equal {
 my($got,$wanted,$directory_full)=@_;
 fail('entry type mismatch')unless$got->{type}eq$wanted->{type};
 my@fields=$got->{type}eq'file'?qw(uid gid mode device inode nlink bytes mtime ctime):
  $directory_full?qw(uid gid mode device inode nlink mtime ctime):qw(uid gid mode device inode);
 for my$k(@fields){fail("entry metadata drift: $k")unless defined($wanted->{$k})&&"$got->{$k}"eq"$wanted->{$k}";}
}
sub validate_file {
 my($path)=@_;my$e=$entries{$path};fail('unlisted regular file')unless$e&&$e->{type}eq'file';
 parents_unchanged($path);
 my($meta,$st)=metadata("$TARGET/$path");fail('unsafe regular file')unless$meta->{type}eq'file'&&$meta->{uid}==$>&&$meta->{nlink}==1;
 metadata_equal($meta,$e,1);
 my($b,$opened)=bytes("$TARGET/$path");
 my$o={type=>'file',uid=>0+$opened->[4],gid=>0+$opened->[5],mode=>0+($opened->[2]&07777),device=>0+$opened->[0],inode=>0+$opened->[1],
 nlink=>0+$opened->[3],bytes=>0+$opened->[7],mtime=>0+$opened->[9],ctime=>0+$opened->[10]};
 metadata_equal($o,$e,1);equal(sha256_hex($b),$e->{sha256},'original file digest drift');
 equal($b,recovery_bytes($descriptors{$path}),'original differs from independently recovered bytes');
 parents_unchanged($path);my($again)=metadata("$TARGET/$path");metadata_equal($again,$e,1);
}
sub population {
 my($check_bytes)=@_;my%seen;
 for my$root(@ROOTS){
  my$path="$TARGET/$root";my($root_meta)=metadata($path);fail('root is not directory')unless$root_meta->{type}eq'directory';
  find({no_chdir=>1,follow=>0,wanted=>sub{
   my$p=$File::Find::name;my$rel=substr($p,length($TARGET)+1);safe_relative($rel);
   fail('extra/duplicate scope member')unless exists$entries{$rel}&&!$seen{$rel}++;
   my($m)=metadata($p);fail('special/foreign/linked member')unless($m->{type}eq'directory'||$m->{type}eq'file')
    &&$m->{uid}==$>&&($m->{type}ne'file'||$m->{nlink}==1)&&!($m->{mode}&07000);
   metadata_equal($m,$entries{$rel},1);hold_directory($p,$entries{$rel})if$m->{type}eq'directory';
  }},$path);
 }
 for my$p(@TOP){fail('duplicate top-level file')if$seen{$p}++;my($m)=metadata("$TARGET/$p");
  fail('unsafe literal top-level file')unless$m->{type}eq'file'&&$m->{uid}==$>&&$m->{nlink}==1;metadata_equal($m,$entries{$p},1);}
 fail('missing scope population')unless keys(%seen)==keys(%entries);
 if($check_bytes){for my$p(sort keys%entries){validate_file($p)if$entries{$p}{type}eq'file';}}
 held_identity($REPO);held_identity($TARGET);
}
sub aux_destination {
 my($p)=@_;my$prefix='compiler-reference-capture-';
 if($p=~m{\Acompiler-reference-capture-cleanup-preparation-2/(.+)\z}){
  my$r=$1;return$BLOB_BASE.($r eq'release.json'?'cleanup-execution-2/release.json':"cleanup-preparation-2/$r");
 }
 if($p=~m{\Acompiler-reference-capture-cleanup-execution-2/(.+)\z}){return$BLOB_BASE."cleanup-execution-2/guard-result/$1";}
 if($p=~m{\Acompiler-reference-capture-final-recovery-1/(.+)\z}){return$BLOB_BASE."final-recovery-1/$1";}
 fail('unlisted auxiliary file')unless$TOP{$p};
 return$BLOB_BASE.'cleanup-mapping/'.$p.($p eq'compiler-reference-capture-recovery-proposal-1.py'?'.txt':'');
}
sub add_entry {
 my($e,$d)=@_;fail('entry object')unless ref($e)eq'HASH';my$p=safe_relative($e->{path});fail('duplicate entry')if exists$entries{$p};
 my($head,$tail)=split m{/},$p,2;fail('entry outside literal scope')unless$ROOT{$head}||(!defined($tail)&&$TOP{$head});
 fail('entry type')unless$e->{type}eq'file'||$e->{type}eq'directory';
 for my$k(qw(uid gid mode device inode nlink bytes mtime ctime)){uint($e->{$k},9007199254740991,"entry $k");}
 fail('foreign/privileged entry')unless$e->{uid}==$>&&$e->{mode}<=0777;
 if($e->{type}eq'file'){
  fail('file link/recovery')unless$e->{nlink}==1&&ref($d)eq'HASH';sha_field($e->{sha256});
  fail('file/recipe extent')unless$d->{bytes}==$e->{bytes}&&$d->{sha256}eq$e->{sha256};$descriptors{$p}=$d;
 }else{fail('directory in literal-file scope')if$TOP{$p};}
 $entries{$p}=$e;
}
sub absence {
 my($path)=@_;my@s=lstat$path;fail("absence is not ENOENT: $path")if@s||$!!=ENOENT;
}
sub empty_directory {
 my($path)=@_;held_identity($path);opendir my$dh,$path or fail("open directory for emptiness $path: $!");
 my@left=grep{$_ ne'.'&&$_ ne'..'}readdir$dh;closedir$dh or fail('directory close');fail("nonempty enumerated directory $path")if@left;
}
sub remove_file {
 my($p)=@_;validate_file($p);event('unlink_intent',{path=>"$TARGET/$p",sha256=>$entries{$p}{sha256},bytes=>$entries{$p}{bytes}});
 unlink("$TARGET/$p")==1 or fail("unlink stopped $p: $!");push@removed_files,"$TARGET/$p";
 absence("$TARGET/$p");event('unlinked',{path=>"$TARGET/$p",absence=>'ENOENT'});
}
sub main {
 equal(getcwd(),$REPO,'run from exact repository cwd');
 fail('exact release arguments required')unless@ARGV==3&&$ARGV[0]eq'--release'&&$ARGV[1]eq$RELEASE&&$ARGV[2]eq'--execute-after-root-release';
 my($self)=bytes($SELF,1048576);my$self_sha=sha256_hex($self);
 my($scope_bytes)=bytes($SCOPE,1048576);equal(sha256_hex($scope_bytes),$SCOPE_SHA,'scope pin');
 my($aux_bytes)=bytes($AUX,1048576);equal(sha256_hex($aux_bytes),$AUX_SHA,'auxiliary pin');
 my($release_bytes)=bytes($RELEASE,65536);my$r=decode_json($release_bytes);
 fail('release framing')unless ref($r)eq'HASH'&&$r->{schema}eq'compiler-reference-capture-final-cleanup-release-1';
 for my$k(qw(execute cooperative_quiescence_confirmed)){fail("release flag $k")unless ref($r->{$k})eq'JSON::PP::Boolean'&&$r->{$k};}
 fail('Python verdict must remain untrusted')unless ref($r->{prior_python_verdict_trusted})eq'JSON::PP::Boolean'&&!$r->{prior_python_verdict_trusted};
 equal($r->{guard_sha256},$self_sha,'released guard');equal($r->{expected_scope_sha256},$SCOPE_SHA,'released scope');
 equal($r->{retention_commit},$RETENTION,'released recovery revision');equal($r->{capture_root},$CAPTURE,'capture root');
 array_equal($r->{directory_roots},\@ROOTS,'released roots');array_equal($r->{top_level_files},\@TOP,'released literal files');
 fail('release totals')unless uint($r->{expected_regular_files},100000,'release files')==4532
  &&uint($r->{expected_directories},100000,'release directories')==145&&uint($r->{expected_logical_bytes},1073741824,'release bytes')==354054095;
 fail('explicit root release reference required')unless defined($r->{root_release_reference})&&!ref($r->{root_release_reference})
  &&length($r->{root_release_reference})>0&&length($r->{root_release_reference})<=1024&&$r->{root_release_reference}!~/PENDING/;
 my$approval=oid_field($r->{approval_commit});
 for my$pair([$BUNDLE_REL.'/guard.pl',$self],[$BUNDLE_REL.'/expected-scope.json',$scope_bytes],[$BUNDLE_REL.'/auxiliary-inventory.json',$aux_bytes]){
  my($approved)=git_blob($approval,$pair->[0]);equal($approved,$pair->[1],'reviewed committed control differs');
 }
 my$s=decode_json($scope_bytes);my$auxiliary=decode_json($aux_bytes);
 fail('scope schema')unless ref($s)eq'HASH'&&$s->{schema}eq'compiler-reference-capture-final-cleanup-scope-1';
 equal($s->{retention_commit},$RETENTION,'scope recovery revision');equal($s->{source_base},$TARGET,'scope source base');
 array_equal($s->{directory_roots},\@ROOTS,'scope roots');array_equal($s->{top_level_files},\@TOP,'scope literal files');
 equal($s->{seven_root_map}{path},$MAP_PATH,'map coordinate');equal($s->{seven_root_map}{sha256},$MAP_SHA,'map descriptor');
 equal($s->{auxiliary_inventory}{path},$BUNDLE_REL.'/auxiliary-inventory.json','auxiliary coordinate');
 equal($s->{auxiliary_inventory}{sha256},$AUX_SHA,'auxiliary descriptor');
 equal($s->{capture_root},$CAPTURE,'scope capture root');
 fail('scope totals')unless$s->{expected_regular_files}==4532&&$s->{expected_directories}==145&&$s->{expected_logical_bytes}==354054095;
 fail('auxiliary schema')unless ref($auxiliary)eq'HASH'&&$auxiliary->{schema}eq'compiler-reference-capture-final-cleanup-auxiliary-inventory-1';
 equal($auxiliary->{source_base},$TARGET,'auxiliary source base');array_equal($auxiliary->{roots},\@AUX_ROOTS,'auxiliary roots');array_equal($auxiliary->{top_level_files},\@TOP,'auxiliary top-level files');
 fail('capture already exists or inaccessible')if lstat$CAPTURE||$!!=ENOENT;
 mkdir$CAPTURE,0700 or fail("create permanent capture: $!");$capture_started=1;
 sysopen$log,"$CAPTURE/progress.jsonl",O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW,0600 or fail("progress create: $!");$log->autoflush(1);
 save_new('release.json',$release_bytes);save_new('invocation.json',$J->encode({argv=>[$^X,$SELF,@ARGV],cwd=>getcwd(),guard_sha256=>$self_sha,
 scope_sha256=>$SCOPE_SHA,auxiliary_sha256=>$AUX_SHA,retention_commit=>$RETENTION,approval_commit=>$approval,roots=>\@ROOTS,top_level_files=>\@TOP,
 started_epoch=>$start,prior_python_verdict_trusted=>JSON::PP::false})."\n");
 event('begin',{directory_roots=>10,literal_top_level_files=>9,expected_regular_files=>4532,expected_directories=>145,expected_bytes=>354054095});
 my($map_bytes)=git_blob($RETENTION,$MAP_PATH);equal(sha256_hex($map_bytes),$MAP_SHA,'committed map pin');
 fail('map length')unless length($map_bytes)==5318396;my$m=decode_json($map_bytes);
 fail('map framing')unless ref($m)eq'HASH'&&$m->{schema}eq'compiler-reference-capture-final-recovery-map-1';
 equal($m->{retained_revision},$OLD,'map historical recovery revision');equal($m->{source_base},$TARGET,'map base');
 array_equal($m->{scopes},\@OLD_ROOTS,'seven root map scope');fail('map unresolved')unless ref($m->{unresolved})eq'ARRAY'&&!@{$m->{unresolved}};
 for my$e(@{$m->{source_entries}}){
  my($head)=split m{/},$e->{path};fail('old map outside seven roots')unless grep{$_ eq$head}@OLD_ROOTS;
  my$d;if($e->{type}eq'file'){fail('file recovery reference')unless$e->{recovery}{kind}eq'provider';
   my$id=$e->{recovery}{id};fail('missing provider')unless defined($id)&&!ref($id)&&exists$m->{providers}{$id};$d=$m->{providers}{$id};}
  add_entry($e,$d);
 }
 my%aux_recovery;
 for my$d(@{$s->{auxiliary_recoveries}}){fail('auxiliary recovery framing')unless ref($d)eq'HASH';
  my$p=safe_relative($d->{source});fail('duplicate auxiliary recovery')if$aux_recovery{$p};equal($d->{git_path},aux_destination($p),'literal auxiliary recovery path');
  sha_field($d->{sha256});uint($d->{bytes},134217728,'auxiliary bytes');$aux_recovery{$p}=$d;}
 fail('auxiliary recovery population')unless keys(%aux_recovery)==64;
 for my$e(@{$auxiliary->{entries}}){
  my$d;if($e->{type}eq'file'){
   my$p=$e->{path};my$c=$aux_recovery{$p};fail('missing auxiliary recovery')unless$c;
   equal($e->{sha256},$c->{sha256},'auxiliary digest binding');fail('auxiliary size binding')unless$e->{bytes}==$c->{bytes};
   my($b,$id)=git_blob($RETENTION,$c->{git_path});equal(sha256_hex($b),$c->{sha256},'direct auxiliary Git bytes');fail('auxiliary Git size')unless length($b)==$c->{bytes};
   $d={kind=>'git-blob',revision=>$RETENTION,path=>$c->{git_path},git_blob=>$id,bytes=>$c->{bytes},sha256=>$c->{sha256}};
  }
  add_entry($e,$d);
 }
 my($files,$dirs,$total)=(0,0,0);for my$e(values%entries){if($e->{type}eq'file'){$files++;$total+=$e->{bytes}}else{$dirs++}}
 fail('complete selected population mismatch')unless$files==4532&&$dirs==145&&$total==354054095;
 fail('anchor list')unless ref($auxiliary->{anchors})eq'ARRAY'&&@{$auxiliary->{anchors}}==2;
 for my$i(0..1){my$want=$i==0?$REPO:$TARGET;equal($auxiliary->{anchors}[$i]{path},$want,'parent anchor path');hold_directory($want,$auxiliary->{anchors}[$i]);}
 population(1);population(0);
 event('complete_preflight',{regular_files=>$files,directories=>$dirs,logical_bytes=>$total,independent_full_byte_recovery_checks=>JSON::PP::true,
 all_populations_rechecked=>JSON::PP::true,no_python_verdict_used=>JSON::PP::true});
 $mutation_started=1;
 for my$root(@ROOTS){
  my@files=sort grep{$entries{$_}{type}eq'file'&&index($_,"$root/")==0}keys%entries;
  for my$p(@files){remove_file($p);}
  my@dirs=sort{scalar(split m{/},$b)<=>scalar(split m{/},$a)||$b cmp$a}
   grep{$entries{$_}{type}eq'directory'&&($_ eq$root||index($_,"$root/")==0)}keys%entries;
  for my$p(@dirs){parents_unchanged($p);empty_directory("$TARGET/$p");event('rmdir_intent',{path=>"$TARGET/$p"});
   rmdir("$TARGET/$p")or fail("rmdir stopped $p: $!");push@removed_dirs,"$TARGET/$p";absence("$TARGET/$p");event('removed_directory',{path=>"$TARGET/$p",absence=>'ENOENT'});}
  push@completed_roots,"$TARGET/$root";event('root_absent',{path=>"$TARGET/$root",absence=>'ENOENT'});
 }
 for my$p(@TOP){remove_file($p);push@completed_top,"$TARGET/$p";}
 held_identity($REPO);held_identity($TARGET);
 for my$p(@ROOTS,@TOP){absence("$TARGET/$p");}
 fail('unclosed mutation intent')if defined$pending_mutation;
 fail('final removal counts')unless@removed_files==4532&&@removed_dirs==145&&@completed_roots==10&&@completed_top==9;
 event('complete',{regular_files=>scalar@removed_files,directories=>scalar@removed_dirs,directory_roots_absent=>10,literal_files_absent=>9});
 close$log or fail('progress close');$log=undef;
 save_new('result.json',$J->encode({status=>'complete',removed_regular_files=>scalar@removed_files,removed_directories=>scalar@removed_dirs,
 directory_roots_absent=>\@completed_roots,literal_top_level_files_absent=>\@completed_top,logical_bytes=>$total,
 elapsed_seconds=>time-$start,retention_commit=>$RETENTION,cooperative_quiescence_contract=>JSON::PP::true,
 atomic_multi_root_guarantee=>JSON::PP::false,python_executed=>JSON::PP::false})."\n");
 save_new('status',"0\n");
 print"final cleanup complete: 10 directory roots + 9 literal files; 4532 regular files / 354054095 bytes; 145 directories; exact ENOENT checks passed\n";
}
$SIG{TERM}=sub{die"received TERM; stop without retry\n"};
$SIG{INT}=sub{die"received INT; stop without retry\n"};
my$ok=eval{main();1};
if(!$ok){
 my$error=$@||'unknown cleanup failure';chomp$error;
 if($capture_started){
  eval{event('stopped',{error=>$error,mutation_started=>$mutation_started?JSON::PP::true:JSON::PP::false,
   pending_mutation_intent=>$pending_mutation,removed_regular_files=>scalar@removed_files,removed_directories=>scalar@removed_dirs,completed_roots=>\@completed_roots,completed_top_level_files=>\@completed_top});};
  eval{save_new('failure.json',$J->encode({status=>'stopped',error=>$error,mutation_started=>$mutation_started?JSON::PP::true:JSON::PP::false,
   pending_mutation_intent=>$pending_mutation,removed_files=>\@removed_files,removed_directories=>\@removed_dirs,completed_roots=>\@completed_roots,completed_top_level_files=>\@completed_top,
   elapsed_seconds=>time-$start,automatic_retry=>JSON::PP::false})."\n");};
  eval{save_new('status',"1\n");};
 }
 warn"final cleanup refused/stopped: $error\n";exit 1;
}
exit 0;
