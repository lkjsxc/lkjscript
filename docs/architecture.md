# Architecture

## Current flow and authority

```text
semantic workbench (preferred) or strict JSON diagnostic projection
    -> client-side packet/view/plan normalization when selected
    -> strict protocol-v8 JSON frame over private Unix socket
    -> synchronous lkjscriptd
    -> one DurableWorkspace writer per workspace
    -> staged typed transaction over immutable Snapshot
    -> full deterministic validation and derived diff
    -> preflighted compact receipt + artifact + LKJHEAD7
    -> durable immutable revision, then in-memory publication
    -> revision-bound scan query or direct SPG-to-Core-IR lowering
    -> Core IR verifier -> derived ownership plan -> ownership verifier
    -> managed interpreter -> typed response
```

The local service process is the only live program-model writer. `graph.rs` owns immutable snapshots
and saved history; `schema.rs` owns closed node contracts and static operation descriptors;
`transaction.rs` owns staging, bounded iterative explicit/inline proposal normalization,
transaction-local nominal target resolution, and compact receipts; `validate.rs` owns graph
acceptance; `type_layout.rs` owns iterative by-value dependency validation and checked derived
layouts; `diff.rs` owns deterministic change facts/digests; `artifact.rs` owns canonical semantic
bytes; `persistence.rs` owns durable publication; `protocol.rs` owns closed logical request/response types;
`transport.rs` owns protocol-v8 length framing and the production client; `query.rs` owns derived scan queries and
repair-context composition; `machine_contract.rs` owns closed schema-discovery DTOs and
`machine.rs` owns strict bounded JSON projection, the executable definition catalogue, and iterative
root closure, including one shared control template and one shared query template projected from the
executable broad descriptors; compact endpoint bindings select exact leaf variants without copied
wrapper forests; `compile.rs` iteratively discovers exact
direct-call and nominal-type closures and lowers
structured regions and aggregates; private `core_ir.rs` owns the dense type table, derived layouts,
multi-function CFG and aggregate/switch contracts, and independent verification; `ownership.rs`
owns derived managed-reference maps, control-flow ownership planning, and a verifier that separately
recomputes liveness, edge cleanup, and uniqueness; `managed.rs` owns generation-checked byte views,
backing buffers, exact ownership claims, reclamation, reuse, and physical metrics; `interpret.rs`
owns public revision-bound runtime-value validation plus the one flat-cell explicit-frame semantic
route and applies only a verified ownership plan.

The agent-facing boundary is kept out of `machine.rs`: `workbench/plan.rs` owns the bounded iterative
proposal grammar and normalization into existing protocol DTOs; `workbench/context.rs` owns pure
revision-bound packet composition, canonical aliases, digesting, and strict packet decoding;
`workbench/view.rs` owns deterministic terminal-safe semantic review and diff rendering;
`workbench/help.rs` derives concise authoring cards from the executable machine description; and
`bin/lkjscript/agent.rs` owns command routing and presentation exit behavior. These modules consume
the same query, schema, transaction, Run, and error types as the raw projection. They define no node,
operation, validation, execution, or persistence contract.
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

One context packet composes those existing queries for a closed purpose and exact target set. Its
digest binds workspace, revision, active machine-schema digest, purpose, options, aliases, and all
returned facts. Alias spelling is local to that packet and never enters a daemon request: the client
resolves it to a canonical Node ID before sending the normalized typed transaction. Saved packet
files are disposable client data revalidated on every read. There is no automatic schema/context
cache, server-side candidate, persistent workbench session, transcript store, or model-ranked
retrieval path.

The semantic review renderer is a one-way presentation over packet facts. It exposes revision,
packet and schema identities, signatures, structured bodies, typed placeholders, exact change facts,
and explicit omissions while excluding Core IDs, layouts, handles, artifact offsets, and storage
order. It cannot be parsed into a proposal and is never written into an artifact.

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
`LKJHEAD7`. HEAD is independently capped at 16 KiB and stores head revision/hash plus at most one
compact keyed fingerprint/receipt. It stores no full semantic diff or full allocation map; unkeyed
commits preserve an existing keyed replay record. Any non-`LKJHEAD7` bytes, including `LKJHEAD6`,
reject without a compatibility reader. The direct cutover prevents canonical-JSON/v8 fingerprints
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
- **workbench plans and packets:** plans are bounded to 8 MiB, 32 open containers, and 65,536
  counted values and parse without user-depth native recursion; packet and rendered-view boundaries
  are independently capped at 4 MiB. Unknown fields/forms, duplicate fields, stale schema or packet
  digests, foreign workspace/revision domains, unknown aliases, trailing input, and malformed UTF-8
  reject before dispatch. Presentation quotes untrusted names and bytes, and packet aliases never
  fall back to names, current head, Node IDs, or draft symbols;
