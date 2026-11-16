use strict;
use warnings;
use Fcntl qw(:DEFAULT :mode);
use File::Find;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
my$root='/home/coder/workspace/lkjscript/target/compiler-reference-capture-setup-acceptance-1';my@paths;
find({no_chdir=>1,wanted=>sub{my$p=$File::Find::name;my@s=lstat$p;die"lstat $p: $!\n"unless@s;return if S_ISDIR($s[2]);die"nonregular $p\n"unless S_ISREG($s[2]);my$r=substr($p,length($root)+1);die"existing seal\n"if$r eq'inventory.tsv'||$r eq'manifest.sha256';push@paths,$r;}},$root);
sub raw{open my$f,'<:raw',$_[0]or die$!;local$/;my$b=<$f>;close$f or die$!;return defined$b?$b:'';}
sub put{my($p,$b)=@_;sysopen my$f,$p,O_WRONLY|O_CREAT|O_EXCL,0600 or die"$p: $!\n";binmode$f;print{$f}$b or die$!;close$f or die$!;}
my%row;my$i="sha256\tbytes\tpath\n";my$total=0;
for my$r(sort@paths){my$b=raw("$root/$r");my$h=sha256_hex($b);$row{$r}={sha256=>$h,bytes=>length$b};$total+=length$b;$i.="$h\t".length($b)."\t$r\n";}
put("$root/inventory.tsv",$i);$row{'inventory.tsv'}={sha256=>sha256_hex($i),bytes=>length$i};$total+=length$i;
my$m=join('',map{"$row{$_}{sha256}  $_\n"}sort keys%row);put("$root/manifest.sha256",$m);$total+=length$m;
for my$r(sort keys%row){my$b=raw("$root/$r");die"seal mismatch $r\n"unless sha256_hex($b)eq$row{$r}{sha256}&&length($b)==$row{$r}{bytes};}
print JSON::PP->new->canonical->pretty->encode({package=>$root,files=>scalar(keys%row)+1,total_bytes=>$total,manifest_records=>scalar(keys%row),manifest_sha256=>sha256_hex($m),inventory_sha256=>sha256_hex($i),acceptance_sha256=>$row{'acceptance.json'}{sha256},report_sha256=>$row{'report.md'}{sha256},audit_sha256=>$row{'audit.pl'}{sha256},original_inventory_sha256=>$row{'original-inventory.json'}{sha256}});
