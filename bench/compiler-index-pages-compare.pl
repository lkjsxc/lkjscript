#!/usr/bin/env perl
use strict; use warnings; use Digest::SHA qw(sha256 sha256_hex hmac_sha256);
use JSON::PP;
my ($left,$right,$key_path,$left_producer,$right_producer,$left_snapshot,$right_snapshot)=@ARGV;
die "usage: compare LEFT RIGHT KEY PRODUCER_A PRODUCER_B SNAPSHOT_A SNAPSHOT_B\n" unless defined $right_snapshot;
sub raw { my($path)=@_; open my $f,'<:raw',$path or die "$path: $!"; local $/; return <$f>; }
my $secret=raw($key_path); die "private key length\n" unless length($secret)==32;
sub frame { return pack('Q<',length($_[0])).$_[0]; }
sub envelope {
 my($path,$producer,$namespace,$key,$maximum)=@_;
 my $bytes=raw($path); my $magic="lkjscript-store-2\n"; my $head=length($magic)+72;
 die "envelope framing $path\n" unless length($bytes)>=$head && substr($bytes,0,length($magic)) eq $magic;
 my $size=unpack('Q<',substr($bytes,length($magic),8));
 die "envelope length $path\n" unless $size<=$maximum && length($bytes)==$head+$size;
 my $digest=substr($bytes,length($magic)+8,32); my $tag=substr($bytes,length($magic)+40,32);
 my $body=substr($bytes,$head);
 my $message=frame('lkjscript-authenticated-store-root-2').frame(pack('H*',$producer)).frame('linux-x86_64').frame($namespace).frame($key).pack('Q<',$size).frame($digest);
 die "envelope authentication $path\n" unless hmac_sha256($message,$secret) eq $tag && sha256($body) eq $digest;
 return $body;
}
sub word { my($s,$at)=@_; die "short word\n" if $$at+8>length($s); my $v=unpack('Q<',substr($s,$$at,8)); $$at+=8; return $v; }
sub field { my($s,$at)=@_; my $n=word($s,$at); die "short field\n" if $n>length($s)-$$at; my $v=substr($s,$$at,$n); $$at+=$n; return $v; }
sub snapshot {
 my($directory,$producer,$id)=@_; my $slot=sha256_hex(pack('H*',$producer));
 my $s=envelope("$directory/semantic-root-$slot-$id",$producer,'semantic-root',$id,376);
 die "snapshot digest\n" unless sha256_hex($s) eq $id;
 my $at=0; die "snapshot version\n" unless word($s,\$at)==2;
 my @v=map { field($s,\$at) } 1..5; my $facts=word($s,\$at);
 die "snapshot shape/producer\n" unless $at==length($s) && $v[0] eq $producer;
 my $locator=envelope("$directory/semantic-locator-$id",sha256_hex('lkjscript-semantic-locator-2'),'semantic-locator',$id,104);
 my $expected=pack('Q<',2).pack('H*',$producer).pack('H*',$v[4]).pack('H*',$v[3]);
 die "locator relation\n" unless $locator eq $expected;
 return {producer=>$v[0], source=>$v[1], checked_ir=>$v[2], root=>$v[3], selector=>$v[4], facts=>$facts, slot=>$slot};
}
my $a=snapshot($left,$left_producer,$left_snapshot); my $b=snapshot($right,$right_producer,$right_snapshot);
for my $name(qw(source checked_ir root selector facts)){die "snapshot $name differs\n" unless $a->{$name} eq $b->{$name};}
die "producers or snapshots unexpectedly equal\n" if $left_producer eq $right_producer || $left_snapshot eq $right_snapshot;
my @todo=([$a->{root},0,'']); my %seen; my %kind; my ($nodes,$branches,$leaves,$logical,$fact_count,$fact_bytes,$value_bytes,$key_bytes,$payload_bytes,$envelope_bytes,$maximum_depth)=(0)x10;
my $manifest=Digest::SHA->new(256);
while(@todo){
 my($id,$depth,$prefix)=@{shift @todo}; die "repeated rooted page\n" if $seen{$id}++;
 my $x=envelope("$left/semantic-page-$a->{slot}-$id",$left_producer,'semantic-page',$id,32768);
 my $y=envelope("$right/semantic-page-$b->{slot}-$id",$right_producer,'semantic-page',$id,32768);
 die "page content or identity differs\n" unless $x eq $y && sha256_hex($x) eq $id;
 $manifest->add(frame($id),frame($x)); $nodes++; $payload_bytes+=length($x); $envelope_bytes+=length($x)+90;
 $maximum_depth=$depth if $depth>$maximum_depth;
 my $at=0; die "node version\n" unless word($x,\$at)==1; my $tag=word($x,\$at); die "node depth\n" unless word($x,\$at)==$depth; my $count=word($x,\$at);
 if($tag==0){
  $leaves++; die "leaf count\n" if $count>64; my $previous='';
  for(1..$count){
   my $name=field($x,\$at); my $hash=field($x,\$at); my $value=field($x,\$at);
   die "leaf key/path/order\n" unless $hash eq sha256_hex($name) && substr($hash,0,$depth) eq $prefix && (!$previous || $previous lt $hash);
   $previous=$hash; $logical++; $key_bytes+=length($name); $value_bytes+=length($value);
   my($kind_name)=split /\t/,$name; my $k=($kind{$kind_name}//={pages=>0,facts=>0,fact_bytes=>0,value_bytes=>0,key_bytes=>0});
   $k->{pages}++;$k->{value_bytes}+=length($value);$k->{key_bytes}+=length($name);
   my $v=0; die "fact page version\n" unless word($value,\$v)==1; my $next=word($value,\$v); my $n=word($value,\$v); die "fact count\n" if $n>16;
   for(1..$n){my $line=field($value,\$v);die "fact byte limit\n" if length($line)+8>16000;$fact_count++;$fact_bytes+=length($line);$k->{facts}++;$k->{fact_bytes}+=length($line);}
   die "fact framing\n" unless $v==length($value);
  }
 }elsif($tag==1){
  $branches++;die "branch count/depth\n" if $count==0 || $count>16 || $depth>=64;my $previous=-1;
  for(1..$count){my $digit=word($x,\$at);my $child=field($x,\$at);die "branch order/key\n" unless $digit>$previous && chr($digit)=~/[0-9a-f]/ && $child=~/\A[0-9a-f]{64}\z/;$previous=$digit;push @todo,[$child,$depth+1,$prefix.chr($digit)];}
 }else{die "node tag\n";}
 die "node trailing bytes\n" unless $at==length($x);
}
die "snapshot fact count differs\n" unless $fact_count==$a->{facts};
my $out={status=>'exact',source=>$a->{source},checked_ir=>$a->{checked_ir},root=>$a->{root},selector=>$a->{selector},facts=>$fact_count,
 producer_a=>$left_producer,producer_b=>$right_producer,snapshot_a=>$left_snapshot,snapshot_b=>$right_snapshot,
 physical_nodes=>$nodes,branches=>$branches,leaves=>$leaves,logical_pages=>$logical,max_depth=>$maximum_depth,
 logical_value_bytes=>$value_bytes,fact_utf8_bytes=>$fact_bytes,logical_key_bytes=>$key_bytes,
 authenticated_page_payload_bytes=>$payload_bytes,authenticated_page_envelope_bytes=>$envelope_bytes,
 ordered_rooted_payload_sha256=>$manifest->hexdigest,kinds=>\%kind};
print JSON::PP->new->canonical->pretty->encode($out);
