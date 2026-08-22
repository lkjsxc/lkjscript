# Current status

Status date: 2026-08-22 UTC. This file describes implemented checkout reality.

## Maintained graph authorities

| Authority | Exact current identity |
|---|---|
| `packages/standard` | repository `repo_c1358d64c351873b51c954b69d1ac988`; revision `rev_2fd85a82b827f2d1b60ef1c474831121fc4d968f9df10df04488cf77c3a2772d`; 12 modules; 6 tests |
| `applications/lkjournal` | repository `repo_95f988c5423fe3eb823c329ef0832d51`; revision `rev_441156fb5408ad27352d91a8e8ac42f60cd6e2704ee9b42f58a01cc64a6ef218`; 3 modules; 2 targets; 11 tests |
| standard package artifact | `artifact_4f7957f6f76520647164247059b5593120f03596915975b2d9f9087428fcdef9`; checked bundle 20,350 bytes |
| lkjournal artifact bundle | `artifact_b90c24ab397d03d641b8b97cddf056a1ad738f8f3e82a63628a36f9db1ca4979`; 160,195 bytes |

The canonical standard store is 21,062 bytes across 16 transportable files. The canonical
lkjournal store is 160,419 bytes across 8 transportable files. Disposable indexes, drafts, and lock
files are excluded. Both maintained histories were deliberately re-rooted at their normalized
graph snapshot during direct cutover; source-era revision ancestry is not current history.

`lkjournal` binds the exact standard revision and package artifact. Its `serve` target selects the
`service.Web/request` HTTP port and its `work` target selects the `worker.Worker/run` worker port.
Both package test suites report bytecode/reference differential equality.

## Current contracts

| Domain | Current contract |
|---|---|
| Meaning graph | `lkjscript-meaning-graph-1`, version 1 |
| Revisions and transaction receipts | packed version 1 |
| Semantic transactions, drafts, diff, merge, query | version 1 |
| Public semantic CLI | version 2, strict JSON |
| Graph artifact bundle | version 2 with package objects version 1 |
| Canonical backup | version 1 |
| Deployment and capability grants | version 1 |
| Execution | `bytecode_v1` production and `semantic_reference_v1` oracle |

Unknown contracts and fields, malformed tagged identities, duplicate or noncanonical order,
trailing bytes, checksum mismatch, foreign IDs, and configured excess reject at their decoding or
semantic boundary.

## Implemented semantic development

The executable command registry provides status, orientation, selected schema, typed ID allocation,
dependency staging, owners/find/show, references/callers/callees/type/capability traversal,
context, impact, closed query, semantic diff, three-way merge, plan/validate/apply, draft lifecycle,
targets, build/test/run, artifact inspection, deterministic text projection, history/revision show,
deep doctor, backup/export, restore, and graph-artifact import.

Queries are revision-pinned, deterministically ordered, projected, budgeted, and continuable.
Exact ID/name queries use local 256-way owner/name shards and load one canonical module only when a
body is requested. Broad relations use a full revision-bound index. Production results are checked
against canonical reconstruction in tests; all derived indexes rebuild on loss or corruption.

Transactions support package metadata/dependencies; module create/rename/delete; declaration
create/replace/rename/move/delete/clone/restore; record fields; variant cases; interface operations;
function signatures and bodies; exact expression replacement and reference rebinding; binding
rename; test expectation replacement; target create/delete; preconditions; idempotent replay; and
atomic ordered batches. Rename and move preserve stable IDs. Deletion records typed tombstones and
nonreuse is validated. Normal CLI responses inline at most 64 affected owners.

Drafts retain bounded operation deltas, holes, diagnostics, conflicts, base revision, generation,
and intent outside accepted authority. They cannot build, run, deploy, or publish until fully
resolved and validated. Three-way merge detects stable-owner add/remove/rename/move/modify conflicts
and publishes only a conflict-free exact-base result.

## Language, compiler, and runtime

The graph represents packages, modules, imports/exports, records, variants, interfaces, closed
externs, constants, pure and task functions, components, ports, capability requirements, targets,
tests, documentation, annotations, patterns, bindings, expressions, and typed relations.

The value/type surface includes unit, bool, checked i64, bytes, text, `StaticText`, opaque
resource/secret values, nominal and structural records, variants, lists, ordered maps, option,
result, streams, and functions. Evaluation order, capability use, lexical transactions, checked
arithmetic, and collection bounds remain explicit. No implicit coercion, floating point, generics,
closure capture, or user-visible scheduler primitive is implemented.

Graph objects lower directly into deterministic exact-closure package artifacts. The same prepared
component ports feed pure tests, command execution, resident HTTP, and workers. PostgreSQL,
configuration, redacted secrets, clock, randomness, UUID, Argon2, streams, memory/local/S3 objects,
durable queues, HTTP, and workers remain generic typed adapters. Application policy is not native
Rust.

## Direct-cutover absence

The maintained tree contains no `.lkj` program modules, `lkjscript.package.json`,
`.lkjscript/source-v1`, source apply/formatting publication, source-derived declaration identity,
active predecessor graph store, profile artifacts/runtimes, product-specific native binary,
private standard/lkjournal builder, dual reader, dual writer, compatibility edition, fallback
alias, or Lean material. Source-era markers are recognized only to return an exact predecessor
rejection when no current graph exists.

The parser and source semantic builder remain test-only independent fixtures. They cannot open or
mutate a maintained project and are not linked as a public authoring workflow.

## Current limits

- Accepted transaction validation reconstructs and validates the complete candidate twice. It is
  correct but is not yet an incremental semantic engine; final 90,000-module batch latency exposes
  this growth.
- Broad index creation reconstructs the current graph. Exact warm lookups are local afterward, but
  cold orientation still loads the root and all module references.
- Graph contract 1 permits at most 100,000 modules, 10,000 operations per transaction, a 64 MiB
  packed module, and a 128 MiB packed global object. A one-million-module fixture therefore rejects
  under current policy and was not executed.
- Canonical history has no public garbage collection, pruning, or segment repacking command.
- Merge conflicts are returned as a closed nonpublishing result; persisting that result into a
  conflict draft and a dedicated conflict-resolution command are not implemented.
- The generic transaction protocol is the complete writer, but several catalog convenience names
  such as extract, inline, change-signature, add-field, conflict-show, receipt-show, and query-save
  are not separate commands.
- Dependency staging is operational and exact but has no automatic registry transport.
- No semantic daemon is implemented; every command is stateless-correct.
- Provider input-token, cached-token, retry, and monetary telemetry is unavailable. Output bytes
  are not used to claim token or cost savings.
- The runtime is not a hostile-code sandbox. Linux x86-64 is the only verified bootstrap and
  service platform. HTTP TLS termination, PostgreSQL TLS, and distributed guarantees are absent.
