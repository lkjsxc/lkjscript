# Agent Handoff

## Purpose

Capture exact Current capability, accepted next contracts, sharp edges, and
verification discipline for autonomous continuation.

## Status

<!-- LKJ-STATUS id=agent-work-state status=current -->
<!-- LKJ-STATUS id=repository-graph-context status=current -->
<!-- LKJ-STATUS id=repository-topology status=current -->
<!-- LKJ-STATUS id=resource-profile-compiler status=current -->
<!-- LKJ-STATUS id=resource-profile-preallocation status=current -->
<!-- LKJ-STATUS id=resource-profile-shared-ledger status=accepted-target -->
<!-- LKJ-STATUS id=semantic-core-target status=accepted-target -->
<!-- LKJ-STATUS id=semantic-resource-plane status=accepted-contract -->
<!-- LKJ-STATUS id=semantic-resource-runtime status=current -->
<!-- LKJ-STATUS id=semantic-session status=current -->
<!-- LKJ-STATUS id=semantic-source status=current -->
<!-- LKJ-STATUS id=typed-holes status=current -->
<!-- LKJ-STATUS id=jit-auto-promotion status=accepted-selection -->
<!-- LKJ-STATUS id=memory-obligations status=current -->
<!-- LKJ-STATUS id=memory-tracing-ratchet status=current -->
<!-- LKJ-STATUS id=memory-plan status=current -->
<!-- LKJ-STATUS id=modules-and-packages status=current -->
<!-- LKJ-STATUS id=deterministic-drop status=accepted-contract -->
<!-- LKJ-STATUS id=generation-safe-resources status=accepted-contract -->
<!-- LKJ-STATUS id=collector-free-value-island status=accepted-contract -->
<!-- LKJ-STATUS id=collector-free-deterministic-memory status=accepted-contract -->
<!-- LKJ-STATUS id=typed-vm-scalars status=current -->

Repository topology and graph/context, bounded task state, exact modules and
packages, canonical Semantic Source and local sessions, explicit capabilities,
generic ADTs and structured control, validated VM, callable baseline JIT, and
forced proof JIT are Current. `lkjscript.memory-obligations` and its inventory
and explain commands are Current descriptive evidence. The machine tracing
ratchet and `memory traced` expose the exact six allowed `HeapObj` families;
builtin storage is removed, and capture-free functions and symbols use artifact IDs.
The measured semantic resource runtime is Current: Linux observation, verified
task graphs, six real policies, owner homes, forced scheduled proof discovery,
scheduled native kernels, shared sealed-image invocation, and retained policy
evidence. Owner compute/kernel managed is the runtime default; proof discovery
remains sequential by default.

Unsafe mechanism ownership is decomposed without aliases. `lkjscript-executable`
owns executable installation, generated entry, native references, and runtime
bridging and is consumed directly by JIT. `lkjscript-linux-host` owns bounded
Linux topology, scheduler, affinity, and binding and is consumed directly by the
app. Residual `lkjscript-sys` retains only host I/O/path/socket/tty/random/poll/
time and SQLite FFI used by the VM. Linux x86-64 remains Current; non-Linux
execution remains untested.

The Linux x86-64 runtime foundation now has real foreground `lkjscriptd`, an
exclusive state-directory lease, durable database-independent journal/snapshot,
kernel-authenticated Unix control, describe/status/stop CLI, and deterministic
Linux, Windows, macOS, session, and container service definitions. Only the
Linux foreground process and Unix transport executed; privileged installation
and non-Linux adapters did not. Trusted in-process VMs now compose app-private
host environments and execute exact arguments, direct stdio, and clock grants;
portable relative paths and directory/database provider contracts exist. A
fixed worker executes bounded Linux process cells with lossless outcomes,
restart, stale rejection, and per-app crash isolation. Durable registry identity,
desired state, authenticated lifecycle/invoke, and daemon restart reconstruction
are Current for process apps. File, terminal, network, SQLite, and stream-resource
VM cutover, session brokers, database attachment, GUI, and SQLite replacement
remain absent.

The authoritative pre-backend HIR memory plan is Current. Safe internal ordinary
regions, sealed shared regions, typed generational pools, and structural owner
homes are Current substrate. They are not selected by HIR or an execution tier.
Exact byte-vector, byte-slice, immutable-bytes, capture-free-function, and
symbol subsets execute without collector interaction. The whole runtime still
traces the remaining structural values; collector-free deterministic memory is
not Current.

## Product Intent

