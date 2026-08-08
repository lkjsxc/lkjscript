# Architecture

**Status: current architecture with explicitly labelled target deltas.** Cargo manifests and
`cargo metadata` own workspace membership and dependency edges. This document explains component
responsibility, data flow, ownership, and trust boundaries; it is not a second dependency graph.

## Current compiler and execution flow

```text
package manifest, lock, and line-oriented .lkjscript files
    -> one compiler-owned required-package verification
    -> private checked text/path importer using the same verified lock and grants
    -> source tree and type/effect/ownership analysis
    -> immutable WorkspaceSnapshot owning typed HIR and complete/typed-hole state
    -> optional in-process Workspace transaction and one-revision Arc publication
    -> compile_snapshot typed completeness and consistency validation
    -> HIR memory planning and captured locked-target validation
    -> typed SSA lowering, verification, and simple normalization
    -> bytecode lowering and unrestricted trusted validation
    -> one baseline-native group attempt, otherwise validated VM execution
```

Text and package files remain persistent importer inputs, not semantic authority and not a post-HIR
compiler input. The product's required-package path verifies the root manifest, decoded lock,
selected module, and typed capability grants once; source import, locked-source checking, provenance
capture, target checking, and capability checking consume that same verified value. `compile_snapshot`
is public and is the sole boundary into memory planning, SSA, and bytecode. All text/path compile
APIs import and delegate. The snapshot has private fields and
owns `Arc`-shared typed HIR, complete or typed-hole-overlay state, immutable import provenance or
deterministic post-edit development provenance, optional presentation/source attachments, and
deterministic semantic indexes. An unedited locked import retains the exact target record needed to
validate its completed memory plan without reconstructing provenance from the file system. A
semantic edit removes source attachments and derives development identity from the prior semantic
digest and published diff, so an edited program never falsely retains locked-source provenance.

Opaque public workspace identities are separate from HIR dense IDs. Entity and node IDs carry a
workspace namespace, logical slot, and generation; a revision carries the same namespace. Import assigns them deterministically inside a fresh namespace, and cross-workspace use fails before
lookup. `Workspace` owns generation-aware allocators; private `EntityAddress` and `NodeAddress` maps
reconcile preserved roots and unchanged descendants while removed nodes are tombstoned. Iterative
index construction records containment, references, calls, dependencies, actual/expected type
headers, and diagnostics. Compilation revalidates complete HIR references, signatures, origins, and
index shape before lowering. Native
execution consumes verified SSA directly; the VM consumes validated bytecode. The deleted optimizer
pipeline, proof metadata, automatic transition, redundant SSA memory inventory, generic
cross-representation identity, and native-specialized SSA copy are not constructed.

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
joins. It does not materialize a suffix-use set per expression or recurse on user depth.

SSA bytecode lowering gathers local type, storage class, and producer kind in one per-function map.
Interference coloring and value emission share that derived map rather than independently scanning
all blocks. The slot map remains authoritative for every emitted operand and cleanup action;
`FunctionProto.locals` is its checked highest physical color plus one, not the number of SSA values.

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
    -> typed batch of rename, flat expression, and typed-hole edits
    -> namespace/generation/revision and complete draft preflight
    -> direct staged HIR mutation
    -> effect, ownership, match-plan, HIR, and index validation
    -> stable-ID reconciliation and semantic diff
    -> one new Arc<WorkspaceSnapshot>, or no publication
```

The flat `ExpressionDraft` representation is child-before-parent and non-recursive. The implemented
constructors are i64/f64/bool/unit literals, visible parameter loads, non-generic function calls,
and `if`. Local storage, generic calls, and match creation fail explicitly as unsupported. Typed
holes retain expected type, goal, owning context, visible entities, and a private replaced address.
Incomplete snapshots retain ordinary query indexes and deterministic diagnostics but expose no
executable placeholder; `compile_snapshot` returns stable hole IDs.

Queries are revision-labelled and deterministically paginated for entities/search, references,
calls, diagnostics, and legal constructors. Definition, node type, and hole context are direct
identity queries. A continuation is bound to its namespace, revision, and query. Semantic diffs
report rename, replacement, created/deleted descendants, hole transitions, and reference/call
rewiring; invalidation currently reports coarse truthful domains rather than incremental cache work.

Selected entity, body, type, reference, and hole headers have one concise deterministic projection.
It traverses body ownership iteratively, reports allocation failure, marks holes as `[HOLE]`, and
requires no source attachments. Projection labels are review/debug spellings, never identity input.
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

**Current fact:** complete text imports and in-process semantic edits share one revision-labelled
snapshot authority. Stable identity, one typed-hole incomplete state, atomic batch edits,
deterministic paginated queries and concise projections, semantic diffs, and direct executable
lowering are implemented for the selected vertical. Directness tests prove editing and compilation
do not invoke the parser; attachment-free and edited complete snapshots execute through the VM.
Formatting-only attachment changes preserve IDs and projection.

**Target, not implemented:** later workspace expansion adds declaration and node
movement/creation/deletion, local storage, generics, matches, unresolved references, ambiguities,
conflicts, recovery nodes, and finer analysis contexts without adding another semantic AST.
Persistence, collaboration, a measured wire consumer, incremental recomputation, daemon, database
service, scheduler, and broader platform work wait for evidence after the local semantic model.
