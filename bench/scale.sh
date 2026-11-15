#!/usr/bin/env bash
# Source-shape ladder; generated trees are disposable, outside the gate.
set -euo pipefail
cd "$(dirname "$0")/.."
scale_root=$PWD
shape_root=$scale_root
structural=no
rename_compiler=
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
    if ps -eo pid=,args= | awk -v run="$candidate" -v self="$$" -v parent="$PPID" '$1!=self && $1!=parent && index($0,run)>0 && $0!~/awk -v run=/ {found=1} END {exit !found}'; then
        echo 'a command still refers to the benchmark tree; cleanup refused' >&2; exit 2
    fi
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
        --compiler|--rungs|--seed|--memory-ceiling-mib|--timeout-ceiling|--results|--query-driver|--operation-driver|--prior-results|--shape-root|--rename-compiler)
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
                --shape-root) shape_root=$2;;
                --rename-compiler) rename_compiler=$2;;
            esac
            shift 2;;
        --no-real) real=no; shift;;
        --structural) structural=yes; shift;;
        *) printf 'unknown option: %s\n' "$1" >&2; exit 2;;
    esac
done
for count in "$seed" "$memory_ceiling_mib" "$timeout_ceiling"; do
    [[ $count =~ ^[1-9][0-9]*$ ]] || { echo 'numbers must be positive decimal integers' >&2; exit 2; }
done
compiler=$(realpath "$compiler")
shape_root=$(realpath "$shape_root")
[[ -f $shape_root/src/main.lkj && -d $shape_root/library ]] || exit 2
if [[ $structural == yes ]]; then
    [[ -n $query_driver ]] || query_driver="$scale_root/bench/scale-query.sh"
    [[ -n $operation_driver ]] || operation_driver="$scale_root/bench/scale-operation.sh"
fi
[[ -x $compiler ]] || { echo 'compiler is not executable' >&2; exit 2; }
compiler_sha256=$(sha256sum "$compiler" | cut -d ' ' -f 1)
if [[ -z $rename_compiler ]]; then rename_compiler=$compiler; else rename_compiler=$(realpath "$rename_compiler"); fi
[[ -x $rename_compiler ]] || { echo 'rename compiler is not executable' >&2; exit 2; }
rename_sha256=$(sha256sum "$rename_compiler" | cut -d ' ' -f 1)
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
[[ ! -e $results ]] || { echo "results already exist: $results" >&2; exit 2; }
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
    "$temporary/scale" shape "$shape_root" > "$temporary/shape"
census_after=$(load)
census_elapsed=$(awk -v first="$census_started" -v last="$(date +%s%N)" 'BEGIN {printf "%.6f", (last-first)/1000000000}')
printf 'source census: %s s, load %s -> %s\n' "$census_elapsed" "$census_before" "$census_after"
awk '$1 != "function" && $1 != "type" && $1 != "module" {print}' "$temporary/shape"
cp "$temporary/shape" "$results.shape"
printf 'compiler_sha256=%s\nshape_root=%s\nshape_sha256=%s\nseed=%s\nstructural=%s\nmemory_ceiling_mib=%s\ntimeout_ceiling_s=%s\n' \
    "$(sha256sum "$compiler" | cut -d ' ' -f 1)" "$shape_root" "$(sha256sum "$temporary/shape" | cut -d ' ' -f 1)" \
    "$seed" "$structural" "$memory_ceiling_mib" "$timeout_ceiling" > "$results.meta"
printf 'rename_compiler_sha256=%s\nrename_compiler=%s\n' "$rename_sha256" "$rename_compiler" >> "$results.meta"
printf 'rung\tlines\tphase\tstatus\twall_s\tuser_s\tkernel_s\trss_kib\tmemory_limit_mib\tstore_bytes\timage_bytes\tinstances\tload_before\tload_after\tstore_budget_bytes\tproducer_sha256\n' > "$results"
: > "$results.phases"
printf 'compiler_sha256=%s seed=%s memory_ceiling_mib=%s timeout_ceiling_s=%s\n' \
    "$(sha256sum "$compiler" | awk '{print $1}')" "$seed" "$memory_ceiling_mib" "$timeout_ceiling"
