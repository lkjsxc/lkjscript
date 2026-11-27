#!/usr/bin/env bash
# Fast wrapper regression controls; no compiler or native workers needed.
set -euo pipefail
cd "$(dirname "$0")/.."
profile() { bash tools/gate-worker-profile.sh "$@"; }

# A command with no batch descendants produces an empty CPU observer file.
[[ -z $(profile true) ]]
output=$(profile sh -c '
    printf "%s\n" "CPU foo 0" "CPU 0 10 20 3 100" "LKJSCRIPT_STATS {\"test_rerun\":2,\"check_ms\":1.5}"
    printf "%s\n" "CPU 0 0 0 0 0" >&2
    printf "%s\n" "LKJSCRIPT_STATS {\"test_rerun\":2}"
')
expected=$(cat <<'EXPECTED'
CPU foo 0
CPU 0 10 20 3 100
LKJSCRIPT_STATS {"test_rerun":2,"check_ms":1.5}
CPU 0 0 0 0 0
LKJSCRIPT_STATS {"test_rerun":2}
LKJSCRIPT_GATE_WORKER worker=0 cpu=unavailable test_rerun=2 check_ms=1.5
EXPECTED
)
[[ $output == "$expected" ]] || { printf 'FAIL unexpected profile output:\n%s\n' "$output" >&2; exit 1; }
status=0
output=$(profile sh -c 'printf "%s\n" "CPU foo 0"; exit 7') || status=$?
[[ $status == 7 && $output == 'CPU foo 0' ]]
printf 'ok gate worker profile: empty observer, arbitrary output, exit status\n'
