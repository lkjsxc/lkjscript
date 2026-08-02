# Polymorphic Value Plane Verification

## Status

Retained command evidence for the Experimental polymorphic value-plane slices.
This file does not promote the complete plane to Current.

## Revision-15 Authenticated Witness Commands

The revision-15 implementation passed canonical verification, structure check
and audit, release build, package and source checks, Docker verification,
focused authenticated-return Miri, focused ASan/LSan/TSan, and release
sealed-scaling evidence. Nightly Miri emitted three atomic-method deprecation
warnings and no errors:

```text
cargo run --locked -p lkjscript-xtask -- quiet verify
cargo run --locked -q -p lkjscript-xtask -- structure check
cargo run --locked -q -p lkjscript-xtask -- structure audit
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
cargo build --workspace --release --locked
cargo run --locked -q -p lkjscript-app --bin lkjscript -- package check
cargo run --locked -q -p lkjscript-xtask -- check-sources
CARGO_TARGET_DIR=target/lkjscript/miri-authenticated-witness cargo +nightly miri test --locked \
  -p lkjscript-core --lib validation::tests::structural::authenticated_ -- --nocapture
# Repeated with address, leak, and thread in separate target directories.
RUSTFLAGS='-Zsanitizer=SANITIZER' cargo +nightly test -Zbuild-std \
  --target x86_64-unknown-linux-gnu --locked -p lkjscript-core --lib \
  validation::tests::structural::authenticated_ -- --nocapture
cargo test --release --locked -p lkjscript-core --test sealed_scaling -- --nocapture
```

## Revision-16 Canonical Closure Commands

The revision-16 semantic closure implementation passed the focused contract,
compiler, IR, core, and authenticated application tests, strict focused Clippy,
canonical verification, structure check/audit, and package check. The full
runtime, Docker, Miri, sanitizer, cross-build, and performance campaigns were
not rerun for this intermediate trust-layer cut.

```text
cargo test --locked -p lkjscript-contracts
cargo test --locked -p lkjscript-compiler
cargo test --locked -p lkjscript-ir
cargo test --locked -p lkjscript-core
cargo test --locked -p lkjscript-app authenticated_
cargo clippy --locked -p lkjscript-contracts -p lkjscript-compiler \
  -p lkjscript-ir -p lkjscript-core --all-targets --all-features -- -D warnings
cargo run --locked -p lkjscript-xtask -- quiet verify
cargo run --locked -p lkjscript-xtask -- structure check
cargo run --locked -p lkjscript-xtask -- structure audit --json
cargo run --locked -p lkjscript-app --bin lkjscript -- package check
git diff --check
```

## Earlier Slice Commands

The following older commands exited zero against implementation commit
`9f82cc5c` or its evidence-and-gate descendant. Canonical `quiet verify`
completed in 23 seconds and its final rerun in 20 seconds; release build plus
ten runtime smokes in 79 seconds, Docker verify in 206 seconds, and the
12,000-level Miri stress in 1,321 seconds in the recorded environment:

```text
cargo test --locked --workspace --all-targets
cargo test --locked -p lkjscript-compiler
cargo test --locked -p lkjscript-core --all-targets
cargo test --locked -p lkjscript-ir --lib
cargo test --locked -p lkjscript-vm --lib
cargo test --locked -p lkjscript-app --test jit_engines segmented_lists::
cargo test --release --locked -p lkjscript-app --test jit_engines segmented_lists::structural_owners::
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked -p lkjscript-app --test jit_engines generic_
cargo test --locked -p lkjscript-app --test cli_contract application_control::
cargo test --locked -p lkjscript-app --test numeric_contract
cargo test --locked -p lkjscript-core --test sealed_scaling
cargo run --locked -q -p lkjscript-xtask -- structure check
cargo run --locked -q -p lkjscript-xtask -- structure audit
cargo run --locked -q -p lkjscript-xtask -- check-sources
cargo run --locked -p lkjscript-xtask -- quiet verify
cargo run --locked -q -p lkjscript-app --bin lkjscript -- package check
cargo build --workspace --release --locked
./target/release/lkjscript run src/examples/jit-scalar/main.lkjscript
./target/release/lkjscript run --engine vm src/examples/jit-scalar/main.lkjscript
./target/release/lkjscript run --engine baseline-jit src/examples/jit-scalar/main.lkjscript
./target/release/lkjscript run --engine optimizing-jit src/examples/jit-optimizing/main.lkjscript
./target/release/lkjscript run --engine auto --auto-jit-threshold 2 src/examples/jit-scalar/main.lkjscript
./target/release/lkjscript run --engine vm src/examples/hello/main.lkjscript
./target/release/lkjscript run --engine vm src/examples/mandel/main.lkjscript
./target/release/lkjscript run --engine vm src/examples/polymorphic-transport/history-snapshot.lkjscript
./target/release/lkjscript run --engine baseline-jit src/examples/polymorphic-transport/history-workload.lkjscript
./target/release/lkjscript run --engine optimizing-jit src/examples/polymorphic-transport/history-workload.lkjscript
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
CARGO_TARGET_DIR=target/lkjscript/miri cargo +nightly miri test --locked \
  -p lkjscript-core --test value_runtime \
  tests::deep::deep_image_conversion_clone_export_and_release_are_iterative \
  -- --exact --nocapture
CARGO_TARGET_DIR=target/lkjscript/miri cargo +nightly miri test --locked \
  -p lkjscript-core --test sealed_scaling -- --nocapture
CARGO_TARGET_DIR=target/lkjscript/miri-structural-equality cargo +nightly miri test \
  --locked -p lkjscript-jit \
  island::structural::equality::tests::semantic_equality_is_iterative_and_exact_for_deep_products \
  -- --exact --nocapture
CARGO_TARGET_DIR=target/lkjscript/asan-structural-plane RUSTFLAGS='-Zsanitizer=address' \
  cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu --locked \
  -p lkjscript-app --test jit_engines segmented_lists:: -- --nocapture
CARGO_TARGET_DIR=target/lkjscript/lsan-structural-plane RUSTFLAGS='-Zsanitizer=leak' \
  cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu --locked \
  -p lkjscript-app --test jit_engines segmented_lists:: -- --nocapture
CARGO_TARGET_DIR=target/lkjscript/tsan-structural-plane RUSTFLAGS='-Zsanitizer=thread' \
  cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu --locked \
  -p lkjscript-app --test jit_engines segmented_lists:: -- --nocapture
```
