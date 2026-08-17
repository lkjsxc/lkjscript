# Current status

Date: 2026-08-17

Campaign base: `59839070f83209155cd5b21d266efd967620736f` on `main`

## Implemented product path

The repository contains one Rust package and one active route from a coding agent to a saved and
runnable program:

```text
semantic workbench (preferred) or strict generic JSON diagnostic CLI
-> private protocol-v8 framed-JSON Unix IPC -> synchronous local service
-> durable workspace -> typed staged transaction -> immutable typed program-model revision
-> revision-bound scan query or direct Core IR lowering -> IR verifier -> ownership plan/verifier
-> managed interpreter
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
- immutable `bytes`, bounded literals, length, checked index and slice, content equality, pure
  concatenation, canonical public base64, and byte fields/payloads in named values;
- one identity-preserving `RefineHole` transition that fills a typed placeholder while retaining its
  Node ID, owner, body position, output zero, and incoming uses;
- deterministic semantic diffs that report `OperationRefined` and `Renamed` rather than identity
  churn;
- atomic commit and validate-only transactions, no identity consumption on rejection or
  validate-only, bounded symbolic proposal labels, anonymous one-use inline value expressions,
  selected receipts with at most 64 returned bindings, and commit-only idempotency;
- revision-bound query batches, paginated node/body/use/reference/dependency/diff/type facts, visible
  values, legal constructors, completeness blockers, owner chains, and bounded repair context;
- client-side context-packet version 1 with exact workspace/revision/schema/purpose binding, at most
  eight targets, at most 256 expanded nodes, deterministic `@n1` aliases, explicit omissions, a
  4 MiB boundary, and strict digest/schema/domain validation on every read;
- a deterministic one-way semantic review and exact semantic-diff rendering with optional full Node
  IDs, terminal-safe quoted content, explicit revision/digest facts, and no render/reparse editing;
- a bounded iterative compact edit/run plan parser that resolves packet aliases, distinguishes draft
  symbols from persistent identity, reports typed line/column failures, deserializes directly into
  the existing closed DTOs, and reaches the same transaction validator and Run route as raw JSON;
- immutable format-5 `.lkjscript` artifacts under semantic schema `lkjscript-spg005`, compact
  checksummed `LKJHEAD7`, contiguous history validation, restart, and strict corruption rejection;
- protocol and strict JSON version 8 with one request/response per private local connection;
- direct lowering of the complete selected-entry reachable definitions and named types to one private
  Core IR, followed by independent verification and one explicit-frame interpreter route;
- exact public `unit`, `bool`, `i64`, `bytes`, record, and variant Run values using semantic declaration/member
  IDs rather than display names or private layout indexes;
- compiler-derived managed-reference maps and ownership actions for calls, branches, loops, records,
  variants, returns, and traps, followed by verifier-owned recomputation before execution;
- a safe managed byte store with typed index-plus-generation handles, precise cycle-free ownership
  counts only for actual sharing, deterministic early reclamation, uniqueness-guided concat reuse,
  an allocate-new fallback and test oracle, separately checked live/cumulative metrics, owned result
  materialization, and daemon usability after ordinary semantic rejection or trap;
- an optional same-vocabulary line session that reuses one CLI process while keeping one daemon
  connection and one publication boundary per request.

`lkjscript agent` is the preferred coding-agent projection. `orient` and `help` provide compact
bootstrap facts; `create`, `context`, `view`, `validate`, `apply`, `diff`, and `run` implement the
explicit maintenance loop. The workbench is client-side and did not change protocol/JSON version 8,
artifact format 5, `lkjscript-spg005`, `LKJHEAD7`, or the v8 idempotency domain. Raw RPC JSON and
schema expansion remain the exact low-level diagnostic surface.

Structured authoring remains a typed proposal. Public `DraftSymbol` strings are transaction-local
labels, not identities. Complete, non-terminating, single-result regionless expressions may instead
appear anonymously in a value position. Holes and region-owning control remain explicit, as do
shared, selected, repairable, and maintenance targets. One iterative worklist flattens inline
children left-to-right before their parent; product fields and match arms use declaration order.
Every normalized expression still receives an ordinary persistent operation ID. Symbol spelling and
proposal nesting do not affect the candidate graph, and the proposal is discarded rather than
persisted as a second program.

## Representative applications

[`examples/agent-maintenance`](../examples/agent-maintenance/) is the sustained-maintenance and
workbench oracle. It constructs the job/release deployment policy through production binaries and
evolves it across eight immutable revisions: reachable incompleteness; rejected wrong-type repair;
identity-preserving valid repair; behavior extension; helper refactor; presentation rename; exact
overflow diagnosis and correction; immutable declaration replacement with mapped construction,
projection, inputs, outputs and calls; blocked then safe deletion; restart; multi-revision diff; and
old/current execution. Its exact results are `Decision.accept(25)` before extension,
`Decision.accept(27)` after extension and migration, `i64(0)` after the debug repair, and the exact
existing rejection alternatives for resource, platform, trust, and rollout failures. The driver
uses only public CLI/service boundaries and imports the retained job-policy payload builder rather
than duplicating a private graph fixture.

[`examples/binary-canonicalizer`](../examples/binary-canonicalizer/) is the ownership and byte-
construction consumer. Through production release binaries and one same-vocabulary CLI session it
discovers exact schema roots, creates a named byte record and result variant, saves a reachable byte
hole, obtains repair context, rejects an `i64` repair atomically, and refines the same identity to
`bytes_concat`. The repaired program checks marker `0xa5`, scans the payload, removes zero octets,
and carries an immutable byte accumulator through a counted loop and helper call. It proves empty,
all-padding, alternating, sparse, and dense output; lazy header rejection; fuel and bounds failures;
presentation-only rename; exact diffs; competing-writer rejection; dropped-response daemon health;
restart, old/current revisions, and corruption rejection. Under the full-result concat fuel and
cumulative-visible policy, dense payload 1,445 is the first accepted size and 1,446 the first
rejected size in this exact program.

[`examples/release-manifest`](../examples/release-manifest/) is the managed-bytes oracle. It builds
an exact 32-byte classifier using every retained byte operation and a bounded payload loop. Through
production release binaries it saves an incomplete payload-policy revision, obtains bounded repair
context, rejects a byte-for-boolean repair without publication or identity movement, validates and
commits a call refinement without identity churn, checks stable/preview acceptance and every typed
rejection, proves wrong-length laziness at fuel that exhausts the selected payload path, triggers an
exact index bounds trap, renames one reason, restarts, and runs old and current revisions. It has no
host parser or effect.

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
projection remains available. The active machine schema is `lkjscript-machine-schema-v8` with
protocol and JSON version 8, artifact format 5, semantic schema `lkjscript-spg005`, and `LKJHEAD7`.
The persisted idempotency fingerprint binds canonical JSON/v8 bytes. Old protocol, artifact, semantic
schema, HEAD, and fingerprint forms reject directly; no compatibility reader remains.

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
exception, project build script, foreign linkage, or generated native code. It added the locked
`base64` crate with default features disabled and only `std` enabled for strict canonical public
encoding. Current accepted language
operations expose no raw pointer, arbitrary address, unchecked load/store, pointer arithmetic,
unchecked cast, byte reinterpretation, explicit deallocation, shared mutable heap, or foreign memory.
Immediate and fixed immutable values use independently verified layouts and bounded flat runtime
cells. Bytes contributes one private 64-bit checked-handle cell while payload and constant-depth
views live in a managed store owned by one `Run`. Handles encode a typed slot and nonzero generation,
never serialize, and validate domain, kind, index, generation, liveness, and range before every
access. Reused slots advance generation and wrap retires the slot. Output bytes are independently
owned before store destruction.

`ownership.rs` derives exact managed-cell maps and compact borrow/share/transfer/drop/reuse facts
from verified Core IR. Its verifier separately recomputes type maps, local liveness, instruction and
edge actions, cleanup roots, and reuse eligibility; malformed plans reject before managed objects are
created. Managed reads borrow, last-use transfers add no count traffic, actual duplication creates a
checked non-atomic claim, and final drops reclaim views and backing. Records enumerate every managed
field and variants enumerate only the active payload. The accepted byte object topology is acyclic;
there is no tracing collector.

That evidence does not prove the whole trusted computing base safe. Memory safety still trusts the
Rust compiler and standard library, Cargo/build tooling, operating system, filesystem/socket
implementation, CPU behavior, and resolved dependencies. Several resolved packages contain unsafe or
platform-specific internals and custom build targets. There is no formal proof or sandbox claim.

Resource exhaustion is separate from memory unsafety. JSON/frame/artifact/name bounds, runtime-value
depth/items/bytes, decoded byte input, managed visible bytes, distinct retained backing, managed
backing/view object count, result bytes, argument count, live cells, fuel, frames, query pages, response bytes, and service
timeouts are explicit operational policies with typed failure. User-scalable validation, deletion,
type-cycle checks, query composition, compilation, runtime calls, and aggregate conversion use
explicit work structures where applicable rather than user-depth native recursion.

The ownership-managed route is the production default for nonescaping, cycle-free immutable bytes.
It reclaims dead values during execution and reuses a verified unique full-left concat buffer after
preflight; shared operands, live borrows, aliases, and partial views fall back to allocate-new. A
simple allocate-new invocation-store mode remains test-only as the differential oracle. There is no
finalizer, surface lifetime syntax, user-authored move/drop operation, general heap, or universal
reference count. Escaping values, lexical-region consumers, real cycles, and external resources are
separate future gates for persistent ownership, region inference, isolated tracing, and semantic
affine cleanup respectively.
There is no cooperative in-Run cancellation: disconnect leaves the bounded synchronous Run to finish
or trap and drop its store, while daemon termination relies on operating-system process reclamation.

## Evidence

The final all-target/all-feature campaign boundary reported 212 active passes and 10 explicitly
ignored measurement or mutation-smoke tests. All-target/all-feature Clippy, formatting, the optimized
release build, and diff check passed. All six production example drivers and the deterministic
seed-1 10,000-case malformed-boundary release smoke passed. Integration tests use the real
`lkjscriptd`, production framed JSON IPC, raw CLI, and semantic workbench. Focused tests cover strict JSON framing and
artifact decoding, operation/schema coverage, stable identity and history, allocation rollback,
validate-only parity, idempotency, publication failure injection, query bounds/cursors, named layouts
and values, compiler/Core IR and ownership-plan rejection, exact managed-reference maps, generation-
safe stale-handle rejection, deterministic reclamation, sharing, unique reuse and fallback,
allocation-failure rollback, interpreter policies and cleanup, generated transaction sequences,
restart/corruption, competing writers, packet determinism/locality, alias domain rejection, plan/JSON
normalization, parser bounds, and semantic review. A 10,000-node subtree exercise proves iterative
validation/deletion; the retained seed-1 10,000-case malformed-boundary release smoke covers artifact,
framed JSON, and JSON byte mutations. A separate seed-1 10,000-case smoke covers workbench plan,
packet, and alias mutations. Neither is coverage-guided fuzzing.

Stable Miri is unavailable, but installed nightly Miri passed six focused managed-store tests, two
ownership-plan tests, the interpreter cleanup test, and all seven focused workbench library tests. Nightly
AddressSanitizer with leak detection passed all 15 interpreter tests in the ownership campaign and
all seven focused workbench library tests in this campaign. The deterministic oracle-versus-ownership corpus passed 256 generated
cases with seed `0x6c6b6a7363726970`. No retained coverage-guided fuzzer or installed `cargo-fuzz`
command was available. A fresh equal-task protocol-v8 observation compared raw JSON with the final
workbench using installed Codex CLI 0.147.0 and explicit model `gpt-5.4`. Both black-box runs
published exactly the intended three revisions and passed the independent public oracle. The final
workbench run had zero unintended correction, three schema and three context requests, and exposed
provider input/cached-input/output/reasoning token classes of 473,927/437,888/8,230/3,110. The raw
run required four unintended protocol/request corrections and exposed
1,253,055/1,177,984/15,290/3,969. Price was not exposed. These are two controlled observations, not
a general model benchmark; exact trial method and caveats belong to performance evidence.

Exact commands, environments, byte counts, artifact growth, timings, build observations, and the
claim boundary are retained in [`docs/performance.md`](performance.md).

## Exact limitations

The current verified baseline is stable Rust on Linux x86-64, one package, synchronous private local
IPC, one request per connection (including session mode), immutable full-revision artifacts, full history, full snapshot
cloning, full validation/diff recomputation, full artifact rewrites, scan-based queries, one verified
Core IR, an explicit-frame interpreter, and flat cells for current values.
Byte payload is invocation-owned derived runtime state rather than another program authority.

There is no source frontend, public network service, sandbox, package system, general collection,
generic type, string, effect or permission-value system, host I/O, resource-owning value, general managed heap,
debugger, optimizer tier, native backend, database, journal, reverse index, automatic schema/context
cache, persistent client connection, candidate session, branch, merge, async runtime, request
concurrency, or cross-platform contract. Packet files may be saved by a client under their exact
digest, but are disposable and never trusted by the daemon. These are current absences, not permanent
prohibitions. A concrete consumer, safety contract, measurement, preserved correctness oracle, and
direct cutover are required before selecting one.

The current machine contract is generated at runtime rather than committed as a file. The retained
12-endpoint projection returns 112 closed definitions in 86,567 compact result bytes (86,645 bytes
as a production framed response). The explicit full result is 135,009 compact bytes (135,087 bytes
as a production framed response); manifest and unchanged results are 1,241/1,319 and 105/183 compact/
framed bytes. Relative to the sealed v7 baseline, the selected and full compact projections grew 740
and 1,235 bytes for concat, active versions, exact failures, and the private handle-cell descriptor.
Ownership actions, counts, generations, and reuse remain absent from the public schema. Bytes are not
converted into token claims.

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
interface evidence. A fresh v7 trial is recorded separately after the managed-bytes verification
boundary.

The fresh isolated protocol-v7 trial completed through production binaries and the retained session
interface without implementation contamination or repository edits. Its reported workspace flow
used 13 daemon requests, 4,298 compact request bytes, and 10,084 compact response bytes. It discovered
bytes, saved a reachable `bool` hole, rejected an `i64` repair with unchanged validate-only candidate
identity, refined the same hole to `bytes_equal`, inspected the exact revision diff, and returned the
external oracle results `LKJM -> true`, `LKJN -> false`, and `LKJ -> false`. The revision-current
confirmation had no unexpected failure or semantic correction. Provider token, cache, price, and
hidden-reasoning telemetry were unavailable. This remains one controlled observation, not a model
benchmark.

A disposable six-tool MCP adapter was tested outside the repository with installed Codex CLI 0.144.6
and was not retained. It preserved a small exact run oracle but added three processes, 2,084 bytes of
tool definitions, MCP traffic, startup schema reads, and an unresolved cancellation boundary without
reducing semantic calls, daemon connections, or forwarded request bytes. The later direct workbench
solves a different action/context problem without an adapter process or request vocabulary. There is
no adapter code, configuration, credential, or second semantic authority in the repository.
