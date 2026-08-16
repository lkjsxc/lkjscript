# Current status

Date: 2026-08-16

Campaign base: `456ef91b692336ce0e8eaafc49bdf61d84a2db44` on `main`

## Implemented product path

The repository contains one Rust package and one active route from a coding agent to a saved and
runnable program:

```text
strict generic JSON CLI (optional) -> private protocol-v4 Unix IPC -> synchronous local service
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
  validate-only, compact bounded receipts, at most 64 selected returned bindings, and commit-only
  idempotency;
- revision-bound query batches, paginated node/body/use/reference/dependency/diff/type facts, visible
  values, legal constructors, completeness blockers, owner chains, and bounded repair context;
- immutable format-3 `.lkjscript` artifacts under semantic schema `lkjscript-spg003`, compact
  checksummed `LKJHEAD3`, contiguous history validation, restart, and strict corruption rejection;
- protocol and strict JSON version 4 with one request/response per private local connection;
- direct lowering of the complete selected-entry reachable definitions and named types to one private
  Core IR, followed by independent verification and one explicit-frame interpreter route;
- exact public `unit`, `bool`, `i64`, record, and variant Run values using semantic declaration/member
  IDs rather than display names or private layout indexes;
- positive fuel/frame policy, checked aggregate live-cell policy, lazy branch and variant-arm
  execution, checked overflow traps, and daemon usability after ordinary semantic rejection or trap.

Structured authoring remains a typed proposal. The service expands implied regions, blocks, block
arguments, and terminators in deterministic depth-first order, allocates all identities before edits,
and validates the same authoritative graph. The proposal is discarded rather than persisted as a
second program.

## Representative applications

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
closed definitions. The default manifest is compact; callers may request any nonempty unique subset
of seven broad sections, the full contract explicitly, or an `unchanged` result for a matching
lowercase fingerprint. The active machine schema remains `lkjscript-machine-schema-v4` with protocol
and JSON version 4. Artifact format 3, `lkjscript-spg003`, and `LKJHEAD3` are unchanged.

The job-policy baseline still needs the same six task-relevant sections as the focused named-data
workflow. Their measured size is material interaction cost, but current evidence does not justify
another projection vocabulary: a selected-definition closure would require new
contract dependency machinery, and narrower groups or exact variants would add discovery and test
surface while this workflow uses most existing families. The current sections, explicit full result,
and fingerprint reuse remain the one active interface; no prototype or compatibility path remains.

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

The complete normal boundary reports 167 active passing tests and nine explicitly ignored
measurement or mutation-smoke tests. Integration tests use the real `lkjscriptd`, production binary
IPC, and generic CLI. Focused tests cover strict JSON/binary/artifact decoding, operation/schema
coverage, stable identity and history, allocation rollback, validate-only parity, idempotency,
publication failure injection, query bounds/cursors, named layouts and values, compiler/Core IR
rejection, interpreter policies and traps, generated transaction sequences, restart/corruption, and
competing writers. A 10,000-node subtree exercise proves iterative validation/deletion; the retained
seed-1 10,000-case malformed-boundary release smoke covers artifact, binary protocol, and JSON byte
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

The current machine contract is generated at runtime rather than committed as a file. The six-section
job-policy response remains large. The deterministic driver did not ask a model to author the program,
so controlled model success through the public interface remains unmeasured. Supplemental
provider-reported telemetry for this campaign's parent coding-agent session is retained separately;
bytes are not converted into token claims. No production readiness, sandboxing, formal memory-safety
proof, or performance leadership claim is made.
