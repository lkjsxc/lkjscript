# Language semantics

This specification owns observable types, values, operations, control flow, compilation, execution,
and semantic resource failures. Identity and workspace publication belong to
[`semantic-model.md`](semantic-model.md).

## Types and values

The closed type set is:

- `unit`;
- `bool`;
- checked signed `i64`;
- immutable `bytes`;
- immutable `text`;
- nominal immutable product declarations;
- nominal immutable sum declarations; and
- nominal immutable sequence declarations with one exact element type.

Product values contain exactly one value for every field in declaration order. Sum values select
exactly one declared variant and carry a payload exactly when that variant declares one. A sequence
value names one exact sequence declaration and contains an ordered homogeneous list. Nominal equality
requires the same declaration and member identities; equal shape or spelling is insufficient. In a
workspace those are workspace-qualified durable IDs. In a reusable graph they are exact
`(ReleaseId, ReleaseItemId)` pairs. Private compiler remapping preserves that equality.

Products and sums embed their members by value, so a by-value nominal cycle rejects. A sequence is a
managed-indirection boundary: recursive type reachability through a sequence is permitted, while
every runtime value must still be finite and satisfy depth, item, element, and byte bounds. Immutable
construction cannot create a pointer cycle or backpatch an existing value.

### Bytes

Bytes are ordered octets. Public JSON uses one strict unpadded URL-safe base64 spelling. Byte backing,
capacity, sharing, views, handles, and reuse are not observable. Bytes are never implicitly text.

### Text

Every `text` value is one valid UTF-8 byte sequence. Equality is exact byte equality. No Unicode
normalization, case folding, locale, collation, grapheme, display-width, or canonical-equivalence
promise exists. Length is always UTF-8 bytes. Arbitrary byte slicing cannot construct text.

Public JSON represents text as a JSON string, not base64. JSON, workspace artifacts, releases,
applications, instance state, queries, and product bindings validate UTF-8 before acceptance.
Controls and escape characters are valid semantic text; terminal-safe rendering is a separate client
obligation. One text value is limited to 65,536 UTF-8 bytes, and a literal retained in semantic source
is limited to 4,096 bytes.

### Sequences

A sequence declaration has one exact nominal identity and one exact element type. Order is semantic;
allocation order and backing identity are not. Empty is canonical. Length is limited to 16,384
elements before allocation or traversal. Public JSON is one ordered array, and every element is
validated against the exact declaration-owned type. Foreign nominal values reject even when their
shape is equal.

Nested sequences remain subject to the global value depth, item, visible-byte, retained-byte,
managed-object, and output limits. Representation sharing cannot bypass logical accounting.

## Operations

The closed operation set includes:

- unit, boolean, integer, byte, and text constants;
- checked `i64` addition, less-than, and equality;
- boolean not, conjunction, and disjunction;
- direct function call;
- lazy `if`;
- counted `for_i64`;
- product construction and field projection;
- sum construction and exhaustive `match_sum`;
- byte length, checked index, checked slice, equality, and concatenation;
- text byte length, equality, and concatenation;
- sequence empty, length, checked zero-based element access, append, and replace;
- typed `hole`; and
- `return` and `yield` terminators.

Integer overflow traps; it never wraps. Byte and sequence indexes are signed `i64` proposals and must
be nonnegative and in range before access. Sequence append and replace return a new immutable value
of the same exact sequence type; replacement preserves length. Text concatenation checks its result
byte length before allocation and remains valid UTF-8 because both operands are valid UTF-8.

A product constructor supplies every field exactly once. A sum match supplies every variant exactly
once and binds a payload only for payload-bearing variants. Calls target durable function entities.
Nominal operations target durable declarations and members. Ordinary operation results, branch
values, loop binders, and match payload binders are revision-local and cannot escape their function.

The small integer/boolean additions are retained because task identity, priority, readiness,
pagination, filtering, and lifecycle paths use them repeatedly. There is no operator overloading,
implicit coercion, polymorphic equality, numeric trait, higher-order collection operation, iterator,
mutable builder, map, set, or hash table.

## Evaluation order and fuel

