# Current status

Date: 2026-08-16

Campaign base: `66e45c69143c1a1720e9ccc2b6682786f9475c8b` on `main`

## Implemented product path

The repository contains one Rust package and one active route from a coding agent to a saved and
runnable program:

```text
strict generic JSON CLI (optional) -> private protocol-v6 framed-JSON Unix IPC -> synchronous local service
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
  validate-only, bounded symbolic proposal labels, anonymous one-use inline value expressions,
  selected receipts with at most 64 returned bindings, and commit-only idempotency;
- revision-bound query batches, paginated node/body/use/reference/dependency/diff/type facts, visible
  values, legal constructors, completeness blockers, owner chains, and bounded repair context;
- immutable format-3 `.lkjscript` artifacts under semantic schema `lkjscript-spg003`, compact
  checksummed `LKJHEAD5`, contiguous history validation, restart, and strict corruption rejection;
- protocol and strict JSON version 6 with one request/response per private local connection;
- direct lowering of the complete selected-entry reachable definitions and named types to one private
  Core IR, followed by independent verification and one explicit-frame interpreter route;
- exact public `unit`, `bool`, `i64`, record, and variant Run values using semantic declaration/member
  IDs rather than display names or private layout indexes;
- positive fuel/frame policy, checked aggregate live-cell policy, lazy branch and variant-arm
  execution, checked overflow traps, and daemon usability after ordinary semantic rejection or trap.

Structured authoring remains a typed proposal. Public `DraftSymbol` strings are transaction-local
labels, not identities. Complete, non-terminating, single-result regionless expressions may instead
appear anonymously in a value position. Holes and region-owning control remain explicit, as do
shared, selected, repairable, and maintenance targets. One iterative worklist flattens inline
children left-to-right before their parent; product fields and match arms use declaration order.
Every normalized expression still receives an ordinary persistent operation ID. Symbol spelling and
proposal nesting do not affect the candidate graph, and the proposal is discarded rather than
persisted as a second program.

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
projection remains available. The active machine schema is `lkjscript-machine-schema-v6` with
protocol and JSON version 6. Artifact format 3 and `lkjscript-spg003` are unchanged. `LKJHEAD5`
directly replaces `LKJHEAD4`; its final unreleased grammar stores exact symbolic returned bindings
while the persisted idempotency fingerprint binds canonical JSON/v6 bytes.

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

The final fresh all-target/all-feature campaign boundary reported 172 active passes and nine explicitly
ignored measurement or mutation-smoke tests. All-target/all-feature Clippy, formatting, the optimized
release build, and diff check passed. The three production example drivers and the deterministic seed-1
10,000-case malformed-boundary release smoke also passed. Integration tests use the real
`lkjscriptd`, production framed JSON IPC, and generic CLI. Focused tests cover strict JSON framing and artifact decoding, operation/schema
coverage, stable identity and history, allocation rollback, validate-only parity, idempotency,
publication failure injection, query bounds/cursors, named layouts and values, compiler/Core IR
rejection, interpreter policies and traps, generated transaction sequences, restart/corruption, and
competing writers. A 10,000-node subtree exercise proves iterative validation/deletion; the retained
seed-1 10,000-case malformed-boundary release smoke covers artifact, framed JSON, and JSON byte
mutations and is not coverage-guided fuzzing. Miri was unavailable because the stable toolchain has
no `cargo-miri` component.

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
12-endpoint projection returns 111 closed definitions in 81,418 compact result bytes (81,493 bytes
as a production response). The explicit full result is 125,995 compact bytes (126,070 bytes as a
production response). The increase from the v5 results is 789 and 1,565 bytes respectively and
includes the recursive value shape, exact inline eligibility, nesting metric, and maintenance rule.

The equal-graph retained replay removes 44 of 111 explicit draft symbols. Its compact proposal falls
from 22,062 to 17,974 bytes and its framed initial request from 22,247 to 18,159 bytes while selected
bindings, created nodes, snapshot and artifact semantics, repair, history, restart, and runtime
oracles remain unchanged. Focused same-workspace tests prove byte-identical snapshots and artifacts
for arithmetic, calls, products, projections, and variants. Provider telemetry remains unavailable;
bytes are not converted into token claims. Final command timings, binary sizes, test counts,
production replays, and unavailable memory-safety tools are recorded in performance evidence. No
production readiness, sandboxing, formal memory-safety proof, or performance leadership claim is made.

A fresh isolated protocol-v6 coding-agent trial was attempted with only the allowed orientation files,
public help and schema discovery, and production binaries. It did not reach a semantic request because
its foreground daemon launch blocked the harness command; that exact daemon was stopped and its
temporary state was removed. Therefore no v6 fresh-agent success, request, error-rate, task-time, or
provider-telemetry claim is made. The retained equal-task replay is deterministic application evidence,
not a substitute model trial; the earlier completed protocol-v5 isolated trial remains historical
interface evidence.

A disposable six-tool MCP adapter was tested outside the repository with installed Codex CLI 0.144.6
and was not retained. It preserved a small exact run oracle but added three processes, 2,084 bytes of
tool definitions, MCP traffic, startup schema reads, and an unresolved cancellation boundary without
reducing semantic calls, daemon connections, or forwarded request bytes. The generic CLI is therefore
the only retained Codex programming surface; there is no adapter code, configuration, credential, or
second request vocabulary in the repository.
