# Reusable semantic release contract

This specification owns immutable reusable semantic units, exact release identity, explicit
exports, exact dependencies, cross-release references, release tests, canonical bytes, graph
validation, and publication. Workspace editing remains owned by
[semantic-model.md](semantic-model.md); runnable application composition is owned by
[application.md](application.md).

## Domains and authority

A workspace is mutable development authority with immutable revisions. One selected workspace
package is a release-construction root, but neither the workspace, package ID, revision, path, nor
allocator history survives as release authority.

A reusable semantic release is one immutable independently transferable artifact. Its canonical
payload is authoritative after strict decode and semantic validation. `ReleaseId` is the
domain-separated BLAKE3 digest of that complete canonical payload under
`lkjscript.reusable-release.identity.v2`. Equality of this full digest is exact release equality and
therefore participates in dependency and nominal identity. The collision contract relies on full
256-bit collision and second-preimage resistance; conflicting bytes claiming one ID are corruption.

`ReleaseContentDigest` hashes the same payload under
`lkjscript.reusable-release.content.v2`. It is an integrity/equality observation only. Coordinate
and user version are bounded identity-bearing release metadata, so changing either changes payload
and `ReleaseId`; neither can select an exact dependency or establish nominal equality. Provenance,
signature, authorization, yanking, freshness, and publisher identity are absent and cannot be
inferred from either digest.

Every semantic item is projected into a compact `ReleaseItemId`, a nonzero durable u63 integer
meaningful only under one `ReleaseId`. A nominal identity is exactly `(ReleaseId,
ReleaseItemId)`. Dependency slots, aliases, filenames, coordinates, user versions, compiler IDs,
and runtime handles are different domains.

## Construction and canonical projection

Release build contract version 2 is the typed lowering boundary. In maintained development, a
contract-1 release target owns its package root, coordinate, user version, exports, exact dependency
target edges, import proxies, and immutable cases. `target build` first selects one exact project
revision and mechanically lowers that accepted target to the build contract; it never infers a
mutable dependency, reads an external build request, or accepts host construction callbacks.

Construction starts from exports and case targets, iteratively traverses owned content and typed
definition references, includes reachable private implementation and nominal signatures, and
excludes unrelated packages, modules, declarations, holes, aliases, history, and mutable workspace
state. The closure cannot cross its selected package except through declared exact imports.
Reachable holes, missing local bodies, dangling values, foreign local references, and unused or
missing dependency slots reject.

The canonicalizer erases workspace and revision IDs. It assigns the release root and selected
package first, sorts modules by their retained names, sorts definitions by kind and retained name,
preserves semantically defined member/parameter/body order, and rebuilds function-local ordinals by
the canonical ownership traversal. All local and cross-release references are rewritten. Output is
independent of workspace ID, durable serial allocation history, unrelated tombstones, source
insertion order, request export/dependency/test order, map iteration order, and prior function-local
numbering. Names, public export keys, coordinate, version, member order, and semantic body structure
are observable release content; changing them may change the release.

Mutually recursive local definitions work because all selected definitions receive canonical IDs
before references are rewritten. The algorithm does not perform general graph isomorphism or erase
private helper factoring. Its work is bounded by release item and edge policies.

## Exports and private implementation

Format 2 exports functions, nominal product types, nominal sum types, and nominal homogeneous
sequence types in one flat canonical release namespace. Text is a primitive signature type and a
sequence signature preserves its exact element type. Export selection is explicit. Names are bounded lowercase ASCII symbols,
lexically ordered, and unique. Fields and variants travel with an exported nominal type; their
exact item IDs are available in inspection.

An exported function signature may use primitives, exported local nominal types, or exact imported
nominal types. A private local nominal type cannot leak through a public signature. Private
functions may be reachable implementation and are validated and encoded, but consumers cannot
target them. Dependency proxies cannot be re-exported in format 2. Private names are retained for
deterministic inspection and diagnostics; artifacts do not provide source confidentiality.

Changing exports, retained private implementation, tests, dependencies, coordinate, or user
version changes canonical content. There is no compatibility promise between two releases.

## Exact dependencies and imports

Every dependency slot is a bounded release-local role bound to one exact `ReleaseId`. The manifest
contains no range, `latest`, workspace HEAD, registry query, or filesystem path. Build receives the
complete transitive artifact set explicitly. A local import proxy is a bodyless function or nominal
declaration bound by slot and exported target name during construction; canonical bytes store its
release-local item, dependency slot, and exact exported item ID.

