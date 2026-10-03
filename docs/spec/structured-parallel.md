# Structured parallel tasks

Status: normative for development 0.1.72. Current acceptance and publication state
belong to [status](../status.md).

## Meaning and admission

`Parallel(left, right)` is an expression in the accepted meaning graph. Both children
are direct named applications of graph-defined tasks with closed empty effect
rows. Native authoring spells this `(parallel LEFT-CALL RIGHT-CALL)`. Each child
uses `call` or `implementation-call` with exact type and implementation arguments;
a monomorphic call is the empty application. A generic enclosing task may supply
its own [explicitly transferable types](transferable-types.md) and exact lexical
implementation parameters. Every application becomes concrete before execution.
The enclosing computation must be a task, even though neither child has an external
effect. A pure computation cannot acquire task authority through an empty row.

Each child accepts a prefix of transferable ordinary data followed by a suffix of consumed
owned values. Each region may be empty. Owned inputs can be ByteBuffer, OwnedI64Cell,
or concrete nested owned products and choices. Every nested case and ordinary type
argument is checked, including unused choice cases and phantom nominal arguments.
Memory loans, capability resources, secrets and callables cannot cross this boundary.
A child may declare type and implementation parameters; symbolic boundary arguments
must prove their obligations under the exact caller scope. Even a phantom actual
type must independently satisfy ordinary or owned transfer admission. Unconstrained,
capture-safe-only and owned-only caller parameters cannot establish transferability.
A child has no effect or requirement parameters. Its body may forward its admitted
types and exact witnesses through generic libraries under the existing rules.
An external declaration or dynamic callable is not a child-task identity.

Each result must prove ordinary or owned transferability and become a closed admitted
type at execution. If both results are ordinary, the expression returns
the existing structural record type `{left: L, right: R}`. If either result is owned,
it returns `(owned-product (field left L) (field right R))`, including the ordinary
field in a mixed pair. Return it directly or bind it to an explicitly typed owned
local; `unpack-owned` consumes and decomposes the pair under the existing aggregate
rules. Its returned owners can
then be borrowed, mutated, consumed or passed into another group. Records and
collections gain no ability to contain owners. Every result case and type argument
is admitted, including inactive cases. Task handles, detached lifetime, channels
and cross-instance communication are separate extensions.

Evaluate every left-call argument, then every right-call argument, exactly once in
the parent. Only after this preparation can either child body begin. An argument
failure prevents both child bodies; already moved owners are disposed of. Consuming
the same local twice is invalid even if the two children would run sequentially.
Effects performed by argument expressions retain their ordinary authored order and
failure behavior. Child effects are empty; no adapter or deployment grant is inherited.

## Custody and join

Each child has a fresh invocation memory identity and resource scope. A private
nonduplicable transfer envelope validates the exact prepared program, target task,
ordered concrete type arguments, source identity, destination identity, type closure
and absence of active loans. The prepared function instance also binds the ordered
nominal implementation references; equal Self types do not make two implementations
interchangeable.
The envelope is the sole custodian between parent and child. Destination adoption
updates every nested owned identity without cloning its live token or copying its
payload allocation. Existing immutable ordinary metadata may retain its sharing.
Raw host ingress still rejects owners and cannot construct a transfer certificate.

A separate nonduplicable result envelope owns each returned carrier between child
local cleanup and parent adoption. It validates the exact prepared program, child
task application, instantiated canonical result type, child source identity, parent destination identity and
absence of active loans. A child may return an input owner or newly allocated owned
storage; neither path copies its payload. Both child results must succeed, and the
parent must reserve the result carrier's modeled storage, before result adoption
and pair construction. Adoption restores every nested owned identity to the parent
without cloning live tokens. An owned choice can explicitly retain an owner on a
typed rejection branch. It remains an owned result when its selected alternative
contains ordinary data.

Normal child return disposes of unreturned owners while the result envelope keeps
returned owners alive. A trap, quota refusal or cancellation releases child locals,
pending arguments and any unadopted or partly adopted input or result envelope.
This includes a successful sibling's result when the other child fails, and all
returned storage when parent adoption or result-pair allocation fails.
Failure requests cooperative cancellation of the sibling;
every started child is joined before the parent continues or reports failure. The
originating failure takes precedence over the sibling's consequent cancellation.
When both independently fail, no temporal ordering of those failures is promised.
This is not transactional rollback or an implicit retry.

