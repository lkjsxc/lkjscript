# Current status

Status date: 2026-08-26 UTC. This file describes implemented checkout reality. Timings and sizes
from predecessor contracts are historical and remain in `docs/performance.md`.

## Maintained authorities and consumers

| Authority | Current inspected identity |
|---|---|
| `packages/standard` | repository `repo_c1358d64c351873b51c954b69d1ac988`; revision `rev_1eb0b76b547ea3a8e2fc02e4f3751da10c2994ec04645d04787894e5b1cffd64`; 12 modules |
| `applications/lkjournal` | repository `repo_95f988c5423fe3eb823c329ef0832d51`; revision `rev_e9cb9134841abe4505cbfc3cf5acd0a240fc1bd02477ea9d7ca9b7d389107ea2`; 3 modules; 2 targets |
| embedded standard package | semantic revision `rev_1eb0b76b547ea3a8e2fc02e4f3751da10c2994ec04645d04787894e5b1cffd64`; package artifact `artifact_45b945d9cab4250bdf4307ddca15d79e17336ff42f5db525a272c088411ca623`; bundle digest `artifact_63056fbb11375ffbd8192878fe09a5bc34c203ec0e58489336b2e39709a01694`; 22,276 bytes |
| current lkjournal build | root package artifact `artifact_7f0f4535e674484aaa80d8d0b899415eb9135d6719b2d09d2af6bde7abcea941`; bundle digest `artifact_81b49bf22d88839e854a85c5a9a8f261f69f4b6060ebc0e44b2eb6882125370a`; 178,791 bytes; exact 2-package closure |

These identities were inspected through the current release executable's `review`, `check`,
`package builtin inspect`, and `build` workflows. The table omits physical root-object identities
because those maintained public workflows do not emit them.

The standard package has 7 graph tests. The `lkjournal` root package has 5 graph tests and its
exact two-package closure runs 12; current direct `check` observations report
bytecode/reference equality. `lkjournal` binds the standard revision and package artifact above.
Its `serve` target selects `service.Web/request`; `work` selects `worker.Worker/run`.

The standard `core.identity<T>(value: T) -> T` function is a graph-authored explicit generic.
`lkjournal::service::json-response` calls it with explicit `Text`, and the binary-only command
template does the same. These are maintained abstraction consumers, not test-only Rust fixtures.

## Current contracts

The executable registry is the current owner of contract identities, versions, storage magic,
digest domains, operations, control models, limits, diagnostic classes, and security nonclaims.
See the generated [contract table](generated/contracts.md) and
[operation table](generated/operations.md). `lkjscript-dev check` verifies those bytes against the
release executable. This status file deliberately does not repeat the current version table.

Execution currently retains the independently implemented `bytecode_v1` and
`semantic_reference_v1` predecessor routes. Normalized compiler and execution implementations are
not yet the released path, so this file does not claim runtime cutover complete.

Unknown contracts and fields, malformed tagged identities, duplicate/noncanonical order, trailing
bytes, checksum mismatch, foreign IDs, and configured exhaustion reject at their owning boundary.
Graph-1/2/3 roots, root-storage-1 manifests, artifact-2/3 and package-object-1/2 bytes,
backup-1/2/3 bytes, transaction-2/3 requests, CLI-v2/3 routing, change-v1/2 requests, draft-v3
objects, and query-v1/2 requests and continuations are not current readers.

## Direct CLI and binary-only bootstrap

The current executable operation set is generated in [operations.md](generated/operations.md).

There is no universal command namespace and no compatibility aliases. Removed names such as the
old namespace, separate ID allocator, graph import, text projection aliases, and backup aliases
return `cli_usage`. `capabilities` owns the registry and schema digest; a known digest receives a
bounded unchanged response.

Normalized `new DEST --template minimal` creates current semantic authority in an absent or empty
safe directory using a private sibling stage and one visibility rename. It rejects the predecessor
command template. A copied executable can discover contracts, create a minimal normalized project,
read status, create meaning through compact records, plan/apply direct `rename.owner`, and inspect
the renamed exact owner without a repository checkout, Rust toolchain, network, or external
bootstrap artifact. The copied executable can also discard the allocation response, recover live
owner identities by canonical namespace or bounded enumeration, and inspect incoming and outgoing
committed relations.
Check, build, run, service, worker, package, history, draft, review, backup, restore, and doctor
still use predecessor authority and cannot consume that new project yet.

`doctor cleanup` implements retention contract 1 as a read-only inventory. Its policy roots are
HEAD's parent DAG plus every live draft and its base DAG. It reports retained-object counts,
reclaimable canonical-looking candidate counts and bytes, derived object counts and bytes,
unknown-entry counts, and an integrity-bound plan digest. It deliberately reports
`destructive_ready: false` and names missing revision-pin, active-reader-lease, and
registered-backup-root authority; it never deletes data.

