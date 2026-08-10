# Architecture

**Status: current architecture with explicitly labelled target deltas.** Cargo manifests and
`cargo metadata` own workspace membership and dependency edges. This document explains component
responsibility, data flow, ownership, and trust boundaries; it is not a second dependency graph.

## Current compiler and execution flow

```text
Workspace::empty OR verified text/path import
    -> one partial-capable SemanticProgram authority
    -> optional source/presentation provenance beside semantic meaning
    -> immutable WorkspaceSnapshot with stable IDs, tombstones, indexes, and blockers
    -> optional atomic semantic transactions and one-revision Arc publication
    -> compile_snapshot structured completeness gate
    -> one derived source-optional complete HIR and consistency witness
    -> HIR memory planning and captured locked-target validation
    -> typed SSA lowering, verification, and iterative baseline normalization
    -> bytecode lowering and unrestricted trusted validation
    -> one baseline-native group attempt, otherwise validated VM execution
```

Text and package files remain persistent importer inputs, not semantic authority or a post-import
compiler input. The required-package path verifies manifest, lock, selected module, source
identities, and grants once, then moves analyzed declarations/expressions into the same
`SemanticProgram` used by source-free construction. Optional imported `Source` records are retained
as diagnostic provenance outside that program; presentation attachments are independently removable.
An unedited locked import retains only target/capability facts needed at compilation. Any semantic
edit drops locked provenance and source attachments without constructing a replacement content
digest.

A semantic program owns bindings, nominal declarations, canonical match plans, functions, an
optional `main`, and expression trees. A durable semantic `Match` node preserves its scrutinee and
ordered arm bodies while its plan preserves typed patterns, usefulness/exhaustiveness decisions,
field projections, and stable payload bindings. Products, enums, matches, and bindings carry explicit
`Source`, `Semantic`, or `Builtin` origin;
source-free nominal/layout identities derive privately from staged stable entities rather than names,
source hashes, or compiler-dense indexes. A hole is an actual leaf with an explicit unknown effect
bit. Hole records own only kind/goal/type/context metadata; they never retain the removed subtree.
Fixed compiler operations, prelude enums, and core traits are excluded from mutable program-entity
indexes. Expression operations use the canonical operation catalog directly rather than a fabricated
editable operation binding. A source-free complete snapshot installs required fixed core metadata
only in its ephemeral compiler HIR.

Opaque public identities remain separate from dense compiler IDs. Tagged `EntityAddress` variants
distinguish main, binding, product/field, enum/variant/field, trait, and implementation domains.
`NodeAddress` adds root-local preorder only as a private reconstruction coordinate. The immutable
snapshot carries the exact generation/free-list state, so reopening a snapshot cannot resurrect a
tombstoned ID. After staged deletion compacts dense placement, reconciliation receives an explicit,
one-to-one old-public-identity to new-private-address relocation for every surviving entity. A
relocated survivor wins over the prior occupant of its new dense address; duplicate, missing,
wrong-kind, stale, or colliding relocation fails before publication. Nodes outside removed/replaced
subtrees and every structural target root are matched explicitly in one parent/child-ordinal pass.
Descendants rebuilt by a replacement receive new identities even when content coincides; fingerprints
never decide semantic continuity. Removed identities are tombstoned and generation advances before
reuse. Index construction builds
one entity-to-address map and performs one lookup per node, then records containment, references,
calls, dependencies, types, and diagnostics iteratively. Private enum, variant, and enum-field
identity maps avoid repeated declaration scans while indexing aggregate references.

`compile_snapshot` is the sole memory/SSA/bytecode boundary. It rejects blockers before any compiler
phase, derives complete HIR once, injects fixed core context when absent, and validates origins,
signatures, known effects, holes, match-plan/site correspondence, and index shape. During this
ephemeral derivation, an iterative transform replaces semantic matches with the canonical
scrutinee-`Let`, ordered `If`, guarded payload projection/binding, and terminal
`MatchUnreachable` form. Downstream memory/SSA/codegen explicitly reject a surviving semantic match.
Source origins lower to source metadata; semantic origins remain source-free semantic memory origins
without a synthetic `SourceId`. A semantic edit inside an imported function therefore keeps the
function's source origin while its new match, plan, and defining locals honestly retain semantic
origin. Native execution consumes verified SSA directly; the VM consumes
validated bytecode. No render/parse, source identity reconstruction, compilation cache, or stale HIR
copy exists in a snapshot.

