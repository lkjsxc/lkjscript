#!/usr/bin/env bash
# Fixed v2/v3 experiment: compiler-query-container-plan.md and identity addendum.
# No automatic cleanup: stores, keys and images remain owned until proof acceptance.
set -euo pipefail
export LC_ALL=C
cd "$(dirname "$0")/.."
repository=$PWD
(($# == 3)) || { echo 'usage: compiler-query-container.sh BASELINE CANDIDATE NEW_EVIDENCE' >&2; exit 2; }
baseline_input=$(realpath "$1")
candidate_input=$(realpath "$2")
evidence=$3
[[ ! -e $evidence ]] || { echo 'refusing existing evidence directory' >&2; exit 2; }
mkdir -p "$evidence"
evidence=$(realpath "$evidence")
budget=2147483648
baseline_sha=adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7
candidate_sha=eed6ca32edcc182a92c4e697f4702ec685dff8e7778b0374a27552aa9c938947
shape_revision=0b30ca8987bd4ba343ab45ab01751b6721c7151a
factory_revision=a0da7d7fd98e0897856d7c94fe5622f949509c7f
shape_sha=0457f8f9424882d2e3160a568f280193d49b9b046a5fae2d6e2d0b6da047072c
generator_source_sha=557d6b124fe739f17c678c77b92e5f6259d87ebca69c2a56aca411071c5ff1e9
free=$(df -Pk "$repository/target" | awk 'NR==2 {printf "%.0f",$4*1024}')
printf 'free_bytes=%s required_bytes=%s\n' "$free" "$((4*budget+536870912))" > "$evidence/preflight"
((free >= 4*budget+536870912)) || { echo 'free-space refusal' >&2; exit 1; }
[[ $(sha256sum "$baseline_input" | awk '{print $1}') == "$baseline_sha" ]]
[[ $(sha256sum "$candidate_input" | awk '{print $1}') == "$candidate_sha" ]]
[[ $(sha256sum bench/scale.lkj | awk '{print $1}') == "$generator_source_sha" ]]
sha256sum bench/compiler-query-container-plan.md bench/compiler-query-container-identities.md \
    bench/compiler-query-container.sh bench/compiler-query-container-compare.pl \
    bench/compiler-query-container-census.pl bench/scale.lkj \
    src/store.lkj src/semantic_store.lkj src/semantic_query.lkj src/driver.lkj \
    src/query_store_cases.lkj > "$evidence/implementation.sha256"
umask 077
fixture=$(mktemp -d "$repository/target/compiler-query-container-run.XXXXXX")
printf '%s\n' "$fixture" > "$evidence/fixture.path"
printf '%s\n' "$repository" > "$fixture/owner-repository"
printf '%s\n' "$BASHPID" > "$fixture/owner-pid"
printf '%s\n' "$evidence" > "$fixture/owner-evidence"
trap 'status=$?; trap - EXIT; printf "supervisor_status=%s\n" "$status" >> "$evidence/completion"; exit "$status"' EXIT
trap 'exit 130' HUP INT TERM
key_directory=$(mktemp -d /tmp/compiler-query-container-key.XXXXXX)
key="$key_directory/key"
printf '%s\n' "$fixture" > "$key_directory/owner-run"
printf '%s\n' "$key_directory" > "$fixture/owner-key"
head -c 32 /dev/urandom > "$key"
cp "$baseline_input" "$fixture/baseline"
cp "$candidate_input" "$fixture/candidate"
chmod 500 "$fixture/baseline" "$fixture/candidate"
baseline="$fixture/baseline"
candidate="$fixture/candidate"
sha256sum "$baseline" "$candidate" > "$evidence/producer.sha256"
[[ $(sha256sum "$baseline" | awk '{print $1}') == "$baseline_sha" ]]
[[ $(sha256sum "$candidate" | awk '{print $1}') == "$candidate_sha" ]]
baseline_slot=$(perl -MDigest::SHA=sha256_hex -e 'print sha256_hex(pack("H*",$ARGV[0]))' "$baseline_sha")
candidate_slot=$(perl -MDigest::SHA=sha256_hex -e 'print sha256_hex(pack("H*",$ARGV[0]))' "$candidate_sha")
printf 'baseline=%s\ncandidate=%s\nproducer_inline_limit=64\nspecimen_inline_limit=0\nfactory_revision=%s\nshape_revision=%s\nseed=20261116\nmemory_limit_bytes=4294967296\ntimeout_seconds=900\nkill_grace_seconds=10\nstore_budget_bytes=%s\nquery_read_cap=default\n' \
    "$baseline_sha" "$candidate_sha" "$factory_revision" "$shape_revision" "$budget" > "$evidence/protocol.meta"
printf 'phase\twall_s\tuser_s\tkernel_s\trss_kib\texit\twrapper_wall_ns\tload_before\tload_after\n' > "$evidence/timings.tsv"
printf 'phase\trung\tproducer\tstate\tindex_requested\n' > "$evidence/phase-order.tsv"
printf 'rung\tstate\tproducer\tsnapshot\troot_present\tlocator_present\n' > "$evidence/retention.tsv"
printf 'rung\tstate\tkind\tidentity\tbaseline_status\tcandidate_status\tcomparison_status\n' > "$evidence/queries.tsv"
load() { awk '{printf "%s,%s,%s",$1,$2,$3}' /proc/loadavg; }
run_timed() {
    local label=$1 directory=$2; shift 2
    local before after status=0 began ended
    printf '%s\n' "$directory" > "$evidence/$label.cwd"
    printf '%q ' "$@" > "$evidence/$label.command"
    printf '\n' >> "$evidence/$label.command"
    before=$(load)
    printf 'load_before=%s\n' "$before" > "$evidence/$label.meta"
    began=$(date +%s%N)
    (cd "$directory" && /usr/bin/time -q -f '%e\t%U\t%S\t%M\t%x' -o "$evidence/$label.time" \
        prlimit --as=4294967296 -- timeout --foreground --kill-after=10s 900 "$@") \
        > "$evidence/$label.stdout" 2> "$evidence/$label.stderr" || status=$?
    ended=$(date +%s%N)
    after=$(load)
    printf 'load_after=%s\nstatus=%s\nwrapper_wall_ns=%s\n' "$after" "$status" "$((ended-began))" >> "$evidence/$label.meta"
    printf '%s\n' "$status" > "$evidence/$label.status"
    printf '%s\t%s\t%s\t%s\t%s\n' "$label" "$(cat "$evidence/$label.time")" "$((ended-began))" "$before" "$after" >> "$evidence/timings.tsv"
    printf '%s status=%s time=%s load=%s->%s\n' "$label" "$status" "$(cat "$evidence/$label.time")" "$before" "$after"
    return "$status"
}
census() {
    local label=$1 store=$2 status=0
    load > "$evidence/$label.census.load-before"
    printf '%q ' perl "$repository/bench/compiler-query-container-census.pl" "$store" "$key" "$budget" > "$evidence/$label.census.command"
    printf '\n' >> "$evidence/$label.census.command"
    /usr/bin/time -q -f '%e\t%U\t%S\t%M\t%x' -o "$evidence/$label.census.time" \
        prlimit --as=4294967296 -- timeout --foreground --kill-after=10s 900 \
        perl "$repository/bench/compiler-query-container-census.pl" "$store" "$key" "$budget" \
        > "$evidence/$label.census.json" 2> "$evidence/$label.census.stderr" || status=$?
    load > "$evidence/$label.census.load-after"
    printf '%s\n' "$status" > "$evidence/$label.census.status"
    return "$status"
}
stats() {
    local label=$1 expected=$2
    perl -MJSON::PP - "$evidence/$label.stderr" "$expected" > "$evidence/$label.stats-validation.json" <<'PERL'
use strict; use warnings;
my ($path,$expected)=@ARGV;
open my $in,'<',$path or die $!;
my ($stats,$summary,%rows);
while (<$in>) {
    if (s/^LKJSCRIPT_STATS //) { die "duplicate stats\n" if $stats; $stats=decode_json($_); }
    elsif (s/^LKJSCRIPT_STORE_IO //) {
        my $row=decode_json($_);
        my $category=$row->{category};
        if ($category eq 'summary') { die "duplicate summary\n" if $summary; $summary=$row; }
        else { die "duplicate category\n" if $rows{$category}; $rows{$category}=$row; }
    } else { die "unexpected stderr $_"; }
}
die "missing stats\n" unless $stats;
die "missing measured-build Store I/O summary/categories\n" if $expected eq 'build' && (!$summary || !keys(%rows));
if ($expected eq 'query') {
    die "pinned query current-producer hash\n" unless exists($stats->{producer_bytes_hashed}) && $stats->{producer_bytes_hashed}==0;
}
my $sum=0;
for my $row(values %rows) {
    $sum += $row->{$_}//0 for qw(read_ns pack_decode_ns pack_retain_ns seal_ns pack_reseal_ns publish_ns prepare_ns metadata_ns stream_write_ns);
}
if ($summary) {
    die "counter failure\n" if $summary->{global_timing_failures} || $summary->{global_counter_overflow} || $summary->{pending_checkpoint};
    die "exclusive counter sum\n" unless $sum==$summary->{accounted_ns};
}
print JSON::PP->new->canonical->encode({status=>'captured-stats-validated',stats=>$stats,
    exclusive_store_counter_sum_ns=>$sum,store_io_summary=>$summary,store_io_categories=>\%rows}),"\n";
PERL
}
snapshot_from() {
    local label=$1
    perl - "$evidence/$label.stdout" <<'PERL'
use strict;use warnings;
open my $in,'<',$ARGV[0] or die $!;my @pins;
while(<$in>){push @pins,$1 if /^LKJSCRIPT_SNAPSHOT ([0-9a-f]{64}) source=[0-9a-f]{64} checked-ir=[0-9a-f]{64}\n$/;}
die "exactly one snapshot expected\n" unless @pins==1;print "$pins[0]\n";
PERL
}
build_phase() {
    local rung=$1 mode=$2 state=$3 phase=$4 indexed=$5
    local compiler=$baseline
    [[ $mode != candidate ]] || compiler=$candidate
    local tree="$fixture/source-$rung" store="$fixture/store-$rung-$mode"
    local label="$rung-$mode-$phase"
    local argv=(build main.lkj --source-dir "$tree" --library-dir "$fixture/frozen/library" \
        --runtime-source "$fixture/frozen/library/runtime.lkj" --store "$store" --store-key "$key" \
        --store-bytes "$budget" --inline-limit 0 --stats -o "$fixture/image-$rung-$mode")
    [[ $indexed == 0 ]] || argv+=(--index)
    printf '%s\t%s\t%s\t%s\t%s\n' "$label" "$rung" "$mode" "$state" "$indexed" >> "$evidence/phase-order.tsv"
    local status=0 census_status=0
    run_timed "$label" "$tree" env LKJSCRIPT_STORE_IO=1 "$compiler" "${argv[@]}" || status=$?
    census "$label" "$store" || census_status=$?
    ((status==0)) || return "$status"
    ((census_status==0)) || return "$census_status"
    stats "$label" build
    sha256sum "$fixture/image-$rung-$mode" > "$evidence/$label.image.sha256"
    perl -MJSON::PP - "$evidence/$label.census.json" <<'PERL'
use strict;use warnings;local $/;open my $in,'<',$ARGV[0]or die$!;my$c=decode_json(<$in>);
my$b=$c->{budget};die"post-build accounting\n"unless$b->{status}eq'authenticated'&&$b->{within_capacity}
 &&!$b->{missing_entries}&&!$b->{physical_length_mismatches}&&!@{$b->{unindexed_regular_names}}&&!@{$c->{nonregular}};
PERL
    if [[ $indexed == 1 ]]; then
        snapshot_from "$label" > "$evidence/$label.observed-snapshot"
        if [[ $phase == unchanged-s0 ]]; then
            cmp "$evidence/$rung-$mode-S0.snapshot" "$evidence/$label.observed-snapshot"
        else
            cp "$evidence/$label.observed-snapshot" "$evidence/$rung-$mode-$state.snapshot"
        fi
    fi
}
mkdir "$fixture/factory" "$fixture/frozen"
git archive "$factory_revision" src library | tar -x -C "$fixture/factory"
git archive "$shape_revision" src library | tar -x -C "$fixture/frozen"
cp bench/scale.lkj "$fixture/factory/src/main.lkj"
(cd "$fixture/factory" && find src library -type f -print0 | sort -z | xargs -0 sha256sum) > "$evidence/factory-inputs.sha256"
(cd "$fixture/frozen" && find src library -type f -print0 | sort -z | xargs -0 sha256sum) > "$evidence/frozen-inputs.sha256"
run_timed generator "$fixture/factory" "$baseline" build src/main.lkj --source-dir "$fixture/factory" \
    --library-dir "$fixture/factory/library" --runtime-source "$fixture/factory/library/runtime.lkj" \
    --inline-limit 0 --no-store -o "$fixture/generator"
sha256sum "$fixture/generator" > "$evidence/generator.sha256"
run_timed shape "$fixture/frozen" "$fixture/generator" shape "$fixture/frozen"
cp "$evidence/shape.stdout" "$evidence/shape"
[[ $(sha256sum "$evidence/shape" | awk '{print $1}') == "$shape_sha" ]]
for rung in 10000 100000; do
    tree="$fixture/source-$rung"
    mkdir "$tree" "$fixture/store-$rung-baseline" "$fixture/store-$rung-candidate"
    run_timed "$rung-generate" "$tree" "$fixture/generator" generate "$tree" "$rung" 20261116 < "$evidence/shape"
    (cd "$tree" && find . -type f -print0 | sort -z | xargs -0 sha256sum) > "$evidence/$rung-S0-inputs.sha256"
    build_phase "$rung" baseline S0 cold-s0 0
    build_phase "$rung" candidate S0 cold-s0 0
    build_phase "$rung" baseline S0 index-s0 1
    build_phase "$rung" candidate S0 index-s0 1
    build_phase "$rung" baseline S0 unchanged-s0 1
    build_phase "$rung" candidate S0 unchanged-s0 1
    for mode in baseline candidate; do cp "$fixture/image-$rung-$mode" "$fixture/image-$rung-$mode-S0"; done
    cmp "$fixture/image-$rung-baseline-S0" "$fixture/image-$rung-candidate-S0"
    printf 'S0 images byte-identical\n' > "$evidence/$rung-S0.image-parity"
    sha256sum "$tree/edit.lkj" > "$evidence/$rung-body.before.sha256"
    perl - "$tree/edit.lkj" <<'PERL'
use strict;use warnings;my($path)=@ARGV;open my$in,'<:raw',$path or die$!;local$/;my$old=<$in>;close$in;
die"guarded fixture body differs\n"unless$old eq"pub fn target(value: u64) -> u64 {\nvalue ^ 1u64\n}\n";
my$new=$old;$new=~s/value \^ 1u64/value ^ 2u64/ or die"no replacement\n";
open my$out,'>:raw',$path or die$!;print{$out}$new;close$out or die$!;
PERL
    sha256sum "$tree/edit.lkj" > "$evidence/$rung-body.after.sha256"
    (cd "$tree" && find . -type f -print0 | sort -z | xargs -0 sha256sum) > "$evidence/$rung-S1-inputs.sha256"
    build_phase "$rung" baseline S1 body-s1 0
    build_phase "$rung" candidate S1 body-s1 0
    build_phase "$rung" baseline S1 index-s1 1
    build_phase "$rung" candidate S1 index-s1 1
    for mode in baseline candidate; do
        cp "$fixture/image-$rung-$mode" "$fixture/image-$rung-$mode-S1"
        if cmp -s "$evidence/$rung-$mode-S0.snapshot" "$evidence/$rung-$mode-S1.snapshot"; then
            echo 'changed indexed source retained the same snapshot' >&2; exit 1
        fi
    done
    cmp "$fixture/image-$rung-baseline-S1" "$fixture/image-$rung-candidate-S1"
    printf 'S1 images byte-identical\n' > "$evidence/$rung-S1.image-parity"
    if cmp -s "$fixture/image-$rung-baseline-S0" "$fixture/image-$rung-baseline-S1"; then
        echo 'body edit retained the same native image' >&2; exit 1
    fi
done
# Build timings are complete before full rooted oracles or correctness queries.
failures=0
for rung in 10000 100000; do
    tree="$fixture/source-$rung"
    for state in S0 S1; do
        pair_present=1
        for mode in baseline candidate; do
            pin=$(cat "$evidence/$rung-$mode-$state.snapshot")
            store="$fixture/store-$rung-$mode"
            if [[ $mode == baseline ]]; then
                root_path="$store/semantic-root-$baseline_slot-$pin"
                locator_path="$store/semantic-locator-$pin"
            else
                root_path="$store/semantic-container-v3-$candidate_slot-$pin"
                locator_path="$store/semantic-locator-v3-$pin"
            fi
            root_present=0; locator_present=0
            [[ ! -f $root_path || -L $root_path ]] || root_present=1
            [[ ! -f $locator_path || -L $locator_path ]] || locator_present=1
            printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$rung" "$state" "$mode" "$pin" "$root_present" "$locator_present" >> "$evidence/retention.tsv"
            if ((root_present==0 || locator_present==0)); then pair_present=0; fi
        done
        if ((pair_present)); then
            sa=$(cat "$evidence/$rung-baseline-$state.snapshot")
            sb=$(cat "$evidence/$rung-candidate-$state.snapshot")
            if ! run_timed "$rung-$state-map-oracle" "$tree" perl "$repository/bench/compiler-query-container-compare.pl" maps \
                "$fixture/store-$rung-baseline" "$fixture/store-$rung-candidate" "$key" "$baseline_sha" "$candidate_sha" "$sa" "$sb"; then
                failures=$((failures+1))
            fi
        else
            printf 'not-run: at least one exact root/locator was evicted; no retry\n' > "$evidence/$rung-$state-map-oracle.unavailable"
        fi
    done
done
kinds=(outline signature text slice uses callers callees impls instances type layout authority dependents)
ids=(edit edit::target edit::target edit::target edit::target edit::target main std::sequence::VecMutate edit::target edit::target u64 edit::target edit::target)
for rung in 10000 100000; do
    tree="$fixture/source-$rung"
    for state in S0 S1; do
        for index in "${!kinds[@]}"; do
            kind=${kinds[$index]}; identity=${ids[$index]}
            baseline_status=unavailable; candidate_status=unavailable
            for mode in baseline candidate; do
                pin=$(cat "$evidence/$rung-$mode-$state.snapshot")
                store="$fixture/store-$rung-$mode"
                compiler=$baseline; slot=$baseline_slot; root_name="semantic-root-$slot-$pin"; locator_name="semantic-locator-$pin"
                if [[ $mode == candidate ]]; then
                    compiler=$candidate; slot=$candidate_slot; root_name="semantic-container-v3-$slot-$pin"; locator_name="semantic-locator-v3-$pin"
                fi
                if [[ ! -f $store/$root_name || -L $store/$root_name || ! -f $store/$locator_name || -L $store/$locator_name ]]; then continue; fi
                label="$rung-$state-$mode-query-$kind"
                status=0
                run_timed "$label" "$tree" "$compiler" query main.lkj --source-dir "$tree" --library-dir "$fixture/frozen/library" \
                    --store "$store" --store-key "$key" --snapshot "$pin" --kind "$kind" --id "$identity" \
                    --depth 1 --limit 96 --bytes 32768 --stats || status=$?
                if ((status==0)); then
                    stats "$label" query
                    [[ $(wc -c < "$evidence/$label.stdout") -le 32768 ]]
                else failures=$((failures+1)); fi
                if [[ $mode == baseline ]]; then baseline_status=$status; else candidate_status=$status; fi
            done
            comparison=unavailable
            map="$evidence/$rung-$state-map-oracle.stdout"
            if [[ $baseline_status == 0 && $candidate_status == 0 && -f $map && $(cat "$evidence/$rung-$state-map-oracle.status") == 0 ]]; then
                comparison=0
                label="$rung-$state-query-$kind-parity"
                run_timed "$label" "$tree" perl "$repository/bench/compiler-query-container-compare.pl" query \
                    "$map" "$key" "$evidence/$rung-$state-baseline-query-$kind.stdout" \
                    "$evidence/$rung-$state-candidate-query-$kind.stdout" "$kind" "$identity" 1 || comparison=$?
                if ((comparison!=0)); then failures=$((failures+1)); fi
            fi
            printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$rung" "$state" "$kind" "$identity" "$baseline_status" "$candidate_status" "$comparison" >> "$evidence/queries.tsv"
        done
    done
done
for rung in 10000 100000; do
    for mode in baseline candidate; do census "$rung-$mode-final" "$fixture/store-$rung-$mode"; done
done
printf 'timed_build_sequence=complete\ncorrectness_failures=%s\nretention_file=retention.tsv\nowned_fixture_retained_until_acceptance=true\n' "$failures" >> "$evidence/completion"
((failures==0))
