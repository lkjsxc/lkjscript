#!/usr/bin/env perl
use strict;
use warnings FATAL => 'all';
use Config;
use Fcntl qw(O_RDONLY O_NOFOLLOW O_CREAT O_EXCL O_WRONLY S_ISREG S_ISDIR);
use Digest::SHA qw(sha256_hex);
use JSON::PP ();
use File::Find ();
use Time::Local qw(timegm);

my $R = '/home/coder/workspace/lkjscript';
my $G = 'bench/compiler-fingerprint-index-data/gate-1';
my $O = 'bench/compiler-fingerprint-index-data/gate-acceptance-1';
my $WR = 'bench/compiler-fingerprint-index-data/gate-run-1.sh';
my $SRC_COMMIT = '31f412a0a7df47d897b6b39e794a9b152b4fa29e';
my $SEED_COMMIT = 'e5294c339d9aefbbafc2e951c997f4fefabcc9ad';
my $J = JSON::PP->new->canonical->pretty;
my $SCALAR_JSON = JSON::PP->new->allow_nonref;
my %read_pins;

sub need ($$) { die "audit refusal: $_[1]\n" unless $_[0]; }
sub same ($$$) { need(defined($_[0]) && defined($_[1]) && "$_[0]" eq "$_[1]", $_[2]); }
sub safe ($) { my ($p)=@_; need($p ne '' && $p !~ m{\A/|(?:\A|/)\.\.?(?:/|\z)|[\0\r\n]}, "unsafe relative path"); return $p; }
sub stat_key ($) { my ($s)=@_; return join(':', @$s[0,1,2,7,9,10]); }
sub opened ($) {
    my ($p)=@_; safe($p);
    my @s=lstat("$R/$p"); need(@s && S_ISREG($s[2]), "not a regular file: $p");
    sysopen(my $fh, "$R/$p", O_RDONLY|O_NOFOLLOW) or die "open $p: $!";
    binmode $fh; my @f=stat($fh);
    same(stat_key(\@f),stat_key(\@s),"changed before read: $p");
    return ($fh,\@s);
}
sub read_raw ($) {
    my ($p)=@_; my ($fh,$s)=opened($p); need($s->[7]<=16*1024*1024,"oversized text record: $p");
    my $b=do { local $/; <$fh> }; $b='' unless defined $b;
    my @after=stat($fh); close($fh) or die "close $p: $!";
    same(stat_key(\@after),stat_key($s),"changed during read: $p"); same(length($b),$s->[7],"short read: $p");
    $read_pins{$p}={sha256=>sha256_hex($b),bytes=>length($b)};
    return $b;
}
sub hash_file ($) {
    my ($p)=@_; my ($fh,$s)=opened($p); my $h=Digest::SHA->new(256); $h->addfile($fh);
    my @after=stat($fh); close($fh) or die "close $p: $!";
    same(stat_key(\@after),stat_key($s),"changed during hash: $p");
    return {sha256=>$h->hexdigest,bytes=>$s->[7]};
}
sub save ($$) {
    my ($name,$b)=@_; safe($name); need($name !~ m{/},"nested output refused");
    sysopen(my $fh,"$R/$O/$name",O_WRONLY|O_CREAT|O_EXCL,0600) or die "claim output $name: $!";
    binmode $fh; print {$fh} $b or die "write $name: $!"; close($fh) or die "close $name: $!";
}
sub git (@) {
    my @a=('git','-c','core.packedGitWindowSize=1m','-c','core.packedGitLimit=64m','-C',$R,@_);
    open(my $fh,'-|',@a) or die "Git open: $!"; binmode $fh;
    my $b=do { local $/; <$fh> }; close($fh) or die "Git read failed: @a";
    return defined($b)?$b:'';
}
sub millis ($) { my ($s)=@_; need($s =~ /\A(\d+)\.(\d{3})\z/,"milliseconds framing"); return $1*1000+$2; }
sub centis ($) { my ($s)=@_; need($s =~ /\A(\d+)\.(\d{2})\z/,"centiseconds framing"); return $1*100+$2; }
sub utc_ns ($) {
    my ($s)=@_; need($s =~ /\A(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})\.(\d{9})Z\n\z/,"UTC framing");
    my ($y,$mo,$d,$h,$mi,$se,$ns)=($1,$2,$3,$4,$5,$6,$7);
    return int(timegm($se,$mi,$h,$d,$mo-1,$y))*1_000_000_000+$ns;
}
sub uint ($) {
    my ($v)=@_; return 0 unless defined($v) && !ref($v);
    return $SCALAR_JSON->encode($v) =~ /\A(?:0|[1-9]\d*)\z/;
}
sub manifest ($) {
    my ($b)=@_; need($b =~ /\n\z/,"manifest terminator"); my %m;
    for my $line (split /\n/,$b) {
        need($line =~ /\A([0-9a-f]{64})  (.+)\z/,"manifest framing");
        my ($h,$p)=($1,$2); safe($p); need(!exists $m{$p},"duplicate manifest path: $p"); $m{$p}=$h;
    }
    return \%m;
}
need($Config{ivsize}>=8,"64-bit integer runtime required");
same(git('cat-file','-t',$SRC_COMMIT),"commit\n","source commit identity");
same(git('cat-file','-t',$SEED_COMMIT),"commit\n","seed commit identity");
my $observed_head=git('rev-parse','HEAD'); chomp $observed_head;
my @top=qw(inputs-before.sha256 producer-seed-before.sha256 wrapper-before.sha256 base-commit source-diff-before cwd environment free-space argv start.utc load-before start.ns stdout stderr time end.ns load-after end.utc status inputs-after.sha256 producer-seed-after.sha256 wrapper-after.sha256 output-sizes log-paths raw.sha256);
opendir(my $gd,"$R/$G") or die "gate directory: $!"; my @found=sort grep {$_ ne '.' && $_ ne '..'} readdir($gd); closedir($gd);
same(join("\0",@found),join("\0",sort(@top,'logs')),"gate top-level population");
my %raw=map {$_=>read_raw("$G/$_")} @top;
same($raw{status},"0\n","gate exit"); same($raw{stderr},'',"gate stderr"); same($raw{cwd},"$R\n","gate cwd");
same($raw{'base-commit'},"88a06df2b212d5856b62aa9bcb49e9092cad1d10\n","captured source working base");
same($raw{'inputs-before.sha256'},$raw{'inputs-after.sha256'},"input manifests changed");
same($raw{'wrapper-before.sha256'},$raw{'wrapper-after.sha256'},"wrapper changed");
my $wrapper_sha='df5b200d730406d8ce2c3d7975cbfd716d86926d14e8ea40d6401f7a9d55455a';
same($raw{'wrapper-before.sha256'},"$wrapper_sha  $WR\n","wrapper pin capture"); same(sha256_hex(read_raw($WR)),$wrapper_sha,"current wrapper");
my @argv=('/usr/bin/time','-o',"$R/$G/time",'-f','wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x','/usr/bin/timeout','--signal=TERM','--kill-after=10s','2400s','/usr/bin/prlimit','--as=4294967296','--','bash','tools/check.sh');
same($raw{argv},join('',map {$_."\0"} @argv),"actual wrapper argv");
same($raw{environment},"LC_ALL=C\nTZ=UTC\nLKJSCRIPT_STORE_IO=unset\nLKJSCRIPT_GATE_JOBS=4\nLKJSCRIPT_STORE=$R/target/compiler-fingerprint-index-gate-store-1\nLKJSCRIPT_STORE_KEY=/home/coder/.local/state/lkjscript/store-key\n","declared environment");
my $raw_expected=join('',map {sha256_hex($raw{$_})."  $R/$G/$_\n"} qw(stdout stderr time));
same($raw{'raw.sha256'},$raw_expected,"raw capture digests");
my ($start,$end)=map {need($raw{$_}=~/\A(\d{19})\n\z/,"nanosecond framing"); 0+$1} qw(start.ns end.ns);
need($end>$start,"outer clock ordering"); my $wall_ns=$end-$start;
my ($utc_before,$utc_after)=map {utc_ns($raw{$_})} qw(start.utc end.utc);
need($utc_before<=$start && $start-$utc_before<1_000_000_000 && $utc_after>=$end && $utc_after-$end<1_000_000_000,"UTC adjacency");
need($raw{time}=~/\Awall_s=(\d+\.\d{2}) user_s=(\d+\.\d{2}) system_s=(\d+\.\d{2}) maxrss_kib=(\d+) exit_status=(\d+)\n\z/,"GNU time framing");
my %time=(wall_s=>$1,user_s=>$2,system_s=>$3,maxrss_kib=>0+$4,exit_status=>0+$5);
same($time{exit_status},0,"GNU exit");
need(abs(centis($time{wall_s})*10_000_000-$wall_ns)<20_000_000,"GNU versus outer wall");
my %loads;
for my $n (qw(load-before load-after)) {need($raw{$n}=~/\A(\d+\.\d+) (\d+\.\d+) (\d+\.\d+)\n\z/,"load framing"); $loads{$n}=[$1,$2,$3];}

