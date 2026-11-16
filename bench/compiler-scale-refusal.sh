#!/usr/bin/env bash
# Prospective ordinary-build cohort; execution needs a committed root release.
set -euo pipefail
export LC_ALL=C
cd "$(dirname "$0")/.."
repository=$(pwd -P)
helper="$repository/bench/compiler-scale-refusal.pl"
usage() {
    echo 'usage: compiler-scale-refusal.sh run RELEASE.json PRODUCER NEW_EVIDENCE'
    echo '       compiler-scale-refusal.sh cleanup RUN_DIRECTORY'
}
if [[ ${1-} == cleanup ]]; then
    (($# == 2)) || { usage >&2; exit 2; }
    candidate=$(realpath -e "$2")
    [[ $candidate == "$repository"/target/compiler-scale-refusal-run.* &&
       ${candidate#"$repository/target/"} != */* && ! -L $2 ]] || exit 2
    [[ -f $candidate/owner-repository && $(cat "$candidate/owner-repository") == "$repository" ]] || exit 2
    owner_pid=$(cat "$candidate/owner-pid")
    [[ $owner_pid =~ ^[1-9][0-9]*$ ]] || exit 2
    if kill -0 "$owner_pid" 2>/dev/null; then
        echo 'owner is still alive; cleanup refused' >&2; exit 2
    fi
    key_directory=$(cat "$candidate/owner-key")
    [[ $key_directory == /tmp/compiler-scale-refusal-key.* &&
       ${key_directory#/tmp/} != */* && ! -L $key_directory ]] || exit 2
    [[ -f $key_directory/owner-run && $(cat "$key_directory/owner-run") == "$candidate" ]] || exit 2
    evidence=$(realpath -e "$(cat "$candidate/owner-evidence")")
    [[ $evidence == "$repository"/target/compiler-scale-refusal-evidence.* &&
       ${evidence#"$repository/target/"} != */* && ! -L $evidence ]] || exit 2
    cleanup_pid=$BASHPID
    process_list=$(ps -eo pid=,ppid=,args=) || { echo 'process inventory failed; cleanup refused' >&2; exit 2; }
    # Exclude only this cleanup and its ancestors. Other matching live users,
    # including descendants and unrelated awk processes, still refuse cleanup.
    live_references=$(printf '%s\n' "$process_list" | awk -v run="$candidate" -v key="$key_directory" -v self="$cleanup_pid" '
        { parents[$1]=$2; commands[$1]=$0 }
        END {
            current=self;
            while (current>0) {
                if (ancestors[current] || !(current in parents)) exit 2;
                ancestors[current]=1;
                current=parents[current];
            }
            for (pid in commands)
                if (!ancestors[pid] && (index(commands[pid],run)>0 || index(commands[pid],key)>0))
                    print pid;
        }') || { echo 'cleanup ancestry could not be established; cleanup refused' >&2; exit 2; }
    if [[ -n $live_references ]]; then
        echo 'a process still refers to the run or key; cleanup refused' >&2; exit 2
    fi
    recovery="$evidence/cleanup"
    mkdir "$recovery"
    cp "$candidate"/owner-* "$recovery/"
    for store in "$candidate"/store-100000-build "$candidate"/store-1000000-build "$candidate"/store-1000000-check; do
        if [[ -d $store && ! -L $store ]]; then du -sb "$store" >> "$recovery/store-apparent-bytes"; fi
    done
    printf 'cleanup_started_utc=%s\nrun=%s\nkey_directory=%s\n' "$(date -u +%FT%TZ)" "$candidate" "$key_directory" > "$recovery/action"
    rm -rf -- "$candidate" "$key_directory"
    printf 'cleanup_completed_utc=%s\n' "$(date -u +%FT%TZ)" >> "$recovery/action"
    printf 'removed owned run and key; retained evidence %s\n' "$evidence"
    exit 0
fi
[[ ${1-} == run && $# == 4 ]] || { usage >&2; exit 2; }
release=$(realpath -e "$2")
producer_input=$(realpath -e "$3")
evidence_argument=$4
[[ $release == "$repository"/bench/* && ! -L $2 ]] || { echo 'release must be a regular bench file' >&2; exit 2; }
release_relative=${release#"$repository/"}
git ls-files --error-unmatch "$release_relative" > /dev/null
git diff --quiet HEAD -- "$release_relative" bench/compiler-scale-refusal-plan.md \
    bench/compiler-scale-refusal.sh bench/compiler-scale-refusal.pl
release_info=$(perl "$helper" release "$repository" "$release" "$producer_input")
mapfile -t released <<< "$release_info"
producer_sha=${released[0]}
source_revision=${released[1]}
release_id=${released[2]}
git cat-file -e "$source_revision^{commit}"
[[ ! -e $evidence_argument && ! -L $evidence_argument ]] || { echo 'refusing existing evidence' >&2; exit 2; }
mkdir -p "$repository/target"
evidence=$(realpath -m "$evidence_argument")
[[ $evidence == "$repository"/target/compiler-scale-refusal-evidence.* &&
   ${evidence#"$repository/target/"} != */* ]] || { echo 'evidence must be a new target/compiler-scale-refusal-evidence.NAME' >&2; exit 2; }
umask 077
mkdir "$evidence"
cp "$release" "$evidence/release.json"
sha256sum "$release" bench/compiler-scale-refusal-plan.md bench/compiler-scale-refusal.sh \
    bench/compiler-scale-refusal.pl bench/scale.lkj > "$evidence/frozen-inputs.sha256"
printf 'release_id=%s\nsource_revision=%s\nproducer_sha256=%s\n' "$release_id" "$source_revision" "$producer_sha" > "$evidence/protocol.meta"
trap 'status=$?; trap - EXIT; printf "supervisor_exit=%s\ncompleted_utc=%s\n" "$status" "$(date -u +%FT%TZ)" >> "$evidence/completion"; exit "$status"' EXIT
trap 'exit 130' HUP INT TERM
load() { awk '{printf "%s,%s,%s",$1,$2,$3}' /proc/loadavg; }
primary_started=no
primary_resource_refusal=no
check_started=no
stop() {
    local reason=$1 higher_reason=$1
    if [[ $primary_resource_refusal == yes ]]; then higher_reason=after_one_million_resource_refusal; fi
    if [[ $primary_started == no ]]; then
        printf '1000000\tnot_run\t%s\t-\t-\t-\t-\n' "$reason" >> "$evidence/skipped.tsv"
    fi
    if [[ $check_started == no && ! -e $evidence/localization.tsv ]]; then
        printf '1000000-check\tnot_run\t%s\n' "$reason" > "$evidence/localization.tsv"
    fi
    printf '%s\n' "$reason" > "$evidence/stop-reason"
    printf '10000000\tnot_run\t%s\t-\t-\t-\t-\n' "$higher_reason" >> "$evidence/skipped.tsv"
    exit 1
}
# These are experiment bounds, not compiler/runtime resource policy.
# Space covers three 2 GiB stores plus publication headroom and source/setup.
free=$(df -Pk "$repository/target" | awk 'NR==2 {printf "%.0f",$4*1024}')
available_kib=$(awk '$1=="MemAvailable:" {print $2}' /proc/meminfo)
printf 'utc=%s\nload=%s\nfree_bytes=%s\nrequired_free_bytes=17179869184\nmem_available_kib=%s\nrequired_mem_available_kib=16777216\n' \
    "$(date -u +%FT%TZ)" "$(load)" "$free" "$available_kib" > "$evidence/preflight"
printf 'lines\tstatus\treason\twall_s\tcpu_s\trss_kib\tload\n' > "$evidence/skipped.tsv"
[[ $free =~ ^[0-9]+$ && $available_kib =~ ^[0-9]+$ ]] || stop invalid_preflight
((free >= 17179869184 && available_kib >= 16777216)) || stop host_headroom_refusal
fixture=$(mktemp -d "$repository/target/compiler-scale-refusal-run.XXXXXX")
key_directory=$(mktemp -d /tmp/compiler-scale-refusal-key.XXXXXX)
key="$key_directory/key"
printf '%s\n' "$repository" > "$fixture/owner-repository"
printf '%s\n' "$BASHPID" > "$fixture/owner-pid"
printf '%s\n' "$evidence" > "$fixture/owner-evidence"
printf '%s\n' "$key_directory" > "$fixture/owner-key"
printf '%s\n' "$fixture" > "$key_directory/owner-run"
printf '%s\n' "$fixture" > "$evidence/fixture.path"
head -c 32 /dev/urandom > "$key"
cp "$producer_input" "$fixture/producer"
chmod 500 "$fixture/producer"
producer="$fixture/producer"
[[ $(sha256sum "$producer" | awk '{print $1}') == "$producer_sha" ]] || stop producer_copy_mismatch
printf 'phase\tkind\toutcome\twall_s\tuser_s\tkernel_s\trss_kib\twrapper_exit\tgnu_exit\tmemory_limit_bytes\ttimeout_seconds\tload_before\tload_after\tcapture_status\n' > "$evidence/timings.tsv"
run_phase() {
    local label=$1 kind=$2 directory=$3 memory=$4 deadline=$5 input=$6
    shift 6
    local before after status=0 capture_status=0
    printf '%s\n' "$directory" > "$evidence/$label.cwd" || return 125
    printf '%q ' "$@" > "$evidence/$label.command" || return 125
    printf '\n' >> "$evidence/$label.command" || return 125
    before=$(load) || return 125
    printf 'phase=%s\nkind=%s\nstarted_utc=%s\nload_before=%s\nmemory_limit_bytes=%s\ntimeout_seconds=%s\nkill_grace_seconds=10\nstdin=%s\n' \
        "$label" "$kind" "$(date -u +%FT%TZ)" "$before" "$memory" "$deadline" "$input" > "$evidence/$label.meta" || return 125
    printf '%s\n' "$label" > "$evidence/current-phase" || return 125
    # time and this shell are outside RLIMIT_AS and the timeout process group.
    # All raw records live outside the disposable run, before child execution.
    (cd "$directory" && /usr/bin/time -q -f '%e\t%U\t%S\t%M\t%x' -o "$evidence/$label.time" \
        prlimit --as="$memory" -- timeout --kill-after=10s "${deadline}s" "$@") \
        < "$input" > "$evidence/$label.stdout" 2> "$evidence/$label.stderr" || status=$?
    after=$(load) || capture_status=1
    printf 'completed_utc=%s\nload_after=%s\nwrapper_exit=%s\n' "$(date -u +%FT%TZ)" "$after" "$status" >> "$evidence/$label.meta" || capture_status=1
    printf '%s\n' "$status" > "$evidence/$label.status" || capture_status=1
    perl "$helper" phase "$evidence" "$label" "$status" "$memory" "$deadline" "$kind" \
        > "$evidence/$label.result.json" 2> "$evidence/$label.capture-error" || capture_status=$?
    if ((capture_status == 0)); then
        perl "$helper" row "$evidence/$label.result.json" >> "$evidence/timings.tsv" || capture_status=$?
        if [[ $(perl "$helper" field "$evidence/$label.result.json" capture_status) != complete ]]; then capture_status=1; fi
    fi
    printf '%s\n' "$capture_status" > "$evidence/$label.capture-status" || return 125
    printf 'phase=%s wrapper_exit=%s capture_exit=%s load=%s->%s\n' "$label" "$status" "$capture_status" "$before" "$after"
    ((capture_status == 0)) || return 125
    return "$status"
}
inventory() {
    local tree=$1 output=$2
    (cd "$tree" && find . -type f -print0 | sort -z | xargs -0 sha256sum) > "$output"
}
retained_bytes() {
    local label=$1 store=$2 image=$3
    du -sb "$store" > "$evidence/$label.store-apparent-bytes" || return 125
    if [[ -f $image ]]; then
        wc -c < "$image" > "$evidence/$label.image-bytes" || return 125
        sha256sum "$image" > "$evidence/$label.image.sha256"
    else
        printf 'absent\n' > "$evidence/$label.image-bytes"
    fi
}
validate_build() {
    local label=$1 kind=$2
    perl "$helper" validate-build "$evidence/$label.result.json" "$kind" \
        > "$evidence/$label.validation.json" 2> "$evidence/$label.validation-error"
}
# Reproduce the accepted generator closure, not the evolving compiler checkout.
factory_revision=a0da7d7fd98e0897856d7c94fe5622f949509c7f
shape_revision=0b30ca8987bd4ba343ab45ab01751b6721c7151a
shape_sha=0457f8f9424882d2e3160a568f280193d49b9b046a5fae2d6e2d0b6da047072c
generator_sha=557d6b124fe739f17c678c77b92e5f6259d87ebca69c2a56aca411071c5ff1e9
factory_producer_sha=adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7
control_manifest_sha=315f2c153190f8f1f0b12fa0bd8d4f0200f0b17c407f2e018617f278f820794e
accepted="$repository/bench/compiler-query-container-data/measurement-1"
[[ $(sha256sum bench/scale.lkj | awk '{print $1}') == "$generator_sha" ]] || stop generator_source_mismatch
[[ $(sha256sum "$accepted/shape" | awk '{print $1}') == "$shape_sha" ]] || stop shape_mismatch
[[ $(sha256sum "$accepted/100000-S0-inputs.sha256" | awk '{print $1}') == "$control_manifest_sha" ]] || stop control_manifest_mismatch
cp "$accepted/shape" "$evidence/shape"
printf 'factory_revision=%s\nshape_revision=%s\nshape_sha256=%s\ngenerator_source_sha256=%s\nfactory_producer_sha256=%s\nseed=20261116\nspecimen_inline_limit=0\nstore_budget_bytes=2147483648\n' \
    "$factory_revision" "$shape_revision" "$shape_sha" "$generator_sha" "$factory_producer_sha" >> "$evidence/protocol.meta"
git show 13c4e6fd:seed/lkjscript.gz | gzip -dc > "$fixture/factory-producer"
[[ $(sha256sum "$fixture/factory-producer" | awk '{print $1}') == "$factory_producer_sha" ]] || stop factory_producer_mismatch
chmod 500 "$fixture/factory-producer"
mkdir "$fixture/factory" "$fixture/frozen"
git archive "$factory_revision" src library | tar -x -C "$fixture/factory"
git archive "$shape_revision" library | tar -x -C "$fixture/frozen"
cp bench/scale.lkj "$fixture/factory/src/main.lkj"
inventory "$fixture/factory" "$evidence/factory-inputs.sha256"
inventory "$fixture/frozen" "$evidence/specimen-library-inputs.sha256"
run_phase generator-build setup "$fixture/factory" 4294967296 900 /dev/null \
    "$fixture/factory-producer" build src/main.lkj --source-dir "$fixture/factory" \
    --library-dir "$fixture/factory/library" --runtime-source "$fixture/factory/library/runtime.lkj" \
    --inline-limit 0 --no-store -o "$fixture/generator" || stop generator_build_failed
sha256sum "$fixture/generator" > "$evidence/generator.sha256"
tree="$fixture/source-100000"
store="$fixture/store-100000-build"
image="$fixture/image-100000-build"
mkdir "$tree" "$store"
run_phase 100000-generate setup "$tree" 4294967296 900 "$evidence/shape" \
    "$fixture/generator" generate "$tree" 100000 20261116 || stop control_generation_failed
inventory "$tree" "$evidence/100000-inputs.sha256"
cmp "$evidence/100000-inputs.sha256" "$accepted/100000-S0-inputs.sha256" || stop control_source_mismatch
cp "$tree/workload.txt" "$evidence/100000-workload.txt"
control_status=0
run_phase 100000-cold build "$tree" 4294967296 900 /dev/null env LKJSCRIPT_STORE_IO=1 \
    "$producer" build main.lkj --source-dir "$tree" --library-dir "$fixture/frozen/library" \
    --runtime-source "$fixture/frozen/library/runtime.lkj" --store "$store" --store-key "$key" \
    --store-bytes 2147483648 --inline-limit 0 --stats -o "$image" || control_status=$?
retained_bytes 100000-cold "$store" "$image"
((control_status == 0)) || stop fresh_100k_control_failed
validate_build 100000-cold build || stop fresh_100k_control_invalid
perl "$helper" predict "$evidence/100000-cold.result.json" > "$evidence/1m-bound.json"
memory=$(perl "$helper" field "$evidence/1m-bound.json" memory_limit_bytes)
deadline=$(perl "$helper" field "$evidence/1m-bound.json" timeout_seconds)
# A second headroom check may refuse; it never enlarges the frozen 8 GiB ceiling.
free=$(df -Pk "$repository/target" | awk 'NR==2 {printf "%.0f",$4*1024}')
available_kib=$(awk '$1=="MemAvailable:" {print $2}' /proc/meminfo)
printf 'utc=%s\nload=%s\nfree_bytes=%s\nmem_available_kib=%s\n' "$(date -u +%FT%TZ)" "$(load)" "$free" "$available_kib" > "$evidence/1m-preflight"
((free >= 17179869184 && available_kib >= 16777216)) || stop one_million_headroom_refusal
tree="$fixture/source-1000000"
store="$fixture/store-1000000-build"
image="$fixture/image-1000000-build"
mkdir "$tree" "$store"
run_phase 1000000-generate setup "$tree" 4294967296 900 "$evidence/shape" \
    "$fixture/generator" generate "$tree" 1000000 20261116 || stop one_million_generation_failed
inventory "$tree" "$evidence/1000000-inputs.sha256"
cp "$tree/workload.txt" "$evidence/1000000-workload.txt"
primary_status=0
primary_started=yes
run_phase 1000000-cold build "$tree" "$memory" "$deadline" /dev/null env LKJSCRIPT_STORE_IO=1 \
    "$producer" build main.lkj --source-dir "$tree" --library-dir "$fixture/frozen/library" \
    --runtime-source "$fixture/frozen/library/runtime.lkj" --store "$store" --store-key "$key" \
    --store-bytes 2147483648 --inline-limit 0 --stats -o "$image" || primary_status=$?
retained_bytes 1000000-cold "$store" "$image"
if ((primary_status == 0)); then
    validate_build 1000000-cold build || stop one_million_build_invalid
    printf '1000000-check\tnot_run\tprimary_build_succeeded\n' > "$evidence/localization.tsv"
    printf '10000000\tnot_run\tunexecuted_by_this_protocol\t-\t-\t-\t-\n' >> "$evidence/skipped.tsv"
    printf 'primary_build_succeeded\n' > "$evidence/outcome"
    exit 0
fi
[[ $(cat "$evidence/1000000-cold.capture-status") == 0 ]] || stop one_million_capture_incomplete
[[ $(perl "$helper" field "$evidence/1000000-cold.result.json" capture_status) == complete ]] || stop one_million_capture_incomplete
[[ $(perl "$helper" field "$evidence/1000000-cold.result.json" outcome) == located_resource_refusal ]] || stop one_million_nonresource_failure
primary_resource_refusal=yes
# This conditional command is a separate cold control, never a retry or warm resume.
(cd "$tree" && sha256sum -c "$evidence/1000000-inputs.sha256") > "$evidence/1000000-source-recheck"
free=$(df -Pk "$repository/target" | awk 'NR==2 {printf "%.0f",$4*1024}')
available_kib=$(awk '$1=="MemAvailable:" {print $2}' /proc/meminfo)
printf 'utc=%s\nload=%s\nfree_bytes=%s\nmem_available_kib=%s\n' "$(date -u +%FT%TZ)" "$(load)" "$free" "$available_kib" > "$evidence/check-preflight"
((free >= 17179869184 && available_kib >= 16777216)) || stop localization_headroom_refusal
check_store="$fixture/store-1000000-check"
mkdir "$check_store"
check_status=0
check_started=yes
run_phase 1000000-check check "$tree" "$memory" "$deadline" /dev/null env LKJSCRIPT_STORE_IO=1 \
    "$producer" check main.lkj --source-dir "$tree" --library-dir "$fixture/frozen/library" \
    --runtime-source "$fixture/frozen/library/runtime.lkj" --store "$check_store" --store-key "$key" \
    --store-bytes 2147483648 --inline-limit 0 --stats || check_status=$?
retained_bytes 1000000-check "$check_store" "$fixture/no-check-image"
if ((check_status == 0)); then
    validate_build 1000000-check check || stop localization_control_invalid
    printf '1000000-check\taccepted\tshared_compile_route_fits_build_specific_difference_remains\n' > "$evidence/localization.tsv"
elif [[ $(cat "$evidence/1000000-check.capture-status") != 0 ]]; then
    printf '1000000-check\tincomplete\tcontrol_capture_incomplete_exact_phase_unknown\n' > "$evidence/localization.tsv"
else
    printf '1000000-check\tfailed\tshared_compile_route_also_failed_exact_phase_unknown\n' > "$evidence/localization.tsv"
fi
printf '10000000\tnot_run\tafter_one_million_resource_refusal\t-\t-\t-\t-\n' >> "$evidence/skipped.tsv"
printf 'primary_resource_refusal_check_exit_%s\n' "$check_status" > "$evidence/outcome"
# A refused primary build remains a failed build, even when its control checks.
exit 1
