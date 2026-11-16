#!/usr/bin/env perl
# Static post-build census: framing is observed; authentication is the page oracle.
use strict; use warnings; use JSON::PP; use Digest::SHA;
my($directory)=@ARGV; die "usage: census STORE\n" unless defined $directory;
opendir my $dir,$directory or die "$directory: $!";
my @names=sort grep { $_ ne '.' && $_ ne '..' } readdir $dir;
closedir $dir;
my %groups; my($files,$bytes,$headers)=(0,0,0);
my $manifest=Digest::SHA->new(256);
for my $name(@names){
 my $path="$directory/$name";
 die "non-regular store entry $path\n" unless -f $path && !-l $path;
 my $size=-s $path; my $kind=$name eq '.budget-index' ? 'budget-index' : $name=~/\A(semantic-(?:page|root|latest|locator))-/ ? $1 : 'other';
 my $g=($groups{$kind}//={files=>0,stored_bytes=>0,framed_envelopes=>0,declared_payload_bytes=>0,header_bytes_read=>0});
 open my $file,'<:raw',$path or die "$path: $!";
 my $header=''; my $read=read($file,$header,90); die "read $path: $!" unless defined $read;
 close $file or die "$path: $!";
 $files++;$bytes+=$size;$headers+=$read;$g->{files}++;$g->{stored_bytes}+=$size;$g->{header_bytes_read}+=$read;
 if(length($header)==90 && substr($header,0,18) eq "lkjscript-store-2\n"){
  my $length=unpack('Q<',substr($header,18,8));
  die "inconsistent envelope size $path\n" unless $size==90+$length;
  $g->{framed_envelopes}++;$g->{declared_payload_bytes}+=$length;
 }elsif($kind ne 'other'){die "unframed semantic/budget entry $path\n";}
 $manifest->add(pack('Q<',length($name)),$name,pack('Q<',$size),pack('Q<',length($header)),$header);
}
print JSON::PP->new->canonical->pretty->encode({measurement=>'post-build file sizes and first 90 bytes; no authentication claim',files=>$files,stored_bytes=>$bytes,header_bytes_read=>$headers,header_manifest_sha256=>$manifest->hexdigest,namespaces=>\%groups});
