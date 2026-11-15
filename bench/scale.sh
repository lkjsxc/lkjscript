#!/usr/bin/env bash
# Source-shape ladder; generated trees are disposable, outside the gate.
set -euo pipefail
cd "$(dirname "$0")/.."
scale_root=$PWD
# Interrupted supervisors cannot run an EXIT trap. Cleanup is an explicit,
# identity-checked recovery action and never targets a live benchmark process.
if [[ ${1-} == --cleanup ]]; then
    (($# == 2)) || { echo 'usage: bench/scale.sh --cleanup RUN_DIRECTORY' >&2; exit 2; }
    candidate=$(realpath -e "$2")
    [[ $candidate == "$scale_root"/target/scale.* && ${candidate#"$scale_root/target/"} != */* && ! -L $2 ]] || exit 2
    [[ -f $candidate/owner-repository && $(cat "$candidate/owner-repository") == "$scale_root" ]] || {
        echo 'refusing cleanup without matching benchmark ownership' >&2; exit 2;
    }
    owner_pid=$(cat "$candidate/owner-pid")
    [[ $owner_pid =~ ^[1-9][0-9]*$ ]] || exit 2
    if kill -0 "$owner_pid" 2>/dev/null; then echo 'benchmark PID is still alive; cleanup refused' >&2; exit 2; fi
    authority=$(cat "$candidate/owner-key")
    [[ $authority == /tmp/lkjscript-scale-key.* && ${authority#/tmp/} != */* && ! -L $authority ]] || exit 2
    [[ -f $authority/owner-run && $(cat "$authority/owner-run") == "$candidate" ]] || exit 2
    evidence="$scale_root/target/${candidate##*/}.recovery"
    mkdir "$evidence"
    for item in checkpoint shape workload time stderr stdout; do
        if [[ -f $candidate/$item && ! -L $candidate/$item ]]; then cp "$candidate/$item" "$evidence/$item"; fi
    done
    rm -rf -- "$candidate" "$authority"
    printf 'removed interrupted run; retained evidence %s\n' "$evidence"
    exit 0
fi
compiler=target/lkjscript
rungs=10000,100000,1000000,10000000
seed=20261116
memory_ceiling_mib=8192
timeout_ceiling=1800
results=target/scale.tsv
query_driver=
operation_driver=
prior_results=
real=yes
while (($#)); do
    case "$1" in
        --compiler|--rungs|--seed|--memory-ceiling-mib|--timeout-ceiling|--results|--query-driver|--operation-driver|--prior-results)
            (($# >= 2)) || { printf 'missing value: %s\n' "$1" >&2; exit 2; }
            case "$1" in
                --compiler) compiler=$2;;
                --rungs) rungs=$2;;
                --seed) seed=$2;;
                --memory-ceiling-mib) memory_ceiling_mib=$2;;
                --timeout-ceiling) timeout_ceiling=$2;;
                --results) results=$2;;
                --query-driver) query_driver=$2;;
                --operation-driver) operation_driver=$2;;
                --prior-results) prior_results=$2;;
            esac
            shift 2;;
        --no-real) real=no; shift;;
        *) printf 'unknown option: %s\n' "$1" >&2; exit 2;;
    esac
done
for count in "$seed" "$memory_ceiling_mib" "$timeout_ceiling"; do
    [[ $count =~ ^[1-9][0-9]*$ ]] || { echo 'numbers must be positive decimal integers' >&2; exit 2; }
done
compiler=$(realpath "$compiler")
[[ -x $compiler ]] || { echo 'compiler is not executable' >&2; exit 2; }
if [[ -n $query_driver ]]; then query_driver=$(realpath "$query_driver"); [[ -x $query_driver ]]; fi
if [[ -n $operation_driver ]]; then operation_driver=$(realpath "$operation_driver"); [[ -x $operation_driver ]]; fi
IFS=, read -r -a sizes <<< "$rungs"
previous_size=0
for size in "${sizes[@]}"; do
    [[ $size =~ ^[1-9][0-9]*$ ]] && ((size >= 100 && size > previous_size)) || {
        echo 'rungs must increase and contain at least 100 lines' >&2; exit 2;
    }
    previous_size=$size