- Build one AI-primary, statically typed, memory-safe language and platform.
- Canonical source uses `.lkjscript`; removed spellings and contracts have no
  aliases.
- Compiler, evaluator, VM, baseline JIT, proof JIT, package, and Semantic Source
  consume one typed semantic authority.
- Keep ordinary source free of lifetime names, retain/release, general `free`,
  raw pointers, and memory-engine switches.
- Preserve exact capabilities, effects, outcomes, budgets, W^X, content
  identities, and proof checking.
- Keep unsafe Rust machine-registered at coherent mechanism boundaries behind
  reviewed safe caller contracts.
- Add no third-party Rust dependency without accepted external review.
- Prefer complete vertical slices and focused conformance over mocks.

## Current Memory Foundation

The stable-index non-moving `GcHeap` still traces explicit legacy-traced
reference values, exact roots, and generated native stack maps. Complete i64
and exact-bit f64 values are inline and never collector allocated. `buf` remains
a traced mutable object. Exact `bytes` uses static or deterministic unique
storage in all four engines. Source `path` remains traced; core unique storage
only establishes its fail-closed migration foundation.

The structural substrate uses nonwrapping domain/root generations, bounded
fallible capacity, typed layout and semantic identities, ledger-only iterative
release, sealed region-level ownership and weak upgrade, typed cyclic pools,
and exact resource-plane homes. See the [focused evidence](../current-state/structural-memory-evidence.md).

`ExecutableProgram` retains the complete content-addressed HIR plan plus a
narrow independently recomputed SSA inventory for direct byte-vector owners,
byte loans, and direct typed resources. Only the opaque memory-verified HIR
wrapper enters SSA lowering. The verified static/dead SSA drop spine carries
closed glue identities, explicit loan-end/drop events, and rejects active-owner
`place-end`. Exact byte-vector, slice, checked little-endian u32, and bytes
operations use bounded unique services in evaluator, VM, and forced native
execution. Static/dead owned-resource exits receive exact implicit glue;
evaluator fake owners and VM bytecode execute it through their core tables,
while explicit close suppresses it. Conditional and exact interned
instruction-failure cleanup now reach evaluator, validated bytecode, VM, forced
baseline, and forced proof execution for each supported owner class. Failed
pre-entry native calls clean transferred arguments separately. Bounded ordered
cleanup failures attach without replacing the primary outcome. VM resources use
reusable generation-bearing guest tokens, exact providers, one execution scope,
reservations, invalidating close, and reverse emergency cleanup. Evaluator fake
providers dispatch the implemented standard-input, terminal, file/directory,
and SQLite lifecycle subset. Broader evaluator operations and native owned
resources remain absent.

## Current Non-Memory Boundaries

- Compiler authority is resolved typed HIR, verified SSA, and validated
  reference bytecode; no backend reinterprets source syntax.
- Imports and packages are exact and content-addressed. The canonical lowercase
  vocabulary remains Accepted Contract while transitional `buf` exists.
- Borrowing is a bounded direct whole-place slice; borrowed returns,
  projections, aggregate partial moves, and resource-bearing aggregates remain
  rejected.
- Forced native claims require synchronous generated entry with zero fallback.
- Collection roots remain required for non-island native functions.
- Execution-tier region/pool selection, cross-call pool loans, sealed compact
  images, no-RC falsification, and family migration remain later work.

## Accepted Next Sequence

1. Complete file, stream, terminal, network, and database VM provider cutover,
   then separate residual SQLite and host I/O mechanisms where evidence supports it.
2. Add authenticated session brokers and native GUI/accessibility applications
   without presenting UI from system service contexts.
3. Integrate database tenants, transactions, cancellation, buffer pages, and
   checkpoints into application-incarnation lifecycle before SQLite migration
   or benchmarks.
4. Continue aggregate/path migration and the six-family tracing ratchet without
   weakening existing memory, resource, forced-tier, or Linux x86-64 evidence.
5. Execute VM/node conformance on native non-Linux and non-x86 hosts; the current
   WASI database probe is not platform acceptance.

This order is an implementation contract, not a Current capability claim.

## Change Discipline

Update authority before public behavior. Keep Current, Accepted Contract,
Accepted Target, Deferred, Rejected, superseded, and historical evidence
distinct. Analysis failure is a compile error, never a tracing fallback.
Generated outputs belong under `target/`; retain compact negative evidence and
remove reproducible temporary outputs.

Use [Verification](verification.md). Record only commands that ran, including
failed attempts and explicit untested gates. Each coherent commit includes
exact `Tested:` and `Not-tested:` trailers and passes the 16×200 topology gate.
