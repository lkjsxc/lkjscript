# Current status

Date: 2026-08-16

Campaign base: `8d08f507d474b335512ea5afdba6be186e3b8517` on `main`

## Implemented product path

The repository contains one Rust package and one active route from a coding agent to a saved and
runnable program:

```text
strict generic JSON CLI (optional) -> private protocol-v5 framed-JSON Unix IPC -> synchronous local service
-> durable workspace -> typed staged transaction -> immutable typed program-model revision
-> revision-bound scan query or direct Core IR lowering -> verifier -> interpreter
```

Program meaning is stored in the typed, versioned model formally specified as the Semantic Program
Graph (SPG). Source files are not required authority, JSON is transport only, and no client owns a
second mutable program. `lkjscriptd` is the only live durable writer for a workspace.

The current public path implements:

- named immutable record types and variant types with fixed alternatives as persistent nodes;
- persistent record-field and variant identity, exact owner/ordinal validation, atomic declarations,
  forward type targets, immutable shape, and deterministic rejection of by-value cycles;
- deterministic derived layouts for record fields, variant discriminants and payloads, and runtime
  cell footprints without persisting layout as program state;
- exact record construction, field projection, variant construction, and lazy handling of every
  variant through identity-keyed structured drafts normalized to declaration order;
- structured functions, parameters, bodies, calls, `if`, counted `for_i64`, typed placeholders,
  yields, returns, checked integer addition, and integer comparison;
- one identity-preserving `RefineHole` transition that fills a typed placeholder while retaining its
  Node ID, owner, body position, output zero, and incoming uses;
- deterministic semantic diffs that report `OperationRefined` and `Renamed` rather than identity
  churn;
- atomic commit and validate-only transactions, no identity consumption on rejection or
  validate-only, bounded symbolic proposal labels and receipts, at most 64 selected returned
  bindings, and commit-only idempotency;
- revision-bound query batches, paginated node/body/use/reference/dependency/diff/type facts, visible
  values, legal constructors, completeness blockers, owner chains, and bounded repair context;
- immutable format-3 `.lkjscript` artifacts under semantic schema `lkjscript-spg003`, compact
  checksummed `LKJHEAD4`, contiguous history validation, restart, and strict corruption rejection;
- protocol and strict JSON version 5 with one request/response per private local connection;
- direct lowering of the complete selected-entry reachable definitions and named types to one private
  Core IR, followed by independent verification and one explicit-frame interpreter route;
- exact public `unit`, `bool`, `i64`, record, and variant Run values using semantic declaration/member
  IDs rather than display names or private layout indexes;
- positive fuel/frame policy, checked aggregate live-cell policy, lazy branch and variant-arm
  execution, checked overflow traps, and daemon usability after ordinary semantic rejection or trap.

Structured authoring remains a typed proposal. Public `DraftSymbol` strings are transaction-local
labels, not identities. Omitted expression bindings and implied scaffolding stay private. The service
expands implied nodes deterministically, allocates all identities before edits, and validates the same
authoritative graph. Symbol spelling does not affect the candidate graph. The proposal is discarded
rather than persisted as a second program. Inline value-position expressions are not yet implemented.

## Representative applications

[`examples/release-channel`](../examples/release-channel/) is the retained equal-task replay for the
control-plane campaign. Its production public-path oracle creates the release policy, saves an
incomplete revision, proves invalid-repair allocation rollback, fills the placeholder without
identity churn, checks seven exact decisions and low-fuel laziness, renames the policy field, restarts,
and checks all three revisions. A fresh isolated coding agent independently completed the same task
through schema discovery and public binaries: 43/43 machine assertions passed, its initial symbolic
creation was accepted on the first attempt, and one later malformed query value shape required a
strict boundary correction. This is one controlled interface observation, not a model benchmark.

[`examples/named-data`](../examples/named-data/) is the focused Reading/Input oracle. It creates one
record and one variant type, obtains named-type repair context, rejects a type-invalid repair, fills a
placeholder without identity churn, checks every lazy variant arm and exact named Run input/output,
and proves incomplete and repaired revisions after restart through release binaries.

