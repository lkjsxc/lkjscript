use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use Fcntl qw(O_WRONLY O_CREAT O_EXCL);
use File::Basename qw(basename);
use IPC::Open2;
use IO::Uncompress::Gunzip;
use JSON::PP;
my $out='/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116/target/compiler-copy-sites-root-inventory-20261116';
my $repo='/home/coder/workspace/lkjscript';
my $pin='416da03c86b6073832c9b2cdbb319ff8a3b8c991';
sub bytes {open my $f,'<:raw',$_[0] or die "$_[0]: $!"; local $/; my $v=<$f>; return defined($v)?$v:'';}
my $records=decode_json(bytes("$out/file-map.json"));
my $pid=open2(my $reader,my $writer,'git','-C',$repo,'cat-file','--batch');
binmode $reader; binmode $writer; select((select($writer),$|=1)[0]);
sub blob {
 my ($ref)=@_; print $writer "$ref\n" or die $!; my $header=<$reader>;
 my ($oid,$size)=$header=~/^([0-9a-f]{40}) blob ([0-9]+)\n$/; die "Git object $ref" unless defined $size;
 my $data='';
 while(length($data)<$size) {my $n=read($reader,my $part,$size-length($data)); die "Git truncation" unless defined($n)&&$n; $data.=$part;}
 my $end; read($reader,$end,1)==1&&$end eq "\n" or die "Git framing";
 return {oid=>$oid,bytes=>0+$size,sha256=>sha256_hex($data),data=>$data};
}
my (%verified_object,@verified,@archives,@members);
for my $r(@$records) {
 next unless $r->{retained_ref};
 my $got=$verified_object{$r->{git_blob}};
 if(!$got) {
  $got=blob($r->{retained_ref});
  delete $got->{data};
  $verified_object{$r->{git_blob}}=$got;
 }
 die "counterpart mismatch $r->{relative}" unless $got->{oid} eq $r->{git_blob} && $got->{bytes}==$r->{bytes} && $got->{sha256} eq $r->{sha256};
 push @verified,{relative=>$r->{relative},bytes=>$r->{bytes},sha256=>$r->{sha256},reference=>$r->{retained_ref}};
}
mkdir "$out/archives",0700 or die "archive output exists or unavailable: $!";
for my $line(split /\n/,bytes("$out/current-tree.tsv")) {
 my ($oid,$size,$path)=$line=~/^[0-7]+ blob ([0-9a-f]{40}) +([0-9]+)\t(bench\/compiler-copy-sites-data\/proposals\/[^\/]+\.tar\.gz)$/;
 next unless defined $path;
 my $ref="$pin:$path"; my $got=blob($ref);
 die "archive metadata" unless $got->{oid} eq $oid && $got->{bytes}==$size;
 my $local="$out/archives/".basename($path);
 sysopen(my $f,$local,O_WRONLY|O_CREAT|O_EXCL,0600) or die $!;
 binmode $f; print $f $got->{data}; close $f or die $!;
 open my $list,'-|','tar','--list','--verbose','--numeric-owner','--full-time','--gzip','--file',$local or die "tar list";
 my @entries;
 while(<$list>) {
  chomp; my ($mode,$owner,$length,$date,$time,$name)=split /\s+/,$_,6;
  die "archive listing" unless defined($name) && $length=~/^[0-9]+$/;
  die "archive entry type" unless substr($mode,0,1) eq '-' || substr($mode,0,1) eq 'd';
  my $checked=$name; $checked=~s{^(?:\./)+}{};
  die "unsafe archive path" unless ($checked eq '' && substr($mode,0,1) eq 'd') || ($checked=~/^[A-Za-z0-9_.\/-]+$/ && $checked!~m{^/|(?:^|/)\.\.?(?:/|$)});
  next if substr($mode,0,1) eq 'd';
  die "oversized proposal member" if $length>8*1024*1024;
  die "unexpected credential member" if basename($name)=~/(?:^|[-_.])(?:key|secret|credential|token)(?:$|[-_.])/i;
  push @entries,[$name,0+$length];
 }
 close $list or die "tar list failed";
 my $count=0; my $total=0;
 for my $entry(@entries) {
  open my $stream,'-|','tar','--extract','--gzip','--to-stdout','--file',$local,'--',$entry->[0] or die "tar member";
  binmode $stream; my $digest=Digest::SHA->new(256); my $length=0;
  while(1) {my $n=read($stream,my $part,1024*1024); die "member read" unless defined $n; last unless $n; $length+=$n; die "proposal bound" if $length>8*1024*1024; $digest->add($part);}
  close $stream or die "tar member failed";
  die "archive member size mismatch" unless $length==$entry->[1];
  push @members,{archive_reference=>$ref,archive_sha256=>$got->{sha256},member=>$entry->[0],bytes=>$length,sha256=>$digest->hexdigest};
  $count++; $total+=$length;
 }
 push @archives,{reference=>$ref,bytes=>$got->{bytes},sha256=>$got->{sha256},regular_members=>$count,member_bytes=>$total};
}
my %member_by_hash;
push @{$member_by_hash{$_->{sha256}}},$_ for @members;
my @member_matches;
for my $r(@$records) {
 next if $r->{retained_ref};
 for my $m(@{$member_by_hash{$r->{sha256}}||[]}) {
  next unless $m->{bytes}==$r->{bytes};
  push @member_matches,{relative=>$r->{relative},bytes=>$r->{bytes},sha256=>$r->{sha256},archive_reference=>$m->{archive_reference},archive_sha256=>$m->{archive_sha256},member=>$m->{member}};
  last;
 }
}
my $seed_ref='7d0731e99aeaf241c4334fc5ff065a341fb388c8:seed/lkjscript.gz';
my $seed=blob($seed_ref);
die "seed identity" unless $seed->{bytes}==6547345 && $seed->{sha256} eq '505be947525d53b4db1bff6c85cb77622cb2f5dc7e7d7fe92b772ee3b8ffc3e2';
my $gunzip=IO::Uncompress::Gunzip->new(\$seed->{data}) or die "seed gzip";
my $digest=Digest::SHA->new(256); my $length=0;
while(1) {my $n=$gunzip->read(my $part,1024*1024); die "seed decode" if $n<0; last if $n==0; $length+=$n; $digest->add($part);}
$gunzip->close or die "seed close"; my $hash=$digest->hexdigest;
die "canonical seed recovery" unless $length==32584542 && $hash eq '6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc';
my @seed_matches;
for my $r(@$records) {
 if($r->{bytes}==$length && $r->{sha256} eq $hash) {push @seed_matches,{relative=>$r->{relative},bytes=>$length,sha256=>$hash,reference=>$seed_ref,transform=>'gzip decompression'};}
 elsif($r->{bytes}==$seed->{bytes} && $r->{sha256} eq $seed->{sha256}) {push @seed_matches,{relative=>$r->{relative},bytes=>$r->{bytes},sha256=>$r->{sha256},reference=>$seed_ref,transform=>'identity'};}
}
close $writer or die $!; close $reader or die $!; waitpid($pid,0)==$pid&&$?==0 or die "Git reader exit";
my $json=JSON::PP->new->canonical->pretty;
for my $pair(
 ['verified-git-counterparts.json',\@verified],['archive-members.json',\@members],
 ['archive-counterparts.json',\@member_matches],['seed-recovery.json',\@seed_matches]
) {open my $f,'>:raw',"$out/$pair->[0]" or die $!; print $f $json->encode($pair->[1]); close $f or die $!;}
my $summary={git_counterpart_files=>scalar(@verified),unique_verified_git_blobs=>scalar(keys%verified_object),archives=>\@archives,archive_counterpart_files=>scalar(@member_matches),seed_matches=>\@seed_matches,source_executed=>JSON::PP::false,deleted=>JSON::PP::false};
open my $f,'>:raw',"$out/recovery-summary.json" or die $!; print $f $json->encode($summary); close $f or die $!;
print $json->encode($summary);