Bytecode validation decodes each function once and partitions decoded instructions at entry, jump
targets, and control boundaries. It retains incoming abstract state only at basic-block entries,
clones that state once per block visit, mutates one working state through the straight-line body, and
merges only into successor block entries. A finite monotone worklist handles joins and backedges.
Failure-cleanup ranges are already shape-validated as sorted and nonoverlapping; each block visit
uses a local sweep cursor rather than searching all ranges per instruction. `State` maintains exact
counts of live placed owners, non-parameter borrowed locals, and structural destinations as the
corresponding dense facts change. Full cleanup plans and place coverage are still checked at range
starts against the exact pre-instruction state.

HIR ownership uses one iterative per-function liveness plan. The plan assigns each expression a
half-open traversal range and indexes direct lexical uses sparsely by binding; ownership checking
queries only future uses in the current range and expires affected loans at semantic operations and
joins. It does not materialize a suffix-use set per expression or recurse on user depth. Memory-plan
production walks each immutable-local initializer before publishing that local's place, matching
evaluation order and the resolver's dense `PlaceId` allocation even when a match is nested in a later
scrutinee.

SSA bytecode lowering gathers local type, storage class, and producer kind in one per-function map.
It also computes nonowned structural values once from block parameters and predecessor edges, then
reuses that set for every load/store decision; emission does not rescan the whole CFG per structural
value. Interference coloring and value emission share the derived maps. The slot map remains
authoritative for every emitted operand and cleanup action; `FunctionProto.locals` is its checked
highest physical color plus one, not the number of SSA values.

The validated VM treats sorted failure-cleanup ranges as an execution index. Unwind performs a
binary half-open lookup. Each active frame stores one cursor for the common sequential path; it
advances one adjacent range directly and falls back to binary lookup for forward skips, backedges,
and other nonlocal movement. Entry, ordinary call, and tail-call frame construction initialize the
cursor. Pre-instruction policy failure includes the unentered call plan, failed call setup cleans
moved arguments in reverse order, and post-instruction policy failure uses the exact next boundary.
Tail-call capacity is reserved before caller cleanup or stack truncation.

Trusted compilation has no compiler profile, cross-phase budget ledger, source-shape quota, HIR
memory count quota, or SSA work quota. Checked timings and work totals are observation. User-scale
source, HIR, SSA, bytecode, structural, and runtime identities are generally wide integers or opaque
wide tokens, and conversion to `usize` is checked before indexing. Compact native and machine
representations are specialization boundaries: a pre-entry decline keeps the generic validated VM
route available.

## Current semantic editing flow

```text
Arc<WorkspaceSnapshot> plus base revision
    -> typed declaration/delete/rename/expression/hole batch
    -> namespace/generation/revision check
    -> clone SemanticProgram and identity allocator into staging
    -> deletion conflict, declaration, shape, scope, type, and disjointness preflight
    -> lower flat drafts, apply disjoint replacements, and validate the final deletion dependency closure
    -> prune callable/nominal roots and compact/remap dense products, implementations, bindings,
       plans, places, slots, and private enum-vector addresses once
    -> recompute partial effects and indexes
    -> on completion derive HIR and validate ownership/matches/consistency
    -> reconcile explicit survivors, tombstones, blockers, diagnostics, and semantic diff
    -> publish one new Arc plus allocator state, or publish nothing
```

`CreateProduct` and `CreateEnum` reserve stable declaration/member entities first, derive private
nominal and runtime-layout identities from those entities, validate all names/types/ownership laws,
and publish the complete declaration atomically. One generalized `CreateFunction` accepts zero or
more ordered type-parameter drafts. A declaration-local `DraftTypeParameterId` can occur at arbitrary
depth in the creation-only `DeclarationType`; validation resolves it only within that edit. Staging
allocates stable function, binder, and value-parameter identities in semantic order, maps the local
handles to private binder names, and constructs canonical `Type::Forall { Type::Fn }` and exact
`Function::bounds` facts only when binders exist. It then creates a real result-typed missing-body
hole with unknown effects. The local handles never enter the semantic program, indexes, query,
projection, or diff. `CreateMain` creates parameterless `main` with a real missing-body hole.
Imported and source-free generic binders are reconciled identically as stable `TypeParameter`
entities owned by the function. Published types use `SemanticType`; both recursive type boundaries
use stable nominal references, keep compiler-local names and dense IDs private, and implement their
depth-sensitive operations iteratively. Creation ordering is independent: tagged addresses preserve
a function when main is added later, and hole scope refreshes when a declaration is added after main.
Created declaration, binder, member, parameter, local, and hole IDs are returned through the diff.

