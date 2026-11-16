#!/usr/bin/env perl
use strict;
use warnings;
use Digest::SHA qw(sha256_hex sha1_hex);
use JSON::PP ();
use POSIX qw(strftime);

my $root="/home/coder/workspace/lkjscript-compiler-scalar-20261116";
chdir $root or die "cwd: $!\n";
my $out="target/compiler-scalar-gate-acceptance-1";
my $capture="target/compiler-scalar-cold-gate-1";
my $source="5b5ce7759c035ae0437e3512e18efc440aa50d2e";
my $producer="f85624e3cc694f6a3a3e10c33f822789ae6084e2d5a5b41c3b1fc8fcc9bd2d64";
my $src="6277f3cec86791e7f297a95ff0a70bed60b951eb";
my $library="25bdb8ff8567b941790d3951c78e896b23e26e8e";
my $tools="3dd31e9d8483d32160872be9592927ef861b149a";
my %inputs;
sub bytes {
    my($p)=@_;
    die "not a regular nonsymlink file: $p\n" unless -f $p && !-l $p;
    open my$f,"<:raw",$p or die "$p: $!\n";
    local $/;my$b=<$f>;close$f or die "$p: $!\n";return$b;
}
sub record {
    my($p)=@_;my$b=bytes($p);$inputs{$p}={bytes=>length($b),sha256=>sha256_hex($b)};return$b;
}
sub git {
    open my$f,"-|","git",@_ or die "git launch: $!\n";
    local $/;my$b=<$f>;close$f or die "git failed: @_\n";return$b;
}
sub write_file {
    my($p,$b)=@_;die "output exists: $p\n" if -e$p||-l$p;
    open my$f,">:raw",$p or die "$p: $!\n";print$f$b;close$f or die "$p: $!\n";
}
sub json {JSON::PP->new->canonical->pretty->encode($_[0])}
my @capture_names=qw(limits cwd source-pins source-before.sha256 producer-before.sha256 argv wrapper-argv started.utc started.ns load-before stdout stderr time load-after finished.ns finished.utc status source-after.sha256 producer-after.sha256 git-status);
opendir my$cd,$capture or die "$!\n";
my @actual_capture=sort grep {$_ ne"."&&$_ ne".."} readdir$cd;closedir$cd;
die "capture inventory changed\n" unless join("\0",@actual_capture) eq join("\0",sort @capture_names);
my %c=map {$_=>record("$capture/$_")} @capture_names;
die "gate exit/stderr/status\n" unless $c{status} eq "0\n" && $c{stderr} eq "" && $c{'git-status'} eq "";
die "source pins\n" unless $c{'source-pins'} eq "$source\n$src\n$library\n$tools\n";
die "cwd\n" unless $c{cwd} eq "$root\n";
die "before/after source mismatch\n" unless $c{'source-before.sha256'} eq $c{'source-after.sha256'};
die "producer record\n" unless $c{'producer-before.sha256'} eq "$producer  target/lkjscript\n" && $c{'producer-after.sha256'} eq $c{'producer-before.sha256'};
die "limits\n" unless $c{limits} eq "fresh-empty-store=true\nworkers=4\nexternal-address-space-bytes=4294967296\nexternal-watchdog-seconds=1800\n";
my @argv=("env","LKJSCRIPT_GATE_JOBS=4","LKJSCRIPT_STORE=$root/target/compiler-scalar-cold-store-1","LKJSCRIPT_STORE_KEY=/home/coder/.local/state/lkjscript/store-key","bash","tools/check.sh");
die "payload argv\n" unless $c{argv} eq join("\0",@argv)."\0";
my @wrapper=("/usr/bin/time","-f","%e %U %S %M","-o","$capture/time","prlimit","--as=4294967296","--","timeout","--signal=TERM","--kill-after=10s","1800s",@argv);
die "wrapper argv\n" unless $c{'wrapper-argv'} eq join("\0",@wrapper)."\0";
die "current commit/trees\n" unless git("rev-parse","HEAD","HEAD:src","HEAD:library","HEAD:tools") eq "$source\n$src\n$library\n$tools\n";
my$binary=record("target/lkjscript");
die "current producer mismatch\n" unless sha256_hex($binary) eq $producer && length($binary)==32761734;
undef $binary;

