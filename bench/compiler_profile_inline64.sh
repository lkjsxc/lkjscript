#!/usr/bin/env bash
# One instrumented producer0/producer64 pair; attribution only, output policy zero.
set -euo pipefail
cd "$(dirname "$0")/.."
trial_root=$PWD
revision=26bf2c3c021dd79810a81fa1d9054d59f3768b06
expected_output_sha=8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132
seed_archive=seed/lkjscript.gz
expected_compiler_sha=4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09
emitter_revision=ec3e2ce8
results=target/compiler-profile-inline64.tsv
memory_mib=4096
deadline=900

# Cleanup is explicit so the retained fixture can be inspected before acceptance.
# Interrupted measurements already write their checkpoints and captures outside it.
if [[ ${1:-} == --cleanup ]]; then
    [[ $# == 2 ]] || exit 2
    owned=$(realpath -e "$2")
    [[ -d $owned && ! -L $2 && $(dirname "$owned") == "$trial_root/target" ]] || exit 2
    [[ $(basename "$owned") == compiler-profile-inline64.* ]] || exit 2
    [[ $(cat "$owned/owner-repository") == "$trial_root" ]] || exit 2
    owner=$(cat "$owned/owner-pid")
    owner_start=$(cat "$owned/owner-start")
    [[ $owner =~ ^[1-9][0-9]*$ && $owner_start =~ ^[0-9]+$ ]] || exit 2
    if [[ -r /proc/$owner/stat ]] && [[ $(awk '{print $22}' "/proc/$owner/stat") == "$owner_start" ]]; then
        echo 'E_INLINE_OWNER recorded owner is still alive' >&2; exit 1
    fi
    # Ignore only this cleanup command and its ancestors, not other users of the fixture.
    ancestors=,
    ancestor=$BASHPID
    while [[ $ancestor =~ ^[1-9][0-9]*$ && -r /proc/$ancestor/stat ]]; do
        ancestors+="$ancestor,"
        ancestor=$(awk '{print $4}' "/proc/$ancestor/stat")
    done
    processes=$(ps -eo pid=,args=)
    if printf '%s\n' "$processes" | awk -v path="$owned" -v excluded="$ancestors" 'index($0,path) && !index(excluded,"," $1 ",") { found=1 } END { exit !found }'; then
        echo 'E_INLINE_ACTIVE a process still references the fixture' >&2; exit 1
    fi
    rm -rf -- "$owned"
    printf 'removed_owned_fixture=%s\n' "$owned"
    exit 0
fi
while (($#)); do
    (($# >= 2)) || exit 2
    case "$1" in
        --seed) seed_archive=$2 ;;
        --results) results=$2 ;;
        --memory-mib) memory_mib=$2 ;;
        --deadline) deadline=$2 ;;
        *) echo 'usage: compiler_profile_inline64.sh [--seed COMPRESSED_SEED] [--results TSV] [--memory-mib N] [--deadline S] | --cleanup OWNED_DIRECTORY' >&2; exit 2 ;;
    esac
    shift 2
done
[[ $expected_compiler_sha =~ ^[0-9a-f]{64}$ && $memory_mib =~ ^[1-9][0-9]*$ && $deadline =~ ^[1-9][0-9]*$ ]] || exit 2
revision=$(git rev-parse --verify "$revision^{commit}")
seed_archive=$(realpath -e "$seed_archive")
results=$(realpath -m "$results")
[[ -f $seed_archive && ! -e $results && ! -e $results.meta && ! -e $results.raw ]] || exit 2
mkdir -p target "$(dirname "$results")"
free_kib=$(df -Pk target | awk 'NR==2 {print $4}')
((free_kib >= 3 * 1024 * 1024)) || { echo 'E_INLINE_SPACE need 3 GiB free' >&2; exit 1; }
umask 077
run=$(mktemp -d "$trial_root/target/compiler-profile-inline64.XXXXXX")
raw="$results.raw"
mkdir "$raw"
printf '%s\n' "$trial_root" > "$run/owner-repository"
printf '%s\n' "$BASHPID" > "$run/owner-pid"
awk '{print $22}' "/proc/$BASHPID/stat" > "$run/owner-start"
printf '%s\n' "$run" > "$results.fixture"
current_phase=prepare
completed=false
finish() {
    local status=$?
    trap - EXIT HUP INT TERM
    printf 'status=%s\ncompleted=%s\nlast_phase=%s\nretained_fixture=%s\n' "$status" "$completed" "$current_phase" "$run" > "$results.outcome"
    printf 'retained_fixture=%s\nresults=%s\n' "$run" "$results"
    exit "$status"
}
trap finish EXIT
trap 'exit 130' HUP INT TERM
gzip -dc "$seed_archive" > "$run/emitter"
chmod 500 "$run/emitter"
compiler_sha=$(sha256sum "$run/emitter" | cut -d ' ' -f 1)
[[ $compiler_sha == "$expected_compiler_sha" ]] || { echo 'E_INLINE_PRODUCER copied emitter digest differs' >&2; exit 1; }
compiler="$run/emitter"
mkdir -p "$run/source/target"
git archive "$revision" src library seed/lkjscript.gz | tar -x -C "$run/source"
gzip -dc "$run/source/seed/lkjscript.gz" > "$run/expected"
[[ $(sha256sum "$run/expected" | cut -d ' ' -f 1) == "$expected_output_sha" && $(stat -c %s "$run/expected") == 16228028 ]] || { echo 'E_INLINE_EXPECTED frozen seed identity differs' >&2; exit 1; }
{
    printf 'revision=%s\nsource_tree=%s\nlibrary_tree=%s\n' "$revision" "$(git rev-parse "$revision:src")" "$(git rev-parse "$revision:library")"
    printf 'emitter_sha256=%s\nemitter_bytes=%s\nemitter_declared_revision=%s\n' "$compiler_sha" "$(stat -c %s "$compiler")" "$emitter_revision"
    printf 'expected_output_sha256=%s\nexpected_output_bytes=16228028\n' "$expected_output_sha"
    printf 'runtime_source_sha256=%s\nemitter_seed_archive_sha256=%s\n' "$(sha256sum "$run/source/library/runtime.lkj" | cut -d ' ' -f 1)" "$(sha256sum "$seed_archive" | cut -d ' ' -f 1)"
    printf 'frozen_optimizer_sha256=%s\nharness_sha256=%s\n' "$(sha256sum "$run/source/src/optimizer.lkj" | cut -d ' ' -f 1)" "$(sha256sum "$0" | cut -d ' ' -f 1)"
    printf 'source_root=%s\nsource_argument=src/main.lkj\nlibrary_selector=physical\nruntime_selector=physical\n' "$run/source"
    printf 'workers=1\nmemory_mib=%s\ndeadline_s=%s\ntermination_grace_s=10\nfree_space_kib_before=%s\n' "$memory_mib" "$deadline" "$free_kib"
    printf 'construction_inline_limits=0,64\nprofile_execution_order=0,64\nexecution_output_inline_limit=0\nproducer_build_and_execution_store=disabled\n'
    printf 'profile_enabled=true\nrequested_period_us=1000\ninstrumented_times_are_not_new_speedup_observations=true\nfile_cache_reset=false\nretries=none\nhash_workload=not_run\noracle=prior_accepted_inline64_campaign\n'
    printf 'counter_decoder_sha256=%s\n' "$(sha256sum bench/compiler_profile_tables.pl | cut -d ' ' -f 1)"
    printf 'primary_wall=gnu_time_elapsed_seconds\nlaunch_capture=before_time_invocation_to_after_exit\nconstruction_stats_include_original_and_candidate_admission=true\n'
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
for limit in 0 64; do
    measure construct "profile$limit" 1 "$run/producer-profile-$limit" "$compiler" build src/main.lkj \
        --source-dir "$run/source" --library-dir "$run/source/library" --runtime-source "$run/source/library/runtime.lkj" \
        --no-store --inline-limit "$limit" --profile --stats -o "$run/producer-profile-$limit"
done
grep -Eq '"inline_accepted":[1-9][0-9]*' "$raw/construct.profile64.1.stderr"
grep -q '"inline_rejected":0' "$raw/construct.profile64.1.stderr"
# Both instrumented observations finish before any profile decoding or inspection.
for limit in 0 64; do
    measure profile "profile$limit" 1 "$run/output-profile-$limit" "$run/producer-profile-$limit" build src/main.lkj \
        --source-dir "$run/source" --library-dir "$run/source/library" --runtime-source "$run/source/library/runtime.lkj" \
        --no-store --inline-limit 0 -o "$run/output-profile-$limit"
    cmp "$run/expected" "$run/output-profile-$limit"
    [[ ! -s $raw/profile.profile$limit.1.stdout && ! -s $raw/profile.profile$limit.1.stderr ]] || { echo 'E_PROFILE_DIAGNOSTICS unexpected profiled execution diagnostics' >&2; exit 1; }
    printf 'profile%s\t1\t%s\t%s\tyes\tyes\tyes\n' "$limit" "$(sha256sum "$run/output-profile-$limit" | cut -d ' ' -f 1)" "$(stat -c %s "$run/output-profile-$limit")" >> "$results.parity.tsv"
done
current_phase=decode
shopt -s nullglob
sample_files=()
for limit in 0 64; do
    samples=("$run/producer-profile-$limit".samples.*)
    [[ ${#samples[@]} == 1 && -f ${samples[0]} ]] || { echo 'E_PROFILE_SAMPLES expected exactly one sample file per policy' >&2; exit 1; }
    sample_files+=("${samples[0]}")
    prlimit --as=$((memory_mib * 1024 * 1024)) -- timeout --kill-after=10s "${deadline}s" \
        "$compiler" profile "${samples[0]}" > "$results.profile$limit.report" 2> "$results.profile$limit.report.stderr"
    [[ ! -s $results.profile$limit.report.stderr ]] || exit 1
done
prlimit --as=$((memory_mib * 1024 * 1024)) -- timeout --kill-after=10s "${deadline}s" \
    perl bench/compiler_profile_tables.pl "$results" "${sample_files[0]}" "${sample_files[1]}"
completed=true
printf 'ok one instrumented 0/64 pair; both outputs equal the frozen seed; existing profile reports accepted\n'