`ExpressionDraft` and `PatternDraft` are flat and non-recursive; iterative traversal makes physical
node order irrelevant while validation requires one connected tree with each child used exactly
once. Draft-local immutable and pattern-binding handles have a separate transaction-scoped identity
domain. Implemented expression constructors are scalar and byte literals, selected canonical
built-in operations, exact generic and non-generic calls, `if`, immutable `let`, copy-safe loads,
byte-vector move/shared borrow, product construction/projection, enum construction/variant testing,
and ordered exhaustive enum matches. Generic drafts key exact type arguments by stable binder
entity; importer inference and semantic edits converge at one exact substitution, assignability, and
bound resolver, which derives auto or explicit implementation witnesses.
Implemented patterns are wildcard, non-generic enum variant/field, and
stable named payload binding. Match preparation allocates hidden scrutinee/projection places and
public payload bindings, establishes one lexical scope per arm, invokes the canonical usefulness and
plan builder, and publishes only if complete-HIR ownership and consistency validation succeed.
Mutable-local construction, generic patterns, unresolved binder forwarding, ownership/reference
generic instantiation, and non-enum source-free pattern spaces remain explicit unsupported edits; no
executable fallback exists. After all disjoint structural
edits and final-state dependency validation, one fallible iterative compaction pass performs callable
and nominal removals
and removes unreachable bindings and match plans. It assigns retained `BindingId` and `MatchPlanId`
values densely once,
rewrites function/main headers, global layout, expression references and definitions, recursive
match locals/plans, and rebuilds each callable's `PlaceId`, local slot, parameter-place vector, and
`local_count`. Function-vector position remains private and memory/SSA function IDs are derived later.
No second mutable IR, sparse dead-binding history, persistence layer, or incremental framework was
added; `SemanticProgram` remains the sole mutable authority and complete HIR remains one-way derived.

`DeleteEntity` first collects callable, product, and enum deletion intent for the whole batch.
`main`, ordinary functions, and user nominals are pruned only after structural replacements and one
iterative final-state dependency traversal. That traversal covers stored signatures and fields,
expression and hole types, aggregate operations, match patterns/plans, generic substitutions and
witnesses, and callable references. Removing a body dependency or deleting every independent owner
in the same batch is therefore order-independent; a surviving dependent rejects with stable public
identity and kind/name context.

Callable containment owns type parameters, value parameters, locals, payloads, nodes, holes, hidden
match storage, plans, and layout participation. Product containment owns fields and the narrow target-
implementation lifecycle; enum containment owns type parameters, variants, and fields. Other
dependents never cascade. One concrete
compaction result rewrites `ProductId`, `ImplId`, `BindingId`, `MatchPlanId`, local places and slots,
and every expression/pattern/witness occurrence. Enum vector positions relocate privately while
stable enum, variant, field, and runtime-layout IDs remain unchanged. Reconciliation forces old
public identities onto every surviving product/member, enum/member, implementation, binding, and
structurally surviving node address. `Type::Product(String)` remains the active HIR type identity:
complete and partial indexes validate globally unique product names, product rename is unsupported,
same-batch same-name recreation cannot rebind a survivor, and immutable snapshots own separate HIR
vectors. The existing stable product and field identities remain unchanged rather than adding a
fourth product identity. Direct member, trait, and implementation deletion remains unsupported;
delete-and-same-name-create is not implicit replacement.

Introducing a typed hole physically replaces and drops its subtree, including owned local/match
facts; filling preserves the hole/root ID. Missing-entry, missing-body, and typed-hole blockers are
structured and projected. Incomplete snapshots retain normal indexes and deterministic diagnostics
but no compiler HIR.

Queries are revision-labelled and deterministically paginated for entities/search, references,
calls, diagnostics, and legal constructors. Definition, structured entity/function/node type, exact
generic signature/call instantiation, hole context with exact lexical/arm visibility, and structured
match inspection are direct identity queries. Signature views expose stable binders and structured
bounds; call views expose canonical substitutions, instantiated parameters/results, derived
witnesses, and named effect bits. `MatchView` reports a stable match node, scrutinee node, result
type, exhaustiveness, ordered arm/body nodes, and deterministic typed pattern nodes/fields referring
to stable variant/field/binding entities. Parallel per-node plan facts and direct-child indexes make match
lookup independent of expression-root size. Nominal and generic type views use stable semantic
identities. Legal-constructor results
distinguish established constructors from move/borrow candidates that still require canonical
ownership validation, and do not expose hidden match temporaries or advertise generic enum
construction. A continuation is bound to its namespace, revision, and query. Semantic diffs report
rename, replacement, created/deleted descendants and pattern bindings, hole transitions, and
reference/call rewiring; invalidation currently reports coarse truthful domains rather than
incremental cache work.

