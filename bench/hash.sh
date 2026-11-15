#!/usr/bin/env bash
# Sequential fixed-byte SHA attribution; native, optional Rust witness, and profile.
set -euo pipefail
task_root=$(cd "$(dirname "$0")/.." && pwd)
compiler="$task_root/target/lkjscript"
input=
previous=
repeats=20
memory_mib=2048
results="$task_root/target/hash-$$.tsv"
witness=1
while (($#)); do
    case "$1" in
        --compiler) compiler=$2; shift 2 ;;
        --input) input=$2; shift 2 ;;
        --previous) previous=$2; shift 2 ;;
        --repeats) repeats=$2; shift 2 ;;
        --memory-mib) memory_mib=$2; shift 2 ;;
        --results) results=$2; shift 2 ;;
        --no-witness) witness=0; shift ;;
        *) printf 'usage: bash bench/hash.sh [--compiler FILE] [--input FILE] [--previous COMPILER] [--repeats N] [--memory-mib N] [--results FILE] [--no-witness]\n' >&2; exit 2 ;;
    esac
done
[[ $repeats =~ ^[1-9][0-9]*$ && $memory_mib =~ ^[1-9][0-9]*$ ]]
compiler=$(realpath "$compiler")
input=$(realpath "${input:-$compiler}")
results=$(realpath -m "$results")
[[ -x $compiler && -f $input && ! -e $results ]]
if [[ -n $previous ]]; then previous=$(realpath "$previous"); [[ -x $previous ]]; fi
mkdir -p "$task_root/target" "$(dirname "$results")"
input_bytes=$(stat -c %s "$input")
available_kib=$(df -Pk "$task_root/target" | awk 'NR==2 {print $4}')
required_kib=$((input_bytes / 1024 + 262144))
if ((available_kib < required_kib)); then
    printf 'E_HASH_SPACE need_kib=%s available_kib=%s\n' "$required_kib" "$available_kib" >&2
    exit 1
fi
run=$(mktemp -d "$task_root/target/hash.XXXXXX")
cleanup() { rm -rf -- "$run"; }
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
# This ceiling applies only to the experiment, not to programs or language policy.
ulimit -v "$((memory_mib * 1024))"
cp --reflink=auto -- "$input" "$run/input"
cp -a -- "$task_root/library" "$run/library"
cp -- "$task_root/bench/hash.lkj" "$run/workload.lkj"
expected=$(sha256sum "$run/input" | cut -d ' ' -f 1)
{
    printf 'input_sha256=%s\ninput_bytes=%s\nrepeats=%s\nmemory_mib=%s\n' "$expected" "$input_bytes" "$repeats" "$memory_mib"
    printf 'producer_sha256=%s\n' "$(sha256sum "$compiler" | cut -d ' ' -f 1)"
    if [[ -n $previous ]]; then printf 'previous_sha256=%s\n' "$(sha256sum "$previous" | cut -d ' ' -f 1)"; fi
    printf 'sha_source_sha256=%s\n' "$(sha256sum "$run/library/sha256.lkj" | cut -d ' ' -f 1)"
    printf 'workload_sha256=%s\n' "$(sha256sum "$run/workload.lkj" | cut -d ' ' -f 1)"
    printf 'rust_flags=edition2024 overflow-checks=yes panic=abort codegen-units=1\n'
    printf 'no_mir_flags=RUSTC_BOOTSTRAP=1 -Zinline-mir=no\n'
    printf 'no_llvm_flags=--force-remove-attribute=alwaysinline --force-attribute=noinline\n'
} > "$results.meta"
load() { cut -d ' ' -f 1-3 /proc/loadavg; }
printf 'route\tstatus\twall_s\tuser_s\tsys_s\trss_kib\tload_before\tload_after\n' > "$results.build"
build() {
    local label=$1 before after status
    shift
    before=$(load)
    set +e
    /usr/bin/time -f '%e\t%U\t%S\t%M' -o "$run/time" "$@" > "$run/build.stdout" 2> "$run/build.stderr"
    status=$?
    set -e
    after=$(load)
    printf '%s\t%s\t%s\t%s\t%s\n' "$label" "$status" "$(tail -n 1 "$run/time")" "$before" "$after" >> "$results.build"
    if ((status)); then
        cat "$run/build.stdout" "$run/build.stderr" >&2
        return "$status"
    fi
}
build native "$compiler" build "$run/workload.lkj" --library-dir "$run/library" --no-store -o "$run/native"
build profile "$compiler" build "$run/workload.lkj" --library-dir "$run/library" --no-store --profile -o "$run/profile"
routes=(native)
if [[ -n $previous ]]; then
    build previous "$previous" build "$run/workload.lkj" --library-dir "$run/library" --no-store -o "$run/previous"
    routes=(previous native previous native previous native)
