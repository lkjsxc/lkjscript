use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use IPC::Open2;
use Fcntl qw(O_CREAT O_EXCL O_WRONLY);
use IO::Uncompress::Gunzip;
use JSON::PP;
my $out='/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116/target/compiler-copy-sites-root-inventory-20261116';
my $repo='/home/coder/workspace/lkjscript';
my $pin='416da03c86b6073832c9b2cdbb319ff8a3b8c991';
sub read_file {open my $f,'<:raw',$_[0] or die "$_[0]: $!";local $/;my $v=<$f>;return $v//'';}
my $records=decode_json(read_file("$out/file-map.json"));
my $pid=open2(my $reader,my $writer,'git','-C',$repo,'cat-file','--batch');
binmode $reader;binmode $writer;select((select($writer),$|=1)[0]);
sub blob {
 my($ref)=@_;print $writer "$ref\n" or die $!;
 my $head=<$reader>;my($oid,$size)=$head=~/^([a-f0-9]{40}) blob ([0-9]+)\n$/;
 die "Git blob $ref" unless defined $size && $size<16*1024*1024;
 my $v='';while(length($v)<$size){my $n=read($reader,my $part,$size-length($v));die "truncated Git" unless $n;$v.=$part;}
 my $end;read($reader,$end,1)==1&&$end eq "\n" or die "Git framing";
 return {data=>$v,bytes=>0+$size,oid=>$oid,sha256=>(sha256_hex($v)),reference=>$ref};
}
my @members;
my $ref="$pin:bench/compiler-copy-sites-data/source-records/operation-sources.tar.gz";
my $archive=blob($ref);
my $local="$out/archives/operation-sources.tar.gz";
sysopen(my $f,$local,O_WRONLY|O_CREAT|O_EXCL,0600) or die $!;
binmode $f;print $f $archive->{data};close $f or die $!;
open my $list,'-|','tar','--list','--verbose','--numeric-owner','--full-time','--gzip','--file',$local or die $!;
my @entries;
while(<$list>){
 chomp;my($mode,$owner,$length,$date,$time,$name)=split /\s+/,$_,6;
 die "archive type" unless defined($name)&&$length=~/^[0-9]+$/&&substr($mode,0,1)=~/[-d]/;
 my $checked=$name;$checked=~s{^(?:\./)+}{};
 die "archive path" unless ($checked eq ''&&substr($mode,0,1)eq'd')||($checked=~/^[A-Za-z0-9_.\/-]+$/&&$checked!~m{^/|(?:^|/)\.\.?(?:/|$)});
 next if substr($mode,0,1)eq'd';
 die "archive member bound" if $length>8*1024*1024;
 die "unexpected credential" if $name=~/(?:^|\/)(?:key|secret|credential)(?:$|[.\/-])/i;
 push @entries,[$name,0+$length];
}
close $list or die "tar list failed";
for my $entry(@entries){
 open my $stream,'-|','tar','--extract','--gzip','--to-stdout','--file',$local,'--',$entry->[0] or die $!;
 binmode $stream;my $digest=Digest::SHA->new(256);my $length=0;
 while(1){my $n=read($stream,my $part,1024*1024);die "read" unless defined $n;last unless $n;$length+=$n;die "member bound" if $length>8*1024*1024;$digest->add($part);}
 close $stream or die "tar stream failed";
 die "member size" unless $length==$entry->[1];
 push @members,{archive_reference=>$ref,archive_sha256=>$archive->{sha256},member=>$entry->[0],bytes=>$length,sha256=>$digest->hexdigest};
}
my $roles=blob("$pin:bench/compiler-copy-sites-data/roles.samples.gz");
my $gunzip=IO::Uncompress::Gunzip->new(\$roles->{data}) or die "roles gzip";
my $digest=Digest::SHA->new(256);my $length=0;
while(1){my $n=$gunzip->read(my $part,65536);die "gzip read" if $n<0;last unless $n;$length+=$n;die "roles bound" if $length>8*1024*1024;$digest->add($part);}
$gunzip->close or die "gzip close";
my $roles_sha=$digest->hexdigest;
my @matches;
for my $r(@$records){
 for my $m(@members){
  next unless $r->{bytes}==$m->{bytes}&&$r->{sha256}eq$m->{sha256};
  push @matches,{%$m,relative=>$r->{relative},transform=>'tar member'};last;
 }
 if($r->{bytes}==$length&&$r->{sha256}eq$roles_sha){
  push @matches,{relative=>$r->{relative},bytes=>$length,sha256=>$roles_sha,reference=>$roles->{reference},compressed_bytes=>$roles->{bytes},compressed_sha256=>$roles->{sha256},transform=>'gzip decompression'};
 }
}
close $writer or die $!;close $reader or die $!;waitpid($pid,0)==$pid&&$?==0 or die "Git close";
delete $archive->{data};delete $roles->{data};
my $json=JSON::PP->new->canonical->pretty;
for my $pair(['supplemental-archive-members.json',\@members],['supplemental-counterparts.json',\@matches]){
 sysopen(my $f,"$out/$pair->[0]",O_WRONLY|O_CREAT|O_EXCL,0600) or die $!;
 print $f $json->encode($pair->[1]);close $f or die $!;
}
my $report={archive=>$archive,archive_members=>scalar(@members),roles_compressed=>$roles,roles_uncompressed_bytes=>$length,roles_uncompressed_sha256=>$roles_sha,matches=>\@matches,no_source_execution=>JSON::PP::true,no_deletion=>JSON::PP::true};
sysopen(my $f,"$out/supplemental-recovery.json",O_WRONLY|O_CREAT|O_EXCL,0600) or die $!;
print $f $json->encode($report);close $f or die $!;
print $json->encode($report);
