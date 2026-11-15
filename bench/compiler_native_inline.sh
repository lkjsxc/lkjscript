#!/usr/bin/env bash
# Separate compiler-producer inlining from the output compiler's inline policy.
set -euo pipefail
cd "$(dirname "$0")/.."
trial_root=$PWD
compiler=target/lkjscript
revision=26bf2c3c021dd79810a81fa1d9054d59f3768b06
results=target/compiler-native-inline.tsv
hash_source=bench/hash.lkj
phase=all
memory_mib=3072
deadline=900
while (($#)); do
    (($# >= 2)) || exit 2
    case "$1" in
        --compiler) compiler=$2 ;;
        --revision) revision=$2 ;;
        --results) results=$2 ;;
        --hash-source) hash_source=$2 ;;
        --phase) phase=$2 ;;
        --memory-mib) memory_mib=$2 ;;
        --deadline) deadline=$2 ;;
        *) echo 'usage: compiler_native_inline.sh [--compiler FILE] [--revision COMMIT] [--results FILE] [--hash-source FILE] [--phase all|validation|oracle] [--memory-mib N] [--deadline S]' >&2; exit 2 ;;
    esac
    shift 2
done
[[ $revision =~ ^[0-9a-f]{7,40}$ && $memory_mib =~ ^[1-9][0-9]*$ && $deadline =~ ^[1-9][0-9]*$ ]] || exit 2
[[ $phase == all || $phase == validation || $phase == oracle ]] || exit 2
revision=$(git rev-parse --verify "$revision^{commit}")
compiler=$(realpath -e "$compiler")
hash_source=$(realpath -e "$hash_source")
results=$(realpath -m "$results")
[[ -x $compiler && ! -e $results && ! -e $results.meta ]] || exit 2
mkdir -p target "$(dirname "$results")"
free_kib=$(df -Pk target | awk 'NR==2 {print $4}')
((free_kib >= 3 * 1024 * 1024)) || { echo 'E_INLINE_SPACE need 3 GiB free' >&2; exit 1; }
umask 077
run=$(mktemp -d "$trial_root/target/compiler-native-inline.XXXXXX")
printf 'run_directory=%s\n' "$run"
printf '%s\n' "$trial_root" > "$run/owner-repository"
printf '%s\n' "$BASHPID" > "$run/owner-pid"
trap 'rm -rf -- "$run"' EXIT
trap 'exit 130' HUP INT TERM
mkdir -p "$run/source" "$run/source/target" "$run/source/bench" "$run/source/tools" "$run/source/tests/fixtures/world"
git archive "$revision" src library seed/lkjscript.gz tools/optimizer-test.sh tools/generators tests/conformance/ir tests/fixtures/world/self_spawn.lkj | tar -x -C "$run/source"
cp "$hash_source" "$run/source/bench/hash.lkj"
oracle_script="$run/source/tools/optimizer-test.sh"
oracle_fixture="$run/source/tests/fixtures/world/self_spawn.lkj"
gzip -dc "$run/source/seed/lkjscript.gz" > "$run/expected"
{
    printf 'revision=%s\nsource_tree=%s\nlibrary_tree=%s\n' "$revision" "$(git rev-parse "$revision:src")" "$(git rev-parse "$revision:library")"
    printf 'emitter_sha256=%s\nexpected_output_sha256=%s\nexpected_output_bytes=%s\n' "$(sha256sum "$compiler" | cut -d ' ' -f 1)" "$(sha256sum "$run/expected" | cut -d ' ' -f 1)" "$(stat -c %s "$run/expected")"
    printf 'hash_source_sha256=%s\noracle_script_sha256=%s\noracle_fixture_sha256=%s\n' "$(sha256sum "$hash_source" | cut -d ' ' -f 1)" "$(sha256sum "$oracle_script" | cut -d ' ' -f 1)" "$(sha256sum "$oracle_fixture" | cut -d ' ' -f 1)"
    printf 'phase=%s\n' "$phase"
    printf 'workers=1\nmemory_mib=%s\ndeadline_s=%s\nexecution_output_inline_limit=0\nproducer_build_and_execution_store=disabled\noracle_uses_own_test_store=true\n' "$memory_mib" "$deadline"
    if [[ $phase == oracle ]]; then printf 'construction_inline_limits=16\n'; else printf 'construction_inline_limits=0,16\n'; fi
    if [[ $phase == all ]]; then printf 'execution_order=0,16,16,0,0,16\n'; else printf 'execution_order=none\n'; fi
    if [[ $phase == oracle ]]; then printf 'hash_repeats=0\n'; else printf 'hash_repeats=20\n'; fi
} > "$results.meta"
printf 'phase\troute\trepeat\tstatus\twall_s\tuser_s\tkernel_s\trss_kib\timage_bytes\timage_sha256\tload_before\tload_after\n' > "$results"
load() { cut -d ' ' -f 1-3 /proc/loadavg; }
measure() {
    local phase=$1 route=$2 repetition=$3 artifact=$4 before after status=0 bytes=- digest=-
    shift 4
    before=$(load)
    printf 'phase=%s\nroute=%s\nrepeat=%s\nload_before=%s\n' "$phase" "$route" "$repetition" "$before" > "$run/checkpoint"
    /usr/bin/time -f '%e\t%U\t%S\t%M' -o "$run/time" \
        prlimit --as=$((memory_mib * 1024 * 1024)) -- timeout --kill-after=10s "${deadline}s" "$@" > "$run/stdout" 2> "$run/stderr" || status=$?
    after=$(load)
    if [[ -f $artifact ]]; then bytes=$(stat -c %s "$artifact"); digest=$(sha256sum "$artifact" | cut -d ' ' -f 1); fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$phase" "$route" "$repetition" "$status" "$(tail -n 1 "$run/time")" "$bytes" "$digest" "$before" "$after" | tee -a "$results"
    cp "$run/stdout" "$results.$phase.$route.$repetition.stdout"
    cp "$run/stderr" "$results.$phase.$route.$repetition.stderr"
    if ((status)); then cat "$run/stderr" >&2; return "$status"; fi
}
construction=construct
if [[ $phase != all ]]; then construction=reconstruct; fi
limits=(0 16)
if [[ $phase == oracle ]]; then limits=(16); fi
for limit in "${limits[@]}"; do
    measure "$construction" "inline$limit" 1 "$run/producer-$limit" "$compiler" build src/main.lkj \
        --source-dir "$run/source" --library-dir "$run/source/library" --runtime-source "$run/source/library/runtime.lkj" \
        --no-store --inline-limit "$limit" --stats -o "$run/producer-$limit"
