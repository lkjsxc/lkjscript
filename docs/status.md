# Status

Updated 2026-11-10. Compiler graph:
`rev_7eca6279f28d37e95c986dae91dbe58139a1e4e25948c6cb8a76c201effc4edc`.

## Works

Ordinary commands execute once on the VM. Explicit graph tests retain reference
comparison. `bash tools/check.sh` builds and runs the routine suite once; focused
and slow selections require test-name filters. Historical gates, seals, receipts
and release-publication machinery are retired. Twelve compiler probe modules
share one public transaction and one immutable graph/program.

Host semantic work uses stacks sized from physical memory; native executables
raise their soft stack allowance within the inherited hard limit. Structural,
expression, JSON/value, VM call/operand and parallel nesting guards are removed.
Finite semantic checks reject growing instantiations and cyclic lexical authority.

The compiler's dependency-count queue reduced digest-ordered task-type admission
at depth 1024 from 335.3 s to 4.34 s in Rust-native execution; self-produced native
took 3.32 s. The scale batch's sampled peak process RSS was 1.28 GiB.

`tools/selfhost/typed-input.lkjc` builds a catalogue with records, variants,
matching, generic lists and structured errors on the evaluator. All 98 types match
the maintained 26,868,750-byte carrier, Rust objects and cursor reader. Type bodies
remain opaque; lookup scans the list. The selfhost README records size/token costs.

## Bootstrap

Clean checkouts build the Rust seed from source and locked dependencies:

```sh
cargo build --release --locked --bin lkjscript
bash tools/selfhost/verify.sh
```

`tools/selfhost/project` is the source seed. The Rust native emitter produces Linux
x86-64 C1; C1 builds C2 and C2 builds C3 from the same raw source. The control checks
C2 = C3, Ahost = A1 = A2 = A3, source mutations and independent behavior. Native
support covers the maintained closure; new idiomatic components use the evaluator.

## Verification and remaining work

The routine passed 3,571 tests in 1,053.43 s; 60 slow tests were ignored. Sampled
peak process RSS was 7.06 GiB. A production-change test rebuild took 195 s, the
later test-only rebuild 127 s, and the release product build 91 s. Focused lifecycle,
reader/cache/mutation, public CLI and geometric depth/stack probes pass.
All three maintained native controls passed in 2,159.24 s: C2 = C3 and
Ahost = A1 = A2 = A3, independent behavior and source mutations, C1/C2 same-process
cleanup, and the full semantic corpus including 100,001-expression sequences.

The ten-minute routine target remains open. Its largest case takes about 519 s
checking sixteen complete custody fixtures across engines; a stage-preserving
reducer is the next test improvement. Byte/item/work quotas, modeled allocation
charges and format-width limits remain.

## Next action

Extend the typed reader into structural-child decoding and reverse adjacency, compare its
DAG pass with the cursor implementation, and use measured collection-update costs
to choose a mutable collection primitive. Remove accounting and narrow compiler
input to current meaning plus exact dependencies alongside that work.
