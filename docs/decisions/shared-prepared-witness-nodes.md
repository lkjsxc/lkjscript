# Share whole immutable prepared witness nodes

Decision: retain each exact prepared implementation application once. Catalogue
entries, prerequisite edges and function applications carry shared handles to that
complete immutable node. The preparation interner also returns the same complete
node, rather than only reusing an integer identity and copying its metadata.

## Correct the starting assumption

Concrete witness DAGs and application-specific shared instruction bodies already
existed before this change. Source `65b3d00428d36827f9d65bf92f7900cf52719d21`
can author, export, prepare and execute the complete 24-layer recursive reader
through the public product. Its copied executable also passes reviewed edits,
restoration and execution after removing both authoring projects and their package
transport. The [concrete public workload](../../examples/concrete-callable-proof/README.md)
is stronger maintained coverage, not a newly enabled language feature.

What remained duplicated was the complete application record at each incoming edge,
function binding and catalogue slot. Shared child arrays avoided exponential trees,
but a cheap clone of one incoming witness still copied its metadata header; the
preparation representation also copied its type-argument vector. Reduce that
redundancy without changing selected meaning or raising any limit.

## Exact identity, local physical ownership

The interning key retains the implementation declaration, its complete ordered type
arguments and the complete ordered child identities. Equal Self types or contract
shapes do not identify equal implementations. Operand order, duplicate selections,
unused prerequisites and all written type operands remain admitted before an exact
cache hit. A cache entry cannot excuse a foreign lexical parameter or excess depth.

The bootstrap uses `Arc` for whole immutable nodes. This is derived prepared
metadata, not a language-level shared-ownership rule. A future dense arena may
replace these handles without changing the language. No interior mutable payload,
application values, capabilities, effect grants, borrow state or cancellation lineage
is shared through a witness node. Each preparation owns its catalogue and caches;
there is no global cache or cross-revision identity reuse.

The runtime catalogue binds every admitted node to its exact preparation. Ordinary
calls and parallel transfer admission require the same immutable node as the
catalogue entry before considering a visited-node shortcut. An equal numeric ID,
an independently allocated equal record, or a node from another preparation is not
that handle. Full suffix depth and each prerequisite obligation still check. The
process-local allocation identity is never encoded, hashed into program meaning,
exposed as authority or used as canonical iteration order.

## Bounded construction and failure

Reserve node bodies, reference-count metadata, handle arrays and interning entries
before modeled growth. Exact reused nodes do not reserve nonexistent deep copies.
The complete-node interner publishes a new key only after normalization succeeds.
Failed scratch state is not an accepted program. Existing work/storage capacities
and cancellation checkpoints remain separate from semantic refusal.

Retain exact/N-1 byte and work tests, early and late cancellation, repeated roots,
longer incoming paths, same-shaped distinct selections and deliberately modified
records. Independent source interpretation and expected literal results remain
oracles; pointer equality alone does not prove program behavior.

## Evidence and limits

[Measurements](../performance.md#whole-prepared-witness-node-sharing) distinguish
the retained witness-node/slot subset from conservative cumulative preparation
reservations, process RSS, allocator traffic and invocation time. The change adds
one allocation per unique complete node while removing redundant copies; it is not
a claim that every workload is faster or allocates fewer physical blocks.

No meaning-graph, interface, type-object, bytecode or artifact encoding is changed.
This closes whole-node sharing within one preparation, not region custody, shared
mutable application state, cross-snapshot interface sharing or all preparation
costs. [Status](../status.md) owns actual source acceptance and publication.