printf 'rung phase status wall_s cpu_s rss_kib memory_limit_mib store_bytes image_bytes instances load_before load_after\n'
rung_peak=0
rung_store_peak=0
rung_wall_peak=0
record_skipped() {
    printf '%s\t%s\t%s\t%s\t-\t-\t-\t-\t%s\t-\t-\t-\t-\t-\t%s\t%s\n' \
        "$1" "$2" "$3" "$4" "$limit_mib" "$store_budget" "$compiler_sha256" >> "$results"
    printf '%s %s %s memory_limit_mib=%s\n' "$1" "$3" "$4" "$limit_mib"
}
measure() {
    local label=$1 line_count=$2 phase=$3
    shift 3
    local before after status=0 image_bytes=0 store_bytes=0 statistics observed outcome=ok cpu producer=$compiler_sha256
    if [[ $phase == rename ]]; then producer=$rename_sha256; fi
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
    if ((store_bytes > rung_store_peak)); then rung_store_peak=$store_bytes; fi
    statistics=$(sed -n '/^LKJSCRIPT_STATS /p' "$temporary/stderr" | head -n 1)
    observed=$(printf '%s\n' "$statistics" | sed -n 's/.*"ir_functions":\([0-9][0-9]*\).*/\1/p')
    if [[ -n $observed && $observed != 0 ]]; then instances=$observed; fi
    if [[ $rss =~ ^[0-9]+$ ]] && ((rss > rung_peak)); then rung_peak=$rss; fi
    rung_wall_peak=$(awk -v old="$rung_wall_peak" -v observed="$wall" 'BEGIN {print (observed>old?observed:old)}')
    if ((status != 0)); then
        outcome="exit_$status"
        if ((status == 124 || status == 137)); then outcome="watchdog_or_signal_$status"; fi
        if rg -q 'R0006|R0008|ENOMEM|allocation failed|Cannot allocate memory' "$temporary/stderr"; then outcome="memory_refusal_$status"; fi
    fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$label" "$line_count" "$phase" "$outcome" "$wall" "$user" "$kernel" "$rss" \
        "$limit_mib" "$store_bytes" "$image_bytes" "$instances" "$before" "$after" "$store_budget" "$producer" >> "$results"
    cpu=$(awk -v user="$user" -v kernel="$kernel" 'BEGIN {printf "%.3f", user+kernel}')
    printf '%s %s %s %s %s %s %s %s %s %s %s %s\n' "$label" "$phase" "$outcome" \
        "$wall" "$cpu" "$rss" "$limit_mib" "$store_bytes" "$image_bytes" "$instances" "$before" "$after"
    if [[ -n $statistics ]]; then
        sed -n '/^LKJSCRIPT_STATS /p' "$temporary/stderr" | awk -v rung="$label" -v phase="$phase" '{print rung, phase, $0}' >> "$results.phases"
    fi
    if [[ $phase == *_transaction || $phase == rename ]]; then
        printf '%s %s\n' "$label" "$phase" >> "$results.operations"
        cat "$temporary/stdout" >> "$results.operations"
    fi
    if ((status != 0)); then
        printf 'rung=%s phase=%s status=%s producer=%s\n' "$label" "$phase" "$outcome" "$producer" >> "$results.failures"
        cat "$temporary/checkpoint" "$temporary/time" "$temporary/stdout" "$temporary/stderr" >> "$results.failures"
        head -c 2048 "$temporary/stderr" >&2
        return 1
    fi
}
build() {
    local phase=$1
    shift
    measure "$label" "$line_count" "$phase" "$compiler" build "$source_path" --source-dir "$tree" --stats \
        --store "$store" --store-key "$key" --store-bytes "$store_budget" -o "$image" "${extra[@]}" "$@"
}
# Never edit the actual compiler source.
if [[ $real == yes ]]; then
    label=compiler
    line_count=$(awk '$1=="source_lines" {print $2}' "$temporary/shape")
    tree=$shape_root
    source_path=src/main.lkj
    store="$temporary/real-store"
    mkdir "$store"
    image="$temporary/real-image"
    limit_mib=$initial_limit_mib
    deadline=$timeout_ceiling
    store_budget=268435456
    instances=-
    extra=(--library-dir "$shape_root/library" --runtime-source "$shape_root/library/runtime.lkj")
    if build cold; then build unchanged || true; fi
    rm -rf "$store" "$image"
