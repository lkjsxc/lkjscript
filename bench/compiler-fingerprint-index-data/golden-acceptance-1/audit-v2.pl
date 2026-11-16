#!/usr/bin/perl
# Independent read-only audit. Never executes the captured helper, compiler or image.
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use Math::BigInt;
use File::Find;
use Fcntl qw(:DEFAULT :mode);
use IPC::Open2;
use IO::Handle;
use Cwd qw(getcwd);
use Time::Local qw(timegm);

my $repo='/home/coder/workspace/lkjscript';
my $root="$repo/target/compiler-fingerprint-index-golden-1";
my $out="$repo/bench/compiler-fingerprint-index-data/golden-acceptance-1/results-v2";
my $prep='042bce952af30959b1ecdff2fc686033f789c497';
my $source_commit='9326dbaab1bf6832b9f35dc156e12d0d62eccd9e';
my $prep_path='bench/compiler-fingerprint-index-data/golden-preparation-1';
my $producer_sha='b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db';
my $image_sha='99a179be5fd5f6cd9c8e7d3e4466468ff4b519a406303e77e935c1d27dcade89';
my $incremental_sha='760485a5a12d97ce320ba9113797370df584c154084ee4a1f9689da28c7d1127';
my $release_sha='9d40835d790473ab8771f47ee96ae2a33c722add32a57493305a9276277b3dba';
my $source_sha='354bb88fe20e3685586fd689cc6681ce6c31d7fdba8c5db6ff9fe4f1d923acf5';
my $j=JSON::PP->new->canonical->utf8;
my $pretty=JSON::PP->new->canonical->pretty->utf8;
die "fixed cwd / no arguments required\n" unless !@ARGV && getcwd() eq $repo;
my @input_records;

sub require_ok { die "$_[1]\n" unless $_[0]; }
sub equal { $j->encode($_[0]) eq $j->encode($_[1]); }
sub is_uint {
 my($v)=@_; return 0 if !defined($v) || ref($v);
 return $j->encode($v)=~/\A(?:0|[1-9][0-9]*)\z/;
}
sub bytes {
 my($p,$cap)=@_; my @s=lstat($p);
 require_ok(@s && S_ISREG($s[2]) && $s[3]==1,"not regular single-link: $p");
 require_ok(!defined($cap) || $s[7]<=$cap,"size bound: $p");
 sysopen(my $f,$p,O_RDONLY|O_NOFOLLOW) or die "open $p: $!\n"; binmode $f;
 my @h=stat($f);
 require_ok($h[0]==$s[0] && $h[1]==$s[1] && $h[7]==$s[7],"opened file mismatch: $p");
 local $/; my $b=<$f>; $b='' unless defined($b);
 close $f or die "close $p: $!\n";
 require_ok(length($b)==$s[7],"short read: $p");
 return $b;
}
sub info {
 my($p)=@_; my $b=bytes($p,64*1024*1024); my @s=lstat($p);
 return {path=>$p,bytes=>length($b),sha256=>sha256_hex($b),mode=>sprintf('%04o',$s[2]&07777)};
}
sub record_input {
 my($rel,$kind)=@_; my $v=info("$root/$rel");
 push @input_records,{%$v,root_relative=>$rel,kind=>$kind};
 return $v;
}
sub put {
 my($p,$b)=@_; require_ok(index($p,"$out/")==0,"output outside fixed acceptance");
 sysopen(my $f,$p,O_WRONLY|O_CREAT|O_EXCL,0644) or die "exclusive output $p: $!\n";
 binmode $f; print {$f} $b or die "write $p: $!\n"; close $f or die "close output $p: $!\n";
}
sub save { put($_[0],$pretty->encode($_[1])); }
sub json_at { $j->decode(bytes($_[0],4*1024*1024)); }
sub command {
 my(@a)=@_; require_ok($a[0] eq 'git',"only read-only Git subprocesses are allowed");
 open my $f,'-|',@a or die "Git open: $!\n"; binmode $f; local $/; my $b=<$f>;
 close $f or die "Git failed: @a\n"; return $b;
}
sub manifest {
 my($b)=@_; require_ok($b=~/\n\z/,"manifest terminal newline");
 my %m;
 for my $l(split /\n/,$b) {
  require_ok($l=~/\A([0-9a-f]{64})  ([A-Za-z0-9_.\/-]+)\z/,"manifest grammar");
  my($sha,$p)=($1,$2);
  require_ok($p!~m{(?:\A|/)\.\.?(?:/|\z)} && $p!~m{\A/} && !exists($m{$p}),"manifest unsafe or duplicate path");
  $m{$p}=$sha;
 }
 return \%m;
}
sub files_under {
 my($base)=@_; my %f; my %d;
 find({no_chdir=>1,wanted=>sub {
  my $p=$File::Find::name; my @s=lstat($p); require_ok(@s,"lstat: $p");
  (my $rel=$p)=~s/^\Q$base\E\/?//;
  if(S_ISDIR($s[2])) { $d{$rel}=1; return; }
  require_ok(S_ISREG($s[2]) && $s[3]==1,"special or linked file: $p");
  $f{$rel}={mode=>sprintf('%04o',$s[2]&07777),bytes=>$s[7]};
 }},$base);
 return (\%f,\%d);
}
my($git_reader,$git_writer);
my $git_pid=open2($git_reader,$git_writer,'git','cat-file','--batch');
binmode $git_reader; binmode $git_writer; $git_writer->autoflush(1);
sub git_blob {
 my($ref)=@_; require_ok($ref!~/[\r\n]/,"Git reference newline");
 print {$git_writer} "$ref\n" or die "Git batch request\n";
 my $line=<$git_reader>;
 require_ok(defined($line) && $line=~/\A([0-9a-f]{40}) blob ([0-9]+)\n\z/,"Git blob missing: $ref");
 my($oid,$n)=($1,0+$2); require_ok($n<=64*1024*1024,"Git blob bound");
 my $b='';
 while(length($b)<$n) {
  my $got=read($git_reader,my $chunk,$n-length($b)); die "Git short blob\n" unless $got; $b.=$chunk;
 }
 my $got=read($git_reader,my $newline,1); require_ok($got==1 && $newline eq "\n","Git blob terminator");
 return ($b,$oid);
}

