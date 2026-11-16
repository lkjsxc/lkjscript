#!/usr/bin/env perl
use strict;
use warnings;
use Cwd qw(abs_path);
use Fcntl qw(:DEFAULT :mode);
use File::Find;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use Archive::Tar;
use IO::Compress::Gzip qw(gzip $GzipError);
use IO::Uncompress::Gunzip qw(gunzip $GunzipError);

my$repo='/home/coder/workspace/lkjscript';
my$base="$repo/target";
my$out="$base/compiler-reference-capture-comparison-recovery-2";
my$comparison='compiler-reference-capture-comparison-2';
my$outer='compiler-reference-capture-comparison-launch-2';
die "no arguments accepted\n" if@ARGV;
my$json=JSON::PP->new->canonical->pretty;
sub read_file {
    my($path)=@_;
    my@before=lstat$path;
    die "unsafe input $path\n" unless @before && S_ISREG($before[2]) && $before[3]==1 && $before[4]==$> && $before[7]<=134217728;
    sysopen my$f,$path,O_RDONLY|O_NOFOLLOW or die "open $path: $!\n";binmode$f;
    my@opened=stat$f;
    die "changed input identity\n" unless $opened[0]==$before[0] && $opened[1]==$before[1] && $opened[2]==$before[2] && $opened[7]==$before[7];
    local$/;my$b=<$f>;$b='' unless defined$b;my@after=stat$f;close$f or die$!;
    die "changed/short input\n" unless length($b)==$before[7] && $after[0]==$before[0] && $after[1]==$before[1]
        && $after[2]==$before[2] && $after[7]==$before[7] && $after[9]==$before[9] && $after[10]==$before[10];
    my@named=lstat$path;die "input pathname replaced\n" unless @named && $named[0]==$before[0] && $named[1]==$before[1] && $named[2]==$before[2] && $named[7]==$before[7];
    return$b;
}
sub write_file {
    my($name,$bytes)=@_;
    die "unsafe package name\n" unless$name=~/\A[a-z0-9][a-z0-9.-]*\z/;
    sysopen my$f,"$out/$name",O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW,0600 or die "create $name: $!\n";
    binmode$f;print{$f}$bytes or die$!;close$f or die$!;
}
sub write_json {write_file($_[0],$json->encode($_[1]));}
sub pin {return{bytes=>length($_[0]),sha256=>sha256_hex($_[0])};}
sub safe_path {
    my($p)=@_;
    die "unsafe path\n" unless defined($p)&&length($p)&&$p!~m{\A/|\\|[\x00-\x1f\x7f]};
    for(split m{/},$p,-1){die "unsafe component\n" if$_ eq''||$_ eq'.'||$_ eq'..';}
    die "credential-like path\n" if$p=~m{(?:^|[/_.-])(?:key|keys|token|tokens|credential|credentials|secret)(?:$|[/_.-])}i;
    return$p;
}
my%expected;
sub expect_file {my($p,$class)=@_;safe_path($p);die "duplicate expectation\n" if exists$expected{$p};$expected{$p}=$class;}
for(qw(controls.json controls.manifest.sha256 order.json protocol.md release.json run.sh trial.pl)){expect_file("$comparison/control/$_",'ordinary');}
for(qw(after-compiler.json after-new-100k.json before-compiler.json before-new-100k.json completion end.ns end.utc environment last-phase launch.argv-lines launch.cwd load-after load-before metadata-subcommands.jsonl phases.tsv preflight.json release.sha256 start.ns start.utc status stop-reason)){expect_file("$comparison/records/$_",'ordinary');}
my@fields=qw(argv.json cwd cwd.actual end.ns end.utc free-bytes load-after load-before start.ns start.utc status stderr stdin stdout time wrapper-argv.json);
for my$phase(qw(preflight postflight)){for(@fields){expect_file("$comparison/records/$phase/$_",'ordinary');}}
my%families=(
 'new-100k'=>{sha256=>'42443b75fe211c9f9efa7c0ee26c9233280c38a124ec5e5b514666aab97ce361',bytes=>4468259,representative=>"$comparison/images/01-new-100k-A1.image",member=>'families/new-100k.image',aliases=>[]},
 'compiler'=>{sha256=>'5ab411b7fb91498adc1453a08d96893f85c1d644a2e050b93b3e9dc850585597',bytes=>21489004,representative=>"$comparison/images/07-compiler-A1.image",member=>'families/compiler.image',aliases=>[]}
);
my$i=0;
for my$input(qw(new-100k compiler)){
 for my$label(qw(A1 B1 B2 A2 A3 B3)){
  my$phase=sprintf('%02d-%s-%s',++$i,$input,$label);
  for(@fields){expect_file("$comparison/records/$phase/$_",'ordinary');expect_file("$comparison/records/validation-$phase/$_",'ordinary');}
  for(qw(row.json validation.status)){expect_file("$comparison/records/$phase/$_",'ordinary');}
  my$p="$comparison/images/$phase.image";expect_file($p,"family:$input");push@{$families{$input}{aliases}},$p;
 }
 my$p="$comparison/outputs/$input.image";expect_file($p,"family:$input");push@{$families{$input}{aliases}},$p;
}
expect_file("$comparison/producer-A",'producer:A');
expect_file("$comparison/producer-B",'producer:B');
expect_file("$comparison/producer-B.gz",'producer:B-compressed');
expect_file("$comparison/summary.json",'ordinary');
for(qw(argv controls-after.sha256 controls-before.sha256 cwd end.ns end.utc load-after load-before release-commit start.ns start.utc status stderr stdout time)){expect_file("$outer/$_",'ordinary');}
die "unexpected fixed file population\n" unless keys(%expected)==501;
my%directories;
for(keys%expected){my@p=split m{/};pop@p;while(@p){$directories{join('/',@p)}=1;pop@p;}}
die "unexpected fixed directory population\n" unless keys(%directories)==32;
sub scan_sources {
 my@rows;my(%seen_files,%seen_dirs);
 for my$scope($comparison,$outer){
  my$root="$base/$scope";my@st=lstat$root;
  die "unsafe source scope\n" unless @st&&S_ISDIR($st[2])&&$st[4]==$>&&abs_path($root)eq$root;
  find({no_chdir=>1,follow=>0,wanted=>sub{
   my$path=$File::Find::name;my@st=lstat$path;die "missing/foreign member\n" unless @st&&$st[4]==$>;
   my$p=safe_path(substr($path,length($base)+1));my$type;
   die "privileged mode in source scope\n" if$st[2]&07000;
   if(S_ISDIR($st[2])){die "unknown/duplicate directory $p\n" unless$directories{$p}&&!$seen_dirs{$p}++;$type='directory';}
   elsif(S_ISREG($st[2])){die "unknown/duplicate/hardlinked file $p\n" unless$expected{$p}&&!$seen_files{$p}++&&$st[3]==1;$type='file';}
   else{die "special member $p\n";}
   push@rows,{path=>$p,type=>$type,mode=>$st[2]&07777,bytes=>$type eq'file'?$st[7]:0};
  }},$root);
 }
 die "incomplete live scopes\n" unless keys(%seen_files)==keys(%expected)&&keys(%seen_dirs)==keys(%directories);
 return[sort{$a->{path}cmp$b->{path}}@rows];
}
my@out=lstat$out;die "unsafe recovery root\n" unless @out&&S_ISDIR($out[2])&&$out[4]==$>&&abs_path($out)eq$out;
die "archive already exists; no repeated packaging\n" if lstat"$out/recovery.tar.gz";
my$metadata_bytes=read_file("$out/metadata-inventory.json");
die "metadata inventory pin mismatch\n" unless sha256_hex($metadata_bytes)eq'1704c53968892d10a57024eacf3a1c40c72e5294151793b4b62b8977069ed25b';
my$first=decode_json($metadata_bytes);
my$initial=scan_sources();
die "source metadata changed since inventory\n" unless$json->encode($initial)eq$json->encode($first->{entries});
my%meta=map{$_->{path}=>$_}@$initial;
die "summary pin mismatch\n" unless sha256_hex(read_file("$base/$comparison/summary.json"))eq'b531ef397e7420418562125d44fadc6399139ac3cc8c3c58c37d1a45c415c33c';
die "cohort/outer not complete\n" unless read_file("$base/$comparison/records/status")eq"0\n"&&read_file("$base/$outer/status")eq"0\n";
my@commands;
sub git_read {
 my(@argv)=@_;push@commands,\@argv;
 open my$f,'-|',@argv or die$!;binmode$f;local$/;my$b=<$f>;$b='' unless defined$b;close$f or die "Git recovery failed $?\n";
 die "unbounded Git recovery object\n" if length($b)>134217728;
 return$b;
}
my%recipes=(
 A=>{commit=>'7d0731e99aeaf241c4334fc5ff065a341fb388c8',path=>'seed/lkjscript.gz',compressed_sha256=>'505be947525d53b4db1bff6c85cb77622cb2f5dc7e7d7fe92b772ee3b8ffc3e2',compressed_bytes=>6547345,expanded_sha256=>'6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc',expanded_bytes=>32584542},
 B=>{commit=>'e5294c339d9aefbbafc2e951c997f4fefabcc9ad',path=>'seed/lkjscript.gz',compressed_sha256=>'c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae',compressed_bytes=>6551428,expanded_sha256=>'b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db',expanded_bytes=>32588975}
);
my(%expanded,%compressed);
for my$name(qw(A B)){
 my$p=$recipes{$name};my$spec="$p->{commit}:$p->{path}";
 my$oid=git_read('/usr/bin/git','-C',$repo,'rev-parse',$spec);chomp$oid;die "invalid Git blob identity\n" unless$oid=~/\A[0-9a-f]{40}\z/;
 my$seed=git_read('/usr/bin/git','-C',$repo,'show',$spec);
 die "compressed producer identity mismatch\n" unless length($seed)==$p->{compressed_bytes}&&sha256_hex($seed)eq$p->{compressed_sha256};
 my$bytes='';gunzip(\$seed=>\$bytes)or die"seed decode: $GunzipError\n";
 die "expanded producer identity mismatch\n" unless length($bytes)==$p->{expanded_bytes}&&sha256_hex($bytes)eq$p->{expanded_sha256};
 $p->{git_blob}=$oid;$p->{git_verified}=JSON::PP::true;$expanded{$name}=$bytes;$compressed{$name}=$seed;
}
my(%family_bytes,%payloads,%payload_modes,@inventory,@family_comparisons,@producer_comparisons);
for my$name(qw(new-100k compiler)){
 my$p=$families{$name};my$b=read_file("$base/$p->{representative}");
 die "representative identity mismatch\n" unless length($b)==$p->{bytes}&&sha256_hex($b)eq$p->{sha256};
 $family_bytes{$name}=$b;$payloads{$p->{member}}=$b;$payload_modes{$p->{member}}=$meta{$p->{representative}}{mode};
}
my($file_bytes,$ordinary_count,$ordinary_bytes)=(0,0,0);
for my$m(@$initial){
 my%e=%$m;$e{mode_octal}=sprintf('%04o',$m->{mode});
 if($m->{type}eq'directory'){push@inventory,\%e;next;}
 my$p=$m->{path};my$b=read_file("$base/$p");die "source size drift\n" unless length($b)==$m->{bytes};
 $e{sha256}=sha256_hex($b);$file_bytes+=length$b;
 my$class=$expected{$p};
 if($class eq'ordinary'){
  my$member;
  if($p=~m{\A\Q$comparison\E/(.+)\z}){$member="files/comparison/$1";}
  elsif($p=~m{\A\Q$outer\E/(.+)\z}){$member="files/outer/$1";}
  else{die "ordinary scope";}
  die "duplicate stored member\n" if exists$payloads{$member};
  $payloads{$member}=$b;$payload_modes{$member}=$m->{mode};$e{storage}={kind=>'archive',member=>$member};
  ++$ordinary_count;$ordinary_bytes+=length$b;
 }elsif($class=~/\Afamily:(.+)\z/){
  my$name=$1;my$f=$families{$name};
  die "output alias full bytes differ: $p\n" unless$b eq$family_bytes{$name};
  $e{storage}={kind=>'archive-alias',member=>$f->{member},representative_original=>$f->{representative}};
  push@family_comparisons,{path=>$p,family=>$name,full_byte_equal=>JSON::PP::true,%{pin($b)},mode=>$m->{mode}};
 }elsif($class=~/\Aproducer:(.+)\z/){
  my$name=$1;my($source,$recipe,$encoding);
  if($name eq'B-compressed'){($source,$recipe,$encoding)=($compressed{B},'B','git-blob');}
  else{($source,$recipe,$encoding)=($expanded{$name},$name,'gunzip-git-blob');}
  die "producer full bytes differ: $p\n" unless$b eq$source;
  $e{storage}={kind=>$encoding,recipe=>$recipe};
  push@producer_comparisons,{path=>$p,recipe=>$recipe,encoding=>$encoding,full_byte_equal=>JSON::PP::true,%{pin($b)},mode=>$m->{mode}};
 }else{die "unknown recovery class\n";}
 push@inventory,\%e;
}
die "unexpected source/storage totals\n" unless $file_bytes==254521215 && $ordinary_count==484 && $ordinary_bytes==1095429
 && @family_comparisons==14 && @producer_comparisons==3;
