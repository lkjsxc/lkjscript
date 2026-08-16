# Architecture

## Current flow and authority

```text
strict JSON CLI projection (optional)
    -> strict protocol-v5 JSON frame over private Unix socket
    -> synchronous lkjscriptd
    -> one DurableWorkspace writer per workspace
    -> staged typed transaction over immutable Snapshot
    -> full deterministic validation and derived diff
    -> preflighted compact receipt + artifact + LKJHEAD4
    -> durable immutable revision, then in-memory publication
    -> revision-bound scan query or direct SPG-to-Core-IR lowering
    -> Core IR verifier -> interpreter -> typed response
```

The local service process is the only live program-model writer. `graph.rs` owns immutable snapshots
and saved history; `schema.rs` owns closed node contracts and static operation descriptors;
`transaction.rs` owns
staging, transaction-local nominal target resolution, and compact receipts; `validate.rs` owns graph
acceptance; `type_layout.rs` owns iterative by-value dependency validation and checked derived
layouts; `diff.rs` owns deterministic change facts/digests; `artifact.rs` owns canonical semantic
bytes; `persistence.rs` owns durable publication; `protocol.rs` owns closed logical request/response types;
`transport.rs` owns protocol-v5 length framing and the production client; `query.rs` owns derived scan queries and
repair-context composition; `machine_contract.rs` owns closed schema-discovery DTOs and
`machine.rs` owns strict bounded JSON projection, the executable definition catalogue, and iterative
root closure, including one shared control template and one shared query template projected from the
executable broad descriptors; compact endpoint bindings select exact leaf variants without copied
wrapper forests; `compile.rs` iteratively discovers exact
direct-call and nominal-type closures and lowers
structured regions and aggregates; private `core_ir.rs` owns the dense type table, derived layouts,
multi-function CFG and aggregate/switch contracts, and independent verification; `interpret.rs` owns
public revision-bound runtime-value validation plus the one flat-cell explicit-frame runtime route.
Generated CFG blocks thread the complete
visible semantic environment in `ValueRef` derived order: reference variant first, then canonical
workspace/node identity, then operation output index. This is a private deterministic lowering
choice, not a public identity or serialized contract.

Static operation descriptors are the shared fact owner for arity, operand/result rules, use modes,
literal fields, completeness, and termination. Validators, queries, codecs, lowering, and machine
description consume those facts without runtime registration or heap allocation for lookup. There
is no second graph, type authority, function body, evaluator, mutable client workspace, or persisted
compiler representation.

Normative program-model, identity, transaction, history, and artifact facts belong to
[`docs/spec/semantic-graph.md`](spec/semantic-graph.md). Language and execution semantics belong to
[`docs/spec/language.md`](spec/language.md); protocol and machine-interface facts belong to
[`docs/spec/protocol.md`](spec/protocol.md). This document owns component responsibility and trust
boundaries. [`docs/status.md`](status.md) owns implemented reality,
[`docs/performance.md`](performance.md) owns measurements, and [`docs/roadmap.md`](roadmap.md) owns
future evidence gates.

Queries are pure over one retained immutable revision (or two exact revisions for diff). Incoming
uses/references, dependencies, visible values, legal constructors, blockers, and repair contexts use
full deterministic scans. Legal-constructor and owner-chain pagination streams and counts candidates
while retaining only the requested page; repair context retains at most its per-category budget while
counting the exact total. Repair context is a bounded composition of typed facts, not prose or model
ranking. There is deliberately no reverse-reference index, query engine, or cache; full scans
remain both implementation and oracle until representative repeated cost justifies a narrow index.

The generic CLI is not another service. Its RPC command strictly decodes one bounded JSON envelope,
sends that same closed typed representation in one private length-framed IPC request, and strictly
encodes one typed response. JSON never becomes semantic state. The separate local `schema` command and daemon
`DescribeSchema` derive a
compact manifest, canonical digest, exact root projections with transitive named-definition closure,
explicit full projection, and matching-digest `unchanged` response from one complete executable
description. Root traversal uses an iterative worklist and validates every dependency before a
matching digest can short-circuit output. Projection is recomputed per request; there is no schema
cache, persisted response, root-to-dependency shadow table, or separately maintained schema table.

## Durability and bounded acknowledgement

A workspace retains immutable `revisions/REVISION.lkjscript` files and one compact non-semantic
`LKJHEAD4`. HEAD is independently capped at 16 KiB and stores head revision/hash plus at most one
compact keyed fingerprint/receipt. It stores no full semantic diff or full allocation map; unkeyed
commits preserve an existing keyed replay record. Any non-`LKJHEAD4` bytes, including `LKJHEAD3`,
reject without a compatibility reader. The direct cutover prevents canonical-JSON/v5 fingerprints
from being interpreted under the prior durable identity.

Workspace creation preflights its exact correlated `WorkspaceCreated` response from the canonical
initial snapshot before creating durable workspace files. A transaction commit preflights exact
artifact, HEAD, and protocol response bytes before publication. It writes and flushes a revision
temporary file, renames and syncs it, writes/flushes a HEAD temporary file,
atomically replaces and syncs HEAD, then publishes the in-memory `Arc<Snapshot>` and acknowledges.
Failures before authoritative HEAD leave the prior state authoritative. If publication and rollback
make outcome unknowable, the daemon reports `CommitOutcomeUnknown` and stops.