done
umask 077
mkdir -p target "$(dirname "$results")"
temporary=$(mktemp -d "$scale_root/target/scale.XXXXXX")
authority_directory=$(mktemp -d /tmp/lkjscript-scale-key.XXXXXX)
printf '%s\n' "$scale_root" > "$temporary/owner-repository"
printf '%s\n' "$BASHPID" > "$temporary/owner-pid"
printf '%s\n' "$authority_directory" > "$temporary/owner-key"
printf '%s\n' "$temporary" > "$authority_directory/owner-run"
printf 'run_directory=%s\n' "$temporary"
trap 'rm -rf "$temporary" "$authority_directory"' EXIT
trap 'exit 130' HUP INT TERM
key=$(XDG_STATE_HOME="$authority_directory" sh tools/store-key.sh)
if [[ -n $prior_results ]]; then cp "$prior_results" "$temporary/prior.tsv"; fi
load() { awk '{printf "%s,%s,%s", $1, $2, $3}' /proc/loadavg; }
preflight() {
    local wanted=$1 free
    free=$(df -Pk "$temporary" | awk 'NR==2 {printf "%.0f", $4*1024}')
    if ((free < wanted)); then
        printf 'disk refusal: need %s bytes, available %s bytes\n' "$wanted" "$free" >&2
        return 1
    fi
}
# External experimental bounds, not a language or tool memory policy.
initial_limit_mib=2048
((initial_limit_mib <= memory_ceiling_mib)) || initial_limit_mib=$memory_ceiling_mib
preflight $((128 * 1024 * 1024)) || exit 1
cp -R src "$temporary/analysis"
cp bench/scale.lkj "$temporary/analysis/main.lkj"
build_before=$(load)
build_started=$(date +%s%N)
prlimit --as=$((initial_limit_mib * 1024 * 1024)) -- timeout --kill-after=10s "${timeout_ceiling}s" \
    "$compiler" build analysis/main.lkj --source-dir "$temporary" --no-store -o "$temporary/scale"
build_after=$(load)
build_elapsed=$(awk -v first="$build_started" -v last="$(date +%s%N)" 'BEGIN {printf "%.6f", (last-first)/1000000000}')
printf 'generator build: %s s, load %s -> %s\n' "$build_elapsed" "$build_before" "$build_after"
census_before=$(load)
census_started=$(date +%s%N)
prlimit --as=$((initial_limit_mib * 1024 * 1024)) -- timeout --kill-after=10s "${timeout_ceiling}s" \
    "$temporary/scale" shape "$scale_root" > "$temporary/shape"
census_after=$(load)
census_elapsed=$(awk -v first="$census_started" -v last="$(date +%s%N)" 'BEGIN {printf "%.6f", (last-first)/1000000000}')
printf 'source census: %s s, load %s -> %s\n' "$census_elapsed" "$census_before" "$census_after"
awk '$1 != "function" && $1 != "type" && $1 != "module" {print}' "$temporary/shape"
cp "$temporary/shape" "$results.shape"
printf 'rung\tlines\tphase\tstatus\twall_s\tuser_s\tkernel_s\trss_kib\tmemory_limit_mib\tstore_bytes\timage_bytes\tinstances\tload_before\tload_after\tstore_budget_bytes\n' > "$results"
: > "$results.phases"
printf 'compiler_sha256=%s seed=%s memory_ceiling_mib=%s timeout_ceiling_s=%s\n' \
    "$(sha256sum "$compiler" | awk '{print $1}')" "$seed" "$memory_ceiling_mib" "$timeout_ceiling"
