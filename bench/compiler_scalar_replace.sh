#!/usr/bin/env bash
# One fixed computation, one scalar-replacement hypothesis, no profiling.
set -euo pipefail
cd "$(dirname "$0")/.."
trial_root=$PWD
revision=26bf2c3c021dd79810a81fa1d9054d59f3768b06
expected_output_sha=8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132
expected_source_tree=3431671024d0a550b3130f2401bde067026d9bd3
expected_library_tree=e442fb3920db7eda2377f92fde0d92761f0562be
expected_runtime_sha=eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14
compiler=target/lkjscript
expected_compiler_sha=
emitter_revision=unrecorded
results=target/compiler-scalar-replace.tsv
memory_mib=4096
deadline=900
while (($#)); do
    [[ $# -ge 2 ]] || exit 2
    case $1 in
        --compiler) compiler=$2 ;;
        --expect-compiler-sha) expected_compiler_sha=$2 ;;
        --emitter-revision) emitter_revision=$2 ;;
        --results) results=$2 ;;
        --memory-mib) memory_mib=$2 ;;
        --deadline) deadline=$2 ;;
        *) echo 'usage: compiler_scalar_replace.sh --compiler FILE --expect-compiler-sha SHA [--emitter-revision COMMIT] [--results TSV] [--memory-mib N] [--deadline S]' >&2; exit 2 ;;
    esac
    shift 2
