use strict;
use warnings;
use Cwd qw(abs_path);
use Digest::SHA qw(sha256_hex);
use Errno qw(ENOENT);
use Fcntl qw(O_RDONLY O_WRONLY O_CREAT O_EXCL O_NOFOLLOW :mode);
use File::Find;
use IO::Uncompress::Gunzip;
use IPC::Open2;
use JSON::PP;
use POSIX qw(strftime);
use Time::HiRes qw(time);

# Preparation is not release. This guard has no dry-run deletion mode, process
# observer, credential input, recursive removal helper, or fallback.
my $repo='/home/coder/workspace/lkjscript';
my $target="$repo/target";
my $prefix='bench/compiler-copy-sites-root-cleanup-data';
my $evidence_pin='416da03c86b6073832c9b2cdbb319ff8a3b8c991';
my $baseline_pin='27c94440646d787efc4e7b43a712405288c22c61';
my $inventory_sha='e50bebef5501856268d0cb9459b9dd4465246cf8d49273803c4076c7dc934736';
my $retention_sha='56825fc0ed67a0e772d46404598c0b0dd2d3ff099d05734e977cda0582317a22';
my $seed_ref='7d0731e99aeaf241c4334fc5ff065a341fb388c8:seed/lkjscript.gz';
my $result_dir='/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116/target/compiler-copy-sites-root-inventory-20261116/cleanup-results';
my @scopes=qw(
 compiler-copy-sites-integrated
 compiler-copy-sites-driver-candidate
 compiler-copy-sites-integration-evidence
 compiler-copy-sites-gate.ccpzZ9
 compiler-copy-sites-evidence.20261116-1
 gate
);
my %allowed_scope=map {$_=>1}@scopes;
my %expected_classes=(
 committed_archive=>[15,790718], committed_blob=>[2364,13061795],
 committed_compressed=>[1,28522], committed_seed_decoded=>[2,65169084],
 committed_seed_gzip=>[2,13094690], newly_retained_archive=>[71,259435],
 disposable_generated_authority=>[14,5078655],
 disposable_generated_executable=>[32,255925355],
 disposable_generated_ir=>[394,84222911],
 disposable_generated_source=>[2040,2594546],
 disposable_generated_witness=>[4,39534],
 disposable_http_test_product=>[80,17755510],
 disposable_store=>[16057,321266634],
);
@ARGV==2 && $ARGV[0] eq '--execute-after-root-release' &&
 $ARGV[1]=~/^[0-9a-f]{40}$/ or die "requires explicit root release flag and exact committed bundle revision\n";
