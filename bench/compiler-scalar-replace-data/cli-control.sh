#!/usr/bin/env bash
# Public scalar-selection and authenticated-reuse contracts.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath -e "${1:-target/lkjscript}")
mkdir -p target
trial=$(mktemp -d "$PWD/target/scalar-replacement-test.XXXXXX")
phase=prepare
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)); then
        printf 'scalar replacement failed during %s\n' "$phase" >&2
        if [[ -f $trial/$phase.log ]]; then tail -25 "$trial/$phase.log" >&2; fi
    fi
    rm -rf -- "$trial"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
umask 077
mkdir "$trial/source" "$trial/store" "$trial/cases"
head -c 32 /dev/urandom > "$trial/key"
cat > "$trial/source/main.lkj" <<'SOURCE'
// expect: value 42
struct Pair { left: i64, right: Box<i64> }
fn make() -> Pair { Pair { left: 20i64, right: Box::new(20i64) } }
fn total(pair: Pair) -> i64 { pair.left + *pair.right }
pub fn probe() -> i64 {
    let mut pair = make();
    let saved = pair.left;
    pair.left = 22i64;
    assert_eq!(saved, 20i64);
    total(pair)
}
SOURCE
cat > "$trial/source/invalid.lkjir" <<'IR'
lkjscript-ir 1
type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
fn "probe" 0 3
local "result" 3 let
@1 return 0
endfn
IR
fail() { printf 'scalar replacement: %s\n' "$*" >&2; exit 1; }
field() {
    sed -n "s/.*\"$2\":\([0-9][0-9]*\).*/\1/p" "$1" |
        awk '{ count += $1 } END { print count + 0 }'
}
build() {
    phase=$1
    shift
    "$compiler" build main.lkj --source-dir "$trial/source" --no-library --entry probe \
        --store "$trial/store" --store-key "$trial/key" --stats -o "$trial/image" "$@" \
        > "$trial/$phase.out" 2> "$trial/$phase.log"
}

# The source is checked before any optimizer can propose a replacement.
phase=invalid
if "$compiler" build "$trial/source/invalid.lkjir" --source-dir "$trial/source" --entry probe \
    --no-library --scalar-replace --stats --no-store -o "$trial/invalid-image" \
    > "$trial/$phase.out" 2> "$trial/$phase.log"; then
    fail 'invalid original was admitted'
fi
grep -q E_MOVE "$trial/$phase.log" || fail 'original rejection changed'
(( $(field "$trial/$phase.log" scalar_attempts) == 0 )) || fail 'replacement ran before original admission'
[[ ! -e $trial/invalid-image ]] || fail 'invalid original produced an image'

# A unit command must refuse an unavailable transformation before reading input.
for command in check-units build-units; do
    phase=$command
    if "$compiler" "$command" missing-manifest.lkj --source-dir "$trial/source" \
        --store "$trial/store" --store-key "$trial/key" --scalar-replace \
        > "$trial/$phase.out" 2> "$trial/$phase.log"; then
        fail "$command silently ignored scalar replacement"
    fi
    grep -q 'E_USAGE.*optimization' "$trial/$phase.log" || fail "$command did not explain its refusal"
done

# Selected native images have distinct complete-request identities.
for policy in original scalar; do
    options=(--inline-limit 0)
    if [[ $policy == scalar ]]; then options+=(--scalar-replace); fi
    build "$policy-cold" "${options[@]}"
    (( $(field "$trial/$phase.log" build_reused) == 0 )) || fail "$policy reused a different policy"
    if [[ $policy == scalar ]]; then
        (( $(field "$trial/$phase.log" scalar_accepted) > 0 )) || fail 'candidate was not independently admitted'
        (( $(field "$trial/$phase.log" scalar_rejected) == 0 )) || fail 'candidate fell back'
        (( $(field "$trial/$phase.log" scalar_aggregates) > 0 )) || fail 'candidate transformed no aggregates'
    fi
    cp "$trial/image" "$trial/$policy-image"
    build "$policy-warm" "${options[@]}"
    (( $(field "$trial/$phase.log" build_reused) > 0 )) || fail "$policy did not reuse its own image"
    cmp "$trial/$policy-image" "$trial/image"
done
if cmp -s "$trial/original-image" "$trial/scalar-image"; then fail 'selection changed no native bytes'; fi
build disabled --scalar-replace --no-opt
(( $(field "$trial/$phase.log" build_reused) > 0 )) || fail '--no-opt did not select the original cached identity'
cmp "$trial/original-image" "$trial/image"

# Original observations and the independent interpreter remain stable.
for command in ir authority; do
    for policy in original scalar; do
        phase=$command-$policy
        options=(--no-opt)
        if [[ $policy == scalar ]]; then options=(--scalar-replace); fi
        "$compiler" "$command" main.lkj --source-dir "$trial/source" --no-library --entry probe \
            --no-store "${options[@]}" > "$trial/$phase.out" 2> "$trial/$phase.log"
    done
    cmp "$trial/$command-original.out" "$trial/$command-scalar.out"
done
for engine in native interpreter; do
    phase=run-$engine
    "$compiler" run main.lkj --source-dir "$trial/source" --no-library --entry probe \
        --engine "$engine" --scalar-replace --no-store \
        > "$trial/$phase.out" 2> "$trial/$phase.log"
    [[ $(cat "$trial/$phase.out") == 42 ]] || fail "$engine value changed"
done

# Exact execution replay is separated by selection policy as well.
printf 'run\tprobe\tmain.lkj\n' > "$trial/batch"
for policy in original scalar; do
    options=(--inline-limit 0)
    if [[ $policy == scalar ]]; then options+=(--scalar-replace); fi
    for repetition in cold warm; do
        phase=batch-$policy-$repetition
        "$compiler" batch "$trial/batch" --source-dir "$trial/source" --no-library \
            --engines native,interpreter --roundtrip --store "$trial/store" --store-key "$trial/key" \
            --stats "${options[@]}" > "$trial/$phase.out" 2> "$trial/$phase.log"
        grep -q '"checkStatus":0,"status":0,"stdoutHex":"34320a"' "$trial/$phase.out" ||
            fail "$policy execution value changed"
    done
    (( $(field "$trial/batch-$policy-cold.log" test_reused) == 0 )) || fail "$policy replayed another execution policy"
    (( $(field "$trial/batch-$policy-warm.log" test_reused) > 0 )) || fail "$policy execution was not reused"
    cmp "$trial/batch-$policy-cold.out" "$trial/batch-$policy-warm.out"
done

# The external test worker receives the explicit option.
cp "$trial/source/main.lkj" "$trial/cases/aggregate.lkj"
phase=worker
"$compiler" test "$trial/cases" --library-dir "$PWD/library" --engines native,interpreter \
    --jobs 1 --scalar-replace --stats --no-store > "$trial/$phase.out" 2> "$trial/$phase.log"
grep -q '1 cases, 2 route checks, 0 pending' "$trial/$phase.out" || fail 'worker case did not pass both routes'
(( $(field "$trial/$phase.log" scalar_accepted) > 0 )) || fail 'worker did not receive the scalar option'
printf 'ok scalar replacement: original admission, unit refusal, image and execution identities, original observations, worker selection\n'
