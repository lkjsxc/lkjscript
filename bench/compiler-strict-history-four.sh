#!/usr/bin/env bash
# Separate fixed four-candidate language-scope probe; never resumes the scoped campaign.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $# != 4 ]]; then
    printf 'usage: %s REPOSITORY BINARY PRIVATE_KEY OUTPUT\n' "$0" >&2
    exit 2
fi
mode=replay
repository=$(realpath "$1")
compiler=$(realpath "$2")
private_key=$(realpath "$3")
output=$(realpath -m "$4")
count=4
scan=300
memory_bytes=${LKJSCRIPT_HISTORY_MEMORY_BYTES:-4294967296}
timeout_seconds=${LKJSCRIPT_HISTORY_TIMEOUT_SECONDS:-900}
store_bytes=${LKJSCRIPT_HISTORY_STORE_BYTES:-2147483648}
keep_fixture=${LKJSCRIPT_HISTORY_KEEP_FIXTURE:-0}
[[ $keep_fixture == 0 || $keep_fixture == 1 ]] || exit 2
history_head=$(git -C "$repository" rev-parse --verify "${LKJSCRIPT_HISTORY_HEAD:-a4fd685a9bb2e7736110a4ed2a43651dfe040d56}^{commit}")
[[ $history_head == a4fd685a9bb2e7736110a4ed2a43651dfe040d56 ]] || exit 2
[[ $memory_bytes == 4294967296 && $timeout_seconds == 900 && $store_bytes == 2147483648 ]] || exit 2
compiler_bytes=$(stat -c %s "$compiler")
compiler_sha=$(sha256sum "$compiler" | cut -d ' ' -f 1)
resume=${LKJSCRIPT_HISTORY_RESUME:-0}
[[ $resume == 0 ]] || { printf 'E_PROBE_RESUME this is a separate non-resuming segment\n' >&2; exit 2; }
[[ $compiler_sha == 4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09 ]] || exit 2
verify_record() {
    [[ $(sha256sum "$1" | cut -d ' ' -f 1) == "$2" ]] || {
        printf 'E_PROBE_RECORD frozen control differs: %s\n' "$1" >&2
        exit 2
    }
}
verify_record bench/compiler-self-history.raw.sha256 f1979bc70f3127080a0fd957772f4fb975f5e2257bad11dd0a0428d628ed3b8b
verify_record bench/compiler-self-history.proofs.tsv 8fd505fc658b051491ddc157e43783bff39befcde721e3ab4b97115661a93eae
verify_record bench/compiler-self-history.outside-language.tsv e5a7ba7ffb4a2f60b241eabbbdfd41e354be9ed962d4c219b512161f92d5401e
verify_record bench/compiler-strict-history-imported.tsv 0aafd3a891bf224bd3113ab9de58219fecba44b84e0c5605a28f8ced0f25acb1
verify_record bench/compiler-strict-history-four.order 1241ad75095c3492af9ab18b9435d88bc6797d74b03590622264498c5ce3f5bd
sha256sum --status -c bench/compiler-self-history.raw.sha256
specification="mode=$mode root_policy=common-main-otherwise-common-lib scan=$scan probe_count=$count memory_bytes=$memory_bytes timeout_seconds=$timeout_seconds store_bytes=$store_bytes"
passed=0
attempted=0
if [[ $resume == 1 ]]; then
    if [[ ! -f $output || ! -f $output.meta || ! -d $output.logs ]]; then
        printf 'E_HISTORY_RESUME missing prior campaign files\n' >&2
        exit 2
    fi
    if ! rg -Fxq "$specification" "$output.meta" || ! rg -Fxq "history_head=$history_head" "$output.meta"; then
        printf 'E_HISTORY_RESUME campaign population or limits differ\n' >&2
        exit 2
    fi
    old_compiler_sha=$(awk '/^[0-9a-f]+  / && length($1)==64 {print $1; exit}' "$output.meta")
    if [[ $old_compiler_sha != "$compiler_sha" ]]; then
        printf 'E_HISTORY_RESUME compiler identity differs\n' >&2
        exit 2
    fi
    if ! awk -F '\t' 'NR==1 && NF==16 && $16=="source_root" { good=1 } END { exit !good }' "$output"; then
        printf 'E_HISTORY_RESUME incompatible result schema\n' >&2
        exit 2
    fi
    passed=$(awk -F '\t' 'NR>1 && $4=="passed" && !seen[$2]++ { n++ } END { print n+0 }' "$output")
    attempted=$(awk -F '\t' 'NR>1 && $1>n { n=$1 } END { print n+0 }' "$output")
    rm -f -- "$output.stop"
