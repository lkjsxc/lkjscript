# Final No-Tracing Runtime Evidence

[Memory authority](../decisions/memory/collector-free-deterministic-memory.md)

## Status

**Current on Linux x86-64 for the implementation and focused/full local gates
listed below.** This is the zero-family cutover: no migration registry,
collector, traced object storage, liveness map, collection service, barrier,
collector configuration, or collector metric remains.

## Closed Cutover

- Products, enums, errors, options, results, strings, paths, and regular finite
  recursive aggregates use bounded structural images.
- Capacity-32 segmented lists and selected acyclic products use typed
  invocation-local regions. Their keys reject at process boundaries.
- Process outcomes retain only scalars, symbols, unique bytes, key-free
  structural images, and canonical owned-list tables. Removed wire tags,
  runtime keys, malformed cycles, and unreachable list records reject.
- Resource-bearing results use a bounded affine VM/evaluator adapter; they do
  not enter structural images or native invocation-region storage.
- Native frames retain typed homes, bounds, cleanup obligations, and structured
  outcome state. Root maps, collection publication, and root writeback are
  absent.
- `LKJ-RUNTIME-NO-TRACING-COLLECTOR` is unconditional. It rejects the removed
  directories and collector identifiers across crate sources.
- Unknown generic arguments and transformed/nonregular recursion remain
  specialization blockers. No hidden VM type-variable metadata is treated as a
  native witness ABI.

## Environment

The integrated working tree was based on `origin/main`
`2f539fa4166892f47951f8478189379f36eb0b2e`; its final pre-squash implementation
checkpoint was `a67bb6e418f3b5cb66a97fdeda98ba02f023e69e`. Verification used Linux
`7.0.0-27-generic` x86-64, `rustc 1.96.0 (ac68faa20 2026-05-25)`, and
`cargo 1.96.0 (30a34c682 2026-05-25)`.

## Executed Evidence

```text
cargo check --locked --workspace --all-targets
  passed with no warnings
cargo test --locked --workspace --all-targets
  passed; 59 suite-result records, zero failures
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
  passed
cargo run --locked -p lkjscript-xtask -- check-sources
  passed, including LKJ-RUNTIME-NO-TRACING-COLLECTOR and the complete source corpus
cargo test --locked -p lkjscript-app --test jit_engines
  passed; 37 tests
cargo test --locked -p lkjscript-app --test numeric_contract
  passed; 10 tests
cargo test --locked -p lkjscript-contracts --lib
  passed; 18 tests
cargo test --locked -p lkjscript-ir --lib
  passed; 56 tests
cargo run --locked -p lkjscript-app --bin lkjscript -- describe --json
  passed; platform revision 11 and coherent recomputed contract digests
cargo build --workspace --release --locked plus the complete runtime acceptance list
  passed; VM, baseline, proof, auto, Brainfuck smoke, editor, HTTP, bulk-byte,
  durable-file, SHA-256, and SQLite workloads all exited zero
python3 meta/results/ai-authoring/validate.py meta/results/ai-authoring/results/*.json
  passed
cargo run --locked -p lkjscript-xtask -- check-unsafe
  passed after moving the registered native boundary to runtime-value services
cargo +nightly miri test --locked -p lkjscript-core --test segmented_lists \
  --test region_products --test structural_roots
  passed; 11 tests
cargo +nightly miri test --locked -p lkjscript-core --test value_runtime -- \
  --skip tests::deep::deep_image_conversion_clone_export_and_release_are_iterative
  passed; 17 tests and one explicitly filtered deep stress test
```

The exact source audit found zero forbidden collector markers outside the gate's
own marker registry. The collector and JIT heap directories are absent.

## Final Separate Gates

```text
cargo run --locked -p lkjscript-xtask -- quiet verify
  passed after the one-commit revision-11 squash; all workspace test commands
  reported zero failures

docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
  passed with result=ok; canonical verification and every configured smoke passed

python3 meta/benchmarks/jit/benchmark.py \
  --binary target/release/lkjscript \
  --output /tmp/lkjscript-zero-jit-benchmark-clean.json
  completed from clean commit 419825823715352cd3c6ed9c63431f00db15dec5;
  retained a Rejected result with SHA-256
  93b2ed90d94a3196d783b50de3a938f8ca57729b416dff39ef6b9828678bef90
```

The performance result is retained at:

[Retained result](../../meta/benchmarks/jit/results/final-no-tracing-linux-x86_64-rejected.json)
Same-commit optimizing native execution improved from median 5,155,943 ns to
4,247,615 ns, or 1.213844x, and exceeded the two-MAD criterion. Exact outcomes,
streams, tier entries, proof evidence, structural runtime accounting, and W^X
passed. The mechanical verdict is nevertheless **Rejected**: historical scalar
native and process ratios were 1.353910x and 2.916027x, above their 1.05
ceilings. No optimizing-performance or no-regression claim follows.

Miri's combined command timed out while executing the 12,000-level deep-image
stress after the first 11 tests passed. The two successful commands above reran
all other selected structural tests. The deep stress remains covered by normal
workspace testing, not by completed Miri evidence. Sanitizers and non-Linux
platforms were not tested.