# The closed preparation is compared with immutable Git blobs, including its exact file set.
my $prep_manifest=bytes("$root/preparation-manifest.sha256");
require_ok(sha256_hex($prep_manifest) eq 'dba40d41be2e0551310fba20b89b09ac5aaf22e2affbf723d0b5c2b41a222949',"preparation manifest pin");
my($prep_manifest_git)=git_blob("$prep:$prep_path/preparation-manifest.sha256");
require_ok($prep_manifest_git eq $prep_manifest,"preparation manifest not committed exact");
my $pm=manifest($prep_manifest);
require_ok(keys(%$pm)==45,"preparation record count");
my($control_files)=files_under("$root/control");
my($preparation_files)=files_under("$root/records/preparation");
my %preparation_actual=map {("control/$_"=>1)} keys %$control_files;
$preparation_actual{"records/preparation/$_"}=1 for keys %$preparation_files;
require_ok(equal([sort keys %preparation_actual],[sort keys %$pm]),"preparation population");
for my $p(sort keys %$pm) {
 my $b=bytes("$root/$p"); my($old)=git_blob("$prep:$prep_path/$p");
 require_ok(sha256_hex($b) eq $pm->{$p} && $b eq $old,"preparation/Git bytes: $p");
 record_input($p,'committed-preparation');
}
record_input('preparation-manifest.sha256','committed-preparation');
my $cm_text=bytes("$root/control/manifest.sha256");
require_ok(sha256_hex($cm_text) eq '915316d240ba16eb905dc055ea34404d8cf954913b26c1610210521669ee50ab',"control manifest pin");
my $cm=manifest($cm_text);
require_ok(keys(%$cm)==14 && keys(%$control_files)==15,"control population");
for my $p(keys %$cm) {
 require_ok($p!~m{/} && sha256_hex(bytes("$root/control/$p")) eq $cm->{$p},"control record: $p");
}
my $pins=json_at("$root/control/pins.json");
my $plan=json_at("$root/control/plan.json");
my $fixtures=json_at("$root/control/fixtures.json");
my $corr=json_at("$root/control/source-correspondence.json");
my $release_text=bytes("$root/release.json");
require_ok(sha256_hex($release_text) eq $release_sha,"release pin");
require_ok(bytes("$repo/bench/compiler-fingerprint-index-data/golden-release-1.json") eq $release_text,"retained release differs");
my $release=$j->decode($release_text);
require_ok($release->{preparation_commit} eq $prep && $release->{source_commit} eq $source_commit && $release->{run_root} eq $root
 && $release->{status} eq 'RELEASED_BY_ROOT_FOR_ONE_BUILD_ONE_DUMP'
 && $release->{producer_sha256} eq $producer_sha && $release->{original_incremental_sha256} eq $incremental_sha
 && $release->{control_manifest_sha256} eq sha256_hex($cm_text),"release fields");
