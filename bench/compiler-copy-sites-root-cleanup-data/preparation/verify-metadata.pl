use strict;
use warnings;
use Cwd qw(abs_path);
use Fcntl qw(:mode O_CREAT O_EXCL O_WRONLY);
use File::Find;
use JSON::PP;
use POSIX qw(strftime);
my$b='/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116/target/compiler-copy-sites-root-inventory-20261116/proposed-cleanup';
my$t='/home/coder/workspace/lkjscript/target';
my@scopes=qw(compiler-copy-sites-integrated compiler-copy-sites-driver-candidate compiler-copy-sites-integration-evidence compiler-copy-sites-gate.ccpzZ9 compiler-copy-sites-evidence.20261116-1 gate);
open my$f,'<',"$b/full-inventory.tsv"or die$!;my$head=<$f>;my%rows;
while(<$f>){chomp;my@v=split/\t/,$_,-1;die"columns"unless@v==15;my$p=pop@v;$rows{$p}=\@v;}
close$f or die$!;my%found;my($files,$dirs,$bytes)=(0,0,0);
for my$scope(@scopes){
 my$root="$t/$scope";die"alias/missing scope"unless-d$root&&abs_path($root)eq$root;
 find({no_chdir=>1,follow=>0,wanted=>sub{
  my$path=$File::Find::name;die"outside target"unless index($path,"$t/")==0;
  my$p=substr($path,length($t)+1);my$r=$rows{$p}or die"extra entry $p";die"duplicate"if$found{$p}++;
  my@s=lstat($path);die"missing $p"unless@s;
  my$type=S_ISREG($s[2])?'f':S_ISDIR($s[2])?'d':'other';
  my@v=($type,sprintf('%o',$s[2]&07777),@s[4,5,3,7,12,0,1,9,10]);
  for my$i(0..10){die"changed metadata $p column $i"unless"$v[$i]"eq"$r->[$i]";}
  if($type eq'f'){$files++;$bytes+=$s[7];}elsif($type eq'd'){$dirs++;}else{die"special";}
 }},$root);
}
die"missing entries/totals"unless keys(%found)==keys(%rows)&&$files==21076&&$dirs==112&&$bytes==779287389;
my$r={checked_utc=>strftime('%Y-%m-%dT%H:%M:%SZ',gmtime),regular_files=>$files,directories=>$dirs,logical_bytes=>$bytes,all_initial_metadata_matches=>JSON::PP::true,scope_absences=>0,scopes=>[map{"$t/$_"}@scopes],whole_file_hashes_repeated_in_this_pass=>JSON::PP::false,note=>'This final preparation pass compares the complete live path set and original metadata. Initial complete file hashes and exact archive comparisons are retained separately; the future guard repeats full hashes before deletion.',deleted=>JSON::PP::false,process_or_fd_observer=>JSON::PP::false};
sysopen(my$g,"$b/preparation-final-metadata.json",O_WRONLY|O_CREAT|O_EXCL,0600)or die$!;
print $g JSON::PP->new->canonical->pretty->encode($r);close$g or die$!;
print JSON::PP->new->canonical->encode($r),"\n";
