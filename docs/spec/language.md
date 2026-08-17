# Language semantics

This specification owns observable types, values, operations, control flow, completeness,
compilation, execution, and resource failures. Identity and publication belong to
[`semantic-model.md`](semantic-model.md).

## Types and values

The closed type set is:

- `unit`;
- `bool`;
- checked signed `i64`;
- immutable `bytes`;
- a nominal product declaration;
- a nominal sum declaration.

Product values contain exactly one value for every declared field in declaration order. Sum values
contain exactly one declared variant and a payload exactly when that variant declares one. Nominal
equality requires the same declaration and member identities; equal shape or names are insufficient.

Bytes are ordered octets. Public JSON uses one strict unpadded URL-safe base64 spelling. Backing
allocation, sharing, views, handles, and reuse are not observable language state. Bytes are not an
implicit text type and no Unicode normalization is defined for runtime values.

## Operations

The closed operation set includes:

- unit, boolean, integer, and byte constants;
- checked `i64` addition and less-than;
- direct function call;
- `if`;
- counted `for_i64`;
- product construction and field projection;
- sum construction and exhaustive `match_sum`;
- byte length, checked index, checked slice, equality, and concatenation;
- typed `hole`;
- `return` and `yield` terminators.

Integer overflow traps; it never wraps. Byte index and slice bounds are checked before access. A
product constructor supplies each field exactly once. A sum match supplies every variant exactly
once and binds a payload only for payload-bearing variants.

Calls target durable function entities. Nominal operations target durable declarations and members.
Ordinary operation results, branch values, loop binders, and match payload binders are
revision-local and cannot escape their owning function body.

## Evaluation order and laziness

Expression order, operand order, field order, argument order, loop order, and selected control edges
are deterministic. `if` evaluates only the selected arm. `match_sum` evaluates only the selected
variant arm. A counted loop uses explicit start, exclusive end, nonzero step, loop-index binder, and
loop-carried binder; its next iteration receives the prior yield.

Calls and user-scalable control use explicit runtime frames. User depth does not consume unbounded
native call stack. Recursion is limited by the explicit maximum-frame policy.

## Incomplete programs

A function with no body and a typed `hole` are valid incomplete accepted states. A selected entry
may run only if its complete dependency closure contains no missing body or hole. Incomplete unused
declarations do not block an otherwise complete entry.

A hole is an explicit durable repair anchor. Refinement must preserve its exact result type and body
scope. Ordinary body terms remain revision-local.

## Compilation

Compilation consumes one immutable accepted snapshot and one durable entry function. It discovers
the complete reachable function and nominal-type closure and lowers only that closure to private
Core IR. Dense compiler IDs, layouts, blocks, values, ownership actions, and source-origin tables are
derived and never become semantic identity.

The independent Core IR verifier checks type tables, nominal closure, blocks, instructions, control
edges, result indexes, layouts, call signatures, switch exhaustiveness, and all bounds before
execution. Invalid derived IR rejects rather than being interpreted.

## Execution

One explicit-frame interpreter defines behavior. `Run` is pure with respect to workspace authority:
success and traps publish nothing. A trap does not poison a reusable engine or session.

`RunPolicy` separately bounds fuel and frames. Additional fixed policies bound arguments, public
value depth/items/bytes, result materialization, flat cells, managed visible bytes, retained backing
bytes, and managed objects. Logical fuel is independent of allocation reuse; optimized and
allocate-new byte execution consume the same fuel and produce the same value or typed trap.

The production byte representation uses verified managed-reference maps, checked generation-tagged
handles, precise acyclic sharing counts, deterministic early reclamation, and uniqueness-guided
left-buffer concat reuse. A test-only allocate-new mode is the correctness oracle. On the retained
concat control, production copies 23 bytes and peaks at 23 backing bytes versus 32/32 for the oracle,
with identical behavior and fuel.

This optimization is not language ownership. Authors cannot observe handles, retain/release actions,
buffer reuse, allocator slots, or memory addresses. A second managed value class, escaping values,
or cycles triggers revalidation of this strategy.

## Public values and failures

Run inputs and outputs use exact typed public values. Nominal values name durable declaration/member
IDs. Every value is validated for type, shape, depth, counts, bytes, and foreign-domain references
before flattening or materialization.

Failures distinguish proposal/semantic rejection, incomplete compilation, invalid derived IR,
runtime traps, fuel/frame/resource policy, I/O, and unknown publication outcome. Diagnostics name a
durable entity or an exact revision-local origin where applicable.

## Safety and effects

Accepted language semantics expose no raw address, unchecked memory access, pointer arithmetic,
unchecked cast, manual deallocation, shared mutable heap, or foreign memory. The Rust package forbids
local unsafe code. This is an implementation safety boundary, not a formal proof.

The language currently has no host effects, permission values, resource-owning values, concurrency,
time, randomness, filesystem access, sockets, process access, or nondeterministic finalization.
Those absences are bootstrap limits, not permanent semantic prohibitions. A future effect must add
explicit typed authority, ordering, cancellation, retry/partial-action, audit, and deterministic
cleanup contracts. Ordinary immutable-value reclamation will remain separate from affine external
resource cleanup.