printf 'rung phase status wall_s cpu_s rss_kib memory_limit_mib store_bytes image_bytes instances load_before load_after\n'
record_skipped() {
    printf '%s\t%s\t%s\t%s\t-\t-\t-\t-\t%s\t-\t-\t-\t-\t-\t%s\n' \
        "$1" "$2" "$3" "$4" "$limit_mib" "$store_budget" >> "$results"
    printf '%s %s %s memory_limit_mib=%s\n' "$1" "$3" "$4" "$limit_mib"
}
measure() {
    local label=$1 line_count=$2 phase=$3
    shift 3
    local before after status=0 image_bytes=0 store_bytes=0 statistics observed outcome=ok cpu
    before=$(load)
    printf 'rung=%s\nlines=%s\nphase=%s\nload_before=%s\nstarted_ns=%s\nmemory_limit_mib=%s\nwatchdog_seconds=%s\nsource=%s\nstore=%s\nimage=%s\nresults=%s\n' \
        "$label" "$line_count" "$phase" "$before" "$(date +%s%N)" "$limit_mib" "$deadline" "$tree" "$store" "$image" "$results" > "$temporary/checkpoint"
    /usr/bin/time -f '%e %U %S %M' -o "$temporary/time" \
        prlimit --as=$((limit_mib * 1024 * 1024)) -- timeout --kill-after=10s "${deadline}s" "$@" \
        > "$temporary/stdout" 2> "$temporary/stderr" || status=$?
    after=$(load)
    read -r wall user kernel rss < <(tail -n 1 "$temporary/time")
    if [[ -f $image ]]; then image_bytes=$(wc -c < "$image"); fi
    if [[ -d $store ]]; then store_bytes=$(du -sb "$store" | awk '{print $1}'); fi
    statistics=$(sed -n '/^LKJSCRIPT_STATS /p' "$temporary/stderr" | tail -n 1)
    observed=$(printf '%s\n' "$statistics" | sed -n 's/.*"ir_functions":\([0-9][0-9]*\).*/\1/p')
    if [[ -n $observed ]]; then instances=$observed; fi
    if ((status != 0)); then
        outcome="exit_$status"
        if ((status == 124 || status == 137)); then outcome="watchdog_or_signal_$status"; fi
        if rg -q 'R0006|R0008|ENOMEM|allocation failed|Cannot allocate memory' "$temporary/stderr"; then outcome="memory_refusal_$status"; fi
    fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$label" "$line_count" "$phase" "$outcome" "$wall" "$user" "$kernel" "$rss" \
        "$limit_mib" "$store_bytes" "$image_bytes" "$instances" "$before" "$after" "$store_budget" >> "$results"
    cpu=$(awk -v user="$user" -v kernel="$kernel" 'BEGIN {printf "%.3f", user+kernel}')
    printf '%s %s %s %s %s %s %s %s %s %s %s %s\n' "$label" "$phase" "$outcome" \
        "$wall" "$cpu" "$rss" "$limit_mib" "$store_bytes" "$image_bytes" "$instances" "$before" "$after"
    if [[ -n $statistics ]]; then printf '%s %s %s\n' "$label" "$phase" "$statistics" >> "$results.phases"; fi
    if ((status != 0)); then head -c 2048 "$temporary/stderr" >&2; return 1; fi
}
build() {
    measure "$label" "$line_count" "$1" "$compiler" build "$source_path" --source-dir "$tree" --stats \
        --store "$store" --store-key "$key" --store-bytes "$store_budget" -o "$image" "${extra[@]}"
}
# Never edit the actual compiler source.
if [[ $real == yes ]]; then
    label=compiler
    line_count=$(awk '$1=="source_lines" {print $2}' "$temporary/shape")
    tree=$scale_root
    source_path=src/main.lkj
    store="$temporary/real-store"
    mkdir "$store"
    image="$temporary/real-image"
    limit_mib=$initial_limit_mib
    deadline=$timeout_ceiling
    store_budget=268435456
    instances=-
    extra=(--library-dir "$scale_root/library" --runtime-source "$scale_root/library/runtime.lkj")
    if build cold; then build unchanged || true; fi
    rm -rf "$store" "$image"
fi
previous_lines=0 previous_rss=0 previous_wall=0 previous_store=0
memory_exponent=1
if [[ -n $prior_results ]]; then
    read -r previous_lines previous_rss previous_wall previous_store < <(
        awk -F '\t' '$1!="compiler" && $3=="cold" && $4=="ok" {row=$2 " " $8 " " $5 " " $10} END {print row}' "$temporary/prior.tsv"
    )
    ((previous_lines > 0 && previous_rss > 0)) || { echo 'prior results contain no successful cold rung' >&2; exit 2; }
    printf 'resource prediction from %s: %s lines, %s KiB peak, %s s, %s stored bytes\n' "$prior_results" "$previous_lines" "$previous_rss" "$previous_wall" "$previous_store"
