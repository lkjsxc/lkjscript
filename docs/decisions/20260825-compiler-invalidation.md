# Compiler input identity and invalidation

Date: 2026-08-25 UTC.

## Status

Accepted and implemented in the normalized compiler cache and linker. Public `check`, `build`, and
`run` still require direct cutover, and large-workflow locality remains to be measured.

## Decision

A compiler unit is derived from the exact semantic owner facts that affect emitted behavior:
owner kind and implementation, referenced canonical type objects, exact dependency package
revision/interface facts, capability requirements, target selection, and the explicit
optimization policy. Compilation manifests bind the package revision, semantic revision,
storage-independent semantic state, and package-interface digest.

Physical semantic-root layout, validation witness or certificate, package transport selection,
pack/catalog identity, documentation, operational receipts, and history metadata do not enter
compiler or artifact identity unless a future language contract makes one of those facts
behavioral. Exact semantic references are lowered to dense artifact/runtime indexes; mutable names
are retained only where behavior or diagnostics require them.

Incremental compilation may reuse a unit only when its typed input identity is equal. Clean and
incremental compilation remain independent paths whose emitted artifact bytes must agree. Compiler
caches and manifests are derived, disposable, and never accepted semantic authority.

## Alternatives

- Keying the cache by accepted physical root was rejected because unrelated map layout and
  revalidation would invalidate every unit.
- Keying only by owner object digest was rejected because dependency interfaces, referenced types,
  requirements, and build policy also affect behavior.
- Retaining stable graph identities directly as hot instruction operands was rejected because
  deterministic dense lowering is smaller and keeps runtime layout replaceable.
- Treating cached compiler output as authority was rejected because clean reconstruction is the
  correctness oracle.

## Evidence

Normalized compiler tests cover clean and incremental reuse, exact invalidation, linked dependency
bodies, deterministic artifact rebuild, malformed/corrupt artifact rejection, and foreign package
material. The package transport replacement test changes physical witness/layout evidence for an
exact dependency while preserving the logical package revision. A separate test supplies a loaded
dependency artifact with a different valid interface-map partition; relinking canonicalizes that
layout and produces byte-identical artifacts with an equal bundle digest. The final compiler suite
passes 20 tests, the complete library passes 326 tests, and warnings-denied Clippy passes.

## Consequences and remaining work

Revalidation, repacking, and transport-index replacement do not force recompilation. A semantic
body, signature, type, effect, requirement, target, dependency interface, or build-policy change
invalidates the exact unit and consumers selected by typed dependency facts.

Current clean inventory and artifact assembly still materialize broad closure state. The campaign
must add independent typed compiler/link budgets, streaming artifact output, complete-workflow unit
read/reuse/rebuild observations, and maintained standard/`lkjournal` clean-versus-incremental
equality before making scale-locality claims.

## Reversal condition

Expand a unit input only when differential clean/incremental evidence proves that the omitted fact
changes emitted behavior. Split or specialize units only when end-to-end measurements show a
material benefit without changing semantic authority or artifact determinism.
