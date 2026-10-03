# Same-task owned transfer

A direct named graph task can consume and return owned memory within one runtime
invocation. This extends [buffers](owned-byte-buffers.md),
[Owned generics and static witnesses](owned-generics.md),
[products](owned-products.md) and [choices](owned-choices.md). It does not create
a child task, cross an invocation identity, enqueue a message or authorize I/O.

## Signature and call contract

Parameters have three ordered regions: ordinary data/callbacks, owned memory,
and exact capability resources. Any region may be empty. Owned arguments remain
exact live locals. A task's owned parameters must explicitly `consume`; even an
unused `borrow` parameter is invalid. A pure helper may still borrow and reborrow
synchronously, and cannot retain, return or consume a loan. A task may call such
a pure helper, with its loans ending before control returns to the task.

A direct result may be ByteBuffer, OwnedI64Cell, an owned product or choice, or an
exact in-scope Owned type parameter. Ordinary containers, callable descriptors,
partial application and capture do not gain permission to contain these values.
A first-order task can declare Owned type parameters and explicit implementation
witnesses without effect or requirement parameters. Concrete task effect rows are
supported, including the empty row. The subsequent [task-method extension](owned-generics.md#nominal-contracts-and-exact-static-operands)
also permits monomorphic task methods with exact closed rows and consuming Self
arguments. A witness selects that exact signature, never the caller's effect
allowance or an execution grant.

For example, this helper transfers one owner without inspecting or copying it:

```text
(function create relay (visibility public) (effect (task))
  (type-parameter create T (constraint owned))
  (parameter create value (type T) (use consume))
  (returns T)
  (body (local value)))
```

The type constraint and effect kind belong to accepted meaning, not this spelling.
A pure caller cannot invoke an empty-row task. Resource parameters remain a final
suffix, carry their own exact requirement/interface/use, and are checked by the
existing affine resource rules. An owned token never supplies a resource grant,
operation allowance, authority to retain a resource, or a persistence codec.
Repeated resource borrowing keeps its existing rules independently of memory.

## Custody, evaluation and failure

Arguments are evaluated in authored order. Consuming an owned local transfers its
single custodian and invalidates that local. A later argument failure disposes of
already evaluated transfers. A callee owns its consuming arguments for the call;
returning an owner transfers custody back to the caller under the same invocation
identity. Returning an ordinary result, implicit scope exit, a trap, cancellation
or a refused quota releases any remaining owned storage and synchronous loans.
Pending continuations retain their own live locals. Existing eligible tail-call
transfer does not require a copied payload or a new origin.

A refusal represented by an owned choice returns the owner only when its explicit
rejected arm contains that owner. An exception or cancellation is not such an arm,
and does not return an owner implicitly. Already completed external effects may
survive a later failure. No automatic retry, rollback, exactly-once delivery or
transaction spanning memory and capability adapters is promised.

Raw host ingress, adapter-created values and returned raw values cannot manufacture
or export a live owner. Tokens remain sealed to a fresh invocation. A Rust clone
is still an inert marker, not another custodian. JSON/data codecs, ports, constants,
stream elements and persistence retain their existing boundary restrictions.

## Admission and evidence boundary

Kernel validation, independent source classification, canonical preparation and
both evaluators enforce the signature and custody rules. Unused parameters,
unreachable applications and imported signatures are checked. Semantic validator
22 invalidated prior proof reuse for the initial same-task extension, without a
graph, owner/type, request, instruction or artifact format change. The subsequent
task-method extension uses semantic validator 23 and authored request codec 24;
the existing graph, instruction and artifact layouts remain unchanged.
Historical accepted content remains historical; new task-owned meaning is not
claimed executable by an older validator or host.

Literal public examples live in `tests/fixtures/owned-task-library.lkjc` and
`owned-task-consumer.lkjc`. The public task/resource fixture composes an owned
buffer with an exact DurableQueue lease. Source and executable tests separately
cover concrete and generic calls, selected witnesses, product/choice transfers,
source-free package artifacts, illegal loans, raw boundaries, traps, cancellation
and allocation refusal. A test design is not an acceptance result; current run
outcomes belong to the campaign and release owners.

Development 0.1.68 adds a separate [structured parallel boundary](structured-parallel.md):
two exact empty-effect child tasks consume owned inputs under fresh invocation
identities and join before continuation. Development 0.1.69 also returns closed
owned child results through a joined OwnedProduct, with separate child-to-parent
custody and complete consuming decomposition. The same-task rules above remain intact.
Channels, owner-returning send refusal and receiver lifecycle remain separate work.
