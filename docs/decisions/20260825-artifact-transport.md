# Logical package closure and artifact transport

Date: 2026-08-25 UTC.

## Status

Accepted and implemented in the normalized package, linker, and artifact path. The released
`build`, package, and runtime commands have not yet cut over, so this record does not claim a
complete public workflow.

## Problem

The former normalized package object combined logical package identity, physical semantic-root
layout, validation witness layout, and recursively selected child transports. Artifacts copied
that object and validation certificate into their manifest. Revalidation, repacking, or choosing
another valid child transport could therefore change artifact bytes or make a diamond dependency
closure ambiguous without changing application meaning.

## Decision

- `PackageRevision` is the storage-independent logical package object. It embeds a verified
  `RevisionCore`, the package identity, the content-derived public-interface digest, and sorted
  exact logical dependency records.
- `PackageTransport` binds one package revision to one exact physical semantic root, validation
  witness, and deterministic interface-map root. It never names child transports.
- Every transported semantic root is included as an object, decoded, and checked for exact
  repository, package, and semantic-state agreement with the embedded revision core.
- Logical dependency traversal follows `PackageRevision` records. Package-count and total-edge
  budgets are independent and charged before enqueue.
- Each logical revision owns a revision-scoped `PACKAGE-TRANSPORTS/<revision>/` bucket. Immutable
  candidate registrations are published only after their transport objects are durable, and one
  strict bounded `CURRENT` record is replaced atomically. Missing, corrupt, resource-invalid,
  stale, or invalid `CURRENT` bytes use a bounded two-pass scan of that revision's independently
  verified candidates. Infrastructure read failures remain errors. Replaced transports and
  candidate registrations stay readable; unrelated revision history cannot consume the bucket's
  work budget.
- Compilation and artifact manifests bind repository, package, semantic revision, semantic state,
  package revision, public-interface digest, and build policy. They do not bind transport digest,
  physical semantic-root digest, witness layout, validator certificate, or transport index.
- Artifact bundles carry logical package revision objects, deterministic public-interface maps,
  compiler units, runtime/reference owners, types, and blobs. Strict segmented framing owns
  offsets, lengths, ordering, per-object integrity, and complete-bundle integrity.

## Alternatives

- Retaining one recursive package object was rejected because it made replaceable evidence and
  layout transitive artifact identity.
- Storing child transport digests in parents was rejected because independently revalidated
  diamond branches could not select one logical dependency consistently.
- Trusting the transport index was rejected because a disposable acceleration cannot define which
  package meaning is readable.
- Omitting exact acceptance transport entirely was rejected because dependency staging still
  needs a durable, independently checked binding from logical revision to semantic root and
  evidence.
- Keeping the predecessor package object as an alternate decoder was rejected by direct-cutover
  policy.

## Evidence

The final package/transport suite passes 10 tests, publication repository suite 40 tests, compiler
suite 20 tests, and normalized execution suite 18 tests. The exact committed library passes 323 tests and
warnings-denied Clippy.

Focused tests reject semantic-root substitution, foreign revision and package binding, malformed
and trailing transport/selection bytes, a dependency count encoded above its owning bound,
duplicate selections, dependency-edge exhaustion, per-revision candidate exhaustion, and
aggregate staging byte or entry exhaustion before object accumulation. Repository tests remove,
corrupt, or oversize `CURRENT`, install a valid selector naming missing or foreign transport bytes,
and verify bounded independent fallback; a selector symlink/read error remains an infrastructure
failure. Unrelated retained revision buckets do not affect lookup. A linked dependency is staged
through a second valid physical witness and interface-page layout: HEAD and logical package
revision stay unchanged and the former transport remains readable. A separate artifact test then
supplies an exact dependency artifact with a genuinely different valid interface-page partition;
the linker rebuilds every dependency interface into its own deterministic pages, and the final
artifact bytes and bundle digest are identical.

## Consequences and remaining work

Validation evidence can evolve without renaming package meaning or executable artifacts. Diamond
closures select physical evidence locally while agreeing on exact logical revisions. Old valid
transports are deliberately retained; canonical deletion remains disabled until reader, backup,
draft, reachability, and interruption prerequisites exist.

The current artifact builder still retains the complete assembled bundle in memory. Streaming
write/load budgets and retained complete-workflow memory evidence remain required before claiming
bounded large-artifact assembly. Each logical revision currently admits at most 10,000 registered
physical candidates; this deterministic recovery budget is independent of repository history but
still requires a proven retention policy before canonical reclamation. Maintained package assets
must be recreated only after the released package/build path cuts over; regenerating them through
predecessor commands would produce the wrong contract.

## Reversal condition

Replace the transport index or bundle segmentation only when measured complete workflows show a
material correctness, locality, or resource benefit. Any replacement must preserve logical
package identity across revalidation/repacking, independent root verification, deterministic
artifact equality, typed closure budgets, and a correctness path that does not trust derived
selection state.
