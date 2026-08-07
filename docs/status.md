# Current implementation status

**Status: currently implemented in this checkout.** This is a concise report of checkout behavior,
not a compatibility promise or normative specification. Code, tests, CLI definitions, schemas, and
manifests remain the executable authority.

## User path

The compiler still treats package files and the provisional line-oriented `.lkjscript` notation as
program authority. It loads exact package modules and imports, produces resolved typed HIR, checks
types/effects/ownership and a HIR memory plan, lowers and verifies SSA, validates bytecode, prepares
the program, and executes it.

Implemented source behavior includes typed functions and calls, bindings and explicit mutation,
conditionals and loops, nominal products and enums, exhaustive matching, generics and trait-dispatch
subsets, numeric conversions, bytes and byte vectors, lists, typed host resources, errors, and
explicit capabilities. Executable examples and compiler/runtime tests own the exact accepted
surface; this document does not copy their tables.

`lkjscript run` exposes one execution policy. It synchronously attempts the supported
baseline-native group reachable from `main`; lowering, installation, or typed pre-entry decline
drops the complete native attempt and executes the unchanged validated program in the VM. Direct
generated calls and eligible structural, resource, and unique islands execute inside the installed
group. Once native entry begins, its returned value, trap, exit, resource/deadline/host outcome, or
entered failure is final and the VM is not run. Unsupported I/O, generic, recursive-stack, and other
native shapes therefore remain valid through the generic VM path. There is no public engine,
threshold, automatic-transition, or forced-native option.

The broadly tested host is Linux x86-64. Portable Rust may build elsewhere, but another host or
native target is not claimed as tested.

## Phase 4 executable boundary result

Native image installation remains a pre-entry, failure-atomic operation. It validates image
integrity and contracts, accounts the object, applies relocations in a private RW mapping, seals the
mapping RX, and publishes installer usage only after success. Dropping an installed image releases
both its mapping and accounted lease.

Native invocation no longer returns one mixed `InvocationError` or a
`NativeStackBoundary { retry_safe }` heuristic. Collector-free `prepare_invocation` and explicit
`prepare_region_invocation` validate entry and typed arguments, materialize and reserve the
machine-call and runtime bookkeeping state, and perform immediate cancellation, deadline, resource,
and configured whole-group stack checks. Success returns a non-cloneable `PreparedInvocation`.
`enter(self)` consumes that value immediately across the unsafe generated ABI call and returns an
`InvocationReport` or `EnteredInvocationError`. Pre-entry and entered failures are disjoint types;
after entry, traps and host/resource/deadline results remain outcomes and no error is VM-retry safe.

The product runtime now uses that boundary for one synchronous baseline-native reachable-group
attempt before effects, with VM execution after any typed decline and no fallback after entry.
Declined installed state is dropped before VM construction; the VM receives the original validated
bytecode, inputs, and `ExecutionPolicy`. The VM is non-generic and has no JIT dependency, native
branches, or transition state. Product metrics report the actual `baseline-native` or `vm-fallback`
path, a nullable decline reason, the native-entry commit fact, and
preflight/lower/install/prepare/native/VM/total durations. Automatic thresholds, per-function call
records, retries, invalidation, lookup, runtime sessions, forced execution helpers, optimization
certificates, and the proof-oriented optimizer are deleted. Baseline normalization remains a small
independently verified sequence of constant folding, copy propagation, branch simplification,
reachability, empty-block forwarding, effect-aware dead-code elimination, direct-call resolution,
and canonical block ordering.

## Phase 1 scale and policy result

The ordinary trusted local path has completed the Phase 1 validity-policy cutover:

- there are no compiler profiles, cross-phase count budgets, or source/HIR/SSA count ledgers that
  decide whether an ordinary program is valid;
- trusted source loading and compilation select an explicit unrestricted source-byte policy;
- initial validation of compiler-produced bytecode selects `ValidationPolicy::Unrestricted`;
- ordinary local `lkjscript run` explicitly selects `ExecutionPolicy::Unrestricted`; the policy has
  no `Default`, and unrestricted resources are absent rather than maximum-value sentinels;
- the former token, child, top-level-form, nesting, per-source, aggregate-source, source-unit,
  HIR-memory, SSA-CFG, SSA-ownership-work, and retained-state-cell admission quotas are removed;
- source positions and spans, revision-scoped source nodes, HIR and SSA identities, executable
  operands and links, structural metadata, runtime structural identities, and process/outcome
  lengths use wide representations where they carry user-scale data, with checked conversion before
  host indexing; and
- parser/source-tree operations, package traversal, type and match usefulness paths, CFG traversal,
  structural graph operations, and deep destruction have explicit work stacks or equivalent
  stack-safe designs on their tested paths.

A small set of recursive compiler mechanisms remains behind
[`crates/lkjscript-compiler/src/stack.rs`](../crates/lkjscript-compiler/src/stack.rs). That localized
`stacker` wrapper grows repeatable heap-backed segments; its red-zone and segment sizes are private
tuning, not accepted-depth limits. It is not evidence that every recursive runtime, transaction, or
specialization path is stack-safe.

