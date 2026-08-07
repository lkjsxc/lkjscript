# Architecture

**Status: current architecture with explicitly labelled target deltas.** Cargo manifests and
`cargo metadata` own workspace membership and dependency edges. This document explains component
responsibility, data flow, ownership, and trust boundaries; it is not a second crate graph or a
statement that target work is implemented.

## Current compiler and execution flow

```text
package manifest, lock, and line-oriented .lkjscript files
    -> checked package/source loading and exact import resolution
    -> source tree
    -> resolved typed HIR
    -> type, effect, ownership, and HIR memory-plan analysis
    -> typed SSA lowering, verification, and normalization
    -> bytecode lowering and unrestricted trusted validation
    -> in-process prepared descriptor and bound prepared identity
    -> auto VM/native execution (or an explicit diagnostic engine)
```

Text and package files are the current authority. The compiler does not accept a
syntax-independent semantic snapshot. The compiled program retains typed HIR memory authority,
verified SSA, validated bytecode, and links consumed by execution. It no longer constructs the
former unconsumed parallel SSA memory-obligation inventory.

Trusted compilation has no compiler profile, cross-phase budget ledger, source-shape quota, HIR
memory count quota, or SSA work/count quota. Checked phase timings and work totals are observation.
Source and package reads use checked arithmetic and fallible growth. Package traversal, source-tree
operations, type and CFG traversal, match usefulness, structural graph operations, and tested deep
destruction paths use explicit worklists or equivalent stack-safe mechanisms. The compiler-local
[`stack.rs`](../crates/lkjscript-compiler/src/stack.rs) is the single wrapper for recursive compiler
paths not yet converted to explicit continuation stacks; its heap-backed segment geometry is private
tuning rather than a language depth boundary.

User-scale source, HIR, SSA, bytecode, structural, and runtime identities are generally `u64` or
opaque `u64` tokens. Dense vectors remain internal storage, and conversion to `usize` is checked
before access. Insertion-order vectors own canonical order; hash maps accelerate lookup but do not
define serialization or diagnostics. Compact native and machine representations are specialization
boundaries: automatic execution retains validated VM bytecode when native preflight declines.

## Current semantic editing flow

```text
JSON or stdio Semantic Source request
    -> strict schema and coarse boundary policy
    -> load and flatten syntax-shaped source snapshot
    -> source-derived entity/node/hole query, or staged text transaction
    -> source and HIR validation
    -> atomic source-file publication or typed failure
    -> later text compilation through the ordinary compiler flow
```

The service is useful bootstrap infrastructure, not semantic authority. Node identity is a revision
plus dense `u64` index. Transactions have base revisions and file/entity/node preconditions, but
successful edits publish text files. Snapshot records include paths, spans, canonical subtrees, and
source fingerprints. This shape does not provide edit-stable semantic identity or direct compilation.

## Current execution and resource ownership

`lkjscript run` selects `ExecutionPolicy::Unrestricted`. The default `auto` engine always has a
validated heap-framed VM route and may install native code for eligible call groups. Executable
installation is a failure-atomic pre-entry phase: image integrity, contracts, relocation, accounting,
and the RW-to-RX transition complete before an installed mapping is published. Invocation then has
two typed phases. `prepare_invocation` validates the execution domain, entry and arguments, prepares
the closed machine ABI arguments, reserves invocation bookkeeping, and checks immediate policy,
cancellation, deadline, and whole-group native-stack requirements. It returns either an affine
`PreparedInvocation` or `PreEntryError`; only that error class is safe for automatic VM fallback.
`PreparedInvocation::enter` consumes the preparation and crosses the unsafe generated ABI boundary
exactly once. Every later malformed-state or native-stack failure is an
`EnteredInvocationError`, while traps, exits, deadlines, resource exhaustion, and host failure are
entered outcomes. There is no post-entry retry classification or boolean heuristic. Forced
diagnostic engines either enter their requested generated path or report a failure.

Untrusted or isolated process callers construct `ExecutionPolicy::Limited` explicitly. Limited
execution owns coarse fuel, values/frames, heap/allocation, handle, output, deadline, and cleanup
reporting resources. Semantic Source, artifact loading, process framing, and outcome decoding own
separate boundary-local byte policy. These policies govern requests; they are not compiler or
language validity.