my$inventory={schema=>'compiler-reference-capture-comparison-recovery-inventory-2',source_base=>$base,
 scopes=>[$comparison,$outer],scope_contract=>'Exact regular bytes and modes plus directory paths/modes; no deletion or restored filesystem is produced.',
 original_files=>501,original_directories=>32,original_file_bytes=>$file_bytes,
 metadata_snapshot_sha256=>sha256_hex($metadata_bytes),entries=>\@inventory};
my$recovery={schema=>'compiler-reference-capture-producer-recovery-2',repository=>$repo,recipes=>\%recipes,
 comparisons=>\@producer_comparisons,git_commands=>\@commands,
 contract=>'Recover the pinned Git gzip blob; verify its complete bytes; decompress only for gunzip-git-blob entries; verify expanded complete bytes and restore each original path/mode. No producer binary is recompressed into this archive.'};
my$family={schema=>'compiler-reference-capture-output-family-proof-2',families=>\%families,pre_dedup_full_byte_comparisons=>\@family_comparisons,
 contract=>'All fourteen original output paths were compared in full before deduplication. Each original is recovered as an independent file from its family representative, with its own inventoried mode.'};
my$readme=<<'README';
# Lossless comparison-2 recovery

This archive covers only the completed comparison-2 and its comparison-launch-2 outer capture.
Every ordinary record/control/summary/capture file is stored separately under files/comparison or files/outer.
The fourteen output paths use two stored representatives under families; the inventory retains each original path, mode, size and digest.
A/B executables and producer-B.gz use the exact committed Git seed recipes, whose complete compressed/expanded bytes were checked against the originals.

