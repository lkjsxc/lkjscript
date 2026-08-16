# Architecture

## Current flow and authority

```text
strict JSON CLI projection (optional)
    -> strict protocol-v7 JSON frame over private Unix socket
    -> synchronous lkjscriptd
    -> one DurableWorkspace writer per workspace
    -> staged typed transaction over immutable Snapshot
    -> full deterministic validation and derived diff
    -> preflighted compact receipt + artifact + LKJHEAD6
    -> durable immutable revision, then in-memory publication
    -> revision-bound scan query or direct SPG-to-Core-IR lowering
    -> Core IR verifier -> interpreter -> typed response
```

The local service process is the only live program-model writer. `graph.rs` owns immutable snapshots
and saved history; `schema.rs` owns closed node contracts and static operation descriptors;
`transaction.rs` owns staging, bounded iterative explicit/inline proposal normalization,
transaction-local nominal target resolution, and compact receipts; `validate.rs` owns graph
acceptance; `type_layout.rs` owns iterative by-value dependency validation and checked derived
layouts; `diff.rs` owns deterministic change facts/digests; `artifact.rs` owns canonical semantic
bytes; `persistence.rs` owns durable publication; `protocol.rs` owns closed logical request/response types;
`transport.rs` owns protocol-v7 length framing and the production client; `query.rs` owns derived scan queries and
repair-context composition; `machine_contract.rs` owns closed schema-discovery DTOs and
`machine.rs` owns strict bounded JSON projection, the executable definition catalogue, and iterative
root closure, including one shared control template and one shared query template projected from the
executable broad descriptors; compact endpoint bindings select exact leaf variants without copied
wrapper forests; `compile.rs` iteratively discovers exact
direct-call and nominal-type closures and lowers
structured regions and aggregates; private `core_ir.rs` owns the dense type table, derived layouts,
multi-function CFG and aggregate/switch contracts, and independent verification; `interpret.rs` owns
public revision-bound runtime-value validation plus the one flat-cell explicit-frame runtime route,
including the invocation-scoped immutable-byte arena and its private checked handles.
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
description. Recursive inline value DTOs are flattened by the transaction worklist into ordinary
persistent operations; the nested proposal never reaches graph storage, lowering, or execution.
Root traversal uses an iterative worklist and validates every dependency before a
matching digest can short-circuit output. Projection is recomputed per request; there is no schema
cache, persisted response, root-to-dependency shadow table, or separately maintained schema table.
The optional `session` command keeps only the CLI process alive. Each line passes through the same
decoder, `Client`, and encoder and still creates one ordinary daemon connection; there is no second
request DTO, persistent daemon connection, correlation layer, or retry policy.

## Durability and bounded acknowledgement

A workspace retains immutable `revisions/REVISION.lkjscript` files and one compact non-semantic
`LKJHEAD6`. HEAD is independently capped at 16 KiB and stores head revision/hash plus at most one
compact keyed fingerprint/receipt. It stores no full semantic diff or full allocation map; unkeyed
commits preserve an existing keyed replay record. Any non-`LKJHEAD6` bytes, including `LKJHEAD5`,
reject without a compatibility reader. The direct cutover prevents canonical-JSON/v7 fingerprints
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
- **runtime:** verified Core IR only, exact revision-bound primitive/bytes/product/sum values, bounded
  nesting/items/structural bytes/result projection, positive bounded fuel and frame policy, and one
  non-recursive interpreter loop over explicit frames; each frame uses a flat cell arena with separate
  initialized facts. A separate invocation-owned arena stores immutable byte backing and
  constant-depth views behind nonzero checked handle cells; handles never serialize, escape, or
  reuse an index during the invocation. Aggregate instructions read or initialize exact cell ranges directly, switch
  reads only the discriminant, and block entry invalidates facts without clearing the arena. Fuel is
  charged before work as one base per instruction/transfer plus `max(1,cells)` per logical value
  transfer, with per-octet byte equality work and full-sum charging for variant canonicalization plus
  the active payload's logical transfer. The 65,536-cell peak covers live frame arrays plus
  exact argument/edge/return/public-flatten scratch and prospective callee arenas before allocation or
  transfer. Separate policies bound cumulative visible byte allocation, distinct retained backing,
  managed backing/view objects, decoded input, and materialized output. Public output becomes owned bytes
  before arena drop; Rust scope cleanup covers success and every trap/policy failure. The language
  runtime generates no program machine code and exposes no implicit host access or
  foreign program boundary.

There is no cooperative in-Run cancellation mechanism. A disconnected client does not make a byte
handle escape: the fuel/frame-bounded synchronous Run continues to a normal result or trap and then
drops its arena. Daemon-process termination relies on operating-system process reclamation rather
than language-level cleanup ordering. A future cancellable effect or resource must define a stronger
explicit cleanup contract before it is accepted.

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

