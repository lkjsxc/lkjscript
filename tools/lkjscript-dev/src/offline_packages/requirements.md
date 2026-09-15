# Requirement-parametric ordinary-library witness

These literal requests define a designed composition workload, not application adoption.
`requirements.producer.{lkjc,structural.lkjc}` owns a generic typed-cell algorithm. The observer
supplies observed bases, exact package selections, executable arguments and disposable operational
resources. It does not generate the library body or perform its cell updates.

The public library API is:

```text
try-update<T; E; R>(space: StaticText, default: T,
    transform: TaskFn(T)->T ! E, key: List<DataKeyPart>)
    -> TransactionOutcome<T> ! ({R} union E)

make-cell-updater<T: capture-safe; E; R>(space, default, transform)
    -> TaskFn(List<DataKeyPart>)->TransactionOutcome<T> ! ({R} union E)
```

The second function is pure. It binds the immutable prefix; the consumer also binds the key and
invokes the resulting zero-argument task. The core imposes no capture constraint. An empty-row task
callback remains a task and cannot be invoked from pure execution. Selection and factory creation
grant no execution authority.

R constrains the exact standard `DataStore` interface and at least `get`, `put`, `transaction`.
The counter requirement also permits `schema-read`; a callback explicitly using that whole
requirement may perform that operation. The formal minimum does not attenuate its argument.
The Text callback uses a separately supplied Configuration requirement through E and its own grant.

The core computes, encodes and conditionally writes inside `transaction-outcome`, then returns its
completed result directly. The expression binds the exact ordinary standard `TransactionOutcome`
and `TransactionAbortReason` declarations and all four cases through public named references.
Successful completion yields `Committed(T)`; a failed condition or commit-time conflict yields
`Aborted(ConditionFailed|Conflict)` without a candidate payload. These are constructible nominal
values. They are not execution authority or unforgeable receipts. The guarantee belongs to
evaluation of the lexical expression.

The initial direct and bound I64 paths advance 10 to 13 to 16; the Text paths advance `a` to `a!`
to `a!!`. The decisive false callback fails `Missing` against an existing auxiliary record and
returns normally; the primary write can still report true, while the library returns
`Aborted(ConditionFailed)` and the store's data and HEAD stay unchanged. Another callback stages
successful writes both before and after its failed condition; all those writes remain absent.
A trapping callback stages a write then divides by zero, produces no normal outcome and never
reaches the primary write. A multi-key successful update publishes both independently expected
values and returns `Committed(13)`.

The additional literal consumer pair, `requirements.outcomes.consumer.{flat,structural}.lkjc`,
matches both ordinary outcome variants and both abort-reason variants. It also checks successful
no-write false and unit bodies, a read-only snapshot, a false-condition-only transaction and an
unrelated application-defined nominal payload through ordinary typed-data encode/decode. No-write completion creates no new revision and
does not promise latest-HEAD revalidation. A callback completes a transaction through a separately
granted store before failing the outer store's condition; the inner store's `survived` Text value
remains visible. The helper adds no retry, savepoint or distributed rollback.

Absent values and incompatible typed-data encoding/layout use the supplied default. Cancellation,
quota exhaustion and execution failures are not defaults. The closed
`TransactionOutcome<Fn(I64)->I64>` external result rejects before an unavailable secret or either
uncreated store is opened. Capture safety and transient nominal wrapping grant no serialization
eligibility. The caller continues to own keys, spaces, schema and migration policy.

The observer derives only the independent frozen TypeObject 10 I64/Text scalar envelopes and
compares complete stored bytes after opening fresh snapshots. It retains every state observation,
including unchanged HEAD on abort/no-write and a distinct HEAD for the independently completed
store. It never commits an observer transaction. Two public imported updates use a distinct
component with a DataStore grant and an HTTP callback grant limited to one call each. An owned
loopback barrier waits for both GET requests before sending either response, placing both bodies
on the same initial snapshot. Exactly one returns `Committed(13|23)` and the other
`Aborted(Conflict)`. Independent HEAD/history/reopen observations require one winning key and
one new revision; the host only schedules replies and observes state. Runtime cancellation, reentry and
prepublication wrapper-capacity cases use the existing independent controlled adapters and the
physical data owner's HEAD/reopen/fault tests; no live callback is replayed differentially.

The supplier edit raises both minimum operation constraints and replaces the pure factory and
substantial generic update bodies through reviewed public input. Its handwritten flat and
structural update replacements transform twice before encoding. The repaired I64 path advances
16 to 22; the repaired Text path advances `a!!` to `a!!!!` with two Configuration reads. An
insufficient dependency replacement rejects without changing consumer authority; an explicit
requirement edit and replacement succeeds. Original and repaired artifacts run after producer
and consumer authoring paths are unavailable. A moved owned consumer copy supports public recovery.

The public workflow plans both complete literal notations against the same base and exact
declaration/reference prelude. Their strict canonical reviews must agree byte-for-byte, including
typed reference inventories, commitments, allocated identities and retirements. Structural apply
consumes the flat plan token. Public inspection verifies retained function, parameter and T/E/R
identities while the old body owners retire. Both notations are retained as literal proof inputs;
the reader requires them, the exact artifacts, completed results and independent data observations.

The former `attempt-update -> UpdateAttempt<T>` API is preserved as authentic v0.1.36 source,
package and artifact material under `tests/fixtures/transaction-outcome-predecessor`. It keeps its
honest candidate-plus-primary-boolean meaning. Original, rebuilt and imported old artifacts run
against separate fresh stores and must preserve that result and publication behavior. The new
library result is an explicit API change in this designed workload. The earlier authentic scalar
predecessor fixture also remains unchanged.

Resource composition continues to use the public `DurableQueue` helper in
`requirements.resource-library.structural.lkjc`. Transported execution changes the independently
observed raw queue record from ready to completed with one attempt and exact result bytes, then
reports no available lease without changing the record. Existing controlled resource scripts
exercise matching, borrowing, consumption, cancellation and exact accounting in both evaluators.
