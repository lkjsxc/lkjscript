package CompilerClosure;
use strict; use warnings;
use Archive::Tar;
use Digest::SHA qw(sha256_hex);
use Fcntl qw(O_RDONLY O_WRONLY O_CREAT O_EXCL O_NOFOLLOW O_DIRECTORY :mode);
use File::Basename qw(dirname);
use IO::Uncompress::Gunzip qw(gunzip $GunzipError);
use JSON::PP;
use Time::HiRes ();
our $MAIN='/home/coder/workspace/lkjscript';
our $OWN='/home/coder/workspace/lkjscript-compiler-scale-refusal-20261116';
our $QUERY='/home/coder/workspace/lkjscript-compiler-query-container-20261116';
our $NATIVE='/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116';
our $PREP="$OWN/target/compiler-completed-evidence-closure-20261116";
our $BUNDLE='bench/compiler-completed-evidence-closure-data';
our $RESULT="$MAIN/bench/compiler-completed-evidence-closure-result";
our ($RELEASE,$PREPARING)=(undef,0);
our @OLD_ROOTS=(
 (map {"$MAIN/target/$_"} qw(compiler-copy-sites-evidence.20261116-2 compiler-copy-sites-run.20261116-2 compiler-copy-sites-census-output.20261116-1 compiler-copy-sites-census-evidence.20261116-1)),
 (map {"$OWN/target/$_"} qw(compiler-copy-sites-docs compiler-copy-sites-measurement-plan compiler-copy-sites-ci.43872738366.M8M145 compiler-copy-sites-preflight-diagnostic.kGyrqc compiler-copy-sites-acceptance.20261116-2.FFPpPx compiler-copy-sites-census-audit.20261116-1.fIdBSp compiler-copy-sites-fn5719-review.20261116-1.bcni8c)),
 (map {"$QUERY/target/$_"} qw(compiler-copy-sites-profiler-candidate compiler-copy-sites-accounting-candidate compiler-copy-sites-extractor-candidate compiler-copy-sites-extractor-package-20261116 compiler-copy-sites-census-join-20261116 compiler-copy-sites-census-join-package-20261116 compiler-copy-sites-family-census-20261116 compiler-copy-sites-family-census-package-20261116)),
 (map {"$NATIVE/target/$_"} qw(compiler-copy-sites-cleanup-20261116 compiler-legacy-root-inventory-20261116 compiler-ci-43878716520-20261116 compiler-copy-sites-root-inventory-20261116 compiler-ci-964be9a4-20261116 compiler-copy-sites-cleanup-guard-v2-20261116))
);
our @MIRRORS=(
 {path=>"$QUERY/target/compiler-query-worktree-closure-proposal-20261116",kind=>'d',prefix=>'bench/compiler-query-temp-closure-data'},
 {path=>"$NATIVE/target/compiler-native-temp-closure-20261116",kind=>'d',prefix=>'bench/compiler-native-temp-closure-data'},
 {path=>"$OWN/target/compiler-copy-sites-closure-proposal.md",kind=>'f',prefix=>"$BUNDLE/eleven-scope-proposal.md"}
);
our $JSON=JSON::PP->new->canonical->utf8;
sub json { return $JSON->encode($_[0]); }
sub pretty { return JSON::PP->new->canonical->utf8->pretty->encode($_[0]); }
sub clean_absolute {
 my($p)=@_; die "nonliteral path $p\n" unless $p=~m{\A/} && $p!~m{[\x00\r\n\t]|//|(?:\A|/)\.\.?(?:/|\z)|/\z};
 return $p;
}
sub clean_relative {
 my($p)=@_; die "unsafe Git/member path\n" unless defined($p) && length($p) && $p!~m{\A/|[\x00\r\n\t:]|(?:\A|/)\.\.?(?:/|\z)|//|/\z};
 return $p;
}
sub metadata {
 my($p,$fh)=@_; my @s=defined($fh)?Time::HiRes::stat($fh):Time::HiRes::lstat($p);
 die "stat $p: $!\n" unless @s;
 my $kind=S_ISREG($s[2])?'f':S_ISDIR($s[2])?'d':'other';
 die "symlink or special entry $p\n" if $kind eq 'other';
 die "multiple regular-file links $p\n" if $kind eq 'f' && $s[3]!=1;
 return {kind=>$kind,device=>"$s[0]",inode=>"$s[1]",mode=>"$s[2]",links=>"$s[3]",uid=>"$s[4]",gid=>"$s[5]",
  bytes=>"$s[7]",mtime=>sprintf('%.9f',$s[9]),ctime=>sprintf('%.9f',$s[10])};
}
sub same_metadata {
 my($a,$b,$label)=@_; for my$k(qw(kind device inode mode links uid gid bytes mtime ctime)){die "metadata changed: $label ($k)\n" unless defined($a->{$k})&&defined($b->{$k})&&$a->{$k} eq $b->{$k};}
}
sub identity {
 my($m)=@_; return {map {$_=>$m->{$_}} qw(kind device inode mode uid gid)};
}
sub same_identity {
 my($a,$b,$label)=@_; for my$k(qw(kind device inode mode uid gid)){die "parent identity changed: $label ($k)\n" unless defined($a->{$k})&&defined($b->{$k})&&$a->{$k} eq $b->{$k};}
}
sub parent_paths {
 my($p)=@_; my @a; for(my $d=dirname($p);; $d=dirname($d)){push @a,$d;last if $d eq '/';} return @a;
}
sub check_parents {
 my($p,$expected)=@_;
 for my $d(parent_paths($p)){my $m=metadata($d);die "non-directory parent $d\n" unless $m->{kind} eq 'd';
  same_identity($m,$expected->{$d},$d) if defined($expected) && exists $expected->{$d};
  die "unrecorded parent $d\n" if defined($expected) && !exists $expected->{$d};
 }
}
sub open_file {
 my($p,$expected)=@_; clean_absolute($p); my $before=metadata($p);
 die "not a regular file $p\n" unless $before->{kind} eq 'f';
 same_metadata($before,$expected,$p) if defined $expected;
 sysopen(my $fh,$p,O_RDONLY|O_NOFOLLOW) or die "open $p: $!\n"; binmode($fh);
 same_metadata(metadata($p,$fh),$before,$p);
 return ($fh,$before);
}
sub read_checked {
 my($p,$expected)=@_; check_parents($p,undef);my($fh,$before)=open_file($p,$expected);
 local $/;my $data=<$fh>; $data='' unless defined $data;
 die "length changed $p\n" unless length($data)==$before->{bytes};
 same_metadata(metadata($p,$fh),$before,$p);same_metadata(metadata($p),$before,$p);
 close($fh) or die "close $p: $!\n";return($data,$before);
}
sub bytes { return (read_checked($_[0],undef))[0]; }
sub load_json { return decode_json(bytes($_[0])); }
sub save {
 my($path,$data)=@_;clean_absolute($path);
 die "output outside preparation/result\n" unless index($path,"$PREP/")==0 || index($path,"$RESULT/")==0;
 check_parents($path,undef);sysopen(my $f,$path,O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW,0600) or die "claim $path: $!\n";
 binmode($f); print {$f} $data or die "write $path: $!\n";close($f) or die "close $path: $!\n";
}
sub scan {
 my($roots)=@_;my(%files,%dirs,%ancestors);
 for my $r(@$roots){clean_absolute($r->{path});check_parents($r->{path},undef);
  my @todo=($r->{path});
  while(@todo){my $p=pop @todo;my$m=metadata($p);
   die "root kind $p\n" if $p eq $r->{path} && $m->{kind} ne $r->{kind};
   if($m->{kind} eq 'f'){die "overlapping file $p\n" if exists$files{$p};$files{$p}=$m;next;}
   die "overlapping directory $p\n" if exists$dirs{$p};$dirs{$p}=$m;
   sysopen(my$pin,$p,O_RDONLY|O_DIRECTORY|O_NOFOLLOW) or die "open directory $p: $!\n";
   same_metadata(metadata($p,$pin),$m,$p);
   opendir(my$d,$p) or die "enumerate $p: $!\n";
   my @names=sort grep {$_ ne '.' && $_ ne '..'} readdir($d);closedir($d) or die "close directory $p\n";
   same_metadata(metadata($p),$m,$p);same_metadata(metadata($p,$pin),$m,$p);close($pin)or die"directory pin close\n";
   for my$n(reverse @names){die "unsafe entry $p\n" if $n=~m{[\x00\r\n\t/]} || $n eq '.' || $n eq '..';push @todo,"$p/$n";}
  }
 }
 for my$p(keys%files,keys%dirs){for my$d(parent_paths($p)){$ancestors{$d}=$dirs{$d}//metadata($d);die"parent type\n"unless$ancestors{$d}{kind}eq'd';}}
 return {files=>\%files,dirs=>\%dirs,parents=>\%ancestors};
}
sub git_bytes {
 my($rev,$path)=@_;clean_relative($path);die "revision format\n" unless $rev=~/\A[0-9a-f]{40}\z/;
 open(my$f,'-|','git','-C',$MAIN,'cat-file','blob',"$rev:$path")or die"Git read open\n";binmode$f;local$/;my$b=<$f>;
 close($f)or die"Git blob failed $rev:$path\n";return$b//'';
}
sub release_bytes {
 my($path)=@_;clean_relative($path);
 if($PREPARING){
  return bytes("$PREP/".substr($path,length($BUNDLE)+1)) if index($path,"$BUNDLE/")==0;
  return bytes("$MAIN/$path") if $path=~m{\Abench/compiler-(?:query|native)-temp-closure-data/};
  open(my$f,'-|','git','-C',$MAIN,'rev-parse','HEAD')or die"Git HEAD\n";my$r=<$f>;close$f or die"Git HEAD\n";chomp$r;
  return git_bytes($r,$path);
 }
 die "no pinned release\n" unless defined$RELEASE;return git_bytes($RELEASE,$path);
}
my(%archives,%containers,%unzipped);
sub recovery_bytes {
 my($r)=@_;die"disposable has no recovery\n"if$r->{kind}eq'disposable';
 my$key=$r->{revision}.':'.$r->{path};
 my$raw;
 if($r->{kind}eq'archive'||$r->{kind}eq'gunzip'){
  $containers{$key}//=($r->{revision}eq'release'?release_bytes($r->{path}):git_bytes($r->{revision},$r->{path}));$raw=$containers{$key};
 }else{$raw=$r->{revision}eq'release'?release_bytes($r->{path}):git_bytes($r->{revision},$r->{path});}
 die"container digest differs\n"if defined($r->{container_sha256})&&sha256_hex($raw)ne$r->{container_sha256};
 if($r->{kind}eq'blob'){return$raw;}
 if($r->{kind}eq'range'){
  die"range bounds\n"unless$r->{offset}=~/\A[0-9]+\z/&&$r->{length}=~/\A[0-9]+\z/&&$r->{offset}+$r->{length}<=length($raw);
  return substr($raw,$r->{offset},$r->{length});
 }
 if($r->{kind}eq'gunzip'){if(!exists$unzipped{$key}){my$out='';gunzip \$raw=>\$out or die"gzip recovery $GunzipError\n";$unzipped{$key}=$out;}return$unzipped{$key};}
 if($r->{kind}eq'archive'){
  my$key=$r->{revision}.':'.$r->{path}.':'.sha256_hex($raw);
  die"archive pin differs\n"if defined($r->{archive_sha256})&&sha256_hex($raw)ne$r->{archive_sha256};
  if(!exists$archives{$key}){my$plain='';gunzip \$raw=>\$plain or die"archive gzip $GunzipError\n";
   open(my$fh,'<',\$plain)or die"archive scalar open\n";binmode$fh;my$tar=Archive::Tar->new;$tar->read($fh,0)or die"archive read\n";close$fh or die"archive close\n";
   my%members;for my$m($tar->get_files){next if$m->is_dir;die"archive special entry\n"unless$m->is_file;
    my$n=$m->full_path;$n=~s{\A\./}{};clean_relative($n);die"duplicate archive member\n"if exists$members{$n};$members{$n}=$m->get_content;}
   $archives{$key}=\%members;
  }
  my$n=$r->{member};$n=~s{\A\./}{};clean_relative($n);die"missing archive member $n\n"unless exists$archives{$key}{$n};return$archives{$key}{$n};
 }
 die"unknown recovery kind $r->{kind}\n";
}
sub verify_record {
 my($record,$parents)=@_;my$p=$record->{path};check_parents($p,$parents);
 my($data)=read_checked($p,$record->{metadata});
 die "source digest differs $p\n" unless length($data)==$record->{bytes}&&sha256_hex($data)eq$record->{sha256};
 if($record->{recovery}{kind}ne'disposable'){my$recover=recovery_bytes($record->{recovery});die"recovery bytes differ $p\n"unless$data eq$recover;}
 return length($data);
}
sub release_tree {
 my($prefix)=@_;clean_relative($prefix);die"no release\n"unless defined$RELEASE;
 open(my$f,'-|','git','-C',$MAIN,'ls-tree','-r','-z','--full-tree',$RELEASE,'--',$prefix)or die"Git tree open\n";
 local$/;my$raw=<$f>;close$f or die"Git tree\n";my@paths;
 for my$row(split/\0/,$raw//''){die"unexpected Git tree entry\n"unless$row=~/\A(100644|100755) blob [0-9a-f]{40}\t(.+)\z/s;
  my$p=$2;die"tree escaped prefix\n"unless index($p,"$prefix/")==0;clean_relative($p);push@paths,$p;}
 die"empty committed tree\n"unless@paths;return sort@paths;
}
1;