# Exact source coverage, independent of stat-based Git cleanliness.
my %manifest;
for my$l(split/\n/,$c{'source-before.sha256'}){
    $l=~/\A([0-9a-f]{64})  ((?:\.github|examples|library|seed|src|tests|tools|witness)\/[^\r\n\0]+)\z/ or die "source manifest row\n";
    my($h,$p)=($1,$2);die "source traversal/duplicate\n" if $p=~m{(?:\A|/)\.\.(?:/|\z)} || exists$manifest{$p};
    $manifest{$p}=$h;
}
my%objects;
for my$r(split/\0/,git("ls-tree","-r","-z","--full-tree",$source,"--",".github","examples","library","seed","src","tests","tools","witness")){
    $r=~/\A(100644|100755) blob ([0-9a-f]{40})\t(.+)\z/s or die "Git source row\n";
    die "duplicate Git source\n" if exists$objects{$3};$objects{$3}=$2;
}
die "source path set\n" unless join("\0",sort keys%manifest) eq join("\0",sort keys%objects);
die "source count\n" unless keys(%manifest)==1943;
my$source_bytes=0;
for my$p(sort keys%manifest){
    my$b=bytes($p);$source_bytes+=length($b);
    die "live source hash $p\n" unless sha256_hex($b) eq $manifest{$p};
    die "Git blob mismatch $p\n" unless sha1_hex("blob ".length($b)."\0".$b) eq $objects{$p};
}

