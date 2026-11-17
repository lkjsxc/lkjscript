#!/usr/bin/env bash
# Explicit-package scale workload, outside the gate. One compiler process at a time.
set -euo pipefail
export LC_ALL=C
script_root=$(cd "$(dirname "$0")" && pwd -P)
repository=$(cd "$script_root/.." && pwd -P)
compiler=
results=
rungs=10000,100000,1000000,10000000
unit_lines=10000
memory_ceiling=8192
time_ceiling=3600
real=yes
generated_check=yes
continue_from=
while (($#)); do
    case "$1" in
        --repository) repository=$(realpath "$2"); shift 2 ;;
        --compiler) compiler=$(realpath "$2"); shift 2 ;;
        --results) results=$(realpath -m "$2"); shift 2 ;;
        --rungs) rungs=$2; shift 2 ;;
        --continue-from) continue_from=$(realpath "$2"); shift 2 ;;
        --unit-lines) unit_lines=$2; shift 2 ;;
        --memory-ceiling-mib) memory_ceiling=$2; shift 2 ;;
        --time-ceiling) time_ceiling=$2; shift 2 ;;
        --skip-real) real=no; shift ;;
        --skip-generated-check) generated_check=no; shift ;;
        *) printf 'unknown unit-scale option: %s\n' "$1" >&2; exit 2 ;;
    esac
done
[[ -n "$compiler" ]] || compiler="$repository/target/lkjscript"
[[ -n "$results" ]] || results="$repository/bench/unit-scale-results"
[[ -x "$compiler" && -d "$repository/library" && -f "$repository/src/main.lkj" ]] || exit 2
[[ "$memory_ceiling" =~ ^[1-9][0-9]*$ && "$time_ceiling" =~ ^[1-9][0-9]*$ && "$unit_lines" =~ ^[1-9][0-9]*$ ]] || exit 2
((unit_lines >= 100)) || exit 2
((memory_ceiling <= 8192)) || { echo 'shared-host memory ceiling must not exceed 8192 MiB' >&2; exit 2; }
IFS=, read -r -a sizes <<< "$rungs"
previous=0
for lines in "${sizes[@]}"; do
    [[ "$lines" =~ ^[1-9][0-9]*$ ]] && ((lines >= 100 && lines > previous && lines <= 10000000)) || exit 2
    previous=$lines