Validate-only follows the same semantic preparation and byte preflight but writes and publishes
nothing. Restart bounds files before reading, decodes every contiguous retained artifact, validates
history, checks HEAD checksum/hash, and recomputes compact receipt facts from retained snapshots.
Corrupt or ambiguous durable state is rejected, not repaired heuristically.

## Trust boundaries

- **JSON stdin/stdout:** 8 MiB input, bounded nesting, strict fields/variants/canonical IDs/trailing
  policy; streaming 32 MiB output cap and boundary-error messages capped at 1,024 UTF-8 bytes;
- **framed JSON IPC:** private socket permissions/peer filesystem boundary, checked lengths before
  allocation, strict envelopes, request correlation, request-side EOF before dispatch, response-side
  EOF before acceptance, and absolute connection deadlines;
- **artifact and HEAD bytes:** separate bounded canonical decoders, hashes/checksums, graph/history
  validation, strict trailing-byte policy;
- **agent proposals:** untrusted model output reaches deterministic validators only as closed typed requests;
- **runtime:** verified Core IR only, exact revision-bound primitive/product/sum values, bounded
  nesting/items/encoded bytes/result projection, positive bounded fuel and frame policy, and one
  non-recursive interpreter loop over explicit frames; each frame uses a flat cell arena with separate
  initialized facts. Aggregate instructions copy or initialize exact arena ranges directly, switch
  reads only the discriminant, and block entry invalidates facts without clearing the arena. Fuel is
  charged before work as one base per instruction/transfer plus `max(1,cells)` per logical value copy,
  with full-sum charging for variant canonicalization plus the active payload's logical copy. The 65,536-cell peak covers live arenas plus
  exact argument/edge/return/public-flatten scratch and prospective callee arenas before allocation or
  copy; the language runtime generates no program machine code and exposes no implicit host access or
  foreign program boundary.

User-scalable graph traversals use loops and explicit work collections rather than unbounded native
recursion. Operational page/frame/context limits protect specific boundaries; they are resource
policies, not claims that memory-safe execution is unbounded or that semantic programs are infinite.

## Memory safety and trusted computing base

Memory safety is an enduring requirement. The active Rust package applies
`unsafe_code = "forbid"` to the library, both binaries, and package tests; the current language has no
unchecked memory operation, and current decoders, artifact loaders, compiler, and runtime validate
lengths, counts, conversions, indexes, tags, layouts, and allocation policies before use. Safe Rust
is an implementation control, not a complete proof. In particular, it does not by itself prevent
resource exhaustion, process stack exhaustion from an accidentally recursive algorithm, logic
errors, corrupt external components, or misuse inside trusted dependencies.

The current trusted computing base includes the Rust compiler and standard library, Cargo and build
tooling, the operating system, filesystem and Unix-socket implementations, CPU-specific code selected
by dependencies, and every resolved dependency. Direct normal dependencies have these consumers:

- `blake3` provides artifact, change, transaction, and machine-contract hashing;
- `fs2` provides exclusive workspace/service file locking through platform interfaces;
- `getrandom` provides workspace identity entropy through operating-system facilities;
- `serde` and `serde_json` provide the closed strict JSON transport and generated descriptions;
- `tempfile` is development-only and owns isolated test directories.

The exact resolved `blake3`, `getrandom`, `serde`, and `serde_json` packages include custom build
targets. Resolved dependency source also contains unsafe or platform-specific implementation code;
`blake3` ships CPU-specific Rust and native source, while `fs2` and `getrandom` reach operating-system
interfaces. Fresh builds execute dependency build targets (including the `cc` build dependency used
by the resolved hashing package); the repository has no project-owned build script. This repository
does not claim an unsafe-free or native-code-free transitive build. `Cargo.lock`, dependency review,
boundary tests, and platform scoping are part of the evidence.

Future FFI, generated machine code, native runtime workers, foreign memory, or native dependencies
would extend the trusted computing base and require an explicit isolated boundary, validation before
entry, a documented safety invariant, and applicable Miri, sanitizer, fuzz, or differential evidence.
None exists in the current implementation. The local service is not a sandbox.

Memory safety, resource exhaustion, resource ownership, deterministic cleanup, aliasing,
concurrency safety, and permission security are separate contracts. Current pure immutable values use
flat cells and copying. No universal future heap or lifetime-management mechanism has been selected;
real sharing, cycles, mutation, external resources, latency, memory, concurrency, and agent-authoring
workloads must drive that choice.

## Deliberate restraint

The measured bootstrap retains `BTreeMap`, vectors, full snapshot clones, full semantic
recomputation/diff materialization, and full artifact rewrites. It has no database, journal, async
runtime, generic graph framework, runtime schema registry, reverse index, cache, source projection,
native backend, managed heap, plugin mechanism, or remote service. A replacement requires a current
consumer, measurements, one preserved authority path, and evidence that supports its added cost.
