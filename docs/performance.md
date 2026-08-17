# Current evidence and decisions

This file owns reproduced measurements, controlled observations, selected architecture, limits,
and reversal conditions for the current checkout. It is not a campaign diary. Exact structured
values are retained in
[`evidence/20260818-application-closure.json`](evidence/20260818-application-closure.json); prior
semantic-core evidence remains a historical baseline, not current authority.

## Measurement boundary

The application-closure work started from commit
`da62ef361c6b5fd5a43ed440d1da45733614c5d7` on branch `main`. Measurements were made on Linux
7.0.0-29-generic x86-64, AMD Ryzen 9 9955HX, 32 logical CPUs, 32 GiB RAM, and ZFS with a 131,072-byte
block size. The active stable toolchain was rustc/cargo 1.96.0; `Cargo.lock` SHA-256 was
`d23b75fc162e485b7149d92f1e3349f3cca39f00420a9fef68f8abea6c405620`.

The unchanged starting checkout passed formatting, all-target/all-feature Clippy, all tests, release
build, and all six production examples. It contained 218 tests, of which four were ignored by the
ordinary boundary. Times below are single observations unless a sample count is explicit; they are
not benchmark distributions. `/usr/bin/time` was unavailable, so no process maximum-RSS result is
reported. One shell-timed release build after the contract cutover took 110.223 seconds; this was an
incremental LTO build, not a clean-build distribution.

## Application-closure observation

The retained `binary-canonicalizer` driver creates an incomplete program, repairs it, runs semantic
controls, renames one field, reopens history, and then performs the application lifecycle using only
production commands. One final run observed:

| Measure | Result |
|---|---:|
| application artifact | 3,207 B |
| semantic nodes | 69 |
| immutable release cases | 3 |
| validate-only / published digest equality | pass |
| repeated-build byte equality | pass |
| failed release blocked before output | pass |
| source workspace removed | pass |
| standalone validate / inspect / test / typed run / stream | pass |
| corrupt artifact rejected | pass |
| application CLI processes | 10 |
| application input bytes | 3,547 |
| application output bytes | 12,155 |
| application diagnostic bytes | 133 |
| application process wall boundary | 30,933,533 ns |
| intentionally failing processes | 2 |

The observed application digest was
`1f46d25de8e87b47c6b7f2c53e756d97dfd9474315a4d2b69448b2e6a0954b45` and the reconstructed semantic
digest was `4de752e7748ddce75be2796e8888030e94598ffbb8992cbd2d691b17330da6a9`.
These are one run's exact values, not checked-in release coordinates: each fresh example creates a
random workspace identity. Determinism is demonstrated by the two equal builds from the same exact
workspace revision.

The preceding author/repair/history workload used 43 version-10 RPC calls in two direct sessions,
four total Engine opens, zero socket connections, 56,615 request bytes, and 131,520 response bytes.
Its summed request boundary was 104,742,017 ns. The dependency-closed schema projection was 87,342
JSON bytes. Its dense payload boundary accepted 1,445 bytes and rejected 1,446 with
`managed_visible_byte_policy_exceeded`.

The workload's independent oracle remains the driver: exact sparse/dense canonical bytes, expected
typed traps, historical results, artifact equality, source-state removal, and corrupt-byte
rejection. The artifact implementation cannot make that external oracle pass by changing expected
runtime behavior.

## Application architecture decision

Serious candidates were a renamed workspace snapshot, semantic closure, serialized Core IR only,
semantic plus executable cache, and editable text as distribution authority. The selected artifact
is a target-neutral dependency-closed semantic projection with one exact manifest and release-case
set.

It wins because it can independently validate, inspect, test, recompile, and run without history or
private compiler state. Renaming a snapshot would retain unrelated development state; IR-only would
create a second untrusted executable format and weaken inspectability; a combined cache would couple
independent versions without measured startup value; editable text would promote a proposal view.

Direct-cutover effects:

- application magic/version/hash domain are independent from workspace artifacts;
- package entry is removed from application semantics, leaving one manifest entry;
- workspace history, HEAD, idempotency, aliases, caches, paths, unrelated declarations, and Core IR
  are absent;
