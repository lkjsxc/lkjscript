# Requirement-parametric ordinary-library witness

These literal requests are a designed composition workload, not evidence of existing application
adoption. `requirements.producer.lkjc` owns the typed-cell algorithm. The observer supplies only
observed bases, exact package selections, executable arguments and isolated operational resources;
it neither builds the semantic body nor performs the cell updates. Resource composition also has
a public `DurableQueue` helper in `requirements.resource-library.lkjc`. Its transported execution
changes an independently observed raw queue record from ready to completed with one attempt and
the exact result bytes, then reports no available lease without changing that record. The existing
service wire observer supplies these expected transitions. Separate deterministic resource scripts
exercise local matching, borrowing, consumption, cancellation and exact accounting in both execution
owners; those adapters are disjoint from live effects.

The public library API is:

```text
attempt-update<T; E; R>(space: StaticText, default: T,
    transform: TaskFn(T)->T ! E, key: List<DataKeyPart>)
    -> UpdateAttempt<T> ! ({R} union E)

make-cell-updater<T: capture-safe; E; R>(space, default, transform)
    -> TaskFn(List<DataKeyPart>)->UpdateAttempt<T> ! ({R} union E)
```

The second function is pure. It binds the immutable prefix; a consumer may bind the remaining key
and invoke the resulting zero-argument task. The core imposes no capture constraint. The callback
uses the existing exact task-callable kind, including an empty effect row for a calculation with no
capability use. An empty-row task is still a task and cannot be invoked from pure execution.

R constrains the exact standard DataStore interface and at least `get`, `put`, `transaction`.
The initial consumer's counter requirement also permits `schema-read`. A callback explicitly using
that whole requirement may call `schema-read`; the formal minimum does not attenuate its argument.
The text callback instead uses a separate Configuration requirement supplied through E and its own
deployment grant. Selection and factory construction grant no authority.

`UpdateAttempt<T>` is an ordinary nominal record. Its `candidate` is the transformed value and
`primary_condition_matched` is the boolean from the primary conditional write, returned after normal
lexical completion. They are not a persistence certificate. The false-auxiliary-condition callback
first receives false from `Missing` on an existing key, then returns normally. The primary write
reports true while the entire transaction publishes nothing. The trapping callback stages an
auxiliary write and then divides by zero; its auxiliary write rolls back, and the later primary
write is never reached. Earlier completed invocations remain visible.

Absent values and incompatible typed-data encoding/layout use the supplied default. This is the
typed-data codec, not JSON; cancellation, quota exhaustion and execution failures are not defaults.
The closed `UpdateAttempt<Fn(I64)->I64>` external result rejects before an unavailable secret or
either uncreated store is opened. Closed unencodable applications retain pre-effect rejection.
Capture safety is not serialization.
The caller owns keys, spaces, schemas and migrations. The helper neither retries nor coordinates
another store or Configuration/HTTP effects into distributed rollback.

The observer compares I64/Text bytes using independent frozen TypeObject 10 scalar identities and
the typed-value envelope contract, then reads the ordered store directly without committing an
observer transaction. It saves observations after each invocation, preserves failures, and creates
fresh owned state for a new invocation after an observer correction. The repaired producer changes
both minimum constraints and the pure factory body; an insufficient dependency replacement leaves
consumer authority unchanged, and a sufficient public requirement edit/rebind succeeds. Original
and repaired artifacts run after the producer and consumer authoring paths are removed; the moved
owned consumer copy supports public recovery checks.

The flat expression syntax requires a separate expression occurrence for each read and explicit
argument edges. The checked-in requests retain this friction for a later authoring assessment;
line/edge counts do not measure authoring time, model tokens or billing. This campaign does not add
a parser block notation or a typed-cell intrinsic.
