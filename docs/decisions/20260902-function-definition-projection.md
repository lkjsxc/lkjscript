# Revision-pinned local-function definition projection

Date: 2026-09-02 UTC.

## Status

Accepted and implemented by campaign `202609021013` for unreleased product snapshot 0.1.16,
inspection-definition contract 1, registry 6, and CLI contract 18. Immutable public v0.1.15 and all
maintained graph, package, artifact, deployment, and operational identities remain unchanged.

## Problem

Public compact change can author complete typed function bodies, including lexical, match,
transaction, generic, capability, and affine-resource forms. Exact owner inspection exposes only a
summary, and bounded context exposes summaries plus relations. Reconstructing one accepted
nontrivial function therefore required storage or checkout-specific Rust access, making ordinary
distributed inspect/edit workflows asymmetric.

Whole-project source export, storage-shaped output, an application formatter, or accepting projected
records as change input would create a parallel representation with excessive authority. Recursively
expanding referenced declarations would also make one function's result unbounded by its structural
ownership.

## Decision

One exact `inspect owner KIND ID --detail definition` operation projects a live local pure or task
function with a body from one immutable accepted `RepositoryView`. It validates and materializes the
complete function contract, structurally owned body records, exact references at named-owner and type
boundaries, and owner-bound validation facts before rendering success. Referenced declarations and
dependency implementations are not expanded.

Canonical output orders the revision header, semantic contract fields, structurally owned body in
slot/index preorder, references by role and typed target, then facts in body order. Fixed logical
admissions are 4,096 body records, 16,384 structural/reference edges, 32,768 fact reads, depth 256,
and 8 MiB canonical logical encoding. Separately derived point-read, map, object, decode, literal,
continuation, record, and output admissions remain executable-discovered.

Every stateless `icont_` continuation binds repository, package, revision, function, projection
contract and digest, ordering, section, exclusive resume key, and integrity. It carries no body,
frontier, cursor, cache, or process identity. Resume reconstructs and validates the complete result;
only page item and byte budgets may change.

Complete typed reconstruction is the independent oracle. It does not call production point
traversal, ordering, rendering, paging, token, or expected-result helpers. Maintained proof compares
the affine `lkjournal` worker and the independently selected largest maintained function through a
copied candidate and isolated authority copy. A separate copied-binary HTTP workflow discovers,
projects, authors an ordinary compact body replacement, plans, applies, reinspects, checks, builds,
serves on loopback, and cleans up.

## Rejected alternatives

- Source text, raw storage, JSON, recursive declaration expansion, whole-project export, and
  projection round-trip input were rejected as parallel or overbroad authority surfaces.
- A body cache, index, mutable session, process cursor, or frontier-bearing continuation was rejected
  because the complete admitted result is replayable from immutable authority.
- Dependency implementation disclosure and an `lkjournal`-specific dumper were rejected because the
  required boundary is one local function and the maintained application is evidence, not product
  authority.
- Partial results on corruption or logical/physical exhaustion were rejected because paging must not
  conceal an incomplete definition.

## Consequences and reversal conditions

Summary inspection remains unchanged. Missing, retired, foreign, dependency, non-function, shared,
cyclic, noncanonical, summary-inconsistent, malformed, stale, or selector-mismatched requests reject
without writes. Output exposes accepted literals under normal escaping but never consults storage
paths, caches, artifacts, runtime handles, deployment secrets, operational data, queue transition
tokens, object bytes, or evidence paths. Projection records remain unknown compact-change input.

Change this design only when a maintained workflow exceeds the fixed admissions or proves that
stateless reconstruction is materially unacceptable. A replacement must retain one editable graph
authority, exact revision and complete-result binding, the named-owner reference cutoff, atomic
failure, independent full-authority equality, explicit resource dimensions, copied-binary proof,
and dependency-closed deletion of inspection-definition-1 and every `icont_` predecessor.

Campaign `202609021736` subsequently advanced the projection owner to contract 2 so a function
parameter record renders its optional exact requirement binding. Ordering, paging, continuation
shape, bounds, and the prohibition on using projection records as authored input are unchanged;
contract-1 continuations and projection identities reject as predecessors.
