#!/usr/bin/env perl
use strict;
use warnings;
use Fcntl qw(:DEFAULT :mode);
use Errno qw(ENOENT);
use Cwd qw(getcwd);
use File::Find;
use Digest::SHA qw(sha1_hex sha256_hex);
use JSON::PP;
use IO::Uncompress::Gunzip;
use IO::Handle;
use Time::HiRes qw(time);
my $REPO='/home/coder/workspace/lkjscript';
my $PREP="$REPO/target/compiler-reference-capture-cleanup-preparation-2";
my $SELF="$PREP/guard.pl";
my $RELEASE="$PREP/release.json";
my $CAPTURE="$REPO/target/compiler-reference-capture-cleanup-execution-2";
my $MAP="$REPO/target/compiler-reference-capture-recovery-map-1.json";
my $PROOF="$REPO/target/compiler-reference-capture-recovery-proof-1.sha256";
my $MAP_SHA='ff159c384304fb66cd5aa644a06485a5cec2bcc4aab393ccd679a3b7f6faa4bd';
my $PROOF_SHA='347e1900c21920ae9b0b6bfad44edd117721c45e4c0765019b90c74ec85c8729';
my $MAP_COMMIT='d84cac6edcee59e2c2ad631fb6ca15d63340a1c8';
my $RECOVERY_COMMIT='e68ea0eb29b755cb48df5034c57fb226a1f7c3c6';
my $BLOB_BASE='bench/compiler-reference-capture-data/';
my $MAP_BASE=$BLOB_BASE.'cleanup-mapping/';
my @SEEDS=('7d0731e99aeaf241c4334fc5ff065a341fb388c8','e5294c339d9aefbbafc2e951c997f4fefabcc9ad');
my %SEED=(
 $SEEDS[0]=>['505be947525d53b4db1bff6c85cb77622cb2f5dc7e7d7fe92b772ee3b8ffc3e2','6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc',32584542],
 $SEEDS[1]=>['c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae','b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db',32588975]
);
my @ROOTS=map {'compiler-reference-capture-'.$_} qw(
 adoption-acceptance-1 bootstrap-1 cache-cleanup-1 cache-cleanup-acceptance-1 ci-1
 commit-acceptance-1 core-check-1 core-operation-1 desired-1 gate-1 gate-acceptance-1
 integration-preparation-1 measurement-1 preliminary-retention-1 preparation-1 query-1
 query-acceptance-1 seed-1 semantics-1 semantics-2 semantics-3 semantics-4
 setup-acceptance-1 setup-acceptance-2 setup-launch-1 setup-launch-2
 setup-preparation-1 setup-preparation-2 tests-operation-1
);
my @KEEP=map {'compiler-reference-capture-'.$_} qw(
 measurement-preparation-1 measurement-2 trial-preparation-2 comparison-2 comparison-launch-2
);
my %ALLOWED_ROOT=map {$_=>1} @ROOTS;
my %KEPT=map {$_=>1} @KEEP;
my $J=JSON::PP->new->canonical->utf8;
my (%git_cache,%archive_cache,%expanded_cache,%recover_cache,%held_dirs);
my ($log,$capture_started,$mutation_started)=(undef,0,0);
my (@removed_files,@removed_dirs,@completed_roots);
my $start=time;
$|=1;
sub fail { die "$_[0]\n"; }
sub equal { fail($_[2]) unless $_[0] eq $_[1]; }
sub safe_relative {
 my($p)=@_;fail('unsafe relative path')unless defined($p)&&!ref($p)&&$p ne''&&$p=~m{\A[A-Za-z0-9_./-]+\z}&&$p!~m{\A/};
 fail('unsafe relative component')if grep {$_ eq''||$_ eq'.'||$_ eq'..'}split m{/},$p,-1;
 return $p;
}
sub uint {
 my($n,$max,$why)=@_;fail($why)unless defined($n)&&!ref($n)&&"$n"=~/\A(?:0|[1-9][0-9]*)\z/&&length("$n")<=16&&$n<=$max;return 0+$n;
}
sub sha_field { fail('bad SHA-256 field')unless defined($_[0])&&!ref($_[0])&&$_[0]=~/\A[0-9a-f]{64}\z/;return $_[0]; }
sub oid_field { fail('bad Git object field')unless defined($_[0])&&!ref($_[0])&&$_[0]=~/\A[0-9a-f]{40}\z/;return $_[0]; }
sub array_equal {
 my($a,$b,$why)=@_;fail($why)unless ref($a)eq'ARRAY'&&ref($b)eq'ARRAY'&&@$a==@$b;
 for my$i(0..$#$a){fail($why)unless defined($a->[$i])&&defined($b->[$i])&&!ref($a->[$i])&&!ref($b->[$i])&&"$a->[$i]"eq"$b->[$i]";}
}
sub stat_root { my($p)=@_;my@s=lstat$p;fail("root missing/type $p")unless@s&&S_ISDIR($s[2])&&$s[4]==$>;return [0+$s[4],0+$s[5],sprintf('%04o',$s[2]&07777),0+$s[0],0+$s[1]]; }
sub stat_entry {
 my($p,$rel)=@_;my@s=lstat$p;fail("missing entry $p")unless@s;
 return ([ $rel,0+$s[4],0+$s[5],sprintf('%04o',$s[2]&07777),0+$s[7],0+$s[3],0+$s[0],0+$s[1] ],\@s);
}
sub bytes {
 my($p,$cap)=@_;$cap=134217728 unless defined$cap;
 sysopen my$f,$p,O_RDONLY|O_NOFOLLOW or fail("open $p: $!");
 my@s=stat$f;fail("unsafe input $p")unless@s&&S_ISREG($s[2])&&$s[4]==$>&&$s[3]==1&&$s[7]<=$cap;
 binmode$f;my$b='';while(1){my$n=sysread($f,my$part,65536);fail("read $p: $!")unless defined$n;last unless$n;$b.=$part;fail("input cap $p")if length($b)>$cap;}
 my@e=stat$f;close$f or fail("close $p: $!");
 fail("input changed while read $p")unless length($b)==$s[7]&&join(',',@s[0,1,2,3,4,5,7,9,10])eq join(',',@e[0,1,2,3,4,5,7,9,10]);
 return($b,\@s);
}
sub save_new {
 my($name,$b)=@_;safe_relative($name);fail('capture filename nesting')if$name=~m{/};
 sysopen my$f,"$CAPTURE/$name",O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW,0600 or fail("capture $name: $!");
 binmode$f;print{$f}$b or fail("capture write $name: $!");close$f or fail("capture close $name: $!");
}
sub event {
 my($type,$fields)=@_;return unless$log;
 print{$log}$J->encode({event=>$type,elapsed_seconds=>time-$start,%{$fields||{}}})."\n" or fail("progress capture: $!");
 $log->flush or fail("progress flush: $!");
}
sub git {
 my(@args)=@_;open my$f,'-|','/usr/bin/git','-C',$REPO,@args or fail('Git launch');
 binmode$f;local$/;my$b=<$f>;close$f or fail('Git refusal');return defined($b)?$b:'';
}
sub git_blob {
 my($commit,$path)=@_;oid_field($commit);safe_relative($path);
 my$k="$commit:$path";return @{$git_cache{$k}}if exists$git_cache{$k};
 my$id=git('rev-parse','--verify',$k);chomp$id;oid_field($id);
 my$b=git('cat-file','blob',$id);fail('Git blob cap')if length($b)>134217728;
 equal(sha1_hex('blob '.length($b)."\0".$b),$id,'Git blob contents disagree');
 $git_cache{$k}=[$b,$id];return($b,$id);
}
sub inflate {
 my($compressed,$cap)=@_;my$z=IO::Uncompress::Gunzip->new(\$compressed,MultiStream=>0)or fail('gzip open');
 my$out='';while(1){my$n=$z->read(my$part,65536);fail('gzip read')if$n<0;last unless$n;fail('gzip extent cap')if length($out)+$n>$cap;$out.=$part;}
 $z->close or fail('gzip close');return$out;
}
sub tar_text {
 my($x)=@_;fail('tar NUL padding')unless$x=~/\A([^\0]*)\0*\z/s;return$1;
}
sub octal {
 my($x)=@_;$x=~s/[\0 ]+\z//;$x=~s/\A +//;fail('tar octal')unless$x=~/\A[0-7]+\z/&&length($x)<=12;my$n=oct$x;fail('tar integer cap')if$n>134217728;return$n;
}
sub pax {
 my($b)=@_;my(%p,$at);
 $at=0;while($at<length$b){my$tail=substr($b,$at);fail('PAX framing')unless$tail=~/\A([1-9][0-9]*) /;my$n=uint($1,1048576,'PAX length');fail('PAX extent')if$n>length($b)-$at;
 my$record=substr($b,$at,$n);fail('PAX record')unless$record=~/\A[1-9][0-9]* ([A-Za-z0-9_.-]+)=([^\n]*)\n\z/s;
 my($key,$value)=($1,$2);fail('duplicate PAX field')if exists$p{$key};
 fail('unsupported PAX field')unless$key=~/\A(?:path|mtime|atime|ctime|uid|gid|uname|gname|comment)\z/;
 $p{$key}=$value;$at+=$n;}return\%p;
}
sub tar_members {
 my($raw)=@_;fail('tar alignment')unless length($raw)%512==0;my(%members,%names,$pending,$ended);my$at=0;
 while($at+512<=length$raw){
  my$h=substr($raw,$at,512);
  if($h eq"\0"x512){fail('tar terminator')unless length($raw)-$at>=1024&&!defined$pending&&substr($raw,$at)eq"\0"x(length($raw)-$at);$ended=1;last;}
  my$magic=substr($h,257,8);fail('tar format')unless$magic eq"ustar\00000"||$magic eq"ustar  \0";
  my$check=$h;substr($check,148,8)=' 'x8;fail('tar checksum')unless unpack('%32C*',$check)==octal(substr($h,148,8));
  my$name=tar_text(substr($h,0,100));if($magic eq"ustar\00000"){my$p=tar_text(substr($h,345,155));$name="$p/$name"if$p ne'';}
  my$size=octal(substr($h,124,12));my$type=substr($h,156,1);$type='0'if$type eq"\0";
  fail('tar link target')unless tar_text(substr($h,157,100))eq'';
  my$end=$at+512+512*int(($size+511)/512);fail('tar extent')if$end>length$raw;
  my$content=substr($raw,$at+512,$size);fail('tar payload padding')unless substr($raw,$at+512+$size,$end-$at-512-$size)eq"\0"x($end-$at-512-$size);
  if($type eq'L'){
   fail('nested GNU long path')if defined$pending;fail('GNU long path')unless$content=~/\A([^\0]+)\0\z/s;$pending=$1;
  }elsif($type eq'x'||$type eq'g'){
   fail('nested path extension')if defined$pending;my$p=pax($content);
   fail('global path extension')if$type eq'g'&&exists$p->{path};
   $pending=$p->{path}if exists$p->{path};
  }elsif($type eq'0'||$type eq'5'){
   $name=$pending if defined$pending;$pending=undef;
   $name=~s{\A\./}{};$name=~s{/\z}{} if$type eq'5';
   if($name eq''||$name eq'.'){fail('non-directory root header')unless$type eq'5';$at=$end;next;}
   safe_relative($name);fail('duplicate tar member')if$names{$name}++;
   if($type eq'5'){fail('directory payload')if$size;}else{$members{$name}=$content;}
  }else{fail("unsupported tar member type $type");}
  $at=$end;
 }
 fail('unterminated archive')unless$ended;return\%members;
}
sub recovery_bytes {
 my($d)=@_;fail('recovery object')unless ref($d)eq'HASH';
 my$encoded=$J->encode($d);return$recover_cache{$encoded}if exists$recover_cache{$encoded};
 my$commit=oid_field($d->{commit});my$path=safe_relative($d->{path});my$want=sha_field($d->{sha256});my$len=uint($d->{bytes},134217728,'recovery size');
 fail('unapproved recovery location')unless($commit eq$RECOVERY_COMMIT&&index($path,$BLOB_BASE)==0)||exists($SEED{$commit})&&$path eq'seed/lkjscript.gz';
 my($source,$id)=git_blob($commit,$path);my$out;
 if($d->{kind}eq'committed-git-blob'){
  equal($id,oid_field($d->{git_blob}),'recovery blob identity');$out=$source;
 }elsif($d->{kind}eq'committed-gzip-expansion'){
  fail('unapproved expansion')unless exists($SEED{$commit})&&$path eq'seed/lkjscript.gz';
  equal($id,oid_field($d->{gzip_git_blob}),'seed blob identity');
  equal(sha256_hex($source),sha_field($d->{gzip_sha256}),'seed descriptor hash');
  equal(sha256_hex($source),$SEED{$commit}[0],'seed fixed compressed hash');
  if(!exists$expanded_cache{$commit}){$expanded_cache{$commit}=inflate($source,$SEED{$commit}[2]);}
  $out=$expanded_cache{$commit};fail('seed fixed length')unless length($out)==$SEED{$commit}[2];equal(sha256_hex($out),$SEED{$commit}[1],'seed fixed expansion hash');
 }elsif($d->{kind}eq'committed-gzip-tar-member'){
  fail('unapproved member archive')unless$commit eq$RECOVERY_COMMIT&&($path eq$BLOB_BASE.'preliminary/preliminary-roots.tar.gz'||$path eq$BLOB_BASE.'gate-acceptance/raw-logs.tar.gz');
  equal($id,oid_field($d->{archive_git_blob}),'archive blob identity');equal(sha256_hex($source),sha_field($d->{archive_sha256}),'archive hash');
  my$member=safe_relative($d->{member});
  if(!exists$archive_cache{$path}){$archive_cache{$path}=tar_members(inflate($source,134217728));}
  fail('archive member missing')unless exists$archive_cache{$path}{$member};$out=$archive_cache{$path}{$member};
 }else{fail('unsupported recovery kind');}
 fail('recovery length')unless length($out)==$len;equal(sha256_hex($out),$want,'recovery payload hash');$recover_cache{$encoded}=$out;return$out;
}
sub hold_directory {
 my($path,$expect,$root)=@_;
 return if exists$held_dirs{$path};
 my$got=stat_root($path);
 my$wanted=$root?$expect:[$expect->[1],$expect->[2],$expect->[3],$expect->[6],$expect->[7]];
 array_equal($got,$wanted,'directory identity/mode/owner mismatch');
 sysopen my$f,$path,O_RDONLY|O_DIRECTORY|O_NOFOLLOW or fail("hold directory $path: $!");
 my@s=stat$f;array_equal([0+$s[4],0+$s[5],sprintf('%04o',$s[2]&07777),0+$s[0],0+$s[1]],$wanted,'held directory identity mismatch');
 $held_dirs{$path}={handle=>$f,identity=>$wanted};
}
sub held_identity {
 my($p)=@_;fail('directory not held')unless exists$held_dirs{$p};
 array_equal(stat_root($p),$held_dirs{$p}{identity},'directory path drift');
 my@s=stat($held_dirs{$p}{handle});array_equal([0+$s[4],0+$s[5],sprintf('%04o',$s[2]&07777),0+$s[0],0+$s[1]],$held_dirs{$p}{identity},'held directory drift');
}
sub parents_unchanged {
 my($root,$rel)=@_;held_identity($root);my@parts=split m{/},$rel;pop@parts;my$p=$root;for my$n(@parts){$p.="/$n";held_identity($p);}
}
sub validate_file {
 my($root,$row,$recovery)=@_;my$rel=$row->[0];parents_unchanged($root,$rel);
 my($meta,$s)=stat_entry("$root/$rel",$rel);fail('unsafe file before read')unless S_ISREG($s->[2])&&$s->[4]==$>&&$s->[3]==1;
 array_equal($meta,[@$row[0..7]],'file metadata drift');
 my($b,$st)=bytes("$root/$rel");array_equal([$rel,0+$st->[4],0+$st->[5],sprintf('%04o',$st->[2]&07777),0+$st->[7],0+$st->[3],0+$st->[0],0+$st->[1]],[@$row[0..7]],'opened file metadata drift');
 equal(sha256_hex($b),$row->[8],'file content drift');equal($b,$recovery,'original differs from independently verified recovery');
 parents_unchanged($root,$rel);
 my($again)=stat_entry("$root/$rel",$rel);array_equal($again,[@$row[0..7]],'file path changed after read');
}
sub population {
 my($r,$check_bytes,$catalog)=@_;my$root=$r->{path};array_equal(stat_root($root),$r->{root_metadata},'root metadata drift');
 hold_directory($root,$r->{root_metadata},1);
 my(%want_files,%want_dirs);for my$row(@{$r->{rows}}){$want_files{$row->[0]}=$row;}for my$d(@{$r->{directories}}){$want_dirs{$d->[0]}=$d;}
 my(%seen_files,%seen_dirs);
 find({no_chdir=>1,follow=>0,wanted=>sub{
  my$p=$File::Find::name;return if$p eq$root;my$rel=substr($p,length($root)+1);safe_relative($rel);my($m,$s)=stat_entry($p,$rel);
  if(S_ISDIR($s->[2])){
   fail('extra directory')unless exists$want_dirs{$rel};array_equal($m,$want_dirs{$rel},'directory metadata drift');$seen_dirs{$rel}=1;hold_directory($p,$want_dirs{$rel},0);
  }else{
   fail('nonregular entry')unless S_ISREG($s->[2])&&$s->[3]==1&&$s->[4]==$>;
   fail('extra file')unless exists$want_files{$rel};array_equal($m,[@{$want_files{$rel}}[0..7]],'file population metadata drift');$seen_files{$rel}=1;
  }
 }},$root);
 fail('missing file/directory population')unless keys(%seen_files)==keys(%want_files)&&keys(%seen_dirs)==keys(%want_dirs);
 if($check_bytes){for my$row(@{$r->{rows}}){my$b=recovery_bytes($catalog->[$row->[9]]);validate_file($root,$row,$b);}}
}
sub main {
 equal(getcwd(),$REPO,'run from exact repository cwd');
 fail('exact execution arguments required')unless@ARGV==3&&$ARGV[0]eq'--release'&&$ARGV[1]eq$RELEASE&&$ARGV[2]eq'--execute-after-root-release';
 my($self)=bytes($SELF,1048576);my$self_sha=sha256_hex($self);
 my($release_bytes)=bytes($RELEASE,65536);my$rel=decode_json($release_bytes);
 fail('release schema/authorization')unless ref($rel)eq'HASH'&&$rel->{schema}eq'compiler-reference-capture-cleanup-release-2'&&$rel->{execute}&&$rel->{cooperative_quiescence_confirmed}&&$rel->{python_violation_acknowledged};
 equal($rel->{guard_sha256},$self_sha,'released guard hash');equal($rel->{map_sha256},$MAP_SHA,'released map hash');equal($rel->{proof_sha256},$PROOF_SHA,'released proof hash');
 equal($rel->{map_commit},$MAP_COMMIT,'released map commit');equal($rel->{recovery_commit},$RECOVERY_COMMIT,'released recovery commit');
 equal($rel->{capture_root},$CAPTURE,'capture scope');array_equal($rel->{roots},\@ROOTS,'release root list');array_equal($rel->{seed_commits},\@SEEDS,'release seed list');
 fail('release exact counts')unless$rel->{expected_files}==3050&&$rel->{expected_logical_bytes}==160418647;
 my$approval=oid_field($rel->{approval_commit});my$guard_path=safe_relative($rel->{approved_guard_git_path});
 fail('approved guard outside retained evidence')unless index($guard_path,$BLOB_BASE)==0&&$guard_path=~/\/guard\.pl\z/;
 my($approved_guard)=git_blob($approval,$guard_path);equal($approved_guard,$self,'committed reviewed guard differs');
 my($map_bytes)=bytes($MAP,4194304);my($proof_bytes)=bytes($PROOF,65536);
 equal(sha256_hex($map_bytes),$MAP_SHA,'live map pin');equal(sha256_hex($proof_bytes),$PROOF_SHA,'live proof pin');
 my($committed_map)=git_blob($MAP_COMMIT,$MAP_BASE.'compiler-reference-capture-recovery-map-1.json');
 my($committed_proof)=git_blob($MAP_COMMIT,$MAP_BASE.'compiler-reference-capture-recovery-proof-1.sha256');
 equal($map_bytes,$committed_map,'committed map differs');equal($proof_bytes,$committed_proof,'committed proof differs');
 my@proof_lines=split/\n/,$proof_bytes;fail('proof row count')unless@proof_lines==7;my%proof_paths;
 for my$l(@proof_lines){fail('proof framing')unless$l=~/\A([0-9a-f]{64})  target\/(compiler-reference-capture-[A-Za-z0-9_.-]+)\z/;
  my($hash,$name)=($1,$2);fail('duplicate proof path')if$proof_paths{$name}++;my$stored=$name;$stored.='.txt'if$name eq'compiler-reference-capture-recovery-proposal-1.py';
  my($b)=git_blob($MAP_COMMIT,$MAP_BASE.$stored);equal(sha256_hex($b),$hash,'retained proof member hash');
 }
 my$map=decode_json($map_bytes);fail('map framing')unless ref($map)eq'HASH'&&$map->{schema}==1&&ref($map->{roots})eq'ARRAY'&&ref($map->{file_rows})eq'ARRAY'&&ref($map->{recovery_catalog})eq'ARRAY';
 array_equal($map->{file_row_fields},[qw(relative_path uid gid mode_octal4 size nlink dev inode sha256 recovery_catalog_index)],'map row schema');
 my%by_name;for my$r(@{$map->{roots}}){fail('map root record')unless ref($r)eq'HASH'&&defined$r->{name}&&!ref$r->{name};fail('duplicate map root')if$by_name{$r->{name}};$by_name{$r->{name}}=$r;}
 my(@chosen,%used_rows,$count,$total);$count=0;$total=0;
 for my$name(@ROOTS){
  fail('selected scope overlaps kept root')if$KEPT{$name};my$r=$by_name{$name};fail('missing selected literal root')unless$r;
  equal($r->{path},"$REPO/target/$name",'selected absolute path');
  my$at=uint($r->{file_row_start},scalar(@{$map->{file_rows}}),'row start');my$n=uint($r->{file_row_count},100000,'row count');
  fail('row range')if$at+$n>@{$map->{file_rows}};my@rows;my%names;
  for my$i($at..$at+$n-1){fail('overlapping root rows')if$used_rows{$i}++;my$row=$map->{file_rows}[$i];
   fail('file row')unless ref($row)eq'ARRAY'&&@$row==10;safe_relative($row->[0]);fail('duplicate relative file')if$names{$row->[0]}++;
   uint($row->[$_],9007199254740991,'file metadata integer')for(1,2,4,5,6,7,9);
   fail('file owner/link/mode')unless$row->[1]==$>&&$row->[5]==1&&$row->[3]=~/\A0[0-7]{3}\z/;
   sha_field($row->[8]);fail('recovery index')if$row->[9]>=@{$map->{recovery_catalog}};
   fail('recovery row disagreement')unless$map->{recovery_catalog}[$row->[9]]{bytes}==$row->[4]&&$map->{recovery_catalog}[$row->[9]]{sha256}eq$row->[8];
   $total+=$row->[4];$count++;push@rows,$row;
  }
  fail('root directory rows')unless ref($r->{directories})eq'ARRAY';for my$d(@{$r->{directories}}){fail('directory row')unless ref($d)eq'ARRAY'&&@$d==8;safe_relative($d->[0]);fail('duplicate path')if$names{$d->[0]}++;}
  fail('root metadata')unless ref($r->{root_metadata})eq'ARRAY'&&@{$r->{root_metadata}}==5;
  push@chosen,{%$r,rows=>[sort{$a->[0]cmp$b->[0]}@rows]};
 }
 fail('selected total mismatch')unless@chosen==29&&$count==3050&&$total==160418647;
 fail('capture already exists or inaccessible')if lstat$CAPTURE||$!!=ENOENT;
 mkdir$CAPTURE,0700 or fail("new capture: $!");$capture_started=1;
 sysopen$log,"$CAPTURE/progress.jsonl",O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW,0600 or fail("progress: $!");$log->autoflush(1);
 save_new('release.json',$release_bytes);save_new('invocation.json',$J->encode({argv=>[$^X,$SELF,@ARGV],cwd=>getcwd(),guard_sha256=>$self_sha,map_sha256=>$MAP_SHA,proof_sha256=>$PROOF_SHA,map_commit=>$MAP_COMMIT,recovery_commit=>$RECOVERY_COMMIT,roots=>\@ROOTS,excluded=>\@KEEP,started_epoch=>$start})."\n");
 event('begin',{selected_roots=>29,files=>$count,logical_bytes=>$total,prior_python_acceptance_trusted=>JSON::PP::false});
 for my$r(@chosen){population($r,1,$map->{recovery_catalog});event('root_preflight_verified',{root=>$r->{name},files=>scalar(@{$r->{rows}})});}
 for my$r(@chosen){population($r,0,$map->{recovery_catalog});}
 event('complete_preflight',{roots=>29,files=>$count,logical_bytes=>$total,independent_perl_recovery_and_byte_checks=>JSON::PP::true});
 $mutation_started=1;
 for my$r(@chosen){
  my$root=$r->{path};
  for my$row(@{$r->{rows}}){
   my$relpath=$row->[0];validate_file($root,$row,recovery_bytes($map->{recovery_catalog}[$row->[9]]));
   event('unlink_intent',{path=>"$root/$relpath",sha256=>$row->[8],bytes=>$row->[4]});
   unlink("$root/$relpath")==1 or fail("unlink stopped $root/$relpath: $!");
   push@removed_files,"$root/$relpath";event('unlinked',{path=>"$root/$relpath"});
  }
  my@dirs=sort {scalar(split m{/},$b->[0])<=>scalar(split m{/},$a->[0])||$b->[0]cmp$a->[0]}@{$r->{directories}};
  for my$d(@dirs){
   my$p="$root/$d->[0]";parents_unchanged($root,$d->[0]);held_identity($p);
   opendir my$dh,$p or fail("directory open: $!");my@left=grep{$_ ne'.'&&$_ ne'..'}readdir$dh;closedir$dh or fail('directory close');fail('directory nonempty after enumerated removals')if@left;
   event('rmdir_intent',{path=>$p});rmdir$p or fail("rmdir stopped $p: $!");push@removed_dirs,$p;event('removed_directory',{path=>$p});
  }
  held_identity($root);opendir my$dh,$root or fail("root open: $!");my@left=grep{$_ ne'.'&&$_ ne'..'}readdir$dh;closedir$dh or fail('root directory close');fail('root nonempty')if@left;
  event('rmdir_intent',{path=>$root});rmdir$root or fail("root removal stopped: $!");push@removed_dirs,$root;
  my@s=lstat$root;fail('root absence not ENOENT')if@s||$!!=ENOENT;push@completed_roots,$root;event('root_absent',{path=>$root,errno=>'ENOENT'});
 }
 for my$name(@ROOTS){my@s=lstat"$REPO/target/$name";fail('final absence not ENOENT')if@s||$!!=ENOENT;}
 event('complete',{removed_files=>scalar(@removed_files),removed_directories=>scalar(@removed_dirs),roots_absent=>scalar(@completed_roots)});
 close$log or fail('progress close');$log=undef;
 save_new('result.json',$J->encode({status=>'complete',removed_files=>scalar(@removed_files),removed_directories=>scalar(@removed_dirs),roots_absent=>\@completed_roots,excluded=>\@KEEP,elapsed_seconds=>time-$start,no_atomic_multi_root_guarantee=>JSON::PP::true})."\n");
 save_new('status',"0\n");print"cleanup complete: 29 roots, 3050 files, 160418647 bytes; enumerated only; all roots ENOENT\n";
}
my$ok=eval{main();1};
if(!$ok){
 my$error=$@||'unknown guard failure';chomp$error;
 if($capture_started){
  eval{event('stopped',{error=>$error,mutation_started=>$mutation_started?JSON::PP::true:JSON::PP::false,removed_files=>scalar(@removed_files),removed_directories=>scalar(@removed_dirs),completed_roots=>\@completed_roots});};
  eval{save_new('failure.json',$J->encode({status=>'stopped',error=>$error,mutation_started=>$mutation_started?JSON::PP::true:JSON::PP::false,removed_files=>\@removed_files,removed_directories=>\@removed_dirs,completed_roots=>\@completed_roots,elapsed_seconds=>time-$start})."\n");};
  eval{save_new('status',"1\n");};
 }
 warn"cleanup refused/stopped: $error\n";exit 1;
}
exit 0;