Compact change contract `lkjscript-change-records-3` is the released change boundary. `change
plan` and `change apply --plan DIGEST` share the normalized typed lowering and publication path,
require an explicit revision, and return deterministic bounded records. The exposed subset has 13
semantic operations, 49 operation fields, 15 type forms, and 10 expression forms listed by focused
capabilities. One exhaustive typed descriptor inventory owns every operation's ordered fields,
required/optional classification, form token, and direct-form marker; decoding and registry
projection derive from it. Sixteen `change.field-form` records resolve all field form tokens, and
focused discovery enumerates the current visibility, function-effect, deletion-policy, selector,
and reference syntax.
Connected creation and flat expression edges allocate identities in one request; complete function
body replacement retires the prior expression/binding closure. The private authored JSON adapter
and generated schema were deleted, and predecessor JSON change requests reject without advancing
HEAD. An explicit bounded codec with fixed tags, big-endian lengths, and typed fixed-width
identities owns normalized request hashing. Request-local label spelling, operational budgets,
idempotency keys, and intent do not perturb durable allocations; the reviewed plan separately binds
the exact budget and operational options. Authored operation and list order remains
allocation-significant even when lowering stores a collection as keyed graph relations.
`change plan rename.owner --base REVISION --owner OWNER --name NAME` and the corresponding apply
form with the exact reviewed `--plan` accept one exact typed owner plus optional idempotency and
intent. This direct adapter and its equivalent compact record request converge to the same
transport-neutral typed request and publication options before plan comparison, repository access,
preparation, response rendering, or publication. The other 12 public operations remain
record-only.
Exact leaf-owner deletion requires `policy=reject`; it never infers an ownership closure. Reviewed
owned-closure deletion remains unsupported until impact output binds every removed owner and
relation. The executable registry publishes its exact required field forms. A leaf with live
references remains rejected unless the same reviewed request removes those exact relations.
Seven semantic preconditions are public: exact owner existence, absence, name, and parent;
namespace absence and exact binding; and exact dependency binding by package, semantic revision,
and logical package revision. Physical roots and encoded or derived object digests are not accepted
as caller intent. Parent guards read canonical meaning instead of derived ownership; present
namespace witness entries are checked against canonical owner meaning.

Normalized query contract `lkjscript-query-3` is the sole released `query` path. Its exhaustive
actions are `owners`, `find`, and `relations`; focused capabilities enumerate their options,
22 owner kinds, 10 namespace classes, 27 relation kinds, two directions, limits, response fields,
selector fields, and continuation metadata. All three operations open one current immutable
`GraphRepository` view. Owner pages read canonical owner bindings, exact find uses the committed
namespace witness as a bounded locator and revalidates canonical meaning, and relation pages read
the committed forward or reverse relation witness. Results are compact, deterministic, and bound
to the exact observed revision.

Owner and relation pages use stateless `qcont_` logical continuations bound to repository,
package, revision, operation, normalized selector, and exclusive logical resume key. Limits may
change between pages without changing selection. Public query has separate item and output-byte
limits and reports map pages/bytes/entries, catalog lookups, store objects/bytes, canonical records,
witness records, and rendered output bytes separately. It never reconstructs the complete graph,
rebuilds an index, persists a continuation, or writes repository data. Removed callers/callees,
types, capabilities, context, impact, JSON request/file forms, and predecessor continuations reject
without fallback. Context traversal, generic impact, fuzzy search, and historical query remain
unimplemented.

## Persistent root, validation, and derived state

The accepted physical root is a bounded manifest over six immutable canonical Merkle radix maps:
modules by stable ID, module names to IDs, dependencies by package ID, dependency aliases to
package IDs, targets by stable ID, and typed tombstones. Exact root updates path-copy changed
branches and structurally share equal pages. The overlay retains every generated path page, even an
exact physical reuse, and final extraction traverses only generated pages reachable from changed
map roots. Unchanged map roots and accepted-base subtrees are not traversed during ordinary local
publication. Publication writes delta-selected changed module objects and required generated pages.
Backup, restore, artifact reconstruction, corruption tests, and deep doctor understand graph-4
roots and storage-2 pages. Deep doctor remains the exhaustive detector for damage in an untouched
accepted-base subtree.

This physical cutover is not a complete incremental semantic engine. Four precondition-free
transaction classes have local preparation. Eligible pure-function body replacement uses profile
`incremental_pure_body_slice`: it resolves exact owners, loads the selected modules and their
recursive local import dependencies, validates that slice, and publishes only changed target
modules, removed nested-identity tombstones, and persistent-root paths. Structurally different
bodies remain eligible. The focused differential test checks the resulting root and accepted graph
against complete canonicalization and validation.

