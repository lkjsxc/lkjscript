# Identity-preserving graph-native function extraction

Date: 2026-09-02 UTC.

## Status

Accepted and implemented by campaign `202609022319` for unreleased product snapshot 0.1.18.
Immutable public `v0.1.16`, Graph 7, compiler-unit/bytecode 3, Artifact 12, runtime preparation,
deployment authority, and operational data contracts remain unchanged.

## Problem

Complete local-function inspection and low-level typed changes exposed all information and
primitives needed to split a function, but a machine author still had to derive structural closure,
free captures, effect and requirement closure, affine provenance, evaluation placement, identity
continuity, replacement, and deletion itself. The maintained 192-body-record `lkjournal`
definition made that context cost concrete. Rebuilding the selected subtree under fresh identities
would discard stable graph identity, while a source-text or application-specific refactor path would
create another authoring authority.

## Decision

Compact change has one operation named `extract.function`. From an exact base it selects one
existing local nongeneric function, one exact proper structural expression subtree, one
request-local helper symbol, and one absent same-module name. It materializes and validates the
complete definition before planning. The helper is always private, same-module, nongeneric, and
nonrecursive; there is no automatic root or name selection and only one extraction is admitted per
request.

The movable closure is the selected expression plus all structural descendants under one exact
incoming edge. Free function parameters, lexical bindings, and match payloads become ordered helper
parameters. Capture order is first canonical use then typed owner identity; duplicate source names
receive a bounded full-owner-key suffix. Result type is inferred exactly and must be resource-free.
The least effect is pure or the exact required subset of the caller effect in caller order.

One direct capability resource may cross the boundary only through the existing requirement-bound
affine handoff: exact acquiring requirement and interface, one selected consuming use, no later
caller use, final consume parameter, final local-read argument, and an acyclic private same-package
call. Borrowed, multiple, contained, escaping, ambiguous, mismatched, foreign, generic, closure,
transaction-capture, or resource-result shapes reject.

The accepted rewrite preserves the target declaration and every movable owner identity. It
reparents the selected root as helper body, changes only free-local reads to generated parameters,
and replaces the original parent slot with one direct call and effect-free local arguments. Review
binds the exact base-definition digest, moved-owner digest and set, captures and uses, inferred
contract and affine provenance, preserved/changed/generated owners, body counts, semantic diff,
impact, tests, and prepared commitment. Apply recomputes against the exact base and publishes only
through `GraphRepository`'s atomic visibility point.

## Rejected alternatives

- Delete-and-recreate extraction was rejected because semantically unchanged subtree identities
  are part of the refactor contract.
- Source text, projected-definition input, a serialized refactor program, and an `lkjournal`-only
  migrator were rejected as parallel or private authoring paths.
- Automatic root/name selection and multiple extraction were rejected because they enlarge review
  ambiguity and closure coupling.
- Public, cross-module, cross-package, generic, recursive, closure-capturing, borrowed-resource,
  multiple-resource, resource-container, and resource-result helpers remain outside this bounded
  operation.

## Consequences and reversal conditions

The normalized request, logical plan, definition projection, closure witness, contributor oracle,
and compiler caches remain derived evidence. Accepted graph meaning alone owns the new helper and
call. Compact change, authored request codec, logical plan, registry, and CLI contracts advance
because their encodings or behavior change; Graph 7 and downstream execution contracts do not.

Reconsider only when a maintained workload proves this single private same-module operation cannot
express a needed semantics-preserving split. Any extension must independently specify type and
effect inference, evaluation order, ownership and identity, resource transfer, failure and
cancellation, package visibility, bounded review, migration/deletion, and an implementation-disjoint
oracle. This decision does not imply general move/rebind/signature editing, inline, closures,
resource results, async/session semantics, or a source-language refactor surface.