- **framed JSON IPC:** private socket permissions/peer filesystem boundary, checked lengths before
  allocation, strict envelopes, request correlation, request-side EOF before dispatch, response-side
  EOF before acceptance, and absolute connection deadlines;
- **artifact and HEAD bytes:** separate bounded canonical decoders, hashes/checksums, graph/history
  validation, strict trailing-byte policy;
- **agent proposals:** untrusted model output reaches deterministic validators only as closed typed requests;
- **runtime:** verified Core IR only, followed by a derived ownership plan that cannot execute until
  a separate verifier has recomputed managed-cell maps, liveness, exact actions, edge cleanup, and
  reuse eligibility. One non-recursive interpreter loop uses explicit frames, flat cells, and
  separate initialized facts. An invocation-owned managed store keeps byte backing and normalized
  views behind private typed index-plus-generation handles. Every access validates kind, index,
  generation, liveness, and range; stale handles reject, generation wrap retires a slot, and handles
  never serialize or escape. Owned claims use checked non-atomic counts only when sharing remains;
  borrows and transfers do not increment them. Verified last drops reclaim descriptors and backing,
  and verified unique full-left concat may reuse capacity after all semantic and allocation
  preflights. Shared, borrowed, partial-view, or aliased inputs use the allocate-new fallback.
  Aggregate managed-cell maps cover every record field and only the active variant payload. Fuel is
  charged before work as one base per instruction/transfer plus `max(1,cells)` per logical value
  transfer, per-octet byte equality work, and full-result concat work; ownership counts and reuse do
  not alter it. The 65,536-cell peak covers live frame arrays plus exact argument, edge, return,
  public-flatten scratch, and prospective callee frames. Separate policies bound cumulative visible
  construction, live backing, live managed objects, decoded input, and materialized output. Public
  output becomes owned bytes before store destruction; verified cleanup roots cover success and
  every trap/policy failure, with final Rust scope drop as a safe backstop. A test-only allocate-new
  mode over the same Core behavior remains the differential oracle. The runtime generates no program
  machine code and exposes no implicit host access or foreign program boundary.

There is no cooperative in-Run cancellation mechanism. A disconnected client does not make a byte
handle escape: the fuel/frame-bounded synchronous Run continues to a normal result or trap and then
drops its store. Daemon-process termination relies on operating-system process reclamation rather
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
immutable values use flat cells; nonescaping bytes use compiler-inferred ownership, a verified plan,
and a bounded managed store. This is ownership-first rather than a universal memory mechanism:
precise reference counting is only the cycle-free sharing fallback, and lexical regions remain
available when one future common lifetime is simpler. Real long-lived cycles would reopen isolated
tracing; external resources would require semantic affine ownership, explicit permission, and
deterministic close/cleanup contracts. No tracing collector, surface lifetime syntax, or resource
finalizer exists.

## Deliberate restraint

The measured bootstrap retains `BTreeMap`, vectors, full snapshot clones, full semantic
recomputation/diff materialization, and full artifact rewrites. It has no database, journal, async
runtime, generic graph framework, runtime schema registry, reverse index, cache, source frontend,
native backend, general managed heap, plugin mechanism, or remote service. The retained read-only
semantic rendering is not a round-trip source projection. A replacement requires a current
consumer, measurements, one preserved authority path, and evidence that supports its added cost.

## Long-horizon revalidation

This campaign rechecked every architecture lens without turning a bootstrap fact into permanent
policy. “Requirement” below is the enduring constraint; “decision” is only this campaign's choice.

