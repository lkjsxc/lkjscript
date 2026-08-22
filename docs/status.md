# Current status

Status date: 2026-08-22 UTC. This file describes implemented checkout reality. Timings and sizes
from predecessor contracts are historical and remain in `docs/performance.md`.

## Maintained authorities and consumers

| Authority | Current inspected identity |
|---|---|
| `packages/standard` | repository `repo_c1358d64c351873b51c954b69d1ac988`; revision `rev_af36c21e869a22a992b982aafe959c6230311293094e9ded162e29872ce0afdf`; root `root_object_61f185e6332b885353acf6312c779369bcca9ca82acc5141b9beb4bcc2e1aeeb`; 12 modules |
| `applications/lkjournal` | repository `repo_95f988c5423fe3eb823c329ef0832d51`; revision `rev_583079ff88a142c5a8553bb7fd3beffeda8e7d181651370cb6322819eb9f5dfc`; root `root_object_1d2e3529202868c508db87b475ac41b31d6241a01c791ac1db47fb0b1a4e7090`; 3 modules; 2 targets |
| embedded standard package | semantic revision `rev_af36c21e869a22a992b982aafe959c6230311293094e9ded162e29872ce0afdf`; package artifact `artifact_cef17b4730c708a9e3dfdaa934af28fad58902fb011db1e1305fd840f459c57a`; bundle digest `artifact_b2f39efc64b987378a6abcb81ade2f14de354ace122dbea22f02a984de875cea`; 22,259 bytes |
| current lkjournal build | root package artifact `artifact_231583fc727b1ce12854227f2031ed62332ef94eb7b7c6dfe58487047c94dcfd`; bundle digest `artifact_3ea0c5e71f763319514a6747e580b02c02efbf7e35086420b9bfed74e3cd0444`; 178,756 bytes; exact 2-package closure |

The standard package has 7 graph tests. The `lkjournal` root package has 5 graph tests and its
exact two-package closure runs 12; current direct `check` observations report
bytecode/reference equality. `lkjournal` binds the standard revision and package artifact above.
Its `serve` target selects `service.Web/request`; `work` selects `worker.Worker/run`.

The standard `core.identity<T>(value: T) -> T` function is a graph-authored explicit generic.
`lkjournal::service::json-response` calls it with explicit `Text`, and the binary-only command
template does the same. These are maintained abstraction consumers, not test-only Rust fixtures.

## Current contracts

| Domain | Current contract |
|---|---|
| Meaning graph | `lkjscript-meaning-graph-4`, version 4 |
| Physical accepted root | `lkjscript-persistent-root-2`, storage version 2 |
| Revisions and receipts | versions 3 and 3 |
| Public CLI | version 4, strict JSON |
| Public change and internal transaction | versions 3 and 4 |
| Query / broad index / local exact index | versions 2 / 2 / 3 |
| Draft, semantic diff, and merge | versions 4, 2, and 2 |
| Semantic summary / validator identity | `lkjscript-semantic-summary-2` / `lkjscript-semantic-validator-2` |
| Executable artifact / package object | versions 4 and 3 |
| Backup and bootstrap | versions 4 and 2 |
| Read-only retention preview | version 1 |
| Deployment | version 1 |
| Execution | `bytecode_v1` and `semantic_reference_v1` |

Unknown contracts and fields, malformed tagged identities, duplicate/noncanonical order, trailing
bytes, checksum mismatch, foreign IDs, and configured exhaustion reject at their owning boundary.
Graph-1/2/3 roots, root-storage-1 manifests, artifact-2/3 and package-object-1/2 bytes,
backup-1/2/3 bytes, transaction-2/3 requests, CLI-v2/3 routing, change-v1/2 requests, draft-v3
objects, and query-v1 requests are not current readers.

## Direct CLI and binary-only bootstrap

CLI v4 has 17 direct top-level commands:

`capabilities`, `new`, `inspect`, `query`, `change`, `draft`, `history`, `package`,
`check`, `build`, `run`, `serve`, `worker`, `review`, `backup`, `restore`, and
`doctor`.

