# Source namespace, contextual type scope and nominal parameter flow

Date: 2026-10-20. Scope: the existing `LKJSG001` decoded profile.
Acceptance remains subject to the finalized candidate's verification gates.

## Obligation and boundaries

Extend E1 after exact carrier binding, reference closure, syntax membership and
value-local scope. The next checks distinguish package namespaces, shared type
descriptors and nominal parameter flow from syntax containment. They do not admit
expression typing, effects, witnesses, affine custody or source normalization.
The Tier 2 source-driven fixed points and native phase reclamation remain separate
requirements; the Rust bootstrap boundary is unchanged.

Use the existing Bytes and ByteBuffer capabilities. Derived tables are private
execution data, not producer-supplied certificates or a new persisted encoding.
No schema epoch, runtime ABI, VM byte-slice behavior or reference-interpreter
independence changes are required by this slice.

## Measured evidence and minimal composition

The clean starting source at `19d012f` passed the complete-compiler syntax test
on all four execution paths. The live VM observation was 609,466,238 instructions,
57,135,623,188 cumulative modeled allocation bytes and 71,434 milliseconds.
The independent reference observation was 319,865,455 expressions,
65,343,596,240 cumulative modeled allocation bytes and 364,160 milliseconds.
These are measurements of the complete syntax-validation workload, not live heap
or native RSS. The complete test finished successfully in 503.56 seconds.

This workload justifies composing later E1 passes over one admitted carrier and
one derived membership table. Extracting `source-scope::check` retains the existing
public wrapper and diagnostic precedence while allowing later checks to share the
remaining allowance. It adds no trusted producer data and changes no encoding.
Independent malformed, boundary and semantic controls must establish that this
composition preserves earlier admission. Broader E1 source forms and then E2
type/effect/custody/witness checking are the immediate downstream obligations.

Preflight showed approximately 608 GiB free disk and 56 GiB available memory;
neither physical compaction nor an allocator redesign was a prerequisite.
The production VM's immutable byte views and the reference interpreter's copying
implementation remain independent. No native phase-reclamation claim follows.

## Namespace identity

Uniqueness is exactly `(parent identity, namespace class, canonical name)`.
The class projection covers all 12 kernel classes. Module and Target have no
parent; Declaration belongs to its Module; TypeParameter, EffectParameter,
RequirementParameter, Field, Case, Operation, Requirement and Port belong to a
Declaration. Parameter belongs to either its Function declaration or Operation.
Binding, Expression, HttpRoute, Documentation and Annotation are excluded.
Equal names remain legal across different parents or classes and for local
bindings resolved by exact identity.

Collect 16-byte rows containing the parent owner-directory index, class, absolute
name offset and name length. The owner count represents an absent parent. Exact
owner-directory indexes preserve identity domains. Earlier shape and endpoint
admission establish the bounded name and parent fields. Deterministic merge sort
compares parent, class and original name bytes; adjacent equal keys reject.
Names are neither hashed nor copied into independent keys.

The existing decoder admits eight namespace-bearing forms: Module, Declaration,
TypeParameter, Field, Case, Parameter, Port and Target. Whole-carrier uniqueness
controls cover these eight. The 12-class projection and both Parameter parent
domains have separate direct controls. EffectParameter, RequirementParameter,
Operation and Requirement owners still encounter unsupported-form diagnostic 13;
those controls do not establish their complete carrier admission. Adding these
forms requires coherent shape, reference, containment and contextual checks.

## Type structure and using context

Derive structural edges from every supplied type descriptor, including unused
descriptors. Structural fields, List items, function parameters/results and Applied
arguments are child-type edges. Named nominal references are declaration edges,
not structural backedges. Resolve all endpoints before graph proof. Repeatedly
settle descriptors whose children already have heights; lack of progress with
unsettled descriptors identifies a structural cycle. Preserve the kernel's maximum
structural depth of 256 edges.

For each owner carrying type roots, determine its enclosing semantic declaration
through admitted ownership rows. Traverse its type roots in that using context;
a TypeParameter descriptor must refer to a parameter owned by that declaration.
The same digest can therefore succeed for one declaration and fail for another.
No digest-only global scope cache is introduced. Any future shared cache must bind
the complete declaration/callable and binder context, including method scopes and
exact binder identities/kinds as those source forms become supported.

