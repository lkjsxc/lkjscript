# Current implementation status

**Status: currently implemented in this checkout.** This is a concise report of checkout behavior,
not a compatibility promise or normative specification. Code, tests, CLI definitions, schemas, and
manifests remain the executable authority.

## User path

The active product is local package compile/run plus an in-process semantic workspace API. Package
files and the provisional line-oriented `.lkjscript` notation remain persistent importer inputs, not
a sibling semantic authority. Text and path entry points import exactly once into an immutable
`WorkspaceSnapshot`; in-process `Workspace` transactions then edit that semantic snapshot directly
without text publication. Snapshots own clone-safe resolved typed HIR,
complete or typed-hole-overlay state, deterministic post-edit development provenance, optional
source attachments, opaque namespace-scoped stable entity/node identities, semantic indexes, type
facts, diagnostics, and hole contexts. The CLI's required-package compiler entry verifies the root
manifest, lock, selected module, source identities, target, and capability grants once inside the
compiler; it does not preverify the package in the application and then reconstruct it during
import. Every product compile API delegates to public `compile_snapshot`, the sole post-import
boundary, before HIR memory planning, locked-package target validation, SSA lowering and
verification, bytecode validation, and execution.

Implemented source behavior includes typed functions and calls, bindings and explicit mutation,
conditionals and loops, nominal products and enums, exhaustive matching, generics and trait-dispatch
subsets, numeric conversions, bytes and byte vectors, lists, typed host resources, errors, and
explicit capabilities. Executable examples and compiler/runtime tests own the exact accepted
surface; this document does not copy their tables.

`lkjscript run` synchronously attempts one supported baseline-native group reachable from `main`.
Eligibility, lowering, installation, or typed pre-entry decline drops the entire native attempt and
executes the unchanged validated program in the VM. Once native entry begins, its result is final
and the VM is not run. Unsupported I/O, generic, recursive-stack, and other native shapes therefore
remain valid through the generic VM route. There is no public engine, threshold,
automatic-transition, or forced-native option.

The broadly tested host is Linux x86-64. Portable Rust may build elsewhere, but another host or
native target is not claimed as tested.

## Phase 2 broader-platform deletion

The workspace now has 11 members and one application binary, `lkjscript`, as reported by Cargo
metadata. The following speculative products and mechanisms are deleted rather than archived or
feature-flagged:

- `lkjscript-runtime`, `lkjscript-resource`, `lkjscript-database`, and
  `lkjscript-linux-host`;
- daemon, process-cell, cell-test-worker, session-broker, resource benchmark, scheduler,
  topology-observation, and app `system` wiring;
- service database, durable control-store, directory-capability, local-control, and database-tenant
  host providers that had no consumer after service deletion;
- process bootstrap/provenance and execution-outcome codecs, resource-plane/runtime-control/
  component contract descriptors, and the global platform revision; and
- target-matrix, platform-revision, and empty configuration placeholders.

The app no longer has dependencies or tests for those surfaces. Contract and package locks are
regenerated from the retained local product. Verified SSA and validated bytecode remain typed
in-process authorities; there is no generic prepared descriptor, cross-representation program
identity, compilation cache, or unconditional native-specialization artifact. A locked snapshot
retains only the package target fact needed to compare its completed HIR memory plan with the lock.

Deleting the service database wrapper did not delete the language SQLite capability. VM host
operations still dispatch SQLite directly through `lkjscript-sys`; stdio, clock, filesystem,
network, terminal, and entropy behavior used by local programs also remains. The retained hello,
Mandelbrot, editor, HTTP, byte, filesystem, hash, SQLite, and comparison smoke paths exercise the
local product rather than a daemon.

## Executable boundary

Native image installation remains a pre-entry, failure-atomic operation. It validates image
integrity and contracts, accounts the object, applies relocations in a private RW mapping, seals the
mapping RX, and publishes installer usage only after success. Dropping an installed image releases
both its mapping and accounted lease.

Collector-free `prepare_invocation` and explicit `prepare_region_invocation` validate entry and
typed arguments, materialize and reserve machine-call and runtime bookkeeping state, and perform
immediate cancellation, deadline, resource, and configured whole-group stack checks. Success
returns a non-cloneable `PreparedInvocation`; `enter(self)` consumes it exactly once across the
unsafe generated ABI call. Pre-entry and entered failures are disjoint types, and there is no VM
retry after entry.

