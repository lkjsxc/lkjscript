#!/usr/bin/env bash
set -euo pipefail
umask 077
REPO=/home/coder/workspace/lkjscript
RUN="$REPO/target/compiler-reference-capture-measurement-1"
PREP=$(cd -- "$(dirname -- "$0")" && pwd -P)
if [[ $# != 2 || "$1" != --execute-after-root-release || ! "$2" =~ ^[0-9a-f]{64}$ ]]; then
    printf '%s\n' 'Preparation only. Exact root release and control-manifest hash are required.' >&2
    exit 64
fi
[[ $(sha256sum "$PREP/controls.manifest.sha256" | cut -d' ' -f1) == "$2" ]] || {
    printf '%s\n' 'Control manifest pin mismatch; no setup started.' >&2
    exit 65
}
(cd "$PREP" && sha256sum --check controls.manifest.sha256) || exit 65
[[ ! -e "$RUN" && ! -L "$RUN" ]] || {
    printf '%s\n' 'The fixed owned run root already exists; no retry or overwrite.' >&2
    exit 73
}
caller_cwd=$(pwd -P)
cd "$REPO"
[[ $(pwd -P) == "$REPO" ]] || exit 73
[[ $(realpath "$REPO/target") == "$REPO/target" ]] || exit 73
mkdir "$RUN"
mkdir "$RUN/records" "$RUN/artifacts" "$RUN/control"
phase=initialize
finish() {
    local result=$?
    trap - EXIT HUP INT TERM
    set +e
    printf '%s\n' "$result" > "$RUN/records/status"
    printf '%s\n' "$phase" > "$RUN/records/last-phase"
    date -u +%Y-%m-%dT%H:%M:%SZ > "$RUN/records/end.utc"
    date +%s%N > "$RUN/records/end.ns"
    cut -d' ' -f1-3 /proc/loadavg > "$RUN/records/load-after"
    local item record command_status capture_status outcome
    printf 'phase\toutcome\tcommand_status\tcapture_validation_status\n' > "$RUN/records/phases.tsv"
    for item in recover-inputs prepare-factory generator-build generate-100k freeze-artifacts; do
        record="$RUN/records/$item"
        command_status=unavailable
        capture_status=unavailable
        outcome=not-run
        if [[ -d "$record" ]]; then
            [[ ! -f "$record/status" ]] || IFS= read -r command_status < "$record/status"
            [[ ! -f "$record/validation.status" ]] || IFS= read -r capture_status < "$record/validation.status"
            if [[ -f "$record/refusal" ]]; then
                outcome=preflight-refusal
            elif [[ "$command_status" =~ ^[0-9]+$ && "$command_status" != 0 ]]; then
                outcome=failed
            elif [[ "$command_status" == 0 && "$capture_status" == 0 ]]; then
                outcome=ok
            else
                outcome=incomplete
            fi
        fi
        printf '%s\t%s\t%s\t%s\n' "$item" "$outcome" "$command_status" "$capture_status" >> "$RUN/records/phases.tsv"
    done
    if [[ "$result" == 0 ]]; then
        printf '%s\n' 'setup-complete; generated archive verified; A/B remains unreleased' > "$RUN/records/completion"
    else
        printf '%s\n' 'setup-stopped; preserve all partial files; no retry or cleanup' > "$RUN/records/completion"
    fi
    exit "$result"
}
trap finish EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

printf '%s\n' "$2" > "$RUN/records/released-controls.sha256"
printf '%s\n' "$caller_cwd" > "$RUN/records/launch.cwd"
printf '%s\n' "$PREP/setup.sh" --execute-after-root-release "$2" > "$RUN/records/launch.argv-lines"
date -u +%Y-%m-%dT%H:%M:%SZ > "$RUN/records/start.utc"
date +%s%N > "$RUN/records/start.ns"
cut -d' ' -f1-3 /proc/loadavg > "$RUN/records/load-before"
cp "$PREP/setup.sh" "$PREP/setup.pl" "$PREP/controls.json" "$PREP/README.md" \
   "$PREP/controls.manifest.sha256" "$RUN/control/"
[[ $(sha256sum "$RUN/control/controls.manifest.sha256" | cut -d' ' -f1) == "$2" ]]
(cd "$RUN/control" && sha256sum --check controls.manifest.sha256)

run_phase() {
    phase=$1
    local cwd=$2 input=$3 result record available validation_status
    shift 3
    record="$RUN/records/$phase"
    mkdir "$record"
    printf '%s\n' "$cwd" > "$record/cwd"
    printf '%s\n' "$input" > "$record/stdin"
    perl -MJSON::PP -e 'print JSON::PP->new->canonical->encode(\@ARGV), "\n";' -- "$@" > "$record/argv.json"
    perl -MJSON::PP -e 'print JSON::PP->new->canonical->encode(\@ARGV), "\n";' -- \
        /usr/bin/time -f 'wall_s=%e\nuser_s=%U\nsystem_s=%S\nmax_rss_kib=%M\nexit=%x' -o "$record/time" \
        /usr/bin/timeout --kill-after=10s 900s /usr/bin/prlimit --as=4294967296 -- "$@" > "$record/wrapper-argv.json"
    available=$(df -B1 --output=avail "$RUN" | tail -n 1 | tr -d ' ')
    printf '%s\n' "$available" > "$record/free-bytes"
    [[ "$available" =~ ^[0-9]+$ && "$available" -ge 3221225472 ]] || {
        printf '%s\n' '73' > "$record/status"
        printf '%s\n' 'preflight-refusal: fewer than 3 GiB free; command not launched' > "$record/refusal"
        return 73
    }
    date -u +%Y-%m-%dT%H:%M:%SZ > "$record/start.utc"
    date +%s%N > "$record/start.ns"
    cut -d' ' -f1-3 /proc/loadavg > "$record/load-before"
    set +e
    (
        cd "$cwd" || exit 125
        [[ $(pwd -P) == "$cwd" ]] || exit 125
        pwd -P > "$record/cwd.actual"
        /usr/bin/time -f 'wall_s=%e\nuser_s=%U\nsystem_s=%S\nmax_rss_kib=%M\nexit=%x' -o "$record/time" \
            /usr/bin/timeout --kill-after=10s 900s /usr/bin/prlimit --as=4294967296 -- \
            "$@" < "$input"
    ) > "$record/stdout" 2> "$record/stderr"
    result=$?
    set -e
    printf '%s\n' "$result" > "$record/status"
    date -u +%Y-%m-%dT%H:%M:%SZ > "$record/end.utc"
    date +%s%N > "$record/end.ns"
    cut -d' ' -f1-3 /proc/loadavg > "$record/load-after"
    sed -n '/^LKJSCRIPT_STATS /p' "$record/stdout" "$record/stderr" > "$record/stats.raw"
    [[ -s "$record/time" ]] || {
        printf '%s\n' 'missing GNU time capture; setup stopped' > "$record/capture-error"
        return 70
    }
    printf '%s\n' /usr/bin/perl "$RUN/control/setup.pl" validate "$RUN" "$RUN/control/controls.json" "$phase" "$result" > "$record/validation.argv-lines"
    set +e
    /usr/bin/perl "$RUN/control/setup.pl" validate "$RUN" "$RUN/control/controls.json" "$phase" "$result" \
        > "$record/validation.stdout" 2> "$record/validation.stderr"
    validation_status=$?
    set -e
    printf '%s\n' "$validation_status" > "$record/validation.status"
    if [[ "$result" != 0 ]]; then
        printf 'Setup stopped at %s with status %s; all records remain at %s\n' "$phase" "$result" "$RUN" >&2
        return "$result"
    fi
    [[ "$validation_status" == 0 ]] || return 70
}
export LC_ALL=C TZ=UTC
printf '%s\n' 'LC_ALL=C' 'TZ=UTC' 'no private key or store is created' > "$RUN/records/environment"
unset LKJSCRIPT_STORE_IO
run_phase recover-inputs "$REPO" /dev/null /usr/bin/perl "$RUN/control/setup.pl" recover "$RUN" "$RUN/control/controls.json"
run_phase prepare-factory "$REPO" /dev/null /usr/bin/perl "$RUN/control/setup.pl" factory "$RUN" "$RUN/control/controls.json"
run_phase generator-build "$RUN/factory" /dev/null "$RUN/producer-A" build src/main.lkj \
    --source-dir "$RUN/factory" --library-dir "$RUN/factory/library" \
    --runtime-source "$RUN/factory/library/runtime.lkj" --inline-limit 0 --no-store --stats -o "$RUN/generator"
run_phase generate-100k "$RUN" "$RUN/artifacts/shape" "$RUN/generator" generate "$RUN/generated-100k" 100000 20261116
run_phase freeze-artifacts "$REPO" /dev/null /usr/bin/perl "$RUN/control/setup.pl" freeze "$RUN" "$RUN/control/controls.json"
phase=complete
printf 'Setup complete at %s. No A/B build has run; root archive review and a distinct release remain required.\n' "$RUN"
