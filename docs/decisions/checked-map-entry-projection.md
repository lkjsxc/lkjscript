# Checked, lookup-free map entry projection

## Decision

Production `core.map.entries` traverses the existing persistent map once through
an allocation-free, bounded cursor. The cursor borrows one admitted immutable
parent and returns only its actual keys and values. It does not accept a raw child,
asserted type, origin or caller-supplied admission certificate. The reference
interpreter retains its independent key-then-lookup implementation.

Previously the intrinsic enumerated sorted keys and searched from the root again
for each value. The retained AVL tree already exposed ordered entry references;
reusing those references removes the unnecessary logarithmic search factor without
replacing its storage, weakening admission or adding a public intrinsic.

## Admission and custody boundary

Only the production checked-value owner can create the cursor. Creation requires
a free map from the exact prepared program. The private iterator is derived from
that same parent, whose immutable storage remains live for the whole traversal.
Each projected checked value inherits that parent's program origin. A same-shaped
foreign map, raw child or free scalar cannot substitute for this relationship.

The complete raw argument still crosses ordinary ingress admission, including its
last child, unused values, nominal/callable origins and any concealed affine
resource. Checked updates still admit new children independently. This mechanism
neither creates an address cache nor changes transfer, capture, owned allocation,
packing or consuming-unpack admission.

The result owns immutable aliases independently of the map. Text and byte keys
share their existing buffers. Shared records, lists and maps retain their existing
backing; inline option/sum spines still need real clone storage. That storage is
reserved before cloning. Failure after cloning releases the temporary values and
never changes the retained parent. Borrow metadata or a parent owner is not
silently carried into the returned immutable entries.

## Work, reservations and cancellation

The underlying bounded iterator visits each retained node once in key order.
For scalar payloads this changes enumeration tree work from O(n log n) to O(n).
This is not a claim that every payload clone is constant-time: total clone-spine
work and output construction remain additional costs. The output entry records,
field names, intermediate checked-value vector and persistent result list still
allocate and retain their existing reservations. This is not zero allocation,
zero-copy execution, bounded RSS or whole-application linearity.

The cursor checks cancellation before bounded movement, before owned clone growth
and after construction. Entry/list construction keeps its own reservations and
checks. Refusal aborts the entire intrinsic: no successful partial list is exposed,
and prior accepted cumulative work remains charged. One extra terminal poll may
change a deterministic cancellation probe's threshold, not its failure class or
resource-completion contract. Ordinary foreground execution gains no instruction
budget and no existing resource limit is raised.

## Independent evidence

The maintained [public workload](../../examples/map-entry-projection/README.md)
authors maps from ordered entries, projects complete records and compares repeated
enumeration with a header-only length control. Independent expected results use a
separate ordered-map oracle, not the cursor or reference interpreter. The workload
is authored, checked, built and run through a copied product and continues after
deleting its disposable source project and supplier transport.

Runtime tests compare both evaluators against complete independent results, retain
old versions across real insertion/removal paths and test empty, boundary-sized
and differently shaped trees. Foreign origins, non-map roots and corrupted proof
classes reject. Boxed-spine reservation failures and interrupted clones leave the
original payload owners intact. Explicit byte/item limits and a deterministic
cancellation sweep include the boundary after complete ingress and during traversal.
Malformed final scalar, callable, nominal and resource payloads reject before a raw
host callback can run. Successful and failed invocations join their owned resources.

The original inherited failing complexity test and the predecessor's public
negative-control observations are retained separately from successful candidate
acceptance. Modeled visits are not wall time. Matched executable measurements must
hold artifact, input, result and build profile constant, separate invocation from
preparation and encoding, and retain controls and unfavorable samples.

## Compatibility and next boundary

No language, source-graph, interface, type, artifact or data encoding changes are
required. Existing public map order, duplicate replacement, lookup behavior and
persistent versions remain unchanged. Product identity is separate from these
contracts. Source acceptance, final distributed bytes and running applications
remain separately [status-owned](../status.md).

The next optimization question is output construction cost, not another unchecked
projection API. A future fold, view or cursor exposed to native programs needs an
explicit borrowing/lifetime and failure contract. This private immutable traversal
does not by itself implement such a language feature, a compiler replacement,
regions or a JIT.