done
[[ $expected_compiler_sha =~ ^[0-9a-f]{64}$ && $memory_mib =~ ^[1-9][0-9]*$ && $deadline =~ ^[1-9][0-9]*$ ]] || exit 2
revision=$(git rev-parse --verify "$revision^{commit}")
[[ $(git rev-parse "$revision:src") == "$expected_source_tree" && $(git rev-parse "$revision:library") == "$expected_library_tree" ]] || { echo 'E_SRA_SOURCE frozen trees differ' >&2; exit 1; }
compiler=$(realpath -e "$compiler")
results=$(realpath -m "$results")
[[ -x $compiler && ! -e $results && ! -e $results.meta && ! -e $results.raw ]] || exit 2
mkdir -p target "$(dirname "$results")"
free_kib=$(df -Pk target | awk 'NR==2 {print $4}')
((free_kib >= 3 * 1024 * 1024)) || { echo 'E_SRA_SPACE need 3 GiB free' >&2; exit 1; }
umask 077
run=$(mktemp -d "$trial_root/target/compiler-sra.XXXXXX")
raw="$results.raw"
mkdir "$raw"
current_phase=prepare
completed=false
finish() {
    local status=$?
    trap - EXIT HUP INT TERM
    printf 'status=%s\ncompleted=%s\nlast_phase=%s\n' "$status" "$completed" "$current_phase" > "$results.outcome"
    rm -rf -- "$run"
    exit "$status"
}
trap finish EXIT
trap 'exit 130' HUP INT TERM
cp --reflink=auto -- "$compiler" "$run/emitter"
chmod 500 "$run/emitter"
compiler_sha=$(sha256sum "$run/emitter" | cut -d ' ' -f 1)
[[ $compiler_sha == "$expected_compiler_sha" ]] || { echo 'E_SRA_PRODUCER copied emitter digest differs' >&2; exit 1; }
compiler="$run/emitter"
mkdir -p "$run/source/target"
git archive "$revision" src library witness seed/lkjscript.gz tools/optimizer-test.sh tools/generators tests/conformance/ir tests/fixtures/world/self_spawn.lkj | tar -x -C "$run/source"
gzip -dc "$run/source/seed/lkjscript.gz" > "$run/expected"
[[ $(sha256sum "$run/expected" | cut -d ' ' -f 1) == "$expected_output_sha" && $(stat -c %s "$run/expected") == 16228028 ]] || { echo 'E_SRA_EXPECTED frozen seed identity differs' >&2; exit 1; }
[[ $(sha256sum "$run/source/library/runtime.lkj" | cut -d ' ' -f 1) == "$expected_runtime_sha" ]] || { echo 'E_SRA_RUNTIME frozen runtime differs' >&2; exit 1; }
oracle_script="$run/source/tools/optimizer-test.sh"
(cd "$run/source"; rg --files -0 src library | sort -z | xargs -0 sha256sum) > "$results.inputs.sha256"
{
    printf 'revision=%s\nsource_tree=%s\nlibrary_tree=%s\nwitness_tree=%s\n' "$revision" "$(git rev-parse "$revision:src")" "$(git rev-parse "$revision:library")" "$(git rev-parse "$revision:witness")"
    printf 'emitter_sha256=%s\nemitter_bytes=%s\nemitter_declared_revision=%s\n' "$compiler_sha" "$(stat -c %s "$compiler")" "$emitter_revision"
    printf 'expected_output_sha256=%s\nexpected_output_bytes=16228028\n' "$expected_output_sha"
    printf 'runtime_source_sha256=%s\noracle_script_sha256=%s\nharness_sha256=%s\n' "$(sha256sum "$run/source/library/runtime.lkj" | cut -d ' ' -f 1)" "$(sha256sum "$oracle_script" | cut -d ' ' -f 1)" "$(sha256sum "$0" | cut -d ' ' -f 1)"
    printf 'workers=1\nmemory_mib=%s\ndeadline_s=%s\ntermination_grace_s=10\nfree_space_kib_before=%s\n' "$memory_mib" "$deadline" "$free_kib"
    printf 'construction_inline_limit=64\nconstruction_scalar_replace=0,1\nexecution_order=0,1,1,0,0,1,rust-O\nexecution_output_inline_limit=0\nproducer_build_and_execution_store=disabled\n'
    printf 'primary_wall=gnu_time_elapsed_seconds\nacceptance=mean_wall_reduction_at_least_10_percent\nfile_cache_reset=false\nretries=none\nsampling=none\n'
    printf 'construction_stats_include_original_and_candidate_admission=true\n'
    rustc --version
} > "$results.meta"
printf 'phase\troute\trepeat\tstatus\twall_s\tuser_s\tkernel_s\trss_kib\timage_bytes\timage_sha256\tload_before\tload_after\n' > "$results"
printf 'route\trepeat\tsha256\tbytes\tseed_byte_equal\tstdout_empty\tstderr_empty\n' > "$results.parity.tsv"
load() { cut -d ' ' -f 1-3 /proc/loadavg; }
measure() {
    local phase=$1 route=$2 repetition=$3 artifact=$4 before after status=0 bytes=- digest=- start end
    shift 4
    current_phase="$phase/$route/$repetition"
    before=$(load)
    printf 'phase=%s\nload_before=%s\n' "$current_phase" "$before" > "$raw/checkpoint"
    printf '%s\n' "$@" > "$raw/$phase.$route.$repetition.arguments"
    start=$(date +%s%N)
    printf '%s\t%s\n' "$start" "$before" > "$raw/$phase.$route.$repetition.start"
    /usr/bin/time -f '%e\t%U\t%S\t%M' -o "$raw/$phase.$route.$repetition.time" \
        prlimit --as=$((memory_mib * 1024 * 1024)) -- timeout --kill-after=10s "${deadline}s" "$@" \
        > "$raw/$phase.$route.$repetition.stdout" 2> "$raw/$phase.$route.$repetition.stderr" || status=$?
    end=$(date +%s%N)
    after=$(load)
    printf '%s\t%s\t%s\t%s\t%s\n' "$start" "$end" "$before" "$after" "$status" > "$raw/$phase.$route.$repetition.capture"
    if [[ -f $artifact ]]; then bytes=$(stat -c %s "$artifact"); digest=$(sha256sum "$artifact" | cut -d ' ' -f 1); fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$phase" "$route" "$repetition" "$status" "$(tail -n 1 "$raw/$phase.$route.$repetition.time")" "$bytes" "$digest" "$before" "$after" | tee -a "$results"
    if ((status)); then cat "$raw/$phase.$route.$repetition.stderr" >&2; return "$status"; fi
}

for policy in 0 1; do
    optimize=()
    if ((policy == 1)); then optimize=(--scalar-replace); fi
    measure construct "sra$policy" 1 "$run/producer-$policy" "$compiler" build src/main.lkj --source-dir "$run/source" --library-dir "$run/source/library" --runtime-source "$run/source/library/runtime.lkj" --no-store --inline-limit 64 "${optimize[@]}" --stats -o "$run/producer-$policy"
done
for policy in 0 1; do
    grep -Eq '"inline_accepted":[1-9][0-9]*' "$raw/construct.sra$policy.1.stderr"
    grep -q '"inline_rejected":0' "$raw/construct.sra$policy.1.stderr"