fi
stopped=no
for line_count in "${sizes[@]}"; do
    label=$line_count
    if ((previous_lines == 0)); then
        limit_mib=$initial_limit_mib
        deadline=120
        store_budget=268435456
    else
        limit_mib=$(awk -v rss="$previous_rss" -v old="$previous_lines" -v new="$line_count" -v exponent="$memory_exponent" \
            'BEGIN {printf "%.0f", (rss*(new/old)^exponent*2+262144)/1024+1}')
        deadline=$(awk -v elapsed="$previous_wall" -v old="$previous_lines" -v new="$line_count" \
            'BEGIN {printf "%.0f", elapsed*(new/old)*3+120}')
        store_budget=$(awk -v bytes="$previous_store" -v old="$previous_lines" -v new="$line_count" \
            'BEGIN {value=bytes*(new/old)*2;if(value<268435456)value=268435456;printf "%.0f",value}')
    fi
    ((limit_mib <= memory_ceiling_mib)) || limit_mib=$memory_ceiling_mib
    ((deadline <= timeout_ceiling)) || deadline=$timeout_ceiling
    ((deadline >= 120)) || deadline=120
    if [[ $stopped == yes ]]; then record_skipped "$label" "$line_count" cold not_run_after_prior_refusal; continue; fi
    predicted_source=$(awk -v count="$line_count" '$1=="source_lines"{lines=$2}$1=="source_bytes"{bytes=$2} END {printf "%.0f",count*bytes/lines*2}' "$temporary/shape")
    if ! preflight $((predicted_source * 4 + store_budget * 2 + 268435456)); then
        record_skipped "$label" "$line_count" cold disk_refusal
        stopped=yes
        continue
    fi
    tree="$temporary/source"
    store="$temporary/store"
    mkdir "$tree" "$store"
    image="$temporary/image"
    source_path=main.lkj
    instances=-
    extra=()
    generator_before=$(load)
    printf 'rung=%s\nphase=generation\nload_before=%s\nmemory_limit_mib=%s\nwatchdog_seconds=%s\nsource=%s\nresults=%s\n' \
        "$label" "$generator_before" "$limit_mib" "$deadline" "$tree" "$results" > "$temporary/checkpoint"
    generator_started=$(date +%s%N)
    if ! prlimit --as=$((limit_mib * 1024 * 1024)) -- timeout --kill-after=10s "${deadline}s" \
        "$temporary/scale" generate "$tree" "$line_count" "$seed" < "$temporary/shape" > "$temporary/workload"; then
        record_skipped "$label" "$line_count" generation refused
        stopped=yes
        rm -rf "$tree" "$store" "$image"
        continue
    fi
    generator_after=$(load)
    generator_elapsed=$(awk -v first="$generator_started" -v last="$(date +%s%N)" 'BEGIN {printf "%.6f", (last-first)/1000000000}')
    printf '%s generation %s s load %s -> %s\n' "$label" "$generator_elapsed" "$generator_before" "$generator_after"
    cat "$temporary/workload"
    actual_lines=$(find "$tree" -name '*.lkj' -type f -exec cat {} + | wc -l)
    ((actual_lines == line_count)) || { echo 'generated line count differs' >&2; exit 1; }
    if ! build cold; then stopped=yes; rm -rf "$tree" "$store" "$image"; continue; fi
    cold_rss=$rss cold_wall=$wall
    cold_store=$(du -sb "$store" | awk '{print $1}')
    cp "$image" "$temporary/baseline-image"
    if ! build unchanged; then stopped=yes; rm -rf "$tree" "$store" "$image"; continue; fi
    cmp "$temporary/baseline-image" "$image"
    # Explicit projection edits until an identity transaction interface is available.
    sed 's/value \^ 1u64/value ^ 2u64/' "$tree/edit.lkj" > "$temporary/edit"
    mv "$temporary/edit" "$tree/edit.lkj"
    if ! build one_body; then stopped=yes; rm -rf "$tree" "$store" "$image"; continue; fi
    if cmp -s "$temporary/baseline-image" "$image"; then echo 'body edit left image unchanged' >&2; exit 1; fi
    cp "$image" "$temporary/body-image"
    if ! measure "$label" "$line_count" body_control "$compiler" build main.lkj --source-dir "$tree" --stats --no-store -o "$image"; then
        stopped=yes; rm -rf "$tree" "$store" "$image"; continue
    fi
    cmp "$temporary/body-image" "$image"
    sed -e 's/target(value: u64)/target(value: u64, extra: u64)/' -e 's/value \^ 2u64/value ^ extra ^ 2u64/' \
        "$tree/edit.lkj" > "$temporary/edit"
    mv "$temporary/edit" "$tree/edit.lkj"
    dependents=0
    while IFS= read -r file; do
        sites=$(awk '{n+=gsub(/crate::edit::target\(value\)/,"")} END {print n+0}' "$file")
        dependents=$((dependents + sites))
        sed 's/crate::edit::target(value)/crate::edit::target(value, 1u64)/g' "$file" > "$temporary/edit"
        mv "$temporary/edit" "$file"
    done < <(rg -l 'crate::edit::target\(value\)' "$tree" -g '*.lkj' | sort)
    expected=$(awk '$1=="signature_direct_dependents" {print $2}' "$temporary/workload")
    ((dependents == expected)) || { echo 'signature dependent count differs' >&2; exit 1; }
    printf '%s signature direct_dependents=%s\n' "$label" "$dependents"
    if ! build signature; then stopped=yes; rm -rf "$tree" "$store" "$image"; continue; fi
    cp "$image" "$temporary/signature-image"
    if ! measure "$label" "$line_count" signature_control "$compiler" build main.lkj --source-dir "$tree" --stats --no-store -o "$image"; then
        stopped=yes; rm -rf "$tree" "$store" "$image"; continue
    fi
    cmp "$temporary/signature-image" "$image"
    if [[ -n $query_driver ]]; then
        "$query_driver" "$compiler" "$tree" "$store" "$key" "$line_count" "$limit_mib" "$deadline" >> "$results.queries"
    else record_skipped "$label" "$line_count" queries pending_semantic_interface; fi
    if [[ -n $operation_driver ]]; then
        measure "$label" "$line_count" rename "$operation_driver" "$compiler" "$tree" "$store" "$key" || stopped=yes
    else record_skipped "$label" "$line_count" rename pending_transaction_interface; fi
    if ((previous_lines > 0)); then
        memory_exponent=$(awk -v old="$previous_rss" -v new="$cold_rss" -v a="$previous_lines" -v b="$line_count" \
            'BEGIN {power=log(new/old)/log(b/a);if(power<1)power=1;printf "%.6f",power}')
    fi
    previous_lines=$line_count previous_rss=$cold_rss previous_wall=$cold_wall previous_store=$cold_store
    rm -rf "$tree" "$store" "$image" "$temporary/baseline-image" "$temporary/body-image" "$temporary/signature-image"
done
printf 'exponents (successive measured rungs; missing rows are not measurements)\n'
awk -F '\t' '
    NR>1 && $1!="compiler" && $4=="ok" && $5>0 && $6+$7>0 {
        phase=$3
        if (prior_lines[phase]>0) {
            ratio=$2/prior_lines[phase]
            printf "%s %s->%s wall=%.4f cpu=%.4f rss=%.4f\n",phase,prior_lines[phase],$2,
                log($5/prior_wall[phase])/log(ratio),log(($6+$7)/prior_cpu[phase])/log(ratio),
                log($8/prior_rss[phase])/log(ratio)
        }
        prior_lines[phase]=$2;prior_wall[phase]=$5;prior_cpu[phase]=$6+$7;prior_rss[phase]=$8
    }
' "$results"
printf 'results %s; source claims %s; compiler phases %s\n' "$results" "$results.shape" "$results.phases"
