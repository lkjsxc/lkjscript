#!/usr/bin/env bash
# Attack execution proofs separately from compiler verdict reuse.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
runner=${2:-target/gate/closed/runner}
test_directory=$(mktemp -d target/gate-reuse.XXXXXX)
trap 'rm -rf "$test_directory"' EXIT HUP INT TERM
mkdir "$test_directory/source" "$test_directory/store"
key="$test_directory/key"
(umask 077; head -c 32 /dev/urandom > "$key")
contract=tools/closed-semantics.sh
store="$test_directory/store"
fail() { echo "gate reuse: $*" >&2; exit 1; }
expect() {
    local label=$1 expected=$2
    shift 2
    "$@" > "$test_directory/$label.log" 2>&1
    grep -Fq "$expected" "$test_directory/$label.log" || { cat "$test_directory/$label.log" >&2; fail "$label"; }
}
seal_report() {
    local digest
    read -r digest _ < <(sha256sum "$2")
    printf 'closed-image-sha256 %s\n' "$digest" >> "$1"
}
reject_pair() {
    local label=$1 image=$2 report=$3
    if "$runner" "$image" "$report" "$contract" 1 "$store" "$key" main > "$test_directory/$label.log" 2>&1; then
        fail "$label accepted a mismatched image/report"
    fi
    grep -Fq 'exact image digest' "$test_directory/$label.log" || fail "$label did not check the image digest"
}
cat > "$test_directory/source/pass.lkj" <<'SOURCE'
pub fn main() { assert_eq!(17usize + 25usize, 42usize); }
pub fn second() { assert_eq!(6usize * 7usize, 42usize); }
pub fn host(mut world: World) { world.stdout().write(b"host observation"); }
pub fn output(mut out: Stdout) { out.write(b"granted output"); }
SOURCE
cat > "$test_directory/source/fail.lkj" <<'SOURCE'
pub fn main() { assert!(false); }
SOURCE
cat > "$test_directory/source/world.lkj" <<SOURCE
pub fn main(mut world: World) {
    let mut directory = world.directory("$test_directory").unwrap();
    let mut marker = directory.create("host-marker").unwrap();
    marker.write_all(b"host observation").unwrap();
}
SOURCE
"$compiler" ir pass.lkj --source-dir "$test_directory/source" --no-store -o "$test_directory/pass.lkjir"
"$compiler" authority "$test_directory/pass.lkjir" --no-store > "$test_directory/pass.authority"
"$compiler" build "$test_directory/pass.lkjir" --no-store -o "$test_directory/pass"
seal_report "$test_directory/pass.authority" "$test_directory/pass"
invoke=("$runner" "$test_directory/pass" "$test_directory/pass.authority" "$contract")
expect cold 'reused=0 rerun=1' "${invoke[@]}" 1 "$store" "$key" main
expect warm 'reused=1 rerun=0' "${invoke[@]}" 1 "$store" "$key" main

# Fresh checking proves that an unreachable body leaves this closed case intact.
cp "$test_directory/source/pass.lkj" "$test_directory/source/pass.before"
sed -i 's/6usize \* 7usize/21usize * 2usize/' "$test_directory/source/pass.lkj"
"$compiler" ir pass.lkj --source-dir "$test_directory/source" --no-store -o "$test_directory/unrelated.lkjir"
mkdir "$test_directory/unrelated"
"$compiler" build-entries "$test_directory/unrelated.lkjir" --no-store -o "$test_directory/unrelated" --artifact pass=main
cmp "$test_directory/pass" "$test_directory/unrelated/pass"
cmp "$test_directory/pass.authority" "$test_directory/unrelated/pass.authority"
grep -Eq '^checked-closure-sha256 [0-9a-f]{64}$' "$test_directory/unrelated/pass.authority" || fail 'missing checked closure'
expect unrelated 'reused=1 rerun=0' "$runner" "$test_directory/unrelated/pass" "$test_directory/unrelated/pass.authority" "$contract" 1 "$store" "$key" main
mv "$test_directory/source/pass.before" "$test_directory/source/pass.lkj"
# Require exactly one checked identity as well as exact image and contract.
for attack in missing duplicate malformed; do
    case "$attack" in
        missing) sed '/^checked-closure-sha256 /d' "$test_directory/pass.authority" > "$test_directory/closure.authority" ;;
        duplicate) sed '/^checked-closure-sha256 /p' "$test_directory/pass.authority" > "$test_directory/closure.authority" ;;
        malformed) sed 's/^checked-closure-sha256 .*/checked-closure-sha256 bad/' "$test_directory/pass.authority" > "$test_directory/closure.authority" ;;
    esac
    if "$runner" "$test_directory/pass" "$test_directory/closure.authority" "$contract" 1 "$store" "$key" main > "$test_directory/closure-$attack.log" 2>&1; then
        fail "accepted $attack checked closure identity"
    fi
    grep -Fq 'checked closure identity' "$test_directory/closure-$attack.log" || fail 'checked identity diagnostic'