The production evaluator may run child bodies concurrently. An explicitly owned
executor reuses auxiliary workers through bounded per-worker mailboxes. Available
capacity is reserved without waiting; the calling thread runs the left child.
An unavailable worker, closed dispatch, or refusal before submission leaves the
right job intact for execution on the caller. This is its first execution, not a
retry. Nested groups therefore cannot wait for capacity held by their ancestor.
An accepted dispatch retains its reservation until its completion receipt is joined.

Workers are created lazily, up to a process-wide physical ceiling of available CPU
parallelism minus one; unavailable CPU discovery selects zero auxiliary workers.
Zero capacity is valid serial execution. Separate runtime owners share only this
capacity accounting and may execute inline while another owner retains workers.
Workers never run unrelated jobs while waiting for their own child. The finite
structured nesting bound remains independent of cumulative quotas.

Each job owns a shared handle to the admitted immutable program, its control and
quota handles, exact application and sealed inputs. Program metadata and owned
payloads are not cloned for dispatch. Application grants, secrets, adapters,
cancellation lineage, locals and resources remain private to their invocation.
List/map thread-local observations are scoped to each dispatched job and restored
on every exit, including panic; worker reuse cannot inherit previous work counts.

The executor owner is separate from dispatch handles and cannot travel inside a
job. Stopping closes dispatch, drains and joins invocation scopes, then closes
mailboxes and joins every worker before releasing process capacity. Caller unwind
cancels and joins an accepted child before disposing its result and reservation.
A caught child panic reports infrastructure failure after cleanup; the worker may
then execute another job with fresh invocation state. No detached cleanup is success.

These workers supplement the existing resident root executor. The auxiliary ceiling
does not bound total root-plus-child CPU execution, provide CPU reservations or
establish fairness. A parallel expression does not promise two available CPUs,
speedup or a permanent thread per task. Scheduling remains a derived mechanism.

The reference evaluator independently admits and evaluates canonical child calls in
authored order. Its serial result and ownership checks are a semantic oracle, not
evidence of overlapping production execution. The sealed physical token custodian is
shared; reference ordinary-type/value admission remains independently implemented.

## Resource accounting and proof

An optional cumulative instruction, allocation or collection quota belongs to the
whole invocation. The first group seeds a shared ledger with prior parent usage;
all children and subsequent parent execution continue charging that ledger. A nested
group cannot reset fuel or gain another allocation allowance. Charges precede modeled
growth and are not refunded after cleanup. Generic substitution scratch and retained
concrete child signatures are charged before allocation. The non-generic path needs
no substitution map or concrete-signature vector. Call-depth policy includes active ancestor
frames, and structured nesting has an independent finite native-stack bound.
Cancellation and deadlines use the invocation control throughout the group. Ordinary
trusted execution retains its existing unmetered cumulative-work default.

Production observations aggregate child work and expose `parallel_scopes`,
`parallel_worker_dispatches` and `parallel_inline_fallbacks`. Dispatches count
accepted off-thread jobs, including reuse; fallbacks count pairs executed on their
caller. They are invocation totals, not physical thread starts or simultaneous
workers. Pool observations separately report starts, active/peak dispatches,
completions, inline fallbacks, and remaining/joined workers. CLI observation 36
intentionally replaces the former `parallel_workers_spawned` field rather than
changing its meaning. Allocated bytes model admitted cumulative storage, not RSS
or allocator overhead; thread stacks and system scheduling costs are not that metric.
Admission must check the complete canonical closure before live execution, including
unused task declarations and untaken expressions. Strict artifact loading also
reconstructs canonical control, so retargeting or erasing a compiled parallel
instruction cannot be authorized by recomputing hashes.

Graph 21 introduced the expression; Graph 22 adds explicit transferable constraints.
Compiler 22, bytecode 17 and artifact 29 carry child type and implementation operands
alongside the parallel-result type. Strict loaders reconstruct these from canonical
applications, including unused declarations and untaken branches. Validator 27
renews admission. Compiler-unit 21 and earlier reject before current payload decoding;
earlier derived artifacts require rebuilding from accepted meaning. Transfer-bearing
requests select authored codec 26; compact discovery 30 and function projection 12
advertise the constraints. Worker reuse changes no semantic, compiler or artifact
encoding; CLI observations 36 and shared-runtime observations 2 describe its lifecycle.
Existing nonparallel authored intent retains its bytes. This does not upgrade exact
package selections, replace running services or publish a new public executable.

Acceptance separates canonical and independent semantic checks, rehashed-artifact
attacks including forged result types and substituted child types or witnesses, nested allocation identity through both
directions and cleanup tests, controlled real-child overlap, aggregate quotas, and
copied-executable native cross-package computation with complete returned payloads. Each proves
a different boundary; none alone establishes all concurrency properties.