else
    if [[ -e $output || -e $output.logs || -e $output.meta ]]; then
        printf 'E_HISTORY_EXISTS choose a new output or set LKJSCRIPT_HISTORY_RESUME=1\n' >&2
        exit 2
    fi
fi
archive_blocks() {
    git -C "$repository" ls-tree -rl "$1" -- src library tools tests |
        awk '$4 ~ /^[0-9]+$/ { bytes += int(($4 + 4095) / 4096) * 4096 } END { printf "%.0f\n", bytes }'
}
free_space() {
    local before_bytes after_bytes allowance required available
    before_bytes=$(archive_blocks "$1") || return 1
    after_bytes=$(archive_blocks "$2") || return 1
    # Two measured trees, doubled for retained .rs plus .lkj copies; a 128 MiB
    # floor covers directory metadata and small revisions. Reserve 2x the store
    # budget for authenticated envelopes/block overhead, plus binary and logs.
    allowance=$((2 * (before_bytes + after_bytes)))
    if (( allowance < 134217728 )); then allowance=134217728; fi
    required=$((2 * store_bytes + allowance + compiler_bytes + 134217728))
    available=$(df -B1 --output=avail "$PWD" | tail -n 1)
    printf '%s\t%s\t%s\t%s\t%s\n' "$available" "$required" "$allowance" "$1" "$2" >> "$output.space"
    if (( available < required )); then
        printf 'E_HISTORY_SPACE available=%s required=%s measured_tree_allowance=%s\n' "$available" "$required" "$allowance" >&2
        return 1
    fi
}
if [[ $resume == 0 ]]; then : > "$output.space"; fi
free_space "$history_head" "$history_head" || exit 2
fixture=$(mktemp -d "$PWD/target/compiler-strict-history.XXXXXX")
cleanup() {
    if [[ $keep_fixture == 1 ]]; then
        printf 'retained_fixture=%s\n' "$fixture" >> "$output.meta"
    else
        rm -rf "$fixture"
    fi
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
mkdir "$fixture/store"
if [[ $resume == 0 ]]; then mkdir "$output.logs"; fi
compiler_origin=$compiler
cp -- "$compiler" "$fixture/compiler"
compiler="$fixture/compiler"
git -C "$repository" log -n "$scan" --format='%H %P' "$history_head" > "$fixture/revisions"
if [[ $resume == 0 ]]; then
    printf 'ordinal\trevision\tparent\tstatus\tprojection\toperations\tstdout_lines\twall_s\tuser_s\tsys_s\tpeak_kib\tload_before\tload_after\treason\tlibrary_environment\tsource_root\n' > "$output"
    printf '%s\n' "$specification" > "$output.meta"
    printf 'segment=separate-four-candidate-probe\noriginal_ordinals=104,107,112,158\nordinal_kind=probe-sequence\nimported_count=23\nimported_from=5e63d79b03485344e18c0b5170b7aa97f7ccbfa3\nimported_proofs_sha256=0aafd3a891bf224bd3113ab9de58219fecba44b84e0c5605a28f8ced0f25acb1\nsource_proofs_sha256=8fd505fc658b051491ddc157e43783bff39befcde721e3ab4b97115661a93eae\nsource_manifest_sha256=f1979bc70f3127080a0fd957772f4fb975f5e2257bad11dd0a0428d628ed3b8b\nacceptance=canonical-checked-meaning-and-every-changed-language-file\nstore_state=cold\n' >> "$output.meta"
    printf 'history_head=%s\ncompiler_origin=%s\n' "$history_head" "$compiler_origin" >> "$output.meta"
    sha256sum "$compiler" >> "$output.meta"
    printf 'harness_sha256=%s\n' "$(sha256sum "$0" | cut -d ' ' -f 1)" >> "$output.meta"
    printf 'library_policy=historical-roots-when-both-present-else-frozen-compiler-embedded-on-both-sides\nstop_file=%s.stop\n' "$output" >> "$output.meta"
else
    printf 'resume=true previous_attempted=%s previous_passed=%s store_state=cold load=%s\n' \
        "$attempted" "$passed" "$(cut -d ' ' -f 1 /proc/loadavg)" >> "$output.meta"
fi
awk -F '\t' 'NR>1 && ($4=="passed" || $4=="ineligible") {print $2}' "$output" > "$fixture/completed"

# Listing/archiving is restricted to these four trees. Never copy repository
# metadata, generation-one, data/services, releases or unrelated projects.
materialize() {
    local revision=$1 destination=$2
    local areas=() area file projected
    mkdir "$destination" || return 16
    for area in src library tools tests; do
        if git -C "$repository" cat-file -e "$revision:$area" 2>/dev/null; then areas+=("$area"); fi
    done
    [[ ${#areas[@]} -gt 0 ]] || return 10
    git -C "$repository" ls-tree -r --name-only "$revision" -- "${areas[@]}" > "$fixture/archive-paths" || return 15
    if rg -q '(^|/)\.generation-one(/|$)' "$fixture/archive-paths"; then
        return 11
    fi
    git -C "$repository" archive --format=tar "$revision" -- "${areas[@]}" | tar -xf - -C "$destination" || return 15
    if [[ -n $(find "$destination" -type l -print -quit) ]]; then return 12; fi
    while IFS= read -r -d '' file; do
        projected="${file%.rs}.lkj"
        if [[ -e $projected ]]; then
            cmp -s "$file" "$projected" || return 13
        else
            cp -- "$file" "$projected" || return 17
            projection=legacy-rs-copies
        fi
    done < <(rg --files --no-ignore -0 -g '*.rs' "$destination" || true)
    [[ -f "$destination/src/main.lkj" || -f "$destination/src/lib.lkj" ]] || return 14
}
if [[ $mode == replay ]]; then
    # Prefer bounded historical changes, without pretending ineligible revisions
    # were reproduced. The compiler/desired-image check makes the actual choice.
    : > "$fixture/candidates"
    while read -r revision parent rest; do
        [[ -n ${parent:-} ]] || continue
        changes=$(git -C "$repository" diff --numstat "$parent" "$revision" -- src library |
            awk '{ if ($1 ~ /^[0-9]+$/ && $2 ~ /^[0-9]+$/) total += $1 + $2; else total += 1000000 } END { print total + 0 }')
        [[ $changes != 0 ]] || continue
        printf '%012d\t%s\t%s\n' "$changes" "$revision" "$parent" >> "$fixture/candidates"
    done < "$fixture/revisions"
    sort -n "$fixture/candidates" > "$fixture/full-ordered"
    verify_record "$fixture/revisions" dfe6dd8098ed931ce5ccd847cab1ab6411d769c6b4c22a80bc8682babbc0cc4b
    verify_record "$fixture/full-ordered" 1bc2a477d3655301f30edef8a3d18b468d3af2489b06d8879838ebc224bc89e4
    awk 'NR==104||NR==107||NR==112||NR==158 {print NR"\t"$0}' "$fixture/full-ordered" > "$fixture/ordered"
    verify_record "$fixture/ordered" 1241ad75095c3492af9ab18b9435d88bc6797d74b03590622264498c5ce3f5bd
else
    awk -v wanted="$count" 'NF >= 2 { print NR "\t" $1 "\t" $2; if (++seen == wanted) exit; }' "$fixture/revisions" > "$fixture/ordered"
fi
while IFS=$'\t' read -r original_ordinal weight revision parent; do
    if [[ -e "$output.stop" ]]; then printf 'stop_requested=true\n' >> "$output.meta"; break; fi
    if [[ $passed -ge $count ]]; then break; fi
    if rg -Fxq "$revision" "$fixture/completed"; then continue; fi
    ((attempted += 1))
    rm -rf "$fixture/before" "$fixture/after"
    projection=native-lkj
    library_environment=not-loaded
    source_root=unavailable
    if ! free_space "$parent" "$revision"; then
        printf '%s\t%s\t%s\trefused\t%s\t0\t0\t-\t-\t-\t-\t-\t-\tE_HISTORY_SPACE\tnot-loaded\tunavailable\n' \
            "$attempted" "$revision" "$parent" "$projection" >> "$output"
        break
    fi
    prep=0
    materialize "$parent" "$fixture/before" || prep=$?
    if [[ $prep == 0 ]]; then materialize "$revision" "$fixture/after" || prep=$?; fi
    stem="$output.logs/$(printf '%03d' "$attempted")-$revision"
    printf '%s\n' "$original_ordinal" > "$stem.original-ordinal"
    if [[ $prep == 0 ]]; then
        if [[ -f "$fixture/before/src/main.lkj" && -f "$fixture/after/src/main.lkj" ]]; then
            source_root=src/main.lkj
        elif [[ -f "$fixture/before/src/lib.lkj" && -f "$fixture/after/src/lib.lkj" ]]; then
            source_root=src/lib.lkj
        else
            prep=14
        fi
    fi
    if [[ $prep != 0 ]]; then
        printf '%s\t%s\t%s\tineligible\t%s\t0\t0\t-\t-\t-\t-\t-\t-\tarchive-or-projection-%s\tnot-loaded\tunavailable\n' \
            "$attempted" "$revision" "$parent" "$projection" "$prep" >> "$output"
        continue
    fi
    git -C "$repository" diff --name-only --no-renames -z "$parent" "$revision" > "$stem.all-changed-paths"
    perl -0ne 'print if m{^(src|library)/} || /\.(lkj|rs)\x00$/;' "$stem.all-changed-paths" > "$stem.changed-paths"
    load_before=$(cut -d ' ' -f 1 /proc/loadavg)
    if [[ $mode == replay ]]; then
        args=(edit "$source_root" --source-dir "$fixture/before"
            --against "$fixture/after/$source_root")
    else
        args=(compare "$fixture/after/$source_root"
            --against "$fixture/before/$source_root" --limit 20)
    fi
    if [[ -f "$fixture/before/library/lib.lkj" && -f "$fixture/after/library/lib.lkj" ]]; then
        library_environment=historical-roots
        if [[ $mode == replay ]]; then
            args+=(--library-dir "$fixture/before/library" --against-library-dir "$fixture/after/library")
        else
            args+=(--library-dir "$fixture/after/library" --against-library-dir "$fixture/before/library")
        fi
    else
        # The copied compiler is frozen for the entire campaign. Omitting both
        # library roots selects its same embedded standard library on both sides.
        # Historical source bytes, including include data, remain unchanged.
        library_environment=frozen-compiler-embedded
    fi
    args+=(--store "$fixture/store" --store-bytes "$store_bytes" --store-key "$private_key" --stats)
    result=0
    (
        cd "$fixture"
        /usr/bin/time -q -f '%e\t%U\t%S\t%M' -o "$stem.time" \
            prlimit --as="$memory_bytes" -- timeout --kill-after=5s "$timeout_seconds" \
            "$compiler" "${args[@]}" > "$stem.out" 2> "$stem.err"
    ) || result=$?
    load_after=$(cut -d ' ' -f 1 /proc/loadavg)
    lines=$(wc -l < "$stem.out")
    status=ineligible
    operations=0
    reason="exit-$result"
    if [[ $result == 0 ]]; then
        if [[ $mode == replay ]]; then
            if operations=$(perl -ne 'if (/^replay desired=([0-9a-f]{64}) candidate=([0-9a-f]{64}) verified=true operations=([0-9]+)$/ && $1 eq $2) { $count=$3; } END { if ($count) { print $count; } else { exit 1; } }' "$stem.out"); then
                # A proof for a selected root does not prove changes outside
                # that closure. Every changed source/library file must appear
                # in the applied transaction's exact file facts.
                if perl - "$stem.out" "$stem.changed-paths" "$fixture/before" > "$stem.scope" <<'PERL'
use strict; use warnings; use JSON::PP;
my ($plan_path, $changed_path, $before) = @ARGV;
open my $plan, '<:raw', $plan_path or die $!;
my %applied;
while (<$plan>) {
    if (/^("(?:\\.|[^"])*") level=source-claim file before=[0-9a-f]{64} after=[0-9a-f]{64}$/) {
        $applied{decode_json($1)} = 1;
    }
}
open my $changed, '<:raw', $changed_path or die $!;
local $/ = "\0";
my $missing = 0;
while (<$changed>) {
    chomp;
    my $historical = $_;
    s/\.rs$/.lkj/;
    unless ($applied{"$before/$_"}) { print "$historical\n"; $missing++; }
}
exit($missing ? 1 : 0);
PERL
                then
                    status=passed
                    reason=desired-checked-meaning-identical
                else
                    reason=changed-required-source-outside-applied-operation-files
                fi
            else
                status=implementation-failure
                reason=missing-replay-equality-proof
            fi
        elif [[ $lines -le 30 ]] && rg -q '^program level=checked-ir identity-domain=' "$stem.out" \
            && rg -q '^program level=checked-ir verdict=' "$stem.out" \
            && rg -q '^program changed-facts=' "$stem.out"; then
            status=passed
            reason=bounded-comparison
        else
            status=implementation-failure
            reason=missing-or-unbounded-comparison
        fi
    else
        reason=$(perl -ne 'if (/(E_[A-Z0-9_]+)/) { print $1; exit; }' "$stem.err")
        [[ -n $reason ]] || reason="exit-$result"
        if rg -q 'E_REPLAY_(SOURCE|SPAN).*(duplicate source path|conflicting|absent from snapshot|does not match source)' "$stem.err"; then
            status=implementation-failure
        fi
        if [[ -e "$fixture/before/.lkjscript-edit.pending" ]]; then
            (
                cd "$fixture"
                prlimit --as="$memory_bytes" -- timeout --kill-after=5s "$timeout_seconds" \
                    "$compiler" edit "$source_root" --source-dir "$fixture/before" \
                    --recover --no-store --store-key "$private_key"
            ) > "$stem.recovery" 2>&1
        fi
    fi
    metrics=$(tail -n 1 "$stem.time")
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$attempted" "$revision" "$parent" "$status" "$projection" "$operations" "$lines" "$metrics" \
        "$load_before" "$load_after" "$reason" "$library_environment" "$source_root" >> "$output"
    if [[ $status == passed ]]; then ((passed += 1)); fi
    printf 'history mode=%s attempted=%s passed=%s revision=%s result=%s reason=%s\n' \
        "$mode" "$attempted" "$passed" "$revision" "$status" "$reason"
    if [[ $status == implementation-failure ]]; then
        printf 'implementation_failure=%s\n' "$reason" >> "$output.meta"
        break
    fi
    if [[ $mode == replay && $passed -ge $count ]]; then break; fi
done < "$fixture/ordered"
printf 'attempted=%s passed=%s expected_probes=%s\n' "$attempted" "$passed" "$count" >> "$output.meta"
[[ $attempted -eq 4 ]] && ! rg -q '^implementation_failure=' "$output.meta"
