#!/usr/bin/env bash
# Supplemental exact-filename roots from the same frozen 300-commit population.
# This does not change or extend the original 179 compiler-root result table.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ $# == 3 ]] || { printf 'usage: %s REPOSITORY FROZEN_COMPILER OUTPUT_PREFIX\n' "$0" >&2; exit 2; }
repository=$(realpath -e "$1")
compiler=$(realpath -e "$2")
output=$(realpath -m "$3")
frozen_head=1f48bd24c90f98e833c3647d902ea0b4515d4d90
expected_producer=4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09
memory_bytes=4294967296
timeout_seconds=900
store_bytes=2147483648
[[ -x $compiler && ! -e $output.tsv && ! -e $output.meta && ! -e $output.logs ]] || exit 2
[[ $(sha256sum "$compiler" | cut -d ' ' -f 1) == "$expected_producer" ]] || { echo E_ROOTS_PRODUCER >&2; exit 1; }
mkdir -p target "$(dirname "$output")"
free_kib=$(df -Pk target | awk 'NR==2 {print $4}')
(( free_kib >= 3 * 1024 * 1024 )) || { echo E_ROOTS_SPACE >&2; exit 1; }
umask 077
fixture=$(mktemp -d "$PWD/target/compiler-replay-roots.XXXXXX")
mkdir "$output.logs"
finish() {
    local status=$?
    trap - EXIT HUP INT TERM
    printf 'exit=%s\n' "$status" >> "$output.meta"
    rm -rf -- "$fixture"
    exit "$status"
}
trap finish EXIT
trap 'exit 130' HUP INT TERM
cp --reflink=auto -- "$compiler" "$fixture/compiler"
chmod 500 "$fixture/compiler"
compiler="$fixture/compiler"
[[ $(sha256sum "$compiler" | cut -d ' ' -f 1) == "$expected_producer" ]] || exit 1
head -c 32 /dev/urandom > "$fixture/key"
git -C "$repository" log -n 300 --format=%H "$frozen_head" > "$fixture/population"
cat > "$output.candidates.tsv" <<'ROWS'
revision	root	entry	changed_source
43cc8ae9c56bd0951548cece918fc7921d39bfd6	tests/conformance/migrated/backend_vec_probe.rs	probe	tests/conformance/migrated/backend_vec_probe.rs
60b270ebb91074ea60f909f84676bc1d2d8d198b	tests/conformance/output/process_capture.rs	main	tests/conformance/output/process_capture.rs
70680680009787b27640fbafa450f89c82a8b5e2	tests/conformance/output/lifecycle.lkj	main	tests/conformance/output/lifecycle.lkj
075d9801c377e317002548c7edee88a4c9ef2269	examples/http.rs	main	examples/http_server.rs
ROWS
{
    printf 'frozen_population=%s\ncandidates=4\noriginal_compiler_population=179\noriginal_results_unchanged=true\n' "$frozen_head"
    printf 'producer_sha256=%s\nproducer_bytes=%s\nharness_sha256=%s\n' "$expected_producer" "$(stat -c %s "$compiler")" "$(sha256sum "$0" | cut -d ' ' -f 1)"
    printf 'workers=1\nmemory_bytes=%s\ntimeout_s=%s\ntermination_grace_s=5\nstore_bytes=%s\n' "$memory_bytes" "$timeout_seconds" "$store_bytes"
    printf 'filename_projection=none\nsource_adaptation=none\nlegacy_aliases=none\nprogram_execution=false\nfree_space_kib_before=%s\n' "$free_kib"
    printf 'historical_library_trial=always\nembedded_library_fallback=separate_diagnostic_only\n'
    printf 'exact_ir_trial=same_physical_path_desired_and_applied_candidate\n'
} > "$output.meta"
printf 'revision\tstage\tlibrary_environment\texit\twall_s\tuser_s\tsystem_s\tpeak_kib\tload_before\tload_after\n' > "$output.tsv"
printf 'revision\tparent\troot\thistorical_admitted\tembedded_admitted\tdry_run\tpublication\tcoverage\texact_ir_equal\treplay_proof\tclassification\n' > "$output.results.tsv"
load() { cut -d ' ' -f 1-3 /proc/loadavg; }
run() {
    local revision=$1 stage=$2 environment=$3
    shift 3
    local stem="$output.logs/$revision.$stage" before after status=0
    before=$(load)
    printf '%q ' "$@" > "$stem.args"
    printf '\n' >> "$stem.args"
    /usr/bin/time -q -f '%e\t%U\t%S\t%M' -o "$stem.time" \
        prlimit --as="$memory_bytes" -- timeout --kill-after=5s "$timeout_seconds" "$@" \
        > "$stem.out" 2> "$stem.err" || status=$?
    after=$(load)
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$revision" "$stage" "$environment" "$status" \
        "$(tail -n 1 "$stem.time")" "$before" "$after" | tee -a "$output.tsv"
    RUN_STATUS=$status
}
materialize() {
    local revision=$1 destination=$2 root=$3
    mkdir "$destination"
    local paths=()
    while IFS= read -r path; do
        [[ $path == *.lkj || $path == *.rs ]] || continue
        if [[ $path == library/* || $path == "$root" || ( $root == examples/* && $path == examples/* ) ]]; then
            paths+=("$path")
        fi
    done < <(git -C "$repository" ls-tree -r --name-only "$revision" -- library "$root" examples)
    [[ ${#paths[@]} -gt 0 ]] || return 1
    git -C "$repository" archive "$revision" -- "${paths[@]}" | tar -xf - -C "$destination"
    [[ -z $(find "$destination" -type l -print -quit) ]] || return 1
}
source_manifest() {
    local root=$1
    ( cd "$root"; find . -type f \( -name '*.rs' -o -name '*.lkj' \) -print0 | sort -z | xargs -0 sha256sum )
}
while IFS=$'\t' read -r revision root entry changed; do
    [[ $revision == revision ]] && continue
    grep -Fxq "$revision" "$fixture/population"
    parent=$(git -C "$repository" rev-parse "$revision^1")
    [[ -z $(git -C "$repository" diff --name-only "$parent" "$revision" -- src library) ]] || exit 1
    git -C "$repository" diff --name-status "$parent" "$revision" > "$output.logs/$revision.commit-paths"
    ordinary_changes=$(git -C "$repository" diff --name-only "$parent" "$revision" -- examples tests/conformance | rg '\.(rs|lkj)$' || true)
    [[ $ordinary_changes == "$changed" ]] || { echo E_ROOTS_COVERAGE >&2; exit 1; }
    materialize "$parent" "$fixture/before" "$root"
    materialize "$revision" "$fixture/after" "$root"
    source_manifest "$fixture/before" > "$output.logs/$revision.before.sha256"
    source_manifest "$fixture/after" > "$output.logs/$revision.desired.sha256"
    library_environment=historical-roots
    before_library=(--library-dir "$fixture/before/library")
    after_library=(--library-dir "$fixture/after/library")
    replay_library=(--library-dir "$fixture/before/library" --against-library-dir "$fixture/after/library")
    historical_admitted=no
    embedded_admitted=not-run
    dry=not-run publication=not-run coverage=not-run exact_ir=not-run proof=- classification=historical-admission-refused
    run "$revision" historical-before "$library_environment" "$compiler" check "$root" --source-dir "$fixture/before" "${before_library[@]}" --entry "$entry" --no-store --stats
    before_status=$RUN_STATUS
    run "$revision" historical-after "$library_environment" "$compiler" check "$root" --source-dir "$fixture/after" "${after_library[@]}" --entry "$entry" --no-store --stats
    after_status=$RUN_STATUS
    if ((before_status == 0 && after_status == 0)); then
        historical_admitted=yes
    else
        # Exact source bytes/names remain unchanged. This is explicitly a different,
        # current standard-library environment and cannot become a historical replay.
        library_environment=frozen-current-embedded-diagnostic
        before_library=()
        after_library=()
        replay_library=()
        run "$revision" embedded-before "$library_environment" "$compiler" check "$root" --source-dir "$fixture/before" --entry "$entry" --no-store --stats
        before_status=$RUN_STATUS
        run "$revision" embedded-after "$library_environment" "$compiler" check "$root" --source-dir "$fixture/after" --entry "$entry" --no-store --stats
        after_status=$RUN_STATUS
        if ((before_status == 0 && after_status == 0)); then embedded_admitted=yes; else embedded_admitted=no; fi
    fi
    if ((before_status == 0 && after_status == 0)); then
        common=(edit "$root" --source-dir "$fixture/before" --against "$fixture/after/$root"
            "${replay_library[@]}" --entry "$entry" --store "$fixture/store"
            --store-bytes "$store_bytes" --store-key "$fixture/key" --stats)
        run "$revision" dry-run "$library_environment" "$compiler" "${common[@]}" --dry-run
        dry=$RUN_STATUS
        classification=planning-refused
        if ((dry == 0)); then
            run "$revision" apply "$library_environment" "$compiler" "${common[@]}"
            publication=$RUN_STATUS
            classification=publication-refused
            if ((publication == 0)); then
                stem="$output.logs/$revision.apply.out"
                proof=$(sed -n 's/^replay desired=\([0-9a-f]*\) candidate=\([0-9a-f]*\) verified=true operations=\([0-9]*\)$/\1,\2,\3/p' "$stem")
                IFS=, read -r desired_identity candidate_identity operation_count <<< "$proof"
                [[ $desired_identity =~ ^[0-9a-f]{64}$ && $desired_identity == "$candidate_identity" && $operation_count =~ ^[1-9][0-9]*$ ]] || exit 1
                grep -Eq '^transaction source-before=[0-9a-f]{64} source-after=[0-9a-f]{64} mode=apply$' "$stem"
                coverage=no
                if perl -MJSON::PP -e '
                    my ($report,$wanted)=@ARGV;
                    open my $fh,"<",$report or die $!;
                    while(<$fh>) {
                        if (/^("(?:\\.|[^"])*") level=source-claim file before=[0-9a-f]{64} after=[0-9a-f]{64}$/) {
                            exit 0 if decode_json($1) eq $wanted;
                        }
                    }
                    exit 1;
                ' "$stem" "$fixture/before/$changed"; then coverage=yes; fi
                cp -a "$fixture/after" "$fixture/active"
                active_library=()
                if [[ $library_environment == historical-roots ]]; then active_library=(--library-dir "$fixture/active/library"); fi
                run "$revision" exact-desired "$library_environment" "$compiler" ir "$root" --source-dir "$fixture/active" "${active_library[@]}" --entry "$entry" --no-store -o "$fixture/desired.lkjir" --stats
                [[ $RUN_STATUS == 0 ]] || exit 1
                rm -rf -- "$fixture/active"
                cp -a "$fixture/before" "$fixture/active"
                run "$revision" exact-candidate "$library_environment" "$compiler" ir "$root" --source-dir "$fixture/active" "${active_library[@]}" --entry "$entry" --no-store -o "$fixture/candidate.lkjir" --stats
                [[ $RUN_STATUS == 0 ]] || exit 1
                exact_ir=no
                if cmp -s "$fixture/desired.lkjir" "$fixture/candidate.lkjir"; then exact_ir=yes; fi
                sha256sum "$fixture/desired.lkjir" "$fixture/candidate.lkjir" > "$output.logs/$revision.exact-ir.sha256"
                if [[ $historical_admitted == yes && $coverage == yes && $exact_ir == yes ]]; then
                    classification=strict-applied-exact-ir
                else classification=applied-limited-proof; fi
            fi
        fi
    fi
    source_manifest "$fixture/before" > "$output.logs/$revision.final.sha256"
    if [[ $publication != 0 ]]; then cmp "$output.logs/$revision.before.sha256" "$output.logs/$revision.final.sha256"; fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$revision" "$parent" "$root" \
        "$historical_admitted" "$embedded_admitted" "$dry" "$publication" "$coverage" "$exact_ir" "$proof" "$classification" >> "$output.results.tsv"
    rm -rf -- "$fixture/before" "$fixture/after" "$fixture/active" "$fixture/store"
    rm -f -- "$fixture/desired.lkjir" "$fixture/candidate.lkjir"
done < "$output.candidates.tsv"
printf 'completed_candidates=4\n' >> "$output.meta"
