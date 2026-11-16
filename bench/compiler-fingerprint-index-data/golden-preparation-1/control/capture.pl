#!/usr/bin/perl
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use Math::BigInt;
use File::Find;
use Fcntl qw(:DEFAULT :mode);
use Cwd qw(realpath);

my $root='/home/coder/workspace/lkjscript/target/compiler-fingerprint-index-golden-1';
my $control="$root/control";
my $json=JSON::PP->new->canonical->pretty;
sub raw {
 my($path,$cap)=@_; my @s=lstat($path);
 die "not a regular single-link file: $path\n" unless @s && S_ISREG($s[2]) && $s[3]==1;
 die "input exceeds bound: $path\n" if defined($cap) && $s[7]>$cap;
 open my $f,'<:raw',$path or die "$path: $!\n"; local $/; my $b=<$f>; close $f or die "$path close: $!\n"; return $b;
}
sub digest {
 my($path)=@_; my @s=lstat($path);
 die "not a regular single-link file: $path\n" unless @s && S_ISREG($s[2]) && $s[3]==1;
 open my $f,'<:raw',$path or die "$path: $!\n"; my $d=Digest::SHA->new(256)->addfile($f)->hexdigest;
 close $f or die "$path close: $!\n"; return {path=>$path,sha256=>$d,bytes=>$s[7]};
}
sub put {
 my($p,$b)=@_; sysopen my $f,$p,O_WRONLY|O_CREAT|O_EXCL,0600 or die "$p: $!\n";
 binmode $f; print {$f} $b or die "$p write: $!\n"; close $f or die "$p close: $!\n";
}
sub save { put($_[0],$json->encode($_[1])); }
sub line_command {
 my(@a)=@_; open my $f,'-|',@a or die "command open: $!\n";
 local $/; my $b=<$f>; close $f or die "command failed: @a\n"; $b=~s/\n\z//; return $b;
}
sub ns { my $n=line_command('/usr/bin/date','+%s%N'); die "invalid ns capture\n" unless $n=~/\A[0-9]{19,}\z/; return $n; }
sub utc { line_command('/usr/bin/date','-u','+%Y-%m-%dT%H:%M:%S.%NZ'); }
sub load3 {
 open my $f,'<','/proc/loadavg' or die "loadavg: $!\n"; my $l=<$f>; close $f;
 die "load capture\n" unless $l=~/\A(\d+\.\d+) (\d+\.\d+) (\d+\.\d+) /; return [$1,$2,$3];
}
sub checked_source {
 my($pins)=@_; my $manifest=raw("$control/source.overlay.sha256");
 die "source manifest pin changed\n" unless sha256_hex($manifest) eq $pins->{source_manifest_sha256};
 my %expected;
 for my $line(split /\n/,$manifest) {
  die "bad source manifest\n" unless $line=~/\A([0-9a-f]{64})  ((?:src|library|tests)\/[^\r\n\t]+)\z/;
  my($sha,$path)=($1,$2); die "duplicate/unsafe source path\n" if exists($expected{$path}) || $path=~m{(?:\A|/)\.\.?(?:/|\z)};
  $expected{$path}=$sha;
 }
 my $source=$pins->{source}; my $files=0; my $bytes=0; my %seen;
 find({no_chdir=>1,wanted=>sub {
  my $p=$File::Find::name; my @s=lstat($p); die "source lstat\n" unless @s;
  return if S_ISDIR($s[2]);
  die "source special/mode: $p\n" unless S_ISREG($s[2]) && $s[3]==1 && ($s[2]&0777)==0644;
  (my $rel=$p)=~s/^\Q$source\E\///;
  die "unlisted source: $rel\n" unless exists($expected{$rel});
  die "source bytes changed: $rel\n" unless digest($p)->{sha256} eq $expected{$rel};
  $seen{$rel}=1; ++$files; $bytes+=$s[7];
 }},$source);
 die "source population changed\n" unless $files==$pins->{source_files} && $files==keys(%expected) && $files==keys(%seen) && $bytes==$pins->{source_bytes};
 return $manifest;
}
sub checked_producer {
 my($pins)=@_; my $p=digest($pins->{producer}->{owned});
 die "producer changed\n" unless $p->{sha256} eq $pins->{producer}->{sha256} && $p->{bytes}==$pins->{producer}->{bytes};
 return $p;
}
sub inputs {
 my($dir,$when,$pins,$image)=@_;
 put("$dir/source-$when.sha256",checked_source($pins));
 my $p=checked_producer($pins);
 my $value={producer=>$p,source_manifest_sha256=>$pins->{source_manifest_sha256},source_files=>$pins->{source_files}};
 $value->{native_image}=digest($image) if defined($image);
 save("$dir/inputs-$when.json",$value);
 return $value;
}
sub phase {
 my($spec,$pins,$image,$expected_image)=@_; my $dir="$root/records/execution/$spec->{id}";
 mkdir $dir,0700 or die "phase already exists or mkdir failed: $!\n";
 my $before=inputs($dir,'before',$pins,$image);
 die "native image no longer matches successful build\n" if defined($expected_image) && $before->{native_image}->{sha256} ne $expected_image;
 my $format="elapsed_seconds\t%e\nuser_seconds\t%U\nsystem_seconds\t%S\nmax_rss_kib\t%M\nexit\t%x";
 my @wrapped=('/usr/bin/time','-o',"$dir/time.tsv",'-f',$format,
  '/usr/bin/timeout','--signal=TERM','--kill-after=10s','900s',
  '/usr/bin/prlimit','--as=4294967296','--',@{$spec->{argv}});
 save("$dir/argv.json",{argv=>$spec->{argv},wrapper_argv=>\@wrapped,cwd=>$spec->{cwd},environment=>{LC_ALL=>'C',TZ=>'UTC'},unset_environment=>['LKJSCRIPT_STORE_IO'],stdin=>'/dev/null'});
 my $before_utc=utc(); my $before_load=load3(); my $before_ns=ns();
 my $pid=fork(); die "fork: $!\n" unless defined($pid);
 if($pid==0) {
  chdir $spec->{cwd} or die "child cwd: $!\n";
  open STDIN,'<','/dev/null' or die "stdin: $!\n";
  sysopen STDOUT,"$dir/stdout",O_WRONLY|O_CREAT|O_EXCL,0600 or die "stdout: $!\n";
  sysopen STDERR,"$dir/stderr",O_WRONLY|O_CREAT|O_EXCL,0600 or die "stderr: $!\n";
  exec {$wrapped[0]} @wrapped or die "exec resource wrapper: $!\n";
 }
 my $waited; do {$waited=waitpid($pid,0)} while $waited<0 && $!{EINTR};
 die "waitpid: $!\n" unless $waited==$pid;
 my $wait_status=$?;
 my $after_ns=ns(); my $after_load=load3(); my $after_utc=utc();
 my $signal=$wait_status&127; my $exit=$signal ? 128+$signal : $wait_status>>8;
 my $elapsed=Math::BigInt->new($after_ns)->bsub($before_ns)->bstr();
 save("$dir/status.json",{schema=>'fingerprint-golden-phase-status-1',phase=>$spec->{id},exit_code=>$exit,wait_status=>$wait_status,signal=>$signal,
  before_utc=>$before_utc,after_utc=>$after_utc,before_ns=>"$before_ns",after_ns=>"$after_ns",wrapper_elapsed_ns=>"$elapsed",load_before=>$before_load,load_after=>$after_load,
  timing_boundary=>'Wrapper ns begins immediately before fork and ends after wait; includes resource/process launch, stdout/stderr capture and terminal timestamp acquisition. Source/prod verification and metadata before/after are outside it. GNU time is separately retained.',
  stdout=>digest("$dir/stdout"),stderr=>digest("$dir/stderr"),gnu_time=>-f "$dir/time.tsv"?digest("$dir/time.tsv"):undef});
 my $after=inputs($dir,'after',$pins,$image);
 die "native image changed while running\n" if defined($image) && $before->{native_image}->{sha256} ne $after->{native_image}->{sha256};
 die "phase $spec->{id} failed with status $exit; no retry\n" if $exit!=0;
 my $t=raw("$dir/time.tsv",4096); my %time;
 for my $line(split /\n/,$t) {
  die "malformed GNU time row\n" unless $line=~/\A(elapsed_seconds|user_seconds|system_seconds|max_rss_kib|exit)\t([0-9]+(?:\.[0-9]+)?)\z/;
  die "duplicate GNU time row\n" if exists $time{$1}; $time{$1}=$2;
 }
 die "incomplete GNU time record\n" unless keys(%time)==5 && $time{exit}==0;
 return $dir;
}
sub validate_build {
 my($dir,$output)=@_; my $err=raw("$dir/stderr",64*1024*1024);
 my @rows=$err=~/^LKJSCRIPT_STATS (.*)$/mg; die "one build stats row required\n" unless @rows==1;
 my $s=JSON::PP->new->decode($rows[0]);
 die "build stats not done\n" unless ($s->{route}//'') eq 'compiler' && ($s->{stopped_stage}//'') eq 'done';
 for my $k(qw(build_reused checked_reused emitted_reused inline_calls inline_accepted)) {
  die "nonzero/missing $k\n" unless exists($s->{$k}) && "$s->{$k}" eq '0';
 }
 my $image=digest($output);
 die "bad emitted_bytes\n" unless exists($s->{emitted_bytes}) && "$s->{emitted_bytes}"=~/\A[0-9]+\z/ && $s->{emitted_bytes}==$image->{bytes};
 die "unexpected store write\n" if exists($s->{store_writes}) && "$s->{store_writes}" ne '0';
 save("$dir/validation.json",{schema=>'fingerprint-golden-build-validation-1',status=>'accepted',stats=>$s,image=>$image,
  store_writes_phase=>exists($s->{store_writes})?'present_zero':'NA_no_store_counter_not_emitted'});
 return $image;
}
sub validate_dump {
 my($dir,$pins)=@_; my $b=raw("$dir/stdout",16*1024*1024);
 my @line=split /\n/,$b,-1; die "dump missing terminal newline\n" unless pop(@line) eq '';
 my $at=0; die "dump header\n" unless $line[$at++] eq 'lkjscript-fingerprint-goldens 1';
 my %none=map{$_=>1}@{$pins->{none_ordinals}}; my @cases; my $none_count=0;
 for my $index(0..16) {
  my $start=$at; my $id=$pins->{cases}->[$index];
  die "case order/id mismatch at $index\n" unless defined($line[$at]) && $line[$at++] eq "case $index $id";
  my $kind=$line[$at++]; my $record={ordinal=>$index,id=>$id};
  if(defined($kind) && $kind eq 'none') {
   die "unexpected None at $index\n" unless $none{$index};
   ++$none_count; $record->{outcome}='None';
  } else {
   die "unexpected Some or malformed vector at $index\n" unless !$none{$index} && defined($kind) && $kind=~/\Asome ([0-9a-f]{64}) (0|[1-9][0-9]*)\z/;
   my($shape,$count)=($1,0+$2); die "body vector bound\n" if $count>65536;
   my @bodies;
   for my $slot(0..$count-1) {
    die "body slot/framing at $index/$slot\n" unless defined($line[$at]) && $line[$at++]=~/\Abody \Q$slot\E (empty|[0-9a-f]{64})\z/;
    push @bodies,$1 eq 'empty'?'':$1;
   }
   $record->{outcome}='Some'; $record->{shape}=$shape; $record->{bodies}=\@bodies;
  }
  die "case terminator\n" unless defined($line[$at]) && $line[$at++] eq 'end';
  $record->{exact_record}=join("\n",@line[$start..$at-1])."\n";
  push @cases,$record;
 }
 die "dump population/footer/trailing data\n" unless $none_count==6 && defined($line[$at]) && $line[$at++] eq 'complete 17' && $at==@line;
 save("$dir/goldens.json",{schema=>'fingerprint-original-baseline-capture-1',status=>'captured_pending_root_review_not_installed',
  raw_sha256=>sha256_hex($b),raw_bytes=>length($b),cases=>\@cases,some=>11,none=>6,
  producer_sha256=>$pins->{producer}->{sha256},incremental_module_sha256=>$pins->{incremental_module_sha256},
  qualification=>'Complete captured original-function records only. expected() and every production file remain unchanged; no candidate calculation or additional case supplies these values.'});
 return {cases=>17,some=>11,none=>6,stdout_sha256=>sha256_hex($b),stdout_bytes=>length($b)};
}
die "usage: perl capture.pl $root/release.json\n" unless @ARGV==1 && $ARGV[0] eq "$root/release.json";
my $release=JSON::PP->new->decode(raw($ARGV[0],65536));
die "execution not released\n" unless ($release->{schema}//'') eq 'compiler-fingerprint-baseline-golden-release-1'
 && ($release->{status}//'') eq 'RELEASED_BY_ROOT_FOR_ONE_BUILD_ONE_DUMP' && ($release->{run_root}//'') eq $root;
my $manifest=raw("$control/manifest.sha256");
die "release/control manifest mismatch\n" unless sha256_hex($manifest) eq ($release->{control_manifest_sha256}//'');
my %named;
for my $line(split /\n/,$manifest) {
 die "control manifest grammar\n" unless $line=~/\A([0-9a-f]{64})  ([A-Za-z0-9._-]+)\z/;
 my($sha,$name)=($1,$2); die "duplicate control\n" if $named{$name}++;
 die "control bytes drift: $name\n" unless digest("$control/$name")->{sha256} eq $sha;
}
opendir my $dh,$control or die "control directory: $!\n"; my @names=grep{$_ ne '.' && $_ ne '..' && $_ ne 'manifest.sha256'}readdir($dh); closedir $dh;
die "unlisted control\n" unless @names==keys(%named) && !grep{!$named{$_}}@names;
my $pins=JSON::PP->new->decode(raw("$control/pins.json"));
my $plan=JSON::PP->new->decode(raw("$control/plan.json"));
die "plan/root mismatch\n" unless $pins->{root} eq $root && $pins->{source} eq "$root/source" && $plan->{run_root} eq $root;
die "phase population mismatch\n" unless @{$plan->{phases}}==2 && $plan->{phases}->[0]->{id} eq 'build' && $plan->{phases}->[1]->{id} eq 'dump';
checked_source($pins); checked_producer($pins);
die "native output already exists\n" if lstat("$root/bin/fingerprint-golden");
mkdir "$root/records/execution",0700 or die "execution already attempted or mkdir failed: $!\n";
$ENV{LC_ALL}='C'; $ENV{TZ}='UTC'; delete $ENV{LKJSCRIPT_STORE_IO};
save("$root/records/execution/release.json",$release);
save("$root/records/execution/controls.json",{manifest_sha256=>sha256_hex($manifest),pins_sha256=>sha256_hex(raw("$control/pins.json")),plan_sha256=>sha256_hex(raw("$control/plan.json"))});
my @state=({phase=>'build',status=>'not-run'},{phase=>'dump',status=>'not-run'});
my $ok=eval {
 $state[0]->{status}='attempted';
 my $build_dir=phase($plan->{phases}->[0],$pins,undef);
 my $image=validate_build($build_dir,"$root/bin/fingerprint-golden");
 $state[0]->{status}='accepted'; $state[0]->{image}=$image;
 $state[1]->{status}='attempted';
 my $dump_dir=phase($plan->{phases}->[1],$pins,"$root/bin/fingerprint-golden",$image->{sha256});
 $state[1]->{result}=validate_dump($dump_dir,$pins); $state[1]->{status}='accepted';
 1;
};
my $failure=$@;
save("$root/records/execution/outcome.json",{schema=>'fingerprint-baseline-golden-outcome-1',status=>$ok?'captured_pending_review':'stopped',
 phases=>\@state,failure=>$ok?undef:$failure,source_manifest_sha256=>$pins->{source_manifest_sha256},producer_sha256=>$pins->{producer}->{sha256},
 policy=>'Exactly one build and at most one native dump. Any failed command or validation is terminal for this attempt. No retry or expected() installation.'});
die $failure unless $ok;
print "captured 17 original-function cases: 11 Some / 6 None; pending root review\n";
