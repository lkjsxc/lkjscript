#!/usr/bin/env bash
# Frozen ordinary-versus-index-populated Store attribution. One producer; no paging.
set -euo pipefail
cd "$(dirname "$0")/.."
repository=$PWD
if [[ $# == 2 && $1 == --cleanup ]]; then
    run=$(realpath -e "$2")
    [[ $run == "$repository"/target/compiler-cache-index-run.* && ! -L $2 ]] || exit 2
    [[ $(cat "$run/owner") == "$repository" ]] || exit 2
    pid=$(cat "$run/pid")
    [[ $pid =~ ^[1-9][0-9]*$ ]] || exit 2
    if kill -0 "$pid" 2>/dev/null; then echo 'run supervisor is alive' >&2; exit 2; fi
    ps -eo pid=,args= > "$run/process-snapshot"
    if awk -v run="$run" -v self="$$" -v parent="$PPID" \
        '$1!=self && $1!=parent && index($0,run)>0 {found=1} END {exit !found}' "$run/process-snapshot"; then
        echo 'a process still references this run' >&2; exit 2
    fi
    key_directory=$(cat "$run/key-directory")
    [[ $key_directory == /tmp/lkjscript-cache-index-key.* && ! -L $key_directory ]] || exit 2
    [[ $(cat "$key_directory/owner") == "$run" ]] || exit 2
    rm -rf -- "$run" "$key_directory"
    exit 0
fi
(($# == 3)) || { echo 'usage: compiler-cache-index-condition.sh EMITTER INSTRUMENTED RESULT_PREFIX' >&2; exit 2; }
emitter=$(realpath "$1")
instrumented=$(realpath "$2")
prefix=$(realpath -m "$3")
for binary in "$emitter" "$instrumented"; do [[ -x $binary ]] || exit 2; done
[[ ! -e $prefix.tsv && ! -e $prefix.raw ]] || { echo 'results already exist; no retries' >&2; exit 2; }
frozen_commit=b7e8ed035d155e75438406c7474a747220ff9537
expected_emitter=71b2fc066cd74e9e37cb3134173903c9b322b83472b9293f1a49df7c1d18f3b7
[[ $(sha256sum "$emitter" | awk '{print $1}') == "$expected_emitter" ]] || exit 2
expected_instrumented=82d85b1a16818c391f194f0f956a32d077a6c9639452841c86715d04c07c68f7
[[ $(sha256sum "$instrumented" | awk '{print $1}') == "$expected_instrumented" ]] || exit 2
free=$(df -Pk . | awk 'NR==2 {printf "%.0f", $4*1024}')
((free >= 8589934592)) || { echo 'requires 8 GiB free scratch space' >&2; exit 2; }
umask 077
mkdir -p "$(dirname "$prefix")"
mkdir "$prefix.raw"
run=$(mktemp -d "$repository/target/compiler-cache-index-run.XXXXXX")
key_directory=$(mktemp -d /tmp/lkjscript-cache-index-key.XXXXXX)
printf '%s\n' "$repository" > "$run/owner"
printf '%s\n' "$$" > "$run/pid"
printf '%s\n' "$key_directory" > "$run/key-directory"
printf '%s\n' "$run" > "$key_directory/owner"
head -c 32 /dev/urandom > "$key_directory/key"
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    printf 'exit_status=%s\n' "$status" >> "$prefix.meta"
    rm -rf -- "$run" "$key_directory"
    [[ ! -e $run && ! -e $key_directory ]] || { echo 'cleanup incomplete' >&2; exit 1; }
    printf 'cleanup=generated_source_store_key_and_images_removed\n' >> "$prefix.meta"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
load() { awk '{print $1","$2","$3}' /proc/loadavg; }
printf 'protocol=index-conditioned-cache-io-1\nsource_commit=%s\nseed=20261116\nrungs=10000,100000_if_guard_fits\norder=cold_A,copy_A_to_B,index_B,body_transaction,body_A,body_B,no_store_control\nfile_cache_reset=false\nordinary_payload_prewarming=both_sides_after_copy_and_after_index\nmemory_limit_bytes=4294967296\nwatchdog_seconds=900\nstore_budget_bytes=2147483648\noutput_inline_limit=0\nproducer_inline_limit=64\nfree_preflight_bytes=%s\n' "$frozen_commit" "$free" > "$prefix.meta"
printf 'harness_sha256=%s\nprotocol_sha256=%s\n' \
    "$(sha256sum bench/compiler-cache-index-condition.sh | awk '{print $1}')" \
    "$(sha256sum bench/compiler-cache-index-condition.protocol.md | awk '{print $1}')" >> "$prefix.meta"
for role in emitter instrumented; do
    case "$role" in emitter) binary=$emitter;; instrumented) binary=$instrumented;; esac
    printf '%s_path=%s\n%s_sha256=%s\n%s_bytes=%s\n' "$role" "$binary" "$role" "$(sha256sum "$binary" | awk '{print $1}')" "$role" "$(wc -c < "$binary")" >> "$prefix.meta"
done
printf 'sequence\trung\trole\tphase\tstatus\twall_ms\ttime_wall_s\tuser_s\tkernel_s\trss_kib\tload_before\tload_after\tproducer_sha256\tsource_sha256\timage_sha256\timage_bytes\tstore_scope\tstore_bytes\n' > "$prefix.tsv"
printf 'rung\trole\tphase\timage_sha256\tsource_sha256\tverdict\n' > "$prefix.parity.tsv"
sequence=0
rung=setup role=setup source_digest=- image= store= store_scope=none
producer_digest=$expected_emitter
measure() {
    local phase=$1; shift
    sequence=$((sequence + 1))
    local stem before after started ended status=0 elapsed wall user kernel rss image_hash=- image_bytes=0 store_bytes=0
    stem=$(printf '%03d-%s-%s-%s' "$sequence" "$rung" "$role" "$phase")
    printf '%q ' "$@" > "$prefix.raw/$stem.argv"
    printf '\n' >> "$prefix.raw/$stem.argv"
    before=$(load)
    started=$(date +%s%N)
    printf 'sequence=%s\nrung=%s\nrole=%s\nphase=%s\nstarted_ns=%s\nload_before=%s\n' \
        "$sequence" "$rung" "$role" "$phase" "$started" "$before" > "$prefix.raw/$stem.checkpoint"
    /usr/bin/time -f '%e %U %S %M' -o "$prefix.raw/$stem.time" \
        prlimit --as=4294967296 -- timeout --kill-after=10s 900s "$@" \
        > "$prefix.raw/$stem.stdout" 2> "$prefix.raw/$stem.stderr" || status=$?
    ended=$(date +%s%N)
    after=$(load)
    elapsed=$(awk -v a="$started" -v b="$ended" 'BEGIN {printf "%.3f",(b-a)/1000000}')
    read -r wall user kernel rss < <(tail -n 1 "$prefix.raw/$stem.time")
    if [[ -n $image && -f $image ]]; then image_hash=$(sha256sum "$image" | awk '{print $1}'); image_bytes=$(wc -c < "$image"); fi
    if [[ -n $store && -d $store ]]; then store_bytes=$(du -sb "$store" | awk '{print $1}'); fi
    printf 'status=%s\nended_ns=%s\nload_after=%s\n' "$status" "$ended" "$after" >> "$prefix.raw/$stem.checkpoint"
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$sequence" "$rung" "$role" "$phase" "$status" "$elapsed" "$wall" "$user" "$kernel" "$rss" "$before" "$after" \
        "$producer_digest" "$source_digest" "$image_hash" "$image_bytes" "$store_scope" "$store_bytes" >> "$prefix.tsv"
    printf '%s %s %s status=%s wall_ms=%s load=%s->%s\n' "$rung" "$role" "$phase" "$status" "$elapsed" "$before" "$after"
    if ((status != 0)); then
        printf 'stopped_at=%s\nrefusal_status=%s\nlater_rows=not_run\n' "$stem" "$status" >> "$prefix.meta"
        tail -20 "$prefix.raw/$stem.stderr" >&2
        return "$status"
    fi
    last_stem=$stem
    last_rss=$rss
    last_wall_ms=$elapsed
    last_store_bytes=$store_bytes
}
source_manifest() {
    local destination=$1
    (cd "$tree"; rg --files --hidden --no-ignore -g '*.lkj' | LC_ALL=C sort | xargs -r -d '\n' sha256sum) > "$destination"
    source_digest=$(sha256sum "$destination" | awk '{print $1}')
}
mkdir "$run/frozen"
git archive "$frozen_commit" src library | tar -xf - -C "$run/frozen"
printf 'source_tree=%s\nlibrary_tree=%s\nruntime_sha256=%s\n' \
    "$(git rev-parse "$frozen_commit:src")" "$(git rev-parse "$frozen_commit:library")" \
    "$(sha256sum "$run/frozen/library/runtime.lkj" | awk '{print $1}')" >> "$prefix.meta"
cp -R "$run/frozen/src" "$run/analysis"
git show "$frozen_commit:bench/scale.lkj" > "$run/analysis/main.lkj"
printf 'generator_source_sha256=%s\n' "$(sha256sum "$run/analysis/main.lkj" | awk '{print $1}')" >> "$prefix.meta"
image="$run/scale"
measure generator_build "$emitter" build analysis/main.lkj --source-dir "$run" \
    --library-dir "$run/frozen/library" --runtime-source "$run/frozen/library/runtime.lkj" \
    --inline-limit 0 --no-store --stats -o "$image"
producer_digest=$(sha256sum "$image" | awk '{print $1}')
image=
measure census "$run/scale" shape "$run/frozen"
cp "$prefix.raw/$last_stem.stdout" "$prefix.shape"
printf 'shape_sha256=%s\n' "$(sha256sum "$prefix.shape" | awk '{print $1}')" >> "$prefix.meta"
cat > "$run/body.json" <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"edit::target","source":"{\nvalue ^ 2u64\n}"}]}
JSON
cp "$run/body.json" "$prefix.body-request.json"
printf 'rung\tpredicted_index_rss_kib\tpredicted_index_wall_ms\tpredicted_store_bytes\trss_guard_kib\twall_guard_ms\tstore_guard_bytes\tdecision\n' > "$prefix.predictions.tsv"