[`examples/job-policy`](../examples/job-policy/) is the broader current-semantics oracle. Through the
real generic CLI and service it creates:

- `Resources`, `Limits`, and nested `Job` records;
- `Target`, `Mode`, `RejectReason`, and `Decision` variants;
- seven functions using projection, calls, nested conditions, nested complete variant handling, a
  counted loop, and checked arithmetic;
- one reachable `i64` placeholder in `score`.

The workflow saves revision 1 incomplete, obtains sufficient repair context, rejects a `Decision`
constructor at the `i64` placeholder, uses two validate-only allocation probes to prove rollback,
fills the placeholder in revision 2 with the same identity, and observes an exact refinement diff. It
then proves accepted Linux/WebAssembly cases, CPU/memory/target/trust domain rejections, trusted
release acceptance, and selected `main = Decision.accept(25)` with exact named IDs. Low-fuel runs
with deliberately large scoring inputs prove unsupported-target and untrusted-release paths leave
accepted scoring work unselected. Revision 3 renames
`Resources.memory` to `memory_units` without changing identity, ordinal, type, layout, references, or
behavior. Restart preserves all three revisions, selected IDs, old/new names, and representative old
and current runs.

The application uses bounded structured authoring and selected returned bindings, adds no language
operation, and has no host effect. Exact node, operation, binding, artifact, and interaction counts
belong to [`docs/performance.md`](performance.md).

## Machine contract

`DescribeSchema` and local `lkjscript schema` derive one runtime machine contract from executable
closed definitions. The compact manifest advertises 38 exact lowercase roots; one request selects a
nonempty unique set of at most 16. Eighteen endpoint roots cover five control operations and every
query family. Each endpoint binds one of two shared protocol templates, exact selected leaf variants,
the shared boundary-error envelope and typed error, protocol/JSON versions, ID formats, and limits.
The query template defines the complete top request/response, batch/item/outcome, envelope, and error
layers once; explicit contextual parameters bind the endpoint's selected query/result pair. Templates
and bindings project executable broad descriptors rather than maintaining copied accepted shapes.
An iterative worklist returns every transitive named dependency once in lexical order, including both
`page` and its bound element type for `page<T>`. Root results
explicitly document list, optional, tuple, and page constructors. Unknown, duplicate, empty,
noncanonical, or excessive roots reject before digest-based `unchanged` handling. Explicit full
projection remains available. The active machine schema is `lkjscript-machine-schema-v5` with
protocol and JSON version 5. Artifact format 3 and `lkjscript-spg003` are unchanged. `LKJHEAD4`
directly replaces `LKJHEAD3`; its final unreleased grammar stores exact symbolic returned bindings
while the persisted idempotency fingerprint binds canonical JSON/v5 bytes.

The retained examples now request 12 operational endpoint roots covering workspace creation,
transaction application, eight relevant query families, Run, and shutdown rather than leaf payloads
or broad sections. Schema discovery is the bootstrap operation that obtains this task projection; its
endpoint root remains available separately. The production catalogue is built from the same executable
records, variants, typed draft descriptors, scalar domains, operation facts, and synthesized code
families used by closure completeness tests. Numeric control-plane descriptor tags and the section
DTOs are deleted. Full recomputation remains the oracle; there is no cache, compatibility reader, or
second schema authority.

## Memory and resource safety

The package keeps `[lints.rust] unsafe_code = "forbid"`. This campaign added no local unsafe
exception, project build script, foreign linkage, generated unsafe code, or dependency. Current accepted language
operations expose no raw pointer, arbitrary address, unchecked load/store, pointer arithmetic,
unchecked cast, byte reinterpretation, explicit deallocation, shared mutable heap, or foreign memory.
Current pure immutable values use independently verified layouts and bounded flat runtime cells.