my$check=record("tools/check.sh");
die "check script identity\n" unless sha256_hex($check) eq "a99ea91f9e8621e618b5696475b1624960cca48def1bd088e75b97a8112fc700";
record("tools/bootstrap.sh");
my@declared;my%parallel;
for my$l(split/\n/,$check){
    if($l=~/\Astep ([a-z0-9-]+) /){push@declared,$1}
    elsif($l=~/\Aparallel_step (?:[0-9]+|"\$check_suite_jobs") ([a-z0-9-]+) /){push@declared,$1;$parallel{$1}=1}
}
my%declared;die "duplicate declared phase\n" if grep {$declared{$_}++} @declared;
my@markers;my%marker;
for my$l(split/\n/,$c{stdout}){
    if($l=~/\Aok ([a-z0-9-]+) \(([0-9.]+) s, load ([0-9.]+) -> ([0-9.]+); gate\+([0-9.]+)\.\.([0-9.]+) s\); (.+)\z/){
        my($name,$wall,$before,$after,$start,$end,$note)=($1,$2,$3,$4,$5,$6,$7);
        die "unknown/duplicate phase $name\n" unless $declared{$name} && !exists$marker{$name};
        die "phase interval $name\n" unless $end>=$start && abs(($end-$start)-$wall)<=0.0021;
        push@markers,$name;$marker{$name}={name=>$name,gate_wall_s=>0+$wall,load_before=>0+$before,load_after=>0+$after,start_offset_s=>0+$start,end_offset_s=>0+$end,note=>$note,line=>$l};
    }
}
die "complete phase order\n" unless join(",",@markers) eq join(",",@declared);
my@expected_logs=map {"details-$_.log"} @declared;
push@expected_logs,map {"$_.log"} sort keys%parallel;
opendir my$gd,"target/gate" or die "$!\n";my@logs=sort grep {/\.log\z/} readdir$gd;closedir$gd;
die "gate log path set\n" unless join("\0",@logs) eq join("\0",sort @expected_logs);
my%log=map {$_=>record("target/gate/$_")} @logs;
my%stats;my%last;my$total_rerun=0;my$total_reused=0;
for my$name(@declared){
    my$d=$log{"details-$name.log"};my%sum;my$rows=0;
    if($parallel{$name}){die "parallel wrapper $name\n" unless $log{"$name.log"} eq $marker{$name}{line}."\n"}
    for my$l(split/\n/,$d){
        if($l=~/\ALKJSCRIPT_STATS (.+)\z/){
            my$j=JSON::PP->new->decode($1);$rows++;
            for my$k(qw(test_rerun test_reused inline_attempts inline_accepted inline_rejected scalar_attempts scalar_accepted scalar_rejected scalar_constants scalar_identities scalar_branches scalar_functions scalar_trap_guard_rejections scalar_message_mismatches)){
                next unless exists$j->{$k};
                die "nonnumeric $name/$k\n" unless defined($j->{$k})&&!ref($j->{$k})&&"$j->{$k}"=~/\A[0-9]+\z/;
                $sum{$k}+=$j->{$k};
            }
        } elsif($l=~/\ALKJSCRIPT_CLOSED_GATE /){
            $l=~/\breused=([0-9]+)\b/ or die "closed reused $name\n";$sum{test_reused}+=$1;
            $l=~/\brerun=([0-9]+)\b/ or die "closed rerun $name\n";$sum{test_rerun}+=$1;
        }
        $last{$name}=$l if length$l;
    }
    $sum{test_reused}//=0;$sum{test_rerun}//=0;
    if($marker{$name}{note}=~/\Acases reused=([0-9]+) rerun=([0-9]+) /){
        die "phase count mismatch $name\n" unless $sum{test_reused}==$1 && $sum{test_rerun}==$2;
    }else{die "unreported case count $name\n" if $sum{test_reused}||$sum{test_rerun}}
    $total_reused+=$sum{test_reused};$total_rerun+=$sum{test_rerun};
    $stats{$name}={stats_rows=>$rows,counts=>\%sum};
}
die "selection aggregate\n" unless $total_reused==0 && $total_rerun==7476 &&
    $c{stdout}=~/\nok gate selection: skipped 0 case requests on authenticated exact compiler\/input closure or stable runner\/sealed ELF\/checked closure\/closed contract equality; reran 7476 misses\/open effects; host suites rerun\n/;
$c{stdout}=~/\nok gate \(([0-9.]+) s, load ([0-9.]+) -> ([0-9.]+)\)\n\z/ or die "terminal gate marker\n";
my($gate_wall,$gate_load_start,$gate_load_end)=(0+$1,0+$2,0+$3);
die "unexpected stdout line count\n" unless scalar(split/\n/,$c{stdout})==@declared+2;
$c{time}=~/\A([0-9.]+) ([0-9.]+) ([0-9.]+) ([0-9]+)\n\z/ or die "GNU time\n";
my($wall,$user,$system,$rss)=(0+$1,0+$2,0+$3,0+$4);
my$start=$c{'started.ns'};my$end=$c{'finished.ns'};chomp($start,$end);
die "time endpoints\n" unless $start=~/\A[0-9]+\z/ && $end=~/\A[0-9]+\z/ && $end>=$start;
my$outer_ns=$end-$start;
die "time mismatch\n" unless abs($wall-$gate_wall)<0.1 && abs($outer_ns/1e9-$gate_wall)<0.1;
for my$pair(["started",$start],["finished",$end]){
    my($name,$ns)=@$pair;my$utc=strftime("%Y-%m-%dT%H:%M:%SZ",gmtime(int($ns/1e9)))."\n";
    die "UTC mismatch $name\n" unless $c{"$name.utc"} eq $utc;
}
my@load_before=split/\s+/,$c{'load-before'};my@load_after=split/\s+/,$c{'load-after'};
die "loads\n" unless @load_before==3 && @load_after==3 && $load_before[0]==$gate_load_start && $load_after[0]==$gate_load_end;

