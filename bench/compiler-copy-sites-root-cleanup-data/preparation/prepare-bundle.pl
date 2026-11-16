use strict;
use warnings;
use Cwd qw(abs_path);
use Digest::SHA qw(sha256_hex);
use Fcntl qw(O_RDONLY O_WRONLY O_CREAT O_EXCL O_NOFOLLOW :mode);
use JSON::PP;
use POSIX qw(strftime);
my $out='/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116/target/compiler-copy-sites-root-inventory-20261116';
my $bundle="$out/proposed-cleanup";
my $target='/home/coder/workspace/lkjscript/target';
my $pin='416da03c86b6073832c9b2cdbb319ff8a3b8c991';
my @scopes=qw(compiler-copy-sites-integrated compiler-copy-sites-driver-candidate compiler-copy-sites-integration-evidence compiler-copy-sites-gate.ccpzZ9 compiler-copy-sites-evidence.20261116-1 gate);
my %scopes=map {$_=>1}@scopes;
umask 0077;
sub bytes {sysopen(my $f,$_[0],O_RDONLY|O_NOFOLLOW)or die "$_[0]:$!";binmode$f;local$/;my$v=<$f>;close$f or die$!;return$v//'';}
sub data {return decode_json(bytes($_[0]));}
sub put {my($p,$v)=@_;sysopen(my$f,$p,O_WRONLY|O_CREAT|O_EXCL,0600)or die "$p:$!";binmode$f;print {$f} $v;close$f or die$!;}
my $json=JSON::PP->new->canonical->pretty;
my $inventory=bytes("$out/full-inventory.tsv");
my @lines=split /\n/,$inventory;
my $header=shift @lines;
die "inventory schema" unless $header eq join("\t",qw(type mode uid gid links bytes blocks device inode mtime ctime sha256 git_blob hint path));
my(%rows,@order);
for my $line(@lines){
 my@f=split/\t/,$line,-1;die "inventory columns"unless@f==15;
 my$p=pop@f;die "unsafe path"unless$p=~/^[A-Za-z0-9_.\/-]+$/&&$p!~m{(?:^|/)\.\.?(?:/|$)};
 my($scope)=split m{/},$p;die "scope"unless$scopes{$scope};die"duplicate"if exists$rows{$p};
 $rows{$p}=\@f;push@order,$p;
}
my(%recovery,%retained);
for my $name(qw(verified-git-counterparts.json archive-counterparts.json supplemental-counterparts.json)){
 for my$r(@{data("$out/$name")}){
  my$p=$r->{relative};next if exists$recovery{$p};
  my$row=$rows{$p}or die"recovery path";die"recovery mismatch"unless$row->[0]eq'f'&&$row->[5]==$r->{bytes}&&$row->[11]eq$r->{sha256};
  $recovery{$p}={%$r,kind=>$r->{archive_reference}?'committed_archive':$r->{transform}&&$r->{transform}eq'gzip decompression'?'committed_compressed':'committed_blob'};
 }
}
for my$r(@{data("$out/seed-recovery.json")}){
 my$p=$r->{relative};$recovery{$p}={%$r,kind=>$r->{transform}eq'identity'?'committed_seed_gzip':'committed_seed_decoded'};
}
for my$r(@{data("$out/unretained-observations-candidates.json")}){
 my$p=$r->{path};next if$recovery{$p}||$p=~m{/http-profiles/};
 my$row=$rows{$p}or die"retained path";
 die"retained input mismatch"unless$row->[0]eq'f'&&$row->[5]==$r->{bytes}&&$row->[11]eq$r->{sha256};
 $retained{$p}={relative=>$p,bytes=>$r->{bytes},sha256=>$r->{sha256}};
}
die"root retention count"unless keys(%retained)==71;
my$retained_bytes=0;$retained_bytes+=$_->{bytes}for values%retained;
die"root retention size"unless$retained_bytes==259435;
sub verify_live{
 my($p)=@_;my$row=$rows{$p};my$path="$target/$p";
 for my$stage(0,1){
  my@s=lstat($path);die"live file type"unless@s&&S_ISREG($s[2]);
  my@v=('f',sprintf('%o',$s[2]&07777),@s[4,5,3,7,12,0,1,9,10]);
  for my$i(0..10){die"live metadata $p $i"unless"$v[$i]"eq"$row->[$i]";}
  next if$stage;
  sysopen(my$f,$path,O_RDONLY|O_NOFOLLOW)or die$!;binmode$f;
  my@held=stat($f);die"live descriptor identity"unless$held[0]==$s[0]&&$held[1]==$s[1];
  my$sha=Digest::SHA->new(256);$sha->addfile($f);die"live digest $p"unless$sha->hexdigest eq$row->[11];close$f or die$!;
 }
}
verify_live($_)for sort keys%retained;
die"target alias"unless abs_path($target)eq$target;
mkdir$bundle,0700 or die"bundle exists: $!";
put("$bundle/full-inventory.tsv",$inventory);
put("$bundle/retained-inputs.json",$json->encode([map{$retained{$_}}sort keys%retained]));
put("$out/retained-files.nul",join("\0",sort keys%retained)."\0");
my$archive="$bundle/retained-unmatched.tar.gz";
system('bash','-o','pipefail','-c','tar --directory "$1" --null --no-recursion --files-from "$2" --sort=name --format=ustar --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - | gzip -n -9 > "$3"','compiler-retain',$target,"$out/retained-files.nul",$archive)==0 or die"archive create";
my$compressed=bytes($archive);
open my$list,'-|','tar','--list','--verbose','--numeric-owner','--full-time','--gzip','--file',$archive or die$!;
my%members;
while(<$list>){
 chomp;my($mode,$owner,$size,$date,$time,$name)=split/\s+/,$_,6;
 die"new archive shape"unless defined($name)&&substr($mode,0,1)eq'-'&&$size=~/^[0-9]+$/&&exists$retained{$name}&&!$members{$name}++;
 die"archive listed size"unless$size==$retained{$name}{bytes};
}
close$list or die"archive list";
die"archive member count"unless keys(%members)==71;
for my$p(sort keys%members){
 open my$f,'-|','tar','--extract','--gzip','--to-stdout','--file',$archive,'--',$p or die$!;binmode$f;
 my$sha=Digest::SHA->new(256);my$size=0;
 while(1){my$n=read($f,my$part,65536);die"member read"unless defined$n;last unless$n;$size+=$n;$sha->add($part);}
 close$f or die"member close";
 die"archive exact content"unless$size==$retained{$p}{bytes}&&$sha->hexdigest eq$retained{$p}{sha256};
 verify_live($p);
}
my(%stats,%scope_stats,@disposition,@recoverable,@disposable);
for my$p(sort@order){
 my$r=$rows{$p};my($scope)=split m{/},$p;
 my$kind;
 if($r->[0]eq'd'){$scope_stats{$scope}{directories}++;next;}
 die"regular type"unless$r->[0]eq'f';
 if($retained{$p}){$kind='newly_retained_archive';}
 elsif($p=~m{^compiler-copy-sites-gate\.ccpzZ9/store/}){$kind='disposable_store';}
 elsif($recovery{$p}){$kind=$recovery{$p}{kind};push@recoverable,$recovery{$p};}
 elsif($p=~m{^(?:compiler-copy-sites-gate\.ccpzZ9/previous-target-gate|gate)/http-profiles/http\.(?:samples|memory)\.[0-9]+$}){$kind='disposable_http_test_product';}
 elsif($r->[13]eq'elf'){$kind='disposable_generated_executable';}
 elsif($p=~m{^(?:compiler-copy-sites-gate\.ccpzZ9/previous-target-gate|gate)/generated/.*\.lkj$}){$kind='disposable_generated_source';}
 elsif($p=~m{^(?:compiler-copy-sites-gate\.ccpzZ9/previous-target-gate|gate)/.*\.lkjir$}){$kind='disposable_generated_ir';}
 elsif($p=~m{^(?:compiler-copy-sites-gate\.ccpzZ9/previous-target-gate|gate)/.*\.authority$}){$kind='disposable_generated_authority';}
 elsif($p=~m{^(?:compiler-copy-sites-gate\.ccpzZ9/previous-target-gate|gate)/.*\.rs$}){$kind='disposable_generated_witness';}
 else{die"unclassified retained/disposable file $p";}
 $stats{$kind}{files}++;$stats{$kind}{bytes}+=$r->[5];
 $scope_stats{$scope}{files}++;$scope_stats{$scope}{bytes}+=$r->[5];
 $scope_stats{$scope}{classes}{$kind}{files}++;$scope_stats{$scope}{classes}{$kind}{bytes}+=$r->[5];
 push@disposition,join("\t",$kind,$r->[5],$r->[11],$p);
 push@disposable,{relative=>$p,bytes=>0+$r->[5],sha256=>$r->[11],kind=>$kind}if$kind=~/^disposable_/;
}
my($n,$b,$dirs)=(0,0,0);
for my$s(values%scope_stats){$n+=$s->{files};$b+=$s->{bytes};$dirs+=$s->{directories};}
die"partition totals"unless$n==21076&&$b==779287389&&$dirs==112;
die"HTTP test products"unless$stats{disposable_http_test_product}{files}==80&&$stats{disposable_http_test_product}{bytes}==17755510;
die"store products"unless$stats{disposable_store}{files}==16057&&$stats{disposable_store}{bytes}==321266634;
put("$bundle/disposition.tsv",join("\n","class\tbytes\tsha256\tpath",@disposition)."\n");
put("$bundle/recovery.json",$json->encode(\@recoverable));
put("$bundle/disposable.json",$json->encode(\@disposable));
my$archive_record={path=>'retained-unmatched.tar.gz',sha256=>(sha256_hex($compressed)),bytes=>length($compressed),regular_members=>71,member_bytes=>259435,format=>'ustar, sorted exact file list, mtime=0 uid=0 gid=0, gzip -n -9',source_comparison=>'Every member SHA-256 and size exactly matches live input and initial inventory; live metadata checked before and after retention.'};
put("$bundle/retention-verification.json",$json->encode($archive_record));
my$summary={inventory_pin=>$pin,prepared_utc=>strftime('%Y-%m-%dT%H:%M:%SZ',gmtime),target=>$target,scopes=>\@scopes,files=>$n,directories=>$dirs,logical_bytes=>$b,allocated_bytes=>846192640,inventory_sha256=>(sha256_hex($inventory)),retention=>$archive_record,classes=>\%stats,scope_summary=>\%scope_stats,production_source_or_compiler_executed=>JSON::PP::false,process_or_fd_observer=>JSON::PP::false,private_key_read=>JSON::PP::false,deleted=>JSON::PP::false};
put("$bundle/summary.json",$json->encode($summary));
print $json->encode($summary);