- source workspace/revision IDs are retained for exact nominal equality and local diagnostics;
- the artifact is explicitly run-only and makes no import/package-continuity promise; and
- application content digest remains integrity/reuse, not release or entity identity.

Reversal: replace the run-only closure only when a real independently released reusable component
requires exports, dependency identity, provenance, import/remapping, or when measured repeated
compile/startup cost justifies a separately verified executable cache.

## Release-test and invocation decision

The selected test form is application-local immutable data: canonical name, exact function target,
typed arguments, expected value or stable trap, and exact Run policy. No durable test entity,
assertion language, mocking hook, skipped state, or test-only operation was added. All cases run in
lexical order and must pass before build publication. The application digest binds the complete
test set; validate-only receipts and inspection provide bounded deterministic review.

This is weaker identity than a semantic test entity but sufficient for current selection, review,
artifact inclusion, and transfer. Reversal: add durable identity only when independent rename,
repair targeting, cross-artifact reference, or workspace history consumes it. Add richer test
semantics only for an effectful/property-testing workload that exact invocation cases cannot express.

Both exact typed invocation and pure `bytes -> bytes` process invocation are retained. The stream
profile closes a useful command-line lifecycle with no accepted host authority, permissions,
partial external action, or resource-owning value. Reversal: add a narrow effect only for a retained
application whose complete interaction cannot cross an explicit value/byte boundary.

## Runtime complexity decision

The current planner, verifier, and managed store were compared with the allocate-new oracle and a
forced-shared fallback on a 512-octet loop-carried append shape used by the canonicalizer:

| Measure | Allocate new | Ownership/reuse | Forced shared |
|---|---:|---:|---:|
| cumulative visible bytes | 131,840 | 131,840 | 131,840 |
| cumulative allocated backing | 131,840 | 1,528 | 131,840 |
| peak live backing | 1,024 | 513 | 1,024 |
| cumulative managed objects | 2,050 | 1,026 | 2,050 |
| copied backing bytes | 131,840 | 1,024 | 131,840 |
| reuse attempts / hits | 0 / 0 | 512 / 512 | 512 / 0 |

The existing small concat control also reports 32 copied/peak bytes for allocate-new versus 23 for
the optimized route, with one hit. Results, traps, fuel, logical resource charging, and cleanup stay
differentially checked.

The planner therefore remains: the absolute 130,816-byte copy reduction on the representative
append shape is material, while simple sharing reproduced the allocate-new cost. This is not a claim
that the current route wins every workload or that managed handles are language ownership. The
production cost remains 46,639 bytes in `ownership.rs`, 44,259 bytes in `managed.rs`, plus compiler,
interpreter, and verification integration.

Reversal: delete the planner/verifier/handles when broader application distributions show marginal
end-to-end benefit, or when a simpler safe representation matches logical limits with substantially
less source and verification. Any future fast route keeps a simple differential oracle.

## Contract-ownership decision

The manual `contract.rs` owner remains 153,227 source bytes. Current production consumers are the
machine-schema digest, agent help, context/document schema binding, command-local authoring facts,
strict RPC diagnostics, and dependency-closed schema projection. Current output sizes are 1,245
bytes for the compact manifest, 87,342 bytes for the canonicalizer's selected roots, and 136,735
bytes for the full projection.

Application records were not copied into this catalogue. `application.rs` owns their Rust types,
semantic validation, binary layout, and inspection projection; the CLI derives JSON directly from
those closed serde records and has command-local help. This narrows the catalogue to its existing
workspace-RPC consumers.

No macro-rules, proc-macro, build-script IDL, or generic JSON Schema candidate was retained. A
complete single-field-owner cutover was not implemented; existing workspace DTO/descriptor field
duplication remains. A partial generator would have created two active catalogues. Reversal: run
disposable representative additions through declarative/manual alternatives and delete the manual
catalogue only when one reviewable owner preserves strict codecs and materially lowers total source,
build, debugging, and Miri cost.

## Package, grammar, topology, storage, and executable decisions

