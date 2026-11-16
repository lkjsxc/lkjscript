use strict;use warnings;
use Fcntl qw(:mode);
use Cwd qw(abs_path);
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use Errno qw(ENOENT);
my $base='/home/coder/workspace/lkjscript';
my $cap="$base/target/compiler-reference-capture-cache-cleanup-1";
my @spec=(
 {name=>'bootstrap',path=>"$base/target/compiler-reference-capture-bootstrap-store-1",inode=>37597956,files=>13,bytes=>245629076,hash=>'4b6e3890f5593fe0e5707f8cd0e1a1b7f85efddf394d19368d32da33da9b4aa2'},
 {name=>'gate',path=>"$base/target/compiler-reference-capture-gate-store-1",inode=>37608489,files=>16050,bytes=>323185818,hash=>'b0761e9391c9f3dddf1f42d6c4aa88b7ace13ce52b80c6c63b6ba27a75bcb89a'}
);
sub record {my($rel,$s,$type)=@_;return join("\t",$rel,$type,$s->[4],$s->[5],sprintf("%04o",$s->[2]&07777),$s->[7],$s->[3],$s->[0],$s->[1])."\n";}
sub permitted {my($n)=@_;return $n=~/\A\.budget-(?:index|lock)\z/ || $n=~/\A(?:front|check|image|inputs-image|test|inputs-test|gate-closed)-[0-9a-f]{64}-[0-9a-f]{64}\z/ || $n=~/\Anative-[0-9a-f]{64}-[0-9a-f]{64}-[0-9a-f]{64}\z/;}
for my $root(@spec){
 my $path=$root->{path};my @s=lstat $path;
 die "root changed $path\n" unless @s && S_ISDIR($s[2]) && $s[0]==144 && $s[1]==$root->{inode} && $s[4]==1001 && $s[5]==1001 && ($s[2]&07777)==0700 && abs_path($path) eq $path;
 my(@lines,@leaves,@dirs);my @todo=('');my($count,$bytes)=(0,0);
 while(@todo){my $rel=shift @todo;my $dir=length($rel)?"$path/$rel":$path;opendir my $h,$dir or die $!;
  my @names=sort grep {$_ ne '.' && $_ ne '..'} readdir $h;closedir $h or die $!;
  for my $name(@names){my $r=length($rel)?"$rel/$name":$name;my @st=lstat "$path/$r";die "unsafe member $r\n" unless @st && $st[4]==1001 && $st[5]==1001 && $st[0]==144;
   if(S_ISDIR($st[2])){die "unexpected directory $r\n" unless $root->{name} eq 'gate' && $r=~/\Aworker-[0-3]\z/;push @dirs,{path=>$r,stat=>\@st};push @todo,$r;push @lines,record($r,\@st,'d');}
   else{die "unexpected leaf $r\n" unless S_ISREG($st[2]) && $st[3]==1 && permitted($name);my $line=record($r,\@st,'f');push @leaves,{path=>$r,line=>$line};push @lines,$line;$count++;$bytes+=$st[7];}
  }
 }
 my $inventory=join('',sort @lines);
 die "inventory changed $path\n" unless sha256_hex($inventory) eq $root->{hash} && $count==$root->{files} && $bytes==$root->{bytes};
 open my $out,'>:raw',"$cap/$root->{name}-before.tsv" or die $!;print {$out} $inventory;close $out or die $!;
 $root->{leaves}=\@leaves;$root->{dirs}=\@dirs;
}
# Both complete populations have passed before the first unlink.
my @result;
for my $root(@spec){
 my $path=$root->{path};my($removed,$bytes)=(0,0);
 for my $leaf(sort {$a->{path} cmp $b->{path}} @{$root->{leaves}}){
  my $p="$path/$leaf->{path}";my @s=lstat $p;die "leaf drift $p\n" unless @s && S_ISREG($s[2]) && record($leaf->{path},\@s,'f') eq $leaf->{line};
  unlink $p or die "unlink $p: $!";$removed++;$bytes+=$s[7];
 }
 for my $dir(sort {$b->{path} cmp $a->{path}} @{$root->{dirs}}){
  my $p="$path/$dir->{path}";my @s=lstat $p;my $before=$dir->{stat};
  die "directory drift $p\n" unless @s && S_ISDIR($s[2]) && $s[0]==$before->[0] && $s[1]==$before->[1] && $s[4]==$before->[4] && $s[5]==$before->[5] && $s[2]==$before->[2];
  rmdir $p or die "rmdir $p: $!";
 }
 my @s=lstat $path;die "root drift\n" unless @s && S_ISDIR($s[2]) && $s[0]==144 && $s[1]==$root->{inode} && $s[4]==1001 && $s[5]==1001 && ($s[2]&07777)==0700;
 rmdir $path or die "rmdir root: $!";
 $!=0;my @after=lstat $path;die "absence failed\n" unless !@after && $! == ENOENT;
 push @result,{path=>$path,removed_files=>$removed,removed_bytes=>$bytes,removed_directories=>scalar(@{$root->{dirs}})+1,postcondition=>'ENOENT',cache_recovery=>'rebuildable; committed bootstrap and full gate evidence retained'};
}
print JSON::PP->new->canonical->pretty->encode({status=>'complete',mode=>'cooperative enumerated non-atomic deletion; stop on any drift',roots=>\@result});