done
grep -Eq '"inline_accepted":[1-9][0-9]*' "$results.$construction.inline16.1.stderr"
grep -q '"inline_rejected":0' "$results.$construction.inline16.1.stderr"
if [[ $phase != oracle ]] && cmp -s "$run/producer-0" "$run/producer-16"; then echo 'inlining changed no producer bytes' >&2; exit 1; fi
if [[ $phase == all ]]; then
repeat0=0
repeat16=0
for limit in 0 16 16 0 0 16; do
    if ((limit == 0)); then repeat0=$((repeat0 + 1)); repetition=$repeat0; else repeat16=$((repeat16 + 1)); repetition=$repeat16; fi
    measure execute "inline$limit" "$repetition" "$run/output" "$run/producer-$limit" build src/main.lkj \
        --source-dir "$run/source" --library-dir "$run/source/library" --runtime-source "$run/source/library/runtime.lkj" \
        --no-store --inline-limit 0 -o "$run/output"
    cmp "$run/expected" "$run/output"
    [[ ! -s $run/stdout && ! -s $run/stderr ]] || { echo 'unexpected execution diagnostics' >&2; exit 1; }
    rm "$run/output"
done
fi
if [[ $phase != oracle ]]; then
for limit in 0 16; do
    measure hash-build "inline$limit" 1 "$run/hash-$limit" "$compiler" build bench/hash.lkj \
        --source-dir "$run/source" --library-dir "$run/source/library" --runtime-source "$run/source/library/runtime.lkj" \
        --no-store --inline-limit "$limit" --stats -o "$run/hash-$limit"
done
# Cross both executable sizes with both hash policies: growth and hash-code
# speed are separate effects. Internal elapsed_ns excludes build and startup.
for hash_limit in 0 16; do
    for image_limit in 0 16; do
        measure hash "code$hash_limit-image$image_limit" 1 "$run/producer-$image_limit" \
            "$run/hash-$hash_limit" "$run" "producer-$image_limit" 20
        digest=$(sha256sum "$run/producer-$image_limit" | cut -d ' ' -f 1)
        grep -q "sha256=$digest" "$run/stdout"
        [[ ! -s $run/stderr ]] || { echo 'unexpected hash diagnostics' >&2; exit 1; }
    done
done
fi
# Use the frozen revision's actual diagnostic contract, without adapting it to
# current raw paths. This oracle keeps original interpretation, checked passes,
# exact custom/cross-file traps and a bounded executed-mutant campaign.
measure oracle inline16 1 "$run/no-artifact" bash "$oracle_script" "$run/producer-16"
printf 'ok isolated producer trial phase=%s; results=%s\n' "$phase" "$results"