Expression, operand, field, argument, sequence, loop, and selected-edge order are deterministic. `if`
evaluates only the selected arm. `match_sum` evaluates only the selected variant arm. A counted loop
uses explicit start, exclusive end, nonzero step, loop-index binder, and loop-carried binder; its next
iteration receives the prior yield.

Every executed instruction and transferred flat value has the common deterministic charge. Variable
work adds a logical charge: byte/text equality charges the compared prefix, concatenation charges
result bytes, byte slice charges result bytes, sequence length charges element count, and sequence
append/replace charge result element count. Sequence access is checked constant logical work plus
normal value flattening. These charges do not depend on allocation reuse, capacity, `Arc` counts, or
serialization size.

Calls and user-scalable control use explicit runtime frames. User depth does not consume unbounded
native stack. Recursion and dependency traversal are bounded by explicit frames, fuel, and the
application's own work limits.

## Incomplete programs

A function with no body and a typed `hole` are valid incomplete accepted states. A selected entry may
run only if its complete dependency closure contains no missing body or hole. Incomplete unused
declarations do not block another complete entry.

A hole is a durable repair anchor. Refinement preserves its exact result type and body scope. Ordinary
body terms remain revision-local.

## Compilation and execution

Compilation consumes one immutable accepted snapshot and one durable entry. It discovers and lowers
only the complete reachable function and nominal closure. Dense compiler IDs, layouts, blocks,
values, ownership actions, and origins are derived and never semantic identity.

The independent Core verifier checks type tables, nominal closure, indirection-safe layouts, blocks,
instructions, control edges, result indexes, calls, switches, and bounds before execution. Invalid
derived IR rejects instead of being interpreted.

One explicit-frame interpreter is the correctness route. `Run` is pure with respect to workspace and
instance authority: success and traps publish nothing. A trap does not poison a reusable engine or
foreground session.

`RunPolicy` bounds fuel and frames. Fixed policies independently bound arguments, value depth/items/
bytes, result materialization, flat cells, managed cumulative visible bytes, live retained backing,
and managed objects. Lengths and counts are checked before corresponding allocation or work.

## Managed immutable representation

Bytes and text share the existing generation-checked managed byte store. The production store uses
verified managed-reference maps, exact ownership claims, deterministic reclamation, safe immutable
views, and uniqueness-guided concat reuse. A test-only allocate-new byte mode remains the oracle.

Sequences use a safe invocation-local immutable object containing ordered `Arc<RuntimeValue>`
elements. Append and replace shallow-share immutable elements while allocating one new sequence
object. The store separately charges every live sequence the exact retained byte count of the simple
canonical allocate-new representation and the logical visible byte content; sharing therefore cannot
evade limits. Empty/append/replace/access/materialization are differentially checked against canonical
allocate-new encoding. Public results are deeply materialized once at the boundary.

This representation is not language ownership. Authors cannot observe handles, generations,
reference counts, allocation, sharing, capacity, addresses, or reuse. Accounting is exact logical
managed accounting, not process RSS enforcement. No tracing collector is retained because accepted
values are immutable and cannot form pointer cycles.

## Public values and failures

Run inputs and outputs are exact typed public values. Workspace nominal values name workspace IDs;
application nominal values name exact release/item IDs. Values validate type, shape, depth, counts,
UTF-8, bytes, element types, and foreign domains before flattening or materialization.

Failures distinguish proposal/semantic rejection, incomplete compilation, invalid derived IR,
runtime trap, fuel/frame/value/resource policy, I/O, and unknown publication outcome. A sequence
index trap is distinct from malformed retained state or output exhaustion. Diagnostics name a durable
entity or exact revision-local origin where applicable.

## Safety and effects

Accepted semantics expose no raw address, unchecked memory access, pointer arithmetic, unchecked cast,
manual deallocation, shared mutable heap, or foreign memory. This repository contains no local unsafe
Rust. That is an implementation safety boundary, not a formal proof.

Language evaluation has no ambient host authority. Stateful application suspension returns ordinary
typed command data; instance and adapter owners publish and execute it only after separate validation.
The language has no permission values, live resources, concurrency, time, randomness, filesystem,
network, process, signal, or nondeterministic-finalization primitive.
