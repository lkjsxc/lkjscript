use strict;
use warnings;
use Digest::SHA;
use File::Basename qw(basename);
use JSON::PP;
my $out='/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116/target/compiler-copy-sites-root-inventory-20261116';
my $target='/home/coder/workspace/lkjscript/target';
my %scope=map {$_=>1} qw(compiler-copy-sites-integrated compiler-copy-sites-driver-candidate compiler-copy-sites-integration-evidence compiler-copy-sites-gate.ccpzZ9 compiler-copy-sites-evidence.20261116-1 gate);
my %source_names=map {$_=>1} qw(
 compiler-copy-sites-integrated/examples/journal/token.lkj
 compiler-copy-sites-integrated/tests/diagnostics/programs_image_key_grant_overlap.sh
 compiler-copy-sites-integrated/tools/store-key.sh
);
sub bytes {open my $f,'<:raw',$_[0] or die "$_[0]: $!"; local $/; my $v=<$f>; return defined($v)?$v:'';}
my %objects;
for my $pair(
 ['416da03c86b6073832c9b2cdbb319ff8a3b8c991','current-tree.tsv'],
 ['27c94440646d787efc4e7b43a712405288c22c61','baseline-tree.tsv']
) {
 for my $line(split /\n/,bytes("$out/$pair->[1]")) {
  my ($mode,$oid,$size,$path)=$line=~/^([0-7]+) blob ([0-9a-f]{40}) +([0-9]+)\t(.+)$/;
  die "Git tree row" unless defined $path;
  push @{$objects{$oid}},{commit=>$pair->[0],path=>$path,bytes=>0+$size,mode=>$mode};
 }
}
my (@records,@unmatched,%summary);
open my $table,'>:raw',"$out/full-inventory.tsv" or die $!;
print $table join("\t",qw(type mode uid gid links bytes blocks device inode mtime ctime sha256 git_blob hint path)),"\n";
for my $line(split /\n/,bytes("$out/metadata.tsv")) {
 my @f=split /\t/,$line,-1; die "metadata framing" unless @f==12;
 my $path=$f[10];
 die "outside target" unless index($path,"$target/")==0;
 my $relative=substr($path,length($target)+1);
 my ($root)=split m{/},$relative; die "scope" unless $scope{$root};
 die "excluded scope" if $relative=~m{(?:^|/)\.generation-one(?:/|$)|compiler-copy-sites-(?:run|evidence)\.20261116-2|extractor};
 my @a=lstat($path); die "missing $path" unless @a;
 my $type=-f _?'f':-d _?'d':'other';
 die "type changed $path" unless $type eq $f[0] && ($type eq 'f'||$type eq 'd');
 die "metadata changed $path" unless
  sprintf('%o',$a[2]&07777) eq $f[1] && $a[4]==$f[2] && $a[5]==$f[3] &&
  $a[3]==$f[4] && $a[7]==$f[5] && $a[12]==$f[6] &&
  $a[0]==$f[7] && $a[1]==$f[8] && $a[9]==int($f[9]);
 my ($sha,$oid,$hint)=('-','-','directory');
 my $s=($summary{$root}||={files=>0,directories=>0,bytes=>0,matched_files=>0,matched_bytes=>0,unmatched_files=>0,unmatched_bytes=>0});
 if($type eq 'f') {
  die "hardlinked file $relative" unless $a[3]==1;
  if($relative=~m{(?:^|[-_./])(?:key|secret|credential|token)(?:$|[-_./])}i) {
   die "credential candidate not read $relative" unless $source_names{$relative};
  }
  my $digest=Digest::SHA->new(256); my $git=Digest::SHA->new(1); $git->add("blob $a[7]\0");
  open my $fh,'<:raw',$path or die "open $path: $!"; my $prefix='';
  while(1) {
   my $n=read($fh,my $part,1024*1024); die "read $path" unless defined $n; last unless $n;
   $prefix=substr($part,0,8) if $prefix eq '';
   $digest->add($part); $git->add($part);
  }
  close $fh or die $!;
  $sha=$digest->hexdigest; $oid=$git->hexdigest;
  $hint=substr($prefix,0,4) eq "\x7fELF"?'elf':substr($prefix,0,2) eq "\x1f\x8b"?'gzip':substr($prefix,0,7) eq 'LKJPROF'?'profile':'other';
  $s->{files}++; $s->{bytes}+=$a[7];
  my @matches=sort {
   my $ac=$a->{commit} eq '416da03c86b6073832c9b2cdbb319ff8a3b8c991';
   my $bc=$b->{commit} eq '416da03c86b6073832c9b2cdbb319ff8a3b8c991';
   my $abench=$a->{path}=~m{^bench/compiler-copy-sites-data/};
   my $bbench=$b->{path}=~m{^bench/compiler-copy-sites-data/};
   ($bbench<=>$abench)||($bc<=>$ac)||($a->{path}cmp$b->{path})
  } @{$objects{$oid}||[]};
  my $record={path=>$path,relative=>$relative,scope=>$root,bytes=>0+$a[7],sha256=>$sha,git_blob=>$oid,hint=>$hint};
  if(@matches) {
   die "object size" unless $matches[0]{bytes}==$a[7];
   $record->{retained_ref}="$matches[0]{commit}:$matches[0]{path}";
   $s->{matched_files}++; $s->{matched_bytes}+=$a[7];
  } else {
   $s->{unmatched_files}++; $s->{unmatched_bytes}+=$a[7]; push @unmatched,$record;
  }
  push @records,$record;
 } else {$s->{directories}++;}
 my @b=lstat($path); die "entry gone $path" unless @b;
 for my $i(0,1,2,3,4,5,7,9,10,12){die "entry changed during read $path" unless $a[$i]==$b[$i];}
 print $table join("\t",$type,sprintf('%o',$a[2]&07777),$a[4],$a[5],$a[3],$a[7],$a[12],$a[0],$a[1],$a[9],$a[10],$sha,$oid,$hint,$relative),"\n";
}
close $table or die $!;
my $json=JSON::PP->new->canonical->pretty;
for my $pair(['file-map.json',\@records],['unmatched.json',\@unmatched],['mapping-summary.json',\%summary]) {
 open my $f,'>:raw',"$out/$pair->[0]" or die $!; print $f $json->encode($pair->[1]); close $f or die $!;
}
print $json->encode({scopes=>\%summary,unmatched_files=>scalar @unmatched,readable_key_name_exceptions=>[sort keys%source_names],deleted=>JSON::PP::false});
