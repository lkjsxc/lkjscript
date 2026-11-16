use strict;
use warnings;
use Cwd qw(abs_path);
use Digest::SHA qw(sha256_hex);
use Fcntl qw(O_RDONLY O_NOFOLLOW O_CREAT O_EXCL O_WRONLY :mode);
use File::Find;
use IPC::Open2;
use JSON::PP;
use POSIX qw(strftime);
my$root='/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116/target';
my$out="$root/compiler-native-temp-closure-20261116";
my$repo='/home/coder/workspace/lkjscript';
my$pin='99ac7a6043cd9ee411970e55970634a34506a709';
my@scopes=qw(compiler-copy-sites-cleanup-20261116 compiler-legacy-root-inventory-20261116 compiler-ci-43878716520-20261116 compiler-copy-sites-root-inventory-20261116 compiler-ci-964be9a4-20261116 compiler-copy-sites-cleanup-guard-v2-20261116);
my(%summary,@rows,@files,@flags);
for my$scope(@scopes){
 my$base="$root/$scope";die"scope alias"unless-d$base&&abs_path($base)eq$base;
 find({no_chdir=>1,follow=>0,wanted=>sub{
  my$p=$File::Find::name;my$rel=substr($p,length($root)+1);
  my@s=lstat$p;die"missing entry"unless@s;
  my$type=S_ISREG($s[2])?'f':S_ISDIR($s[2])?'d':'other';
  die"nonregular/non-directory entry $rel"unless$type ne'other';
  my@metadata=($type,sprintf('%o',$s[2]&07777),@s[4,5,3,7,12,0,1,9,10]);
  my($sha,$oid,$hint)=('-','-','directory');
  if($type eq'f'){
   my($leaf)=$rel=~m{([^/]+)$};
   if($leaf=~/(?:^|[-_.])(?:key|credential|secret)(?:$|[-_.])/i){
    push@flags,{path=>$rel,bytes=>$s[7],reason=>'Credential-like basename; content not read.'};$hint='unread-credential-name';
   }else{
    sysopen(my$f,$p,O_RDONLY|O_NOFOLLOW)or die$!;binmode$f;my@held=stat$f;
    die"descriptor mismatch"unless$held[0]==$s[0]&&$held[1]==$s[1];
    my$h256=Digest::SHA->new(256);my$h1=Digest::SHA->new(1);$h1->add("blob $s[7]\0");
    my$n=0;my$prefix='';
    while(1){my$got=read($f,my$chunk,1024*1024);die"read"unless defined$got;last unless$got;$prefix.=substr($chunk,0,8-length$prefix)if length($prefix)<8;$n+=$got;$h256->add($chunk);$h1->add($chunk);}
    my@after=stat$f;close$f or die$!;my@now=lstat$p;
    for my$i(0,1,2,3,4,5,7,9,10){die"changed file $rel"unless$s[$i]==$held[$i]&&$held[$i]==$after[$i]&&$after[$i]==$now[$i];}
    die"length"unless$n==$s[7];($sha,$oid)=($h256->hexdigest,$h1->hexdigest);
    $hint=substr($prefix,0,4)eq"\x7fELF"?'ELF':substr($prefix,0,2)eq"\x1f\x8b"?'gzip':'other';
   }
   push@files,{path=>$rel,scope=>$scope,bytes=>$s[7],sha256=>$sha,git_blob=>$oid,hint=>$hint};
   $summary{$scope}{files}++;$summary{$scope}{bytes}+=$s[7];$summary{$scope}{allocated_bytes}+=$s[12]*512;
   push@flags,{path=>$rel,reason=>'Multiple regular-file links',links=>$s[3]}if$s[3]!=1;
  }else{$summary{$scope}{directories}++;}
  push@rows,[\@metadata,$sha,$oid,$hint,$rel];
 }},$base);
}
my@paths;
open my$top,'-|','git','-C',$repo,'ls-tree','--name-only',"$pin:bench"or die$!;
while(<$top>){
 chomp;next unless /^(?:compiler-copy-sites-root-cleanup|compiler-legacy-root-cleanup)/||$_ eq'compiler-copy-sites-data';
 die"unsafe retained prefix"unless/^[A-Za-z0-9_.-]+$/;push@paths,"bench/$_";
}
close$top or die"Git tree";
my%objects;
for my$path(@paths){
 open my$f,'-|','git','-C',$repo,'ls-tree','-r','--long',$pin,'--',$path or die$!;
 while(<$f>){
  chomp;my($oid,$n,$p)=/^[0-7]+ blob ([0-9a-f]{40}) +([0-9]+)\t(.+)$/;next unless defined$p;
  if($path eq'bench/compiler-copy-sites-data'){next unless$p=~m{^bench/compiler-copy-sites-data/(?:native-cleanup|proposals|source-records|ci-[0-9]+)/};}
  next if$p=~/cohort-?2|evidence\.20261116-2|extractor|census|join/;
  $objects{$oid}//={reference=>"$pin:$p",bytes=>0+$n};
 }
 close$f or die"Git tree read";
}
my$pid=open2(my$reader,my$writer,'git','-C',$repo,'cat-file','--batch');binmode$reader;binmode$writer;select((select($writer),$|=1)[0]);
my%verified;
sub blob{
 my($ref)=@_;print$writer "$ref\n"or die$!;my$head=<$reader>;my($oid,$n)=$head=~/^([a-f0-9]{40}) blob ([0-9]+)\n$/;die"Git blob"unless defined$n;
 my$v='';while(length$v<$n){my$got=read($reader,my$part,$n-length$v);die"Git truncation"unless$got;$v.=$part;}
 my$end;read($reader,$end,1)==1&&$end eq"\n"or die"Git framing";return($oid,$n,sha256_hex($v));
}
my(@mapping,@unmatched,%classes);
for my$f(sort{$a->{path}cmp$b->{path}}@files){
 my$class;my$ref='-';
 if(my$o=$objects{$f->{git_blob}}){
  my$v=$verified{$f->{git_blob}};
  if(!$v){my@v=blob($o->{reference});$v=\@v;$verified{$f->{git_blob}}=$v;}
  die"Git exact bytes mismatch"unless$v->[0]eq$f->{git_blob}&&$v->[1]==$f->{bytes}&&$v->[2]eq$f->{sha256};
  $class='exact_committed_counterpart';$ref=$o->{reference};
 }elsif($f->{path}=~m{^compiler-copy-sites-root-inventory-20261116/(?:cleanup-capture-v2/|cleanup-results-v2/|cleanup-completion-v2\.json$|cleanup-v2\.manifest\.sha256$)}){
  $class='new_completed_cleanup_evidence';
 }elsif($f->{scope}=~/^compiler-ci-/){
  $class='captured_ci_observation';
 }elsif($f->{hint}eq'unread-credential-name'){
  $class='preserve_unread_name';
 }else{$class='preparation_intermediate';}
 $f->{class}=$class;$f->{reference}=$ref;
 $classes{$class}{files}++;$classes{$class}{bytes}+=$f->{bytes};
 $summary{$f->{scope}}{classes}{$class}{files}++;$summary{$f->{scope}}{classes}{$class}{bytes}+=$f->{bytes};
 push@mapping,join("\t",$class,$f->{bytes},$f->{sha256},$ref,$f->{path});
 push@unmatched,$f unless$class eq'exact_committed_counterpart';
}
close$writer or die$!;close$reader or die$!;waitpid($pid,0)==$pid&&$?==0 or die"Git exit";
sub put{my($p,$v)=@_;sysopen(my$f,"$out/$p",O_WRONLY|O_CREAT|O_EXCL,0600)or die$!;print{$f}$v;close$f or die$!;}
my$inventory=join("\t",qw(type mode uid gid links bytes blocks device inode mtime ctime sha256 git_blob hint path))."\n";
for my$r(sort{$a->[4]cmp$b->[4]}@rows){$inventory.=join("\t",@{$r->[0]},@$r[1..4])."\n";}
put('inventory.tsv',$inventory);
put('disposition.tsv',"class\tbytes\tsha256\treference\tpath\n".join("\n",@mapping)."\n");
my($files,$dirs,$bytes,$allocated)=(0,0,0,0);
for my$s(values%summary){$files+=$s->{files};$dirs+=$s->{directories};$bytes+=$s->{bytes};$allocated+=$s->{allocated_bytes};}
my$r={prepared_utc=>strftime('%Y-%m-%dT%H:%M:%SZ',gmtime),root=>$root,retention_pin=>$pin,scopes=>\@scopes,files=>$files,directories=>$dirs,logical_bytes=>$bytes,allocated_bytes=>$allocated,classes=>\%classes,scope_summary=>\%summary,flags=>\@flags,unmatched=>\@unmatched,proposal_only=>JSON::PP::true,deleted=>JSON::PP::false,no_compiler_or_image_execution=>JSON::PP::true,no_process_observer=>JSON::PP::true,no_private_key_read=>JSON::PP::true};
put('summary.json',JSON::PP->new->canonical->pretty->encode($r));
my@largest=sort{$b->{bytes}<=>$a->{bytes}}@unmatched;
splice(@largest,10)if@largest>10;
print JSON::PP->new->canonical->pretty->encode({files=>$files,directories=>$dirs,logical_bytes=>$bytes,allocated_bytes=>$allocated,classes=>\%classes,scope_summary=>\%summary,flags=>\@flags,largest_unmatched=>\@largest});