fi
previous_lines=0 previous_rss=0 previous_wall=0 previous_store=0
memory_exponent=1
if [[ -n $prior_results ]]; then
    read -r previous_lines previous_rss previous_wall previous_store < <(
        awk -F '\t' '$1!="compiler" && $3=="cold" && $4=="ok" {lines=$2;wall=$5;stored=$10;peak=$8}
            $1!="compiler" && $2==lines && $4=="ok" {if($8>peak)peak=$8;if($10>stored)stored=$10;if($5>wall)wall=$5}
            END {print lines, peak, wall, stored}' "$temporary/prior.tsv"
    )
    ((previous_lines > 0 && previous_rss > 0)) || { echo 'prior results contain no successful cold rung' >&2; exit 2; }
    printf 'resource prediction from %s: %s lines, %s KiB peak, %s s, %s stored bytes\n' "$prior_results" "$previous_lines" "$previous_rss" "$previous_wall" "$previous_store"
fi
stopped=no
for line_count in "${sizes[@]}"; do
    label=$line_count
    rung_peak=0
    rung_store_peak=0
    rung_wall_peak=0
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
    if [[ -n $query_driver ]] && ((store_budget < 1073741824)); then store_budget=1073741824; fi
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
    if [[ -n $query_driver ]]; then
        if ! build index --index; then stopped=yes; rm -rf "$tree" "$store" "$image"; continue; fi
        snapshot=$(sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]*\) .*/\1/p' "$temporary/stdout")
        [[ $snapshot =~ ^[0-9a-f]{64}$ ]] || { echo 'indexed build omitted snapshot' >&2; exit 1; }
        if ! build index_unchanged --index; then stopped=yes; rm -rf "$tree" "$store" "$image"; continue; fi
        repeated=$(sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]*\) .*/\1/p' "$temporary/stdout")
        [[ $repeated == "$snapshot" ]] || { echo 'unchanged index identity differs' >&2; exit 1; }
        cmp "$temporary/baseline-image" "$image"
        query_status=0
        "$query_driver" "$compiler" "$tree" "$store" "$key" "$line_count" "$limit_mib" "$deadline" "$store_budget" "$snapshot" \
            > "$temporary/queries" 2> "$temporary/query-errors" || query_status=$?
        if [[ -e $results.queries ]]; then tail -n +2 "$temporary/queries" >> "$results.queries"; else cp "$temporary/queries" "$results.queries"; fi
        if ((query_status)); then
            printf 'rung=%s phase=queries status=%s producer=%s\n' "$label" "$query_status" "$compiler_sha256" >> "$results.failures"
            cat "$temporary/query-errors" >> "$results.failures"
            cat "$temporary/query-errors" >&2
            record_skipped "$label" "$line_count" queries "exit_$query_status"
            stopped=yes; rm -rf "$tree" "$store" "$image"; continue
        fi
    else record_skipped "$label" "$line_count" queries pending_semantic_interface; fi
    if [[ $structural == yes ]]; then
        if ! measure "$label" "$line_count" body_transaction "$operation_driver" "$compiler" "$tree" "$store" "$key" body "$store_budget"; then
            stopped=yes; rm -rf "$tree" "$store" "$image"; continue
        fi
    else
        # Historical projection-edit baseline, explicitly not an identity transaction.
        sed 's/value \^ 1u64/value ^ 2u64/' "$tree/edit.lkj" > "$temporary/edit"
        mv "$temporary/edit" "$tree/edit.lkj"
    fi
    if ! build one_body; then stopped=yes; rm -rf "$tree" "$store" "$image"; continue; fi
    if cmp -s "$temporary/baseline-image" "$image"; then echo 'body edit left image unchanged' >&2; exit 1; fi
    cp "$image" "$temporary/body-image"
    if ! measure "$label" "$line_count" body_control "$compiler" build main.lkj --source-dir "$tree" --stats --no-store -o "$image"; then
        stopped=yes; rm -rf "$tree" "$store" "$image"; continue
    fi
    cmp "$temporary/body-image" "$image"
    expected=$(awk '$1=="signature_direct_dependents" {print $2}' "$temporary/workload")
    if [[ $structural == yes ]]; then
        if ! measure "$label" "$line_count" signature_transaction "$operation_driver" "$compiler" "$tree" "$store" "$key" signature "$store_budget"; then
            stopped=yes; rm -rf "$tree" "$store" "$image"; continue
        fi
        printf '%s signature source_dependents=%s transaction=checked\n' "$label" "$expected"
    else
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
    fi
    if ! build signature; then stopped=yes; rm -rf "$tree" "$store" "$image"; continue; fi
    cp "$image" "$temporary/signature-image"
    if ! measure "$label" "$line_count" signature_control "$compiler" build main.lkj --source-dir "$tree" --stats --no-store -o "$image"; then
        stopped=yes; rm -rf "$tree" "$store" "$image"; continue
    fi
    cmp "$temporary/signature-image" "$image"
    if [[ -n $operation_driver ]]; then
        cp -R "$tree" "$temporary/before-rename"
        if ! measure "$label" "$line_count" rename "$operation_driver" "$rename_compiler" "$tree" "$store" "$key" rename "$store_budget"; then
            stopped=yes; rm -rf "$tree" "$store" "$image" "$temporary/before-rename"; continue
        fi
        read -r old_meaning new_meaning < <(sed -n 's/^program level=checked-ir before=\([0-9a-f]*\) after=\([0-9a-f]*\) .*/\1 \2/p' "$temporary/stdout")
        [[ $old_meaning =~ ^[0-9a-f]{64}$ && $old_meaning == "$new_meaning" ]] || { echo 'rename did not prove checked-IR identity' >&2; exit 1; }
        # Deterministic generator spelling is an independent byte-projection oracle.
        # This verification is outside the timed semantic rename, and never edits it.
        while IFS= read -r file; do
            relative=${file#"$tree/"}
            cmp <(sed -e 's/pub fn target(/pub fn target_renamed(/g' -e 's/edit::target(/edit::target_renamed(/g' "$temporary/before-rename/$relative") "$file"
        done < <(rg --files "$tree" -g '*.lkj' | sort)
        printf '%s rename checked_ir=%s source_dependents=%s projection=exact\n' "$label" "$new_meaning" "$expected"
        rm -rf "$temporary/before-rename"
    else record_skipped "$label" "$line_count" rename pending_transaction_interface; fi
    if ((previous_lines > 0)); then
        memory_exponent=$(awk -v old="$previous_rss" -v new="$rung_peak" -v a="$previous_lines" -v b="$line_count" \
            'BEGIN {power=log(new/old)/log(b/a);if(power<1)power=1;printf "%.6f",power}')
    fi
    previous_lines=$line_count previous_rss=$rung_peak previous_wall=$rung_wall_peak previous_store=$rung_store_peak
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
if [[ -e $results.queries ]]; then
    printf 'question exponents (same bounded question, including exact producer hashing)\n'
    awk -F '\t' '
        $1~/^[0-9]+$/ && $2!="discovery" && $7=="ok" && $8>0 {
            name=$2 FS $3 FS $4 FS $5 FS $6
            if (lines[name]>0 && $1>lines[name]) {
                ratio=$1/lines[name]
                printf "%s depth=%s limit=%s %s->%s wall=%.4f", $3,$5,$6,lines[name],$1,log($8/wall[name])/log(ratio)
                if ($18>0 && index_ms[name]>0) printf " lookup=%.4f",log($18/index_ms[name])/log(ratio)
                printf "\n"
            }
            lines[name]=$1;wall[name]=$8;index_ms[name]=$18
        }
    ' "$results.queries"
fi
printf 'results %s; source claims %s; compiler phases %s\n' "$results" "$results.shape" "$results.phases"
