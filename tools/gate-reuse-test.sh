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
    if "$runner" "$image" "$report" "$contract" 1 "$store" "$key" > "$test_directory/$label.log" 2>&1; then
        fail "$label accepted a mismatched image/report"
    fi
    grep -Fq 'exact image digest' "$test_directory/$label.log" || fail "$label did not check the image digest"
}
cat > "$test_directory/source/pass.rs" <<'SOURCE'
pub fn main() { assert_eq!(17usize + 25usize, 42usize); }
SOURCE
cat > "$test_directory/source/fail.rs" <<'SOURCE'
pub fn main() { assert!(false); }
SOURCE
cat > "$test_directory/source/world.rs" <<SOURCE
pub fn main(mut world: World) {
    let mut directory = world.directory("$test_directory").unwrap();
    let mut marker = directory.create("host-marker").unwrap();
    marker.write_all(b"host observation").unwrap();
}
SOURCE
"$compiler" ir pass.rs --source-dir "$test_directory/source" --no-store -o "$test_directory/pass.lkjir"
"$compiler" authority "$test_directory/pass.lkjir" --no-store > "$test_directory/pass.authority"
"$compiler" build "$test_directory/pass.lkjir" --no-store -o "$test_directory/pass"
seal_report "$test_directory/pass.authority" "$test_directory/pass"
invoke=("$runner" "$test_directory/pass" "$test_directory/pass.authority" "$contract")
expect cold 'reused=0 rerun=1' "${invoke[@]}" 1 "$store" "$key"
expect warm 'reused=1 rerun=0' "${invoke[@]}" 1 "$store" "$key"
expect count 'reused=0 rerun=2' "${invoke[@]}" 2 "$store" "$key"

# Invalid envelopes and key rotation cannot claim success.
mapfile -t entries < <(find "$store" -maxdepth 1 -type f -name 'gate-closed-*')
(( ${#entries[@]} == 2 )) || fail 'expected two closed proof identities'
for entry in "${entries[@]}"; do printf 'forged successful result\n' > "$entry"; done
expect forged 'reused=0 rerun=1' "${invoke[@]}" 1 "$store" "$key"
(umask 077; head -c 32 /dev/urandom > "$test_directory/key2")
expect key 'reused=0 rerun=1' "${invoke[@]}" 1 "$store" "$test_directory/key2"

# Full artifact and runner bytes bind the proof, including ELF-ignored suffixes.
cp "$test_directory/pass" "$test_directory/changed"
printf 'changed closed image\n' >> "$test_directory/changed"
reject_pair stale "$test_directory/changed" "$test_directory/pass.authority"
sed '$d' "$test_directory/pass.authority" > "$test_directory/changed.authority"
seal_report "$test_directory/changed.authority" "$test_directory/changed"
expect image 'reused=0 rerun=1' "$runner" "$test_directory/changed" "$test_directory/changed.authority" "$contract" 1 "$store" "$key"
cp "$runner" "$test_directory/runner-generation"
printf 'changed execution producer\n' >> "$test_directory/runner-generation"
expect producer 'reused=0 rerun=1' "$test_directory/runner-generation" "$test_directory/pass" "$test_directory/pass.authority" "$contract" 1 "$store" "$key"
cp "$contract" "$test_directory/contract"
printf '# changed contract\n' >> "$test_directory/contract"
expect script 'reused=0 rerun=1' "$runner" "$test_directory/pass" "$test_directory/pass.authority" "$test_directory/contract" 1 "$store" "$key"
sed '$d' "$test_directory/pass.authority" > "$test_directory/authority"
reject_pair missing-footer "$test_directory/pass" "$test_directory/authority"
printf 'changed report identity\n' >> "$test_directory/authority"
seal_report "$test_directory/authority" "$test_directory/pass"
expect report 'reused=0 rerun=1' "$runner" "$test_directory/pass" "$test_directory/authority" "$contract" 1 "$store" "$key"
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
expect renewed 'reused=0 rerun=1' "${invoke[@]}" 1 "$store" "$key"
expect generation 'reused=1 rerun=0' "$runner" "$test_directory/pass-again" "$test_directory/pass-again.authority" "$contract" 1 "$store" "$key"

# World/effect reports reject before a lookup. Failure is never published.
"$compiler" ir world.rs --source-dir "$test_directory/source" --no-store -o "$test_directory/world.lkjir"
"$compiler" authority "$test_directory/world.lkjir" --no-store > "$test_directory/world.authority"
"$compiler" build "$test_directory/world.lkjir" --no-store -o "$test_directory/world"
seal_report "$test_directory/world.authority" "$test_directory/world"
reject_pair swapped-host "$test_directory/world" "$test_directory/pass.authority"
[[ ! -e "$test_directory/host-marker" ]] || fail 'swapped host image executed before rejection'
if "$runner" "$test_directory/pass" "$test_directory/world.authority" "$contract" 1 "$store" "$key" > "$test_directory/open.log" 2>&1; then
    fail 'open authority was eligible for reuse'
fi
grep -Fq 'no grants or reachable effects' "$test_directory/open.log" || fail 'open authority diagnostic'
"$compiler" ir fail.rs --source-dir "$test_directory/source" --no-store -o "$test_directory/fail.lkjir"
"$compiler" authority "$test_directory/fail.lkjir" --no-store > "$test_directory/fail.authority"
"$compiler" build "$test_directory/fail.lkjir" --no-store -o "$test_directory/fail"
seal_report "$test_directory/fail.authority" "$test_directory/fail"
before=$(find "$store" -maxdepth 1 -type f -name 'gate-closed-*' | wc -l)
for attempt in 1 2; do
    if "$runner" "$test_directory/fail" "$test_directory/fail.authority" "$contract" 1 "$store" "$key" > "$test_directory/failure-$attempt.log" 2>&1; then
        fail 'failing closed image reused success'
    fi
    grep -Fq 'closed execution: status=' "$test_directory/failure-$attempt.log" || fail 'missing closed failure'
done
after=$(find "$store" -maxdepth 1 -type f -name 'gate-closed-*' | wc -l)
[[ "$before" == "$after" ]] || fail 'failed execution published a proof'
expect no-key 'reused=0 rerun=1' "${invoke[@]}" 1 "$store" "$test_directory/missing-key"
echo 'ok closed gate proofs: exact sealed bytes, producer, contract, authority/image matching, key, corruption, cross-compiler reuse, failure and host exclusions'