Selected entity, body, type, reference, call, hole, and match sections have one concise
deterministic projection. It traverses body, pattern, and public type structure iteratively, reports
allocation failure, marks holes as `[HOLE]`, renders ordered arms, stable enum/field/binding
identities, exact generic facts, and named effects, and requires no source attachments. Projection
labels are review/debug spellings, never identity input.
The former syntax-shaped service, dense source-node IDs, stdio/session schemas, text publication and
journal machinery, CLI routes, and protocol contracts are deleted. No wire replacement exists
without a measured consumer.

## Current execution and local host flow

`lkjscript run` selects `ExecutionPolicy::Unrestricted` and exposes no engine selection. The app
lowers the complete eligible group reachable from `main`, installs one baseline image, and prepares
one invocation before source effects. Eligibility, lowering, installation, setup, or typed
`PreEntryError` decline destroys the complete native attempt before giving the unchanged bytecode,
inputs, and policy to a fresh VM invocation.

Executable installation validates image identity and contracts, relocates inside a private RW
mapping, seals it RX, and publishes accounting only on success. `PreparedInvocation::enter`
consumes the affine preparation and crosses the generated ABI boundary once. Entered errors and
execution outcomes are post-commit and never run or rerun the VM.

The VM handles the complete generic operation set. Stdio and clock use the retained host traits.
Filesystem, network, terminal, entropy, and SQLite operations dispatch through the VM's typed
resource table and `lkjscript-sys`; SQLite remains a direct language capability. The former service
database wrapper, directory provider, durable store, database tenant provider, local-control peer,
Linux observation, scheduler, topology, and process-cell layers are absent.

Runtime storage is collector-free for implemented value families. Unique storage, regions,
segmented lists, semantic DAGs, returned snapshots, and host resources stage allocation and
identity-map publication. Opaque handles resolve through runtime-owned wide maps rather than packed
index arithmetic. Cleanup continues even when diagnostic retention is exhausted.

A returned `SemanticValue` is a key-free owned tree, not a graph: aggregate edges move values into
private vector storage and there is no shared owner, reference edge, or unsafe constructor through
which a child can point to an ancestor. Clone and destruction use explicit work vectors. Fallible
runtime equality and infallible Rust trait equality share one iterative comparison algorithm; the
fallible route reports work allocation failure, while the trait follows ordinary Rust allocation
behavior rather than turning failure into inequality. Symbol canonicalization pre-reserves from
validated node metrics before iteratively rewriting leaves. Debug output is a bounded root summary.

`OwnedValue::from_structural` validates kind/payload agreement, UTF-8 strings, paths, checked
node/field/byte accounting, and work allocation before publishing the box. Because cycles are
unrepresentable in this ownership tree, validation visits every node and field once and carries no
ancestor set. `SemanticDagSnapshot` is a separate reverse-topological graph representation and keeps
its graph-reference, reachability, and cycle validation. Runtime-local `StructuralImage` remains the
flat typed owner/handle storage used by VM and native structural services; it is not another semantic
authority.

## In-process compiler authority and snapshots

Verified SSA and validated bytecode are retained directly in `ExecutableProgram` as typed in-process
values. Neither crosses a process, persistence, artifact-load, or compilation-cache boundary, so the
compiler does not canonically traverse and hash both representations, construct a generic prepared
descriptor, or bind a synthetic shared identity back into them. Baseline native receives the
retained verified SSA and computes eligibility and target-specific machine facts only while building
an actual attempt. VM execution receives the retained validated bytecode unchanged.

Package validation remains separate from this deletion. The graph builder returns the root manifest
from the same read that produced the current lock candidate; the compiler compares that candidate
with the decoded lock, parses grants only from the bound manifest, carries the same lock and grants
through source-closure checking, and captures its target record without rereading mutable path state.
Required-package compilation fails when no
root package exists. After HIR memory planning, `compile_snapshot` compares generated target memory
and witness facts with that captured record and rejects any validated-bytecode capability omitted by
the verified manifest; a development snapshot needs no equivalent package check. Executable-image
content identity, relocation validation, contract checks, private RW construction, RX sealing, and
failure-atomic installation remain at the real native artifact boundary.