Independent empty-module creation uses `incremental_independent_module_create`; module rename uses
`incremental_module_rename`, validates only renamed modules and their outgoing import dependencies,
and does not rewrite importers or targets. Declaration rename uses
`incremental_declaration_rename`, validates only owning modules plus outgoing imports, and leaves
every exact-ID importer object unchanged. Canonical imports bind exact package/module IDs, targets
bind exact module/component/port IDs, and types and value references bind exact
package/module/declaration IDs.

Every request with preconditions and every other change class still
calls `reconstruct_current`, clones the complete logical root and module vector, rebuilds canonical
relations, and fully validates the candidate under profile `prepared_once_full_oracle`. Prepared
publication removes duplicate write-lock validation in both paths. A missing exact-index generation
makes a local transaction widen to complete preparation; it does not narrow validation.

The current semantic-summary contract implements integrity-bound module signatures,
implementation/effect digests, and typed dependency facts. The current semantic-fact contract
persists exact summary bindings,
test owners, and flat typed reverse edges in three Merkle maps. Codec, corruption, rebuild,
delta/full equality, private/public classification, and bounded-frontier tests exist. Every
revision-4 core authenticates the exact fact roots with a revision-independent certificate. Local
preparation path-copies changed fact branches; missing or malformed cache state rebuilds from
canonical modules, while a rebuilt certificate mismatch is corruption. This is not general
frontier-driven validation: only the four transaction classes above use local preparation. The
complete validator and packed reconstruction remain the trusted oracles.

The predecessor exact owner/name index v3 and broad relation index remain private disposable state
for exact out-of-scope workspace, semantic diff, legacy inspect, semantic change/transaction, and
predecessor repository consumers. The local transaction profiles update touched exact-index shards;
the broad index remains revision-bound and lazy. Missing or corrupt private index state may rebuild
without changing meaning. Neither index is advertised by or reachable from normalized public query,
and neither is required for its correctness.

Build reconstructs the exact package closure and produces a deterministic graph-native artifact.
There is no incremental compiler-unit cache yet. Deep doctor is exhaustive over retained
revision/root/page/module/receipt bindings up to its history bound, but does not rerun full
cross-package semantic validation for every historical revision or walk dependency artifact and
draft closure.

## Language, compiler, runtime, and applications

Meaning graph 4 represents packages, modules, records, variants, interfaces, closed externs,
constants, pure/task functions, components, ports, requirements, targets, tests, documentation,
annotations, bindings, expressions, explicit type parameters, and typed relations.

Canonical named types, calls, function values, constants, capability requirements, and exports use
exact package/module/declaration references. Constant references are distinct from lexical
variables. Names remain mutable presentation and namespace entries rather than reference authority.

Pure functions and closed externs support ordered rank-1 type parameters with stable identities.
Calls and named function values require exact explicit type arguments; `invoke` applies the
resulting monomorphic function value. Validation recursively substitutes types and rejects generic
task functions and polymorphic recursion that changes type arguments. Bytecode and reference
execution implement the slice and agree in maintained tests.

There are no generic constraints, type-argument inference, higher-rank values, lexical lambdas,
closure capture, floating point, sets, user scheduler primitives, or dynamic evaluation. Function
values are named and have no captured environment.

The same prepared component/port model feeds tests, commands, resident plaintext HTTP, and workers.
PostgreSQL, configuration, redacted secrets, clock, randomness, UUID, Argon2, streams,
memory/local/S3 objects, durable queues, HTTP, and workers remain generic typed adapters.
`lkjournal` routes, SQL, authorization, representation, object, and queue policy remain graph
meaning.

## Direct-cutover absence and security boundary

The maintained tree contains no editable `.lkj` program authority, package source descriptor,
source publication, active graph-1/2 reader/writer, profile runtime, product-specific native binary,
private maintained application builder, compatibility edition, fallback alias, or Lean material.
Source-era markers exist only for exact predecessor rejection. The parser/source semantic builder
remains Rust-test oracle material and has no public application-development route.

The runtime is not a hostile-code sandbox or multi-tenant isolation boundary. Linux x86-64 is the
only verified bootstrap/service platform. The HTTP server is plaintext and PostgreSQL uses
`NoTls`. lkjscript does not plan HTTP TLS termination, PostgreSQL TLS, certificate parsing or
management, certificate issuance/rotation, ACME, or speculative TLS hooks. Encrypted transport
requires an external trusted boundary or a different adapter outside current scope.

## Verification execution

