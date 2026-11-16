#!/usr/bin/env bash
set -euo pipefail
umask 077
export PATH=/usr/bin:/bin LC_ALL=C TZ=UTC
unset LKJSCRIPT_STORE_IO
REPO=/home/coder/workspace/lkjscript
SETUP="$REPO/target/compiler-reference-capture-measurement-2"
RUN="$REPO/target/compiler-reference-capture-comparison-2"
PREP=$(cd -- "$(dirname -- "$0")" && pwd -P)
if [[ $# != 2 || "$1" != --execute-after-root-release || ! "$2" =~ ^[0-9a-f]{64}$ ]]; then
    printf '%s\n' 'Preparation only. A distinct exact cohort2 release hash is required.' >&2
    exit 64
fi
[[ -f "$PREP/release.json" && ! -L "$PREP/release.json" ]] || exit 64
[[ $(sha256sum "$PREP/release.json" | cut -d' ' -f1) == "$2" ]] || exit 65
release_fields=$(perl -MJSON::PP -e '
    local $/; open my $f,"<:raw",$ARGV[0] or die $!; my $r=decode_json(<$f>);
    die "not a released cohort\n" unless $r->{released} && $r->{status} eq "ROOT_RELEASED_ONCE"
        && $r->{cohort} eq "compiler-reference-capture-20261116-2";
    die "manifest pin missing\n" unless $r->{controls_manifest_sha256}=~/\A[0-9a-f]{64}\z/;
    print $r->{controls_manifest_sha256};
' "$PREP/release.json")
[[ $(sha256sum "$PREP/controls.manifest.sha256" | cut -d' ' -f1) == "$release_fields" ]] || exit 65
for name in controls.json order.json protocol.md run.sh trial.pl controls.manifest.sha256; do
    [[ -f "$PREP/$name" && ! -L "$PREP/$name" ]] || exit 65
done
(cd "$PREP" && sha256sum --check controls.manifest.sha256) || exit 65
[[ ! -e "$RUN" && ! -L "$RUN" ]] || {
    printf '%s\n' 'Fixed cohort root exists; no retry or overwrite.' >&2
    exit 73
}
caller_cwd=$(pwd -P)
cd "$REPO"
[[ $(pwd -P) == "$REPO" && $(cd "$REPO/target" && pwd -P) == "$REPO/target" ]] || exit 73
mkdir "$RUN"
mkdir "$RUN/records" "$RUN/images" "$RUN/outputs" "$RUN/control"
phase=initialize
stop_reason=unexpected-capture-or-preparation-failure
finish() {
    local result=$?
    trap - EXIT HUP INT TERM
    set +e
    printf '%s\n' "$result" > "$RUN/records/status"
    printf '%s\n' "$phase" > "$RUN/records/last-phase"
    printf '%s\n' "$stop_reason" > "$RUN/records/stop-reason"
    date -u +%Y-%m-%dT%H:%M:%SZ > "$RUN/records/end.utc"
    date +%s%N > "$RUN/records/end.ns"
    cut -d' ' -f1-3 /proc/loadavg > "$RUN/records/load-after"
    printf 'ordinal\tinput\tlabel\tproducer\tphase\toutcome\tcommand_status\tvalidation_status\n' > "$RUN/records/phases.tsv"
    local ordinal=0 input label producer item record command_status validation_status outcome
    for input in new-100k compiler; do
        for label in A1 B1 B2 A2 A3 B3; do
            ordinal=$((ordinal+1))
            producer=${label:0:1}
            printf -v item '%02d-%s-%s' "$ordinal" "$input" "$label"
            record="$RUN/records/$item"
            command_status=unavailable
            validation_status=unavailable
            outcome=not-run
            if [[ -d "$record" ]]; then
                [[ ! -f "$record/status" ]] || IFS= read -r command_status < "$record/status"
                [[ ! -f "$record/validation.status" ]] || IFS= read -r validation_status < "$record/validation.status"
                if [[ -f "$record/refusal" ]]; then outcome=preflight-refusal
                elif [[ "$command_status" =~ ^[0-9]+$ && "$command_status" != 0 ]]; then outcome=failed
                elif [[ "$command_status" == 0 && "$validation_status" == 0 ]]; then outcome=passed
                else outcome=incomplete
                fi
            fi
            printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$ordinal" "$input" "$label" "$producer" "$item" "$outcome" "$command_status" "$validation_status" >> "$RUN/records/phases.tsv"
        done
    done
    if [[ "$result" == 0 && "$phase" == complete ]]; then
        printf '%s\n' 'complete: exactly twelve builds; all byte/input/producer controls passed' > "$RUN/records/completion"
    else
        printf '%s\n' 'stopped: preserve all rows/partial outputs; remaining builds not-run; no retry' > "$RUN/records/completion"
    fi
    exit "$result"
}
trap finish EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
printf '%s\n' "$PREP/run.sh" --execute-after-root-release "$2" > "$RUN/records/launch.argv-lines"
printf '%s\n' "$caller_cwd" > "$RUN/records/launch.cwd"
printf '%s\n' "$2" > "$RUN/records/release.sha256"
printf '%s\n' 'PATH=/usr/bin:/bin' 'LC_ALL=C' 'TZ=UTC' 'LKJSCRIPT_STORE_IO unset' 'no store or key; no cache reset' > "$RUN/records/environment"
date -u +%Y-%m-%dT%H:%M:%SZ > "$RUN/records/start.utc"
date +%s%N > "$RUN/records/start.ns"
cut -d' ' -f1-3 /proc/loadavg > "$RUN/records/load-before"
cp "$PREP/controls.json" "$PREP/order.json" "$PREP/protocol.md" "$PREP/run.sh" "$PREP/trial.pl" \
   "$PREP/controls.manifest.sha256" "$PREP/release.json" "$RUN/control/"
[[ $(sha256sum "$RUN/control/release.json" | cut -d' ' -f1) == "$2" ]]
[[ $(sha256sum "$RUN/control/controls.manifest.sha256" | cut -d' ' -f1) == "$release_fields" ]]
(cd "$RUN/control" && sha256sum --check controls.manifest.sha256)
LAST_STATUS=0
capture() {
    local name=$1 cwd=$2 record available result
    shift 2
    record="$RUN/records/$name"
    mkdir "$record"
    printf '%s\n' "$cwd" > "$record/cwd"
    printf '%s\n' /dev/null > "$record/stdin"
    perl -MJSON::PP -e 'print JSON::PP->new->canonical->encode(\@ARGV),"\n";' -- "$@" > "$record/argv.json"
    perl -MJSON::PP -e 'print JSON::PP->new->canonical->encode(\@ARGV),"\n";' -- \
        /usr/bin/time -f 'wall_s=%e\nuser_s=%U\nsystem_s=%S\nmax_rss_kib=%M\nexit=%x' -o "$record/time" \
        /usr/bin/timeout --kill-after=10s 900s /usr/bin/prlimit --as=4294967296 -- "$@" > "$record/wrapper-argv.json"
    available=$(df -B1 --output=avail "$RUN" | tail -n 1 | tr -d ' ')
    printf '%s\n' "$available" > "$record/free-bytes"
    if [[ ! "$available" =~ ^[0-9]+$ || "$available" -lt 3221225472 ]]; then
        printf '%s\n' '73' > "$record/status"
        printf '%s\n' 'preflight-refusal: fewer than 3 GiB free; command not launched' > "$record/refusal"
        LAST_STATUS=73
        return 0
    fi
    date -u +%Y-%m-%dT%H:%M:%SZ > "$record/start.utc"
    date +%s%N > "$record/start.ns"
    cut -d' ' -f1-3 /proc/loadavg > "$record/load-before"
    set +e
    (
        cd "$cwd" || exit 125
        [[ $(pwd -P) == "$cwd" ]] || exit 125
        pwd -P > "$record/cwd.actual"
        /usr/bin/time -f 'wall_s=%e\nuser_s=%U\nsystem_s=%S\nmax_rss_kib=%M\nexit=%x' -o "$record/time" \
            /usr/bin/timeout --kill-after=10s 900s /usr/bin/prlimit --as=4294967296 -- "$@" < /dev/null
    ) > "$record/stdout" 2> "$record/stderr"
    result=$?
    set -e
    printf '%s\n' "$result" > "$record/status"
    date -u +%Y-%m-%dT%H:%M:%SZ > "$record/end.utc"
    date +%s%N > "$record/end.ns"
    cut -d' ' -f1-3 /proc/loadavg > "$record/load-after"
    LAST_STATUS=$result
    if [[ ! -s "$record/time" ]]; then
        printf '%s\n' 'GNU time capture missing; original command status retained' > "$record/capture-error"
        if [[ "$result" == 0 ]]; then LAST_STATUS=70; fi
    fi
}
phase=preflight
capture preflight "$REPO" /usr/bin/perl "$RUN/control/trial.pl" prepare "$RUN" "$RUN/control/controls.json" "$RUN/control/release.json"
if [[ "$LAST_STATUS" != 0 ]]; then
    stop_reason=preflight-failed-before-any-measured-build
    exit "$LAST_STATUS"
fi
ordinal=0
for input in new-100k compiler; do
    if [[ "$input" == new-100k ]]; then
        source_root="$SETUP/generated-100k"
        entry=main.lkj
    else
        source_root="$SETUP/compiler-input"
        entry=src/main.lkj
    fi
    output="$RUN/outputs/$input.image"
    [[ ! -e "$output" && ! -L "$output" ]]
    for label in A1 B1 B2 A2 A3 B3; do
        ordinal=$((ordinal+1))
        producer=${label:0:1}
        printf -v phase '%02d-%s-%s' "$ordinal" "$input" "$label"
        capture "$phase" "$source_root" "$RUN/producer-$producer" build "$entry" \
            --source-dir "$source_root" --library-dir "$SETUP/compiler-input/library" \
            --runtime-source "$SETUP/compiler-input/library/runtime.lkj" \
            --no-store --inline-limit 0 --stats -o "$output"
        command_status=$LAST_STATUS
        if [[ -f "$RUN/records/$phase/refusal" ]]; then
            stop_reason=measured-command-not-launched-resource-preflight-refusal
            exit "$command_status"
        fi
        capture "validation-$phase" "$REPO" /usr/bin/perl "$RUN/control/trial.pl" row \
            "$RUN" "$RUN/control/controls.json" "$RUN/control/release.json" "$phase"
        validation_status=$LAST_STATUS
        printf '%s\n' "$validation_status" > "$RUN/records/$phase/validation.status"
        if [[ "$command_status" != 0 ]]; then
            stop_reason=measured-command-failed-original-status-preserved
            exit "$command_status"
        fi
        if [[ "$validation_status" != 0 ]]; then
            stop_reason=measured-capture-statistics-or-output-validation-failed
            exit "$validation_status"
        fi
    done
done
phase=postflight
capture postflight "$REPO" /usr/bin/perl "$RUN/control/trial.pl" complete "$RUN" "$RUN/control/controls.json" "$RUN/control/release.json"
if [[ "$LAST_STATUS" != 0 ]]; then
    stop_reason=postflight-input-producer-or-complete-cohort-control-failed
    exit "$LAST_STATUS"
fi
phase=complete
stop_reason=none
printf 'Completed exactly twelve builds at %s; all rows retained, no threshold applied.\n' "$RUN"
