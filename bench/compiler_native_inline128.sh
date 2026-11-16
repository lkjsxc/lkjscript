#!/usr/bin/env bash
# One predeclared producer64/producer128 hypothesis; measured output policy stays zero.
set -euo pipefail
export LC_ALL=C
cd "$(dirname "$0")/.."
trial_root=$PWD
revision=26bf2c3c021dd79810a81fa1d9054d59f3768b06
expected_output_sha=8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132
seed_revision=a4fd685a9bb2e7736110a4ed2a43651dfe040d56
expected_compiler_sha=4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09
expected_archive_sha=6c4b72837d4982c1c90c24ee8212c0934f6a5b836bb7d06194f5a894be3cf306
emitter_revision=ec3e2ce8
results=target/compiler-native-inline128.tsv
memory_mib=4096
deadline=900

# Cleanup is explicit so the retained fixture can be inspected before acceptance.
# Interrupted measurements already write their checkpoints and captures outside it.
if [[ ${1:-} == --cleanup ]]; then
    [[ $# == 2 ]] || exit 2
    owned=$(realpath -e "$2")
    [[ -d $owned && ! -L $2 && $(dirname "$owned") == "$trial_root/target" ]] || exit 2
    [[ $(basename "$owned") == compiler-native-inline128.* ]] || exit 2
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
        --results) results=$2 ;;
        *) echo 'usage: compiler_native_inline128.sh [--results TSV] | --cleanup OWNED_DIRECTORY' >&2; exit 2 ;;
    esac
    shift 2
done
results=$(realpath -m "$results")
for suffix in '' .meta .raw .attempts.log .outcome .summary .parity.tsv .decision .fixture; do
    [[ ! -e $results$suffix ]] || { echo 'E_INLINE_RESULTS existing evidence must not be overwritten' >&2; exit 2; }