That evidence does not prove the whole trusted computing base safe. Memory safety still trusts the
Rust compiler and standard library, Cargo/build tooling, operating system, filesystem/socket
implementation, CPU behavior, and resolved dependencies. Several resolved packages contain unsafe or
platform-specific internals and custom build targets. There is no formal proof or sandbox claim.

Resource exhaustion is separate from memory unsafety. JSON/frame/artifact/name bounds, runtime-value
depth/items/bytes, argument count, live cells, fuel, frames, query pages, response bytes, and service
timeouts are explicit operational policies with typed failure. User-scalable validation, deletion,
type-cycle checks, query composition, compilation, runtime calls, and aggregate conversion use
explicit work structures where applicable rather than user-depth native recursion.

The current flat-cell/copying model is an implementation for pure immutable values. No final heap,
garbage collection, reference counting, ownership, borrowing, region, handle, or hybrid strategy has
been selected. There are no resource-owning values, shared mutable objects, effects, host operations,
or concurrency semantics to which such a choice could yet apply.

## Evidence

The retained pre-campaign normal boundary reported 167 active passing tests and nine explicitly
ignored measurement or mutation-smoke tests. The final fresh all-target/all-feature campaign boundary
reported 165 active passes and nine ignored tests; all-target/all-feature Clippy, the optimized release
build, and diff check passed. The three production example drivers and the deterministic seed-1
10,000-case malformed-boundary release smoke also passed. Integration tests use the real
`lkjscriptd`, production framed JSON IPC, and generic CLI. Focused tests cover strict JSON framing and artifact decoding, operation/schema
coverage, stable identity and history, allocation rollback, validate-only parity, idempotency,
publication failure injection, query bounds/cursors, named layouts and values, compiler/Core IR
rejection, interpreter policies and traps, generated transaction sequences, restart/corruption, and
competing writers. A 10,000-node subtree exercise proves iterative validation/deletion; the retained
seed-1 10,000-case malformed-boundary release smoke covers artifact, framed JSON, and JSON byte
mutations and is not coverage-guided fuzzing.

Exact commands, environments, byte counts, artifact growth, timings, build observations, and the
claim boundary are retained in [`docs/performance.md`](performance.md).

## Exact limitations

The current verified baseline is stable Rust on Linux x86-64, one package, synchronous private local
IPC, one request per connection, immutable full-revision artifacts, full history, full snapshot
cloning, full validation/diff recomputation, full artifact rewrites, scan-based queries, one verified
Core IR, an explicit-frame interpreter, and flat cells for current values.

There is no source frontend, public network service, sandbox, package system, general collection,
generic type, effect or permission-value system, host I/O, resource-owning value, managed heap,
debugger, optimizer tier, native backend, database, journal, reverse index, cache, async runtime,
request concurrency, or cross-platform contract. These are current absences, not permanent
prohibitions. A concrete consumer, safety contract, measurement, preserved correctness oracle, and
direct cutover are required before selecting one.

The current machine contract is generated at runtime rather than committed as a file. The retained
12-endpoint projection returns 111 closed definitions in 80,629 compact bytes; the isolated agent's
slightly different 12-root selection returned the same definition count in 80,831 bytes. Both remain
operationally complete while smaller than the 124,430-byte explicit full contract and historical
86,009-byte six-section response. The 60-percent reduction was a planning target, not a correctness
shortcut.

The controlled after trial did not reduce initial-construction size or symbol count: its accepted
23,582-byte request used 115 symbols because it selected an allowed extra helper, while the exact
retained baseline graph uses 111 symbols in a 22,247-byte request. Inline single-use expressions
remain unimplemented because the evaluated prototype did not close validation and iterative
normalization obligations. Provider telemetry was unavailable to both isolated task trials; bytes are
not converted into token claims. Final command timings, binary sizes, test counts, production replays,
and the unavailable Miri component are recorded in performance evidence. No production readiness,
sandboxing, formal memory-safety proof, or performance leadership claim is made.