| Domain | Selected decision | Current evidence | Reversal gate |
|---|---|---|---|
| package/module | Retain workspace containment and durable ownership; do not call it distribution. Strip package entry in application closure. | Validators, owner-scoped lookup, documents, review, examples, and nominal/function containment consume it; no second independently released component exists. | Implement exports/dependencies/import identity only for real reuse; collapse hierarchy only through a complete semantic cutover. |
| proposal grammar | Retain the sole bracketed editable document v1. | Exact-base/schema/scope/packet binding and parser oracles pass; no equal-capability source parser was completed. | Reopen only with an isolated bounded parser and equal-task correction/byte evidence; delete the loser. |
| direct Engine | Retain direct CLI primary and line session. | All production examples need no service lifecycle; application preparation is one typed Engine method. | Change only when measured open/restart dominates a retained workload. |
| optional daemon | Retain only as a diagnostic integration. | Exported `Client` plus socket framing/correlation/deadline/disconnect/shutdown/lock tests consume 32,936 production bytes, 77,569 integration-test bytes, and a 4,252,144-byte release binary. | Delete binary, client, transport, tests, and docs together when those consumers move or cease to matter. |
| workspace storage | Retain full canonical snapshots and one writer. | Prior body churn remains 443 B/revision; application work did not make restart or retained history dominant. | Prototype one replacement only after scaled application history crosses retained-byte and restart thresholds. |
| queries | Retain full scans and differential controls. | Application closure did not expose a measured scan bottleneck. | Add one narrow disposable index for a measured shared hotspot. |
| Core IR | Keep derived in memory and independently verified. | Standalone semantic compile is sufficient on the retained artifact; no cache/startup distribution shows need. | Serialize only when at least two of startup, repeated compile, dispatch, or artifact-size dimensions improve materially. |

## Source, binary, and context cost

Rust under `src/` and `tests/` grew from the audited 2,193,899 bytes / 62,103 lines to 2,287,367
bytes / 64,768 lines: +93,468 bytes and +2,665 lines. The two new application owner files occupy
90,664 bytes total; their production prefixes occupy 76,552 bytes and their inline unit-test
suffixes 14,112 bytes. No dependency, build script, proc-macro crate, unsafe Rust, or extra binary
was added.

| Release binary | Starting bytes | Current bytes | Change |
|---|---:|---:|---:|
| `lkjscript` | 7,011,520 | 7,261,216 | +249,696 |
| `lkjscriptd` | 4,260,656 | 4,252,144 | -8,512 |

The direct application vertical pays a visible source and main-binary cost. It earns that cost by
closing validate-only/test/build/inspect/transfer/run through public boundaries; it does not claim
source reduction. The active machine digest is
`e4257d3657752f3f3f0bfae148cec38610e9002c4a749205c846efb3368ba5e1`.

The final application observation uses 10 processes and 3,547 input / 12,155 output bytes for the
application portion. No provider call was made. These are byte/process proxies, not tokens or money.
The previous controlled provider observation is not reused to claim a benefit for this changed
surface.

## Adversarial and tool evidence

- Application unit tests cover exact round-trip, unrelated-node exclusion, standalone execution,
  failing release gates, old command/artifact versions, corruption, trailing bytes, no-overwrite
  publication, and injected failures before write, after write, after file sync, after link, after
  temporary cleanup, and after directory sync.
- Seed `0x1a11cafe20260818` drives 10,000 deterministic one-bit mutations across canonical
  application bytes; all reject. This is deterministic mutation coverage, not coverage-guided
  fuzzing.
- Workspace mutation smoke and prior focused Miri/AddressSanitizer evidence remain applicable to
  unchanged owners. Final current-tool results are recorded in structured evidence and the handoff.
- `cargo-fuzz` is unavailable and no stable fuzz target exists. No model checker or cross-platform
  application run was available. Provider telemetry and price are unavailable.

## Known limits and next evidence gate

The first artifact is run-only, holds one entry, contains public release cases, recompiles on every
run, and is verified only on Linux x86-64. It retains workspace identity and therefore is unsuitable
for import. Application context is command-local rather than a new context-packet purpose. The
manual workspace contract catalogue, broad integration suite, daemon adapter, and managed runtime
remain substantial complexity with explicit consumers and reversal gates.

The next gate is a second independently released application consuming one shared semantic unit.
Only that workload can establish whether reusable package artifacts, exports, dependency identity,
private test separation, or application-local identity remapping pay for their contracts.
