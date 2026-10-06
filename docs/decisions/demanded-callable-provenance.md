# Demand recursive type provenance from declaration slots

Decision: after complete source admission and exact callable-context cycle
selection, construct the backward closure of declaration type-parameter slots.
Materialize a selected witness's type parameters only when that exact path can
feed this closure. Keep the full ordered path and exact callable context in each
request. Never merge paths merely because their implementation shapes agree.

## Why this boundary

[Context-cycle selection](context-scoped-callable-proof.md) removed eager typed
path expansion on acyclic calls. A recursive function that simply forwards an
unused prerequisite could still unfold an exponentially large path tree. A
compact 24-layer duplicate-prerequisite library therefore exhausted metadata
admission even though its recursive type flow was finite. Increasing the limit
would not address this representation cost.

An edge carrying a constructor is introduced by a written type argument. In the
current flow model its source is a type parameter of the calling declaration;
its target may be a declaration parameter or a parameter at a concrete selected
witness path. Forwarding an existing prerequisite, including a method projection,
only introduces nonexpanding edges. Consequently every expanding cycle contains
a declaration parameter slot. Its entire cycle is reachable backward from that
slot. Retaining the backward closure of all declaration parameter slots therefore
preserves every expanding cycle. Removing other witness-only forwarding cannot
remove an expanding cycle. The retained graph uses the original weighted-cycle
criterion rather than treating reachability itself as an error.

This proof depends on constructor edges originating at declaration slots. A future
flow operation that constructs a type from a witness slot must either extend the
seed set or establish an equivalent closure theorem before using this analysis.
Witness-only nonexpanding cycles are permitted; ordinary recursion is not an error.

## Exact transfers and bounded ownership

Requests are keyed by exact callable context and the complete ordered witness
path. Context identity retains package/declaration, selected method and ordered
prerequisite shapes. A request for an ordinary declaration parameter group uses
its existing slots. A requested witness path follows only its own selection-DAG
edges and allocates only its parameter group, not every descendant.

Each recorded incoming call is pulled back through its original source expression
or method map. Written arguments retain their defining lexical scope. A concrete
operand follows the requested child path; a lexical operand requests the matching
source prerequisite and suffix. Method selection removes the selected root and
forwards its children without erasing exact positions. Permutations, duplicate
operands and closed resets therefore retain their separate provenance. A carrier
with no declaration type parameters can still receive demanded witness paths.

Complete discovery still independently admits every declaration, mapped target,
operand, argument type and untaken expression before this proof selection. Missing
bodies, foreign parameters, phantom arguments and wrong arities cannot hide in an
unrequested path. The conservative prerequisite-construction restriction remains
unchanged. This is neither dead-code elimination nor inferred implementation choice.

The operation owns its request table, worklist, incoming edges and slots. Charge
comparisons, path copies and modeled storage before growth, and retain cancellation
checkpoints while discovering and propagating requests. Resource exhaustion remains
distinct from semantic expansion. No global or cross-revision cache is introduced.

## Evidence and limits

The retained pre-change implementation rejects the 24-layer recursive probe with
`kernel_callable_flow_storage`. The new flow passes the same admission limits.
Small matched probes retain an eager recursive-path reference, while the independent
weighted transitive-closure oracle checks substitutions, nested constructors and
resets. Exact same-shape witness permutations and a parameterless forwarding carrier
exercise demanded provenance rather than only the unused-prerequisite fast case.
Exact/N-1 work admission and early/late cancellation probes retain failure classes.

The [literal public workload](../../examples/demanded-callable-proof/README.md)
authors and exports the recursive generic library before creating a concrete item,
then uses bounded concrete selections, borrowed reads, identity-preserving edits and
detached execution. [Measurements](../performance.md#demanded-recursive-type-provenance)
separate admitted proof work and metadata from execution costs; small cases can
require more work. [Status](../status.md) owns actual acceptance and distribution.

This does not prove polynomial preparation in general. The exact context inventory
and the number of genuinely demanded paths may still grow substantially. Concrete
prepared-witness materialization, repeated source reads during transfer and shared
immutable dependency admission remain separate costs. No graph, interface,
type-object or artifact encoding changes are required by this derived analysis.
