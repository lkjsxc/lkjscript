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

## Revision-17 Prepared Sealed Vertical

The revision-17 vertical passed strict local gates, 234 compiler tests, complete
workspace tests, release build and smokes, package checks, all-tier sealed
execution, daemon/process rehydration, the no-per-node-RC comparison, Miri,
ASan/LSan/TSan, and the Docker verified target. Nightly emitted only the
pre-existing atomic-method deprecation warnings.

```text
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
cargo run --locked -p lkjscript-xtask -- structure check
cargo run --locked -p lkjscript-xtask -- quiet verify
cargo build --workspace --release --locked
cargo run --locked -p lkjscript-app --bin lkjscript -- package check
cargo test --locked -p lkjscript-app --test jit_engines \
  generic_products::residual::sealed_residual_generic_executes_in_all_four_tiers
cargo test --locked -p lkjscript-app --test cli_contract process_cells::structural::
cargo test --release --locked -p lkjscript-core \
  --test sealed_strategy_comparison -- --nocapture
CARGO_TARGET_DIR=target/miri-prepared-sealed cargo +nightly miri test --locked \
  -p lkjscript-core --test value_runtime tests::sealed -- --nocapture
CARGO_TARGET_DIR=target/miri-prepared-sealed-dag cargo +nightly miri test --locked \
  -p lkjscript-core --lib outcome::semantic_dag::sealed::tests:: -- --nocapture
CARGO_TARGET_DIR=target/miri-prepared-sealed-jit cargo +nightly miri test --locked \
  -p lkjscript-jit \
  island::structural::tests::cleanup::sealed::registry_failures_dispose_new_runtime_owners \
  -- --exact --nocapture
RUSTFLAGS='-Zsanitizer=SANITIZER' cargo +nightly test -Zbuild-std \
  --target x86_64-unknown-linux-gnu --locked -p lkjscript-app \
  --test jit_engines \
  generic_products::residual::sealed_residual_generic_executes_in_all_four_tiers \
  -- --exact --nocapture
RUSTFLAGS='-Zsanitizer=address' cargo +nightly test -Zbuild-std \
  --target x86_64-unknown-linux-gnu --locked -p lkjscript-app \
  --test cli_contract process_cells::structural:: -- --nocapture
docker buildx build --load --build-context repository-git=<checkout-git-dir> \
  -f meta/Dockerfile --target verified -t lkjscript-prepared-sealed-verify .
docker run --rm --entrypoint cat lkjscript-prepared-sealed-verify \
  /tmp/lkjscript-verification-result
```

`SANITIZER` was run separately as `address`, `leak`, and `thread`; each exact
sealed residual test passed 1/1, and the ASan process suite passed 2/2. The
linked-worktree Compose launcher cannot mount a `.git` indirection file as an
additional directory context, so the equivalent explicit `buildx` command used
the checkout Git directory and returned `result=ok`. The first Docker attempt
also exposed and led to the focused auto-entry rejection for hidden-witness or
more-than-two-argument ABI signatures; default Mandelbrot then passed again. A
later build attempt hit `EMFILE` in concurrent compiler tests; the immediate
unchanged retry completed with `result=ok`.

## Revision-18 Residual Compare Commands

The bounded compare vertical passed exact all-tier execution, malformed SSA
rejection, strict Clippy, all 63 workspace all-target test binaries, canonical
verification, release build and smokes, package and structure checks, retained
result validation, Miri evaluator/VM execution, address/leak/thread sanitizer
all-tier execution, WASI cross-build and Node execution, and Docker verification.
The first Docker run hit the known daemon-control `EAGAIN`; its unchanged retry
passed 234 compiler tests, 169 core tests, all other workspace tests, and wrote
`result=ok`.

```text
cargo test --locked -p lkjscript-app --test jit_engines \
  generic_products::compare::residual_compare_executes_in_all_four_tiers -- --exact
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-targets -- --test-threads=1
cargo run --locked -p lkjscript-xtask -- quiet verify
cargo run --locked -p lkjscript-xtask -- structure check
cargo build --locked --workspace --release
cargo run --locked -p lkjscript-app --bin lkjscript -- package check
# The exact compare test was repeated under Miri and address, leak, and thread sanitizers.
cargo build --locked -p lkjscript-host -p lkjscript-database \
  --target wasm32-wasip1 --example wasi-kernel
node <WASI preview1 runner for target/.../examples/wasi-kernel.wasm>
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

The repository has no fuzz harness, undefined-behavior sanitizer, or non-Linux
native executor. The retained bounded Brainfuck campaign was not rerun; its
inherited `structural limit exceeded: Domains` result remains negative evidence.

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