Runtime storage uses collector-free ownership for implemented value families. Unique storage,
regions, segmented lists, semantic DAGs, returned snapshots, and host resources stage allocation and
identity-map publication. Opaque public handles resolve through runtime-owned wide maps; they are not
arithmetically decoded packed indexes. Cleanup continues even when diagnostic retention is
exhausted.

## Current prepared and process boundary

The compiler constructs a `PreparedProgramDescriptor` and `PreparedProgram` in-process from package
provenance, memory authority, verified SSA identity, optional native-specialization identity,
validated bytecode identity, and required contract identities. It computes one nonzero
`PreparedProgramIdentity` and privately binds that identity to the already verified/validated
wrappers without repeating verification.

Only the prepared identity and other explicit provenance identities cross process bootstrap and
outcome frames. The descriptor, prepared program, HIR, verified SSA object, bytecode under
construction, and JIT plans remain in-process. Process framing revalidates message shape, identity,
provenance, lengths, and limited execution policy.

## Component ownership

This is a conceptual grouping, not an exhaustive dependency graph:

- core language execution data and policies live in `lkjscript-core`;
- source/package analysis, HIR, memory planning, lowering, and Semantic Source live in
  `lkjscript-compiler`;
- SSA, verification, normalization, and the reference evaluator live in `lkjscript-ir`;
- generic validated-bytecode execution and runtime values live in `lkjscript-vm`;
- native planning, tiering, code generation, and executable installation span `lkjscript-native`,
  `lkjscript-jit`, and the `lkjscript-executable` mechanism crate;
- CLI and integration wiring live in `lkjscript-app`;
- host, resource, database, daemon/process, scheduler, and platform mechanisms remain split across
  the corresponding host/resource/database/runtime/system crates; and
- boundary schemas and content/prepared identities live in `lkjscript-contracts`.

The number and edges of these crates must be queried from Cargo. Several platform and runtime
boundaries remain candidates for deletion or consolidation after the production path is selected.

## Trust boundaries

Current fail-closed validation remains at:

- source, semantic-operation, package, manifest/lock, import, path, and symlink entry;
- capability grants and host-provider dispatch;
- untrusted bytecode, persisted stores, process frames, provenance, and outcome codecs;
- relocation, W^X executable-memory installation, generated native entry, and native stack/ABI
  preflight; and
- FFI, SQLite, filesystem, socket, terminal, and operating-system calls.

Within one synchronous trusted compiler pipeline, opaque verified wrappers and Rust ownership carry
validated authority. Prepared-identity binding does not serialize, decode, or independently verify
the same in-process program again.

Workspace crates forbid unsafe code by default. `lkjscript-sys`, `lkjscript-linux-host`, and
`lkjscript-executable` are named mechanism boundaries for FFI, Linux observation, executable
mapping/relocation, and generated entry. Safe callers depend on their checked APIs.

## Target deltas

**Target, not implemented:** replace the current source/editing/compiler flow with:

```text
text or structured import
    -> typed semantic workspace transaction
    -> immutable revision-labelled semantic snapshot
    -> direct typed-core and executable lowering without rendering/reparsing
    -> one measured production execution path
```

The target requires edit-stable logical identities, first-class incomplete semantic states, typed
atomic batch edits, deterministic paginated semantic queries, semantic and text projections from one
snapshot, and direct compilation tests. It begins in memory; persistence or distributed
collaboration waits for measured need.

**Accepted target, boundary implemented but runtime cutover incomplete:** synchronously prepare one
baseline-native reachable group before effects; enter it when preparation succeeds, otherwise run
the retained VM; never retry after native entry. The typed executable boundary above establishes the
required decision point. Current automatic tiering, forced baseline/optimizing CLI modes, optimizer,
and repeated transition policy still exist and must be deleted or demoted during the remaining
cutover. Afterward consolidate platform/process crates and one coarse untrusted host policy without
weakening genuine path, process, artifact, executable-memory, FFI, or database boundaries.
