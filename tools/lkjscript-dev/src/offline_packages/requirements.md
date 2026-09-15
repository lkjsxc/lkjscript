# Requirement-parametric ordinary-library witness

These literal requests are a designed composition workload, not evidence of existing application
adoption. `requirements.producer.structural.lkjc` owns the typed-cell algorithm. The observer supplies only
observed bases, exact package selections, executable arguments and isolated operational resources;
it neither builds the semantic body nor performs the cell updates. Resource composition also has
a public `DurableQueue` helper in `requirements.resource-library.structural.lkjc`. Its transported execution
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
both minimum constraints and the pure factory body. Its independently handwritten
`requirements.update-body.{flat,structural}.lkjc` edit replaces the substantial generic update
body so it applies the supplied transformation twice before encoding and conditionally writing.
This is an intentional supplier behavior change: the repaired I64 invocation advances 16 to 22,
and the repaired Text invocation advances `a!!` to `a!!!!` with two Configuration reads. The
original artifacts retain single transformation and all original false-condition/trap outcomes.
The edit adds no transaction retry. An insufficient dependency replacement leaves consumer
authority unchanged, and a sufficient public requirement edit/rebind succeeds. Original
and repaired artifacts run after the producer and consumer authoring paths are removed; the moved
owned consumer copy supports public recovery checks.

The original flat producer, consumer, queue and supplier-edit literals remain unchanged as an
independent authoring oracle. The public workflow plans both complete literal notations against
the same base, declaration/reference prelude and request controls. Strict canonical review files
must be byte-identical, including request commitments, typed inventories, allocated identities,
candidate owners and retirements. It then applies the structural request with the flat plan token.
Only the resulting structural workload performs the live cell and queue effects.

Structural blocks keep explicit type/effect/requirement applications, binder Names and annotations.
The producer uses sequential let bindings within its transaction, and the queue helper confines
the affine lease to its match arm before borrowing and consuming it. The stronger supplier edit
discovers both functions and all referenced parameters by their existing owners, then replaces the
factory and update bodies. Public definition inspection verifies all seven factory and eight update
signature identities persist. The factory's five old expression owners retire; the update's 41 old
expression/binding owners retire. The new bodies have eight and 43 distinct expression/binding
owners respectively. The
`requirement_structural` observation binds these commands and the retained literal requests/reviews;
the existing receipt reader independently admits those files.

The flat producer has 145 nonempty lines, 41 expression definitions and 20 indexed expression
argument edges; its structural counterpart has 107 lines and two exported block roots. The flat
consumer has 273 lines, 88 expression definitions and 63 argument edges; its structural counterpart
has 182 lines and 11 exported roots. Substantial bodies no longer require per-occurrence labels or
indexed expression edges. Declarations, references, types, effect rows, constraints and dependency
selections still have explicit preludes. These counts describe input assembly; they do not measure
authoring time, model tokens, billing or parser speed.
