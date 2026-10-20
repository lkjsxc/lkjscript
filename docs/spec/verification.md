# Verification and evidence

Verify boundaries with independent expectations and complete admission. Support only
current formats; regenerate maintained inputs after format changes without fallback readers.

## Execution and storage

Use one checkout and one leased Cargo target, `$PWD/target`. Record actual selected
outputs and Cargo's rebuilt/reused observations. Readers must finish before writers
replace shared outputs.

```sh
verifier_root=$(bash tools/bootstrap-verifier.sh)
"$verifier_root/lkjscript-dev" check changed --machine
"$verifier_root/lkjscript-dev" check full --fresh --machine
"$verifier_root/lkjscript-dev" release target
```

Bootstrap returns the shared executable directory, using its optimized profile without
LTO/debug data. `--output` selects an absent receipt directory; `--expected-source`
binds a commit; `--with-controller` builds the release controller. Reject unsupported
Cargo configuration/environment. Bind checker identity to its compiled checkout.

For a frozen precommit tree, use the explicit
[staged-tree bootstrap](../decisions/20261020-staged-tree-verifier-bootstrap.md)
contract; acceptance must precede committing that exact tree.

Default checks atomically replace `.artifacts/lkjscript-dev/latest-receipt.json`
with a receipt smaller than 64 KiB. It records selected gates, execution outcomes,
input stability, elapsed times and output identities. Joined transient logs and
fixtures are deleted, including failed runs. No gate executable is copied.
`--retain-archive` explicitly retains complete originals for a release handoff or
investigation. A compact receipt cannot substitute for missing release originals.
Bootstrap has the same explicit archive option. Do not retain incidental test trees.

Set checker `--jobs` and `CARGO_BUILD_JOBS` independently. Join workers and child
processes before cleanup. Failures, unavailable dependencies, cancellation, input
drift, missing outputs and cleanup failures remain failures. Fresh means gates ran
in this invocation; it does not mean Cargo rebuilt unchanged dependencies.

## Required coverage

| Profile | Required boundary |
| --- | --- |
| focused | Narrow development checks |
| changed | Current Git status and dependency closure |
| product | CLI, packages, maintained assets and discovery |
| service | Distributed/stateful/outbound HTTP and lkjournal |
| full --fresh | All 29 standalone gates |
| release-source --fresh | 25 source gates plus final candidate acceptance |

The [registry](../../tools/lkjscript-dev/src/check/registry.rs) owns commands, limits
and dependencies; [selection](../../tools/lkjscript-dev/src/check/snapshot.rs) owns
changed-path widening. Preserve every workspace/all-target/all-feature/reference
test, safe-Rust/lint/policy gate, native planner, guide and maintained-asset check.
Counts cannot replace successful outcomes. Changed selection reads working-tree
state. Use optimized test profiles; debug tests do not establish full acceptance.

The release build selects all eight workspace test harnesses. Their gate runs two
at a time with sixteen test threads each, without taking a Cargo writer lease.
Exact executable proofs, complete named/ignored inventories and terminal outcomes
remain bound to the producer. Every target runs after a failure. Only the copied
candidate lifecycle is excluded there because its own dependent gate executes it.

Release source readers require explicitly retained full or release-source originals
with exact source, toolchain, environment, verifier, commands, dependency outputs
and Cargo records. Re-admit the whole original receipt before selecting the required
25 gates. Final acceptance still requires six target owners, seven native smokes,
ELF policy, two pinned userlands and installation recovery. Source semantic cases
and final executable behavior retain distinct evidence. See [release](../release.md).

## Semantic and process tests

Run finite public adapters in process. Use bounded `MemProjectStore` projects and
copy-on-write forks where the test does not exercise filesystem publication.
Integration modules share `public_cli` and its caches; focus by module or test name.
Clones share heads; forks have independent graph/data heads and share immutable
charged bytes. Reuse admitted standard packages and compiled supplier programs
once per process. Memory storage must retain the same canonical decoding, complete
validation, idempotency, stale-base and atomic publication rules as disk storage.
Explicit file, process, signal, deployment, installation and release boundaries
remain real boundary tests. Never describe host semantic results as execution of
candidate bytes. Keep complete case inventories while batching equivalent routes.

Stateful/service source checks use `--source-reads` for linked queries and
inspection. Their receipts bind those calls to the verifier image; mutation, HTTP,
restart and process observations still bind the copied product. Transferred
candidate acceptance rejects every source-read receipt and semantic observation.
Offline composition runs all nine independent fixture families on eight workers,
after shared public discovery. Reserved command/export/runner ranges prevent collisions;
all workers join before ordered evidence merge and complete independent admission.

Production executes normalized bytecode. The reference interpreter reconstructs
meaning independently from canonical owners, including signatures, layouts, types,
targets, tests and blobs. Removal of compiled tables must not disable reference
behavior. Missing compiled tests must fail inventory comparison. Pure runs require
both evaluator equality and independent expected values. Live effects execute once;
neutral scripted adapters own differential effect tests.

Independent oracles cover ownership/provenance, effect rows and grants, package
closures, queries/extraction, route overlap, collection histories, catalog reachability,
recursive proofs and native planning. Neutral codecs/digests may be shared; a
production algorithm cannot generate its own expected answers. Required fault
injections must fail discriminating controls, then restored controls must pass.

## Admission and resource invariants

- Validate complete closures, including unused, untaken and final-invalid syntax.
  Preserve identities, ordering, scope, types, witnesses, provenance and requirements
  across graph, package, compiler, artifact and runtime boundaries.
- Reject malformed input/results/captures before callbacks, effects or secrets.
  Strict independent loaders admit both neutral valid reconstructions and hostile
  metadata controls. Rehashing cannot repair invalid meaning or authority.
- Reserve modeled growth before mutation. Exact fit passes; one less refuses.
  Semantic validity, preparation capacity, quotas, cancellation and deadlines differ.
  Count raw-result/input nodes, constructor work, repeated nodes and hidden scans.
  Modeled work, allocator work, live memory and RSS are distinct measurements.
- Publish under the owning lock after revalidation. Stale, invalid, exhausted or
  interrupted candidates expose no partial authority. Accepted keyed retries retain
  original outcomes after descendants. Concurrent same-base writers have one winner.
- Join children and release loans before ancestors on success, trap, cancellation
  and refusal. Retain unrelated custody and preserve primary plus cleanup errors.
  Missing output never proves rollback or safe retry of effects.
- Maintain public literal authoring, refresh/draft round-trips, exact dependency
  transports and detached-source tests with independent full expected results.
  Test both evaluators through direct/imported, empty/exact-bound/oversized cases.

Keep feature matrices in their owning tests/specifications. Report full wall time
and peak/disk usage separately from correctness. Acceptance has no three-minute
requirement; complete successful outcomes and bounded storage remain required.