my $inputs=manifest($raw{'inputs-before.sha256'});
my %paths; for my $p (split /\0/,git('ls-files','--cached','--others','--exclude-standard','-z','--',qw(src library tests tools examples witness .github))) {$paths{$p}=1;}
same(join("\0",sort keys %paths),join("\0",sort keys %$inputs),"current complete input path set");
my ($input_bytes,%input_groups)=(0);
for my $p (sort keys %$inputs) {
    need($p =~ m{\A(src|library|tests|tools|examples|witness|\.github)/},"input outside scope");
    $input_groups{$1}++; my $h=hash_file($p); same($h->{sha256},$inputs->{$p},"current input differs: $p"); $input_bytes+=$h->{bytes};
}
my %exact_source=(
 'tools/check.sh'=>'ae4f2e6e698132bd458aab70bd57269a475fad8610c20c732514e03dae384d52',
 'tools/bootstrap.sh'=>'1e8371471dd3b8cba3a4a5c2d6b0bef99d89efa811b4dea7bd46dd762fd5cdc3',
 'tools/closed-semantics.sh'=>'ebbd18fb4edb6fa98b61efc458b38e9d94f3ace1ebf45c362db0b48fa34deb25',
 'src/elaborate/incremental.lkj'=>'5e917048bb58d9cce214a7821ada2cc8a7539209623dc7d20d73d049cebb9316',
 'src/semantic_main.lkj'=>'61cacbbcefc9353d37770f84783780b5d516abaddddd89bf957272c21f70c0f3',
 'src/semantic_tests.lkj'=>'356e0b8fe45bb9b012212cc9116cdb9e042071f9b4de84badaa951b251901de6',
 'src/semantic_tests/incremental_fingerprint.lkj'=>'ccaad8644eddaab9f4ae344648922ef7e3df032751a732e8923dcdbe1df7d956'
);
my %source_bytes;
for my $p (sort keys %exact_source) {
    same($inputs->{$p},$exact_source{$p},"captured reviewed source pin: $p");
    my $b=read_raw($p); same(sha256_hex($b),$exact_source{$p},"reviewed source hash: $p");
    same(git('show',"$SRC_COMMIT:$p"),$b,"committed gated source bytes: $p"); $source_bytes{$p}=$b;
}
my %trees; for my $p(qw(src library tests tools)){my $t=git('rev-parse',"$SRC_COMMIT:$p");chomp $t;need($t=~/\A[0-9a-f]{40}\z/,"Git tree identity");$trees{$p}=$t;}
same($trees{src},'59fcc8447761f00edaae5dacefbf5edb9b23df36',"source tree identity");
same($trees{library},'25bdb8ff8567b941790d3951c78e896b23e26e8e',"library tree identity");
same($trees{tests},'e2b30ac0f2fa8f897f366293189d6af552a41980',"test tree identity");

