#!/usr/bin/env bash
# Prepared current-compiler contract cohort. Requires a frozen post-gate addendum.
# This script retains every fixture and key; cleanup needs separate acceptance.
set -euo pipefail
cd "$(dirname "$0")/.."
benchmark_root=$PWD
compiler= identities= results=
while (($#)); do
    case "$1" in
        --compiler|--identities|--results)
            (($# >= 2)) || exit 2
            case "$1" in
                --compiler) compiler=$2 ;;
                --identities) identities=$2 ;;
                --results) results=$2 ;;
            esac
            shift 2 ;;
        *) printf 'usage: bash bench/compiler-query-current.sh --compiler FILE --identities POST_GATE_JSON --results NEW_TARGET_DIRECTORY\n' >&2; exit 2 ;;
    esac
done
[[ -n $compiler && -n $identities && -n $results ]] || exit 2
compiler=$(realpath -e "$compiler")
identities=$(realpath -e "$identities")
results=$(realpath -m "$results")
[[ -x $compiler && -f $identities && ! -L $identities && ! -e $results && ! -L $results ]] || exit 2
[[ $results == "$benchmark_root"/target/* && $results != "$benchmark_root/target/" ]] || exit 2
validator="$benchmark_root/bench/compiler-query-current-validate.pl"
questions="$benchmark_root/bench/compiler-query-current-questions.tsv"
plan="$benchmark_root/bench/compiler-query-current-plan.md"
umask 077
mkdir -p "$benchmark_root/target" "$(dirname "$results")"
mkdir "$results" "$results/raw"
fixture=
keydir=
phase=preflight
finish() {
    local status=$?
    trap - EXIT
    printf 'phase=%s\nstatus=%s\nfixture=%s\nkey_directory=%s\nfinished_utc=%s\n' \
        "$phase" "$status" "$fixture" "$keydir" "$(date -u +%FT%TZ)" > "$results/completion"
    printf 'retained evidence=%s fixture=%s key_directory=%s status=%s\n' "$results" "$fixture" "$keydir" "$status"
    exit "$status"
}
trap finish EXIT
trap 'exit 130' HUP INT TERM
printf '%s\n' "$benchmark_root" > "$results/owner-repository"
printf '%s\n' "$BASHPID" > "$results/owner-pid"
cp "$identities" "$results/identities.submitted.json"
sha256sum "$identities" "$validator" "$questions" "$plan" "$benchmark_root/bench/compiler-query-current.sh" > "$results/harness-inputs.sha256"
perl "$validator" questions "$questions" > "$results/questions-validation.stdout" 2> "$results/questions-validation.stderr"
perl "$validator" identity "$identities" "$benchmark_root" "$compiler" > "$results/identities.json" 2> "$results/identity-validation.stderr"
revision=$(perl -MJSON::PP -e 'local $/;open my $f,"<",$ARGV[0] or die $!;print decode_json(<$f>)->{revision}' "$results/identities.json")
expected_producer=$(perl -MJSON::PP -e 'local $/;open my $f,"<",$ARGV[0] or die $!;print decode_json(<$f>)->{producer_sha256}' "$results/identities.json")
free_kib=$(df -Pk "$benchmark_root/target" | awk 'NR==2 {print $4}')
[[ $free_kib =~ ^[0-9]+$ ]] || exit 2
printf 'available_bytes=%s\nrequired_bytes=6442450944\n' "$((free_kib * 1024))" > "$results/preflight"
if ((free_kib < 6 * 1024 * 1024)); then
    printf 'E_QUERY_BENCH_SPACE need six GiB before fixture expansion\n' > "$results/preflight.stderr"
    exit 1
fi
phase=prepare
fixture=$(mktemp -d "$benchmark_root/target/compiler-query-current-run.XXXXXX")
keydir=$(mktemp -d /tmp/compiler-query-current-key.XXXXXX)
printf '%s\n' "$benchmark_root" > "$fixture/owner-repository"
printf '%s\n' "$results" > "$fixture/owner-evidence"
printf '%s\n' "$BASHPID" > "$fixture/owner-pid"
printf '%s\n' "$keydir" > "$fixture/owner-key"
printf '%s\n' "$fixture" > "$keydir/owner-run"
printf '%s\n' "$fixture" > "$results/fixture.path"
printf '%s\n' "$keydir" > "$results/key-directory.path"
mkdir "$fixture/source" "$fixture/store" "$fixture/output" "$fixture/bin"
git archive "$revision" src library tools tests | tar -x -C "$fixture/source"
cp "$compiler" "$fixture/bin/producer"
chmod 700 "$fixture/bin/producer"
[[ $(sha256sum "$fixture/bin/producer" | cut -d ' ' -f 1) == "$expected_producer" ]] || {
    printf 'E_QUERY_BENCH_PRODUCER copied bytes differ\n' > "$results/preparation.stderr"; exit 1;
}
head -c 32 /dev/urandom > "$keydir/key"
chmod 600 "$keydir/key"
[[ $(stat -c %s "$keydir/key") == 32 ]] || exit 1
source_root="$fixture/source"
key="$keydir/key"
case "$key" in "$source_root"/*) printf 'key entered source grant\n' >&2; exit 1 ;; esac
# No directory, store, output, binary or key is created under the source grant.
find "$source_root" -type l -print > "$results/source-symlinks"
[[ ! -s "$results/source-symlinks" ]] || { printf 'source archive contains symbolic links\n' >&2; exit 1; }
(
    cd "$source_root"
    find src library tools tests -type f -print0 | sort -z | xargs -0 sha256sum
) > "$results/inputs-before.sha256"
cp "$questions" "$results/requests.tsv"
sha256sum "$fixture/bin/producer" > "$results/producer-before.sha256"
{
    printf 'version=1\nrevision=%s\nproducer_sha256=%s\nsource_root=%s\nsource_grant=.\n' "$revision" "$expected_producer" "$source_root"
    printf 'source_entry=src/main.lkj\nsource_directory_option=absent\nlibrary_dir=library\nruntime_source=library/runtime.lkj\ninline_limit=64\n'
    printf 'store_budget_bytes=2147483648\nbuild_memory_bytes=4294967296\nbuild_deadline_s=900\nquery_memory_bytes=2147483648\nquery_deadline_s=30\nkill_grace_s=10\n'
    printf 'query_store_bytes_option=absent\nquery_reader_cap=default_per_extent\nworkers=1\nmetadata_prequeries=0\nfile_cache_reset=false\n'
    printf 'population=40_owner_and_slice_then10_supplemental\nthreshold_ns=100000000\nretries=0\n'
    printf 'wrapper_includes=subprocess_resource_launch_and_stdout_stderr_capture\nwrapper_excludes=prior_command_cwd_load_metadata_and_subsequent_hash_validation\n'
    printf 'historical_output_inline_limit=0\ncausal_historical_latency_comparison=false\n'
} > "$results/protocol.meta"
load() { cut -d ' ' -f 1-3 /proc/loadavg | tr ' ' ','; }
measure() {
    local tag=$1 memory=$2 deadline=$3
    shift 3
    local before after started finished status=0
    printf '%s\n' "$source_root" > "$results/raw/$tag.cwd"
    perl -MJSON::PP -e 'print JSON::PP->new->canonical->encode(\@ARGV),"\n"' "$@" > "$results/raw/$tag.argv.json"
    before=$(load)
    printf 'phase=%s\ntag=%s\nload_before=%s\n' "$phase" "$tag" "$before" > "$results/checkpoint"
    started=$(date +%s%N)
    (
        cd "$source_root"
        /usr/bin/time -f '%e %U %S %M' -o "$results/raw/$tag.time" \
            prlimit --as="$memory" -- timeout --kill-after=10s "${deadline}s" "$@"
    ) > "$results/raw/$tag.stdout" 2> "$results/raw/$tag.stderr" || status=$?
    finished=$(date +%s%N)
    after=$(load)
    perl -MJSON::PP -e 'my($status,$start,$finish,$before,$after)=@ARGV;print JSON::PP->new->canonical->encode({status=>0+$status,start_ns=>0+$start,finish_ns=>0+$finish,wrapper_wall_ns=>$finish-$start,load_before=>$before,load_after=>$after}),"\n"' \
        "$status" "$started" "$finished" "$before" "$after" > "$results/raw/$tag.capture.json"
    printf '%s\t%s\t%s\t%s\t%s\n' "$tag" "$status" "$((finished-started))" "$before" "$after" >> "$results/phase-order.tsv"
    return "$status"
}
printf 'tag\tstatus\twrapper_wall_ns\tload_before\tload_after\n' > "$results/phase-order.tsv"
# Relative source/library/runtime naming exactly matches the canonical factory.
# Omitting --source-dir under this owned cwd grants "." only and avoids changing
# diagnostic source strings embedded in the executable.
build_argv=(build src/main.lkj --library-dir library --runtime-source library/runtime.lkj
    --store "$fixture/store" --store-key "$key" --store-bytes 2147483648
    --index --inline-limit 64 --stats -o "$fixture/output/image")
phase=cold-index
status=0
measure cold-index 4294967296 900 env LKJSCRIPT_STORE_IO=1 "$fixture/bin/producer" "${build_argv[@]}" || status=$?
if ((status != 0)); then
    perl "$validator" builds "$results" "$fixture" "$results/identities.json" > "$results/build-validation.stdout" 2> "$results/build-validation.stderr" || true
    exit "$status"
fi
[[ -f "$fixture/output/image" && ! -L "$fixture/output/image" ]] || { printf 'cold image missing\n' > "$results/build-copy.stderr"; exit 1; }
cp "$fixture/output/image" "$fixture/output/cold-index.image"
phase=unchanged-index
status=0
measure unchanged-index 4294967296 900 env LKJSCRIPT_STORE_IO=1 "$fixture/bin/producer" "${build_argv[@]}" || status=$?
if [[ -f "$fixture/output/image" && ! -L "$fixture/output/image" ]]; then
    cp "$fixture/output/image" "$fixture/output/unchanged-index.image"
fi
validation=0
perl "$validator" builds "$results" "$fixture" "$results/identities.json" > "$results/build-validation.stdout" 2> "$results/build-validation.stderr" || validation=$?
((status == 0 && validation == 0)) || exit 1
# Hash validation is backed by direct byte comparisons, outside timed commands.
cmp "$fixture/bin/producer" "$fixture/output/cold-index.image"
cmp "$fixture/bin/producer" "$fixture/output/unchanged-index.image"
printf 'cold=unchanged=gated-canonical; direct cmp success\n' > "$results/image-parity"
snapshot=$(perl -MJSON::PP -e 'local $/;open my $f,"<",$ARGV[0] or die $!;print decode_json(<$f>)->{snapshot}' "$results/pins.json")
[[ $snapshot =~ ^[0-9a-f]{64}$ ]] || exit 1
query_argv=(query src/main.lkj --library-dir library --store "$fixture/store" --store-key "$key" --snapshot "$snapshot" --stats)
phase=queries
count=0
while IFS=$'\t' read -r ordinal group kind identity depth limit bytes; do
    [[ $ordinal == ordinal ]] && continue
    count=$((count + 1))
    [[ $ordinal == "$count" ]] || exit 1
    tag=$(printf 'query-%02d' "$ordinal")
    # A refusal remains a row; no pin repair, rerun or substitute is permitted.
    measure "$tag" 2147483648 30 env -u LKJSCRIPT_STORE_IO "$fixture/bin/producer" \
        "${query_argv[@]}" --kind "$kind" --id "$identity" --depth "$depth" --limit "$limit" --bytes "$bytes" || true
done < "$results/requests.tsv"
[[ $count == 50 ]] || exit 1
printf 'timed_queries=50\ncompleted_utc=%s\n' "$(date -u +%FT%TZ)" > "$results/timed-completion"
phase=postprocessing
validation=0
perl "$validator" queries "$results" "$results/requests.tsv" > "$results/query-validation.stdout" 2> "$results/query-validation.stderr" || validation=$?
(
    cd "$source_root"
    find src library tools tests -type f -print0 | sort -z | xargs -0 sha256sum
) > "$results/inputs-after.sha256"
sha256sum "$fixture/bin/producer" > "$results/producer-after.sha256"
cmp "$results/inputs-before.sha256" "$results/inputs-after.sha256"
cmp "$results/producer-before.sha256" "$results/producer-after.sha256"
printf 'frozen source and executable inventories unchanged\n' > "$results/final-inventory-parity"
phase=complete
exit "$validation"