The contributor-only `lkjscript-dev check` command owns an explicit gate dependency DAG, runs independent ready nodes with a
bounded worker count, and stops descendants of failed prerequisites while retaining independent
evidence. Reusable passes bind exact gate command, dependency evidence, tracked and relevant
untracked content, `Cargo.lock`, compiler/tool/platform/environment identity, required outputs,
timeout, and log policy. Missing, stale, malformed, or digest-inconsistent cache records miss or
reject; receipts label `fresh` and `reused`. The `full` profile bypasses reuse and fails if any
pass is not fresh. Gates that write a debug executable precede gates that launch it; independent
release and non-Cargo work may still run in parallel. Default success remains one aggregate result
plus a retained receipt.

The semantic-fact cutover content passed 17/17 fresh gates with no reuse in 116.312 seconds at
`.artifacts/check/20260822T152519.342035Z-853126/receipt.json`; exact staged content repeated the
17/17 policy in 6.376 seconds at
`.artifacts/check/20260822T152856.487020Z-858100/receipt.json` and was committed without content
changes as `8ec09e24efc9968d900cfd3a4fa9ef63035a06d8`. The final handoff, rather than this mutable
status document, owns the later checkpoint-commit and fresh-checkout receipt identities.

Normalized query verification includes a copied release executable that creates and changes a
normalized project, discards allocation output before rediscovery, pages owners and relations,
renames while preserving identity, rejects stale and foreign continuations and predecessor input,
and proves query leaves HEAD and the repository content inventory unchanged. A separate
10,000-owner/9,999-relation release test retains exact locality and resource observations in
`docs/evidence/20260826-normalized-query-scale-10000.json`; it observed zero repository bytes
written and 77,660 KiB peak RSS for the test process. This is one fixture, not a distribution.

## Current limits and unproved properties

- The stored root has no explicit module-count field ceiling, but complete logical
  reconstruction/canonicalization and monolithic package artifacts still impose implementation and
  memory pressure. No one-million-owner complete workflow has been retained.
- A module object is bounded to 64 MiB, 100,000 declarations, 2,000,000 retained identities, and
  expression depth 256. These are current representation/decoder limits, not demonstrated language
  scalability.
- A transaction defaults to 1,000 operations, 1,000,000 work, and 10,000 affected owners; hard
  request budgets are 10,000, 10,000,000, and 100,000 respectively. Change files are bounded to
  16 MiB and finite CLI output to 4 MiB.
- The current executable artifact is monolithic and bounded to 128 MiB; artifact closure is
  bounded to 1,024 packages. Backup contract 4 is a segmented directory: a bounded manifest and
  bounded index segments bind canonical object files, which backup and restore copy and verify one
  at a time while retaining an O(object-count) sorted key set in memory. Backup history traversal
  is bounded to 10,000 revisions. No authority larger than the predecessor 128 MiB bundle has yet
  been retained as scale evidence. Restore verifies every entry and deep retained structure before
  visibility but does not rerun the complete cross-package semantic validator.
- Public normalized owner/relation pages default to 50 items and 64 KiB and allow at most 10,000
  items and 4 MiB. Compact output has a 1,536-byte minimum sufficient for its fixed envelope;
  `qcont_` tokens are bounded to 320 bytes. These are public operation boundaries, not project
  count ceilings.
- Private predecessor query-index single objects are bounded to 128 MiB, 2,000,000 owners, and
  10,000,000 relations. Semantic-fact keys and values use the persistent-map 256-byte/48-KiB
  boundaries and 64-KiB hostile page decoder bound; these are physical object boundaries for
  out-of-scope consumers, not normalized public query limits.
- Imports, targets, exports, and declaration references are exact-ID bound; module and declaration
  rename are local. Declaration move is not a concise change-v3 form and exact references still
  carry their owning module ID, so no declaration-move locality claim is made.
- Merge conflicts are returned as bounded nonpublishing results; persistent conflict drafts and
  typed conflict resolution are not implemented.
- CLI discovery names every current high-level change, top-level type, concise expression, owner
  kind, and relation role, but does not yet emit the complete nested JSON schema for those forms.
- Canonical history has no public retention pruning, garbage collection, or immutable pack
  compaction. The read-only retention preview cannot authorize deletion because pins,
  active-reader leases, and registered backup roots are absent. One-file-per-module/page/version
  behavior has no current million-owner comparison.
- No stdio agent session is implemented. Standalone commands remain correct. Provider token,
  cached-token, retry, and monetary telemetry is unavailable; output bytes are not token or cost
  estimates.
- One commit-bound graph-4/fact-3 10,000-background-module public workflow and one normalized-query
  10,000-owner/9,999-relation locality fixture are retained. Neither is a distribution. A
  100,000-module attempt produced no retained result after its execution output became unavailable,
  and no million-owner workflow was run. Complete-workflow RSS, CPU, fsync, topology breadth, and
  long-history evidence remains incomplete. Historical graph-1 rows are labeled in
  `docs/performance.md`.