my $before=manifest($raw{'producer-seed-before.sha256'}); my $after=manifest($raw{'producer-seed-after.sha256'});
same(join(',',sort keys %$before),'seed/lkjscript.gz,target/lkjscript',"producer before paths");
same(join(',',sort keys %$after),'seed/lkjscript.gz,target/lkjscript',"producer after paths");
same($before->{'target/lkjscript'},'b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db',"captured old producer");
same($before->{'seed/lkjscript.gz'},'c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae',"captured seed");
same($after->{'seed/lkjscript.gz'},$before->{'seed/lkjscript.gz'},"seed changed during gate");
need($after->{'target/lkjscript'} ne $before->{'target/lkjscript'},"expected bootstrap transition missing");
my %sizes;
for my $line(split /\n/,$raw{'output-sizes'}) {need($line=~/\A(target\/lkjscript|seed\/lkjscript\.gz) (\d+)\z/,"output size framing");need(!exists $sizes{$1},"duplicate output size");$sizes{$1}=0+$2;}
same(scalar(keys %sizes),2,"output size population");
my %current_artifacts;
for my $p(sort keys %$after) {my $h=hash_file($p);same($h->{sha256},$after->{$p},"current artifact pin: $p");same($h->{bytes},$sizes{$p},"current artifact size: $p");$current_artifacts{$p}=$h;}
same(sha256_hex(git('show',"$SEED_COMMIT:seed/lkjscript.gz")),$after->{'seed/lkjscript.gz'},"unchanged seed Git bytes");

