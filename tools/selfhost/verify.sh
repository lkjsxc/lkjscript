#!/usr/bin/env bash
# Focused two-level compiler slice verification; not a self-hosting claim.
set -euo pipefail
[[ $(git rev-parse --show-toplevel) == "$PWD" ]]
export CARGO_TARGET_DIR="$PWD/target"
mkdir -p "$CARGO_TARGET_DIR"
exec {selfhost_lease}> "$CARGO_TARGET_DIR/.lkjscript-check.lock"
flock --exclusive "$selfhost_lease"
mkdir -p .artifacts/selfhost
sha256sum tools/selfhost/input.lkjc tools/selfhost/elf.lkjc \
  src/platform/compiler/selfhost_input.rs \
  src/platform/execution/normalized/selfhost_tests.rs > .artifacts/selfhost/inputs.sha256
rg --files tools/selfhost/project src/platform/compiler/elf_x86_64 \
  src/platform/compiler/elf_x86_64.rs \
  src/platform/execution/normalized/native_elf_tests.rs \
  src/platform/execution/normalized/native_filter_tests.rs \
  tests/fixtures/native-bytes.lkjc tests/fixtures/native-filter.lkjc \
  tests/fixtures/blake3-unkeyed-vectors.json | LC_ALL=C sort > .artifacts/selfhost/paths
while IFS= read -r selfhost_input; do
  sha256sum "$selfhost_input" >> .artifacts/selfhost/inputs.sha256
done < .artifacts/selfhost/paths
cargo test --release --locked --lib selfhost_ -- --test-threads=2 --nocapture \
  2>&1 | tee .artifacts/selfhost/latest.log
cargo test --release --locked --lib native_elf_ -- --test-threads=2 \
  2>&1 | tee .artifacts/selfhost/native.log
sha256sum --check .artifacts/selfhost/inputs.sha256
