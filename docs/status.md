# Current implementation status

**Status: currently implemented in this checkout.** This is a concise report of checkout behavior,
not a compatibility promise or normative specification. Code, tests, CLI definitions, schemas, and
manifests remain the executable authority.

## User path

The active product is local package compile/run plus an in-process semantic workspace API. The
`.lkjscript` extension is fixed; the current line-oriented bytes are a provisional importer format,
not a textuality or compatibility promise and not semantic authority. `Workspace::empty` creates a
source-free revision with no entities, source/path/hash attachment, entry point, or body. Text and
path entry points import exactly once into the same partial-capable immutable `WorkspaceSnapshot`.

Snapshots own one clone-safe `SemanticProgram`, optional imported diagnostic/presentation origins,
opaque namespace/slot/generation IDs, allocator tombstones, derived indexes, type facts, real hole
nodes, diagnostics, and structured completeness blockers. Source-free and imported complete
revisions derive one ephemeral complete HIR at `compile_snapshot`; they never render or reparse
source. The CLI's required-package compiler entry still verifies the root manifest, lock, selected
module, source identities, target, and capability grants once. Every product compile API delegates
to `compile_snapshot` before HIR memory planning, locked-package target validation, SSA lowering and
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
stacks or equivalent stack-safe designs. The owned `SemanticValue` product/enum tree uses iterative
clone, destruction, fallible and trait equality, symbol rewriting, image conversion, and outcome
validation. Its Debug implementations report bounded root kind/type/field or byte-count summaries;
they do not expand descendants or complete leaf bytes. Safe ownership makes cycles unrepresentable,
so owned-outcome validation performs one checked node/field/byte traversal rather than scanning an
ancestry list. The separate `SemanticDagSnapshot` remains a validated graph boundary.

Ordinary 2,048-level and ignored 20,000-level tests run the complete owned tree boundary on a 128
KiB native stack. A generated VM program also constructs, returns, clones, compares, and destroys a
20,000-level alternating enum/product result; baseline native declines its recursive call graph
before entry and the unchanged program executes once in the VM. A few compiler-recursive paths
remain localized behind [`stack.rs`](../crates/lkjscript-compiler/src/stack.rs); its heap-backed
segment geometry is private tuning, not a language-depth limit.

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

The compiler's public `workspace` module owns one syntax-independent in-memory authority. An
in-process `Workspace` owns its current `Arc<WorkspaceSnapshot>` and stages the exact
identity-allocator state. Public namespace/revision/entity/node/hole IDs are opaque. Tagged private
entity addresses are independent of public ordering, so adding `main` does not renumber an existing
function. Removed slots retain tombstone generations across snapshot cloning and reopening.

`Workspace::empty` reports `Incomplete`, one missing-entry blocker/diagnostic, zero entities/nodes,
and no attachments. `Transaction` adds `CreateFunction` and `CreateMain` to rename, replacement, and
hole operations. Current declaration construction is deliberately scalar and non-generic. Created
function parameters and body holes are returned as stable `EntityCreated` and `HoleIntroduced` diff
entries. Flat drafts implement scalar literals, visible parameter loads, non-generic calls, and
conditionals; placeholder storage/generic/match draft variants are gone.

The authoritative `SemanticProgram` permits absent `main` and real hole expression leaves. Missing
body and typed-hole metadata describe those leaves; no prior expression survives introduction.
Effects use an explicit unknown fact while holes remain and are recomputed after every transaction.
Completion derives and validates HIR, including ownership, before publication. Scalar construction
rejects unsupported owned/affine signature shapes; stale/foreign IDs, invisible bindings, bad arity
or types, cyclic drafts, overlapping subtree edits, duplicate parameters, duplicate/reserved global
creation or function rename, and stale revisions fail before publication.
Failure preserves the exact `Arc`, revision, diagnostics, projection, tombstones, and future IDs.

Completeness blockers distinguish missing entry point, missing body with declaration/hole/type, and
typed hole with hole/type/owner/context. Incomplete snapshots remain fully queryable and projectable.
`compile_snapshot` returns those revision-labelled blockers before deriving HIR or entering memory,
SSA, bytecode, or runtime phases. A complete snapshot derives one source-optional HIR, installs fixed
compiler-owned core context only in that derived compiler value when needed, validates consistency,
and lowers directly. Complete source-free `identity(value: i64) -> i64` plus
`main() -> identity(42)` returns `42` through bytecode/VM with zero parser invocations. Canonical
memory-plan origins explicitly tag source-backed and source-free cases; current package locks reflect
that non-colliding encoding.

Revision-labelled queries implement deterministic pagination, definitions/references, calls,
actual/expected types, diagnostics, hole context, and expected-type-filtered legal constructors.
Hole visibility refreshes when declarations are added in a later revision. Projections render state
and blockers before selected entity/body/type/reference/hole headers, use stable review-local
labels, and require no source attachment. Index root-address resolution performs one map lookup per
semantic node rather than scanning every entity.

The source/path importer privately owns loading, parsing, initial analysis, package validation, and
source provenance capture, then moves all language forms into the same `SemanticProgram`. Fixed
compiler operations/prelude/core traits are excluded from mutable program-entity queries. Imported
and source-free equivalent fixtures agree on user declaration kinds/signatures, expression kinds,
references/calls, diagnostics, compilation, and VM outcome. Attachment changes preserve IDs and
projection. The ignored 20,000-level source-free release fixture performs creation, flat draft
lowering, semantic clone, indexing/reconciliation, projection, complete-HIR derivation, memory/SSA/
bytecode compilation, VM execution, and destruction on a 128 KiB worker stack. Canonical block
ordering is iterative on this path.

The hidden-body hole overlay, test-only HIR construction surrogate, syntax-shaped editing service,
dense source-node identities, protocol/session schemas, text journal/publication path, CLI routing,
unsupported draft placeholders, and unconsumed development semantic digest are deleted. No wire
replacement exists pending a measured consumer.

## Known gaps

- Text remains a persistent package/import format, but not a compiler or editing authority. The
  concise projection is review/debug output, not a complete source renderer. Declaration creation
  currently covers scalar non-generic functions and parameterless scalar `main`; deletion/movement,
  locals, owned/affine body construction, generic/match creation, unresolved names, ambiguities,
  conflicts, and recovery states remain.
- There is no persistence, journal, wire service, or collaboration layer for workspace snapshots.
  Add one only after a measured consumer establishes the boundary and resource policy.
- Owned runtime structural values and the source-free scalar workspace/compiler path now each have
  20,000-level release evidence on a 128 KiB worker stack. This does not prove every compiler form,
  type traversal, ownership failure, or general runtime throughput path stack-safe.
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