| Lens | Current fact | Requirement | Campaign decision | Entry evidence / reversal condition |
|---|---|---|---|---|
| Product boundary | Semantic workbench over a local model service | Deterministic one-authority acceptance | Prefer context/view/plan; retain raw JSON as diagnostic | Reverse if equal-task success, correction, or review evidence regresses |
| Human role | Humans own intent and review | Preserve governance and explanation | Deterministic semantic review plus exact typed expansion | Reject views that hide identity-changing edits |
| Source independence | No source frontend; one-way semantic review text | Text can never be coequal authority | Retain non-round-trip rendering | Reject render/reparse editing dependence |
| Semantic shape | Closed direct Rust types | One typed model and validator | Keep SPG; no generic graph | Reverse abstraction that duplicates ownership facts |
| Stable identity | Monotonic workspace Node IDs | Independent of names, bytes, positions | Preserve identity through inline normalization | Reverse if proposal spelling changes IDs |
| Identity granularity | Every semantic node has an ID | Identity needs continuity or targeting | Remove labels, not semantic IDs | Reopen only with artifact/history/query closure |
| Incomplete programs | Missing bodies and typed holes | Exact typed queryable repair | Holes remain explicit | Reject anonymous holes without retrieval |
| Transactions | Atomic commit and validate-only | Rejection changes nothing | Normalize wholly before publication | Reverse mutation outside one transaction |
| History and diffs | Immutable revisions, semantic diffs | History ignores proposal spelling | Inline nesting is discarded | Reverse if views become history authority |
| Proposal language | Typed drafts plus compact ephemeral plan projection | Graph validator remains final | Iterative one-to-one plan normalization | Delete if grammar needs independent semantic rules |
| Machine contract | Executable roots and closure | Complete accepted shapes | Extend the same recursive catalogue | Reject hand-maintained lite schemas |
| Codex integration | Preferred workbench; raw JSON and process-only session remain diagnostic | Projection cannot own semantics | Retain direct CLI, no adapter or persistent connection | Reopen lifecycle work only for measured remaining overhead |
| Prompt/cache economics | Stable policy, task packets, digest facts | Correctness outranks compactness | Keep packets bounded and cache files disposable | Reject token claims inferred from bytes |
| Diagnostics | Typed errors, semantic origins, targeted debug packets | Local deterministic correction facts | Current runtime facts suffice; no trace framework | Add bounded call-path facts only if a retained task needs them |
| Type system | Closed primitives and nominal data | Explicit exact equality/conversion | No generics or dynamic types | Require a second real abstraction consumer |
| Primitive values | Unit, bool, checked i64, immutable bytes | Exact checked semantics | Dedicated bounded bytes | Reject concision through hidden coercion or generic sequence scope |
| Numeric semantics | Checked add and exact compare | Order and traps are observable | Preserve normalized evaluation order | Reverse changed fuel or trap behavior |
| Named data | Immutable records and fixed variants | Identity-based nominal equality | Exercise inline construction/projection | Reject structural typing as collateral |
| Recursive data | By-value cycles reject | Recursion needs lifetime semantics | No recursive values | Reject lists as a heap shortcut |
| Generics | None in language | Need identity/substitution/lowering rules | Defer | Enter only for repeated consumers |
| Collections | No sequence, map, or set | Explicit order/size/allocation | No collection framework | Reject one-consumer generalization |
| Bytes and text | Managed immutable bytes; no text | Canonical encoding and bounded sharing | Retain URL-safe unpadded base64 and six byte operations including concat | Add text only for its own consumer |
| Memory management | Flat cells plus managed byte store | Techniques may differ by value class | Verified ownership, early reclaim, narrow RC fallback; no tracing | Reopen at escape, cycle, region, or resource trigger |
| Ownership/borrowing | Duplicable immutable values; derived physical actions | Keep memory choreography out of ordinary semantics | Infer and verify borrow/share/transfer/drop | Add surface ownership only for an observable consumer |
| Resource values | None | Non-duplication and deterministic cleanup | Defer | Reject ambient integer handles |
| Effects/capabilities | Pure semantic programs | Every effect needs typed authority | No effects | Enter with vertical permission/failure contract |
| Host I/O | Service I/O only | Separate service operation from language effect | Expose no host operation | Require worker and permission boundary |
| Errors/traps/results | Distinct structured outcomes | Do not collapse domains | Keep typed proposal errors; no exceptions | Reject catch-all strings |
| Determinism | Pure ordered observable boundaries | Nondeterminism must be explicit | Canonical inline and member order | Reverse hidden map/thread/filesystem order |
| Concurrency/async | Synchronous one-request connection | Explicit snapshot/publication semantics | No async or request concurrency | Enter only with throughput/isolation evidence |
| Cancellation/timeout | Transport deadlines; fuel/frames | State effect outcome exactly | Preserve publication rules | Reject silent retry after timeout |
| Packages/modules | One workspace, no ecosystem | Immutable declared dependency authority | No package manager | Require named dependency consumer |
| Persistence format | Artifact 5 / SPG005, HEAD7 | Canonical bounded unambiguous bytes | Concat tag and v8 fingerprint, no derived plans | Reject compatibility reader without users |
| Journal/compaction | Full artifact per revision | Preserve authority and non-reuse | No journal | Enter at measured size/write threshold |
| Branch/merge | One head | Explicit parents and semantic conflicts | No branches | Require collaborative consumer |
| Queries/index/cache | Deterministic full scans plus client-saved packets | Recomputation remains oracle | No automatic cache or index | Differential-test any measured optimization |
| Incrementality | Broad recomputation | Derived revision-bound results | No framework | Require representative dependency cost |
| Core IR | One private verified IR | Derived semantics only | Add concat and derived managed-cell maps | Verify each future instruction independently |
| Interpreter | One explicit-frame semantic route | Exact fuel, values, order, and traps | Verified ownership is default; allocate-new is a test oracle | Delete reuse if differential evidence fails |
| AOT/JIT/native | None | Bind exact revision and preserve oracle | No native tier | Enter after interpreter cost dominates workload |
| Sandboxing | Local service is not a sandbox | Claims match enforced isolation | No sandbox work | Require threat model before effects/foreign code |
| Cross-platform | Linux x86-64 Unix IPC | Semantic portability unless explicit | No cfg expansion | Require named platform consumer |
| Daemon topology | One local durable writer; one request per connection | One logical authority per namespace | Session reuses only client process | Reject hidden multi-frame or split-brain writers |
| Multi-tenancy | OS directory/socket ownership | Auth, authorization, quota, isolation | Keep local-only boundary | Require deployment threat model |
| Security model | Inputs, plans, handles, and durable bytes untrusted | Unknown forms reject | Preserve strict v8 cutover and plan verification | Reject permissive recovery or unknown fields |
| Supply chain | Small locked dependency set | Every dependency has a consumer | Add safe-feature `base64` for the one canonical codec | Remove if build/safety cost or aliases appear |
| Formal methods | Tests, models, mutations | Use tools for sustained state risk | No ceremonial proof layer | Enter for concurrency/merge/resource ownership |
| Testing/fuzzing | Focused tests plus deterministic mutation | Retain success and rejection evidence | Extend equivalence/depth/protocol tests | Do not call mutation smoke fuzzing |
| Observability/debugging | Structured outcomes and bounded semantic packets; no debugger | Bounded revision-bound observation | Retain one-way review and targeted debug context | Reject unbounded global traces |
| Repository topology | One Rust package; separate workbench plan/context/view owners | Change locality over line quotas | Keep machine authority untouched by presentation code | Split another large owner only when its changed path proves need |
| Documentation | Small role-specific maintained set | One fact owner and plain language | Update existing owners only | Delete duplicate catalogues |
| Compatibility/versioning | Pre-release direct replacement | Active bytes unambiguous | v8/artifact5/SPG005/HEAD7; no old reader | Reject hidden editions/fallbacks |
| API economics | Bytes/calls/errors and exposed provider token classes measured | Never trade correctness for cost | Retain workbench after equal-task observation | Reverse if broader tasks lose the success/correction/context tradeoff |
| Runtime performance | Canonicalizer exposes repeated concat | Optimize representative workloads | Retain measured unique-left reuse with fallback | Delete below copied-byte/peak benefit threshold |
| Memory/energy footprint | Early reclaim plus reuse | Separate peak, cumulative, retained, copy, and time | Record physical metrics apart from fuel | Reverse any policy that changes semantics implicitly |
| Self-hosting | Rust implementation only | Real component without privilege escape | Defer | Reject symbolic milestone pressure |
| Standard library | No ecosystem or broad library | Semantic, reproducible, capability-aware | Add no convenience framework | Require repeated application need |
| Distribution/deployment | Local binaries and state dirs | Reproducible ownership and upgrade rules | No installer/service manager | Require named deployment |
| Reproducibility/provenance | Deterministic revision hashes | Bind exact public inputs, not hidden reasoning | Record environment and public evidence | Reject prompt provenance as validity |
| Recovery/disaster | Corruption and ambiguity reject | Never guess authority | Preserve writer-stop behavior | Reject automatic artifact repair |
| Governance/evolution | Incompatible change is allowed | Use freedom to converge | One direct cutover and one next gate | Reverse churn without evidence |
