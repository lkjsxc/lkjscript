# Matched FIFO representation costs

This experiment uses the same native two-stack algorithm, carrier operations,
operation traces and independent expected results for structural and nominal
queues. The [structural request](structural.lkjc) copies the earlier standard
FIFO operations into an experiment-owned module. The
[nominal request](nominal.lkjc) changes only the queue type and adds its owning
declaration. Both retain the refill algorithm, argument order and
`fifo-front` result's `borrow-from queue` relationship. These requests are
experimental inputs; they do not replace the accepted standard library.

Three lanes separate two effects:

| Lane | Executable | Representation |
| --- | --- | --- |
| baseline-structural | Retained predecessor copy | Structural owned product |
| candidate-structural | Candidate copy | The same structural owned product |
| candidate-nominal | The identical candidate bytes | Public nominal owned record |

The candidate nominal/structural comparison holds executable bytes and the
selected standard fixed while changing the queue declaration, type references
and derived artifact. The candidate structural/baseline comparison changes
both the executable and selected standard generation; it cannot attribute a
change to nominal support alone. Each lane builds its own artifact through the
public product; incompatible historical artifacts are not relabeled as current.
The experiment includes the ordinary standard dependency and complete public
artifact preparation, so preparation times and metadata also include that
lane's selected standard package.

## Retained literal consumer inputs

Retain `library.lkjc`, `steps.lkjc`, `trace.lkjc`, `elements.lkjc` and
`application.lkjc` in two input directories, `structural/` and `nominal/`.
The structural copies come from predecessor source
`7285431327104ea539e74858064ec735d71216be`; the nominal copies come from the
current [consumer](../README.md). For this worktree they were retained at
`.artifacts/owned-nominals/cost-inputs/`, with their origin recorded in
`origin.json`. Use new input paths when reproducing the capture; preserve
previous inputs and failed measurements.

The runner checks that those two sets differ only in queue type annotations.
It changes the queue-operation module locator to the experiment's `fifo-cost`
module, writes the complete literal request, and lets ordinary `change plan`
and `change apply` own all semantic admission and publication. It does not
construct a graph, resolve types or compute native program results.

The host oracle uses an ordinary FIFO array with push, shift, front and clear.
Cells, two-byte buffers and stamped packets preserve the consumer's three
different observations. Every successful invocation must equal the complete
independent event trace, drainage, empty length and reuse result.

## Author, build and measure

The [runner](measure.mjs) uses Node's standard library and public `lkjscript`
commands. Its text, oracle and compact-output checks invoke no product executable:

```sh
node --check examples/owned-fifo/costs/measure.mjs
node examples/owned-fifo/costs/measure.mjs --self-check \
  --inputs .artifacts/owned-nominals/cost-inputs
```

After the candidate is built and all owned builds have stopped, prepare fresh
projects and artifact copies outside the checkout's accepted graph directories.
Choose an absent output path, and select the exact copied candidate executable:

For a predecessor/current timing comparison, first retain both producer
receipts and verify matching compiler profiles, toolchain, feature selection
and build command. The selected repeat uses `cargo build --workspace --release
--locked --message-format=json` for both generations, with optimization level
3, no debug information, and assertions/overflow checks disabled. The
[workspace release baseline binding](../../../.artifacts/owned-nominals/baseline-workspace-release-binding.json)
records the predecessor source `99232e109758f646c88d1f36f59d93bf1e1b8b38`,
producer command and SHA-256
`daf81df9850ad7db14c71fb61f9b113d13ad3eafcd2c944b4adaf9a7bcf8190b`.
Retain an immutable current copy from the matching workspace-release producer
before preparation. Expected standard revisions verify semantic selection;
they do not verify compiler configuration. The completed matched-profile
measurement is reported below.

