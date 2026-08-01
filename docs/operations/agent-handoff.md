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
<!-- LKJ-STATUS id=memory-tracing-ratchet status=superseded -->
<!-- LKJ-STATUS id=no-tracing-runtime status=current -->
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
and explain commands are Current descriptive evidence. The migration tracing
ratchet and `memory traced` command are removed. The unconditional
`LKJ-RUNTIME-NO-TRACING-COLLECTOR` gate rejects collector directories, object
families, services, liveness maps, barriers, configuration, and metrics.
Capture-free functions and symbols use artifact IDs.
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
portable relative paths and directory providers exist. One daemon-owned ordered
application database now supplies tenant- and incarnation-bound providers; its
transactions abort on lifecycle release. A fixed worker executes bounded Linux process cells with lossless outcomes,
restart, stale rejection, and per-app crash isolation. Durable registry identity,
desired state, authenticated lifecycle/invoke, and daemon restart reconstruction
are Current for process apps. Authenticated ephemeral session-broker presence is
Current on Linux. File, terminal, network, SQLite, stream-resource, database
process/VM operations, interactive cells, GUI, and SQLite replacement remain absent.

The authoritative pre-backend HIR memory plan is Current. A narrow ordinary-region
route is selected for acyclic products closed over copy lists and scalar leaves; broader regions,
sealed sharing, pools, and owner homes remain substrate only. Exact bytes,
functions, symbols, copy products, enums, errors, options, results, selected
region products, lists, strings, and paths execute through deterministic
storage. The zero-family no-tracing runtime is Current.

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

Complete i64 and exact-bit f64 values are inline. Exact `bytes` and affine
`byte-vector` use static or deterministic unique storage in all four engines.
Strings, paths, products, enums, errors, options, and results use deterministic
structural owners/images. Lists use capacity-32 segmented invocation regions;
copy leaves and recursively nested copy lists have selected exact witnesses.
Acyclic products closed over copy lists, scalar leaves, and region products use
invocation-owned records in all four tiers. Their keys cannot cross processes.
Aggregates outside structural or invocation-region storage reject; VM
copy-variable transport is not a native witness ABI. No collector, liveness
map, collection service, barrier, collector configuration, or collector metric
remains.

The structural substrate uses nonwrapping domain/root generations, bounded
fallible capacity, typed layout and semantic identities, ledger-only iterative
release, sealed region-level ownership and weak upgrade, typed cyclic pools,
and exact resource-plane homes. A compact session-private root table now binds
64-bit slot/generation keys to complete typed roots, exact owner state, and
stale-safe loans without deciding liveness. See the
[focused evidence](../current-state/structural-memory-evidence.md).

`ExecutableProgram` retains the complete content-addressed HIR plan plus a
narrow independently recomputed SSA inventory for direct byte-vector owners,
byte loans, and direct typed resources. Concrete structural witness IDs reach
SSA and validated bytecode; zero or duplicate IDs reject before execution. Only
the opaque memory-verified HIR wrapper enters SSA lowering. The verified static/dead SSA drop spine carries
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
  vocabulary and removed-spelling diagnostics are Current.
- Borrowing remains bounded and explicit. Structural field and UTF-8 views,
  aggregate payload transfer, and exact generation-safe resource adapters are
  Current; borrowed returns and unrestricted partial moves remain rejected.
- Forced native claims require synchronous generated entry with zero fallback.
- Native frames retain typed homes, bounds, cleanup obligations, and structured
  outcome state; no liveness root map or collector publication remains.
- Broader region/pool selection, cross-call pool loans, and sealed compact
  images remain later work; the zero-family cutover and no-per-node-RC evidence
  are Current.

## Accepted Next Sequence

1. Retain adversarial ownership, borrowing, destination, failure-cleanup,
   resource-adapter, recursive-call, and limit evidence for every Current
   structural group.
2. Expand the Current concrete structural witness propagation into package and
   residual generic ABIs, then admit structural-owner list elements without
   accepting unknown substitutions.
3. Infer ownership, borrowing, regions, sealed sharing, and pools without
   exposing lifetime syntax or retaining atomic/shared counts or tracing.
4. Measure complete alternatives and remove rejected implementations before
   selecting additional language storage domains.
5. Execute VM/node conformance on native non-Linux and non-x86 hosts; the Current
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
