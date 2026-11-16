use strict;
use warnings FATAL => 'all';
use JSON::PP ();
use Digest::SHA qw(sha256_hex);
use Fcntl qw(O_RDONLY O_NOFOLLOW O_WRONLY O_CREAT O_EXCL);
use Errno qw(ENOENT);
use POSIX qw(strftime);
use Config ();
die "64-bit Perl required\n" unless $Config::Config{ivsize} >= 8;
my $root='/home/coder/workspace/lkjscript';
my $prep="$root/target/compiler-scalar-cohort-cleanup-preparation-1";
my $raw="$root/target/compiler-scalar-cohort-cleanup-execution-1";
my $out="$root/target/compiler-scalar-cohort-cleanup-acceptance-1";
my $release='2af0ea4e10417850ad4277d6ff6e4bc315375417';
my $commit='0b0b27803d27b932a5b91cde2d766bd3c8bb82b9';
my $package='bench/compiler-scalar-simplify-data/cohort-package';
my $retained_prep='bench/compiler-scalar-simplify-data/cleanup-preparation';
my $emitter='bench/compiler-scalar-simplify-data/preparation/artifacts/f85624e3cc694f6a3a3e10c33f822789ae6084e2d5a5b41c3b1fc8fcc9bd2d64.compiler.gz';
my $canonical='6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc';
my @roots=('target/compiler-scalar-simplify.20261116-1','target/compiler-scalar-trial-capture.20261116-1');
my $j=JSON::PP->new->canonical->pretty;
my %inputs;
sub need ($$) { my ($ok,$why)=@_; die "$why\n" unless $ok; }
sub same { my ($a,$b,$why)=@_; need(defined($a)&&defined($b)&&$a eq $b,$why); }
sub bytes {
 my ($p)=@_; my @a=lstat($p); need(@a && -f _ && !-l _,"not regular: $p");
 sysopen(my $f,$p,O_RDONLY|O_NOFOLLOW) or die "open $p: $!\n"; binmode $f;
 my @s=stat($f); need($s[0]==$a[0] && $s[1]==$a[1],"changed opened file");
 local $/; my $b=<$f>; $b='' unless defined $b; close $f or die "close $p: $!\n";
 my @z=lstat($p); need(@z && $z[0]==$a[0] && $z[1]==$a[1] && $z[7]==$a[7] && $z[9]==$a[9] && length($b)==$a[7],"changed read $p");
 $inputs{$p}={bytes=>length($b),sha256=>sha256_hex($b)}; return $b;
}
sub line { my $b=bytes($_[0]); need($b =~ s/\n\z// && $b !~ /[\r\n\0]/,"line framing"); return $b; }
sub json { return JSON::PP->new->decode(bytes($_[0])); }
sub git {
 open my $f,'-|','git','-C',$root,@_ or die "git: $!\n"; binmode $f;
 local $/; my $b=<$f>; close $f or die "git exit $?\n"; return $b;
}
sub pinned {
 my ($p,$sha)=@_; my $b=bytes($p); same(sha256_hex($b),$sha,"pin $p"); return $b;
}
sub write_new {
 my ($name,$b)=@_; sysopen(my $f,"$out/$name",O_WRONLY|O_CREAT|O_EXCL,0600) or die "$name: $!\n";
 binmode $f; print {$f} $b or die "$name write\n"; close $f or die "$name close\n";
}
same(git('rev-parse',"$release^{commit}"),"$release\n",'release commit');
same(git('rev-parse',"$commit^{commit}"),"$commit\n",'recovery commit');
for my $r (
 ['guard.pl','486fcd8db56a2ccd1d36117df5621cb97fa5656320cfe15511ce509632f4bf27'],
 ['launch.sh','5b41fed67f2a9c6b84ff01dabda0adc0d4f4e9e5d98c5b62efdd28426ea311d4'],
 ['snapshot.json','529395e9b74464531137d27bc05b72df3d368bbe0c77edfbea97f68149efe132']
) {
 my $b=pinned("$prep/$r->[0]",$r->[1]);
 same($b,git('show',"$release:$retained_prep/$r->[0]"),"released $r->[0]");
}
my $snapshot=json("$prep/snapshot.json");
same($snapshot->{repo},$root,'snapshot repo'); same($snapshot->{commit},$commit,'snapshot commit');
same(join("\0",@{$snapshot->{roots}}),join("\0",@roots),'snapshot roots');
same($snapshot->{files},337,'snapshot files'); same($snapshot->{bytes},207027139,'snapshot bytes'); same(scalar(@{$snapshot->{directories}}),22,'snapshot dirs');
my %meta;
for my $r (
 ['inventory','retention-proposal/inventory.json','235e976799d2640bf6599068204d7955bd359673696b3e657387b021f822a070'],
 ['policy','retention-proposal/later-artifacts.json','5f99c5746ce18811468904ff4ff6a979fa994634f242df518a2076abaed6e1de'],
 ['proof','verification-corrected.json','1df0de7ff739413788067a4c131a7d74889f3a7a2b56784baeb349052e7fd8da']
) {
 my $b=pinned("$root/$package/$r->[1]",$r->[2]); same($b,git('show',"$commit:$package/$r->[1]"),"committed $r->[0]");
 $meta{$r->[0]}=JSON::PP->new->decode($b);
}
my $manifest=pinned("$root/$package/manifest.sha256",'3bea8d64d065041a61c44e654c7e65315f4a6f28ddd0d8124310d9b69e602438');
same($manifest,git('show',"$commit:$package/manifest.sha256"),'committed recovery manifest');
my %manifest_paths;
for my $l (split /\n/,$manifest) { need($l =~ /\A([0-9a-f]{64})  ([A-Za-z0-9_.\/-]+)\z/,'manifest row'); $manifest_paths{"$package/$2"}=$1; }
same(scalar(keys %manifest_paths),70,'manifest count');
$manifest_paths{"$package/manifest.sha256"}='3bea8d64d065041a61c44e654c7e65315f4a6f28ddd0d8124310d9b69e602438';
$manifest_paths{$emitter}='ec4d686eb3a296c97757c479ce308a7c6388f3da8d30dca0079af9cb5684e7c2';
my %blobs;
for my $l (split /\0/,git('ls-tree','-r','-z','--full-tree',$commit,'--',$package,$emitter)) {
 need($l =~ /\A(100644|100755) blob ([0-9a-f]{40})\t(.+)\z/s,'Git recovery row'); my ($sha,$p)=($2,$3);
 need(exists($manifest_paths{$p})&&!exists($blobs{$p}),'unexpected Git recovery path'); $blobs{$p}=$sha;
}
same(scalar(keys %blobs),72,'durable recovery population');

my %inventory; my %classes;
for my $e (@{$meta{inventory}{files}}) {
 need(!exists($inventory{$e->{path}}),'duplicate inventory original'); $inventory{$e->{path}}=$e; $classes{$e->{classification}}++;
}
same(scalar(keys %inventory),337,'inventory originals');
my %mapping;
sub mapped {
 my ($p,$class,$sha,$n,$where)=@_; my $e=$inventory{$p}; need(defined($e)&&!exists($mapping{$p}),'invalid recovery mapping');
 same($e->{classification},$class,'mapping class'); same($e->{sha256},$sha,'mapping digest'); same($e->{bytes},$n,'mapping bytes'); $mapping{$p}=$where;
}
my $proof=$meta{proof}; my $policy=$meta{policy};
for my $r (@{$proof->{raw}{rows}}) { need($r->{byte_equal},'raw equality proof'); mapped($r->{path},'retain-raw-record',$r->{sha256},$r->{bytes},"$package/$proof->{raw}{artifact}::$r->{path}"); }
for my $r (@{$policy->{new_gzips}}) {
 my $class=$r->{source} =~ /producer-A\z/?'retain-producer-A':$r->{source} =~ /producer-B\z/?'retain-producer-B':'retain-source-archive';
 need(exists($blobs{"$package/$r->{artifact}"}),'gzip recovery committed'); mapped($r->{source},$class,$r->{original_sha256},$r->{original_bytes},"$package/$r->{artifact}");
}
for my $r (@{$proof->{source_archive}{rows}}) { need($r->{byte_equal},'source equality proof'); mapped("$roots[0]/fixture/source/$r->{path}",'source-tree-duplicate',$r->{sha256},$r->{bytes},"$package/$proof->{source_archive}{artifact}::$r->{path}"); }
my $e=$proof->{existing_emitter}; need($e->{full_bytes_equal},'emitter equality proof');
mapped("$roots[0]/fixture/emitter",'existing-emitter-duplicate',$e->{original_sha256},$e->{original_bytes},$emitter);
my $seed=$proof->{shared_seed}; need($seed->{all_seven_byte_equal},'seed equality proof');
for my $p (@{$seed->{outputs}}) { mapped($p,'seed-equal-output-duplicate',$seed->{sha256},$seed->{bytes},"$package/$proof->{source_archive}{artifact}::seed/lkjscript.gz (expanded)"); }
same(scalar(keys %mapping),337,'complete expected recovery map');

my @raw_names=qw(argv.json cwd end.ns end.utc guard.sha256 limits load-after load-before start.ns start.utc status stderr stdout time);
opendir my $d,$raw or die "$!\n"; my @actual=sort grep {$_ ne '.' && $_ ne '..'} readdir $d; closedir $d;
same(join("\0",@actual),join("\0",sort @raw_names),'raw capture population');
for my $name (@raw_names) { bytes("$raw/$name"); }
same(line("$raw/status"),0,'captured exit'); same(bytes("$raw/stderr"),'','empty stderr'); same(line("$raw/cwd"),$root,'captured cwd');
same(line("$raw/guard.sha256"),"486fcd8db56a2ccd1d36117df5621cb97fa5656320cfe15511ce509632f4bf27  $prep/guard.pl",'captured guard');
my $argv=json("$raw/argv.json");
my @wanted=('/usr/bin/time','-o',"$raw/time",'-f','wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x','/usr/bin/timeout','--signal=TERM','--kill-after=10s','900s','/usr/bin/prlimit','--as=4294967296','--','perl',"$prep/guard.pl",'--remove');
same(join("\0",@{$argv->{argv}}),join("\0",@wanted),'exact cleanup argv');
same($argv->{cwd},$root,'argv cwd'); same($argv->{recovery_commit},$commit,'argv recovery commit');
same($argv->{guard_sha256},'486fcd8db56a2ccd1d36117df5621cb97fa5656320cfe15511ce509632f4bf27','argv guard');
same($argv->{snapshot_sha256},'529395e9b74464531137d27bc05b72df3d368bbe0c77edfbea97f68149efe132','argv snapshot');
my @time=line("$raw/time") =~ /\Awall_s=([0-9.]+) user_s=([0-9.]+) system_s=([0-9.]+) maxrss_kib=(\d+) exit_status=(\d+)\z/;
same(scalar(@time),5,'GNU time framing'); same($time[4],0,'GNU exit');
my $start=line("$raw/start.ns"); my $end=line("$raw/end.ns"); need($start =~ /^\d+$/ && $end =~ /^\d+$/ && $end>$start,'ns ordering');
my $load_before=line("$raw/load-before"); my $load_after=line("$raw/load-after");
my @before=split / /,$load_before; my @after=split / /,$load_after; need(@before>=3 && @after>=3,'load capture');
my @events=map {JSON::PP->new->decode($_)} split /\n/,bytes("$raw/stdout");
same(scalar(@events),363,'event count');
my $pre=shift @events; same($pre->{event},'preflight-pass','preflight first'); same($pre->{mode},'--remove','remove mode');
same($pre->{commit},$commit,'preflight recovery commit'); same($pre->{files},337,'preflight files'); same($pre->{bytes},207027139,'preflight bytes'); same($pre->{directories},22,'preflight directories');
same($pre->{committed_recovery_files},72,'preflight committed recovery'); same($pre->{canonical_sha256},$canonical,'preflight canonical');
same($j->encode($pre->{recovery_classes}),$j->encode(\%classes),'preflight recovery classes');
my $count=0;
for my $p (sort keys %inventory) {
 my $event=shift @events; same($event->{event},'unlinked','unlink sequence'); same($event->{path},$p,'exact unique unlink path');
 same($event->{count},++$count,'unlink count'); same($event->{recovery},$mapping{$p},'unlink recovery mapping');
}
my @directories=map {$_->{path}} @{$snapshot->{directories}};
my %dir_unique=map {$_=>1} @directories; same(scalar(keys %dir_unique),22,'unique directories');
$count=0;
for my $p (sort {length($b)<=>length($a)||$b cmp $a} @directories) {
 my $event=shift @events; same($event->{event},'removed-empty-directory','directory phase'); same($event->{path},$p,'exact rmdir path'); same($event->{count},++$count,'rmdir count');
}
my @absence;
for my $p (@roots) {
 my $event=shift @events; same($event->{event},'root-absent','ENOENT phase'); same($event->{path},$p,'ENOENT root'); same($event->{errno},'ENOENT','recorded ENOENT');
 $!=0; my @s=lstat("$root/$p"); my $err=0+$!; need(!@s && $err==ENOENT,"current root not ENOENT: $p");
 push @absence,{path=>"$root/$p",errno=>'ENOENT'};
}
my $complete=shift @events;
same($complete->{event},'cleanup-complete','completion last'); same($complete->{removed_files},337,'completed files'); same($complete->{removed_directories},22,'completed directories');
same($complete->{canonical_sha256},$canonical,'completion canonical'); same($complete->{unrelated_paths_mutated},0,'declared mutation scope'); same(scalar(@events),0,'no extra events');
my $canonical_bytes=bytes("$root/target/lkjscript"); same(sha256_hex($canonical_bytes),$canonical,'current canonical digest'); same(length($canonical_bytes),32584542,'canonical bytes');
my @current=lstat("$root/target/lkjscript"); my $pin=$snapshot->{canonical};
same($current[0],$pin->{dev},'canonical device'); same($current[1],$pin->{ino},'canonical inode'); same(sprintf('%04o',$current[2]&07777),$pin->{mode},'canonical mode');

my $summary={
 schema=>'compiler-scalar-cohort-cleanup-independent-acceptance-1',status=>'accepted',release_commit=>$release,recovery_commit=>$commit,
 gateway_launch=>'compiler-scalar-cohort-cleanup-v1457',gateway_completion_note=>'root reported final poll v1458',
 raw_capture=>$raw,guard_sha256=>$argv->{guard_sha256},snapshot_sha256=>$argv->{snapshot_sha256},
 files_removed=>337,directories_removed=>22,logical_bytes_removed=>207027139,events=>363,unique_exact_recovery_mappings=>337,
 raw_capture_files=>14,retained_committed_recovery_files=>72,recovery_classes=>\%classes,current_root_absence=>\@absence,
 canonical=>{path=>"$root/target/lkjscript",sha256=>$canonical,bytes=>32584542,device=>$current[0],inode=>$current[1],mode=>$pin->{mode}},
 measured_cleanup=>{wall_s=>$time[0],user_s=>$time[1],system_s=>$time[2],cpu_s=>$time[1]+$time[2],peak_rss_kib=>$time[3],wrapper_exit=>0,gnu_exit=>0,
 start_ns=>$start,end_ns=>$end,launch_inclusive_wall_ns=>''.($end-$start),start_utc=>line("$raw/start.utc"),end_utc=>line("$raw/end.utc"),
 load_before=>[@before[0..2]],load_after=>[@after[0..2]]},
 observed_utc=>strftime('%Y-%m-%dT%H:%M:%SZ',gmtime),
 qualifications=>[
  'Read-only result audit: guard and launcher were not rerun, and no compiler/native executable, decompressor, compressor, process observer or broad original-tree scan ran.',
  'All 363 progress events were checked against exact committed metadata, including the complete 337-path recovery map and directory order. Only the two named roots were tested for current ENOENT.',
  'Current canonical bytes/device/inode/mode match the prepared snapshot. Recovery commit identity and committed metadata/blob population were verified without repeating archive expansion or the prior full artifact audit.',
  'The recorded cooperative non-atomic cleanup completed successfully. Event scope and inspected guard delimit the action; this is not a global proof about unrelated concurrent filesystem activity.',
  'GNU elapsed and launch-inclusive nanosecond intervals have different boundaries and remain separate.'
 ]
};
write_new('acceptance.json',$j->encode($summary));
write_new('input-pins.json',$j->encode(\%inputs));
write_new('recovery-blobs.json',$j->encode(\%blobs));
print "accepted: 337 exact unlink mappings, 22 directories, two current ENOENT roots, canonical preserved\n";
