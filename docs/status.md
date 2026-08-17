# Implemented status

This file describes the current checkout. Normative behavior belongs to `docs/spec/`; measurements
and architecture decisions belong to `docs/performance.md` and `docs/architecture.md`.

## Product boundary

`lkjscript` currently provides a local typed semantic programming engine for pure deterministic
programs. A coding agent can create a workspace, inspect bounded exact context, render or author an
editable semantic document, validate or publish an immutable revision, review semantic change, and
compile and run an exact historical revision.

The supported bootstrap is stable Rust edition 2024 on Linux x86-64. The repository forbids local
unsafe Rust. It is not a sandbox or a production deployment system.

## Active versions

| Contract | Active value | Older forms |
|---|---|---|
| logical protocol / strict JSON | 9 | reject |
| machine contract | `lkjscript-machine-schema-v9` | reject |
| workbench | 2 | reject |
| context packet | 2 | reject |
| editable semantic document | 1, root `document` | `plan` rejects |
| semantic artifact | 6 | reject |
| semantic schema | `lkjscript-tsm006` | reject |
| artifact magic | `LKJTSM\0\x06` | reject |
| HEAD | `LKJHEAD8` | reject |

There is no compatibility reader, migration mode, edition split, or silent fallback.

## Primary workflow

The production CLI opens the topology-neutral `Engine` directly under one state-directory authority
lock. No foreground daemon must be launched. `lkjscript session` retains one direct engine for a
line-delimited bounded batch. `lkjscriptd` remains an optional private Unix-socket diagnostic adapter
that calls the same engine.

The preferred agent commands are:

- `agent orient` for stable orientation and the embedded machine-contract digest;
- `agent create` for a new workspace;
- `agent context` for an exact purpose- and target-scoped packet;
- `agent document` for one packet-bound editable function scope;
- `agent validate` and `agent apply` for the same document bytes;
- `agent view` and `agent diff` for deterministic review;
- `agent run` for an exact workspace, revision, entry, arguments, and policy.

`agent context --known-digest` returns a compact unchanged response only after rebuilding the exact
requested packet and matching its digest. Schema description remains available for diagnostics, but
normal work uses the contract digest and help embedded in the matched client.

## Semantic model and identity

One immutable `Snapshot` is authoritative. It contains the closed typed semantic model, exact
workspace/revision, root, durable allocator state, deletion history, and canonical hash.

Durable identity is assigned to continuity-bearing workspace/package/module scaffolding currently
required by the model, named declarations, fields, variants, functions, parameters, and explicit
typed hole anchors. Function-body regions, blocks, block arguments, ordinary operations, and implied
terminators use function-local IDs. Local IDs are revision-bound, cannot cross their owning function,
do not consume durable serials, and do not create tombstones.

The public `replace_function_body` edit preserves the function entity, deterministically replaces
its local body, and refuses to erase a durable hole anchor implicitly. Rename preserves the targeted
durable entity. Changed declaration shape is replacement identity; the current product has no
general continuity-map or specialized mapped-migration endpoint.

Two public-path migration shapes are covered:

- `Limits` to `DeploymentLimits`, including uses across construction, projection, signatures,
  outputs, calls, and safe deletion;
- a variant replacement with rename, reorder, and a new alternative, including blocked old
  deletion and exact behavior after replacement.

Neither migration established a shared safe rewrite abstraction, so explicit scoped replacement is
retained.

## Editable semantic documents

Document version 1 binds exact schema digest, workspace, base revision, editable scope, and optional
packet digest. It uses a closed grammar and normalizes into the same typed transaction as JSON.
Formatting is not semantic and parser syntax is never persisted.

Function documents render exact signatures and body structure and submit one
`replace_function_body`. Workspace documents can use the closed transaction edit vocabulary.
Read-only packet context cannot be edited, omission cannot delete content, and stale or foreign
scope rejects. Render-parse no-op, JSON-equivalence, alpha-stable local labels, deep/large-input
limits, precise locations, and old `plan` rejection have focused coverage.

The selected grammar is the former compact bracketed workbench grammar narrowed and renamed as an
exact document. Source-like and line-oriented alternatives were eliminated at design review on
strict parser surface and shared-value/control representation; equal-parser empirical comparison was
not completed and remains an explicit evidence limitation.

## Transactions, history, and persistence

Every mutation names exact workspace and base revision. Boundary decoding, proposal normalization,
semantic validation, history validation, response/artifact/HEAD preflight, durable publication, and
receipt construction are ordered. Rejection and validate-only publish nothing and consume no
durable identity. Keyed retries return the exact compact receipt or reject a fingerprint conflict.

The workspace store retains one canonical full artifact per immutable revision plus compact HEAD.
Publication writes the revision before replacing HEAD. Restart validates bounded paths and files,
decodes every contiguous retained revision, checks history and hashes, and rejects corruption or an
ambiguous publication state. Failure injection covers stages before artifact creation, after data
write, before/through HEAD replacement, rollback, reopen, and unknown outcome.