done
grep -q '"scalar_attempts":0' "$raw/construct.sra0.1.stderr"
grep -Eq '"scalar_accepted":[1-9][0-9]*' "$raw/construct.sra1.1.stderr"
grep -q '"scalar_rejected":0' "$raw/construct.sra1.1.stderr"
if cmp -s "$run/producer-0" "$run/producer-1"; then echo 'E_SRA_UNCHANGED no producer bytes changed' >&2; exit 1; fi
measure construct rust-O 1 "$run/cargo/release/lkjscript" env -u RUSTC_BOOTSTRAP CARGO_BUILD_JOBS=1 CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_OPT_LEVEL=3 CARGO_TARGET_DIR="$run/cargo" RUSTFLAGS= cargo build --manifest-path "$run/source/witness/Cargo.toml" --release --quiet --locked --offline --bin lkjscript
cp "$run/cargo/release/lkjscript" "$run/rust-O"
rm -rf -- "$run/cargo"
repeat0=0
repeat1=0
for policy in 0 1 1 0 0 1; do
    if ((policy == 0)); then repeat0=$((repeat0 + 1)); repetition=$repeat0; else repeat1=$((repeat1 + 1)); repetition=$repeat1; fi
    measure execute "sra$policy" "$repetition" "$run/output" "$run/producer-$policy" build src/main.lkj --source-dir "$run/source" --library-dir "$run/source/library" --runtime-source "$run/source/library/runtime.lkj" --no-store --inline-limit 0 -o "$run/output"
    cmp "$run/expected" "$run/output"
    [[ ! -s $raw/execute.sra$policy.$repetition.stdout && ! -s $raw/execute.sra$policy.$repetition.stderr ]] || { echo 'E_SRA_DIAGNOSTICS unexpected execution diagnostics' >&2; exit 1; }
    printf 'sra%s\t%s\t%s\t%s\tyes\tyes\tyes\n' "$policy" "$repetition" "$(sha256sum "$run/output" | cut -d ' ' -f 1)" "$(stat -c %s "$run/output")" >> "$results.parity.tsv"
    rm -- "$run/output"
done
measure execute rust-O 1 "$run/output" "$run/rust-O" build src/main.lkj --source-dir "$run/source" --library-dir "$run/source/library" --runtime-source "$run/source/library/runtime.lkj" --no-store --inline-limit 0 -o "$run/output"
cmp "$run/expected" "$run/output"
[[ ! -s $raw/execute.rust-O.1.stdout && ! -s $raw/execute.rust-O.1.stderr ]] || { echo 'E_SRA_DIAGNOSTICS unexpected Rust execution diagnostics' >&2; exit 1; }
printf 'rust-O\t1\t%s\t%s\tyes\tyes\tyes\n' "$(sha256sum "$run/output" | cut -d ' ' -f 1)" "$(stat -c %s "$run/output")" >> "$results.parity.tsv"
rm -- "$run/output"
measure oracle sra1 1 "$run/no-artifact" bash "$oracle_script" "$run/producer-1"
(cd "$run/source"; sha256sum --status -c "$results.inputs.sha256")
[[ $(sha256sum "$compiler" | cut -d ' ' -f 1) == "$compiler_sha" ]] || { echo 'E_SRA_PRODUCER emitter changed during trial' >&2; exit 1; }
perl - "$results" > "$results.summary" <<'PERL'
use strict; use warnings;
my (%wall, %cpu, %count, %ticks);
sub hundredths {
    my ($value) = @_;
    $value =~ /\A([0-9]+)(?:\.([0-9]{1,2}))?\z/ or die "malformed GNU time value\n";
    return 100 * $1 + (defined($2) ? $2 . ('0' x (2 - length($2))) : 0);
}
open my $input, '<', $ARGV[0] or die $!;
while (<$input>) {
    chomp;
    my @field = split /\t/;
    next unless $field[0] eq 'execute';
    $field[3] == 0 or die "execution failed\n";
    $wall{$field[1]} += $field[4];
    $ticks{$field[1]} += hundredths($field[4]);
    $cpu{$field[1]} += $field[5] + $field[6];
    $count{$field[1]}++;
}
$count{sra0} == 3 && $count{sra1} == 3 && $count{'rust-O'} == 1 or die "incomplete fixed set\n";
my $base = $wall{sra0} / 3;
my $candidate = $wall{sra1} / 3;
my $reduction = 100 * ($base - $candidate) / $base;
printf "baseline_mean_wall_s=%.6f\ncandidate_mean_wall_s=%.6f\nwall_reduction_percent=%.6f\n", $base, $candidate, $reduction;
printf "baseline_mean_cpu_s=%.6f\ncandidate_mean_cpu_s=%.6f\nnative_rust_O_ratio=%.6f\n", $cpu{sra0}/3, $cpu{sra1}/3, $candidate/$wall{'rust-O'};
printf "retain_pass=%s\nwithin_twice_rust_O=%s\n", 10*$ticks{sra1} <= 9*$ticks{sra0} ? 'true' : 'false', $candidate <= 2*$wall{'rust-O'} ? 'true' : 'false';
PERL
completed=true
cat "$results.summary"