record_input('release.json','root-release');

# Source recovery: exact Git file set, contents and modes plus only two declared overlays.
my %tree;
my $listing=command('git','ls-tree','-r','-z',$source_commit,'--','src','library','tests');
for my $row(split /\0/,$listing) {
 require_ok($row=~/\A(100644) blob ([0-9a-f]{40})\t((?:src|library|tests)\/[^\r\n\t]+)\z/,"source Git tree row");
 my($mode,$oid,$p)=($1,$2,$3);
 require_ok(!exists($tree{$p}) && $p!~m{(?:\A|/)\.\.?(?:/|\z)},"duplicate source Git path");
 $tree{$p}={mode=>$mode,oid=>$oid};
}
require_ok(keys(%tree)==1743,"original Git source population");
for my $name(qw(src library tests)) {
 my $oid=command('git','rev-parse',"$source_commit:$name"); chomp $oid;
 require_ok($oid eq $pins->{trees}->{$name},"source subtree identity");
}
my $orig_text=bytes("$root/control/source.git.sha256");
my $final_text=bytes("$root/control/source.overlay.sha256");
require_ok(sha256_hex($orig_text) eq $pins->{original_manifest_sha256}
 && sha256_hex($final_text) eq $source_sha && $pins->{source_manifest_sha256} eq $source_sha,"source manifest pins");
my $om=manifest($orig_text); my $fm=manifest($final_text);
require_ok(equal([sort keys %tree],[sort keys %$om]),"original manifest population");
my %overlay=('src/main.lkj'=>'golden-main.txt','src/fingerprint_fixture.lkj'=>'incremental-fingerprint-module.txt');
my %expected=map {($_=>1)} keys %tree; $expected{'src/fingerprint_fixture.lkj'}=1;
require_ok(equal([sort keys %expected],[sort keys %$fm]),"final source population");
require_ok($corr->{commit} eq $source_commit && $corr->{copied_git_files}==1743 && $corr->{final_files}==1744
 && equal($corr->{trees},$pins->{trees}) && @{$corr->{files}}==1744,"correspondence header");
my %correspondence;
for my $r(@{$corr->{files}}) { require_ok(!exists($correspondence{$r->{path}}),"duplicate correspondence"); $correspondence{$r->{path}}=$r; }
require_ok(equal([sort keys %correspondence],[sort keys %expected]),"correspondence population");
my($live_files,$live_dirs)=files_under("$root/source");
require_ok(equal([sort keys %$live_files],[sort keys %expected]),"live source population");
my %expected_dirs=(''=>1);
for my $p(keys %expected) { while($p=~s{/[^/]+\z}{}) { $expected_dirs{$p}=1; } }
require_ok(equal([sort keys %$live_dirs],[sort keys %expected_dirs]),"source directory population");
my $source_bytes=0; my @source_proof;
for my $p(sort keys %expected) {
 my($original,$git_oid);
 if(exists($tree{$p})) {
  ($original,$git_oid)=git_blob($tree{$p}->{oid});
  require_ok($git_oid eq $tree{$p}->{oid} && sha256_hex($original) eq $om->{$p},"original source content: $p");
 }
 my $expected_bytes=exists($overlay{$p})?bytes("$root/control/$overlay{$p}"):$original;
 my $actual=bytes("$root/source/$p");
 require_ok($live_files->{$p}->{mode} eq '0644' && $actual eq $expected_bytes && sha256_hex($actual) eq $fm->{$p},"live source exact bytes/mode: $p");
 my $r=$correspondence{$p};
 require_ok($r->{path} eq $p && $r->{mode} eq '100644' && $r->{bytes}==length($actual) && $r->{sha256} eq sha256_hex($actual)
 && equal($r->{git_oid},$git_oid) && equal($r->{overlay},$overlay{$p}),"correspondence row: $p");
 $source_bytes+=length($actual);
 push @source_proof,join("\t",$p,defined($git_oid)?$git_oid:'-',sha256_hex($actual),length($actual),'0644',exists($overlay{$p})?$overlay{$p}:'Git exact');
}
require_ok($source_bytes==9390696 && $source_bytes==$pins->{source_bytes} && $pins->{source_files}==1744
 && $pins->{commit} eq $source_commit && $pins->{source} eq "$root/source","source totals/pins");
