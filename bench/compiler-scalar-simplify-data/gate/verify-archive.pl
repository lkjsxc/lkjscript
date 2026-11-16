#!/usr/bin/env perl
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use JSON::PP ();
my$root="/home/coder/workspace/lkjscript-compiler-scalar-20261116";
chdir$root or die"$!\n";
my$out="target/compiler-scalar-gate-acceptance-1";
my$archive="$out/gate-captures.tar.gz";
sub read_file{
    my$p=shift;die"not regular $p\n"unless-f$p&&!-l$p;
    open my$f,"<:raw",$p or die"$p: $!\n";local$/;my$b=<$f>;close$f or die"$!\n";return$b;
}
sub put{
    my($p,$b)=@_;die"existing output $p\n"if-e$p||-l$p;
    open my$f,">:raw",$p or die"$p: $!\n";print {$f} $b;close$f or die"$!\n";
}
my$inventory=read_file("$out/captures-inventory.tsv");
my%wanted;
for my$l(split/\n/,$inventory){
    next if$l eq"member\tsource\tmode\tbytes\tsha256";
    my($member,$source,$mode,$size,$sha)=split/\t/,$l,-1;
    die"inventory format\n"unless defined$sha&&$member eq$source&&$mode=~/\A[0-7]{4}\z/&&$size=~/\A[0-9]+\z/&&$sha=~/\A[0-9a-f]{64}\z/&&!exists$wanted{$member};
    $wanted{$member}={source=>$source,mode=>oct($mode),bytes=>0+$size,sha256=>$sha};
}
die"member count\n"unless keys(%wanted)==143;
open my$pipe,"-|","gzip","-dc",$archive or die"gzip launch: $!\n";
local$/;my$tar=<$pipe>;close$pipe or die"gzip decode failed\n";
sub field{my$s=shift;$s=~s/\0.*\z//s;return$s}
sub number{my$s=shift;$s=~s/\A +//;$s=~s/[\0 ]+\z//;die"non-octal tar field\n"unless$s=~/\A[0-7]*\z/;return length($s)?oct($s):0}
my$at=0;my%seen;my@members;my$total=0;my$end=0;
while($at+512<=length$tar){
    my$h=substr($tar,$at,512);
    if($h eq"\0"x512){
        my$tail=substr($tar,$at);
        die"tar trailing bytes\n"unless length($tail)>=1024&&length($tail)%512==0&&$tail!~/[^\0]/;
        $end=1;last;
    }
    die"tar format\n"unless substr($h,257,6)eq"ustar\0"&&substr($h,263,2)eq"00";
    my$checksum=number(substr($h,148,8));my$hc=$h;substr($hc,148,8)=" "x8;
    die"tar checksum\n"unless unpack("%32C*",$hc)==$checksum;
    my$name=field(substr($h,0,100));my$prefix=field(substr($h,345,155));my$member=length($prefix)?"$prefix/$name":$name;
    my$type=substr($h,156,1);
    die"nonregular/unexpected/duplicate member $member\n"unless($type eq"0"||$type eq"\0")&&exists$wanted{$member}&&!$seen{$member}++;
    die"link member $member\n"if length(field(substr($h,157,100)));
    my$size=number(substr($h,124,12));my$mode=number(substr($h,100,8));
    die"member size/mode $member\n"unless$size==$wanted{$member}{bytes}&&$mode==$wanted{$member}{mode};
    die"nondeterministic metadata $member\n"unless number(substr($h,108,8))==0&&number(substr($h,116,8))==0&&number(substr($h,136,12))==0;
    my$payload=substr($tar,$at+512,$size);
    die"payload truncated/hash $member\n"unless length($payload)==$size&&sha256_hex($payload)eq$wanted{$member}{sha256};
    my$pad=(512-$size%512)%512;
    die"padding $member\n"unless substr($tar,$at+512+$size,$pad)eq"\0"x$pad;
    my$copy=read_file("$out/captures/$member");my$live=read_file($wanted{$member}{source});
    die"retained/live mismatch $member\n"unless sha256_hex($copy)eq$wanted{$member}{sha256}&&sha256_hex($live)eq$wanted{$member}{sha256};
    push@members,{path=>$member,bytes=>$size,sha256=>$wanted{$member}{sha256},mode=>sprintf("%04o",$mode)};
    $total+=$size;$at+=512+$size+$pad;
}
die"archive termination/set\n"unless$end&&keys(%seen)==keys(%wanted);
my$compressed=read_file($archive);
my$result={schema=>"compiler-scalar-gate-captures-1",status=>"passed",archive_path=>$archive,archive_sha256=>sha256_hex($compressed),archive_bytes=>length($compressed),inventory_sha256=>sha256_hex($inventory),regular_members=>scalar@members,payload_bytes=>$total,member_set_exact=>JSON::PP::true,member_payloads_match_copies_and_live_inputs=>JSON::PP::true,uid=>0,gid=>0,mtime=>0,format=>"ustar+gzip-n",members=>\@members};
put("$out/archive-verification.json",JSON::PP->new->canonical->pretty->encode($result));
print"verified143 archive members, $total payload bytes; archive",length($compressed),"B SHA",sha256_hex($compressed),"\n";
