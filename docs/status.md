# Status

Updated 2026-11-10. Maintained compiler graph:
`rev_7eca6279f28d37e95c986dae91dbe58139a1e4e25948c6cb8a76c201effc4edc`.

## Works

Ordinary commands execute once on the VM. `bash tools/check.sh` builds and runs
one routine suite; focused and slow selections require test-name filters. Compiler
probes share one immutable preparation. Historical publication gates are retired.

`tools/selfhost/typed-input.lkjc` decodes all 35 structural type constructors;
`typed-dag.lkjc` resolves child identities, builds reverse adjacency and orders
dependencies using records, variants, matching and persistent maps/lists. Structured
errors retain entry and byte locations. The Rust evaluator is this path's bootstrap.
All 98 maintained types and edges agree with independent Rust and cursor checks:
43 ms for the typed DAG pass versus 2,398 ms for the cursor pass. Chain and wide
probes reach 4,096 types in under a second. The selfhost README holds API and
notation measurements.

Host semantic work reserves stacks from physical memory; native programs raise
their soft stack allowance within the inherited hard limit. Depth guards and
ordinary VM call/operand ceilings are removed. Maps, owned sequences, read-view
storage and typed-data codecs have no million-item admission guard; JSON has no
item-count policy. Their enforcing counters and batching item accounting are gone.
Lifecycle probes pass at 262,144, 1,048,576 and 2,097,152 items (1.52 GiB high-water
RSS), and type admission passes through 2,097,151 visits with cancellation/retry.

## Bootstrap

A clean checkout builds the Rust seed from source and locked dependencies:

```sh
cargo build --release --locked --bin lkjscript
bash tools/selfhost/verify.sh
```

`tools/selfhost/project` is the maintained source seed. The Rust native emitter
produces Linux x86-64 C1; C1 builds C2 and C2 builds C3. The controls check C2 = C3,
Ahost = A1 = A2 = A3, independent behavior and source mutations. The native path
covers the maintained closure. Run the new evaluator path with
`bash tools/check.sh focused typed_compiler_input` and its explicit slow controls.

## Verification and remaining work

Focused checks passed 13 tests; four selected scale/maintained controls passed in
75.09 s. The broad routine ran 3,575 cases in 1,039.15 s: 3,572 passed and three
stale generated-guide/item-policy assertions failed. After refreshing the guides
and replacing quota expectations, all three focused checks passed in 77.91 s.
The routine left 63 slow controls ignored. The production-change test rebuild took
187 s (142 s for the later small correction); the final release product build took 95.1 s.

Byte controls, encoded widths, modeled allocation accounting, selected invocation
quotas and kernel/associated-work ceilings remain. The ten-minute routine target
remains open; complete custody fixtures still dominate its tail.

## Next action

Project current meaning directly from `RepositoryView::require_current_validation`
and its validated snapshot, with exact dependencies from the admitted package
closure. Introduce compiler input without retirements or revision cores; attach
publication identity in the caller/packager after compilation. Reuse the typed
structural pass. Check equal meaning under different histories, changed executable
meaning, missing live references and two exact dependency revisions, then run the
maintained native/bootstrap controls when adopting the new input.
