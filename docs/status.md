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
and no attachments. `Transaction` adds non-generic `CreateProduct`, `CreateEnum`, `CreateFunction`,
and `CreateMain` to rename, replacement, and hole operations. Products, enums, variants, fields,
functions, parameters, locals, bodies, and holes receive opaque stable entities independent of
compiler-dense nominal/layout identities. Public `SemanticTypeRef` inputs use those entities for
nominal types. Invalid names, duplicate declarations/members, ownership-containing aggregate fields,
foreign/stale/wrong-kind type identities, and allocation failure reject without consuming their
reserved stable IDs.

`ExpressionDraft` is a flat non-recursive tree with transaction-local lexical binding handles; its
physical node order is irrelevant. It implements scalar and byte literals, selected canonical
built-in operations, non-generic calls, conditionals, immutable lexical locals, copy-safe loads,
byte-vector moves and shared borrows, product construction/projection, enum construction and variant
tests, and exhaustive non-generic enum matches. Each ordered match arm owns a flat `PatternDraft`;
wildcards, enum variants, fields, and named payload bindings lower through the canonical usefulness,
exhaustiveness, match-plan, ownership, memory, SSA, and VM path. Payload bindings are stable public
immutable-local entities. Compiler-only scrutinee and field-projection locals have an explicit hidden
binding kind and never enter entity/search/constructor results.
Malformed/disconnected/cyclic/reused pattern or expression trees, duplicate handles/names/fields,
foreign/stale/wrong-kind pattern identities, forward or cross-arm binding uses, field coverage/type
failures, empty/nonexhaustive/useless arms, incompatible arm results, and contradictory overlapping
or deletion-owned edits reject. Mutable-local construction, generic calls, non-enum source-free
pattern spaces, and executable placeholders remain absent. Imported mutable-local subtrees can be
removed through ordinary replacement because the lifecycle remap covers their existing HIR form.

The authoritative `SemanticProgram` permits absent `main`, real hole expression leaves, and durable
semantic `Match` nodes linked to canonical match plans. Missing body and typed-hole metadata describe
hole leaves; no prior expression survives introduction. Match arm/body relationships remain directly
queryable, and scrutinee, arm-body, or whole-match nodes use the ordinary targeted edit/hole
operations. Complete-HIR derivation iteratively replaces each semantic match with the existing
canonical `Let`/ordered-`If`/`MatchUnreachable` lowering; memory,
SSA, bytecode, and VM layers never accept an unlowered semantic match. Effects use an explicit
unknown fact while holes remain and are recomputed after every transaction. Shape, lexical scope,
type, usefulness, and exhaustiveness preflight lower once into staged semantic state; canonical
complete-HIR ownership validation decides move/borrow legality and cleanup before publication.
Failure preserves the exact `Arc`, revision, diagnostics, projection, tombstones, and deterministic
future IDs. Replacement and hole introduction now remove local-defining `let`, imported mutable
local, and semantic-match subtrees. One iterative staged compaction prunes unreachable bindings and
match plans, rewrites dense binding/plan references, and rebuilds per-callable places, slots, and
local counts before canonical validation. Removed local and payload identities tombstone; unaffected
entities, nodes, and holes retain identity across private relocation.

`DeleteEntity` supports `main` and ordinary imported or source-free non-builtin functions. Deletion
owns the callable's parameters, locals, payload bindings, nodes, holes, hidden match descendants,
plans, and function-layout participation. A retained final call/reference rejects, while a batch that
removes the dependency and deletes the function publishes once independent of edit order. Deleting
`main` yields the canonical `MissingEntryPoint` incomplete state; recreation uses normal generation
advancement. Direct parameter/local/payload/nominal/member/trait/implementation deletion and public
movement are not implemented.

Completeness blockers distinguish missing entry point, missing body with declaration/hole/type, and
typed hole with hole/type/owner/context. Incomplete snapshots remain fully queryable and projectable.
`compile_snapshot` returns those revision-labelled blockers before deriving HIR or entering memory,
SSA, bytecode, or runtime phases. A complete snapshot derives one source-optional HIR, installs fixed
compiler-owned core context only in that derived compiler value when needed, validates consistency,
and lowers directly. Selected source-free scalar, nominal aggregate, lexical-local, byte-vector
borrow-then-move, and enum-payload-match paths enter source loading and parsing zero times, retain
canonical memory-plan
obligations, compile to validated bytecode, execute in the VM, and clean up on normal and trapped
paths. Canonical memory-plan origins explicitly tag source-backed and source-free cases; current
package locks reflect that non-colliding encoding.