```sh
node examples/owned-fifo/costs/measure.mjs --prepare-only \
  --baseline-executable .artifacts/owned-nominals/baseline-workspace-release-lkjscript \
  --candidate-executable /absolute/path/to/copied-workspace-release-candidate/lkjscript \
  --expected-baseline-standard-revision rev_55fb7bfafcef4ec97b158b23e5f8d49c244e8cf64d32786fa81448f136effc59 \
  --expected-candidate-standard-revision rev_6818785d8919028a69659b24df495b2d7a42f2676a95fd1aa0ae143a32623013 \
  --inputs .artifacts/owned-nominals/cost-inputs \
  --output /absolute/path/to/new-fifo-cost-run
```

Preparation creates each generic supplier before its concrete consumer,
stages an exact exported supplier, plans and applies literal requests, checks
both evaluators, verifies unchanged canonical drafts, and builds a detached
artifact. It records exact package, revision, transport and SHA-256 identities.
Then it removes only its own source projects and active transports. Literal
requests, drafts, command arguments, outputs and failures remain retained.
It never edits `packages/standard` or any other existing graph.

The optional expected-revision arguments make both copied executables inspect
their embedded standard before any project is authored or artifact is built.
A mismatch rejects and retains the selected identities and original output.
Use the observed public accepted revisions for a later increment. A bootstrap
binary can still embed the preceding standard even after the source graph has
advanced; select the copied binary rebuilt after standard regeneration.
Command-template project creation binds and stages that exact embedded standard
automatically. The generic supplier export adds its own exact dependency later.

Run the measurement separately so compilation does not overlap the samples:

```sh
node examples/owned-fifo/costs/measure.mjs \
  --measure-only /absolute/path/to/new-fifo-cost-run --samples 7
```

The matrix has three carriers and six traces: bursts of length 0, 31, 33, 256
and 1,024, plus the 512-operation deterministic mixed trace. Each burst includes
peek, pop and further enqueues while outgoing elements remain. Completion also
drains and reuses the empty queue. One warmup triple and seven measured triples
per workload produce **432 executions** over 18 workloads. Lane order rotates
through all six permutations. The runner serializes execution, retains warmups
and all original samples, and reports every workload, including unfavorable
ratios. A later repeat creates a new measurement directory.

## Counters and interpretation

Each invocation retains the complete public `production-observation` and raw
command output. Report `allocated_bytes`, `allocation_charges`,
`type_metadata_bytes`, `type_derivation_steps`, instruction/call counts and
`value_work` separately from wall time. The runner reports medians for public
preparation, invocation and result-encoding nanoseconds, plus host process wall
time. Preparation and process startup are not steady execution, and result
encoding is not queue transfer work.

Every successful invocation must release all public live handles, call frames,
locals, operands, transactions and type bindings. Foreground cleanup must stop
admission with no remaining tasks or failures; the structured executor must
close dispatch with no active dispatches or remaining workers. Executable and
artifact hashes must remain unchanged through the complete run.

Allocation counters describe modeled reservations, not allocator calls, copied
bytes, retained capacity or RSS. Joined cleanup establishes the absence of
remaining public custody, not how many custody envelopes were created. Actual
envelope counts and preservation of backing allocation identity are test-only
storage observations. The complementary `owned_nominal` runtime tests check
one nominal envelope, unchanged child identity and backing allocations across
transfers, shared worker borrowing and zero remaining owners/loans. Their fresh
test receipt must be reported separately from these public measurements.

The literal inputs alone establish no timing or allocation improvement. A
completed run's `measurement-*/samples.jsonl` and `summary.json` are the
observation authority. A source or workload change requires a new run;
preserve failures, refusals and unfavorable originals. The constant amortized
sequence-transfer bound does not by itself establish constant VM time,
reduced allocation or a performance improvement.

## Observed results, 2026-10-11

The retained sixth run used matching release producers and completed all
**432 executions**: 54 warmups and 378
measured samples across 18 workloads. All complete results matched the
independent oracle, every invocation joined cleanup with zero public live
resources, and executable/artifact hashes remained unchanged. Preparation
passed 117 supplier and 118 consumer tests in each lane, with production and
reference agreement; measured invocations used production execution and the
independent host oracle. The measurement ran serially without overlapping
Cargo or test jobs, on Linux x64 / AMD Ryzen 9 9955HX with Node v24.18.1.