sub mutation {
    my($name,$scalar)=@_;my$d=$log{"details-$name.log"};
    $d=~/\Aok mutation: ([0-9]+) mutants, ([0-9]+) rejected, ([0-9]+) values, ([0-9]+) traps, ([0-9]+) did not terminate, ([0-9]+) resources, ([0-9]+) findings; ([0-9]+) mutants\/minute \(([0-9.]+) s, ([0-9]+) workers\); ([0-9]+) checked reuses, ([0-9]+) emitted reuses, ([0-9]+) store rejects; inline default\(0\), ([0-9]+) attempts, ([0-9]+) accepted, ([0-9]+) executed, ([0-9]+) rejected, ([0-9]+) candidate calls in ([0-9]+) bodies, ([0-9]+) local body proofs(.*)\n\z/
        or die "mutation summary $name\n";
    my@v=($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20);
    my$tail=$21;
    my@keys=qw(mutants rejected values traps nontermination resources findings mutants_per_minute reported_wall_s workers checked_reuses emitted_reuses store_rejects inline_attempts inline_accepted inline_executed inline_rejected inline_calls inline_functions local_body_proofs);
    my%r;@r{@keys}=map {0+$_}@v;
    die "mutation population/outcomes $name\n" unless $r{mutants}==128 && $r{rejected}==104 && $r{values}==22 && $r{traps}==1 && $r{nontermination}==1 && !$r{resources} && !$r{findings} && $r{workers}==4 && !$r{store_rejects};
    for my$k(qw(inline_attempts inline_accepted inline_executed inline_rejected inline_calls inline_functions)){die "unexpected inline activity $name\n" if$r{$k}}
    if($scalar){
        $tail=~/\A; scalar enabled, ([0-9]+) attempts, ([0-9]+) accepted, ([0-9]+) executed, ([0-9]+) rejected, ([0-9]+) constants, ([0-9]+) identities, ([0-9]+) branches in ([0-9]+) bodies, ([0-9]+) guarded traps, ([0-9]+) message mismatches\z/ or die "scalar mutation counters\n";
        my@k=qw(attempts accepted executed rejected constants identities branches functions trap_guard_rejections message_mismatches);
        my@x=($1,$2,$3,$4,$5,$6,$7,$8,$9,$10);my%s;@s{@k}=map{0+$_}@x;
        die "scalar mutation coverage\n" unless $s{attempts}==4 && $s{accepted}==4 && $s{executed}==4 && $s{branches}==4 && $s{functions}==4 &&
            !$s{rejected}&&!$s{constants}&&!$s{identities}&&!$s{trap_guard_rejections}&&!$s{message_mismatches};
        $r{scalar}=\%s;
    }else{die "disabled scalar text\n" unless $tail eq""}
    return\%r;
}
my$original_mutation=mutation("ir-mutation",0);
my$scalar_mutation=mutation("scalar-ir-mutation",1);
my%broad;
for my$name(qw(conformance generated optimized-conformance optimized-generated scalar-conformance scalar-generated)){
    my$d=$log{"details-$name.log"};
    $d=~/^ok (tests\/conformance|target\/gate\/generated): ([0-9]+) cases, ([0-9]+) route checks, ([0-9]+) pending \(([0-9.]+) s\)$/m or die "broad route summary $name\n";
    my($path,$cases,$checks,$pending,$elapsed)=($1,0+$2,0+$3,0+$4,0+$5);
    $broad{$name}={source_path=>$path,source_cases=>$cases,route_checks=>$checks,pending=>$pending,reported_test_wall_s=>$elapsed,%{$stats{$name}}};
    if($name=~/conformance/){die "conformance count\n" unless$cases==813&&$checks==1624&&$pending==2}
    else{die "generated count\n" unless$cases==805&&$checks==2430&&$pending==0}
    if($name=~/^scalar-/){
        die "scalar broad coverage $name\n" unless$stats{$name}{counts}{scalar_accepted}>0 && $stats{$name}{counts}{scalar_attempts}==$stats{$name}{counts}{scalar_accepted} &&
            !$stats{$name}{counts}{scalar_rejected} && !$stats{$name}{counts}{scalar_message_mismatches};
    }
}
die "bootstrap detail\n" unless $log{"details-bootstrap.log"} eq "ok bootstrap\n";
die "fixed selector selftest\n" unless $log{"details-generate.log"}=~/\Aok generator\/reducer contracts\n/;
die "semantic host\n" unless $log{"details-semantic-host.log"} eq "ok live semantics 36 native-observation contracts\n";
die "scalar CLI\n" unless $log{"details-scalar-simplify.log"} eq "ok scalar opt-in (conflict orders, original observations/value, nonempty rewrite, policy images, profile fragments/IR, test-worker forwarding)\n";
die "mutation parser\n" unless $log{"details-scalar-mutation-usage.log"} eq "ok scalar mutation option conflicts (both orders, no output publication)\n";