Full snapshots remain the sole storage path. Identity stratification removed persistent body churn
from the durable allocator and kept the retained eight-revision corpus between 8,354 and 9,457 bytes
per artifact. No delta log, content-addressed object store, database, checkpoint, compactor, or
storage garbage collector is present.

A workspace revision is not a package artifact. Package manifests, dependencies, import/export,
signatures, registries, and executable artifact caches are absent.

## Queries, review, and caches

Pure queries cover node selection, incoming uses, dependencies, owner chains, visible values, legal
constructors, blockers, repair context, snapshot summaries, and semantic diffs. Pagination and batch
budgets are exact and deterministic. Queries still use full scans; no persistent semantic index is
present.

Context packets state their purpose, exact target set, aliases, editable observations, read-only
dependencies, legal forms, blockers, limits, and omissions. The only active reuse mechanism is an
exact known-digest unchanged response. There is no server-side context cache, ranking model,
transcript store, apply-and-refresh delta, or implicit current-HEAD lookup.

## Language, compiler, and runtime

Implemented values and semantics are:

- `unit`, `bool`, checked `i64`, and immutable `bytes`;
- named immutable records and fixed-alternative variants;
- typed functions, calls, parameters, regions, blocks, and block arguments;
- constants, exact record/variant construction and projection, checked byte operations, checked
  arithmetic/comparison, conditionals, counted loops, exhaustive lazy matching, returns, yields,
  and typed holes;
- exact public values containing nominal declaration/member identities.

Compilation includes only the complete selected-entry dependency closure. It lowers to verified
Core IR with private dense IDs and exact semantic origins. The runtime uses explicit frames and
cannot consume user-controlled native stack through semantic recursion.

Immutable bytes use generation-checked handles, a separately verified ownership plan, deterministic
early reclamation, precise sharing counts, and verified unique-left concat reuse. A test-only
allocate-new route is the differential oracle. On the retained concat corpus it copies and peaks at
23 backing bytes versus 32 for allocate-new, with one reuse. Fuel and logical byte charging do not
depend on the optimization.

Runtime policies independently bound fuel, frames, live cells, managed visible bytes, retained
backing, managed objects, allocations, decoded inputs, and materialized results. Cleanup is tested on
success and traps. Run never publishes workspace state.

## Retained public applications

- `job-policy`: nested records/variants, calls, conditions, counted loops, hole repair, rename,
  historical execution, and deterministic decisions;
- `named-data`: exact record and variant values, complete lazy handling, invalid and valid repair;
- `release-channel`: broad creation replay and explicit/inline proposal equivalence;
- `release-manifest`: exact byte parsing, bounds trap, repair, rename, and history;
- `binary-canonicalizer`: byte construction, loop-carried accumulation, ownership differential,
  process session, failures, and historical execution;
- `agent-maintenance`: eight revisions covering create, repair, extend, refactor, rename, debug,
  declaration replacement, deletion, review, reopen, and old/current runs.

All drivers use production binaries and public CLI boundaries with private temporary state.

## Source and contract locality

Strict JSON is now separate from the executable contract catalogue. `machine.rs` is a small codec
facade; `contract.rs` owns descriptor bulk; codec-agreement tests live in `machine/tests.rs`.
Production bodies and large invariant tests for transaction, query, persistence, compilation, and
interpretation are likewise separated into owner-local test modules. The old campaign-named test
module and workbench plan module are gone.

The contract catalogue is still manually assembled rather than derived from DTO field declarations.
`transaction.rs` still combines several production concerns, and `tests/agent_repair_json.rs`
remains a broad integration file. These are current locality limitations, not hidden completed work.

## Exact absences

The current checkout does not implement:

- first-class semantic tests or documentation fields;
- multiple meaningful package graphs, package publication, or dependency resolution;
- editable module/package documents or continuity maps;
- branches, candidate commit chains, merge, or parallel-writer identity allocation;
- text values, general collections, generics, mutable values, or cyclic values;
- host effects, permission values, external resources, deterministic close operations, or retries;
- deployment hosting, hot reload, remote service, authentication, tenancy, sandboxing, telemetry, or
  a GUI;
- bytecode, JIT, AOT/native compilation, or cross-platform execution evidence;
- incremental semantic validation, persistent indexes, incremental storage, compaction, or bounded
  historical retention;
- automatic schema-fragment cache, context deltas, or apply-and-refresh.

## Evidence limits

The current provider comparison is a controlled two-run observation from the previous semantic
workbench campaign, not a benchmark distribution. No fresh provider trial was run for document
version 1, so no new provider-token or monetary-cost claim is made. Byte counts are reported as
bytes, not inferred tokens. The final verification commands and exact campaign measurements are
owned by [performance evidence](performance.md).
