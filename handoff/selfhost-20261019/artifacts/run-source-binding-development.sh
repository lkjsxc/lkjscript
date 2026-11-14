#!/usr/bin/env bash
set -euo pipefail
unset GH_TOKEN GITHUB_TOKEN GH_ENTERPRISE_TOKEN GITHUB_ENTERPRISE_TOKEN
export PATH="/home/coder/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=8
exec {source_binding_lease}> target/.lkjscript-check.lock
flock --exclusive "$source_binding_lease"
[[ $(git rev-parse HEAD) == c282cfcabcdc4ad41bae1e7ff9652eb63e5d5f6f ]]
jq -e '.status == "passed" and .source == "c282cfcabcdc4ad41bae1e7ff9652eb63e5d5f6f"' .artifacts/selfhost/accepted-e-source-records/receipt.json > /dev/null
for module in map state history object; do
  node .artifacts/selfhost/upsert.cjs ".artifacts/selfhost/source-$module-draft.lkjc"
  target/release/lkjscript --project tools/selfhost/project tx --input-file .artifacts/selfhost/current.lkjx --compact > ".artifacts/selfhost/source-binding-author-$module-v1.log" 2>&1
done
node .artifacts/selfhost/integrate-source-bindings.cjs
cargo fmt --all -- --check > .artifacts/selfhost/source-binding-format-v1.log 2>&1 || cargo fmt --all
cargo test --release --locked --lib selfhost_source_ -- --test-threads=2 --nocapture > .artifacts/selfhost/source-binding-tests-v1.log 2>&1
