# Structural Witness And Nested-List Evidence

## Status

**Current only for concrete structural witness identity propagation and
recursively nested copy lists.** This evidence does not claim residual generic
witness ABI, structural-owner list elements, sealed language storage, affine
resource aggregates, capturing closures, or stateful application sessions.

## Starting Truth

The cut started from clean `main` at
`ef48675fdf1bb3f52114cd0348563ef566001574`, equal to `origin/main`, with
platform revision 11 and the unconditional no-tracing gate Current. The
integrated public cut uses platform revision 12.

The inherited HIR plan already produced independently checked content-addressed
`MemoryWitnessId` records. Unknown type parameters remained
`SpecializationRequired`; they did not have an executable fallback. SSA and
bytecode structural metadata carried plan, type, and layout identities but not
the exact type witness. Nested copy lists executed in all four engines and
materialized key-free process outcomes, but their outer HIR witness remained
unselected.

## Implemented Slice

Each deterministic structural type selected by HIR now carries the exact HIR
witness bytes into verified SSA and validated bytecode. SSA and bytecode reject
zero and duplicate structural witness identities before execution. Compiler
conformance proves a product's SSA and bytecode identities are present in the
retained HIR witness authority.

The producer and independent HIR verifier now select `list<list<T>>`
recursively when the child is a concrete selected capacity-32 segmented list
whose transitive element mode is `copy`. The resulting witness records an
ordinary-region domain, region-handle copy strategy, process-codec eligibility,
exact child witness, selected storage, and capacity 32.

Nested strings, unknown parameters, affine bytes, borrows, resources,
structural products/enums, and transformed or unresolved recursion remain
rejected. No runtime key enters a process outcome. No new count, tracing edge,
root scan, or collector metadata was added.

## Exact Cost

The witness field adds 32 bytes to each bytecode structural type record. The
validator's deterministic structural metadata charge therefore changed from 93
to 125 bytes per structural type. Validation uses one bounded uniqueness set.
There is no runtime witness dispatch in this slice and no generated-code size or
call-path change. Nested copy lists retain the measured capacity-32 segment and
existing arena-local key representation.

Revision-12 exact identities include typed HIR
`9444655cc5e865ef8a448f8abbb0e987cd2a590da708f5a003f6a0cfec04e7d1`,
verified SSA `c7e8e68c4eea921d350c4ba375180283a2b105d1d24a8aa6cd67cc964fb75c1f`,
and bytecode `5ae97f438122448609baa73506a5e172ca572caa46c701cf825b2e74ec04d491`.

## Baseline Gates Actually Run

Before implementation, these passed on `ef48675f`:

```text
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-targets
cargo run --locked -p lkjscript-xtask -- check-unsafe
cargo run --locked -p lkjscript-xtask -- quiet verify
cargo run --locked -p lkjscript-xtask -- structure check
cargo run --locked -p lkjscript-xtask -- structure audit --json
cargo build --locked --workspace --release
cargo run --locked -p lkjscript-app --bin lkjscript -- package check
python3 meta/results/ai-authoring/validate.py meta/results/ai-authoring/results/*.json
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

The complete documented release runtime command set passed, including VM,
baseline, proof, auto, Brainfuck, editor, HTTP, bulk bytes, durable files,
SHA-256, and SQLite. Focused semantic session, runtime, database, JIT, process
cell, application-control, and session-broker suites passed. An attempted test
target named `canonical` failed because it is a module; the correct
`cli_contract` target passed all eight tests.

Previously omitted safety gates also passed on the baseline:

```text
cargo +nightly miri test --locked -p lkjscript-core \
  --test segmented_lists --test region_products --test structural_roots
cargo +nightly miri test --locked -p lkjscript-core --test value_runtime -- \
  --skip tests::deep::deep_image_conversion_clone_export_and_release_are_iterative
```

The first command passed 11 tests. The second passed 17 and explicitly filtered
the inherited deep stress. Focused address, leak, and thread sanitizer runs each
passed the four structural-root tests. Rust nightly has no supported undefined
behavior sanitizer. `cargo-fuzz`, `wasmtime`, Wine, and cross execution tools
were unavailable. The available `wasm32-wasip1` host/database build passed;
Windows, macOS, and Linux AArch64 targets were not installed.

## Focused Implementation Gates

The nested-list slice ran compiler nested tests, strict compiler Clippy,
structure check, and the existing evaluator/VM/forced-baseline/forced-proof
nested-list conformance with zero reported failures. Structural witness work ran
all IR, core, and compiler tests; focused malformed SSA and bytecode witness
tests; strict Clippy for all three crates; formatting; structure; and diff
checks. Exact final full-workspace and separate safety results are recorded only
if rerun after the integrated contract and lock update.

## Negative Evidence And Remaining Blockers

This slice deliberately retains these blockers:

- no hidden witness parameters, specialization budget, package witness
  requirements, or residual native ABI;
- no list of string, path, product, enum, option, or result owners;
- no sealed segment or sealed language-value selection;
- no resource-bearing aggregate, borrowed `str`, ranged source view, or
  capturing closure;
- no long-lived VM state root, provider plane, generic database source
  operations, contained process directory, or stateful lkjscript service.

The 32-byte metadata cost is accepted for exact validation evidence, not as a
claim that full residual witness dispatch is free. Structural list elements
remain blocked because copying a root key would alias one unique owner; they
need detached-image copy or sealed dependency ownership before selection.
