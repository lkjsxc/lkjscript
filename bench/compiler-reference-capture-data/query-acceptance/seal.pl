use strict;
use warnings;
use Fcntl qw(:DEFAULT :mode);
use File::Find;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
my $root='/home/coder/workspace/lkjscript/target/compiler-reference-capture-query-acceptance-1';
my @paths;
find({no_chdir=>1,wanted=>sub{
 my $p=$File::Find::name;my @s=lstat $p;die "lstat $p: $!\n" unless @s;
 return if S_ISDIR($s[2]);die "nonregular $p\n" unless S_ISREG($s[2]);
 my $rel=substr($p,length($root)+1);die "existing seal file\n" if $rel eq 'inventory.tsv'||$rel eq 'manifest.sha256';
 push @paths,$rel;
}},$root);
sub raw {open my $f,'<:raw',$_[0] or die $!;local $/;my $b=<$f>;close $f or die $!;return defined($b)?$b:'';}
sub write_new {my($p,$b)=@_;sysopen my $f,$p,O_WRONLY|O_CREAT|O_EXCL,0600 or die "$p: $!\n";binmode$f;print {$f}$b or die $!;close$f or die$!;}
my %row;my $inventory="sha256\tbytes\tpath\n";my $total=0;
for my $rel(sort @paths){my$b=raw("$root/$rel");my$h=sha256_hex($b);$row{$rel}={sha256=>$h,bytes=>length($b)};$total+=length($b);$inventory.="$h\t".length($b)."\t$rel\n";}
write_new("$root/inventory.tsv",$inventory);$row{'inventory.tsv'}={sha256=>sha256_hex($inventory),bytes=>length($inventory)};$total+=length($inventory);
my$manifest=join('',map{"$row{$_}{sha256}  $_\n"}sort keys%row);
write_new("$root/manifest.sha256",$manifest);$total+=length($manifest);
for my$rel(sort keys%row){my$b=raw("$root/$rel");die"seal verification $rel\n" unless sha256_hex($b)eq$row{$rel}{sha256}&&length($b)==$row{$rel}{bytes};}
print JSON::PP->new->canonical->pretty->encode({package=>$root,files=>scalar(keys%row)+1,total_bytes=>$total,manifest_records=>scalar(keys%row),manifest_sha256=>sha256_hex($manifest),inventory_sha256=>sha256_hex($inventory),acceptance_sha256=>$row{'acceptance.json'}{sha256},report_sha256=>$row{'report.md'}{sha256},audit_sha256=>$row{'audit.pl'}{sha256},input_pins_sha256=>$row{'input-pins.json'}{sha256}});