Recovery is to a new destination, not over either original root:
1. Verify the archive identity and member inventory from the surrounding package.
2. Recreate the thirty-two original directories and their recorded modes.
3. Copy every archive-backed inventory entry to its recorded original relative path; copy aliases independently, then restore each recorded mode.
4. For a git-blob entry, obtain the exact committed seed blob and verify it. For gunzip-git-blob, additionally decompress and verify the full expanded bytes.
5. Verify all 501 recovered regular file sizes/digests and modes against recovery/inventory.json.

No source-preparation tree, benchmark input tree, key/store, mutable canonical compiler or unrelated scope is included.
Filesystem inode allocation, ownership and incidental timestamps are not claimed to be recreated; the requested bytes, file modes and directory layout/modes are complete.
The original roots remain untouched. This package authorizes no cleanup, workload rerun or automatic restoration.
README
my%metadata=(
 'inventory.json'=>$json->encode($inventory),
 'producer-recovery.json'=>$json->encode($recovery),
 'family-proof.json'=>$json->encode($family),
 'readme.md'=>$readme
);
for my$name(sort keys%metadata){write_file($name,$metadata{$name});$payloads{"recovery/$name"}=$metadata{$name};$payload_modes{"recovery/$name"}=0600;}
my$tar=Archive::Tar->new;
my@member_inventory;
for my$name(sort keys%payloads){
 safe_path($name);die "archive member exceeds bounded short-name format\n" if length($name)>100;
 my$b=$payloads{$name};
 $tar->add_data($name,$b,{mode=>$payload_modes{$name},mtime=>0,uid=>0,gid=>0,uname=>'',gname=>'',type=>'0',size=>length$b})
  or die"tar add: ".Archive::Tar->error."\n";
 push@member_inventory,{path=>$name,mode=>$payload_modes{$name},%{pin($b)}};
}
die "unexpected archive population\n" unless@member_inventory==490;
my$tar_bytes=$tar->write();
die "tar serialization failed\n" unless defined$tar_bytes&&length$tar_bytes;
my$gzip='';
gzip(\$tar_bytes=>\$gzip,Time=>0,Name=>'',Comment=>'',Level=>9)or die"gzip: $GzipError\n";
write_file('recovery.tar.gz',$gzip);
my$decoded='';gunzip(\$gzip=>\$decoded)or die"archive decode: $GunzipError\n";
die "compressed archive byte roundtrip differs\n" unless$decoded eq$tar_bytes;
my$again=Archive::Tar->new;
$again->read("$out/recovery.tar.gz",1)or die"archive reread: ".Archive::Tar->error."\n";
my(%seen,%stored);
for my$m($again->get_files){
 die "nonregular archive member\n" unless$m->is_file;
 my$name=safe_path($m->full_path);
 die "unknown/duplicate archive member\n" unless exists($payloads{$name})&&!$seen{$name}++;
 my$b=$m->get_content;
 die "archive member bytes/mode/header differ\n" unless$b eq$payloads{$name} && $m->mode==$payload_modes{$name}
  && $m->uid==0 && $m->gid==0 && $m->mtime==0;
 $stored{$name}=$b;
}
die "archive omitted member\n" unless keys(%seen)==keys(%payloads);
my$final=scan_sources();
die "source metadata changed during packaging\n" unless$json->encode($final)eq$json->encode($initial);
my($direct,$alias,$git)=(0,0,0);
for my$e(@inventory){
 next unless$e->{type}eq'file';
 my$b=read_file("$base/$e->{path}");
 die "source final digest/size drift\n" unless length($b)==$e->{bytes}&&sha256_hex($b)eq$e->{sha256};
 my$s=$e->{storage};
 if($s->{kind}eq'archive'){die "direct source/archive bytes differ\n" unless$b eq$stored{$s->{member}};++$direct;}
 elsif($s->{kind}eq'archive-alias'){die "alias/archive full bytes differ\n" unless$b eq$stored{$s->{member}};++$alias;}
 else{my$expected=$s->{kind}eq'git-blob'?$compressed{$s->{recipe}}:$expanded{$s->{recipe}};die "Git-recovered/source bytes differ\n" unless$b eq$expected;++$git;}
}
write_json('archive-members.json',\@member_inventory);
my$result={schema=>'compiler-reference-capture-comparison-recovery-verification-2',status=>'passed',
 original_roots_unchanged=>JSON::PP::true,source_files=>501,source_directories=>32,source_file_bytes=>$file_bytes,
 ordinary_files_stored=>$ordinary_count,ordinary_file_bytes=>$ordinary_bytes,
 output_original_paths=>14,output_families_stored=>2,representative_bytes=>4468259+21489004,
 producer_paths_recovered_from_git=>3,producer_binary_compressions=>0,
 archive=>{path=>'recovery.tar.gz',%{pin($gzip)}},uncompressed_tar_bytes=>length($tar_bytes),archive_members=>scalar@member_inventory,
 pre_dedup_family_full_byte_comparisons=>scalar@family_comparisons,
 pre_dedup_producer_full_byte_comparisons=>scalar@producer_comparisons,
 archive_member_byte_and_mode_comparisons=>scalar@member_inventory,
 post_archive_direct_full_byte_comparisons=>$direct,post_archive_alias_full_byte_comparisons=>$alias,
 post_archive_git_full_byte_comparisons=>$git,compressed_byte_roundtrip=>JSON::PP::true,
 tar_metadata=>'sorted short regular member names; uid/gid0; mtime0; each stored payload mode preserved',
 gzip_metadata=>'Time0, empty Name/Comment, level9',archive_tar_version=>$Archive::Tar::VERSION,
 summary_sha256=>'b531ef397e7420418562125d44fadc6399139ac3cc8c3c58c37d1a45c415c33c',
 workload_execution=>JSON::PP::false,deletion=>JSON::PP::false};
write_json('verification.json',$result);
print$json->encode($result);
