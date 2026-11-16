#!/usr/bin/env perl
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use File::Path qw(make_path);
my$root="/home/coder/workspace/lkjscript-compiler-scalar-20261116";
chdir$root or die"$!\n";
my$out="target/compiler-scalar-gate-acceptance-1";
sub read_file{
    my$p=shift;die"not regular $p\n"unless-f$p&&!-l$p;
    open my$f,"<:raw",$p or die"$p: $!\n";local$/;my$b=<$f>;close$f or die"$!\n";return$b;
}
sub put{
    my($p,$b)=@_;die"existing output $p\n"if-e$p||-l$p;
    open my$f,">:raw",$p or die"$p: $!\n";print {$f} $b;close$f or die"$!\n";
}
my$manifest=read_file("$out/input-inventory.tsv");
die"input inventory identity\n"unless sha256_hex($manifest)eq"7cbf67bf9d3e14a673ba8abb8ac933a8178366e15e85073c4b360beb3eb09f25";
my%files;
for my$l(split/\n/,$manifest){
    next if$l eq"path\tbytes\tsha256";
    my($path,$size,$sha)=split/\t/,$l,-1;
    next unless$path=~m{\Atarget/compiler-scalar-cold-gate-1/[^/]+\z}||$path=~m{\Atarget/gate/[^/]+\.log\z};
    die"inventory row\n"unless$size=~/\A[0-9]+\z/&&$sha=~/\A[0-9a-f]{64}\z/&&!exists$files{$path};
    $files{$path}={bytes=>0+$size,sha256=>$sha};
}
my@cold=sort grep{m{\Atarget/compiler-scalar-cold-gate-1/}}keys%files;
my@logs=sort grep{m{\Atarget/gate/}}keys%files;
die"inventory counts\n"unless@cold==20&&@logs==123;
for my$spec(["target/compiler-scalar-cold-gate-1",\@cold,0],["target/gate",\@logs,1]){
    my($dir,$wanted,$only_logs)=@$spec;opendir my$d,$dir or die"$!\n";
    my@names=sort map{"$dir/$_"}grep{$_ ne"."&&$_ ne".."&&(!$only_logs||/\.log\z/)}readdir$d;closedir$d;
    die"live path set $dir\n"unless join("\0",@names)eq join("\0",@$wanted);
}
die"copy scope exists\n"if-e"$out/captures"||-l"$out/captures";
make_path("$out/captures/target/compiler-scalar-cold-gate-1","$out/captures/target/gate");
my$inventory="member\tsource\tmode\tbytes\tsha256\n";my$members="";my$total=0;
for my$p(sort keys%files){
    my$b=read_file($p);my$r=$files{$p};
    die"live identity $p\n"unless length($b)==$r->{bytes}&&sha256_hex($b)eq$r->{sha256};
    my$mode=(stat($p))[2]&0777;
    my$dest="$out/captures/$p";put($dest,$b);chmod$mode,$dest or die"chmod $dest: $!\n";
    my$copied=read_file($dest);
    die"copy mismatch $p\n"unless length($copied)==length($b)&&sha256_hex($copied)eq$r->{sha256};
    $inventory.=join("\t",$p,$p,sprintf("%04o",$mode),length($b),$r->{sha256})."\n";
    $members.="$p\0";$total+=length($b);
}
put("$out/captures-inventory.tsv",$inventory);
put("$out/archive-members.nul",$members);
put("$out/capture-copy-verification.txt","cold_capture_files=20\ngate_log_files=123\nregular_members=143\npayload_bytes=$total\ninventory_sha256=".sha256_hex($inventory)."\n");
print"copied and verified143 regular files, $total payload bytes; no fixtures/stores/keys/binaries included\n";