Revision-labelled queries implement deterministic pagination, definitions/references, calls,
structured entity/function/node types, diagnostics, hole context with exact lexical and arm-local
visibility, expected-type-filtered legal constructors, and a structured `MatchView` containing the
scrutinee, ordered arms, arm-body nodes, pattern types/kinds/fields, stable enum/member identities,
and payload-binding entities. Known nominal types carry stable entity IDs; unsupported generic views
are explicit and preserve a nominal ID when available. Copy loads are advertised only for copy-safe
values; affine move/borrow candidates are marked `RequiresOwnershipValidation`, and unsupported
generic enum constructors are omitted. Hole visibility refreshes when declarations are added in a
later revision. Projections render state and blockers before selected
entity/body/type/reference/hole/match sections, including arm and pattern structure, use stable
review-local labels, and require no source attachment. Index root-address resolution performs one map
lookup per semantic node, while enum, variant, enum-field, and match-plan relation indexing uses
private identity maps rather than repeated declaration scans.

The source/path importer privately owns loading, parsing, initial analysis, package validation, and
source provenance capture, then moves all language forms into the same `SemanticProgram`. Fixed
compiler operations/prelude/core traits are excluded from mutable program-entity queries, and HIR
operations carry only canonical catalog operation identity/signature. Imported and source-free
scalar, product, enum, lexical-local, borrow/move, and exhaustive enum-payload-match fixtures agree
on normalized entities and structured types, containment, references/dependencies, node
kinds/types/effects, selected memory-obligation kinds, the main bytecode stream, VM outcomes, traps,
and cleanup.
Attachment changes preserve IDs and projection. Separate ignored locked-release fixtures construct
and compile a 20,000-level nested expression, 20,000 lexical locals, or 20,000 nested semantic enum
matches, then project, execute, and destroy the complete path on a 128 KiB worker stack. Draft and
pattern traversal/lowering, semantic match derivation, semantic clone, indexing/reconciliation,
projection, and canonical block ordering are iterative on these paths. Bytecode structural-local
classification computes nonowned structural values once per function with linear predecessor-edge
propagation rather than rescanning the CFG for every emitted value.

The hidden-body hole overlay, test-only HIR construction surrogate, syntax-shaped editing service,
dense source-node identities, protocol/session schemas, text journal/publication path, CLI routing,
unsupported draft placeholders, and unconsumed development semantic digest are deleted. No wire
replacement exists pending a measured consumer.

## Known gaps

- Text remains a persistent package/import format, but not a compiler or editing authority. The
  concise projection is review/debug output, not a complete source renderer. Declaration creation
  covers non-generic products, enums, functions, and parameterless `main`; expression construction
  covers immutable locals, the selected byte-vector move/borrow vertical, and exhaustive
  non-generic enum payload matches. The source-free pattern surface currently supports wildcard,
  enum-variant, field, and payload-binding patterns over non-generic enum scrutinees; Boolean,
  integer, product, and generic pattern construction remain explicit unsupported edits. Nominal
  declaration fields still reject ownership/reference types, so current source-free payload-match
  ownership evidence uses the strongest supported copy-safe payload geometry. Callable deletion and
  local/payload removal are implemented; nominal/member deletion, explicit public movement,
  mutable-local construction, generic calls, unresolved names, ambiguities, conflicts, and recovery
  states remain.
- There is no persistence, journal, wire service, or collaboration layer for workspace snapshots.
  Add one only after a measured consumer establishes the boundary and resource policy.
- Owned runtime structural values and source-free nested-expression, lexical-local, and nested
  enum-match workspace/compiler paths have 20,000-level release evidence on a 128 KiB worker stack.
  This does not prove every compiler form, type traversal, ownership failure, or general runtime
  throughput
  path stack-safe.
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
