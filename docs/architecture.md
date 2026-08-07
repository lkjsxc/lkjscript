# Architecture

**Status: current architecture with explicitly labelled target deltas.** Cargo manifests and
`cargo metadata` own workspace membership and dependency edges. This document explains component
responsibility, data flow, ownership, and trust boundaries; it is not a second dependency graph.

## Current compiler and execution flow

```text
package manifest, lock, and line-oriented .lkjscript files
    -> checked package/source loading and exact import resolution
    -> source tree
    -> resolved typed HIR
    -> type, effect, ownership, and HIR memory-plan analysis
    -> typed SSA lowering, verification, and simple normalization
    -> bytecode lowering and unrestricted trusted validation
    -> in-process prepared descriptor and bound prepared identity
    -> one baseline-native group attempt, otherwise validated VM execution
```

Text and package files remain current authority. The compiler does not accept a syntax-independent
semantic snapshot. A compiled program retains HIR memory authority, verified SSA, validated
bytecode, and one prepared identity. Native execution consumes verified SSA directly; the VM
consumes validated bytecode. The deleted optimizer pipeline, proof metadata, automatic transition,
and redundant SSA memory inventory are not constructed.

Trusted compilation has no compiler profile, cross-phase budget ledger, source-shape quota, HIR
memory count quota, or SSA work quota. Checked timings and work totals are observation. User-scale
source, HIR, SSA, bytecode, structural, and runtime identities are generally wide integers or opaque
wide tokens, and conversion to `usize` is checked before indexing. Compact native and machine
representations are specialization boundaries: a pre-entry decline keeps the generic validated VM
route available.

## Current semantic editing flow

```text
JSON or stdio Semantic Source request
    -> strict schema and coarse request policy
    -> syntax-shaped source snapshot
    -> entity/node/hole query, or staged text transaction
    -> source and HIR validation
    -> atomic source-file publication or typed failure
    -> later text compilation through the ordinary compiler flow
```

This service is bootstrap infrastructure, not semantic authority. Node identity is a revision plus
dense `u64` index. Transactions have base revisions and typed preconditions, but successful edits
publish text files. Snapshot records include paths, spans, canonical subtrees, and source
fingerprints. Direct semantic compilation and edit-stable identity are not implemented.

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

## Prepared identity and snapshots

The compiler constructs `PreparedProgramDescriptor` and `PreparedProgram` in-process from:

- locked or explicit development package content, root, entry, and memory closure;
- verified HIR memory-plan and witness-closure identities;
- semantic SSA and optional native-specialization SSA identities;
- validated bytecode identity; and
- prepared-program, runtime-call, native-layout, verified-SSA, and bytecode contract digests.

One nonzero `PreparedProgramIdentity` is privately bound to the already verified SSA and validated
bytecode without serialization or revalidation. The identity remains in-process and may participate
in compilation cache keys. There is no process provenance, bootstrap frame, global platform
revision, runtime-control digest, or process-outcome-codec digest in the descriptor.

Execution outcomes are ordinary in-memory Rust values. The process outcome wire codec is deleted.
`SemanticDagSnapshot` remains a validated in-memory graph, and `SealedSemanticDagRuntime` retains
authenticated snapshot import/export used by VM/JIT and differential tests. Memory-plan and witness
facts call this capability `semantic_snapshot`; it is not a process transport promise.

## Component ownership

Cargo metadata currently reports 11 workspace members and one app binary. Conceptually:

- `lkjscript-contracts` owns retained language, IR, memory, package, prepared, and semantic protocol
  descriptors and content identities;
- `lkjscript-core` owns values, execution policy/outcomes, validated bytecode, memory witnesses,
  structural storage, semantic snapshots, and resource tables;
- `lkjscript-compiler` owns source/package analysis, HIR, memory planning, lowering, package locks,
  and Semantic Source;
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

- source and Semantic Source input;
- package manifest/lock/import resolution and compiler path/symlink handling;
- capability grants and typed host-operation dispatch;
- bytecode validation and malformed operand/index handling;
- relocation, W^X executable installation, generated entry, and native ABI/stack preflight; and
- filesystem, socket, terminal, SQLite, FFI, and operating-system calls.

Within one synchronous compiler pipeline, typed verified wrappers and Rust ownership carry
validated authority. Prepared-identity binding does not serialize or independently verify the same
program again.

`lkjscript-executable` is the narrow unsafe executable-memory and generated-entry mechanism.
`lkjscript-sys` is the direct operating-system/SQLite FFI mechanism. The deleted process framing,
peer authorization, service persistence, database tenancy, directory-sandbox provider, Linux host
observation, scheduler, and resource-plane boundaries are intentionally not claimed as retained.
Local filesystem/network capabilities use the current process's OS authority after language
capability checking; they are not a replacement service sandbox.

## Target delta

**Target, not implemented:** replace the current source/editing/compiler flow with:

```text
text or structured import
    -> typed semantic workspace transaction
    -> immutable revision-labelled semantic snapshot
    -> direct typed-core and executable lowering without rendering/reparsing
    -> the selected baseline-native-with-VM-fallback product path
```

The target requires edit-stable logical identities, first-class incomplete semantic states, typed
atomic batch edits, deterministic paginated semantic queries, semantic and text projections from one
snapshot, and direct compilation tests. It begins in memory. Persistence, collaboration, daemon,
database service, scheduler, and broader platform work wait for measured need after the local
semantic model works.