fi
if ((witness)) && command -v rustc >/dev/null; then
    rustc --version >> "$results.meta"
    {
        printf '#[path="%s/witness/bridge/output.rs"] mod host;\n' "$task_root"
        printf '#[path="%s/library/sha256.lkj"] mod sha256;\n' "$run"
        printf 'mod case {\nuse crate::host::World;\n'
        sed -e 's|^//!|//|' -e 's|use std::sha256::Sha256;|use crate::sha256::Sha256;|' "$run/workload.lkj"
        printf '}\nfn main() { host::enter(case::main); }\n'
    } > "$run/witness.rs"
    flags=(--edition=2024 -A warnings -C overflow-checks=yes -C panic=abort -C codegen-units=1)
    for level in 0 1 2 3; do
        build "rust-opt$level" rustc "${flags[@]}" -C "opt-level=$level" "$run/witness.rs" -o "$run/rust-opt$level"
        routes+=("rust-opt$level")
    done
    build rust-opt3-no-mir env RUSTC_BOOTSTRAP=1 rustc "${flags[@]}" -C opt-level=3 -Zinline-mir=no "$run/witness.rs" -o "$run/rust-opt3-no-mir"
    build rust-opt3-noinline env RUSTC_BOOTSTRAP=1 rustc "${flags[@]}" -C opt-level=3 -Zinline-mir=no -C llvm-args=--force-remove-attribute=alwaysinline -C llvm-args=--force-attribute=noinline "$run/witness.rs" -o "$run/rust-opt3-noinline"
    routes+=(rust-opt3-no-mir rust-opt3-noinline)
else
    printf 'witness=not-run\n' >> "$results.meta"
fi
routes+=(profile)
printf 'route\tstatus\tbytes\trepeats\telapsed_ns\twall_s\tuser_s\tsys_s\trss_kib\timage_bytes\tload_before\tload_after\tsha256\n' > "$results"
for route in "${routes[@]}"; do
    before=$(load)
    set +e
    /usr/bin/time -f '%e\t%U\t%S\t%M' -o "$run/time" "$run/$route" "$run" input "$repeats" > "$run/output" 2> "$run/error"
    status=$?
    set -e
    after=$(load)
    bytes=$(sed -n 's/^bytes=\([0-9]*\).*/\1/p' "$run/output")
    elapsed=$(sed -n 's/.* elapsed_ns=\([0-9]*\).*/\1/p' "$run/output")
    digest=$(sed -n 's/.* sha256=\([0-9a-f]*\)$/\1/p' "$run/output")
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$route" "$status" "$bytes" "$repeats" "$elapsed" "$(tail -n 1 "$run/time")" "$(stat -c %s "$run/$route")" "$before" "$after" "$digest" | tee -a "$results"
    if ((status)) || [[ $digest != "$expected" || $bytes != "$input_bytes" || ! $elapsed =~ ^[0-9]+$ ]]; then
        cat "$run/output" "$run/error" >&2
        printf 'E_HASH_RESULT route=%s\n' "$route" >&2
        exit 1
    fi
done
"$compiler" profile "$run"/profile.samples.* > "$results.profile"
printf 'results=%s\n' "$results"
