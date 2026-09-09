# Explicit pure-function environments

Status: accepted and implemented. BND-1–BND-6 and exact-target admission are recorded at the
implementation boundary in the [evidence owner](../evidence/202609090021-pure-function-binding.json).

Configured reducers need to preserve ordinary runtime inputs while satisfying the existing reducer
function type. Requiring every caller to carry configuration through fold state makes otherwise
reusable function interfaces depend on each factory's configuration. A public configured-reducer
witness justifies this foundation; prior adoption by multiple maintained applications is not required.

Accepted meaning gains one `Bind { callee, arguments }` expression. A named graph function remains
the only body authority. The runtime resolves its exact prepared target and explicit type arguments,
then retains an immutable flat prefix. Rebinding concatenates prefixes. Empty binding preserves the
callable, and complete binding constructs a zero-argument function. No creator frame, lexical
substitution map, mutable environment, or serialized closure becomes program authority.

Each evaluator owns construction, capture admission, and invocation decisions. The VM admits each
capture before evaluating the next expression; strict bytecode verification protects the unfinished
prefix. The reference evaluates canonical expressions and resolves canonical signatures independently.
Both carry checked retained values directly into calls and eligible tail transfers. Capture admission
has its own node observations and cumulative allocation/item charges; invocation does not repeat it.

Capture safety is stricter than affine freedom. Every stored nominal field and variant case must be
safe, including absent resource cases. Secrets, streams, resources, and unknown stored type parameters
reject. A function signature is a leaf because its future arguments and result are not stored values.
This supports generic graph-owned `function-compose<A,B,C>` using a private generic graph helper,
while leaving unconstrained factories that capture a bare `T` outside this slice. Task-port values
retain their separate authority and cannot enter a public pure environment.

Graph 11 changes expression meaning encoding. Type objects remain the single current version-10
codec, including magic, domains, bytes, and digests. Typed-data layout identities therefore remain
stable for unchanged layouts. The cutover materializes inspected maintained meaning through
`GraphRepository` in isolated repositories, compares old owners and exact type bytes, and selects
the new standard dependency explicitly. Operational data is outside this cutover.

The bounded evidence must distinguish correct output from independent admission, lifetime, tail-space,
and no-rescan proof. Wrong-prefix and restored-scan mutations must fail fixed expectations. A violation
of these public boundaries is grounds to revise the mechanism; it does not authorize retaining a
second editable body representation, adding task closures, inferring generic capture constraints,
or weakening the fixed acceptance obligations.

The later [capture-safe rank-one decision](20260909-capture-safe-rank-one-abstraction.md) permits
stored parameters only through an explicit exact-declaration constraint. It supersedes this slice's
lack of constrained parameters while retaining its rejection of unconstrained captures and its
lifetime, callable, effect and data boundaries.