There is no universal command namespace and no compatibility aliases. Removed names such as the
old namespace, separate ID allocator, graph import, text projection aliases, and backup aliases
return `cli_usage`. `capabilities` owns the registry and schema digest; a known digest receives a
bounded unchanged response.

`new DEST --template minimal|command` creates graph-4 authority in an absent or empty safe
directory using a private sibling stage and one visibility rename. The command template uses the
embedded exact standard artifact and the same change-v3 lowering/publication machinery as later
changes. A copied executable can inspect/export the built-in package, create, inspect, change,
check, build, run, back up, restore, and deep-doctor without a repository checkout, Rust toolchain,
network, or external bootstrap artifact. Black-box `tests/public_cli.rs` covers this workflow,
conflicts, corruption, local identities, exact reproduction, and predecessor-command rejection.

`doctor cleanup` implements retention contract 1 as a read-only inventory. Its policy roots are
HEAD's parent DAG plus every live draft and its base DAG. It reports retained-object counts,
reclaimable canonical-looking candidate counts and bytes, derived object counts and bytes,
unknown-entry counts, and an integrity-bound plan digest. It deliberately reports
`destructive_ready: false` and names missing revision-pin, active-reader-lease, and
registered-backup-root authority; it never deletes data.

Change v3 exposes 13 high-level forms: dependency add/replace/remove; module, record, variant, pure
function, component, test, and target creation; module/declaration rename; and body replacement.
Typed request-local symbols allocate stable identities deterministically and return the complete
map. A common commit prepares its selected validation path once, then publication rechecks its
exact base, result, delta, changed modules, and facts instead of repeating semantic validation.

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

Semantic summary contract 2 implements integrity-bound module signatures, implementation/effect
digests, typed dependency facts, rebuildable reverse dependencies, private/public change
classification, and an invalidation frontier. Codec, corruption, rebuild, and frontier tests exist.
Content-addressed summaries and a revision-bound reverse index are persisted under disposable
index storage. Every revision core authenticates the exact fact set with a revision-independent
semantic certificate. Local preparation delta-updates and rebinds that index; missing cache bytes
rebuild from canonical modules, while a certificate mismatch is corruption. This is not general
frontier-driven validation: only the four transaction classes above use local preparation. The
complete validator and packed reconstruction remain the trusted oracles.

Query indexes remain disposable. Exact owner/name index v3 uses revision-independent,
content-addressed shards and one revision/root-bound manifest containing 256 owner and 256 name
shard slots. The four local transaction profiles update only touched shards and reuse every
unchanged digest; a body-only edit writes no new exact-index object. Initial and full-candidate
publication seed the exact generation from graph values already in memory. Missing/corrupt state
rebuilds without changing meaning. The broad relation index remains revision-bound and lazy;
building it reconstructs canonical modules, accepted changes do not delta-update it, and project
orientation still reconstructs all logical root entries.

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

`tools/check` now owns an explicit gate dependency DAG, runs independent ready nodes with a
bounded worker count, and stops descendants of failed prerequisites while retaining independent
evidence. Reusable passes bind exact gate command, dependency evidence, tracked and relevant
untracked content, `Cargo.lock`, compiler/tool/platform/environment identity, required outputs,
timeout, and log policy. Missing, stale, malformed, or digest-inconsistent cache records miss or
reject; receipts label `fresh` and `reused`. The `full` profile bypasses reuse and fails if any
pass is not fresh. Gates that write a debug executable precede gates that launch it; independent
release and non-Cargo work may still run in parallel. Default success remains one aggregate result
plus a retained receipt.

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
- Query-index single objects are bounded to 128 MiB, 2,000,000 owners, and 10,000,000 relations.
  Semantic reverse-index codec bounds are explicitly implementation/hostile-input bounds and
  require sharding before they can constrain a legitimate project.
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
- Current graph-4 performance, RSS, I/O, fsync, dense-fanout, long-history, and million-owner
  evidence is incomplete. Historical graph-1 rows are labeled in `docs/performance.md`.