require_ok(sha256_hex(bytes("$root/source/src/elaborate/incremental.lkj")) eq $incremental_sha,"original incremental module changed");
require_ok(sha256_hex(bytes("$root/source/library/runtime.lkj")) eq $pins->{runtime}->{sha256}
 && $pins->{runtime}->{sha256} eq 'eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14',"explicit runtime pin");
my $fixture=bytes("$root/source/src/fingerprint_fixture.lkj");
require_ok($fixture=~/elaborate::incremental::fingerprint\(&program,\s*&inputs\)/
 && $fixture=~/panic!\("fingerprint golden \{case\} is pending the root baseline capture"\)/,"original call and pending expected() marker");
put("$out/source-audit.tsv","path\toriginal_git_blob\tactual_sha256\tbytes\tmode\torigin\n".join("\n",@source_proof)."\n");

my $producer=record_input('bin/compiler-baseline','executing-compiler');
my $image=record_input('bin/fingerprint-golden','built-native-image');
require_ok($producer->{sha256} eq $producer_sha && $producer->{bytes}==32588975 && ($producer->{mode} eq '0755' || $producer->{mode} eq '0700'),"live producer identity");
require_ok($image->{sha256} eq $image_sha && $image->{bytes}==1568827 && ($image->{mode} eq '0755' || $image->{mode} eq '0700'),"live image identity");
my $producer_fact={path=>"$root/bin/compiler-baseline",bytes=>32588975,sha256=>$producer_sha};
my $image_fact={path=>"$root/bin/fingerprint-golden",bytes=>1568827,sha256=>$image_sha};

# Validate every raw phase record against the frozen commands and closed inputs.
my @ids=qw(multi-base multi-reversed-functions duplicate-parsed-later duplicate-parsed-first missing-bodyless-source duplicate-first-truncated ignored-invalid-function-file reversed-body-bounds body-past-eof overlapping-bodies type-past-eof unparsed-order unparsed-duplicate include-base include-parsed-body include-binary-payload include-string-payload);
my @none=(4,5,7,8,9,10);
require_ok(equal($pins->{cases},\@ids) && equal($pins->{none_ordinals},\@none)
 && equal([map {$_->{id}} @{$fixtures->{cases}}],\@ids) && equal($plan->{frozen_population}->{case_order},\@ids)
 && equal($plan->{frozen_population}->{none_ordinals},\@none),"fixed case population");
require_ok(equal([map {$_->{id}} @{$plan->{phases}}],[qw(build dump)]),"plan phase count/order");
my @build=("$root/bin/compiler-baseline",'build','src/main.lkj','--source-dir',"$root/source",'--library-dir','library',
 '--runtime-source','library/runtime.lkj','--inline-limit','0','--no-store','--stats','-o',"$root/bin/fingerprint-golden");