Committed positive tests cross former boundaries rather than merely increasing constants. Examples
include 20,000 nested source expressions and 2,048-deep match diagnostics on a 256 KiB thread stack,
10,000-block CFG verification, 44,000 owned SSA parameters/places, 300-field products and
300-variant enums, 1,024 parameters/arguments/locals, table index 65,536, and runtime structural
collections above 65,535. The relevant retained harnesses include
[`source_scale.rs`](../crates/lkjscript-app/tests/source_scale.rs),
[`executable_width.rs`](../crates/lkjscript-app/tests/executable_width.rs),
[`verification_cfg_scale.rs`](../crates/lkjscript-ir/src/tests/verification_cfg_scale.rs), and
[`ownership_scale.rs`](../crates/lkjscript-ir/src/tests/ownership_scale.rs). Some largest production
geometries are explicitly ignored release stress tests rather than default-suite coverage.

## Boundary policy and validation

Untrusted Semantic Source requests have explicit coarse request/source/response byte, request-count,
and cancellation policy. Untrusted artifact validation can select one total-artifact-byte limit.
Isolated process execution uses an explicit limited policy over fuel, VM values/frames, heap and
allocations, handles, output, wall time, hard-deadline requirements, and cleanup-report retention.
Process framing and codecs retain separate transport-byte policy. Exhaustion is distinct from
malformed input or language invalidity, and tests cover unchanged work failing under low policy and
succeeding under sufficient or unrestricted policy.

These boundaries are not yet one simplified daemon/process policy. Existing process protocol
argument, output, frame, diagnostic, and flush bounds and platform observation limits remain
operational constraints. They require classification and simplification before daemon and platform
work can be treated as completing the same cutover as ordinary local execution.

Path containment, symlink and import checks, package/lock validation, capability checks, persisted
state validation, process provenance, bytecode validation, relocation and W^X installation, native
entry checks, FFI, SQLite, and operating-system error handling remain fail-closed boundaries.

## Semantic Source bootstrap

The current Semantic Source JSON/stdio service provides source-derived snapshots, revision-labelled
node and entity reads, diagnostics, typed-hole context and legal-action queries, and atomic preview
or publish transactions for declaration rename, expression replacement, and typed-hole operations.
It stages and validates source publication and rejects stale revisions and failed preconditions.
Candidate and legal-action responses are complete when returned; if the response-byte policy cannot
carry the complete result, the service returns typed `OutputLimit` before publication rather than
claiming truncation. It does not yet provide target pagination.

This is not the target semantic workspace. Its schema mirrors the current syntax tree, spans,
canonical text subtrees, and source files. A `NodeId` contains a revision and dense `u64` index: the
width prevents saturation or aliasing, but identity is not stable across edits. Transactions rewrite
text, and the compiler reparses text. General incomplete semantic states, edit-stable entity/node
identity, broad semantic queries, stable pagination, and direct semantic compilation are not
implemented.

## Prepared programs and process identity

Prepared program descriptors and prepared program values are constructed and retained in-process.
The compiler binds their nonzero prepared identity to already verified SSA and validated bytecode
without reconstructing or revalidating those representations. The prepared identity, not the full
descriptor or prepared program, crosses process bootstrap and outcome provenance boundaries.
Canonical SSA and bytecode identity remain distinct from prepared identity, and optional native
transport specialization does not claim native support when no specialization exists.

## Known gaps

- Text is still source authority; direct compilation from a syntax-independent semantic snapshot is
  absent.
- Semantic Source remains syntax-shaped and its wide dense IDs are not edit-stable semantic IDs.
- Recursive semantic-operation, transaction, runtime structural-value, and specialization paths do
  not all have deep-stack evidence. Some scale paths retain poor complexity and high peak memory.
- The app has one baseline-native-with-VM-fallback product path, and the VM has no JIT knowledge.
  Forced baseline and optimizing helpers and the proof optimizer are deleted. The SSA evaluator is
  an explicit test oracle behind the opt-in `lkjscript-ir/test-oracle` feature. Production
  dependencies leave it disabled; app and compiler development dependencies enable it for
  differential tests. Workspace `--all-features` verification necessarily compiles the oracle, but
  it is not a public runtime engine or app execution choice.
- Daemon, process-cell, scheduler, database, resource-topology, and platform crates remain in the
  workspace even though the local language foundation does not require all of them. The IR and JIT
  crates no longer depend on `lkjscript-resource`; the resource crate remains for other consumers.
- Process/daemon policy is broader and more fragmented than the intended small coarse boundary
  policy. Existing transport and OS/ABI limits remain real boundaries or pending audits, not
  language validity rules.
- Compact native layouts, machine-code offsets, registers/opcodes, OS fields, SQLite fields, and
  host `usize` are private or external representation boundaries. Preferred execution must keep the
  generic VM fallback when a native representation cannot carry an otherwise supported program.
- Final post-reset cold build, startup, execution, memory, and runtime-path comparisons are pending;
  see [`performance.md`](performance.md).
