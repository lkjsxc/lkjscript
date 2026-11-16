use strict;
use warnings;
use Fcntl qw(:DEFAULT :mode);
use File::Find;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
my $root='/home/coder/workspace/lkjscript';
my $pkg="$root/target/compiler-scalar-cohort-package-1";
my $proposal="$root/target/compiler-scalar-cohort-retention-proposal-1";
my $json=JSON::PP->new->canonical->pretty;
chdir $root or die "cwd: $!";
sub open_regular {
 my($p)=@_;sysopen my$f,$p,O_RDONLY|O_NOFOLLOW or die"open $p: $!";
 my@s=stat($f);die"not regular: $p\n"unless@s&&S_ISREG($s[2]);binmode$f;return$f;
}
sub raw {my$f=open_regular(shift);local$/;my$b=<$f>;close$f or die"close: $!";return defined($b)?$b:'';}
sub hash_file {
 my($p)=@_;my$f=open_regular($p);my@s=stat($f);my$d=Digest::SHA->new(256);$d->addfile($f);close$f or die"close $p: $!";
 return($d->hexdigest,$s[7],$s[2]&07777);
}
sub exact {
 my($p,$h,$n)=@_;my($a,$b)=hash_file($p);die"hash/size mismatch: $p\n"unless$a eq$h&&(!defined($n)||$b==$n);return$b;
}
sub save {
 my($p,$b,$mode)=@_;$mode=0600 unless defined$mode;
 sysopen my$f,$p,O_WRONLY|O_CREAT|O_EXCL,$mode or die"create $p: $!";binmode$f;print{$f}$b or die"write $p: $!";
 close$f or die"close $p: $!";chmod$mode,$p or die"chmod $p: $!";
}
sub save_json {my($p,$v)=@_;save($p,$json->encode($v));}
sub sq {my$s=shift;$s=~s/'/'\\''/g;return"'$s'";}
exact("$proposal/manifest.sha256",'1a93aa925b1c9037023eb1ef80ec5fb323ba351384bec7d32d4a0d8cddc3fc3f',1323);
exact("$proposal/inventory.json",'235e976799d2640bf6599068204d7955bd359673696b3e657387b021f822a070',254250);
exact("$proposal/later-artifacts.json",'5f99c5746ce18811468904ff4ff6a979fa994634f242df518a2076abaed6e1de',3145);
my$inv=decode_json(raw("$proposal/inventory.json"));my$policy=decode_json(raw("$proposal/later-artifacts.json"));
die"inventory counts\n"unless$inv->{total_files}==337&&$inv->{total_bytes}==207027139;
my%all=map{$_->{path}=>$_}@{$inv->{files}};
die"inventory duplicate\n"unless keys(%all)==337;
my%records=map{$_->{path}=>$_}grep{$_->{classification}eq'retain-raw-record'}@{$inv->{files}};
die"raw records\n"unless keys(%records)==154;
sub check_scopes {
 my%seen;
 for my$s(@{$inv->{scopes}}){
  my$base="$root/$s->{path}";my@top=lstat($base);die"scope not directory $base\n"unless@top&&S_ISDIR($top[2]);
  find({no_chdir=>1,follow=>0,wanted=>sub{
   my$p=$File::Find::name;my@st=lstat($p);die"lstat $p\n"unless@st;
   return if S_ISDIR($st[2]);die"nonregular $p\n"unless S_ISREG($st[2]);
   my$r=substr($p,length($root)+1);die"unknown/duplicate $r\n"unless exists$all{$r}&&!$seen{$r}++;
   my$e=$all{$r};die"size/mode changed $r\n"unless$st[7]==$e->{bytes}&&sprintf('%04o',$st[2]&07777)eq$e->{mode};
   exact($p,$e->{sha256},$e->{bytes});
  }},$base);
 }
 die"scope population\n"unless keys(%seen)==337;
}
check_scopes();
mkdir"$pkg/retention-proposal",0700 or die"mkdir proposal: $!";
my%props;
for my$l(split/\n/,raw("$proposal/manifest.sha256")){
 die"proposal manifest framing\n"unless$l=~/\A([0-9a-f]{64})  ([A-Za-z0-9._-]+)\z/;
 die"duplicate proposal\n"if exists$props{$2};$props{$2}=$1;
}
$props{'manifest.sha256'}='1a93aa925b1c9037023eb1ef80ec5fb323ba351384bec7d32d4a0d8cddc3fc3f';
opendir my$pd,$proposal or die"proposal dir: $!";my@pn=sort grep{$_ ne'.'&&$_ ne'..'}readdir$pd;closedir$pd;
die"proposal population mismatch\n"unless@pn==18&&join("\0",@pn)eq join("\0",sort keys%props);
my@copies;
for my$n(@pn){
 my($h,$bytes,$mode)=hash_file("$proposal/$n");die"proposal source changed $n\n"unless$h eq$props{$n};
 my$b=raw("$proposal/$n");die"proposal read changed $n\n"unless sha256_hex($b)eq$h;
 save("$pkg/retention-proposal/$n",$b,$mode);
 push@copies,{source=>"target/compiler-scalar-cohort-retention-proposal-1/$n",copy=>"retention-proposal/$n",sha256=>$h,bytes=>$bytes};
}
save_json("$pkg/proposal-copies.json",\@copies);
save("$pkg/raw-members.nul",join('',map{"$_\0"}sort keys%records));
save("$pkg/raw-members.sha256",join('',map{"$records{$_}{sha256}  $_\n"}sort keys%records));
die"approved member list changed\n"unless raw("$pkg/raw-members.nul")eq raw("$proposal/raw-members.nul")&&raw("$pkg/raw-members.sha256")eq raw("$proposal/raw-members.sha256");
mkdir"$pkg/artifacts",0700 or die"mkdir artifacts: $!";
my@commands=({kind=>'raw-records',shell=>join(' ','/usr/bin/tar','--format=ustar','--mtime=@0','--owner=0','--group=0','--numeric-owner','--hard-dereference','--no-recursion','--null','--verbatim-files-from','-C',sq($root),'-T',sq("$pkg/raw-members.nul"),'-cf','-','|','/usr/bin/gzip','-n','-9','>',sq("$pkg/raw-cohort-records.tar.gz"))});
for my$a(@{$policy->{new_gzips}}){
 exact("$root/$a->{source}",$a->{original_sha256},$a->{original_bytes});
 push@commands,{kind=>$a->{source},shell=>'/usr/bin/gzip -n -9 -c -- '.sq("$root/$a->{source}").' > '.sq("$pkg/$a->{artifact}")};
}
save_json("$pkg/commands.json",\@commands);
for my$c(@commands){print"create $c->{kind}\n";my$s=system('/bin/bash','-o','pipefail','-o','noclobber','-c',$c->{shell});die"creation failed ($s) $c->{kind}\n"unless$s==0;}
sub take {
 my($f,$n)=@_;my$b='';while(length($b)<$n){my$r=read($f,my$v,$n-length($b));die"read: $!"unless defined$r;die"unexpected EOF\n"unless$r;$b.=$v;}return$b;
}
sub octal {my$v=shift;$v=~s/\0.*\z//s;$v=~s/^\s+|\s+$//g;die"bad tar octal\n"unless$v=~/\A[0-7]{1,12}\z/;return oct($v);}
sub tar_text {my$v=shift;$v=~s/\0.*\z//s;return$v;}
sub verify_tar {
 my($archive,$expected,$compare_base,$normalized,$manifest)=@_;
 open my$z,'-|','/usr/bin/gzip','-dc','--',$archive or die"gzip open: $!";binmode$z;
 my(%seen,%dirs);my@meta;my@rows;my$total=0;
 while(1){
  my$h=take($z,512);
  if($h eq"\0"x512){die"tar end framing\n"unless take($z,512)eq"\0"x512;while(1){my$n=read($z,my$b,65536);die"tar tail read: $!"unless defined$n;last unless$n;die"tar nonzero tail\n"if$b=~/[^\0]/;}last;}
  my$checksum=octal(substr($h,148,8));my$copy=$h;substr($copy,148,8)=' 'x8;
  die"tar header checksum\n"unless unpack('%32C*',$copy)==$checksum;
  die"tar ustar marker\n"unless substr($h,257,6)eq"ustar\0";
  my$name=tar_text(substr($h,0,100));my$prefix=tar_text(substr($h,345,155));$name="$prefix/$name"if length$prefix;
  my$type=substr($h,156,1);my$n=octal(substr($h,124,12));my$mode=octal(substr($h,100,8));
  if(!$normalized&&$type eq'g'){
   die"unexpected global metadata\n"unless$name eq'pax_global_header'&&$n==52&&!@meta;
   my$b=take($z,$n);die"source archive Git identity\n"unless$b eq"52 comment=26bf2c3c021dd79810a81fa1d9054d59f3768b06\n";
   my$pad=(512-($n%512))%512;die"metadata padding\n"if$pad&&take($z,$pad)ne"\0"x$pad;
   push@meta,{name=>$name,bytes=>$n,text=>$b};next;
  }
  if(!$normalized&&$type eq'5'){
   die"directory size/name\n"unless$n==0&&$name=~m{/\z}&&$name!~m{(?:^|/)\.\.?(?:/|$)};
   die"directory not an expected file parent\n"unless grep{index($_,$name)==0}keys%$expected;
   die"duplicate directory\n"if$dirs{$name}++;next;
  }
  die"tar nonregular member\n"unless$type eq'0'||$type eq"\0";
  die"unexpected/duplicate member $name\n"unless exists$expected->{$name}&&!$seen{$name}++;
  my$e=$expected->{$name};die"tar member length $name\n"unless$n==$e->{bytes};
  if($normalized){
   die"raw mode changed\n"unless sprintf('%04o',$mode&07777)eq$e->{mode};
   die"raw normalized metadata\n"unless octal(substr($h,108,8))==0&&octal(substr($h,116,8))==0&&octal(substr($h,136,12))==0;
  }
  my$o=open_regular("$compare_base/$name");my$sha=Digest::SHA->new(256);my$left=$n;
  while($left){my$want=$left>65536?65536:$left;my$b=take($z,$want);die"member byte mismatch $name\n"unless take($o,$want)eq$b;$sha->add($b);$left-=$want;}
  my$extra=read($o,my$tail,1);die"member source read\n"unless defined$extra;die"member source extra bytes\n"if$extra;close$o or die"close original\n";
  my$pad=(512-($n%512))%512;die"member padding\n"if$pad&&take($z,$pad)ne"\0"x$pad;
  my$actual=$sha->hexdigest;die"member hash $name\n"unless$actual eq$e->{sha256};
  push@rows,{path=>$name,sha256=>$actual,bytes=>$n,archive_mode=>sprintf('%04o',$mode&07777),reference_mode=>$e->{mode},byte_equal=>JSON::PP::true};$total+=$n;
 }
 close$z or die"gzip CRC/exit: $?";die"archive population\n"unless keys(%seen)==keys(%$expected);
 die"missing source archive identity\n"if!$normalized&&@meta!=1;
 @rows=sort{$a->{path}cmp$b->{path}}@rows;save($manifest,join('',map{"$_->{sha256}  $_->{path}\n"}@rows));
 my($h,$size)=hash_file($archive);
 return{artifact=>substr($archive,length($pkg)+1),sha256=>$h,bytes=>$size,members=>scalar(@rows),member_bytes=>$total,directory_headers=>scalar(keys%dirs),metadata=>\@meta,all_members_byte_equal=>JSON::PP::true,rows=>\@rows};
}
my$raw_verified=verify_tar("$pkg/raw-cohort-records.tar.gz",\%records,$root,1,"$pkg/verified-raw-members.sha256");
die"raw verified manifest mismatch\n"unless raw("$pkg/raw-members.sha256")eq raw("$pkg/verified-raw-members.sha256");
sub verify_gzip {
 my($compressed,$original,$expected_sha,$expected_bytes)=@_;
 open my$z,'-|','/usr/bin/gzip','-dc','--',$compressed or die"gzip: $!";binmode$z;my$o=open_regular($original);
 my$d=Digest::SHA->new(256);my$total=0;
 while(1){my$n=read($z,my$b,65536);die"decompression read: $!"unless defined$n;last unless$n;die"gzip original bytes differ\n"unless take($o,$n)eq$b;$d->add($b);$total+=$n;}
 my$extra=read($o,my$tail,1);die"source read\n"unless defined$extra;die"original trailing bytes\n"if$extra;close$o or die"original close\n";
 close$z or die"gzip CRC/exit $?";die"expanded pin mismatch\n"unless$total==$expected_bytes&&$d->hexdigest eq$expected_sha;
 return{original_sha256=>$d->hexdigest,original_bytes=>$total,full_bytes_equal=>JSON::PP::true};
}
my@new;
for my$a(@{$policy->{new_gzips}}){
 my$r=verify_gzip("$pkg/$a->{artifact}","$root/$a->{source}",$a->{original_sha256},$a->{original_bytes});
 my($h,$size)=hash_file("$pkg/$a->{artifact}");$r->{artifact}=$a->{artifact};$r->{compressed_sha256}=$h;$r->{compressed_bytes}=$size;
 push@new,$r;print"verified $a->{source} original=$r->{original_bytes} compressed=$size\n";
}
my$source_prefix='target/compiler-scalar-simplify.20261116-1/fixture/source/';
my%sources;
for my$r(@{$inv->{files}}){next unless$r->{classification}eq'source-tree-duplicate';die"source prefix\n"unless index($r->{path},$source_prefix)==0;$sources{substr($r->{path},length($source_prefix))}=$r;}
die"source population\n"unless keys(%sources)==172;
my$shared=$policy->{shared_output_recovery};
my$source_verified=verify_tar("$pkg/$shared->{artifact}",\%sources,"$root/target/compiler-scalar-simplify.20261116-1/fixture/source",0,"$pkg/verified-source-members.sha256");
die"embedded seed digest\n"unless$sources{$shared->{member}}{sha256}eq$shared->{member_sha256};
my$seed_cmd='/usr/bin/gzip -dc -- '.sq("$pkg/$shared->{artifact}").' | /usr/bin/tar -xOf - -- '.sq($shared->{member}).' | /usr/bin/gzip -dc';
save_json("$pkg/shared-seed-command.json",{argv=>['/bin/bash','-o','pipefail','-c',$seed_cmd],outputs=>$shared->{paths},no_output_file_created=>JSON::PP::true});
open my$seed,'-|','/bin/bash','-o','pipefail','-c',$seed_cmd or die"seed recovery: $!";binmode$seed;
my@outputs=map{open_regular("$root/$_")}@{$shared->{paths}};die"seven outputs required\n"unless@outputs==7;
my$seed_hash=Digest::SHA->new(256);my$seed_bytes=0;
while(1){my$n=read($seed,my$b,65536);die"seed read: $!"unless defined$n;last unless$n;for my$o(@outputs){die"shared seed/output byte mismatch\n"unless take($o,$n)eq$b;}$seed_hash->add($b);$seed_bytes+=$n;}
for my$o(@outputs){my$extra=read($o,my$t,1);die"output read\n"unless defined$extra;die"extra output bytes\n"if$extra;close$o or die"output close\n";}
close$seed or die"seed pipeline/CRC status $?";
die"shared seed pin\n"unless$seed_bytes==$shared->{original_bytes}&&$seed_hash->hexdigest eq$shared->{original_sha256};
my$e=$policy->{existing_emitter_gzip};
my$ef=open_regular("$root/$e->{path}");my@es=stat($ef);die"E gzip size\n"unless$es[7]==$e->{bytes};
my$ed=Digest::SHA->new(256);my$gitsha=Digest::SHA->new(1);$gitsha->add("blob $es[7]\0");
while(1){my$n=read($ef,my$b,65536);die"E gzip read: $!"unless defined$n;last unless$n;$ed->add($b);$gitsha->add($b);}
close$ef or die"E gzip close\n";die"E compressed pin\n"unless$ed->hexdigest eq$e->{sha256};
die"E committed Git blob mismatch\n"unless$gitsha->hexdigest eq'95a68fd1c3c298e3686138bb28ad6bf63036e8e2';
my@emitter=grep{$_->{classification}eq'existing-emitter-duplicate'}@{$inv->{files}};die"emitter population\n"unless@emitter==1;
my$e_verified=verify_gzip("$root/$e->{path}","$root/$emitter[0]{path}",$e->{original_sha256},$e->{original_bytes});
$e_verified->{existing_path}=$e->{path};$e_verified->{compressed_sha256}=$ed->hexdigest;$e_verified->{compressed_bytes}=$es[7];$e_verified->{git_blob}=$gitsha->hexdigest;
$e_verified->{commit}='330272b97ddba28b376ce3c44f57b04c195991be';$e_verified->{copied}=JSON::PP::false;
check_scopes();
for my$c(@copies){exact("$root/$c->{source}",$c->{sha256},$c->{bytes});exact("$pkg/$c->{copy}",$c->{sha256},$c->{bytes});}
save_json("$pkg/verification.json",{
 schema=>1,status=>'passed',committed_result=>'330272b97ddba28b376ce3c44f57b04c195991be',
 scope_files=>337,scope_bytes=>207027139,raw=>$raw_verified,new_gzips=>\@new,
 source_archive=>$source_verified,existing_emitter=>$e_verified,
 shared_seed=>{sha256=>$seed_hash->hexdigest,bytes=>$seed_bytes,embedded_compressed_sha256=>$shared->{member_sha256},outputs=>$shared->{paths},all_seven_byte_equal=>JSON::PP::true,no_expanded_seed_or_output_copy=>JSON::PP::true},
 proposal_copy_files=>18,all_original_inputs_unchanged=>JSON::PP::true,
 no_compiler_native_performance=>JSON::PP::true,no_deletion=>JSON::PP::true,
 acceptance=>$policy->{acceptance}
});
print"PASS raw=154 source_members=172 new_gzips=3 existing_E=verified_once shared_seed_outputs=7 proposal_copies=18\n";
