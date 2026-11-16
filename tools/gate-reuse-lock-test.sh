#!/usr/bin/env bash
# Lookup/disposal must not hold the store writer lock while a closed child runs.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
runner=${2:-target/gate/closed/runner}
test_directory=$(mktemp -d target/gate-reuse-lock.XXXXXX)
runner_pid=
child_pid=
cleanup() {
    if [[ -n "$child_pid" ]]; then kill -KILL "$child_pid" 2>/dev/null || true; fi
    if [[ -n "$runner_pid" ]]; then
        kill -TERM "$runner_pid" 2>/dev/null || true
        wait "$runner_pid" 2>/dev/null || true
    fi
    rm -rf "$test_directory"
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM
fail() { echo "gate reuse lock: $*" >&2; exit 1; }
mkdir "$test_directory/store" "$test_directory/images"
key="$test_directory/key"
store="$test_directory/store"
(umask 077; head -c 32 /dev/urandom > "$key")
cat > "$test_directory/closed.lkj" <<'SOURCE'
pub fn main() { loop {} }
pub fn pass() { assert_eq!(17usize + 25usize, 42usize); }
SOURCE
"$compiler" ir "$test_directory/closed.lkj" --no-store -o "$test_directory/closed.lkjir"
"$compiler" build-entries "$test_directory/closed.lkjir" --no-store -o "$test_directory/images" \
    --artifact stalled=main --artifact pass=pass
invoke() {
    local name=$1 entry=$2
    "$runner" "$test_directory/images/$name" "$test_directory/images/$name.authority" \
        tools/closed-semantics.sh 1 "$store" "$key" "$entry"
}
# An infinite, authority-free child makes both observations independent of CPU
# speed. Observe its existence, stop it, then kill and reap it after each probe.
start_stalled() {
    local label=$1
    "$runner" "$test_directory/images/stalled" "$test_directory/images/stalled.authority" \
        tools/closed-semantics.sh 1 "$store" "$key" main > "$test_directory/$label.log" 2>&1 &
    runner_pid=$!
    for ((attempt=0; attempt<500; attempt++)); do
        if [[ -r "/proc/$runner_pid/task/$runner_pid/children" ]]; then
            read -r child_pid _ < "/proc/$runner_pid/task/$runner_pid/children" || true
            if [[ "$child_pid" =~ ^[0-9]+$ ]]; then break; fi
        fi
        if ! kill -0 "$runner_pid" 2>/dev/null; then
            cat "$test_directory/$label.log" >&2
            fail "$label runner exited before its closed child started"
        fi
        sleep 0.01
    done
    [[ "$child_pid" =~ ^[0-9]+$ ]] || fail "$label closed child was not observed"
    kill -STOP "$child_pid"
}
finish_failed() {
    local label=$1
    kill -KILL "$child_pid"
    if wait "$runner_pid"; then fail "$label killed closed child was accepted"; fi
    runner_pid=
    child_pid=
    grep -Fq 'closed execution: status=' "$test_directory/$label.log" || fail "$label missing child-failure diagnostic"
    [[ $(find "$store" -maxdepth 1 -type f -name 'gate-closed-*' | wc -l) == 0 ]] \
        || fail "$label failed execution left a success proof"
}
# A genuinely absent read now performs no writer accounting at all.
start_stalled absent
[[ -z $(find "$store" -mindepth 1 -maxdepth 1 -print -quit) ]] || fail 'cold lookup created a store entry'
finish_failed absent
# A regular but unauthentic proof still exercises disposal and its writer lock.
# Reproduce the exact length-framed identity independently; a changed identity
# format must fail the disposal assertions rather than silently test a cold miss.
proof_name=$(perl -MDigest::SHA=sha256,sha256_hex - "$runner" \
    "$test_directory/images/stalled" "$test_directory/images/stalled.authority" tools/closed-semantics.sh <<'PERL'
use strict;
use warnings;
sub bytes { open my $in, '<:raw', $_[0] or die $!; local $/; return <$in> // ''; }
sub frame { return pack('Q<', length($_[0])) . $_[0]; }
my ($runner, $image, $authority, $script) = @ARGV;
my $identity = "lkjscript-closed-gate-3\0";
$identity .= frame(bytes($image)) . frame(bytes($authority)) . frame(bytes($script));
$identity .= pack('Q<', 1) . frame('main');
$identity .= frame('argv=[];stdin=[];status=0;stdout=[];stderr=[];result=[]');
print 'gate-closed-' . sha256_hex(sha256(bytes($runner))) . '-' . sha256_hex($identity);
PERL
)
printf 'unauthentic closed proof\n' > "$store/$proof_name"
start_stalled rejected
[[ ! -e "$store/$proof_name" && ! -L "$store/$proof_name" ]] || fail 'rejected lookup did not dispose of the exact proof'
[[ -f "$store/.budget-lock" ]] || fail 'rejected lookup did not reach store accounting'
flock -n "$store/.budget-lock" true || fail 'rejected proof held the writer lock during closed execution'
finish_failed rejected
# Genuine simultaneous short writers still cause harmless misses; publication
# does not wait, and the next serial execution can publish an authenticated hit.
exec {proof_lock}> "$store/.budget-lock"
flock -xn "$proof_lock"
timeout 5 "$runner" "$test_directory/images/pass" "$test_directory/images/pass.authority" \
    tools/closed-semantics.sh 1 "$store" "$key" pass > "$test_directory/busy.log" 2>&1
grep -Fq 'reused=0 rerun=1' "$test_directory/busy.log" || fail 'busy writer did not remain a miss'
exec {proof_lock}>&-
invoke pass pass > "$test_directory/cold.log" 2>&1
grep -Fq 'reused=0 rerun=1' "$test_directory/cold.log" || fail 'unpublished observation claimed reuse'
invoke pass pass > "$test_directory/warm.log" 2>&1
grep -Fq 'reused=1 rerun=0' "$test_directory/warm.log" || fail 'successful serial observation was not retained'
echo 'ok closed gate lock: absent read writes nothing, rejected proof releases before execution, failure publishes nothing, busy writer never waits, serial success reuses'
