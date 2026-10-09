# Direct construction of immutable list storage

## Decision

Build each final node once in the private `List::from_items` boundary. Keep the
existing 32-way immutable representation, element handles, persistent append,
indexing, traversal, destruction and independently admitted value semantics.
An unpublished intermediate list does not need persistent-update path copying.

For nonempty length `n`, the tail contains `((n - 1) % 32) + 1` elements. Its prefix
has `floor((n - 1) / 32)` complete leaves. Choose the minimal existing checked
prefix height, then construct each child at the requested height. A partial
rightmost child must not collapse its one-leaf branch layers: later indexing and
append rely on those layers. Empty input has neither root nor tail.

The builder moves each raw item and each new child handle into its final location.
It does not flatten an existing list, clone its payloads, publish prefixes, create
a new element representation or expose a transient mutation API. Recursion is
bounded by the existing storage height, not by guest call depth or nested values.

## Ownership and resource boundaries

Reserve before each node and element allocation. `RawArguments` owns pending raw
values, `Element` owns a value before its fallible reservation, and bounded local
branches or the partial result own already attached values. Preserve the final
zero-charge cancellation checkpoint, including on empty input. Failure must
release all temporary ownership without changing unrelated live owners.

Raw construction is not semantic admission. Complete raw occurrences, exact
origins, type eligibility, ownership, capture, equality and effects remain the
responsibility of their existing independent admission paths. Shared storage is
not proof that an occurrence may be skipped. No instruction, artifact, discovery,
public signature or compatibility generation changes with this mechanism.

Logical length, maximum height and configured limits do not increase. Cumulative
construction charges decrease because intermediate branch allocations disappear;
requests with enough budget for the final tree may now succeed where persistent
prefix reconstruction exhausted that budget. This is an intentional resource-cost
change, not evidence that every physical allocation or retained byte is modeled.
The final retained topology and its admission metadata remain unchanged.

## Independent verification

Count final leaves as `ceil(n / 32)`. Starting from the completed prefix leaf
count, repeatedly ceil-divide by 32 until at most one remains and sum those levels
to count branches. This flat arithmetic oracle calls neither the builder nor its
capacity, metadata or charge helpers. Successful construction must reserve exactly
these final nodes, allocate one handle per raw item, and copy no existing handles.
Exact and one-less slot/byte limits test reservation, not just observations.

The storage regressions cover tail and root boundaries through one million
values, complete forward/reverse traversal and indexing, subsequent append,
retained siblings, unchanged payload identity, and final metadata. Every
reservation failure and cancellation checkpoint is exercised through a 1,057-item
multi-level prefix. Deep pending and attached payloads must release on a bounded
stack; real owned-cell observations additionally preserve an unrelated live owner
and distinguish resource refusal from cancellation.

The existing detached public Map-entry matrix exercises bulk entry-list ingress
through ordinary fresh authoring, compilation and complete-result execution. Its
independent admission-count, malformed-tail, resource-refusal, recovery and joined
cleanup obligations remain unchanged. Fresh full-source acceptance is separate
from these focused tests and from finalized public-release acceptance.

## Limits and next decisions

Lower construction-node counts are not a language-wide speed ratio, RSS change,
retained-memory saving, instruction reduction or application benchmark. The source
vector and per-element allocations still exist. Persistent single-element append
still copies its bounded path. This decision does not implement list fusion,
streaming Map enumeration, native cursors or a public builder ownership contract.
Choose those changes from independently measured consumers and semantic needs,
not from the cost of an unpublished prefix that this builder can already avoid.
