#!/usr/bin/env bash
# Exact fixed-source compiler attribution, separate from the producer-hash workload.
set -euo pipefail
cd "$(dirname "$0")/.."
attribution_root=$PWD
if [[ ${1-} == --cleanup ]]; then
    (($# == 2)) || exit 2
    abandoned=$(realpath -e "$2")
    [[ $abandoned == "$attribution_root"/target/compiler-attribution.* && ${abandoned#"$attribution_root/target/"} != */* && ! -L $2 ]] || exit 2
    [[ -f $abandoned/owner-repository && $(cat "$abandoned/owner-repository") == "$attribution_root" ]] || exit 2
    owner_pid=$(cat "$abandoned/owner-pid")
    [[ $owner_pid =~ ^[1-9][0-9]*$ ]] || exit 2
    if kill -0 "$owner_pid" 2>/dev/null; then echo 'benchmark owner is alive; cleanup refused' >&2; exit 2; fi
    # Every compiler/build command uses an absolute path inside its owned tree.
    if ps -eo pid=,args= | awk -v run="$abandoned" -v self="$$" -v parent="$PPID" '$1!=self && $1!=parent && index($0,run)>0 && $0!~/awk -v run=/ {found=1} END {exit !found}'; then
        echo 'a command still refers to the benchmark tree; cleanup refused' >&2; exit 2
    fi
    evidence="$attribution_root/target/${abandoned##*/}.recovery"
    mkdir "$evidence"
    for name in checkpoint time stdout stderr; do
        if [[ -f $abandoned/$name && ! -L $abandoned/$name ]]; then cp "$abandoned/$name" "$evidence/$name"; fi
    done
    rm -rf -- "$abandoned"
    printf 'removed interrupted tree; evidence=%s\n' "$evidence"
    exit 0
fi
compiler=target/lkjscript
revision=26bf2c3c021dd79810a81fa1d9054d59f3768b06
memory_mib=3072
deadline=900
results=target/compiler-attribution.tsv
while (($#)); do
    case "$1" in
        --compiler|--revision|--memory-mib|--deadline|--results)
            (($# >= 2)) || exit 2
            case "$1" in
                --compiler) compiler=$2 ;;
                --revision) revision=$2 ;;
                --memory-mib) memory_mib=$2 ;;
                --deadline) deadline=$2 ;;
                --results) results=$2 ;;
            esac
            shift 2 ;;
        *) echo 'usage: bash bench/compiler_attribution.sh [--compiler FILE] [--revision COMMIT] [--memory-mib N] [--deadline S] [--results FILE]' >&2; exit 2 ;;
    esac
done
[[ $revision =~ ^[0-9a-f]{7,40}$ && $memory_mib =~ ^[1-9][0-9]*$ && $deadline =~ ^[1-9][0-9]*$ ]] || exit 2
revision=$(git rev-parse --verify "$revision^{commit}")
compiler=$(realpath "$compiler")
results=$(realpath -m "$results")
[[ -x $compiler && ! -e $results ]] || exit 2
mkdir -p target "$(dirname "$results")"
free_kib=$(df -Pk target | awk 'NR==2 {print $4}')
if ((free_kib < 3 * 1024 * 1024)); then echo 'E_ATTRIBUTION_SPACE need 3 GiB free' >&2; exit 1; fi
umask 077
run=$(mktemp -d "$attribution_root/target/compiler-attribution.XXXXXX")
printf '%s\n' "$attribution_root" > "$run/owner-repository"
printf '%s\n' "$BASHPID" > "$run/owner-pid"
printf 'run_directory=%s\n' "$run"
trap 'rm -rf -- "$run"' EXIT
trap 'exit 130' HUP INT TERM
mkdir "$run/source"
git archive "$revision" src library witness seed/lkjscript.gz | tar -x -C "$run/source"
gzip -dc "$run/source/seed/lkjscript.gz" > "$run/seed"
chmod 700 "$run/seed"
{
    printf 'revision=%s\nsource_tree=%s\nlibrary_tree=%s\nwitness_tree=%s\n' "$revision" "$(git rev-parse "$revision:src")" "$(git rev-parse "$revision:library")" "$(git rev-parse "$revision:witness")"
    printf 'seed_sha256=%s\ncandidate_producer_sha256=%s\n' "$(sha256sum "$run/seed" | cut -d ' ' -f 1)" "$(sha256sum "$compiler" | cut -d ' ' -f 1)"
    printf 'memory_mib=%s\ndeadline_s=%s\nworkers=1\ncodegen_units=1\noverflow_checks=true\npanic=manifest-default\n' "$memory_mib" "$deadline"
    printf 'native_inline_limit=0\nno_mir_flags=-Zinline-mir=no\nno_llvm_flags=--force-remove-attribute=alwaysinline --force-attribute=noinline\n'
    printf 'oracle=all measured compiler outputs byte-identical; selected native instructions separately compared with original checked-IR interpretation\n'
    rustc --version
} > "$results.meta"
printf 'phase\troute\trepeat\tstatus\twall_s\tuser_s\tkernel_s\trss_kib\timage_bytes\timage_sha256\tload_before\tload_after\n' > "$results"
load() { cut -d ' ' -f 1-3 /proc/loadavg; }
measure() {
    local phase=$1 route=$2 repetition=$3 artifact=$4 before after status=0 bytes=- digest=-
    shift 4
    before=$(load)
    printf 'phase=%s\nroute=%s\nrepeat=%s\nload_before=%s\nstarted_ns=%s\nresults=%s\n' "$phase" "$route" "$repetition" "$before" "$(date +%s%N)" "$results" > "$run/checkpoint"
    /usr/bin/time -f '%e\t%U\t%S\t%M' -o "$run/time" \
        prlimit --as=$((memory_mib * 1024 * 1024)) -- timeout --kill-after=10s "${deadline}s" "$@" > "$run/stdout" 2> "$run/stderr" || status=$?
    after=$(load)
    if [[ -f $artifact ]]; then
        bytes=$(stat -c %s "$artifact")
        digest=$(sha256sum "$artifact" | cut -d ' ' -f 1)
    fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$phase" "$route" "$repetition" "$status" "$(tail -n 1 "$run/time")" "$bytes" "$digest" "$before" "$after" | tee -a "$results"
    cp "$run/stdout" "$results.$phase.$route.$repetition.stdout"
    cp "$run/stderr" "$results.$phase.$route.$repetition.stderr"
    if ((status)); then cat "$run/stderr" >&2; return "$status"; fi
}
compile_source() {
    local route=$1 producer=$2 output=$3
    shift 3
    measure build "$route" 1 "$output" "$producer" build src/main.lkj --source-dir "$run/source" \
        --library-dir "$run/source/library" --runtime-source "$run/source/library/runtime.lkj" \
        --no-store "$@" -o "$output"
}
compile_source candidate "$compiler" "$run/candidate"
compile_source baseline-profile "$run/seed" "$run/baseline-profile" --profile
compile_source candidate-profile "$compiler" "$run/candidate-profile" --profile
routes=(baseline candidate)
for route in opt0 opt1 opt2 opt3 opt3-no-mir opt3-noinline; do
    level=${route#opt}; level=${level%%-*}
    rust_flags=
    bootstrap=(env -u RUSTC_BOOTSTRAP)
    if [[ $route == opt3-no-mir || $route == opt3-noinline ]]; then
        bootstrap=(env RUSTC_BOOTSTRAP=1)
        rust_flags='-Zinline-mir=no'
    fi
    if [[ $route == opt3-noinline ]]; then
        rust_flags+=' -C llvm-args=--force-remove-attribute=alwaysinline -C llvm-args=--force-attribute=noinline'
    fi
    measure build "$route" 1 "$run/cargo/release/lkjscript" "${bootstrap[@]}" \
        CARGO_BUILD_JOBS=1 CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_OPT_LEVEL="$level" \
        CARGO_TARGET_DIR="$run/cargo" RUSTFLAGS="$rust_flags" \
        cargo build --manifest-path "$run/source/witness/Cargo.toml" --release --quiet --locked --offline --bin lkjscript
    cp "$run/cargo/release/lkjscript" "$run/$route"
    rm -rf -- "$run/cargo"
    routes+=("$route")
done
expected=
for repetition in 1 2; do
    for route in "${routes[@]}"; do
        # Rust levels need one row; native baseline/candidate are interleaved twice.
        if ((repetition == 2)) && [[ $route != baseline && $route != candidate ]]; then continue; fi
        executable="$run/$route"
        if [[ $route == baseline ]]; then executable="$run/seed"; fi
        output="$run/output-$route"
        measure run "$route" "$repetition" "$output" "$executable" build src/main.lkj --source-dir "$run/source" \
            --library-dir "$run/source/library" --runtime-source "$run/source/library/runtime.lkj" --no-store -o "$output"
        if [[ -z $expected ]]; then
            expected="$run/expected"
            cp "$output" "$expected"
            printf 'expected_output_sha256=%s\nexpected_output_bytes=%s\n' "$(sha256sum "$expected" | cut -d ' ' -f 1)" "$(stat -c %s "$expected")" >> "$results.meta"
        fi
        cmp "$expected" "$output"
        [[ ! -s $run/stdout && ! -s $run/stderr ]] || { echo 'unexpected compiler output' >&2; exit 1; }
        rm -- "$output"
    done
done
for route in baseline-profile candidate-profile; do
    output="$run/output-$route"
    measure profile "$route" 1 "$output" "$run/$route" build src/main.lkj --source-dir "$run/source" \
        --library-dir "$run/source/library" --runtime-source "$run/source/library/runtime.lkj" --no-store -o "$output"
    cmp "$expected" "$output"
    "$run/seed" profile "$run/$route".samples.* > "$results.$route.profile"
done
printf 'ok exact compiler output across every route; results=%s\n' "$results"
