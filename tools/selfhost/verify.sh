#!/usr/bin/env bash
# Focused two-level backend verification; not a Tier 2 semantic self-hosting claim.
set -euo pipefail
selfhost_test_threads=${RUST_TEST_THREADS-2}
if [[ ! $selfhost_test_threads =~ ^[0-9]+$ || ! $selfhost_test_threads =~ [1-9] ]]; then
  printf '%s\n' 'RUST_TEST_THREADS must be a positive decimal integer' >&2
  exit 2
fi
printf 'selfhost test threads=%s\n' "$selfhost_test_threads" >&2
[[ $(git rev-parse --show-toplevel) == "$PWD" ]]
export CARGO_TARGET_DIR="$PWD/target"
mkdir -p "$CARGO_TARGET_DIR"
exec {selfhost_lease}> "$CARGO_TARGET_DIR/.lkjscript-check.lock"
flock --exclusive "$selfhost_lease"
mkdir -p .artifacts/selfhost
sha256sum src/platform/compiler/selfhost_input.rs \
  src/platform/compiler/selfhost_artifact_input.rs \
  src/platform/compiler/selfhost_source_input.rs \
  src/platform/compiler/selfhost_source_input/export.rs \
  src/platform/compiler/selfhost_source_input/tests.rs \
  src/platform/execution/normalized/selfhost_source_fixture.rs \
  src/platform/execution/normalized/selfhost_source_unit_tests.rs \
  src/platform/execution/normalized/selfhost_source_interface_tests.rs \
  tests/fixtures/selfhost-source-public-export.lkjp \
  tests/fixtures/selfhost-source-profile3.lkjsg \
  tests/fixtures/selfhost-source-units.lkjc \
  tests/fixtures/selfhost-source-interface.lkjc \
  tests/fixtures/selfhost-source-interface-probes.lkjc \
  src/platform/execution/normalized/selfhost_source_artifact_tests.rs \
  src/platform/execution/normalized/selfhost_source_stage_controls.rs \
  src/platform/execution/normalized/selfhost_source_bulk_tests.rs \
  tests/fixtures/bulk-byte-intrinsics.lkjc \
  src/platform/execution/normalized/selfhost_source_artifact_support.rs \
  tests/fixtures/selfhost-source-artifact.lkjc \
  tests/support/selfhost_proposals.rs \
  src/platform/execution/normalized/selfhost_tests.rs \
  src/platform/execution/normalized/selfhost_bytes_tests.rs \
  src/platform/execution/normalized/selfhost_admission_tests.rs \
  src/platform/execution/normalized/selfhost_artifact_tests.rs \
  src/platform/execution/normalized/selfhost_artifact_measure_tests.rs \
  src/platform/execution/normalized/selfhost_artifact_order_tests.rs \
  src/platform/execution/normalized/selfhost_closure_tests.rs \
  src/platform/execution/normalized/selfhost_source_tests.rs \
  src/platform/execution/normalized/selfhost_source_scalar_tests.rs \
  src/platform/execution/normalized/selfhost_source_compound_tests.rs \
  src/platform/execution/normalized/selfhost_source_custody_tests.rs \
  src/platform/execution/normalized/selfhost_source_custody_evidence_tests.rs \
  tests/fixtures/selfhost-source-scalar.lkjc \
  tests/fixtures/selfhost-source-scalar-intrinsics.lkjc \
  tests/fixtures/selfhost-source-compound.lkjc \
  tests/fixtures/selfhost-source-custody.lkjc \
  tests/fixtures/selfhost-source-custody-mixed.lkjc \
  tests/fixtures/selfhost-source-custody-probes.lkjc \
  src/platform/execution/normalized/selfhost_source_profile_tests.rs \
  src/platform/execution/normalized/selfhost_source_profile_flow_tests.rs \
  src/platform/execution/normalized/selfhost_source_measure_tests.rs \
  src/platform/execution/normalized/selfhost_source_index_tests.rs \
  src/platform/execution/normalized/selfhost_source_parent_tests.rs \
  tests/fixtures/selfhost-source-index-probes.lkjc \
  src/platform/execution/normalized/selfhost_source_binding_tests.rs \
  src/platform/execution/normalized/selfhost_source_graph_tests.rs \
  src/platform/execution/normalized/selfhost_source_scope_tests.rs \
  src/platform/execution/normalized/selfhost_source_namespace_tests.rs \
  src/platform/execution/normalized/selfhost_source_effect_tests.rs \
  src/platform/execution/normalized/selfhost_source_effect_owner_tests.rs \
  src/platform/execution/normalized/selfhost_source_effect_scope_tests.rs \
  src/platform/execution/normalized/selfhost_source_effect_shape_tests.rs \
  tests/fixtures/selfhost-source-effect-owners.lkjc \
  src/platform/execution/normalized/selfhost_source_type_scope_tests.rs \
  src/platform/execution/normalized/selfhost_source_nominal_flow_tests.rs \
  tests/fixtures/selfhost-source-namespace.lkjc \
  tests/fixtures/selfhost-source-namespace-probes.lkjc \
  tests/fixtures/selfhost-source-type-scope.lkjc \
  tests/fixtures/selfhost-source-type-scope-probes.lkjc \
  tests/fixtures/selfhost-source-nominal-flow.lkjc \
  tests/fixtures/selfhost-source-nominal-probes.lkjc \
  src/platform/execution/normalized/selfhost_source_function_binder_oracle_tests.rs \
  tests/fixtures/selfhost-source-function-binders.lkjc \
  tests/fixtures/selfhost-source-scope-probes.lkjc \
  tests/fixtures/selfhost-source-lexical.lkjc \
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
cargo test --release --locked --lib selfhost_ -- --test-threads="$selfhost_test_threads" --nocapture \
  2>&1 | tee .artifacts/selfhost/latest.log
cargo test --release --locked --lib source_input_ -- --test-threads="$selfhost_test_threads" \
  2>&1 | tee .artifacts/selfhost/source-input.log
cargo test --release --locked --lib native_elf_ -- --test-threads="$selfhost_test_threads" \
  2>&1 | tee .artifacts/selfhost/native.log
sha256sum --check .artifacts/selfhost/inputs.sha256