my @dump=("$root/bin/fingerprint-golden");
my $format="elapsed_seconds\t%e\nuser_seconds\t%U\nsystem_seconds\t%S\nmax_rss_kib\t%M\nexit\t%x";
my @phase;
my %expected_exec=map {($_=>1)} qw(release.json controls.json outcome.json);
my @common=qw(source-before.sha256 inputs-before.json argv.json stdout stderr time.tsv status.json source-after.sha256 inputs-after.json);
for my $which(qw(build dump)) { $expected_exec{"$which/$_"}=1 for @common,($which eq 'build'?'validation.json':'goldens.json'); }
my($ef,$ed)=files_under("$root/records/execution");
require_ok(equal([sort keys %$ef],[sort keys %expected_exec]) && equal([sort keys %$ed],['','build','dump']),"execution record population");
require_ok(equal(json_at("$root/records/execution/release.json"),$release)
 && bytes("$root/records/execution/release.json") eq $pretty->encode($release),"execution release fields / declared serializer");
my $controls=json_at("$root/records/execution/controls.json");
require_ok(equal($controls,{manifest_sha256=>sha256_hex($cm_text),pins_sha256=>sha256_hex(bytes("$root/control/pins.json")),plan_sha256=>sha256_hex(bytes("$root/control/plan.json"))}),"execution control correspondence");
sub utc_ns {
 my($v)=@_; require_ok($v=~/\A(\d{4})-(\d\d)-(\d\d)T(\d\d):(\d\d):(\d\d)\.(\d{9})Z\z/,"UTC format");
 return Math::BigInt->new(timegm($6,$5,$4,$3,$2-1,$1))->bmul(1000000000)->badd($7);
}
for my $index(0,1) {
 my $name=$index==0?'build':'dump'; my $dir="$root/records/execution/$name";
 my $a=json_at("$dir/argv.json"); my $s=json_at("$dir/status.json");
 my $argv=$index==0?\@build:\@dump; my $cwd=$index==0?"$root/source":$root;
 my @wrapped=('/usr/bin/time','-o',"$dir/time.tsv",'-f',$format,'/usr/bin/timeout','--signal=TERM','--kill-after=10s','900s',
 '/usr/bin/prlimit','--as=4294967296','--',@$argv);
 require_ok(equal($plan->{phases}->[$index],{id=>$name,cwd=>$cwd,argv=>$argv})
 && equal($a,{argv=>$argv,cwd=>$cwd,wrapper_argv=>\@wrapped,environment=>{LC_ALL=>'C',TZ=>'UTC'},unset_environment=>['LKJSCRIPT_STORE_IO'],stdin=>'/dev/null'}),"exact phase invocation: $name");
 require_ok($s->{phase} eq $name && is_uint($s->{exit_code}) && $s->{exit_code}==0 && is_uint($s->{wait_status}) && $s->{wait_status}==0
 && is_uint($s->{signal}) && $s->{signal}==0,"phase process status: $name");
 for my $k(qw(before_ns after_ns wrapper_elapsed_ns)) { require_ok(defined($s->{$k}) && $s->{$k}=~/\A[0-9]+\z/,"phase ns field"); }
 my $before=Math::BigInt->new($s->{before_ns}); my $after=Math::BigInt->new($s->{after_ns});
 require_ok($after->bcmp($before)>=0 && $after->copy()->bsub($before)->bstr() eq $s->{wrapper_elapsed_ns},"phase ns arithmetic");
 require_ok(utc_ns($s->{before_utc})->bcmp($before)<=0 && utc_ns($s->{after_utc})->bcmp($after)>=0,"UTC/ns ordering");
 for my $field(qw(load_before load_after)) {
  require_ok(ref($s->{$field}) eq 'ARRAY' && @{$s->{$field}}==3 && !(grep {!defined($_) || $_!~/\A[0-9]+\.[0-9]+\z/} @{$s->{$field}}),"load record");
 }
 for my $pair(['stdout','stdout'],['stderr','stderr'],['gnu_time','time.tsv']) {
  my $actual=info("$dir/$pair->[1]"); delete $actual->{mode};
  require_ok(equal($actual,$s->{$pair->[0]}),"phase captured digest: $name/$pair->[1]");
 }
 my %time;
 for my $l(split /\n/,bytes("$dir/time.tsv")) {
  require_ok($l=~/\A(elapsed_seconds|user_seconds|system_seconds|max_rss_kib|exit)\t([0-9]+(?:\.[0-9]+)?)\z/,"GNU time row");
  my($k,$v)=($1,$2); require_ok(!exists($time{$k}),"duplicate GNU time field"); $time{$k}=$v;
 }
 require_ok(keys(%time)==5 && $time{exit} eq '0',"GNU time completion");
 for my $when(qw(before after)) {
  require_ok(bytes("$dir/source-$when.sha256") eq $final_text,"captured source manifest: $name/$when");
  my $expected_inputs={producer=>$producer_fact,source_files=>1744,source_manifest_sha256=>$source_sha};
  $expected_inputs->{native_image}=$image_fact if $name eq 'dump';
  require_ok(equal(json_at("$dir/inputs-$when.json"),$expected_inputs),"captured producer/image correspondence: $name/$when");
 }
 push @phase,{name=>$name,argv=>$argv,cwd=>$cwd,wrapper_argv=>\@wrapped,status=>$s,gnu_time=>\%time};
}
require_ok(Math::BigInt->new($phase[0]->{status}->{after_ns})->bcmp($phase[1]->{status}->{before_ns})<0,"build/dump order");
my $build_err=bytes("$root/records/execution/build/stderr");
require_ok($build_err=~/\ALKJSCRIPT_STATS ([^\r\n]+)\n\z/,"exact one stats line / no diagnostics");
my $stats=$j->decode($1);
require_ok($stats->{route} eq 'compiler' && $stats->{stopped_stage} eq 'done',"completed compiler route");
my @zero=qw(build_reused checked_local_reused checked_reused emitted_reused front_reused inline_accepted inline_attempts inline_calls inline_functions inline_rejected test_reused test_rerun);
for my $field(@zero) { require_ok(is_uint($stats->{$field}) && $stats->{$field}==0,"nonzero/nonnumeric stats $field"); }
require_ok(is_uint($stats->{emitted_bytes}) && $stats->{emitted_bytes}==$image->{bytes},"emitted size matches image");
require_ok(!exists($stats->{store_writes}) || (is_uint($stats->{store_writes}) && $stats->{store_writes}==0),"store write counter");
require_ok(bytes("$root/records/execution/build/stdout") eq '' && bytes("$root/records/execution/dump/stderr") eq '',"unexpected build stdout/dump stderr");
my $bv=json_at("$root/records/execution/build/validation.json");
require_ok(equal($bv->{stats},$stats) && equal($bv->{image},$image_fact) && $bv->{status} eq 'accepted'
 && $bv->{store_writes_phase} eq 'NA_no_store_counter_not_emitted' && !exists($stats->{store_writes}),"build validation agrees with raw");