- `base64` provides canonical unpadded URL-safe public byte encoding/decoding with default features
  disabled and only its safe `std` feature enabled;
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
concurrency safety, and permission security are separate contracts. Current immediate and fixed
immutable values use flat cells; nonescaping bytes use a bounded invocation arena and opaque handles.
That choice does not select a universal future heap. Escaping cycle-free values would reopen precise
ownership or reference counting; measured long-invocation retention would reopen lexical regions;
real long-lived cycles would reopen isolated tracing; and external resources would require affine
ownership plus explicit deterministic cleanup.

## Deliberate restraint

The measured bootstrap retains `BTreeMap`, vectors, full snapshot clones, full semantic
recomputation/diff materialization, and full artifact rewrites. It has no database, journal, async
runtime, generic graph framework, runtime schema registry, reverse index, cache, source projection,
native backend, general managed heap, plugin mechanism, or remote service. A replacement requires a current
consumer, measurements, one preserved authority path, and evidence that supports its added cost.

## Long-horizon revalidation

This campaign rechecked every architecture lens without turning a bootstrap fact into permanent
policy. “Requirement” below is the enduring constraint; “decision” is only this campaign's choice.

| Lens | Current fact | Requirement | Campaign decision | Entry evidence / reversal condition |
|---|---|---|---|---|
| Product boundary | External agent, local model service | Deterministic one-authority acceptance | Improve proposal interface only | Agent authors/repairs publicly; reverse model-dependent correctness |
| Human role | Humans own intent and review | Preserve governance and explanation | Keep human-first docs and review facts | Reject opaque changes without bounded explanation |
| Source independence | No source frontend | Text can never be coequal authority | No parser; keep future views open | Reject render/reparse editing dependence |
| Semantic shape | Closed direct Rust types | One typed model and validator | Keep SPG; no generic graph | Reverse abstraction that duplicates ownership facts |
| Stable identity | Monotonic workspace Node IDs | Independent of names, bytes, positions | Preserve identity through inline normalization | Reverse if proposal spelling changes IDs |
| Identity granularity | Every semantic node has an ID | Identity needs continuity or targeting | Remove labels, not semantic IDs | Reopen only with artifact/history/query closure |
| Incomplete programs | Missing bodies and typed holes | Exact typed queryable repair | Holes remain explicit | Reject anonymous holes without retrieval |
| Transactions | Atomic commit and validate-only | Rejection changes nothing | Normalize wholly before publication | Reverse mutation outside one transaction |
| History and diffs | Immutable revisions, semantic diffs | History ignores proposal spelling | Inline nesting is discarded | Reverse if views become history authority |
| Proposal language | Structured bounded drafts | Graph validator remains final | One inline value form only | Delete if it grows into a macro language |
| Machine contract | Executable roots and closure | Complete accepted shapes | Extend the same recursive catalogue | Reject hand-maintained lite schemas |
| Codex integration | Generic CLI plus same-vocabulary session; disposable MCP adapter rejected | Projection cannot own semantics | Retain process-only session | Delete if no harness can keep it alive or process reduction ceases to matter |
| Prompt/cache economics | Policy and tools cost context | Correctness outranks compactness | Shrink durable policy; stabilize schemas | Reject token claims inferred from bytes |
| Diagnostics | Typed bounded errors and paths | Local deterministic correction facts | Add anonymous inline paths | Reverse if locality materially worsens |
| Type system | Closed primitives and nominal data | Explicit exact equality/conversion | No generics or dynamic types | Require a second real abstraction consumer |
| Primitive values | Unit, bool, checked i64, immutable bytes | Exact checked semantics | Dedicated bounded bytes | Reject concision through hidden coercion or generic sequence scope |
| Numeric semantics | Checked add and exact compare | Order and traps are observable | Preserve normalized evaluation order | Reverse changed fuel or trap behavior |
| Named data | Immutable records and fixed variants | Identity-based nominal equality | Exercise inline construction/projection | Reject structural typing as collateral |
| Recursive data | By-value cycles reject | Recursion needs lifetime semantics | No recursive values | Reject lists as a heap shortcut |
| Generics | None in language | Need identity/substitution/lowering rules | Defer | Enter only for repeated consumers |
| Collections | No sequence, map, or set | Explicit order/size/allocation | No collection framework | Reject one-consumer generalization |
| Bytes and text | Managed immutable bytes; no text | Canonical encoding and bounded sharing | Retain URL-safe unpadded base64 and five byte operations | Reopen representation only by direct cutover or add text for its own consumer |
| Memory management | Flat fixed cells plus invocation byte arena | Techniques may differ by value class | Checked handles for nonescaping bytes; no RC/GC | Reopen at measured retention, escape, cycle, or resource trigger |
| Ownership/borrowing | Read-only duplicable values; no move or borrow rules | Explicit alias/lifetime semantics when needed | Infer Run lifetime; no borrow checker | Require a real exclusive/shared consumer |
| Resource values | None | Non-duplication and deterministic cleanup | Defer | Reject ambient integer handles |
| Effects/capabilities | Pure semantic programs | Every effect needs typed authority | No effects | Enter with vertical permission/failure contract |
| Host I/O | Service I/O only | Separate service operation from language effect | Expose no host operation | Require worker and permission boundary |
| Errors/traps/results | Distinct structured outcomes | Do not collapse domains | Keep typed proposal errors; no exceptions | Reject catch-all strings |
| Determinism | Pure ordered observable boundaries | Nondeterminism must be explicit | Canonical inline and member order | Reverse hidden map/thread/filesystem order |
| Concurrency/async | Synchronous one-request connection | Explicit snapshot/publication semantics | No async or request concurrency | Enter only with throughput/isolation evidence |
| Cancellation/timeout | Transport deadlines; fuel/frames | State effect outcome exactly | Preserve publication rules | Reject silent retry after timeout |
| Packages/modules | One workspace, no ecosystem | Immutable declared dependency authority | No package manager | Require named dependency consumer |
| Persistence format | Artifact 4 / SPG004, HEAD6 | Canonical bounded unambiguous bytes | Raw bounded literals and v7 fingerprint | Reject compatibility reader without users |
| Journal/compaction | Full artifact per revision | Preserve authority and non-reuse | No journal | Enter at measured size/write threshold |
| Branch/merge | One head | Explicit parents and semantic conflicts | No branches | Require collaborative consumer |
| Queries/index/cache | Deterministic full scans | Recomputation remains oracle | No index or cache | Differential-test any measured optimization |
| Incrementality | Broad recomputation | Derived revision-bound results | No framework | Require representative dependency cost |
| Core IR | One private verified IR | Derived semantics only | Unchanged | Verify each future instruction independently |
| Interpreter | Explicit-frame oracle | Exact fuel, memory, order | Preserve behavior | Reject undifferentiated fast paths |
| AOT/JIT/native | None | Bind exact revision and preserve oracle | No native tier | Enter after interpreter cost dominates workload |
| Sandboxing | Local service is not a sandbox | Claims match enforced isolation | No sandbox work | Require threat model before effects/foreign code |
| Cross-platform | Linux x86-64 Unix IPC | Semantic portability unless explicit | No cfg expansion | Require named platform consumer |
| Daemon topology | One local durable writer; one request per connection | One logical authority per namespace | Session reuses only client process | Reject hidden multi-frame or split-brain writers |
| Multi-tenancy | OS directory/socket ownership | Auth, authorization, quota, isolation | Keep local-only boundary | Require deployment threat model |
| Security model | Inputs and durable bytes untrusted | Unknown forms reject | Preserve strict v7 cutover | Reject permissive recovery or unknown fields |
| Supply chain | Small locked dependency set | Every dependency has a consumer | Add safe-feature `base64` for the one canonical codec | Remove if build/safety cost or aliases appear |
| Formal methods | Tests, models, mutations | Use tools for sustained state risk | No ceremonial proof layer | Enter for concurrency/merge/resource ownership |
| Testing/fuzzing | Focused tests plus deterministic mutation | Retain success and rejection evidence | Extend equivalence/depth/protocol tests | Do not call mutation smoke fuzzing |
| Observability/debugging | Structured outcomes, no debugger | Bounded revision-bound observation | Temporary campaign metrics only | Reject unbounded global traces |
| Repository topology | One Rust package, fact-owned modules | Change locality over line quotas | Keep one package and touched owners | Split only for measured boundary |
| Documentation | Small role-specific maintained set | One fact owner and plain language | Update existing owners only | Delete duplicate catalogues |
| Compatibility/versioning | Pre-release direct replacement | Active bytes unambiguous | v7/artifact4/SPG004/HEAD6; no old reader | Reject hidden editions/fallbacks |
| API economics | Bytes/calls/errors measured | Never trade correctness for cost | Compare equal explicit/inline work | Reject unequal-task savings |
| Runtime performance | Interpreter bootstrap observations | Optimize representative workloads | No runtime optimization | Preserve oracle and reversal metric |
| Memory/energy footprint | Flat-cell bounds, full snapshots | Separate peak, retained, copy, CPU cost | Syntax savings make no runtime claim | Require named bottleneck |
| Self-hosting | Rust implementation only | Real component without privilege escape | Defer | Reject symbolic milestone pressure |
| Standard library | No ecosystem or broad library | Semantic, reproducible, capability-aware | Add no convenience framework | Require repeated application need |
| Distribution/deployment | Local binaries and state dirs | Reproducible ownership and upgrade rules | No installer/service manager | Require named deployment |
| Reproducibility/provenance | Deterministic revision hashes | Bind exact public inputs, not hidden reasoning | Record environment and public evidence | Reject prompt provenance as validity |
| Recovery/disaster | Corruption and ambiguity reject | Never guess authority | Preserve writer-stop behavior | Reject automatic artifact repair |
| Governance/evolution | Incompatible change is allowed | Use freedom to converge | One direct cutover and one next gate | Reverse churn without evidence |