my $bundle_commit=$ARGV[1];
die "repository/target alias" unless abs_path($repo) eq $repo && abs_path($target) eq $target;
my @target_identity=lstat($target);
die "target is not a directory" unless @target_identity && S_ISDIR($target_identity[2]);
die "result path already exists; do not repeat cleanup" if -e $result_dir || -l $result_dir;
umask 0077;
mkdir $result_dir,0700 or die "create result directory: $!";
my $started=time;
my $record={
 bundle_commit=>$bundle_commit,evidence_pin=>$evidence_pin,inventory_sha256=>$inventory_sha,
 started_utc=>strftime('%Y-%m-%dT%H:%M:%SZ',gmtime),state=>'preflight',
 deleted_files=>0,deleted_directories=>0,deleted_file_bytes=>0,
 disposable_bytes_recoverable=>JSON::PP::false,
 process_or_fd_observer=>JSON::PP::false,private_key_read=>JSON::PP::false,
 compiler_or_image_executed=>JSON::PP::false,
};
my($git_pid,$reader,$writer);
my(%rows,%classes,%contents,%archive_cache);
my $archive_serial=0;
sub slurp {
 my($p)=@_;sysopen(my$f,$p,O_RDONLY|O_NOFOLLOW)or die"open $p:$!";
 binmode$f;local$/;my$v=<$f>;close$f or die"close $p:$!";return$v//'';
}
sub put {
 my($p,$v)=@_;sysopen(my$f,$p,O_WRONLY|O_CREAT|O_EXCL,0600)or die"create $p:$!";
 binmode$f;print {$f} $v;close$f or die"close output $p:$!";
}
sub git_blob {
 my($ref)=@_;print $writer "$ref\n"or die"Git input:$!";
 my$header=<$reader>;die"Git header"unless defined$header;
 my($oid,$size)=$header=~/^([0-9a-f]{40}) blob ([0-9]+)\n$/;
 die"missing or oversized Git blob $ref"unless defined($size)&&$size<=64*1024*1024;
 my$v='';while(length($v)<$size){
  my$n=read($reader,my$part,$size-length($v));die"truncated Git object"unless defined($n)&&$n;$v.=$part;
 }
 my$end;read($reader,$end,1)==1&&$end eq"\n"or die"Git framing";return$v;
}
sub close_git {
 return unless defined$git_pid;
 close$writer or die"Git input close";close$reader or die"Git output close";
 waitpid($git_pid,0)==$git_pid&&$?==0 or die"Git reader exit";undef$git_pid;
}
sub in_scope {
 my($p)=@_;die"unsafe relative path"unless$p=~/^[A-Za-z0-9_.\/-]+$/&&$p!~m{(?:^|/)\.\.?(?:/|$)};
 my($scope)=split m{/},$p;die"outside literal scopes $p"unless$allowed_scope{$scope};
}
sub check_stat {
 my($p,@s)=@_;die"missing stat $p"unless@s;
 my$type=S_ISREG($s[2])?'f':S_ISDIR($s[2])?'d':'other';
 my@actual=($type,sprintf('%o',$s[2]&07777),@s[4,5,3,7,12,0,1,9,10]);
 my$row=$rows{$p}or die"unexpected entry $p";
 for my$i(0..10){die"changed metadata $p column $i"unless"$actual[$i]"eq"$row->[$i]";}
}
sub verify_file {
 my($p)=@_;my$path="$target/$p";check_stat($p,lstat($path));
 sysopen(my$f,$path,O_RDONLY|O_NOFOLLOW)or die"open live file $p:$!";binmode$f;
 check_stat($p,stat($f));my$sha=Digest::SHA->new(256);$sha->addfile($f);
 die"changed file hash $p"unless$sha->hexdigest eq$rows{$p}[11];
 check_stat($p,stat($f));close$f or die"close live file $p";check_stat($p,lstat($path));
}
sub verify_target {
 my@s=lstat($target);die"target identity changed"unless@s&&S_ISDIR($s[2])&&
 $s[0]==$target_identity[0]&&$s[1]==$target_identity[1]&&$s[2]==$target_identity[2]&&
 $s[4]==$target_identity[4]&&$s[5]==$target_identity[5]&&abs_path($target)eq$target;
}
sub verify_paths {
 verify_target();my%found;
 for my$scope(@scopes){
  my$path="$target/$scope";my@s=lstat($path);
  die"missing or aliased scope $scope"unless@s&&S_ISDIR($s[2])&&abs_path($path)eq$path;
  find({no_chdir=>1,follow=>0,wanted=>sub{
   my$absolute=$File::Find::name;die"discovery escaped target"unless index($absolute,"$target/")==0;
   my$p=substr($absolute,length($target)+1);in_scope($p);
   die"extra/duplicate entry $p"unless exists($rows{$p})&&!$found{$p}++;
   check_stat($p,lstat($absolute));
  }},$path);
 }
 die"missing inventory entries"unless keys(%found)==keys(%rows);
}
sub verify_parents {
 my($p)=@_;verify_target();my@parts=split m{/},$p;pop@parts;
 while(@parts){
  my$parent=join('/',@parts);my@s=lstat("$target/$parent");my$r=$rows{$parent}or die"uninventoried parent";
  die"parent identity changed $parent"unless@s&&S_ISDIR($s[2])&&sprintf('%o',$s[2]&07777)eq$r->[1]&&
   $s[4]==$r->[2]&&$s[5]==$r->[3]&&$s[0]==$r->[7]&&$s[1]==$r->[8];
  pop@parts;
 }
}
sub archive_members {
 my($label,$sha,$bytes)=@_;
 return$archive_cache{$sha}if exists$archive_cache{$sha};
 die"archive bytes identity"unless sha256_hex($bytes)eq$sha;
 my$local="$result_dir/recovery-archive-".(++$archive_serial).".tar.gz";put($local,$bytes);
 open my$list,'-|','tar','--list','--verbose','--numeric-owner','--full-time','--gzip','--file',$local or die"tar list";
 my(%entries,%members);
 while(<$list>){
  chomp;my($mode,$owner,$size,$date,$time,$name)=split/\s+/,$_,6;
  die"archive listing"unless defined($name)&&$size=~/^[0-9]+$/&&substr($mode,0,1)=~/[-d]/;
  my$checked=$name;$checked=~s{^(?:\./)+}{};
  die"unsafe archive member"unless($checked eq''&&substr($mode,0,1)eq'd')||
   ($checked=~/^[A-Za-z0-9_.\/-]+$/&&$checked!~m{^/|(?:^|/)\.\.?(?:/|$)});
  next if substr($mode,0,1)eq'd';
  die"oversized/duplicate archive member"if$size>8*1024*1024||exists$entries{$name};
  $entries{$name}=0+$size;
 }
 close$list or die"tar list failed";
 for my$name(sort keys%entries){
  open my$f,'-|','tar','--extract','--gzip','--to-stdout','--file',$local,'--',$name or die"tar member";
  binmode$f;my$d=Digest::SHA->new(256);my$n=0;
  while(1){my$got=read($f,my$part,65536);die"archive member read"unless defined$got;last unless$got;$n+=$got;die"archive bound"if$n>8*1024*1024;$d->add($part);}
  close$f or die"tar member failed";die"archive member length"unless$n==$entries{$name};
  $members{$name}={bytes=>$n,sha256=>$d->hexdigest};
 }
 $archive_cache{$sha}=\%members;return\%members;
}
sub gzip_digest {
 my($bytes,$bound)=@_;my$g=IO::Uncompress::Gunzip->new(\$bytes)or die"gzip open";
 my$d=Digest::SHA->new(256);my$n=0;
 while(1){my$got=$g->read(my$part,1024*1024);die"gzip read"if$got<0;last unless$got;$n+=$got;die"gzip bound"if$n>$bound;$d->add($part);}
 $g->close or die"gzip close";return($n,$d->hexdigest);
}
sub safe_disposable {
 my($p,$class)=@_;
 return$p=~m{^compiler-copy-sites-gate\.ccpzZ9/store/}if$class eq'disposable_store';
 return$p=~m{^(?:compiler-copy-sites-gate\.ccpzZ9/previous-target-gate|gate)/http-profiles/http\.(?:samples|memory)\.[0-9]+$}if$class eq'disposable_http_test_product';
 return$rows{$p}[13]eq'elf'&&$p=~m{^(?:compiler-copy-sites-gate\.ccpzZ9/previous-target-gate|gate|compiler-copy-sites-integration-evidence)/}if$class eq'disposable_generated_executable';
 return$p=~m{^(?:compiler-copy-sites-gate\.ccpzZ9/previous-target-gate|gate)/generated/.*\.lkj$}if$class eq'disposable_generated_source';
 return$p=~m{^(?:compiler-copy-sites-gate\.ccpzZ9/previous-target-gate|gate)/.*\.lkjir$}if$class eq'disposable_generated_ir';
 return$p=~m{^(?:compiler-copy-sites-gate\.ccpzZ9/previous-target-gate|gate)/.*\.authority$}if$class eq'disposable_generated_authority';
 return$p=~m{^(?:compiler-copy-sites-gate\.ccpzZ9/previous-target-gate|gate)/.*\.rs$}if$class eq'disposable_generated_witness';
 return 0;
}
my$ok=eval{
 system('git','-C',$repo,'merge-base','--is-ancestor',$bundle_commit,'HEAD')==0 or die"bundle is not committed in main history";
 $git_pid=open2($reader,$writer,'git','-C',$repo,'cat-file','--batch');
 binmode$reader;binmode$writer;select((select($writer),$|=1)[0]);
 my$manifest=git_blob("$bundle_commit:$prefix/manifest.sha256");
 for my$line(split/\n/,$manifest){
  my($sha,$name)=$line=~/^([a-f0-9]{64})  ([A-Za-z0-9_.\/-]+)$/;
  die"manifest path"unless defined($name)&&$name!~m{(?:^|/)\.\.?(?:/|$)}&&!exists$contents{$name};
  my$v=git_blob("$bundle_commit:$prefix/$name");die"committed bundle mismatch $name"unless sha256_hex($v)eq$sha;
  $contents{$name}=$v;
 }
 for my$n(qw(guard.pl full-inventory.tsv disposition.tsv recovery.json disposable.json retained-inputs.json retained-unmatched.tar.gz summary.json plan.json)){
  die"bundle lacks $n"unless exists$contents{$n};
 }
 die"guard differs from committed bytes"unless sha256_hex(slurp(__FILE__))eq sha256_hex($contents{'guard.pl'});
 die"inventory identity"unless sha256_hex($contents{'full-inventory.tsv'})eq$inventory_sha;
 my@lines=split/\n/,$contents{'full-inventory.tsv'};
 die"inventory schema"unless shift(@lines)eq join("\t",qw(type mode uid gid links bytes blocks device inode mtime ctime sha256 git_blob hint path));
 my($files,$dirs,$bytes)=(0,0,0);
 for my$line(@lines){
  my@f=split/\t/,$line,-1;die"inventory columns"unless@f==15;my$p=pop@f;in_scope($p);
  die"duplicate inventory"if exists$rows{$p};die"metadata numbers"if grep{$_!~/^[0-9]+$/}@f[1..10];
  if($f[0]eq'f'){
   die"unsafe regular file"unless$f[4]==1&&$f[11]=~/^[a-f0-9]{64}$/&&$f[12]=~/^[a-f0-9]{40}$/;
   $files++;$bytes+=$f[5];
  }elsif($f[0]eq'd'){die"directory digest"unless$f[11]eq'-';$dirs++;}
  else{die"special inventory entry";}
  $rows{$p}=\@f;
 }
 die"inventory totals"unless$files==21076&&$dirs==112&&$bytes==779287389;
 my@dlines=split/\n/,$contents{'disposition.tsv'};die"disposition schema"unless shift(@dlines)eq"class\tbytes\tsha256\tpath";
 my%totals;
 for my$line(@dlines){
  my($c,$n,$sha,$p)=split/\t/,$line,-1;die"disposition class"unless defined($p)&&exists$expected_classes{$c};
  my$r=$rows{$p}or die"disposition path";die"disposition identity"unless$r->[0]eq'f'&&$r->[5]==$n&&$r->[11]eq$sha&&!exists$classes{$p};
  $classes{$p}=$c;$totals{$c}[0]++;$totals{$c}[1]+=$n;
 }
 die"disposition coverage"unless keys(%classes)==21076;
 for my$c(keys%expected_classes){die"class total $c"unless exists($totals{$c})&&$totals{$c}[0]==$expected_classes{$c}[0]&&$totals{$c}[1]==$expected_classes{$c}[1];}
 my%covered;
 for my$item(@{decode_json($contents{'disposable.json'})}){
  my$p=$item->{relative};my$r=$rows{$p}or die"disposable path";
  die"disposable identity"unless$r->[0]eq'f'&&$r->[5]==$item->{bytes}&&$r->[11]eq$item->{sha256}&&$classes{$p}eq$item->{kind}&&!$covered{$p}++;
  die"disposable scope/type"unless safe_disposable($p,$item->{kind});
 }
 my%verified;
 for my$item(@{decode_json($contents{'recovery.json'})}){
  my$p=$item->{relative};my$r=$rows{$p}or die"recovery path";
  die"recovery identity"unless$r->[0]eq'f'&&$r->[5]==$item->{bytes}&&$r->[11]eq$item->{sha256}&&$classes{$p}eq$item->{kind}&&!$covered{$p}++;
  my$kind=$item->{kind};my($n,$sha);
  if($kind eq'committed_blob'){
   my$ref=$item->{reference};
   die"blob recovery pin/path"unless$ref=~m{^(?:\Q$evidence_pin\E|\Q$baseline_pin\E):(src|library|tools|tests|examples|docs|bench|seed)/}&&$ref!~/\.generation-one|(?:^|\/)\.\.(?:\/|$)|[\r\n]/;
   my$v=$verified{$ref}//=git_blob($ref);($n,$sha)=(length($v),sha256_hex($v));
  }elsif($kind eq'committed_archive'){
   my$ref=$item->{archive_reference};
   die"archive recovery pin/path"unless$ref=~m{^\Q$evidence_pin\E:bench/compiler-copy-sites-data/(?:proposals/[A-Za-z0-9_.-]+|source-records/operation-sources)\.tar\.gz$};
   my$v=$verified{$ref}//=git_blob($ref);
   my$members=archive_members($ref,$item->{archive_sha256},$v);
   my$m=$members->{$item->{member}}or die"missing recovery member";($n,$sha)=@{$m}{qw(bytes sha256)};
  }elsif($kind eq'committed_compressed'){
   my$ref=$item->{reference};die"compressed recovery path"unless$ref eq"$evidence_pin:bench/compiler-copy-sites-data/roles.samples.gz";
   my$v=$verified{$ref}//=git_blob($ref);die"compressed identity"unless length($v)==$item->{compressed_bytes}&&sha256_hex($v)eq$item->{compressed_sha256};
   ($n,$sha)=gzip_digest($v,8*1024*1024);
  }elsif($kind eq'committed_seed_gzip'||$kind eq'committed_seed_decoded'){
   die"seed reference"unless$item->{reference}eq$seed_ref;
   my$v=$verified{$seed_ref}//=git_blob($seed_ref);
   die"exact seed gzip"unless length($v)==6547345&&sha256_hex($v)eq'505be947525d53b4db1bff6c85cb77622cb2f5dc7e7d7fe92b772ee3b8ffc3e2';
   if($kind eq'committed_seed_gzip'){($n,$sha)=(length($v),sha256_hex($v));}
   else{($n,$sha)=gzip_digest($v,32584542);die"exact 6cd producer recovery"unless$n==32584542&&$sha eq'6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc';}
  }else{die"unsupported recovery class";}
  die"recovered bytes mismatch $p"unless$n==$r->[5]&&$sha eq$r->[11];
 }
 my$new=$contents{'retained-unmatched.tar.gz'};
 die"new retention compressed identity"unless length($new)==40221&&sha256_hex($new)eq$retention_sha;
 my$members=archive_members('new retention',$retention_sha,$new);
 my$retained=decode_json($contents{'retained-inputs.json'});die"retention cardinality"unless@$retained==71&&keys(%$members)==71;
 for my$item(@$retained){
  my$p=$item->{relative};my$r=$rows{$p}or die"retention path";my$m=$members->{$p}or die"retention member missing";
  die"retention coverage/bytes"unless$r->[0]eq'f'&&$classes{$p}eq'newly_retained_archive'&&!$covered{$p}++&&
   $r->[5]==$item->{bytes}&&$r->[11]eq$item->{sha256}&&$m->{bytes}==$r->[5]&&$m->{sha256}eq$r->[11];
 }
 die"file recovery/disposal coverage"unless keys(%covered)==21076;
 close_git();
 verify_paths();for my$p(sort keys%rows){verify_file($p)if$rows{$p}[0]eq'f';}verify_paths();
 $record->{state}='deleting';$record->{classes}=\%totals;
 for my$p(sort keys%rows){
  next unless$rows{$p}[0]eq'f';verify_parents($p);verify_file($p);
  unlink("$target/$p")==1 or die"unlink approved file $p:$!";
  $record->{deleted_files}++;$record->{deleted_file_bytes}+=$rows{$p}[5];
 }
 my@directories=grep{$rows{$_}[0]eq'd'}keys%rows;
 for my$p(sort{length($b)<=>length($a)||$a cmp$b}@directories){
  verify_parents($p);my@s=lstat("$target/$p");my$r=$rows{$p};
  die"directory identity changed $p"unless@s&&S_ISDIR($s[2])&&$s[0]==$r->[7]&&$s[1]==$r->[8]&&
   $s[4]==$r->[2]&&$s[5]==$r->[3]&&sprintf('%o',$s[2]&07777)eq$r->[1];
  rmdir("$target/$p")or die"rmdir approved directory $p:$!";$record->{deleted_directories}++;
 }
 my@absent;for my$scope(@scopes){
  my@s=lstat("$target/$scope");die"scope remains $scope"if@s;
  die"absence not established $scope:$!"unless$! == ENOENT;
  push@absent,"$target/$scope";
 }
 die"final totals"unless$record->{deleted_files}==21076&&$record->{deleted_directories}==112&&$record->{deleted_file_bytes}==779287389;
 $record->{absent_paths}=\@absent;$record->{state}='complete';1;
};
my$error=$@;
if(defined$git_pid){close$writer;close$reader;waitpid($git_pid,0);}
$record->{finished_utc}=strftime('%Y-%m-%dT%H:%M:%SZ',gmtime);
$record->{elapsed_seconds}=time-$started;$record->{exit_status}=$ok?0:1;$record->{error}=$error unless$ok;
my$output=JSON::PP->new->canonical->pretty->encode($record);
put("$result_dir/result.json",$output);print$output;exit($ok?0:1);