done
mkdir -p target "$(dirname "$results")"
umask 077
raw="$results.raw"
mkdir "$raw"
run=none
current_phase=prepare
completed=false
printf 'phase=prepare started=true retries=none\n' > "$results.attempts.log"
finish() {
    local status=$?
    trap - EXIT HUP INT TERM
    printf 'phase=%s status=%s completed=%s\n' "$current_phase" "$status" "$completed" >> "$results.attempts.log"
    printf 'status=%s\ncompleted=%s\nlast_phase=%s\nretained_fixture=%s\n' "$status" "$completed" "$current_phase" "$run" > "$results.outcome"
    printf 'retained_fixture=%s\nresults=%s\n' "$run" "$results"
    exit "$status"
}
trap finish EXIT
trap 'exit 130' HUP INT TERM
revision=$(git rev-parse --verify "$revision^{commit}")
seed_revision=$(git rev-parse --verify "$seed_revision^{commit}")
free_kib=$(df -Pk target | awk 'NR==2 {print $4}')
((free_kib >= 3 * 1024 * 1024)) || { echo 'E_INLINE_SPACE need 3 GiB free' >&2; exit 1; }
run=$(mktemp -d "$trial_root/target/compiler-native-inline128.XXXXXX")
printf '%s\n' "$trial_root" > "$run/owner-repository"
printf '%s\n' "$BASHPID" > "$run/owner-pid"
awk '{print $22}' "/proc/$BASHPID/stat" > "$run/owner-start"
printf '%s\n' "$run" > "$results.fixture"
git show "$seed_revision:seed/lkjscript.gz" > "$run/emitter-seed.gz"
archive_sha=$(sha256sum "$run/emitter-seed.gz" | cut -d ' ' -f 1)
[[ $archive_sha == "$expected_archive_sha" ]] || { echo 'E_INLINE_SEED pinned seed archive digest differs' >&2; exit 1; }
gzip -dc "$run/emitter-seed.gz" > "$run/emitter"
chmod 500 "$run/emitter"
compiler_sha=$(sha256sum "$run/emitter" | cut -d ' ' -f 1)
[[ $compiler_sha == "$expected_compiler_sha" && $(stat -c %s "$run/emitter") == 20627356 ]] || { echo 'E_INLINE_PRODUCER pinned emitter identity differs' >&2; exit 1; }
compiler="$run/emitter"
mkdir -p "$run/source/target"
git archive "$revision" src library seed/lkjscript.gz tools/optimizer-test.sh tools/generators tests/conformance/ir tests/fixtures/world/self_spawn.lkj | tar -x -C "$run/source"
gzip -dc "$run/source/seed/lkjscript.gz" > "$run/expected"
[[ $(sha256sum "$run/expected" | cut -d ' ' -f 1) == "$expected_output_sha" && $(stat -c %s "$run/expected") == 16228028 ]] || { echo 'E_INLINE_EXPECTED frozen seed identity differs' >&2; exit 1; }
oracle_script="$run/source/tools/optimizer-test.sh"
# Keep the frozen oracle itself byte-identical. Its exact optimized-limit argv
# value is translated only when the conditional oracle runs, never in timing rows.
cat > "$run/oracle-compiler" <<'ORACLE'
#!/usr/bin/env bash
set -euo pipefail
oracle_root=$(cd "$(dirname "$0")" && pwd)
arguments=("$@")
translated=0
for ((index=0; index + 1 < ${#arguments[@]}; index++)); do
    if [[ ${arguments[index]} == --inline-limit && ${arguments[index + 1]} == 64 ]]; then
        arguments[index + 1]=128
        translated=$((translated + 1))
    fi
done
{ printf 'translated_inline64_pairs=%s argv' "$translated"; printf ' %q' "${arguments[@]}"; printf '\n'; } >> "$oracle_root/oracle-invocations.log"
exec "$oracle_root/producer-128" "${arguments[@]}"
ORACLE
chmod 700 "$run/oracle-compiler"
{
    printf 'revision=%s\nsource_tree=%s\nlibrary_tree=%s\n' "$revision" "$(git rev-parse "$revision:src")" "$(git rev-parse "$revision:library")"
    printf 'emitter_sha256=%s\nemitter_bytes=%s\nemitter_declared_revision=%s\n' "$compiler_sha" "$(stat -c %s "$compiler")" "$emitter_revision"
    printf 'emitter_seed_revision=%s\nemitter_seed_archive_sha256=%s\n' "$seed_revision" "$archive_sha"
    printf 'expected_output_sha256=%s\nexpected_output_bytes=16228028\n' "$expected_output_sha"
    printf 'runtime_source_sha256=%s\noracle_script_sha256=%s\n' "$(sha256sum "$run/source/library/runtime.lkj" | cut -d ' ' -f 1)" "$(sha256sum "$oracle_script" | cut -d ' ' -f 1)"
    printf 'frozen_optimizer_sha256=%s\nharness_sha256=%s\n' "$(sha256sum "$run/source/src/optimizer.lkj" | cut -d ' ' -f 1)" "$(sha256sum "$0" | cut -d ' ' -f 1)"
    printf 'source_root=%s\nsource_argument=src/main.lkj\nlibrary_selector=physical\nruntime_selector=physical\n' "$run/source"
    printf 'workers=1\nmemory_mib=%s\ndeadline_s=%s\ntermination_grace_s=10\nfree_space_kib_before=%s\n' "$memory_mib" "$deadline" "$free_kib"
    printf 'construction_inline_limits=64,128\nexecution_order=64,128,128,64,64,128\nexecution_output_inline_limit=0\nproducer_build_and_execution_store=disabled\n'
    printf 'oracle_inline_limit=128\noracle_condition=mean-wall-reduction-at-least-10-percent\noracle_script_unchanged=true\noracle_argv_translation=only-inline-limit-64-to-128\noracle_uses_own_test_store=true\nfile_cache_reset=false\nretries=none\nhash_workload=not_run\n'
    printf 'oracle_wrapper_sha256=%s\nprimary_retention_metric=arithmetic-mean-wall-all-three-runs-per-policy\n' "$(sha256sum "$run/oracle-compiler" | cut -d ' ' -f 1)"

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
for limit in 64 128; do
    measure construct "inline$limit" 1 "$run/producer-$limit" "$compiler" build src/main.lkj \
        --source-dir "$run/source" --library-dir "$run/source/library" --runtime-source "$run/source/library/runtime.lkj" \
        --no-store --inline-limit "$limit" --stats -o "$run/producer-$limit"
    grep -Eq '"inline_accepted":[1-9][0-9]*' "$raw/construct.inline$limit.1.stderr"
    grep -q '"inline_rejected":0' "$raw/construct.inline$limit.1.stderr"
done
if cmp -s "$run/producer-64" "$run/producer-128"; then echo 'E_INLINE_UNCHANGED threshold changed no producer bytes' >&2; exit 1; fi
repeat64=0
repeat128=0
for limit in 64 128 128 64 64 128; do
    if ((limit == 64)); then repeat64=$((repeat64 + 1)); repetition=$repeat64; else repeat128=$((repeat128 + 1)); repetition=$repeat128; fi
    output="$run/output-inline$limit-$repetition"
    measure execute "inline$limit" "$repetition" "$output" "$run/producer-$limit" build src/main.lkj \
        --source-dir "$run/source" --library-dir "$run/source/library" --runtime-source "$run/source/library/runtime.lkj" \
        --no-store --inline-limit 0 -o "$output"
    cmp "$run/expected" "$output"
    [[ ! -s $raw/execute.inline$limit.$repetition.stdout && ! -s $raw/execute.inline$limit.$repetition.stderr ]] || { echo 'E_INLINE_DIAGNOSTICS unexpected execution diagnostics' >&2; exit 1; }
    printf 'inline%s\t%s\t%s\t%s\tyes\tyes\tyes\n' "$limit" "$repetition" "$(sha256sum "$output" | cut -d ' ' -f 1)" "$(stat -c %s "$output")" >> "$results.parity.tsv"
done
current_phase=decision
# GNU time emits two decimal places; compare integer centiseconds so the
# predeclared >=10% condition does not depend on floating-point rounding.
awk -F '\t' '
    function centiseconds(value, parts) {
        if (value !~ /^[0-9]+[.][0-9][0-9]$/) exit 2
        split(value,parts,"."); return parts[1]*100+parts[2]
    }
    $1=="execute" {
        if ($4 != 0 || ($2 != "inline64" && $2 != "inline128")) exit 2
        n[$2]++
        wall[$2]+=centiseconds($5)
        cpu[$2]+=centiseconds($6)+centiseconds($7)
    }
    END {
        if (n["inline64"] != 3 || n["inline128"] != 3 || wall["inline64"] <= 0 || cpu["inline64"] <= 0) exit 2
        printf "mean_wall_s_inline64=%.6f\nmean_wall_s_inline128=%.6f\n", wall["inline64"]/300,wall["inline128"]/300
        printf "mean_cpu_s_inline64=%.6f\nmean_cpu_s_inline128=%.6f\n", cpu["inline64"]/300,cpu["inline128"]/300
        printf "wall_reduction_percent=%.9f\ncpu_reduction_percent=%.9f\n",100*(1-wall["inline128"]/wall["inline64"]),100*(1-cpu["inline128"]/cpu["inline64"])
        printf "candidate_qualifies=%s\n",10*wall["inline128"] <= 9*wall["inline64"] ? "true" : "false"
    }
' "$results" > "$results.summary"
if grep -qx 'candidate_qualifies=true' "$results.summary"; then
    measure oracle inline128 1 "$run/no-artifact" bash "$oracle_script" "$run/oracle-compiler"
    cp -- "$run/oracle-invocations.log" "$raw/oracle-invocations.log"
    printf 'decision=threshold-cleared-oracle-passed\npolicy_change=none-awaiting-separate-review\n' > "$results.decision"
else
    printf 'decision=discard-inline128-below-threshold\noracle=not-run-below-threshold\npolicy_change=none\n' > "$results.decision"
fi
completed=true
printf 'ok fixed producer64/128 trial; all six outputs equal the frozen seed; see the predeclared decision\n'