Product metrics report `baseline-native` or `vm-fallback`, one nullable typed native decline with
stage/code/function/detail, whether native entry began, package validation and compiler/native/VM
timings, published installed-artifact size/work counts when available, and explicitly saturating
native runtime/cleanup observations when available. Missing native observations are `null`, not
measured zero. Automatic thresholds, per-function call records, retries, invalidation, runtime
sessions, forced execution, optimizing native lowering, optimization certificates, and the
proof-oriented optimizer are absent. Baseline normalization retains only the independently
verified simple pass sequence.

## Scale and policy result

The ordinary trusted local path has no compiler profiles, cross-phase count budgets, or source/HIR/
SSA count ledgers that decide language validity. Trusted source loading, initial bytecode validation,
and local execution explicitly select unrestricted policy. The former token, child, top-level-form,
nesting, source-unit, HIR-memory, SSA-CFG, SSA-ownership-work, and retained-state-cell admission
quotas are removed.

Source positions and spans, semantic workspace entity/node IDs, HIR and SSA identities, executable
operands and links, structural metadata, and runtime structural identities use wide representations
where they carry user-scale data, with checked conversion before host indexing. Parser/source-tree,
package, type/match, CFG, structural graph, and tested deep-destruction paths use explicit work
stacks or equivalent stack-safe designs. A few compiler-recursive paths remain localized behind
[`stack.rs`](../crates/lkjscript-compiler/src/stack.rs); its heap-backed segment geometry is private
tuning, not a language-depth limit.

Committed generated tests cross former boundaries, including 20,000 nested source expressions,
10,000-block CFG verification, 44,000 owned SSA parameters/places, 300-field products and enums,
1,024 parameters/arguments/locals, table index 65,536, and runtime structural collections above
65,535. Some largest production geometries remain ignored release stress tests.

Borrow ownership builds one iterative liveness plan over complete HIR child traversal. Half-open
expression ranges and sparse per-binding direct-use indexes replace suffix reconstruction; targeted
loan expiration preserves call argument pinning, branches, loops, and mutable-local continuation
without scanning every live reference at every expression.

Bytecode local allocation classifies each SSA value's type, storage, and producer once. Coloring and
emission consume that same per-function metadata, and emitted frame size is checked highest physical
slot plus one rather than SSA value count. Straight-line borrow-call temporaries therefore reuse
three physical locals at every retained matrix size.

Bytecode control-flow validation stores abstract state only at basic-block entries, mutates one
working state through each straight-line block, and merges only at block edges. Failure-cleanup
ranges use a block-local sorted-range cursor, while an incrementally maintained summary makes
cleanup-required checks constant-time between plan-validation boundaries. VM execution uses binary
lookup for unwind and nonlocal cleanup-range movement plus one advancing cursor per active frame for
sequential boundaries. Failed call entry cleans moved arguments in reverse order, and post-step
policy failure unwinds the exact next boundary before ordinary frame cleanup.

The retained 16,385-call fixture completes in release mode on the measured host without a validity
quota. Final medians are 7.303 ms HIR ownership analysis, 7.924 ms bytecode lowering, 6.612 ms VM
execution, and 290.426 ms total compile time; compact protocol, tails, RSS, and exact-stress evidence
are recorded in [`performance.md`](performance.md).

## Retained validation and host boundaries

Artifact validation may select a total-artifact-byte policy. The reusable execution API still
supports explicit limited policy for fuel, VM values/frames, heap/allocation, handles, output, wall
time, and cleanup-report retention; ordinary local compile/run selects the unrestricted form.
These policies control an execution or artifact boundary and do not redefine language validity.
There is currently no untrusted semantic request service or request-byte policy.

Fail-closed validation remains at source importer input, package/manifest/lock/import and compiler
path/symlink entry, typed workspace transactions, capability dispatch, bytecode validation,
relocation and W^X installation, generated native entry, FFI, SQLite,
filesystem/socket/terminal calls, and operating-system errors. Filesystem/network/SQLite language operations use the current process's explicitly
granted capability and direct system mechanism; the deleted service sandbox, process framing,
peer-authorization, database-tenant, durable-store, and platform-observation boundaries are not
claimed as retained.

The execution-outcome wire codec is absent. `SemanticDagSnapshot` and authenticated sealed
structural snapshot import/export remain in memory for VM/JIT behavior and differential tests.
Their memory witness facts are named semantic-snapshot facts rather than process-codec facts.

## Semantic workspace compiler cutover

