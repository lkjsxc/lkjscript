# Compact callable proof for an acyclic generic library

The [library proposal](library.lkjc) declares an exact `Reader<Self>` contract,
a generic two-prerequisite `Pair<T>` reader, and `read0` through `read24`.
Each step passes the same selected reader twice to the preceding step. The
literal proposal is small; expanding every typed prerequisite path would give
`read24` more than sixteen million leaf occurrences. The selected witness
shapes form a compact DAG instead. `Pair` reads through its first prerequisite;
the unused second prerequisite remains independently admitted.

This is a designed source-admission workload, not a production workload or a
claim that an equally deep concrete executable is cheap. The public test admits
and exports the complete generic library **before creating any concrete item or
consumer**. It then exercises four concrete layers, retaining both exact leaf
identities, borrowing and eventual consumption of the original cell.

## Public reproduction

Copy the executable and both literal proposals outside the compiler checkout.
Create a `minimal` library project. Prepend `request base=BASE` to the library
proposal using the current `status` revision, plan it, apply that unchanged input
with its exact review token, and run `check`. Draft `compact-proof` and require
an unchanged plan. Export its ordinary package transport before creating the
consumer. No standard-library dependency is required by this witness.

Create a `command` consumer project, stage the exact exported transport and add
its exact dependency. Import `compact-proof` with the exported package ID and
package revision, then author [consumer.lkjc](consumer.lkjc) through the same
plan/apply/check operations. The [native library guide](../../docs/guides/native-library.md)
documents dependency staging and exact export commands.

Build `compact-proof.lkja` and use a deployment descriptor selecting target
`compact-proof`, with no listener, grants or secrets. The command takes one I64.
For `[17]` its complete result is:

```json
{"alternate":99,"observed":17,"restored":17}
```

`Cell` and `Alternate` have the same Self type and contract but remain distinct
selected implementations. Both reads borrow the same cell. Extraction happens
only after those scoped reads finish and returns its original input value.

The public harness covers the minimum and maximum I64 values, negative input,
zero and 17. It edits the alternate result from 99 to 100 while preserving both
implementation identities, then restores it. The original artifact runs after
both authoring projects and the transport are removed. Results are checked
against literal independent expectations, with no live handles, remaining tasks
or cleanup failures. Foreign lexical prerequisites and extra application operands
must reject without changing accepted HEAD.

## Scope and limits

Source admission first discovers all exact callable contexts and all source
operands. Only the proof of recursive type growth is restricted to edges inside
callable cycles. Types, visibility, ownership, effects, scopes and unused methods
remain independently checked everywhere; this is not dead-code elimination.

The recursive proof retains distinct typed prerequisite paths. Equal witness
shapes alone are not sufficient to merge their type provenance. Large recursive
contexts can still exhaust finite proof capacity. Prepared concrete witness
materialization is also a separate cost: this workload does not claim that
invoking `read24` with a concrete leaf has compact runtime preparation.

[Status](../../docs/status.md) owns actual source acceptance and public availability.
The [generic implementation contract](../../docs/spec/generic-owned-implementations.md)
owns the admission rule. The public `native_owned_compact_callable_proof` test
belongs to the mandatory finalized-byte `native_owned_` harness family.