The original authorities are the
[summary](../../../.artifacts/owned-nominals/cost-sixth/measurement-1791693814286/summary.json),
[all samples](../../../.artifacts/owned-nominals/cost-sixth/measurement-1791693814286/samples.jsonl),
[prepared identities](../../../.artifacts/owned-nominals/cost-sixth/prepared.json)
and [environment/input bindings](../../../.artifacts/owned-nominals/cost-sixth/environment.json).
Each sample names its retained command receipt, raw output, arguments and
complete result file. A read-only audit reconciled all 432 of these originals
with the summary medians, observations and expected results.

The two current lanes used the same copied `candidate-workspace-release-lkjscript`,
SHA-256 `1d95cb7a214b9228be1832a5e86b428cd024b0c5d0a97b40515d78c4566c1657`.
The predecessor workspace-release copy was SHA-256
`daf81df9850ad7db14c71fb61f9b113d13ad3eafcd2c944b4adaf9a7bcf8190b`.
The embedded standard revisions matched the exact expected revisions in the
preparation command above. `prepared.json` binds each separately built artifact
and exported supplier to its full identities and SHA-256. `environment.json`
binds every retained literal input, including the predecessor source named
above and the captured current authored proposals. The measured runner was
SHA-256 `da9fd3118409df866eb7561ac827f8af82a729018139cabd207422e92451a9b8`.
These receipts identify those copies and inputs; they do not certify subsequent
source changes or publication.

The [current producer/copy binding](../../../.artifacts/owned-nominals/candidate-workspace-release-binding.json)
and predecessor binding above identify matching Cargo artifact profiles and
empty feature selections: optimization level 3, no debug information, debug
assertions disabled, overflow checks disabled and `test=false`. Both copies
come from `cargo build --workspace --release --locked --message-format=json`.
This removes the fifth run's compiler-profile confound. The generation
comparison still changes the executable, selected standard and derived
artifact together; it cannot isolate the cost of one internal mechanism.

Each table entry gives two ratios of **median invocation time**:
`current nominal / current structural`, then
`current structural / predecessor structural`. Values above 1 mean longer
measured time. All
workload ratios are retained, including the unfavorable ones.

| Trace | Cells | Two-byte buffers | Stamped packets |
| --- | --- | --- | --- |
| burst-0 | 0.945 / 0.936 | 0.916 / 0.980 | 0.989 / 0.971 |
| burst-31 | 0.860 / 0.976 | 0.927 / 1.008 | 0.948 / 1.041 |
| burst-33 | 0.946 / 0.975 | 0.943 / 0.912 | 0.942 / 1.008 |
| burst-256 | 0.915 / 0.941 | 0.970 / 0.938 | 0.917 / 1.064 |
| burst-1024 | 0.920 / 0.958 | 0.975 / 0.926 | 0.928 / 1.049 |
| mixed-512 | 0.910 / 0.939 | 0.895 / 0.980 | 0.936 / 1.055 |

Nominal invocation medians were lower in all 18 comparisons: ratios
0.860–0.946 for cells, 0.895–0.975 for buffers and 0.917–0.989 for packets.
For burst-1024, current structural/nominal medians were 85.810/78.946 ms for
cells, 91.436/89.129 ms for buffers and 126.855/117.687 ms for packets.
The current structural/predecessor ratios were mixed, ranging 0.912–1.064.
The six cell medians were lower, but buffer burst-31 and five packet workloads
were higher; packet burst-256 measured 29.968 ms in the predecessor versus
31.879 ms in current structural execution. These unfavorable comparisons
remain part of the result.

Preparation and process startup account for much of the complete invocation
path. Current nominal preparation medians were 112.621–122.078 ms; process wall
medians were 126.565–263.295 ms. Nominal/structural preparation ratios were
0.952–1.019, wall ratios 0.951–1.015 and result-encoding ratios 0.558–2.483.
Current structural/predecessor preparation ratios were 0.984–1.046, wall
ratios 0.970–1.062 and result-encoding ratios 0.482–1.709. Thus the lower
nominal invocation medians did not produce
a consistent process-wall improvement. Seven samples in one host batch do
not establish portable speedups or an implementation-level cause.

