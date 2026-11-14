#!/usr/bin/env bash
set -euo pipefail
prefix=$1
[[ "$prefix" =~ ^source-accept-v[0-9]+-$ ]]
unset GH_TOKEN GITHUB_TOKEN GH_ENTERPRISE_TOKEN GITHUB_ENTERPRISE_TOKEN
export PATH="/home/coder/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=8
candidate=$(git rev-parse HEAD)
base=".artifacts/selfhost/$prefix"
[[ ! -e "${base}full-finished.json" ]]
verifier_root=$(bash tools/bootstrap-verifier.sh --expected-source "$candidate" 2> "${base}bootstrap.log")
cp target/bootstrap-receipt/receipt.json "${base}bootstrap.json"
"$verifier_root/lkjscript-dev" check changed --jobs 4 --machine > "${base}changed-summary.json" 2> "${base}changed.log"
cp .artifacts/lkjscript-dev/latest-receipt.json "${base}changed.json"
node .artifacts/selfhost/watch-source-acceptance.cjs "$candidate" "$prefix" > "${base}watch.log" 2>&1 &
watch_pid=$!
if "$verifier_root/lkjscript-dev" check full --fresh --jobs 4 --machine > "${base}full-summary.json" 2> "${base}full.log"; then full_code=0; else full_code=$?; fi
jq -e --arg source "$candidate" '.git_head == $source and .profile == "full"' .artifacts/lkjscript-dev/latest-receipt.json > /dev/null
cp .artifacts/lkjscript-dev/latest-receipt.json "${base}full.json"
printf '{"exit_code":%s}\n' "$full_code" > "${base}full-finished.json"
wait "$watch_pid"
if (( full_code != 0 )); then exit "$full_code"; fi
"$verifier_root/lkjscript-dev" release target > "${base}release-target.json" 2> "${base}release-target.log"