# Hash only A's preexisting ordinary entries, not B's added index payloads.
# Both sides are read outside all compiler timing before the body edit.
ordinary_parity() {
    local phase=$1
    perl - "$store_a" "$store_b" "$phase" > "$prefix.$rung.$phase.tsv" <<'PERL'
use strict; use warnings; use Digest::SHA;
my ($a,$b,$phase)=@ARGV;
opendir my $dir,$a or die $!;
my @names=sort grep { /^(?:front|check|native-|image|inputs-image)/ && -f "$a/$_" } readdir $dir;
closedir $dir;
print "phase\tentry\tA_bytes\tB_bytes\tA_sha256\tB_sha256\tverdict\n";
for my $name (@names) {
    open my $af,'<',"$a/$name" or die $!; binmode $af;
    my $ah=Digest::SHA->new(256)->addfile($af)->hexdigest; close $af;
    my ($bh,$bs)=('-',0);
    if (-f "$b/$name") {
        open my $bf,'<',"$b/$name" or die $!; binmode $bf;
        $bh=Digest::SHA->new(256)->addfile($bf)->hexdigest; close $bf; $bs=-s "$b/$name";
    }
    print join("\t",$phase,$name,-s "$a/$name",$bs,$ah,$bh,$ah eq $bh ? 'exact' : 'different'),"\n";
}
PERL
    if [[ $phase == after_copy ]] && awk -F '\t' 'NR>1 && $7!="exact" {bad=1} END {exit !bad}' "$prefix.$rung.$phase.tsv"; then
        echo 'closed store copy is not exact' >&2; exit 1
    fi
}
store_census() {
    perl - "$rung" "$store_a" "$store_b" > "$prefix.$rung.final-census.tsv" <<'PERL'
use strict; use warnings;
my $rung=shift @ARGV;
print "rung\tcondition\tcategory\tfiles\tfile_bytes\n";
for my $condition (qw(A B)) {
    my $directory=shift @ARGV; opendir my $dir,$directory or die $!;
    my %counts;
    for my $name (readdir $dir) {
        my @s=lstat "$directory/$name"; next unless @s && -f _ && !-l _;
        my $category = $name =~ /^native-/ ? 'native' :
            $name =~ /^semantic-/ ? 'index' :
            $name =~ /^inputs-image-/ ? 'inputs-image' :
            $name =~ /^inputs-index-/ ? 'inputs-index' :
            $name =~ /^inputs-test-/ ? 'inputs-test' :
            $name =~ /^front-/ ? 'front' :
            $name =~ /^check-/ ? 'check' :
            $name =~ /^image-/ ? 'image' :
            $name =~ /^test-/ ? 'test' :
            $name =~ /^\./ ? 'metadata' : 'other';
        $counts{$category}[0]++; $counts{$category}[1]+=$s[7];
    }
    closedir $dir;
    print join("\t",$rung,$condition,$_,@{$counts{$_}}),"\n" for sort keys %counts;
}
PERL
}
for rung in 10000 100000; do
    if ((rung == 100000)); then
        # Historical observed index RSS scale2,670,368/370,004; prediction only.
        predicted_rss=$(awk -v r="$index_rss_10k" 'BEGIN {printf "%.0f",int(r*2670368/370004*1.20+0.999999)}')
        predicted_wall=$(awk -v w="$index_wall_10k" 'BEGIN {printf "%.0f",int(w*10*1.50+0.999999)}')
        predicted_store=$(awk -v b="$index_store_10k" 'BEGIN {printf "%.0f",int(b*10*1.20+0.999999)}')
        decision=run
        if ((predicted_rss > 3670016 || predicted_wall > 900000 || predicted_store > 2147483648)); then decision=predicted_refusal; fi
        printf '%s\t%s\t%s\t%s\t3670016\t900000\t2147483648\t%s\n' "$rung" "$predicted_rss" "$predicted_wall" "$predicted_store" "$decision" >> "$prefix.predictions.tsv"
        if [[ $decision != run ]]; then
            printf 'rung_100000=not_run_predicted_resource_refusal\ncompleted_rows=%s\n' "$sequence" >> "$prefix.meta"
            exit 0
        fi
    fi
    free=$(df -Pk "$run" | awk 'NR==2 {printf "%.0f",$4*1024}')
    ((free >= 8589934592)) || { printf 'disk_refusal_rung=%s\n' "$rung" >> "$prefix.meta"; exit 1; }
    printf 'rung_%s_free_preflight_bytes=%s\n' "$rung" "$free" >> "$prefix.meta"
    tree="$run/source"
    mkdir "$tree"
    role=setup
    producer_digest=$(sha256sum "$run/scale" | awk '{print $1}')
    source_digest=- image= store= store_scope=none
    measure generate "$run/scale" generate "$tree" "$rung" 20261116 < "$prefix.shape"
    cp "$prefix.raw/$last_stem.stdout" "$prefix.$rung.workload"
    actual_lines=$(find "$tree" -name '*.lkj' -type f -exec cat {} + | wc -l)
    ((actual_lines == rung)) || { echo 'generated line count differs' >&2; exit 1; }
    source_manifest "$prefix.$rung.pristine.sha256"
    pristine_digest=$source_digest
    role=instrumented
    producer_digest=$expected_instrumented
    store_a="$run/store-A"
    store_b="$run/store-B"
    operation_store="$run/operation-store"
    mkdir "$store_a" "$operation_store"
    store=$store_a store_scope=ordinary-A image="$run/image"
    build() {
        local phase=$1; shift
        measure "$phase" "$instrumented" build main.lkj --source-dir "$tree" \
            --library-dir "$run/frozen/library" --runtime-source "$run/frozen/library/runtime.lkj" \
            --inline-limit 0 --stats --store "$store" --store-key "$key_directory/key" \
            --store-bytes 2147483648 -o "$image" "$@"
    }
    build cold_A
    cp "$image" "$run/cold-expected"
    printf '%s\tinstrumented\tcold_A\t%s\t%s\treference\n' "$rung" "$(sha256sum "$image" | awk '{print $1}')" "$source_digest" >> "$prefix.parity.tsv"
    role=preparation image= store=$store_b store_scope=copy-target-B producer_digest=-
    measure copy_A_to_B cp -a "$store_a" "$store_b"
    ordinary_parity after_copy
    role=instrumented image="$run/image" store_scope=indexed-B producer_digest=$expected_instrumented
    build populate_B --index
    if ((rung == 10000)); then
        index_rss_10k=$last_rss index_wall_10k=$last_wall_ms index_store_10k=$last_store_bytes
    fi
    cmp "$run/cold-expected" "$image"
    printf '%s\tinstrumented\tpopulate_B\t%s\t%s\texact\n' "$rung" "$(sha256sum "$image" | awk '{print $1}')" "$source_digest" >> "$prefix.parity.tsv"
    ordinary_parity after_index
    if awk -F '\t' 'NR>1 && $7!="exact" {bad=1} END {exit !bad}' "$prefix.$rung.after_index.tsv"; then
        printf 'rung_%s_preexisting_ordinary_payloads=index_preparation_changed_some\n' "$rung" >> "$prefix.meta"
    else
        printf 'rung_%s_preexisting_ordinary_payloads=byte_identical\n' "$rung" >> "$prefix.meta"
    fi
    image= store=$operation_store store_scope=operation producer_digest=$expected_emitter
    role=preparation
    measure body_transaction "$emitter" edit main.lkj --source-dir "$tree" \
        --library-dir "$run/frozen/library" --runtime-source "$run/frozen/library/runtime.lkj" \
        --inline-limit 0 --stats --store "$operation_store" --store-key "$key_directory/key" \
        --store-bytes 2147483648 --request "$run/body.json"
    source_manifest "$prefix.$rung.body.sha256"
    [[ $source_digest != "$pristine_digest" ]] || { echo 'body transaction did not change source' >&2; exit 1; }
    role=instrumented image="$run/image" producer_digest=$expected_instrumented
    store=$store_a store_scope=ordinary-A
    build body_A
    if cmp -s "$run/cold-expected" "$image"; then echo 'body edit left image unchanged' >&2; exit 1; fi
    cp "$image" "$run/body-expected"
    printf '%s\tinstrumented\tbody_A\t%s\t%s\treference\n' "$rung" "$(sha256sum "$image" | awk '{print $1}')" "$source_digest" >> "$prefix.parity.tsv"
    store=$store_b store_scope=indexed-B
    build body_B
    cmp "$run/body-expected" "$image"
    printf '%s\tinstrumented\tbody_B\t%s\t%s\texact\n' "$rung" "$(sha256sum "$image" | awk '{print $1}')" "$source_digest" >> "$prefix.parity.tsv"
    store= store_scope=none
    measure body_control "$instrumented" build main.lkj --source-dir "$tree" \
        --library-dir "$run/frozen/library" --runtime-source "$run/frozen/library/runtime.lkj" \
        --inline-limit 0 --stats --no-store -o "$image"
    cmp "$run/body-expected" "$image"
    printf '%s\tinstrumented\tbody_control\t%s\t%s\texact\n' "$rung" "$(sha256sum "$image" | awk '{print $1}')" "$source_digest" >> "$prefix.parity.tsv"
    store_census
    rm -rf -- "$tree" "$store_a" "$store_b" "$operation_store" "$image" "$run/cold-expected" "$run/body-expected"
done
printf 'completed_rows=%s\nparity=exact_copy_and_all_index_body_no_store_images\n' "$sequence" >> "$prefix.meta"