# Parse only the complete captured wire. No fingerprint calculation is implemented here.
my $wire=bytes("$root/records/execution/dump/stdout");
require_ok(length($wire)==4531 && sha256_hex($wire) eq '346340880cf8b9bd247c0caf30f75ed534a702c8b0257dc81fcea2cb5849e4cd',"raw dump pin");
my @line=split /\n/,$wire,-1; require_ok(pop(@line) eq '',"wire terminal newline");
my $at=0; require_ok($line[$at++] eq 'lkjscript-fingerprint-goldens 1',"wire header");
my %none=map {($_=>1)} @none; my @cases;
for my $n(0..16) {
 my $start=$at;
 require_ok(defined($line[$at]) && $line[$at++] eq "case $n $ids[$n]","wire case ordinal/identity");
 my $tag=$line[$at++]; my $r={ordinal=>$n,id=>$ids[$n]};
 if($none{$n}) {
  require_ok(defined($tag) && $tag eq 'none',"expected None"); $r->{outcome}='None';
 } else {
  require_ok(defined($tag) && $tag=~/\Asome ([0-9a-f]{64}) (2|6)\z/,"expected bounded Some");
  my($shape,$count)=($1,0+$2); require_ok($count==($n<13?6:2),"body vector width");
  my @body;
  for my $slot(0..$count-1) {
   require_ok(defined($line[$at]) && $line[$at++]=~/\Abody \Q$slot\E (empty|[0-9a-f]{64})\z/,"body slot framing");
   push @body,$1 eq 'empty'?'':$1;
  }
  $r->{outcome}='Some'; $r->{shape}=$shape; $r->{bodies}=\@body;
 }
 require_ok(defined($line[$at]) && $line[$at++] eq 'end',"case end");
 $r->{exact_record}=join("\n",@line[$start..$at-1])."\n"; push @cases,$r;
}
require_ok(defined($line[$at]) && $line[$at++] eq 'complete 17' && $at==@line,"complete wire footer / no trailing data");
my $g=json_at("$root/records/execution/dump/goldens.json");
require_ok(equal($g->{cases},\@cases) && $g->{some}==11 && $g->{none}==6 && $g->{raw_bytes}==length($wire) && $g->{raw_sha256} eq sha256_hex($wire)
 && $g->{producer_sha256} eq $producer_sha && $g->{incremental_module_sha256} eq $incremental_sha
 && $g->{status} eq 'captured_pending_root_review_not_installed',"recorded goldens agree with independently parsed wire");