Execution outcomes are ordinary in-memory Rust values. The process outcome wire codec is deleted.
`SemanticDagSnapshot` remains a validated in-memory graph, and `SealedSemanticDagRuntime` retains
authenticated snapshot import/export used by VM/JIT and differential tests. Memory-plan and witness
facts call this capability `semantic_snapshot`; it is not a process transport promise.

## Component ownership

Cargo metadata currently reports 11 workspace members and one app binary. Conceptually:

- `lkjscript-contracts` owns retained language, IR, memory, package, native, and runtime-call
  descriptors and identities;
- `lkjscript-core` owns values, execution policy/outcomes, validated bytecode, memory witnesses,
  structural storage, semantic snapshots, and resource tables;
- `lkjscript-compiler` owns the public immutable semantic workspace snapshot and direct snapshot
  compiler boundary, plus the private source/package importer, HIR, memory planning, lowering,
  package locks, and concise semantic projection;
- `lkjscript-ir` owns SSA, verification, normalization, and the opt-in test oracle;
- `lkjscript-vm` owns generic validated-bytecode execution and typed host-operation dispatch;
- `lkjscript-native`, `lkjscript-jit`, and `lkjscript-executable` own baseline-native lowering,
  generated code, relocation, W^X mapping, and invocation;
- `lkjscript-host` owns only retained stdio, clock, cancellation, and logging interfaces;
- `lkjscript-sys` owns direct OS/FFI mechanisms for files, sockets, time, terminal, entropy, and
  SQLite; and
- `lkjscript-app` owns the sole `lkjscript` CLI and local integration wiring.

The deleted runtime/resource/database/Linux-host crates and five secondary binaries have no Cargo
edge or shadow implementation.

## Retained trust boundaries

Fail-closed validation remains at:

- text importer and typed semantic transaction input;
- package manifest/lock/import resolution and compiler path/symlink handling;
- capability grants and typed host-operation dispatch;
- bytecode validation and malformed operand/index handling;
- relocation, W^X executable installation, generated entry, and native ABI/stack preflight; and
- filesystem, socket, terminal, SQLite, FFI, and operating-system calls.

Within one synchronous compiler pipeline, typed verified wrappers and Rust ownership carry
validated authority. The compiler does not serialize, hash, or independently reverify those values
to manufacture another in-process authority token.

`lkjscript-executable` is the narrow unsafe executable-memory and generated-entry mechanism.
`lkjscript-sys` is the direct operating-system/SQLite FFI mechanism. The deleted process framing,
peer authorization, service persistence, database tenancy, directory-sandbox provider, Linux host
observation, scheduler, and resource-plane boundaries are intentionally not claimed as retained.
Local filesystem/network capabilities use the current process's OS authority after language
capability checking; they are not a replacement service sandbox.

## Target delta

**Current fact:** source-free genesis and text import share one revision-labelled `SemanticProgram`
authority. Missing entry/body and real typed-hole nodes; non-generic product and enum creation;
generic and non-generic function plus entry creation; immutable lexical locals; selected byte-vector
move/borrow and canonical operations; aggregate construction/observation; exhaustive non-generic enum
payload matches with stable arm-local bindings; exact calls to imported or source-free generic
functions with stable binders, structured types, shared resolution, and derived witnesses; atomic
batch edits; tombstone-stable identities; structured
stable nominal, generic, and match views; deterministic queries/projections/diffs; one canonical
complete-HIR match derivation; and direct execution are implemented. Source-loading/parser/compiler-
phase counters, imported scalar, nominal, local, ownership, match, generic-declaration, and generic-
call convergence, malformed/atomic retry tests, exact per-node index work, and 20,000-level nested-
expression, local, semantic-match, published-type, and declaration-type small-stack release execution
protect the selected vertical.
Formatting-only attachment changes preserve IDs and projection.

**Target, not implemented:** later workspace expansion adds direct nominal-member mutation, one
concrete public semantic movement operation only when ownership/order semantics are defined,
mutable-local construction, generic patterns, Boolean/integer/product pattern construction, generic
ownership/reference instantiation, unresolved
references, ambiguities, conflicts, recovery nodes, richer declaration kinds, and finer
analysis contexts without adding another mutable semantic AST. Persistence,
collaboration, a measured wire consumer, incremental recomputation, daemon, database service,
scheduler, and broader platform work wait for evidence after real use of the local semantic model.