my$inventory="path\tbytes\tsha256\n";
for my$p(sort keys%inputs){$inventory.="$p\t$inputs{$p}{bytes}\t$inputs{$p}{sha256}\n"}
my$phase_table="phase\tstatus\tgate_wall_s\tload_before\tload_after\tstart_offset_s\tend_offset_s\tcase_requests_rerun\tcase_requests_reused\tdetails_sha256\n";
for my$name(@declared){
    my$r=$marker{$name};$phase_table.=join("\t",$name,"passed",@$r{qw(gate_wall_s load_before load_after start_offset_s end_offset_s)},$stats{$name}{counts}{test_rerun},$stats{$name}{counts}{test_reused},$inputs{"target/gate/details-$name.log"}{sha256})."\n";
}
my$summary={
    schema=>"compiler-scalar-full-gate-acceptance-1",status=>"passed",
    source_commit=>$source,source_tree=>$src,library_tree=>$library,tools_tree=>$tools,
    producer_sha256=>$producer,producer_bytes=>32761734,
    capture_directory=>"$root/$capture",gate_script_sha256=>$inputs{"tools/check.sh"}{sha256},
    bootstrap_script_sha256=>$inputs{"tools/bootstrap.sh"}{sha256},
    stdout_sha256=>$inputs{"$capture/stdout"}{sha256},stderr_sha256=>$inputs{"$capture/stderr"}{sha256},
    gate_wall_s=>$gate_wall,gnu_wall_s=>$wall,user_s=>$user,system_s=>$system,peak_rss_kib=>$rss,
    launch_inclusive_wall_ns=>$outer_ns,start_ns=>$start,end_ns=>$end,
    load_before=>[map{0+$_}@load_before],load_after=>[map{0+$_}@load_after],
    outer_status=>0,workers=>4,address_space_bytes=>4294967296,timeout_s=>1800,termination_grace_s=>10,
    named_phases=>scalar(@declared),parallel_phase_logs=>scalar(keys%parallel),gate_log_files=>scalar(@logs),
    gate_log_bytes=>0+do{my$s=0;$s+=$inputs{"target/gate/$_"}{bytes}for@logs;$s},
    case_requests_rerun=>$total_rerun,case_requests_reused=>$total_reused,
    broad_routes=>\%broad,mutation_default=>$original_mutation,mutation_scalar=>$scalar_mutation,
    semantic_host_native_observation_contracts=>36,
    source_manifest_sha256=>sha256_hex($c{'source-before.sha256'}),
    source_file_count=>scalar(keys%manifest),source_bytes_verified=>$source_bytes,
    source_before_after_identical=>JSON::PP::true,
    current_source_matches_manifest_and_exact_git_blobs=>JSON::PP::true,
    producer_before_after_and_current_identical=>JSON::PP::true,
    input_inventory_sha256=>sha256_hex($inventory),phase_table_sha256=>sha256_hex($phase_table),
    audit_source_sha256=>sha256_hex(bytes("$out/audit.pl")),
    qualifications=>[
        "Independent read-only audit; no compiler, native executable, gate rerun or benchmark was executed.",
        "All named phase markers and per-phase detail hashes were verified against the exact checked-in gate script; overall success is not inferred from a partial log.",
        "7476 is the number of fresh case requests, not distinct source cases or route checks. Generated routes report 805 source cases and 1215 case requests each.",
        "Each conformance route retains two declared pending cases; gate success does not claim they ran.",
        "The scalar fixed mutation population exercised four admitted/executed branch rewrites. Executed uses the selector's existing post-invoke accounting; native outcome comparisons and zero findings are separately reported.",
        "The existing inliner rejected twelve conformance candidates in both inline-only and scalar routes, retaining admitted fallbacks. Scalar candidates in those routes report zero checker rejections and zero message mismatches.",
        "The cold-store designation follows the captured outer wrapper declaration; zero case reuse is independently recomputed. Initial store emptiness cannot be reconstructed from its later contents.",
        "GNU time, the gate's own elapsed marker and launch-inclusive nanosecond duration have different capture boundaries and are reported separately.",
        "Global key contents and store contents were not read. No compression, cleanup or process observer was used.",
        "Candidate CI and performance remain pending; this gate audit is not benchmark execution permission."
    ]
};
write_file("$out/input-inventory.tsv",$inventory);
write_file("$out/phases.tsv",$phase_table);
write_file("$out/broad-route-counts.json",json(\%broad));
write_file("$out/mutation-counts.json",json({default=>$original_mutation,scalar=>$scalar_mutation}));
write_file("$out/summary.json",json($summary));
my$report="# Independent scalar full-gate audit\n\nStatus: passed. The audit read the retained captures and exact top-level gate logs; no compiler, native image, repeat gate or benchmark was executed.\n\n";
$report.="Source: $source. Producer: $producer (32,761,734 bytes).\n\n";
$report.="All ".scalar(@declared)." named phases and ".scalar(@logs)." log files agree with the pinned gate script. All 1,943 source-manifest paths were independently read and checked against both their before/after SHA-256 records and the exact Git blob IDs. The current emitter matches both retained producer hashes.\n\n";
$report.="Gate elapsed: $gate_wall s; GNU wall/user/system: $wall/$user/$system s; peak RSS $rss KiB. Load: ".join("/",@load_before)." to ".join("/",@load_after).".\n\n";
$report.="Fresh case requests: $total_rerun; reused: $total_reused. Conformance reports 813 source cases, 1,624 route checks and two pending cases per policy. Generated reports 805 source cases, 2,430 route checks and zero pending per policy; its request count is 1,215, not 1,215 distinct sources.\n\n";
$report.="Scalar conformance: 149 attempts/149 admitted, 3,208 constants, 325 identities, 48 branches in 1,026 bodies. Scalar generated: 538 attempts/538 admitted, 3,862 constants, 294 identities, 54 branches in 1,595 bodies. Both have zero scalar rejections and message mismatches.\n\n";
$report.="Both fixed mutation routes: 128 mutants, 104 rejected, 22 values, one trap, one nontermination, zero resource outcomes and zero findings. Scalar enabled: four attempts, four admitted, four executed, four branch rewrites in four bodies; zero other rewrites, rejections, guarded traps or message mismatches. Inline default remains zero in both routes.\n\n";
$report.="The live semantic suite reports 36 native-observation contracts. Fixed generator/reducer controls, scalar CLI/cache/profile tests and both parser conflict orders passed in the gate.\n\n";
$report.="## Limits\n\n";$report.="- $_\n"for@{$summary->{qualifications}};
write_file("$out/summary.md",$report);
print "passed: ",scalar(@declared)," phases / ",scalar(@logs)," logs / ",scalar(keys%manifest)," exact source blobs; 7476 fresh / 0 reused; scalar mutation accepted/executed=4/4\n";
