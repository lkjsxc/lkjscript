# Implemented status

This file describes the current checkout. Normative behavior belongs to `docs/spec/`; measurements
and architecture decisions belong to `docs/performance.md` and `docs/architecture.md`.

## Product boundary

`lkjscript` is a local typed semantic programming system for pure deterministic applications. A
coding agent can create a workspace, observe bounded context, author an exact semantic document,
validate or publish an immutable revision, review it, attach immutable release cases, seal one exact
dependency-closed application artifact, inspect and test the artifact, and run it after removing the
source workspace.

The supported bootstrap is stable Rust edition 2024 on Linux x86-64. The repository forbids local
unsafe Rust. The application runner is not a sandbox, package manager, native executable runtime, or
production deployment system.

## Active versions

| Contract | Active value | Older forms |
|---|---|---|
| workspace logical protocol / strict JSON | 10 | 9 and older reject |
| machine contract | `lkjscript-machine-schema-v10` | v9 and older reject |
| workbench | 2 | reject |
| context packet | 2 | packet 1 rejects |
| editable semantic document | 1, root `document` | `plan` rejects |
| application CLI JSON contract | 1 | other versions reject |
| application artifact | 1, `LKJAPP\0\x01` | other versions reject |
| workspace semantic artifact | 6, `LKJTSM\0\x06` | format 5 rejects |
| semantic schema | `lkjscript-tsm006` | older schemas reject |
| workspace HEAD | `LKJHEAD8` | `LKJHEAD7` rejects |

There is no compatibility reader, migration mode, edition split, successful old-form alias, or
silent fallback.

## Development and application workflow

The primary CLI opens the topology-neutral `Engine` directly under one state-directory authority
lock. `lkjscript session` retains one Engine for independent line-delimited requests. No daemon is
required for normal authoring or application work.

Agent commands are `orient`, `create`, `context`, `document`, `validate`, `apply`, `view`, `diff`,
and exact-revision `run`. Context digest reuse returns a compact unchanged marker only after
rebuilding the exact packet. Editable document version 1 is exact-base, schema-bound, scope-bound,
and packet-bound when it uses aliases.

Application commands are:

- `app build --state DIR --validate-only` for full closure, byte, and release-test preflight;
- `app build --state DIR --output FILE` for no-overwrite atomic publication;
- `app validate` and `app inspect` from artifact bytes alone;
- `app test` for all immutable release cases;
- `app run` for versioned exact public-value invocation; and
- `app stream` for a compatible pure `bytes -> bytes` process profile.

Every build request names exact workspace, revision, entry, invocation profile, policy, and tests.
Artifact commands never infer current HEAD or require workspace history.

## Semantic model and identity

One immutable `Snapshot` is authoritative. Durable workspace-qualified IDs belong to the root,
package/module containment, declarations, members, functions, parameters, and explicit repairable
hole anchors. Regions, blocks, block arguments, ordinary operations, and implied terminators use
revision-bound function-local IDs. Local IDs cannot escape their function, consume no durable
serial, and create no tombstone.

`replace_function_body` preserves the function and deterministically rebuilds local terms. Shape
changes replace durable declarations and require explicit use updates and safe deletion. Names are
owner-scoped lookup and display metadata, not identity.

Application artifact version 1 retains the exact source workspace/revision identity domain so
nominal values and local diagnostics remain valid after transfer. It is explicitly run-only: it
does not define import, vendoring, fork, merge, cross-artifact continuity, or package coordinates.
Its digest is an integrity/content key only.

## Application artifact and release tests

An application artifact contains one entry, one typed or bytes-stream profile, one exact Run policy,
at least one entry-targeting release case, all test targets, and the complete transitive semantic
closure. Package entry fields are removed because the application manifest is the sole entry owner.
Workspace history, HEAD, idempotency, context state, aliases, proposal text, paths, caches, unrelated
declarations, Core IR, ownership plans, and runtime handles are absent.

The 64-MiB canonical binary has independent magic, format, semantic schema, payload length, and
domain-separated BLAKE3 digest. It permits at most 100,000 nodes, 256 lexically ordered unique
tests, a highest durable serial of 262,144, and aggregate declared suite fuel of 100,000,000. Decode
checks bounds, strict ordering, tags, identities, values, closure, digest, truncation, and trailing
bytes, reconstructs a validated Snapshot, and requires exact re-encoding.

Release tests are application-local data, not durable semantic entities or an assertion language.
Each has a canonical name, exact function target, typed arguments, exact expected value or stable
trap, and policy. Results distinguish pass, value/trap mismatch, invalid/incomplete case, resource
failure, and engine failure. Only exact equality passes; every case runs before build publication.
Inspection reports exact artifact, path, and runtime limits plus each test's domain-separated case digest, target,
argument/result types, expectation kind, expected trap when any, and policy. The digest binds large
argument and expected values without silently truncating them. Test execution never publishes
workspace state.

