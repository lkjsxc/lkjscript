# Finite-instantiation recursive nominal data

Status: implemented; authoritative campaign acceptance is pending.

The previous nominal-data decision rejected any definition cycle involving a generic declaration
and any concrete cycle involving an application. The selected campaign replaces that implementation
bound with non-expansive parameter flow. Canonical identities, explicit invariant applications,
ordinary containment, exact bounds, provenance, atomic publication, finite immutable values, and
existing executable-recursion and resource policies remain independent requirements.

A slot is an exact nominal declaration reference and its stable ordered parameter identity. For
every syntactic application of B in a member of A, an occurrence of A.i in argument j adds an edge
from A.i to B.j. The edge is plain only when the argument is exactly A.i; any constructor around
the occurrence makes it expanding. Nested applications contribute their own edges. The enclosing
member's list, record, case, or function is not growth of an application's argument. All signatures,
phantom arguments, absent cases and empty-container element types count. Closed arguments add no
source edge, but their nested nominal references still join the declaration closure.

Admission requires that no strongly connected slot component contain an expanding edge. An
expanding edge on a cycle can increase an occurrence's nesting on every lap. Otherwise expanding
edges occur only on paths through the finite acyclic component graph. Plain cycles select,
duplicate or permute among finitely many forwarded finite terms. Closed replacements reset a
path to a finite term. Thus finite roots have finitely many instantiated terms. This does not
promise a small closure or linear total size; duplication, permutations, and many roots can exhaust
separate work or storage admissions.

Production collection follows exact candidate/package members, records member/argument paths,
and uses iterative strongly connected components with a bounded cycle certificate. Success is
cached only within that validator's exact read view. Concrete closure uses complete canonical
applications; revisiting an instance adds an edge. No hidden monomorphic declaration or recursive
structural hashing becomes authoritative. The small-term proof tool instead uses weighted
transitive closure and direct term substitution/enumeration. A finite enumeration prefix is
never used to prove divergence.

Type dependency analysis, concrete preparation, and property-specific child relations are
different traversals. Ordinary containment and capture treat callable signatures as leaves;
finite-instantiation analysis follows them. Property proofs must inspect every relevant sibling
after a recurrence, and raw admission must inspect distinct nested values even when their exact
type is shared. Uninhabited recursive records may be admitted without a finite constructor result.

Typed layout bytes and logical graph generations change only if their corresponding encoding
contract changes. Validation/witness compatibility must change when the acceptance proof changes.
Persistence, copied public packages, strict loading, public retained sessions, scale observations,
authoritative checks, and immutable delivery are required by
[the campaign](../campaigns/202609110659.md); this decision does not report them as completed.

The codec audit found that bounded layout/value walks had no active cancellation control and that
decoder fallbacks swallowed every diagnostic class. Both independent typed codecs and typed JSON
now receive the owning control during type and value traversal. Operational cancellation/resource
failures propagate through checked and raw evaluator calls; malformed encodings and layout mismatch
still select the exact typed fallback. This is a required failure-boundary correction for recursive
data, with unchanged limits, successful bytes, nominal identities and execution-fuel accounting.

The predecessor experiment exposed a separate source-recovery defect: transport decoding required
a current producer witness before the source admission path could rebuild its own proof. Transport
now checks the unchanged witness schema, canonical certificate and exact source binding as content;
full canonical validation remains mandatory before readiness. Repository/cache witness codecs keep
their current-contract requirement. A transported historical witness is never reused as proof, and
compiled-unit 8 still rejects. This separates historical acceptance metadata from current admission
without adding a predecessor graph reader or changing source/package identities.