done
expect count 'reused=0 rerun=2' "${invoke[@]}" 2 "$store" "$key" main

# The trusted caller supplies the exact entry, never a report-controlled header.
if "${invoke[@]}" 1 "$store" "$key" wrong_entry > "$test_directory/entry.log" 2>&1; then
    fail 'authority report accepted for a different entry'
fi
grep -Fq 'no grants or reachable effects' "$test_directory/entry.log" || fail 'entry diagnostic'
if "${invoke[@]}" 1 "$store" "$key" $'main\nspoofed' > "$test_directory/entry-control.log" 2>&1; then
    fail 'entry accepted report control characters'
fi
grep -Fq 'invalid entry' "$test_directory/entry-control.log" || fail 'entry control diagnostic'

# Invalid envelopes and key rotation cannot claim success.
mapfile -t entries < <(find "$store" -maxdepth 1 -type f -name 'gate-closed-*')
(( ${#entries[@]} == 2 )) || fail 'expected two closed proof identities'
for entry in "${entries[@]}"; do printf 'forged successful result\n' > "$entry"; done
expect forged 'reused=0 rerun=1' "${invoke[@]}" 1 "$store" "$key" main
(umask 077; head -c 32 /dev/urandom > "$test_directory/key2")
expect key 'reused=0 rerun=1' "${invoke[@]}" 1 "$store" "$test_directory/key2" main

# Full artifact and runner bytes bind the proof, including ELF-ignored suffixes.
cp "$test_directory/pass" "$test_directory/changed"
printf 'changed closed image\n' >> "$test_directory/changed"
reject_pair stale "$test_directory/changed" "$test_directory/pass.authority"
sed '$d' "$test_directory/pass.authority" > "$test_directory/changed.authority"
seal_report "$test_directory/changed.authority" "$test_directory/changed"
expect image 'reused=0 rerun=1' "$runner" "$test_directory/changed" "$test_directory/changed.authority" "$contract" 1 "$store" "$key" main
cp "$runner" "$test_directory/runner-generation"
printf 'changed execution producer\n' >> "$test_directory/runner-generation"
expect producer 'reused=0 rerun=1' "$test_directory/runner-generation" "$test_directory/pass" "$test_directory/pass.authority" "$contract" 1 "$store" "$key" main
cp "$contract" "$test_directory/contract"
printf '# changed contract\n' >> "$test_directory/contract"
expect script 'reused=0 rerun=1' "$runner" "$test_directory/pass" "$test_directory/pass.authority" "$test_directory/contract" 1 "$store" "$key" main
sed '$d' "$test_directory/pass.authority" > "$test_directory/authority"
reject_pair missing-footer "$test_directory/pass" "$test_directory/authority"
printf 'changed report identity\n' >> "$test_directory/authority"
seal_report "$test_directory/authority" "$test_directory/pass"
expect report 'reused=0 rerun=1' "$runner" "$test_directory/pass" "$test_directory/authority" "$contract" 1 "$store" "$key" main
printf 'trailing bytes\n' >> "$test_directory/authority"
reject_pair final-footer "$test_directory/pass" "$test_directory/authority"

# A different compiler producer may produce the same closed artifact. Only its
# already checked exact output is reused; no compiler-check proof crosses over.
cp "$compiler" "$test_directory/compiler-generation"
printf 'another compiler producer\n' >> "$test_directory/compiler-generation"
"$test_directory/compiler-generation" build "$test_directory/pass.lkjir" --no-store -o "$test_directory/pass-again"
cmp "$test_directory/pass" "$test_directory/pass-again"
"$test_directory/compiler-generation" authority "$test_directory/pass.lkjir" --no-store > "$test_directory/pass-again.authority"
seal_report "$test_directory/pass-again.authority" "$test_directory/pass-again"
cmp "$test_directory/pass.authority" "$test_directory/pass-again.authority"
expect renewed 'reused=0 rerun=1' "${invoke[@]}" 1 "$store" "$key" main
expect generation 'reused=1 rerun=0' "$runner" "$test_directory/pass-again" "$test_directory/pass-again.authority" "$contract" 1 "$store" "$key" main

# World/effect reports reject before a lookup. Failure is never published.
"$compiler" ir world.lkj --source-dir "$test_directory/source" --no-store -o "$test_directory/world.lkjir"
"$compiler" authority "$test_directory/world.lkjir" --no-store > "$test_directory/world.authority"
"$compiler" build "$test_directory/world.lkjir" --no-store -o "$test_directory/world"
seal_report "$test_directory/world.authority" "$test_directory/world"
reject_pair swapped-host "$test_directory/world" "$test_directory/pass.authority"
[[ ! -e "$test_directory/host-marker" ]] || fail 'swapped host image executed before rejection'
if "$runner" "$test_directory/pass" "$test_directory/world.authority" "$contract" 1 "$store" "$key" main > "$test_directory/open.log" 2>&1; then
    fail 'open authority was eligible for reuse'
fi
grep -Fq 'no grants or reachable effects' "$test_directory/open.log" || fail 'open authority diagnostic'
"$compiler" ir fail.lkj --source-dir "$test_directory/source" --no-store -o "$test_directory/fail.lkjir"
"$compiler" authority "$test_directory/fail.lkjir" --no-store > "$test_directory/fail.authority"
"$compiler" build "$test_directory/fail.lkjir" --no-store -o "$test_directory/fail"
seal_report "$test_directory/fail.authority" "$test_directory/fail"
before=$(find "$store" -maxdepth 1 -type f -name 'gate-closed-*' | wc -l)
for attempt in 1 2; do
    if "$runner" "$test_directory/fail" "$test_directory/fail.authority" "$contract" 1 "$store" "$key" main > "$test_directory/failure-$attempt.log" 2>&1; then
        fail 'failing closed image reused success'
    fi
    grep -Fq 'closed execution: status=' "$test_directory/failure-$attempt.log" || fail 'missing closed failure'
done
after=$(find "$store" -maxdepth 1 -type f -name 'gate-closed-*' | wc -l)
[[ "$before" == "$after" ]] || fail 'failed execution published a proof'
expect no-key 'reused=0 rerun=1' "${invoke[@]}" 1 "$store" "$test_directory/missing-key" main
# One admission emits several entries, with exact ordinary-build parity.
mkdir "$test_directory/multi" "$test_directory/multi-warm" "$test_directory/rejected"
"$compiler" build-entries "$test_directory/pass.lkjir" -o "$test_directory/multi" \
    --artifact first=main --artifact second=second --artifact host=host --store "$store" --store-key "$key"
cmp "$test_directory/pass" "$test_directory/multi/first"
cmp "$test_directory/pass.authority" "$test_directory/multi/first.authority"
for selected in second host; do
    "$compiler" build "$test_directory/pass.lkjir" --entry "$selected" --no-store -o "$test_directory/$selected"
    "$compiler" authority "$test_directory/pass.lkjir" --entry "$selected" --no-store > "$test_directory/$selected.authority"
    seal_report "$test_directory/$selected.authority" "$test_directory/$selected"
    cmp "$test_directory/$selected" "$test_directory/multi/$selected"
    cmp "$test_directory/$selected.authority" "$test_directory/multi/$selected.authority"
done
"$test_directory/multi/first"
"$test_directory/multi/second"
"$compiler" build-entries "$test_directory/pass.lkjir" -o "$test_directory/multi-warm" \
    --artifact first=main --artifact second=second --artifact host=host --store "$store" --store-key "$key" --stats \
    > "$test_directory/multi-warm.log" 2>&1
grep -q '"build_reused":0' "$test_directory/multi-warm.log" || fail 'multi-entry retained whole-image cache'
grep -Eq '"emitted_reused":[1-9][0-9]*' "$test_directory/multi-warm.log" || fail 'multi-entry fragment reuse'
for entry in "$store"/image-* "$store"/inputs-image-*; do
    [[ ! -e "$entry" ]] || fail 'multi-entry published redundant whole-image cache'
done
for artifact in first second host; do
    cmp "$test_directory/multi/$artifact" "$test_directory/multi-warm/$artifact"
    cmp "$test_directory/multi/$artifact.authority" "$test_directory/multi-warm/$artifact.authority"
done
if "$runner" "$test_directory/multi/host" "$test_directory/multi/host.authority" "$contract" 1 "$store" "$key" host \
    > "$test_directory/multi-host.log" 2>&1; then
    fail 'multi-entry World report was eligible for closed reuse'
fi
grep -Fq 'no grants or reachable effects' "$test_directory/multi-host.log" || fail 'multi-entry World authority diagnostic'
for bad_entry in absent output; do
    if "$compiler" build-entries "$test_directory/pass.lkjir" -o "$test_directory/rejected" \
        --artifact first=main --artifact bad="$bad_entry" --no-store > "$test_directory/multi-reject.log" 2>&1; then
        fail 'multi-entry rejected entry was accepted'
    fi
    grep -Fq 'E_ENTRY' "$test_directory/multi-reject.log" || fail 'multi-entry preflight diagnostic'
    [[ ! -e "$test_directory/rejected/first" && ! -e "$test_directory/rejected/first.authority" ]] \
        || fail 'multi-entry published before all entries were admitted'
done
bash tools/gate-reuse-lock-test.sh "$compiler" "$runner"
echo 'ok closed gate proofs: exact sealed bytes, producer, contract, authority/image/entry matching, key, corruption, cross-compiler reuse, failure, host exclusions and multi-entry parity/preflight'