Whole-graph validation checks proxy kind and full signature, member order/name/type, direct-slot
membership, export visibility, exact bytes, and all limits before flattening. A reference cannot
escape to an undeclared transitive dependency or private target. Every declared slot must be used.
Distinct slots may bind the same exact release; the graph stores and validates that object once.
Distinct releases under one coordinate coexist. Graph cycles, self-dependencies, missing objects,
extra objects, or conflicting bytes reject.

The initial model has no resolver or lockfile. Human intent and candidate selection, if added later,
must finish before release construction and cannot change an accepted exact binding.

## Release tests

Release cases are producer-owned immutable typed invocation data in the release payload. They use
canonical unique names, exact local function targets, ordered validated arguments, exact result or
stable trap expectations, and exact policies. Text and nominal sequence values use the same type and
resource validators as execution. Cases can reach private local functions but not dependency-private
items; application cases cover cross-release public nominal composition.

Only exact pass counts as pass. A dependent release target validates the entire selected target
graph and runs every release suite before publication. `target test` exercises accepted project
cases; `release test` independently exercises an immutable distribution artifact. Cases are target
meaning and release payload facts, not property tests, tags, mocks, or a second assertion language.

## Canonical artifact version 2

The release envelope contains:

- magic `LKJREL\0\x02` and little-endian format `2`;
- semantic schema `lkjscript-tsm008` from the workspace artifact owner;
- checked little-endian payload length;
- canonical payload;
- exact `ReleaseId`; and
- separate `ReleaseContentDigest`.

Payload order is coordinate, user version, unit root, exact dependency slots, imports, exports,
tests, and semantic nodes. Collections use strict canonical order; durable local IDs are contiguous.
The decoder treats bytes as hostile, checks every length/count before allocation, rejects unknown
tags, invalid UTF-8, invalid/foreign IDs, duplicates, noncanonical order, wrong domains, malformed
semantics, digest mismatch, truncation, and trailing bytes, independently validates the release,
and requires byte-identical re-encoding.

Limits are 64 MiB per release, 100,000 semantic items, 256 exports, 256 dependencies, 4,096 imports,
256 tests, 100,000,000 aggregate release-case fuel, 256 bytes per retained/export/test name, 128
coordinate bytes, 64 user-version bytes, and 64 dependency-slot bytes. Exact graphs are limited to
256 release nodes, 4,096 edges, depth 64, and 256 MiB aggregate release bytes. These maxima are
reported by inspection.

## Target derivation, publication, and distribution commands

An accepted target change already validates and prepares the exact release closure. `target test`
prepares and tests without artifact publication. `target build` prepares the same release,
preflights its bounded project receipt, then uses a private temporary file, synchronization, atomic
no-replace link, cleanup, and parent-directory synchronization. A relative output is resolved against
the command working directory before canonical path checks. It never overwrites and never silently
retries an unknown output outcome. Target derivation publishes no development revision.

```text
target show TARGET [--project PROJECT] [--at REVISION]
target test TARGET [--project PROJECT] [--at REVISION]
target build TARGET [--project PROJECT] [--at REVISION] --output FILE
release validate --artifact FILE
release inspect --artifact FILE
release test --artifact FILE [--dependency FILE ...]
```

The command-local `release build --state` predecessor rejects. External exact release artifacts are
still explicit immutable dependency bytes for artifact-only validation/testing, but project target
edges refer to exact target IDs and lower their complete dependency closure directly.

Inspection is bounded and exposes exact identity, content digest, coordinate, user version, root,
exports with signatures/member IDs, exact dependencies, test summaries, counts, limits, and the
explicit absence of provenance and signatures. It does not expose workspace identity or claim a
current test pass.

## Offline and supply-chain contract

Release artifacts and one bundled application artifact are sufficient after every producer and
consumer workspace is removed. Correctness never depends on a local immutable store, filename,
ambient resolver state, or network. Application format 8 embeds the exact reachable graph once, so
execution needs no external release files.

Release format 1 and every predecessor reject directly. No compatibility reader or migration path
remains.

The implemented contract supplies content integrity and exact semantic identity. It supplies no
authenticity, publisher authorization, provenance, freshness, revocation, signing, attestation,
transparency, registry, mirroring policy, or hostile-host sandbox. Those are separate future
artifacts or services if a real trust consumer appears.