Application publication uses a private mode-0600 file in the destination directory, file sync, an
atomic no-replace hard link, temporary cleanup, and directory sync. Before-link failure leaves no
destination. After-link failure reports `artifact_publication_outcome_unknown`. Existing
destinations never overwrite. Application paths are absolute, lexically canonical, limited to
4,096 bytes, and reject symlink inputs or parents. Exact success JSON is size-preflighted before the
link, and only an opaque release-tested `PreparedApplication` can invoke publication.

## Language, compiler, and runtime

Implemented values and semantics are:

- `unit`, `bool`, checked `i64`, immutable `bytes`, nominal immutable records, and fixed variants;
- functions, calls, parameters, regions, blocks, and block arguments;
- constants, checked addition/comparison, byte construction/access/slice/concat, conditions,
  counted loops, exhaustive lazy matching, exact aggregate construction/projection, returns,
  yields, and typed holes; and
- exact public values carrying nominal declaration/member IDs.

Compilation includes only the complete selected entry closure and lowers to independently verified
Core IR with private dense IDs. Artifact test targets use the same compiler and interpreter. No
serialized IR is trusted or distributed. The explicit-frame interpreter prevents semantic call and
control depth from consuming unbounded native stack.

Immutable bytes use generation-checked handles, a separately verified ownership plan, deterministic
early reclamation, exact sharing counts, and verified unique-left concat reuse. The allocate-new
route is a differential oracle. On the retained 512-octet append control, production copies 1,024
backing bytes versus 131,840 for allocate-new and peaks at 513 versus 1,024 backing bytes. This is an
implementation optimization, not accepted ownership semantics.

Fuel, frames, live cells, visible managed bytes, retained backing, objects, allocations, decoded
input, and materialized result are separate bounded policies. Cleanup is tested on success and trap.

## Storage, queries, and topology

Workspace persistence retains one full canonical artifact per immutable revision plus compact HEAD.
Restart decodes every contiguous retained revision and validates adjacent history. Rejection and
validate-only publish nothing and consume no identity. One writer remains the only authority.

Queries remain deterministic full scans with full-scan differential controls. There is no persistent
semantic index, server-side context cache, context ranking, apply-and-refresh state, delta store,
object store, database, checkpoint, compactor, or storage garbage collector.

`lkjscriptd` remains an optional private Unix-socket diagnostic adapter over the same Engine because
the exported framed `Client` and integration suites still consume correlation, deadline,
disconnect, shutdown, and competing-authority behavior. It is not a primary workflow or sandbox.

## Retained public applications

- `binary-canonicalizer` is the representative distributable application. It creates and repairs a
  byte program, validates release cases, performs validate-only and repeated equal builds, removes
  source state, then validates, inspects, tests, typed-runs, stream-runs, and corrupts the artifact.
- `job-policy`, `named-data`, `release-channel`, `release-manifest`, and `agent-maintenance` retain
  broad semantic, history, identity, repair, migration, review, and diagnostic-RPC coverage.

All drivers use production binaries and public boundaries with private temporary state. Only
`binary-canonicalizer` currently exercises the complete standalone application lifecycle.

## Contract and source ownership

`application.rs` owns application closure, tests, canonical bytes, independent validation,
standalone execution, and publication. `bin/lkjscript/application.rs` owns only versioned JSON/raw
process presentation. Application fields are not copied into the workspace machine catalogue.

The global machine catalogue remains manually assembled in `contract.rs`. It still has current
consumers in schema-digest binding, workbench help, context/document contracts, dependency-closed
diagnostic projection, and strict external-client agreement. No proc macro, build script, IDL, or
second catalogue was added. Manual field duplication and the broad `agent_repair_json` suite remain
locality debt.

## Exact absences

The checkout does not implement:

- reusable package artifacts, explicit exports, dependency resolution, import/remapping,
  registries, signatures, attestations, or user release versions;
- durable semantic test entities, test tags/subsets, property tests, assertions, mocks, or private
  test artifacts;
- multiple application exports, application import, executable cache, serialized Core IR,
  bytecode, JIT, AOT, or native code;
- host effects, permission values, external resources, deterministic close, deployment hosting,
  authentication, tenancy, or sandboxing;
- branches, candidates, merge, or parallel-writer identity allocation;
- text, general collections, generics, mutable values, or cyclic values;
- cross-platform build/run evidence, automatic schema derivation, alternate proposal grammar,
  context delta, or apply-and-refresh.

## Evidence limits

The artifact mutation corpus is deterministic rather than coverage-guided. No fresh provider trial,
provider pricing, cross-platform run, package workload, source-like parser prototype, runtime
representation replacement, model checker, or new fuzz target was completed. Bytes are reported as
bytes, never inferred tokens. Exact current measurements and tool availability belong to
[performance evidence](performance.md).