Every modeled counter repeated exactly across the seven samples for a given
lane/workload. Current nominal versus current structural saved **9,488
modeled bytes** and 362 type-derivation steps in every workload. That entire
byte difference was type metadata: 3,244,673 versus 3,254,161 bytes. The
predecessor used 3,258,239 metadata bytes; current structural saved 4,078 bytes
and 1,388 derivation steps relative to it. These are fixed per-invocation
differences across all queue lengths, including the empty trace, rather than
savings that grow with enqueues. Allocation charges, instructions, calls,
collection-item counts and the complete `value_work` object were identical
across all three lanes for each workload. For cell burst-1024, nominal execution
reserved 60,340,600 modeled bytes with 546,239 charges, versus 60,350,088 bytes
with the same charges for current structural execution.
The fixed deltas describe these exact prepared artifacts. The separately
prepared fifth run had different metadata deltas, so these counts are not a
universal savings constant for nominal declarations.

Explicit input-prefix and result admission still grow with the workload.
Bursts 31, 33, 256 and 1,024 contain 104, 110, 854 and 3,414 input actions;
their input-admission counts are 313, 331, 2,563 and 10,243, exactly three
nodes per action plus the outer list. Cell raw-result admission counts are
442, 466, 3,592 and 14,344. Cell instruction counts grow from 103,788 at
burst-256 to 413,804 at burst-1024, while list node visits grow from 3,619 to
18,087. Internal guard descendant visits remain zero in every lane and size,
and metadata bytes/derivation steps stay fixed within each lane. This
distinguishes explicit boundary and trace/output work from internal descendant
walks; it does not establish constant total VM work per queue operation.

These counters are modeled reservations and logical work, not allocator calls,
RSS, physical copied bytes, custody-envelope counts or API billing. They show
fixed metadata savings without reduced allocation charges or value work in
this witness. Actual storage-envelope/backing-identity evidence remains the
separate runtime-test obligation described above.

## Qualified fifth-run evidence

The earlier fifth run also completed 432 executions with equal results and
joined cleanup. Its [summary](../../../.artifacts/owned-nominals/cost-fifth/measurement-1791692906183/summary.json),
[all samples](../../../.artifacts/owned-nominals/cost-fifth/measurement-1791692906183/samples.jsonl)
and [prepared bindings](../../../.artifacts/owned-nominals/cost-fifth/prepared.json)
remain unchanged, including every favorable and unfavorable observation.
Its current nominal/structural invocation ratios were 0.872–1.058 on identical
candidate bytes; its 11,312-byte modeled metadata difference belongs to that
preparation.

That run compared candidate SHA-256
`5e190171d51618379196d7a21917c03d0a74b7a63618658c28cbc0be7d54ca06`,
whose [producer/copy binding](../../../.artifacts/owned-nominals/product-build-eighth-executable.json)
and [Cargo output](../../../.artifacts/owned-nominals/product-build-eighth.jsonl)
show optimization level 2 with debug assertions/overflow checks enabled,
against predecessor SHA-256
`81fc61a73fab9190b3da09d4f64e52cd681628a6700b268400bfccddf4a3a5b0`.
The predecessor matches the
[retained release lifecycle executable](../../../.artifacts/lkjscript-dev/check/1791680514015518854-558889-0/retained/release_command_lifecycle/0),
whose [producer output](../../../.artifacts/lkjscript-dev/check/1791680514015518854-558889-0/release_command_lifecycle.stdout.log)
and [gate receipt](../../../.artifacts/lkjscript-dev/check/1791680514015518854-558889-0/receipt.json)
bind optimization level 3 with assertions/overflow checks disabled to a
`cargo test --release` public CLI producer. **The fifth run's predecessor/current
timing ratios are compiler-configuration-confounded and do not establish an
implementation regression.** The sixth run above supplies the matched-profile
comparison; it does not rewrite or relabel the fifth run.
