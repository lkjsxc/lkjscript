# Structured parallel tasks

Status: normative for development 0.1.68. Exact acceptance evidence belongs to the
[implementation campaign](../campaigns/20261002-structured-parallel.md).

## Meaning and admission

`Parallel(left, right)` is an expression in the accepted meaning graph. Both children
are direct named calls to graph-defined, monomorphic tasks with closed empty effect
rows. Native authoring spells this `(parallel (call LEFT ...) (call RIGHT ...))`.
The enclosing computation must be a task, even though neither child has an external
effect. A pure computation cannot acquire task authority through an empty row.

Each child accepts a prefix of closed ordinary data followed by a suffix of consumed
owned values. Each region may be empty. Owned inputs can be ByteBuffer, OwnedI64Cell,
or concrete nested owned products and choices. Every nested case and ordinary type
argument is checked, including unused choice cases and phantom nominal arguments.
Memory loans, capability resources, secrets, callables and unresolved generic
parameters cannot cross this boundary. The direct child declaration has no type,
effect, requirement or implementation parameters. Its body may call ordinary generic
libraries and select exact static implementation witnesses under the existing rules.
An external declaration or dynamic callable is not a child-task identity.

Both results must be closed ordinary data. The expression returns the existing
structural record type `{left: L, right: R}`. Records and collections gain no ability
to contain owners. Returning owners from children, task handles, detached lifetime,
channels and cross-instance communication are separate extensions.

Evaluate every left-call argument, then every right-call argument, exactly once in
the parent. Only after this preparation can either child body begin. An argument
failure prevents both child bodies; already moved owners are disposed of. Consuming
the same local twice is invalid even if the two children would run sequentially.
Effects performed by argument expressions retain their ordinary authored order and
failure behavior. Child effects are empty; no adapter or deployment grant is inherited.

## Custody and join

Each child has a fresh invocation memory identity and resource scope. A private
nonduplicable transfer envelope validates the exact prepared program, target task,
source identity, destination identity, type closure and absence of active loans.
The envelope is the sole custodian between parent and child. Destination adoption
updates every nested owned identity without cloning its live token or copying its
payload allocation. Existing immutable ordinary metadata may retain its sharing.
Raw host ingress still rejects owners and cannot construct a transfer certificate.

Normal child return disposes of unreturned owners. A trap, quota refusal or
cancellation also releases child locals, pending arguments and any unadopted or
partly adopted envelope. Failure requests cooperative cancellation of the sibling;
every started child is joined before the parent continues or reports failure. The
originating failure takes precedence over the sibling's consequent cancellation.
When both independently fail, no temporal ordering of those failures is promised.
This is not transactional rollback or an implicit retry.

The production evaluator may run child bodies concurrently. Available worker capacity
is acquired without waiting; the calling thread runs one child, and exhausted worker
capacity uses the caller for both. Nested groups therefore cannot wait for a worker
held by their own ancestor. Worker capacity is process-wide, finite, and remains held
until join. The current implementation uses scoped OS threads and a finite structured
nesting capacity; these are replaceable scheduling choices. A parallel expression
does not promise two available CPUs, a speedup, fairness or a permanent thread per task.

The reference evaluator independently admits and evaluates canonical child calls in
authored order. Its serial result and ownership checks are a semantic oracle, not
evidence of overlapping production execution. The sealed physical token custodian is
shared; reference ordinary-type/value admission remains independently implemented.

## Resource accounting and proof

An optional cumulative instruction, allocation or collection quota belongs to the
whole invocation. The first group seeds a shared ledger with prior parent usage;
all children and subsequent parent execution continue charging that ledger. A nested
group cannot reset fuel or gain another allocation allowance. Charges precede modeled
growth and are not refunded after cleanup. Call-depth policy includes active ancestor
frames, and structured nesting has an independent finite native-stack bound.
Cancellation and deadlines use the invocation control throughout the group. Ordinary
trusted execution retains its existing unmetered cumulative-work default.

Production observations aggregate child work and expose `parallel_scopes` and
`parallel_workers_spawned`. These are invocation totals, not maximum simultaneous
workers. Allocated bytes model admitted cumulative storage, not RSS or allocator
overhead; thread stacks and system scheduling costs are not claimed as that metric.
Admission must check the complete canonical closure before live execution, including
unused task declarations and untaken expressions. Strict artifact loading also
reconstructs canonical control, so retargeting or erasing a compiled parallel
instruction cannot be authorized by recomputing hashes.

Graph 21 adds the expression and preserves supported historical meaning readers.
Compiler 19, bytecode 15 and artifact 26 require rebuilding earlier derived artifacts
from retained accepted meaning. Authored request 25, compact discovery 29, function
projection 11, CLI observations 35 and semantic validator 24 expose the increment.
Existing nonparallel authored intent retains its bytes. This does not upgrade exact
package selections, replace running services or publish a new public executable.

Acceptance separates canonical and independent semantic checks, rehashed-artifact
attacks, nested allocation identity and cleanup tests, controlled real-child overlap,
aggregate quotas, and copied-executable native cross-package computation. Each proves
a different boundary; none alone establishes all concurrency properties.
