#!/usr/bin/env bash
# Focused two-level backend verification; not a Tier 2 semantic self-hosting claim.
set -euo pipefail
[[ $(git rev-parse --show-toplevel) == "$PWD" ]]
export CARGO_TARGET_DIR="$PWD/target"
mkdir -p "$CARGO_TARGET_DIR"
exec {selfhost_lease}> "$CARGO_TARGET_DIR/.lkjscript-check.lock"
flock --exclusive "$selfhost_lease"
mkdir -p .artifacts/selfhost
sha256sum src/platform/compiler/selfhost_input.rs \
  src/platform/compiler/selfhost_artifact_input.rs \
  tests/support/selfhost_proposals.rs \
  src/platform/execution/normalized/selfhost_tests.rs \
  src/platform/execution/normalized/selfhost_bytes_tests.rs \
  src/platform/execution/normalized/selfhost_admission_tests.rs \
  src/platform/execution/normalized/selfhost_artifact_tests.rs \
  src/platform/execution/normalized/selfhost_closure_tests.rs \
  src/platform/execution/normalized/selfhost_source_tests.rs \
  src/platform/execution/normalized/selfhost_source_binding_tests.rs \
  src/platform/execution/normalized/selfhost_source_graph_tests.rs \
  tests/fixtures/selfhost-closure.lkjc \
  tests/fixtures/selfhost-artifact-probes.lkjc \
  tests/fixtures/selfhost-source-probes.lkjc \
  tests/fixtures/selfhost-source-record-probes.lkjc \
  tests/fixtures/selfhost-source-map-probes.lkjc \
  tests/fixtures/selfhost-source-state-probes.lkjc \
  tests/fixtures/selfhost-source-history-probes.lkjc \
  tests/fixtures/selfhost-source-object-probes.lkjc \
  tests/fixtures/selfhost-source-graph-probes.lkjc \
  tests/fixtures/selfhost-source-nominal.lkjc \
  tests/fixtures/selfhost-artifact-empty-runtime.lkjc \
  tests/fixtures/selfhost-artifact-object-scan.lkjc \
  tests/fixtures/selfhost-generic-list.lkjc \
  tests/fixtures/selfhost-aggregates.lkjc \
  tests/fixtures/selfhost-buffer-borrow-tail.lkjc \
  tests/fixtures/selfhost-forgotten-buffer.json \
  tests/fixtures/selfhost-unregistered-intrinsic.lkjc \
  tests/fixtures/selfhost-untaken-unsupported.lkjc > .artifacts/selfhost/inputs.sha256
rg --files tools/selfhost src/platform/compiler/elf_x86_64 \
  src/platform/compiler/elf_x86_64.rs \
  src/platform/execution/normalized/native_elf_tests.rs \
  src/platform/execution/normalized/native_filter_tests.rs \
  src/platform/execution/normalized/native_derive_tests.rs \
  tests/fixtures/native-bytes.lkjc tests/fixtures/native-filter.lkjc \
  tests/fixtures/blake3-unkeyed-vectors.json \
  tests/fixtures/blake3-context-escaping.json \
  tests/fixtures/blake3-derive-vectors.json | LC_ALL=C sort > .artifacts/selfhost/paths
while IFS= read -r selfhost_input; do
  sha256sum "$selfhost_input" >> .artifacts/selfhost/inputs.sha256
done < .artifacts/selfhost/paths
cargo test --release --locked --lib selfhost_ -- --test-threads=2 --nocapture \
  2>&1 | tee .artifacts/selfhost/latest.log
cargo test --release --locked --lib native_elf_ -- --test-threads=2 \
  2>&1 | tee .artifacts/selfhost/native.log
sha256sum --check .artifacts/selfhost/inputs.sha256