The compiler's public `workspace` module owns one syntax-independent in-memory
`WorkspaceSnapshot` representation and an in-process `Workspace` owner for its current `Arc` and
identity allocators. Public `WorkspaceNamespace`, `RevisionId`, `EntityId`, `NodeId`, and `HoleId`
values are opaque. Entity/node IDs contain a namespace, logical slot, and tombstone generation rather
than exposing HIR vector positions. Private entity/node address maps reconcile unchanged identities
across rename, replacement, branch movement/reordering, attachment removal, and unrelated edits;
removed descendants become stale and allocator reuse changes generation.

`Transaction` batches `RenameEntity`, `ReplaceExpression`, `IntroduceHole`, `RefineHole`, and
`FillHole` against one base revision. Expression proposals are dense flat child-before-parent graphs
for scalar literals, visible parameter loads, non-generic function calls, and conditionals. The
staging path validates namespace/generation/revision, draft shape, lexical visibility, arity, type,
effect, ownership, and HIR consistency, then publishes one revision. Failure leaves the prior `Arc`,
revision, IDs, and provenance unchanged. Success returns deterministic rename/replacement,
created/deleted-descendant, hole, reference, and call diff entries plus diagnostics and coarse
invalidation domains. Semantic edits remove source attachments and derive development provenance
from the prior semantic digest and diff rather than retaining locked-source claims.

A typed-hole snapshot remains queryable but its private backing HIR cannot be compiled. Hole
introduction prunes and tombstones replaced descendants from public indexes; refinement changes the
goal; fill lowers a checked flat draft directly to HIR, preserves the hole/root `NodeId`, and returns
to complete state after the last hole. `compile_snapshot` returns `CompileSnapshotError::Incomplete`
with stable hole IDs and never installs an executable placeholder.

Revision-labelled queries implement paginated entity listing/search, definition and references,
callers/callees, actual/expected node types, diagnostics, hole context, and legal constructors.
Continuations are stable over one result ordering and reject another namespace, revision, or query.
A deterministic concise projection renders selected entity, body, type, reference, and explicit
`[HOLE]` headers. It uses stable snapshot-local labels, works without source attachments, and does
not create identity. Projection body traversal is iterative and allocation failure is typed.

The source/path importer privately owns loading, parsing, initial analysis, package validation, and
initial provenance capture. Parser-counter tests prove rename, replacement, hole fill, and direct
compile perform no parse, render, or text round trip; complete edited snapshots execute in the VM.
Attachment-change tests prove formatting and source attachment removal do not alter semantic IDs or
projection. An opt-in 20,000-level small-stack edit/drop/projection stress test covers the flat
draft, HIR clone, index, diff, rendering, and destruction path. The former syntax-shaped editing
service, dense source-node identities, protocol/session schemas, text journal/publication path, CLI
routing, and protocol contracts are deleted. No replacement wire service exists pending a measured
consumer.

## Known gaps

- Text remains a persistent package/import format, but not a compiler or editing authority. The
  implemented concise projection is review/debug output, not a complete source renderer. Semantic
  transactions cover only rename, scalar/load/non-generic-call/if replacement, and one typed-hole
  state; declaration creation/deletion/movement, local storage, generic/match creation, unresolved
  names, ambiguities, conflicts, and recovery states remain.
- There is no persistence, journal, wire service, or collaboration layer for workspace snapshots.
  Add one only after a measured consumer establishes the boundary and resource policy.
- Recursive transaction and runtime structural-value paths do not all have deep-stack evidence.
  The measured borrow-call ownership, lowering, frame-size, and VM cleanup-range path is repaired;
  no broader compiler or runtime throughput claim follows from that generated fixture.
- The SSA evaluator is an explicit test oracle behind `lkjscript-ir/test-oracle`; it is not a public
  runtime engine. Workspace `--all-features` verification compiles it for tests.
- Compact native layouts, machine-code offsets, registers/opcodes, OS fields, SQLite fields, and host
  `usize` remain private or external representation boundaries. Native lowering must decline to the
  generic VM before entry when it cannot represent an otherwise supported program.
- Daemon, multi-tenant database, distributed, scheduler, and broader platform products are absent by
  design until the local semantic model and measurements justify them.
- The representative five-sample selected-product baseline in
  [`performance.md`](performance.md) covers process wall time, approximate process-tree RSS,
  compiler/runtime phases, typed native declines, published native code/mapping sizes, exact
  outcomes, cleanup, and host effects. Total allocator counts/bytes, semantic edit/query latency,
  other targets, and application-scale steady-state throughput remain unmeasured.