my @relations;
sub relation { my($id,$ok,$detail)=@_; require_ok($ok,"relation $id"); push @relations,{id=>$id,status=>'pass',detail=>$detail}; }
sub changed_slots {
 my($x,$y)=@_; require_ok(@$x==@$y,"vector comparison width");
 return [grep {$x->[$_] ne $y->[$_]} 0..$#$x];
}
relation('fixed-outcomes',scalar(grep {$_->{outcome} eq 'Some'} @cases)==11 && scalar(grep {$_->{outcome} eq 'None'} @cases)==6,'Exactly 17 fixed records; None only at 4,5,7,8,9,10.');
relation('case-2-value-equality',$cases[2]->{shape} eq $cases[0]->{shape} && equal($cases[2]->{bodies},$cases[0]->{bodies}),'Fingerprint shape and body vector match case0; case-name wrappers intentionally differ.');
relation('case-3-one-body',$cases[3]->{shape} eq $cases[0]->{shape} && equal(changed_slots($cases[0]->{bodies},$cases[3]->{bodies}),[5]),'Only alpha::left body slot5 differs; shape is unchanged.');
relation('case-1-reversed-vector',equal($cases[1]->{bodies},[reverse @{$cases[0]->{bodies}}]) && $cases[1]->{shape} ne $cases[0]->{shape},'Complete six-slot vector is reversed, including the empty slot; shape differs.');
relation('case-6-ignored-body',$cases[6]->{shape} ne $cases[0]->{shape} && $cases[6]->{bodies}->[0] eq '' && $cases[6]->{bodies}->[1] eq ''
 && equal(changed_slots($cases[0]->{bodies},$cases[6]->{bodies}),[1]),'Ignored function first adds one empty slot at1; pre-existing bodyless slot0 remains empty. Other slots match.');
for my $n(11,12) {
 relation("case-$n-extra-input",$cases[$n]->{shape} ne $cases[0]->{shape} && equal($cases[$n]->{bodies},$cases[0]->{bodies}),"Shape differs from case0; full six-body vector matches.");
}
relation('case-14-included-parsed-body',$cases[14]->{shape} ne $cases[13]->{shape} && equal(changed_slots($cases[13]->{bodies},$cases[14]->{bodies}),[1]),'Shape and child body slot1 differ from case13; root body slot0 matches.');
for my $n(15,16) {
 relation("case-$n-include-payload",$cases[$n]->{shape} ne $cases[13]->{shape} && equal($cases[$n]->{bodies},$cases[13]->{bodies}),"Shape differs from case13; full two-body vector matches.");
}
my $outcome=json_at("$root/records/execution/outcome.json");
require_ok($outcome->{status} eq 'captured_pending_review' && !defined($outcome->{failure}) && $outcome->{producer_sha256} eq $producer_sha
 && $outcome->{source_manifest_sha256} eq $source_sha && @{$outcome->{phases}}==2
 && equal($outcome->{phases}->[0],{phase=>'build',status=>'accepted',image=>$image_fact})
 && equal($outcome->{phases}->[1],{phase=>'dump',status=>'accepted',result=>{cases=>17,some=>11,none=>6,stdout_bytes=>length($wire),stdout_sha256=>sha256_hex($wire)}}),"outcome correspondence");
close $git_writer or die "Git batch input close\n";
while(read($git_reader,my $extra,4096)) { die "unexpected Git batch trailing output\n"; }
close $git_reader or die "Git batch output close\n";
require_ok(waitpid($git_pid,0)==$git_pid && $?==0,"Git batch completion");

# Retain the closed execution records byte-for-byte, without copying or running executables.
mkdir "$out/raw",0755 or die "raw output directory: $!\n";
mkdir "$out/raw/build",0755 or die "raw build directory: $!\n";
mkdir "$out/raw/dump",0755 or die "raw dump directory: $!\n";
for my $rel(sort keys %expected_exec) {
 my $b=bytes("$root/records/execution/$rel"); put("$out/raw/$rel",$b);
 record_input("records/execution/$rel",'closed-execution-record');
}
put("$out/preparation-manifest.sha256",$prep_manifest);
save("$out/parsed-goldens.json",{schema=>'independent-fingerprint-wire-parse-1',cases=>\@cases,some=>11,none=>6,raw_bytes=>length($wire),raw_sha256=>sha256_hex($wire)});
save("$out/relations.json",\@relations);
save("$out/input-records.json",\@input_records);
save("$out/phase-audit.json",\@phase);
my $summary={
 schema=>'compiler-fingerprint-golden-independent-acceptance-1',status=>'accepted_capture_not_installed',
 auditor_language=>'Perl',captured_attempt=>'root1605 / final1606',root=>$root,
 preparation_commit=>$prep,preparation_manifest_sha256=>sha256_hex($prep_manifest),release_sha256=>$release_sha,
 execution_release_sha256=>sha256_hex(bytes("$root/records/execution/release.json")),release_fields_equal=>JSON::PP::true,
 source=>{commit=>$source_commit,trees=>$pins->{trees},git_files=>1743,live_files=>1744,bytes=>$source_bytes,
  overlays=>\%overlay,source_manifest_sha256=>$source_sha,incremental_sha256=>$incremental_sha,
  comparisons=>'All live source bytes equal the exact Git blob or one of two committed overlays; all four captured source manifests equal the independently checked live manifest.'},
 producer=>$producer_fact,image=>$image_fact,
 commands=>{phases=>2,build_exit=>0,dump_exit=>0,exact_frozen_argv_and_wrappers=>JSON::PP::true},
 phases=>[map {{name=>$_->{name},gnu_time=>$_->{gnu_time},wrapper_elapsed_ns=>$_->{status}->{wrapper_elapsed_ns},
  load_before=>$_->{status}->{load_before},load_after=>$_->{status}->{load_after}}} @phase],
 build_stats=>{emitted_bytes=>$stats->{emitted_bytes},zero_fields=>\@zero,store_writes=>'not emitted; --no-store command checked'},
 dump=>{cases=>17,some=>11,none=>6,none_ordinals=>\@none,bytes=>length($wire),sha256=>sha256_hex($wire),relations=>scalar(@relations)},
 qualifications=>[
  'This audit reads only retained records, owned binary bytes and immutable Git blobs. It does not rerun the capture helper, compiler, image or fingerprint algorithm.',
  'Before/after correspondence is established by the four captured complete source manifests and producer/image records plus this independent live/Git comparison; no process observer was used.',
  'GNU dump time 0.00 seconds is centisecond rounding. Captured launch-inclusive wrapper interval is 6771441 ns, not zero execution time.',
  'The image uses the declared temporary main/fixture overlay and explicit runtime. No canonical self-image equality is claimed.',
  'The fixture still calls the unchanged original function; expected() remains a pending-capture panic. Golden installation and any candidate comparison are separate work.',
  'Fingerprint values are compared independently of case-name/header labels. Full raw labels and every empty body slot remain retained.'
 ]};
save("$out/summary.json",$summary);
print "accepted: 45 committed preparation records; 1744 exact source files; 2 successful fixed phases; 17 cases (11 Some / 6 None); 10 value relations; no workload rerun\n";