my $check=$source_bytes{'tools/check.sh'};
my @roster=($check=~/^(?:step|parallel_step\s+(?:\d+|"\$check_suite_jobs"))\s+([a-z][a-z0-9-]*)\s/gm);
my @parallel=($check=~/^parallel_step\s+(?:\d+|"\$check_suite_jobs")\s+([a-z][a-z0-9-]*)\s/gm);
my %roster=map {$_=>1} @roster; my %parallel=map {$_=>1} @parallel;
same(scalar(@roster),60,"source phase count");same(scalar(keys %roster),60,"unique source phases");same(scalar(@parallel),54,"parallel roster count");
need(index($check,'step ir-mutation timeout --kill-after=10s 600s "$check_binary" mutate 1701 128 --jobs "$check_jobs"')>=0,"fixed mutation source command");
my @log_paths=split /\n/,$raw{'log-paths'};
same($raw{'log-paths'},join('',map {$_."\n"} sort @log_paths),"sorted log path framing");
my (%listed,%logs,%bound,@log_proof);
$bound{"target/gate/details-$_.log"}='phase-detail' for @roster;
$bound{"target/gate/$_.log"}='phase-summary' for @parallel;
$bound{"target/gate/closed/structure-$_.log"}='structural-runner' for qw(main source_0 source_1 source_2 source_3);
for my $p(@log_paths) {
    need($p=~m{\Atarget/gate/.+\.log\z},"log outside scope");safe($p);need(!$listed{$p}++,"duplicate log");
    my $copy=read_raw("$G/logs/$p");same(read_raw($p),$copy,"permanent log copy differs: $p");$logs{$p}=$copy;
    push @log_proof,{path=>$p,bytes=>length($copy),sha256=>sha256_hex($copy),role=>$bound{$p}//'retained-only'};
}
need(exists($listed{$_}),"missing roster-bound log: $_") for sort keys %bound;
my @copied;
File::Find::find({no_chdir=>1,wanted=>sub {
    my $p=$File::Find::name; my @s=lstat($p);need(@s,"copied path disappeared");
    return if S_ISDIR($s[2]);need(S_ISREG($s[2]),"special copied log");
    my $prefix="$R/$G/logs/";need(index($p,$prefix)==0,"copy path scope");push @copied,substr($p,length($prefix));
}}, "$R/$G/logs");
same(join("\0",sort @copied),join("\0",sort @log_paths),"complete copied log set");

my (@phases,@other,%seen); my ($sum_ms,$fresh,$reused,$stats_records,$closed_records)=(0,0,0,0,0);
for my $line(split /\n/,$raw{stdout}) {
    if ($line =~ /\Aok ([a-z0-9-]+) \((\d+\.\d{3}) s, load (\d+\.\d+) -> (\d+\.\d+); gate\+(\d+\.\d{3})\.\.(\d+\.\d{3}) s\); (.+)\z/) {
        my ($name,$dur,$lb,$la,$lo,$hi,$result)=($1,$2,$3,$4,$5,$6,$7);
        need($roster{$name} && !$seen{$name}++,"unrecognized/duplicate phase");
        my ($dm,$lm,$hm)=map {millis($_)} ($dur,$lo,$hi);need($hm>=$lm && abs($hm-$lm-$dm)<=1,"phase clock rounding: $name");$sum_ms+=$dm;
        my ($rr,$ru,$nr,$nc,$counted)=(0,0,0,0,0);my @row_stats;my @closed;
        for my $d(split /\n/,$logs{"target/gate/details-$name.log"}) {
            if($d =~ /\ALKJSCRIPT_STATS (.+)\z/) {
                my $x=JSON::PP::decode_json($1);need(ref($x) eq 'HASH',"stats object: $name");$nr++;
                for my $k(qw(test_reused test_rerun)) {need(uint($x->{$k}),"noninteger $k: $name") if exists $x->{$k};}
                if(exists $x->{test_reused}) {$ru+=$x->{test_reused};$counted=1;}
                $rr+=$x->{test_rerun}//0;
                push @row_stats,{sha256=>sha256_hex($d."\n"),test_reused=>$x->{test_reused}//0,test_rerun=>$x->{test_rerun}//0,contains_test_reused=>exists($x->{test_reused})?JSON::PP::true:JSON::PP::false,stopped_stage=>$x->{stopped_stage}//undef};
            }
            if($d =~ /\ALKJSCRIPT_CLOSED_GATE /) {
                need($d =~ /\ALKJSCRIPT_CLOSED_GATE reused=(\d+) rerun=(\d+) proof=authenticated-checked-closure-exact-runner-sealed-ELF-and-input-contract closure=([0-9a-f]{64})\z/,"closed record framing");
                my ($ruse,$rrun,$closure)=(0+$1,0+$2,$3);$ru+=$ruse;$rr+=$rrun;$counted=1;$nc++;
                push @closed,{reused=>$ruse,rerun=>$rrun,closure=>$closure};
            }
        }
        my $expected=$nc ? "cases reused=$ru rerun=$rr (authenticated exact stable runner, sealed ELF, checked closure and closed input contract)"
          : $counted && $rr+$ru>0 ? "cases reused=$ru rerun=$rr (authenticated producer and input closure; misses/open effects rerun)"
          : 'rerun suite (live host observations or fresh admission)';
        same($result,$expected,"phase detail aggregation: $name");
        same($logs{"target/gate/$name.log"},"$line\n","parallel summary: $name") if $parallel{$name};
        $fresh+=$rr;$reused+=$ru;$stats_records+=$nr;$closed_records+=$nc;
        push @phases,{name=>$name,wall_ms=>$dm,start_ms=>$lm,end_ms=>$hm,load_before=>$lb,load_after=>$la,rerun=>$rr,reused=>$ru,stats=>\@row_stats,closed=>\@closed,detail_sha256=>sha256_hex($logs{"target/gate/details-$name.log"})};
    } else {push @other,$line;}
}
same(join(',',map {$_->{name}} @phases),join(',',@roster),"actual source-roster phase order");
same($fresh,5459,"fresh request sum");same($reused,0,"reused request sum");same(scalar(@other),2,"unexpected stdout records");
same($other[0],"ok gate selection: skipped $reused case requests on authenticated exact compiler/input closure or stable runner/sealed ELF/checked closure/closed contract equality; reran $fresh misses/open effects; host suites rerun","selection total");
need($other[1]=~/\Aok gate \((\d+\.\d{3}) s, load (\d+\.\d+) -> (\d+\.\d+)\)\z/,"final gate framing");
my ($gate_ms,$glb,$gla)=(millis($1),$2,$3);
same($glb,$loads{'load-before'}[0],"start load");same($gla,$loads{'load-after'}[0],"end load");
need(abs($gate_ms*1_000_000-$wall_ns)<20_000_000,"internal versus outer wall");
need($_->{end_ms}<=$gate_ms,"phase outside gate") for @phases;
my @intervals=sort {$a->[0]<=>$b->[0]} map {[$_->{start_ms},$_->{end_ms}]} @phases;
my ($union,$low,$high)=(0,@{$intervals[0]});
for my $i(@intervals[1..$#intervals]) {if($i->[0]>$high){$union+=$high-$low;($low,$high)=@$i;}elsif($i->[1]>$high){$high=$i->[1];}}
$union+=$high-$low;
my %phase=map {$_->{name}=>$_} @phases;
same(join(',',map {$_->{rerun}} @{$phase{'semantic-structure'}{closed}}),'196,134,134,134,134',"structural runner counts");
same($phase{'semantic-structure'}{rerun},732,"structural total");
same(join('',map {$logs{"target/gate/closed/structure-$_.log"}} qw(main source_0 source_1 source_2 source_3)),$logs{'target/gate/details-semantic-structure.log'},"structural child/detail equality");
same($logs{'target/gate/details-bootstrap.log'},"ok bootstrap\n","fixed-point bootstrap success");

my $mut=$logs{'target/gate/details-ir-mutation.log'};
need($mut =~ /\Aok mutation: (\d+) mutants, (\d+) rejected, (\d+) values, (\d+) traps, (\d+) did not terminate, (\d+) resources, (\d+) findings; (\d+) mutants\/minute \((\d+\.\d{3}) s, (\d+) workers\); (\d+) checked reuses, (\d+) emitted reuses, (\d+) store rejects; inline default\(0\), 0 attempts, 0 accepted, 0 executed, 0 rejected, 0 candidate calls in 0 bodies, 0 local body proofs\n\z/,"mutation record framing");
my @mc=($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13);
same(join(',',@mc[0..6]),'128,104,22,1,1,0,0',"mutation outcomes");
same($mc[9],4,"mutation workers");same($mc[0],$mc[1]+$mc[2]+$mc[3]+$mc[4]+$mc[5],"mutation partition");
need(millis($mc[8])<=$phase{'ir-mutation'}{wall_ms}+1,"mutation internal versus phase wall");
my @extra=sort grep {!exists $bound{$_}} @log_paths;
my $total_log_bytes=0;$total_log_bytes+=$_->{bytes} for @log_proof;
my $report={
 schema=>'compiler-fingerprint-index-gate-independent-acceptance-1',status=>'accepted',
 source=>{captured_working_base=>'88a06df2b212d5856b62aa9bcb49e9092cad1d10',gated_source_commit=>$SRC_COMMIT,observed_head=>$observed_head,trees=>\%trees,input_files=>scalar(keys %$inputs),input_bytes=>$input_bytes,input_groups=>\%input_groups,input_manifest_sha256=>sha256_hex($raw{'inputs-before.sha256'}),before_after_byte_equal=>JSON::PP::true,current_full_path_set_and_hashes_equal=>JSON::PP::true,reviewed_source_sha256=>\%exact_source},
 producer=>{before=>$before,after=>$after,current_artifacts=>\%current_artifacts,seed_git_commit=>$SEED_COMMIT,seed_unchanged=>JSON::PP::true,bootstrap_detail_sha256=>sha256_hex($logs{'target/gate/details-bootstrap.log'}),bootstrap_script_sha256=>$exact_source{'tools/bootstrap.sh'}},
 gate=>{argv=>\@argv,status=>0,gnu=>\%time,gnu_cpu_centiseconds=>centis($time{user_s})+centis($time{system_s}),outer_wall_ns=>sprintf('%d',$wall_ns),start_ns=>sprintf('%d',$start),end_ns=>sprintf('%d',$end),start_utc=>$raw{'start.utc'},end_utc=>$raw{'end.utc'},load_before=>$loads{'load-before'},load_after=>$loads{'load-after'},internal_wall_ms=>$gate_ms,phase_count=>scalar(@phases),phase_duration_sum_ms=>$sum_ms,rounded_phase_interval_union_ms=>$union,rounded_phase_interval_gap_ms=>$gate_ms-$union,rerun=>$fresh,reused=>$reused,stats_records=>$stats_records,closed_records=>$closed_records,structural_contracts=>732},
 mutation=>{seed=>1701,total=>0+$mc[0],rejected=>0+$mc[1],values=>0+$mc[2],traps=>0+$mc[3],nontermination=>0+$mc[4],resources=>0+$mc[5],findings=>0+$mc[6],reported_per_minute=>0+$mc[7],reported_wall_ms=>millis($mc[8]),load_before=>$phase{'ir-mutation'}{load_before},load_after=>$phase{'ir-mutation'}{load_after},workers=>0+$mc[9],checked_reuses=>0+$mc[10],emitted_reuses=>0+$mc[11],store_rejects=>0+$mc[12],detail_sha256=>sha256_hex($mut)},
 retention=>{raw_root=>$G,top_level_files=>scalar(@top),copied_logs=>scalar(@log_paths),log_bytes=>$total_log_bytes,roster_bound_logs=>scalar(keys %bound),retained_only_log_paths=>\@extra,all_listed_live_and_permanent_logs_byte_equal=>JSON::PP::true},
 phases=>\@phases,
 limitations=>[
  'The 60 source-declared successful phases bind freshness. Any extra copied log is retained data only; no freshness claim comes from file presence alone.',
  'The gate began from working base 88a06df2 with the recorded source changes and newly added module. The five gated files were subsequently committed as 31f412a0 without changing their bytes; the complete captured input set still matches.',
  'Bootstrap intentionally changed canonical b01 to the captured after digest. The fixed-point claim is the retained success of the pinned bootstrap comparison script, not a rerun by this audit. Seed c791 stayed unchanged.',
  'Phase wall intervals overlap under four workers. Their sum is not total wall or CPU; rounded interval endpoints carry millisecond precision. GNU CPU is user plus system.',
  'The fixed mutation aggregate is verified, including its 12 checked/5 emitted internal reuses. Those are distinct from the zero skipped case requests. No individual mutant record is reconstructed.',
  'No compiler, native image, test, official reader, new observation, archive operation, source modification, key read or cache read ran in this audit.'
 ]};
save('acceptance.json',$J->encode($report));
save('phases.tsv',"phase\twall_ms\tstart_ms\tend_ms\tload_before\tload_after\trerun\treused\tstats_records\tclosed_records\n".join('',map {join("\t",@$_{qw(name wall_ms start_ms end_ms load_before load_after rerun reused)},scalar(@{$_->{stats}}),scalar(@{$_->{closed}}))."\n"} @phases));
save('log-copy-proof.json',$J->encode(\@log_proof));
save('read-pins.json',$J->encode(\%read_pins));
print "accepted phases=",scalar(@phases)," fresh=$fresh reused=$reused structure=732 mutation=128/0 inputs=",scalar(keys %$inputs)," logs=",scalar(@log_paths)," canonical=$after->{'target/lkjscript'}\n";