done
[[ ! -e "$results" ]] || { echo 'result directory already exists; a cohort is never overwritten' >&2; exit 2; }
mkdir -p "$repository/target"
mkdir -p "$(dirname "$results")"
mkdir "$results"
run=$(mktemp -d "$repository/target/unit-scale.XXXXXX")
key_root=$(mktemp -d /tmp/lkjscript-unit-scale-key.XXXXXX)
cleanup() {
    local status=$?
    trap - EXIT
    rm -rf -- "$run" "$key_root"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
umask 077
dd if=/dev/urandom of="$key_root/key" bs=32 count=1 status=none
cp "$compiler" "$run/producer"
chmod u-w "$run/producer"
producer="$run/producer"
producer_sha=$(sha256sum "$producer" | cut -d' ' -f1)
unset LKJSCRIPT_STORE LKJSCRIPT_STORE_KEY
{
    printf 'started_utc=%s\nrepository=%s\ncompiler=%s\nrungs=%s\n' "$(date -u +%FT%TZ)" "$repository" "$compiler" "$rungs"
    printf 'compiler_sha256=%s\n' "$producer_sha"
    printf 'source_commit='; git -C "$repository" rev-parse HEAD
    printf 'memory_ceiling_mib=%s\ntime_ceiling_s=%s\nmaximum_unit_lines=%s\n' "$memory_ceiling" "$time_ceiling" "$unit_lines"
    printf 'workload=connected explicit-unit DAG; separate from the historical shape-sampled ladder\n'
    if [[ "$generated_check" == yes ]]; then
        printf 'generated_phases=cold-build,cold-check,unchanged-build,body-build\n'
    else
        printf 'generated_phases=cold-build,unchanged-build,body-build\n'
        printf 'generated_cold_check=explicitly omitted; cold build independently checks every unit\n'
    fi
    printf 'ten_million=one cold-build invocation only; no fresh check, warm build or retry at that rung\n'
    printf 'memory=per-process virtual-address envelope; GNU time reports largest process RSS\n'
} > "$results/cohort.txt"
(cd "$repository" && rg --files library -g '*.lkj' -0 | sort -z | xargs -0 sha256sum) > "$results/library.sha256"
sha256sum "$script_root/unit-scale.sh" "$script_root/unit-scale-generate.pl" "$script_root/unit-scale-record.pl" "$script_root/unit-scale-real.pl" > "$results/harness.sha256"
git -C "$repository" status --short -- src library > "$results/source-status.txt"
current_memory=2048
((current_memory <= memory_ceiling)) || current_memory=$memory_ceiling
current_seconds=300
((current_seconds <= time_ceiling)) || current_seconds=$time_ceiling
rung=
rung_peak=0
rung_disk=0
cold_wall=0
fit_next_envelope() {
    local fitted
    fitted=$(perl -MMath::BigInt -MMath::BigRat - "$rung_peak" "$cold_wall" "$memory_ceiling" "$time_ceiling" <<'PERL'
use strict;
use warnings;
my ($rss, $wall, $memory_limit, $time_limit) = @ARGV;
my $maximum = Math::BigInt->new("9223372036854775807");
for my $value ($rss, $memory_limit, $time_limit) {
    die "invalid unit-scale envelope integer\n"
        unless $value =~ /\A[1-9][0-9]{0,18}\z/ && Math::BigInt->new($value) <= $maximum;
}
die "invalid unit-scale envelope duration\n"
    unless $wall =~ /\A(?:0|[1-9][0-9]{0,18})(?:[.][0-9]{1,2})?\z/;
my $duration = Math::BigRat->new($wall)->bmul(30)->badd(61);
die "unit-scale envelope duration exceeds integer representation\n" if $duration > $maximum;
my $memory = Math::BigInt->new($rss)->badd(1023)->bdiv(1024)->bmul(3)->badd(512);
$memory = Math::BigInt->new(1536) if $memory < 1536;
$memory = Math::BigInt->new($memory_limit) if $memory > $memory_limit;
my $seconds = $duration->as_int();
$seconds = Math::BigInt->new(300) if $seconds < 300;
$seconds = Math::BigInt->new($time_limit) if $seconds > $time_limit;
print "$memory\t$seconds\n";
PERL
    ) || return 2
    IFS=$'\t' read -r current_memory current_seconds <<< "$fitted"
}
predict_space() {
    # Compute the existing proportional prediction exactly, including values
    # whose prediction exceeds a shell integer. Only the fit flag is compared.
    perl -MMath::BigInt - "$1" "$previous_lines" "$previous_disk" "$2" <<'PERL'
use strict;
use warnings;
my ($lines, $previous, $disk, $free_kib) = @ARGV;
for my $value ($lines, $previous, $disk, $free_kib) {
    die "invalid unit-scale disk input\n" unless $value =~ /\A(?:0|[1-9][0-9]*)\z/;
}
my $size = Math::BigInt->new($lines);
die "invalid unit-scale rung\n" if $size < 100 || $size > 10000000;
my $required;
if (Math::BigInt->new($previous) > 0) {
    $required = Math::BigInt->new($disk)->bmul($size)->bdiv($previous)->bmul(2);
} else {
    $required = $size->copy()->bmul(128);
}
$required->badd(256 * 1024 * 1024);
my $free = Math::BigInt->new($free_kib)->bmul(1024);
my $fits = $free >= $required ? "yes" : "no";
print "$free\t$required\t$fits\n";
PERL
}
store_size() {
    local total=0 directory bytes
    for directory in "$rung/store-build" "$rung/store-check"; do
        if [[ -d "$directory" ]]; then
            bytes=$(du -sb "$directory" | cut -f1)
            total=$((total + bytes))
        fi
    done
    printf '%s\n' "$total"
}
run_phase() {
    local label=$1 phase=$2 lines=$3 metadata=$4 artifact=$5
    shift 5
    local prefix="$results/$label-$lines-$phase" before after status bytes=0 stores=0 peak disk
    read -r before _ < /proc/loadavg
    printf '%q ' "$@" > "$prefix.command"
    printf '\n' >> "$prefix.command"
    status=0
    /usr/bin/time -o "$prefix.time" -f '%e\t%U\t%S\t%M\t%x' \
        timeout --signal=TERM --kill-after=10s "${current_seconds}s" \
        bash -c 'ulimit -v "$1" || exit 125; shift; exec "$@"' unit-scale "$((current_memory * 1024))" "$@" \
        > "$prefix.stdout" 2> "$prefix.stderr" || status=$?
    read -r after _ < /proc/loadavg
    [[ "$status" != 0 || "$artifact" == '-' || ! -f "$artifact" ]] || bytes=$(stat -c%s "$artifact")
    if [[ -n "$rung" && -d "$rung" ]]; then
        stores=$(store_size)
        disk=$(du -sb "$rung" | cut -f1)
        ((disk <= rung_disk)) || rung_disk=$disk
    fi
    perl "$script_root/unit-scale-record.pl" "$results" "$label" "$phase" "$lines" "$metadata" "$prefix" \
        "$status" "$before" "$after" "$current_memory" "$current_seconds" "$bytes" "$stores"
    peak=$(awk -F'\t' 'NF == 5 && $4 ~ /^[0-9]+$/ { value=$4 } END { print value+0 }' "$prefix.time")
    ((peak <= rung_peak)) || rung_peak=$peak
    if [[ "$phase" == cold-build ]]; then cold_wall=$(awk -F'\t' 'NF == 5 { value=$1 } END { print value+0 }' "$prefix.time"); fi
    return "$status"
}
native_control() {
    local phase=$1 expected=$2 artifact=$3
    local prefix="$results/generated-$lines-$phase-native"
    timeout --signal=TERM --kill-after=10s 120s "$artifact" > "$prefix.stdout" 2> "$prefix.stderr"
    printf '%s\n' "$expected" > "$prefix.expected"
    cmp "$prefix.expected" "$prefix.stdout"
    [[ ! -s "$prefix.stderr" ]]
}
cd "$repository"
previous_lines=0
previous_disk=0
if [[ -n "$continue_from" ]]; then
    # Refuse changed or malformed evidence before any producer invocation,
    # including the real specimens that precede the generated rungs.
    [[ -f "$continue_from/next-rung.tsv" && -f "$continue_from/cohort.txt" ]] || exit 2
    grep -Eq '^finished_utc=[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$' "$continue_from/cohort.txt" || exit 2
    continuation_values=$(perl -MMath::BigInt -MMath::BigRat - "$continue_from/next-rung.tsv" "$unit_lines" "$producer_sha" "${sizes[0]}" <<'PERL'
use strict;
use warnings;
my ($path, $unit_lines, $producer, $next) = @ARGV;
sub refuse { die "invalid unit-scale continuation metadata: $_[0]\n" }
my $header = "source_lines\tworking_disk_bytes\tpeak_rss_kib\tcold_wall_s\tmaximum_unit_lines\tcompiler_sha256";
# The exact six-field representation bounds the read; there is no compiler budget.
my $maximum_bytes = length($header) + 1 + 7 + 19 + 19 + 22 + 19 + 64 + 5 + 1;
open my $file, "<:raw", $path or refuse("cannot read row");
my $count = read($file, my $bytes, $maximum_bytes + 1);
close $file;
refuse("oversized row") unless defined($count) && $count <= $maximum_bytes;
my ($row) = $bytes =~ /\A\Q$header\E\n([^\n]*)\n\z/;
refuse("expected one header and one complete row") unless defined($row);
my @values = split /\t/, $row, -1;
refuse("expected exactly six fields") unless @values == 6;
my ($lines, $disk, $rss, $wall, $width, $identity) = @values;
my $maximum = Math::BigInt->new("9223372036854775807");
for my $value ($lines, $disk, $rss, $width) {
    refuse("integer representation") unless $value =~ /\A[1-9][0-9]{0,18}\z/
        && Math::BigInt->new($value) <= $maximum;
}
my $prior = Math::BigInt->new($lines);
refuse("completed rung range") if $prior < 100 || $prior > 9999999;
refuse("next rung range") unless $next =~ /\A[1-9][0-9]{0,7}\z/
    && Math::BigInt->new($next) >= 100 && Math::BigInt->new($next) <= 10000000;
refuse("rung order") unless $prior < Math::BigInt->new($next);
refuse("duration representation")
    unless $wall =~ /\A(?:0|[1-9][0-9]{0,18})(?:[.][0-9]{1,2})?\z/;
my $duration = Math::BigRat->new($wall)->bmul(30)->badd(61);
refuse("duration overflow") if $duration > $maximum;
refuse("unit width changed") unless $width eq $unit_lines && Math::BigInt->new($width) >= 100;
refuse("producer changed") unless $identity =~ /\A[0-9a-f]{64}\z/ && $identity eq $producer;
print "$row\n";
PERL
    ) || exit 2
    IFS=$'\t' read -r previous_lines previous_disk rung_peak cold_wall prior_unit_lines prior_producer <<< "$continuation_values"
    cmp "$continue_from/library.sha256" "$results/library.sha256" || exit 2
    cmp "$continue_from/harness.sha256" "$results/harness.sha256" || exit 2
    printf 'source_lines\tworking_disk_bytes\tpeak_rss_kib\tcold_wall_s\tmaximum_unit_lines\tcompiler_sha256\n%s\n' \
        "$continuation_values" > "$results/continued-rung.tsv"
    printf 'continued_from=%s\n' "$continue_from" >> "$results/cohort.txt"
    fit_next_envelope
fi
generated_memory=$current_memory
generated_seconds=$current_seconds
if [[ "$real" == yes ]]; then
    current_memory=$memory_ceiling
    current_seconds=$time_ceiling
    for phase in check build; do
        command=("$producer" "$phase" src/main.lkj --source-dir "$repository" --library-dir "$repository/library" --no-store --stats)
        artifact=-
        if [[ "$phase" == build ]]; then
            artifact="$run/ordinary-compiler"
            command+=(--runtime-source "$repository/library/runtime.lkj" --inline-limit 64 -o "$artifact")
        fi
        if ! run_phase compiler "$phase" NA - "$artifact" "${command[@]}"; then
            printf 'compiler specimen refused; generated cohort continues\n' >> "$results/walls.txt"
        fi
    done
    real_source=examples/real_programs/levenshtein
    if [[ -f "$real_source/upstream.lkj" ]]; then
        cp "$real_source/upstream.json" "$results/levenshtein-historical-provenance.json"
        rung="$run/levenshtein-units"
        mkdir "$rung"
        perl "$script_root/unit-scale-real.pl" prepare "$repository" "$rung/source" \
            > "$results/workload-levenshtein-original-units.json"
        metadata="$results/workload-levenshtein-original-units.json"
        real_lines=$(perl -MJSON::PP -0777 -ne 'print decode_json($_)->{source_lines}' "$metadata")
        mkdir "$rung/store-build" "$rung/store-check"
        real_common=(units.lkj --source-dir "$rung/source" --library-dir "$repository/library" \
            --store-key "$key_root/key" --store-bytes 8589934592 --stats)
        if ! run_phase levenshtein-original-units cold-check "$real_lines" "$metadata" - \
            "$producer" check-units "${real_common[@]}" --store "$rung/store-check"; then
            printf 'original levenshtein unit check refused; see the exact diagnostic\n' >> "$results/walls.txt"
        fi
        artifact="$rung/program"
        if run_phase levenshtein-original-units cold-build "$real_lines" "$metadata" "$artifact" \
            "$producer" build-units "${real_common[@]}" --store "$rung/store-build" \
            --runtime-source "$repository/library/runtime.lkj" --grant output -o "$artifact"; then
            gzip -dc "$repository/tests/fixtures/real_programs_levenshtein.output.gz" > "$rung/expected.out"
            prefix="$results/levenshtein-original-units-native"
            printf '%q\n' "$artifact" > "$prefix.command"
            /usr/bin/time -o "$prefix.time" -f '%e\t%U\t%S\t%M\t%x' \
                timeout --signal=TERM --kill-after=10s 120s "$artifact" > "$prefix.stdout" 2> "$prefix.stderr"
            perl "$script_root/unit-scale-real.pl" verify-output "$rung/expected.out" \
                "$prefix.stdout" "$prefix.stderr" "$artifact" > "$prefix.control.json"
            rm "$prefix.stdout" "$prefix.stderr"
        else
            printf 'original levenshtein unit build refused; see the exact diagnostic\n' >> "$results/walls.txt"
        fi
        rm -rf -- "$rung"
        rung=
    else
        printf 'levenshtein specimen is not present in the selected repository revision\n' >> "$results/walls.txt"
    fi
    current_memory=$generated_memory
    current_seconds=$generated_seconds
fi
for lines in "${sizes[@]}"; do
    free_kib=$(df -Pk "$repository/target" | awk 'NR == 2 {print $4}')
    space_prediction=$(predict_space "$lines" "$free_kib") || exit 2
    IFS=$'\t' read -r free_bytes required_bytes disk_fits <<< "$space_prediction"
    if [[ "$disk_fits" != yes ]]; then
        printf '%s lines: pool-space wall, available=%s predicted_required=%s bytes\n' "$lines" "$free_bytes" "$required_bytes" >> "$results/walls.txt"
        break
    fi
    rung="$run/rung-$lines"
    mkdir "$rung"
    perl "$script_root/unit-scale-generate.pl" generate "$rung/source" "$lines" "$unit_lines" > "$results/workload-$lines.summary.json"
    cp "$rung/source/workload.json" "$results/workload-$lines.json"
    metadata="$results/workload-$lines.json"
    expected=$(perl -MJSON::PP -0777 -ne 'print decode_json($_)->{expected_baseline}' "$metadata")
    expected_edited=$(perl -MJSON::PP -0777 -ne 'print decode_json($_)->{expected_edited}' "$metadata")
    mkdir "$rung/store-build"
    if [[ "$generated_check" == yes ]]; then mkdir "$rung/store-check"; fi
    artifact="$rung/program"
    common=(units.lkj --source-dir "$rung/source" --library-dir "$repository/library" --store-key "$key_root/key" --store-bytes 8589934592 --stats)
    build=("$producer" build-units "${common[@]}" --store "$rung/store-build" --runtime-source "$repository/library/runtime.lkj" -o "$artifact")
    rung_peak=0
    rung_disk=0
    cold_wall=0
    if ((lines == 10000000)); then
        (set -o noclobber; printf '%s\n' "$(date -u +%FT%TZ)" > "$results/ten-million-attempt")
    fi
    if ! run_phase generated cold-build "$lines" "$metadata" "$artifact" "${build[@]}"; then
        printf '%s lines: cold build refused under %s MiB/%s s; see compiler diagnostic\n' "$lines" "$current_memory" "$current_seconds" >> "$results/walls.txt"
        break
    fi
    if ((lines == 10000000)); then
        printf '10000000 lines: the one cold build succeeded; other 10M phases were not attempted\n' >> "$results/walls.txt"
        break
    fi
    native_control baseline "$expected" "$artifact"
    cold_image=$(sha256sum "$artifact" | cut -d' ' -f1)
    if [[ "$generated_check" == yes ]]; then
        if ! run_phase generated cold-check "$lines" "$metadata" - "$producer" check-units "${common[@]}" --store "$rung/store-check"; then
            printf '%s lines: fresh checking refused; see diagnostic\n' "$lines" >> "$results/walls.txt"
            break
        fi
        rm -rf -- "$rung/store-check"
    fi
    if ! run_phase generated unchanged-build "$lines" "$metadata" "$artifact" "${build[@]}"; then
        printf '%s lines: unchanged build refused; see diagnostic\n' "$lines" >> "$results/walls.txt"
        break
    fi
    [[ "$(sha256sum "$artifact" | cut -d' ' -f1)" == "$cold_image" ]] || {
        printf '%s lines: warm image differs from cold image\n' "$lines" >> "$results/walls.txt"; exit 1;
    }
    perl "$script_root/unit-scale-generate.pl" edit "$rung/source" > "$results/body-edit-$lines.json"
    if ! run_phase generated body-build "$lines" "$metadata" "$artifact" "${build[@]}"; then
        printf '%s lines: one-body build refused; see diagnostic\n' "$lines" >> "$results/walls.txt"
        break
    fi
    native_control edited "$expected_edited" "$artifact"
    previous_lines=$lines
    previous_disk=$rung_disk
    # Retain actual envelope inputs so the one 10M attempt can follow this cohort.
    printf 'source_lines\tworking_disk_bytes\tpeak_rss_kib\tcold_wall_s\tmaximum_unit_lines\tcompiler_sha256\n%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$lines" "$rung_disk" "$rung_peak" "$cold_wall" "$unit_lines" "$producer_sha" > "$results/next-rung.tsv"
    fit_next_envelope
    rm -rf -- "$rung"
    rung=
done
printf 'finished_utc=%s\n' "$(date -u +%FT%TZ)" >> "$results/cohort.txt"
printf 'unit scale evidence: %s\n' "$results"
