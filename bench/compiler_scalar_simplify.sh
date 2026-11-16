#!/usr/bin/env bash
# One fixed scalar-policy hypothesis. This file does not release its own run.
set -euo pipefail
export LC_ALL=C
cd "$(dirname "$0")/.."
trial_root=$PWD
harness=$(realpath -e "$0")
revision=26bf2c3c021dd79810a81fa1d9054d59f3768b06
expected_output_sha=8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132
expected_output_bytes=16228028
expected_source_tree=3431671024d0a550b3130f2401bde067026d9bd3
expected_library_tree=e442fb3920db7eda2377f92fde0d92761f0562be
expected_runtime_sha=eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14
pin=
results=
while (($#)); do
    (($# >= 2)) || { printf 'E_SCALAR_ARGUMENT missing value\n' >&2; exit 2; }
    case "$1" in
        --pin) pin=$2 ;;
        --results) results=$2 ;;
        *) printf 'usage: compiler_scalar_simplify.sh --pin PIN.json --results target/compiler-scalar-simplify.NAME\n' >&2; exit 2 ;;
    esac
    shift 2
done
[[ -n $pin && -n $results ]] || { printf 'E_SCALAR_ARGUMENT pin and results are required\n' >&2; exit 2; }
pin=$(realpath -e "$pin")
results=$(realpath -m "$results")
[[ $(dirname "$results") == "$trial_root/target" &&
   $(basename "$results") =~ ^compiler-scalar-simplify\.[A-Za-z0-9_.-]+$ &&
   ! -e $results && ! -L $results ]] ||
    { printf 'E_SCALAR_RESULTS require a fresh owned target directory\n' >&2; exit 2; }
umask 077
mkdir "$results"
raw="$results/raw"
fixture="$results/fixture"
mkdir "$raw"
phase=prepare
completed=false
load() { cut -d ' ' -f 1-3 /proc/loadavg; }
finish() {
    local status=$?
    trap - EXIT HUP INT TERM
    printf 'status=%s\ncompleted=%s\nlast_phase=%s\nfixture=%s\n' \
        "$status" "$completed" "$phase" "$fixture" > "$results/outcome"
    date -u +%Y-%m-%dT%H:%M:%SZ > "$results/finished.utc"
    load > "$results/finished.load"
    printf 'retained_results=%s\nstatus=%s\ncompleted=%s\n' "$results" "$status" "$completed"
    exit "$status"
}
trap finish EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
date -u +%Y-%m-%dT%H:%M:%SZ > "$results/started.utc"
load > "$results/started.load"
printf '%s\n' "$PWD" > "$results/cwd"
cp -- "$pin" "$results/producer-pin.json"
sha256sum "$results/producer-pin.json" > "$results/producer-pin.sha256"

# The later root-reviewed addendum supplies exact bytes/source and already passed
# candidate validation. An old producer's CI is never substituted for this gate.
perl - "$results/producer-pin.json" "$harness" "$results" <<'PERL'
use strict;
use warnings;
use JSON::PP ();
use Digest::SHA qw(sha256_hex);
use File::Copy qw(copy);
sub bytes { open my $f,'<:raw',$_[0] or die "$_[0]: $!\n"; local $/; my $s=<$f>; close $f or die "$!\n"; return $s; }
sub string {
    my ($s,$name)=@_;
    die "invalid $name\n" unless defined($s) && !ref($s) && length($s) && $s!~/[\r\n\0]/;
    return $s;
}
sub digest { my ($s,$name)=@_; string($s,$name); die "invalid $name\n" unless $s=~/\A[0-9a-f]{64}\z/; return $s; }
my ($pin,$harness,$out)=@ARGV;
my $p=JSON::PP->new->decode(bytes($pin));
die "pin schema\n" unless $p->{schema} eq 'lkjscript-scalar-six-run-1';
die "harness identity\n" unless sha256_hex(bytes($harness)) eq digest($p->{harness_sha256},'harness');
my $protocol=string($p->{protocol_path},'protocol_path');
die "protocol identity\n" unless sha256_hex(bytes($protocol)) eq digest($p->{protocol_sha256},'protocol');
copy($protocol,"$out/protocol.md") or die "copy protocol: $!\n";
my $e=$p->{emitter};
string($e->{path},'emitter.path');
digest($e->{sha256},'emitter.sha256');
die "emitter bytes\n" unless "$e->{bytes}"=~/\A[1-9][0-9]*\z/;
die "emitter commit\n" unless $e->{source_commit}=~/\A[0-9a-f]{40}\z/;
for my $tree (qw(source_tree library_tree)) {
    die "emitter tree\n" unless $e->{$tree}=~/\A[0-9a-f]{40}\z/;
}
for my $name (qw(focused full_gate)) {
    my $v=$p->{validation}{$name};
    die "$name has not passed\n" unless $v->{status} eq 'passed';
    die "$name source/producer mismatch\n"
        unless $v->{source_commit} eq $e->{source_commit} && $v->{producer_sha256} eq $e->{sha256};
    my $path=string($v->{evidence_path},"$name evidence");
    die "$name evidence identity\n" unless sha256_hex(bytes($path)) eq digest($v->{evidence_sha256},"$name evidence");
    copy($path,"$out/$name.evidence") or die "copy $name: $!\n";
}
die "CI attribution\n" unless $p->{candidate_ci} eq 'pending-until-if-main-adoption';
die "predeclared order\n" unless join(',',@{$p->{execution_order}}) eq 'A1,B1,B2,A2,A3,B3';
open my $args,'>:raw',"$out/pin-values" or die "$!\n";
print {$args} join("\n",@$e{qw(path sha256 bytes source_commit source_tree library_tree)}),"\n";
close $args or die "$!\n";
PERL
mapfile -t values < "$results/pin-values"
[[ ${#values[@]} == 6 ]] || { printf 'E_SCALAR_PIN malformed decoded values\n' >&2; exit 1; }
compiler=$(realpath -e "${values[0]}")
compiler_sha=${values[1]}
compiler_bytes=${values[2]}
compiler_revision=${values[3]}
[[ -x $compiler && -f $compiler ]] || { printf 'E_SCALAR_PIN compiler is not executable\n' >&2; exit 1; }
[[ $(git rev-parse --verify "$compiler_revision^{commit}") == "$compiler_revision" &&
   $(git rev-parse "$compiler_revision:src") == "${values[4]}" &&
   $(git rev-parse "$compiler_revision:library") == "${values[5]}" ]] ||
    { printf 'E_SCALAR_SOURCE emitter source pin differs\n' >&2; exit 1; }
[[ $(git rev-parse --verify "$revision^{commit}") == "$revision" &&
   $(git rev-parse "$revision:src") == "$expected_source_tree" &&
   $(git rev-parse "$revision:library") == "$expected_library_tree" ]] ||
    { printf 'E_SCALAR_SOURCE frozen source tree differs\n' >&2; exit 1; }
free_kib=$(df -Pk "$trial_root/target" | awk 'NR==2 {print $4}')
printf '%s\n' "$free_kib" > "$results/free-space-kib"
[[ $free_kib =~ ^[0-9]+$ ]] && ((free_kib >= 3 * 1024 * 1024)) ||
    { printf 'E_SCALAR_SPACE need 3 GiB free\n' >&2; exit 1; }
mkdir "$fixture" "$fixture/source"
cp --reflink=auto -- "$compiler" "$fixture/emitter"
chmod 500 "$fixture/emitter"
[[ $(sha256sum "$fixture/emitter" | cut -d ' ' -f 1) == "$compiler_sha" &&
   $(stat -c %s "$fixture/emitter") == "$compiler_bytes" ]] ||
    { printf 'E_SCALAR_PRODUCER copied emitter differs\n' >&2; exit 1; }
compiler="$fixture/emitter"
git archive "$revision" src library seed/lkjscript.gz > "$fixture/source.tar"
tar -xf "$fixture/source.tar" -C "$fixture/source"
gzip -dc "$fixture/source/seed/lkjscript.gz" > "$fixture/expected"
[[ $(sha256sum "$fixture/expected" | cut -d ' ' -f 1) == "$expected_output_sha" &&
   $(stat -c %s "$fixture/expected") == "$expected_output_bytes" &&
   $(sha256sum "$fixture/source/library/runtime.lkj" | cut -d ' ' -f 1) == "$expected_runtime_sha" ]] ||
    { printf 'E_SCALAR_EXPECTED frozen output/runtime differs\n' >&2; exit 1; }
source_manifest() {
    (cd "$fixture/source"; rg --files --hidden -0 src library | sort -z | xargs -0 sha256sum)
}
source_manifest > "$results/source-before.sha256"
sha256sum "$fixture/source.tar" "$fixture/source/seed/lkjscript.gz" "$fixture/expected" "$compiler" > "$results/input-artifacts.sha256"
{
    printf 'frozen_revision=%s\nsource_tree=%s\nlibrary_tree=%s\n' "$revision" "$expected_source_tree" "$expected_library_tree"
    printf 'emitter_sha256=%s\nemitter_bytes=%s\nemitter_source_commit=%s\n' "$compiler_sha" "$compiler_bytes" "$compiler_revision"
    printf 'expected_output_sha256=%s\nexpected_output_bytes=%s\nruntime_sha256=%s\n' "$expected_output_sha" "$expected_output_bytes" "$expected_runtime_sha"
    printf 'source_root=%s\nsource_argument=src/main.lkj\nlibrary_selector=physical\nruntime_selector=physical\n' "$fixture/source"
    printf 'construction_order=A,B\nA=inline64\nB=inline64+scalar-simplify\nexecution_order=A1,B1,B2,A2,A3,B3\n'
    printf 'execution_output_inline_limit=0\nexecution_scalar_flag=absent\nstore=disabled\nprofile=disabled\nwarmups=0\nretries=0\nfile_cache_reset=false\n'
    printf 'workers=1\naddress_space_bytes=4294967296\nwatchdog_seconds=900\ntermination_grace_seconds=10\nrequired_free_space_bytes=3221225472\n'
    printf 'primary_wall=gnu_time_elapsed_seconds\nprimary_threshold=10*sum_B<=9*sum_A\nsecondary=median_wall\n'
    printf 'capture_ns=before_start_checkpoint_write_to_after_command_exit\ncandidate_ci=pending-until-if-main-adoption\ncleanup_mode=none\n'
} > "$results/metadata"
printf 'phase\troute\trepeat\tstatus\twall_s\tuser_s\tkernel_s\trss_kib\timage_bytes\timage_sha256\tload_before\tload_after\n' > "$results/rows.tsv"
printf 'row\timage_sha256\timage_bytes\tseed_bytes_equal\tstdout_empty\tstderr_empty\n' > "$results/parity.tsv"
measure() {
    local name=$1 route=$2 repetition=$3 artifact=$4 status=0 before after start end bytes=- digest=-
    shift 4
    phase="$name.$route.$repetition"
    printf '%s\n' "$PWD" > "$raw/$phase.cwd"
    printf '%s\0' "$@" > "$raw/$phase.argv"
    local command=(/usr/bin/time -f '%e\t%U\t%S\t%M' -o "$raw/$phase.time"
        prlimit --as=4294967296 -- timeout --signal=TERM --kill-after=10s 900s "$@")
    printf '%s\0' "${command[@]}" > "$raw/$phase.wrapper-argv"
    date -u +%Y-%m-%dT%H:%M:%SZ > "$raw/$phase.started.utc"
    before=$(load)
    printf '%s\n' "$before" > "$raw/$phase.load-before"
    start=$(date +%s%N)
    printf '%s\n' "$start" > "$raw/$phase.started.ns"
    printf '%s\n' "$phase" > "$results/checkpoint"
    "${command[@]}" > "$raw/$phase.stdout" 2> "$raw/$phase.stderr" || status=$?
    end=$(date +%s%N)
    after=$(load)
    printf '%s\n' "$end" > "$raw/$phase.finished.ns"
    date -u +%Y-%m-%dT%H:%M:%SZ > "$raw/$phase.finished.utc"
    printf '%s\n' "$after" > "$raw/$phase.load-after"
    printf '%s\n' "$status" > "$raw/$phase.status"
    printf '%s\t%s\t%s\t%s\t%s\n' "$start" "$end" "$before" "$after" "$status" > "$raw/$phase.capture"
    if [[ -f $artifact ]]; then bytes=$(stat -c %s "$artifact"); digest=$(sha256sum "$artifact" | cut -d ' ' -f 1); fi
    local timing='-\t-\t-\t-'
    if [[ -f $raw/$phase.time ]]; then timing=$(tail -n 1 "$raw/$phase.time"); fi
    printf '%s\t%s\t%s\t%s\t%b\t%s\t%s\t%s\t%s\n' "$name" "$route" "$repetition" "$status" "$timing" "$bytes" "$digest" "$before" "$after" >> "$results/rows.tsv"
    ((status == 0)) || return "$status"
    local elapsed user kernel peak extra
    IFS=$'\t' read -r elapsed user kernel peak extra <<< "$timing"
    [[ -z ${extra:-} && $elapsed =~ ^[0-9]+(\.[0-9]{1,9})?$ &&
       $user =~ ^[0-9]+(\.[0-9]{1,9})?$ && $kernel =~ ^[0-9]+(\.[0-9]{1,9})?$ &&
       $peak =~ ^[0-9]+$ && $start =~ ^[0-9]+$ && $end =~ ^[0-9]+$ ]] &&
       ((end >= start)) ||
        { printf 'E_SCALAR_CAPTURE malformed/nonmonotonic capture for %s\n' "$phase" >&2; return 1; }
}
for route in A B; do
    options=()
    if [[ $route == B ]]; then options=(--scalar-simplify); fi
    measure construct "$route" 1 "$fixture/producer-$route" "$compiler" build src/main.lkj \
        --source-dir "$fixture/source" --library-dir "$fixture/source/library" \
        --runtime-source "$fixture/source/library/runtime.lkj" --no-store --inline-limit 64 \
        "${options[@]}" --stats -o "$fixture/producer-$route"
done
phase=construction-controls
perl - "$raw/construct.A.1.stderr" "$raw/construct.B.1.stderr" > "$results/construction-controls.stdout" 2> "$results/construction-controls.stderr" <<'PERL'
use strict;
use warnings;
use JSON::PP ();
my @keys=qw(scalar_attempts scalar_accepted scalar_rejected scalar_constants scalar_identities
    scalar_branches scalar_functions scalar_trap_guard_rejections scalar_message_mismatches);
for my $i (0..1) {
    open my $f,'<:raw',$ARGV[$i] or die "$!\n"; local $/; my $t=<$f>; close $f or die "$!\n";
    my @rows=$t=~/^LKJSCRIPT_STATS (.+)$/mg; die "construction stats count\n" unless @rows==1;
    my $s=JSON::PP->new->decode($rows[0]);
    for my $k (@keys,qw(inline_accepted inline_rejected)) {
        die "missing construction count $k\n" unless exists($s->{$k}) && "$s->{$k}"=~/\A[0-9]+\z/;
    }
    die "inlining not admitted\n" unless $s->{inline_accepted}>0 && $s->{inline_rejected}==0;
    if (!$i) { for my $k (@keys) { die "A ran scalar stage\n" unless $s->{$k}==0; } }
    else {
        die "B admitted no nonempty scalar rewrite\n" unless $s->{scalar_attempts}>0
            && $s->{scalar_accepted}>0 && $s->{scalar_functions}>0
            && $s->{scalar_constants}+$s->{scalar_identities}+$s->{scalar_branches}>0;
        die "B scalar candidate rejected or message inventory changed\n"
            unless $s->{scalar_rejected}==0 && $s->{scalar_message_mismatches}==0;
    }
}
print "both constructions admitted; B has a nonempty scalar selection\n";
PERL
[[ ! -s $raw/construct.A.1.stdout && ! -s $raw/construct.B.1.stdout ]] ||
    { printf 'E_SCALAR_CONSTRUCTION unexpected construction stdout\n' >&2; exit 1; }
sha256sum "$fixture/producer-A" "$fixture/producer-B" > "$results/constructed-producers.sha256"
if cmp -s "$fixture/producer-A" "$fixture/producer-B"; then
    printf 'E_SCALAR_UNCHANGED producer image bytes are identical\n' >&2; exit 1
fi
for label in A1 B1 B2 A2 A3 B3; do
    route=${label:0:1}
    repetition=${label:1:1}
    artifact="$fixture/output-$label"
    measure execute "$route" "$repetition" "$artifact" "$fixture/producer-$route" build src/main.lkj \
        --source-dir "$fixture/source" --library-dir "$fixture/source/library" \
        --runtime-source "$fixture/source/library/runtime.lkj" --no-store --inline-limit 0 -o "$artifact"
    phase="parity-$label"
    equal=no stdout_empty=no stderr_empty=no
    artifact_sha=$(sha256sum "$artifact" | cut -d ' ' -f 1)
    artifact_bytes=$(stat -c %s "$artifact")
    if [[ $artifact_sha == "$expected_output_sha" && $artifact_bytes == "$expected_output_bytes" ]] &&
        cmp -s "$fixture/expected" "$artifact"; then equal=yes; fi
    if [[ ! -s $raw/execute.$route.$repetition.stdout ]]; then stdout_empty=yes; fi
    if [[ ! -s $raw/execute.$route.$repetition.stderr ]]; then stderr_empty=yes; fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$label" "$artifact_sha" \
        "$artifact_bytes" "$equal" "$stdout_empty" "$stderr_empty" >> "$results/parity.tsv"
    [[ $equal == yes && $stdout_empty == yes && $stderr_empty == yes ]] ||
        { printf 'E_SCALAR_PARITY %s changed the fixed output or diagnostics\n' "$label" >&2; exit 1; }
done
phase=final-controls
source_manifest > "$results/source-after.sha256"
cmp "$results/source-before.sha256" "$results/source-after.sha256"
sha256sum -c "$results/input-artifacts.sha256" "$results/constructed-producers.sha256" > "$results/final-input-check.stdout" 2> "$results/final-input-check.stderr"
[[ $(sha256sum "$compiler" | cut -d ' ' -f 1) == "$compiler_sha" ]] ||
    { printf 'E_SCALAR_PRODUCER emitter changed during the campaign\n' >&2; exit 1; }
sha256sum "$compiler" "$fixture/producer-A" "$fixture/producer-B" "$fixture"/output-* > "$results/final-artifacts.sha256"
perl - "$results/rows.tsv" "$results/summary.json" <<'PERL'
use strict;
use warnings;
use JSON::PP ();
sub ns {
    my ($text)=@_; die "invalid GNU elapsed value $text\n" unless $text=~/\A([0-9]+)(?:\.([0-9]{1,9}))?\z/;
    my ($whole,$fraction)=($1,$2//''); return $whole*1_000_000_000+($fraction.('0'x(9-length($fraction))));
}
my ($path,$output)=@ARGV;
open my $f,'<:raw',$path or die "$!\n"; my $head=<$f>; my @rows=map { chomp; [split /\t/,$_,-1] } <$f>; close $f or die "$!\n";
die "expected two constructions and six executions\n" unless @rows==8;
my @wanted=('construct:A:1','construct:B:1','execute:A:1','execute:B:1','execute:B:2','execute:A:2','execute:A:3','execute:B:3');
my (%wall,%cpu,%peak);
for my $i (0..$#rows) {
    my $r=$rows[$i]; die "bad measured row\n" unless @$r==12 && join(':',@$r[0..2]) eq $wanted[$i] && $r->[3] eq '0';
    next if $i<2;
    push @{$wall{$r->[1]}},ns($r->[4]);
    push @{$cpu{$r->[1]}},ns($r->[5])+ns($r->[6]);
    $peak{$r->[1]}=$r->[7] if !defined($peak{$r->[1]}) || $r->[7]>$peak{$r->[1]};
}
my (%sum,%median,%cpu_sum);
for my $r (qw(A B)) {
    $sum{$r}=0; $sum{$r}+=$_ for @{$wall{$r}};
    $cpu_sum{$r}=0; $cpu_sum{$r}+=$_ for @{$cpu{$r}};
    my @sorted=sort {$a<=>$b} @{$wall{$r}}; $median{$r}=$sorted[1];
}
die "zero elapsed baseline\n" unless $sum{A}>0;
my $qualified=10*$sum{B}<=9*$sum{A};
my $summary={
    primary_metric=>'arithmetic mean GNU-time wall, all three executions per policy',
    runtime_sum_ns=>\%sum, runtime_mean_ns=>{map {$_=>$sum{$_}/3} qw(A B)},
    runtime_median_ns=>\%median, runtime_cpu_sum_ns=>\%cpu_sum, peak_runtime_rss_kib=>\%peak,
    mean_B_over_A=>$sum{B}/$sum{A}, mean_wall_reduction_percent=>100*(1-$sum{B}/$sum{A}),
    threshold=>'10*sum_B <= 9*sum_A', qualifies=>$qualified?JSON::PP::true:JSON::PP::false,
    decision=>$qualified?'threshold-cleared-awaiting-separate-review':'discard-scalar-below-threshold',
    policy_change=>'none', candidate_ci=>'pending-until-if-main-adoption',
    fixed_order=>[qw(A1 B1 B2 A2 A3 B3)], timing_repetitions_per_policy=>3,
    timing_claim_limit=>'one fixed-order six-run cohort; no randomized effect or backend floor',
};
open my $out,'>:raw',$output or die "$!\n"; print {$out} JSON::PP->new->canonical->pretty->encode($summary); close $out or die "$!\n";
PERL
phase=complete
completed=true
printf 'ok fixed scalar-policy cohort; six outputs match the frozen seed; no policy change\n'