This scope check covers the current TypeParameter-bearing profile. Read-scope,
effect-parameter and implementation-witness forms excluded by the decoder retain
diagnostic 13; their full contextual semantics are not implied by this slice.

Content-addressed type digests commit to their children. Rehashing ordinary type
objects cannot construct a self-consistent structural cycle. Exercise cycle proof
with independently specified derived graph fixtures, and exercise actual carrier
paths with acyclic descriptors and contextual scope mutations. Do not describe
the direct cycle fixtures as end-to-end hash-consistent carrier coverage.

## Nominal parameter-slot flow

Scan every included nominal field and case payload, including unused members.
Traverse nested applications and all argument descriptors, including phantom
arguments and function signatures. Require the exact ordered argument arity for applications reached through these
member payloads. Applications appearing only in function signatures/bodies or unused
descriptors remain outside this arity check and require general application typing.
For each occurrence of a source parameter in a transmitted argument, add an edge
from that exact parameter slot to the target declaration's exact parameter slot.
An argument equal to the parameter gives a plain edge; a constructor surrounding
that occurrence inside the argument gives an expanding edge.

Reject when an expanding edge has a return path through the slot graph. This is
equivalent to an expanding edge within a cyclic strongly connected component.
`tree<T> = leaf(T) or branch(List<tree<T>>)` is permitted: its transmitted argument
is still T. `grow<T> = stop or next(grow<List<T>>)` is rejected: the transmitted
argument expands T on a cycle. Parameter permutation, duplication, selection,
closed replacement and growth on acyclic slot paths remain permitted. Nominal
cycles and callable generic finite-flow are distinct admission obligations.

## Work and diagnostics

The composed path prepares membership once, then carries one remaining allowance
through `source-scope::check`, `source-namespace::check`, contextual type/DAG checks
and nominal flow. Internal checks return a nonnegative remainder or a negative
diagnostic; public wrappers return zero or the positive diagnostic. No later pass
resets the allowance. Reserve traversal work before the work or growth it covers.
Namespace comparisons additionally reserve bounded name-prefix inspection; graph
work includes node, edge and membership visits. Nominal queue and edge-table
appends also reserve copied four-byte words before concatenation. Exact fit succeeds.

| Code | Meaning |
| --- | --- |
| 6 | Invalid or exhausted additional proof-work allowance; no semantic-invalid claim |
| 16 | Duplicate exact namespace key after earlier admission |
| 17 | Contextual type-parameter scope, structural DAG or structural-depth failure |
| 18 | Nominal argument arity or expanding cyclic parameter-slot flow |

Codes 12 through 15 preserve their earlier meanings. Native rejection retains
exit status 65, the exact numeric diagnostic and no partial output. Proof-work
units are distinct from VM instructions, cumulative modeled allocation, native
live allocation, retained mappings and process RSS.

## Required evidence

Rebind canonical objects, directories and neutral map/state commitments for every
semantic mutation. Assert that each negative passes all preceding layers and
fails at its intended diagnostic. Compare applicable failures with exact Rust
kernel diagnostic classes/codes and use independent positive and negative cases
across VM, canonical reference, Rust-native and selfhost-native execution.

The Rust full-snapshot oracle requires exactly the type descriptors reachable from
live owners, whereas the carrier deliberately binds every supplied descriptor.
Nominal member mutations can leave their replaced descriptors unused. For the
independent Rust oracle only, derive the live descriptor inventory through neutral
owner roots and child-type links, and rebind the changed owner map. Preserve every
owner and the semantic-state digest. Do not modify or filter the raw carrier.
A separate positive regression retains two unused descriptors in all four carrier
execution paths while proving that the Rust inventory projection removes only those
descriptors. Negative controls reject unexpected unreachable-type diagnostics.

Retain complete carrier SHA-256 and semantic-state identity separately. Report
restricted-profile evidence and unsupported boundaries explicitly; this decision
does not establish E1-final or Tier 2 completion.
