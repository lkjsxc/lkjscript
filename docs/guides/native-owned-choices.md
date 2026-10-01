# Recoverable owned outcomes

Use an explicit owned choice when different results carry different ownership:
for example, an ordinary success value or the original buffer/cell returned after
an operation declines it. Ordinary Option/Result types cannot be used to hide an
owned payload. This feature does not catch runtime traps or roll back effects.

The [literal generic library](../../tests/fixtures/owned-choices-outcome.lkjc)
defines `attempt<T:Owned>` and `recover<T:Owned>` with explicit static implementation
parameters. Accepted operations return the selected implementation's read result;
rejected operations return their original T. The caller can choose what to do with
that original owner rather than losing it inside an error flag.

## Construction and consuming case analysis

```text
(function create reject (visibility public) (effect pure)
  (type-parameter create T (constraint owned))
  (parameter create original (type T) (use consume))
  (returns (owned-choice (case accepted I64) (case rejected T)))
  (body
    (choose-owned
      (type (owned-choice (case accepted I64) (case rejected T)))
      (case rejected) (local original))))
```

The selected owned payload must be a live owning local. Bind an owned temporary
before selecting a case; do not pass it directly to `choose-owned`. An ordinary
case may evaluate an ordinary expression. The whole choice remains affine even
when that expression produces a scalar or Unit.

```text
(match-owned
  (type (owned-choice (case accepted I64) (case rejected OwnedI64Cell)))
  (local outcome)
  (case accepted (binding value (type I64)) (in (local value)))
  (case rejected (binding original (type OwnedI64Cell))
    (in (call cell::read (local original)))))
```

The second example assumes the exact scalar library is imported as `cell` and
`outcome` is a live local of the annotated type. It consumes the parent. Only the
selected arm runs, and its payload binding exists only within that arm. Both arms
must be present and return the same type. The read in the rejected arm is borrowed;
its remaining owner is released when the arm exits. A different body may transfer
the owner into another operation or return an owned result instead.

At least one case must have an owned payload. Other payload types must be closed
ordinary first-order data. Scoped Owned type parameters, nested choices and owned
products compose; open ordinary metadata parameters and arbitrary owned containers
do not. Case names and type/arm order are canonicalized, but separate same-name
payload bindings keep distinct lexical identities. An arm cannot use another
arm's binding, and a consumed choice cannot be selected again.

## Ordinary public workflow and exact implementation selection

Each fixture is a native declarations request body. Start a minimal project, read
its current `status`, prepend `request base=REVISION`, then use `change plan` and
`change apply`. To share the library, use `package current export --kind transport`,
stage its exact transport, add its exact dependency, and bind native `use` aliases
to the observed package and revision. The [library guide](native-library.md) covers
the complete operation sequence; no compiler-internal graph editing is required.

The [public CLI acceptance case](../../tests/public_cli/native_owned_choices.rs)
uses four independently created packages: generic contracts/outcomes, a scalar
implementation, a byte-buffer implementation, and the
[consumer](../../tests/fixtures/owned-choices-consumer.lkjc). It rejects a wrong-Self
implementation without changing the accepted revision, then performs canonical
draft re-entry, an identity-preserving literal edit, native checks and an artifact
build. It executes both branches at the signed integer extremes after removing
all source projects and package transport files.

Concrete implementation operands are explicit, for example
`(implementations concrete@cell::Scalar)`. A `parameter@FUNCTION@ID` operand forwards
an existing exact implementation parameter. The test also uses an alternate read
implementation for the same scalar Self: accepted results use that selection,
while rejected outcomes retain the original scalar value and may be consumed by a
different, explicitly selected compatible implementation. Nothing implicitly
chooses a global implementation by name, and no witness grants capability authority.

The [composition fixture](../../tests/fixtures/owned-choices.lkjc) nests a choice
inside a product inside another choice and restores the selected payload. The
[closed-Self fixture](../../tests/fixtures/owned-choices-witness.lkjc) implements
an exact consuming method for a three-case choice and exercises recursive generic
transfer and synchronous whole-owner reborrowing. Whole-choice loans cannot inspect
or select a child. Field borrowing and asynchronous ownership transfer are not
provided by these examples.

## Failure and availability

Dropping an unused choice releases its selected owned payload. A cancellation or
allocation-admission failure also uses ordinary joined cleanup; it is not
converted into the example's rejected case. Independently performed effects may
survive, so a returned payload alone does not prove that retrying an operation is
safe. Future asynchronous queues require their own acceptance and cancellation
contracts before this type can be used for message transfer.

Choices cannot cross raw host, JSON/data, durable session or retained callable
boundaries. Return ordinary data at command ports. Rebuild predecessor derived
artifacts with the current compiler; accepted source graphs remain separate from
operational data and running services.

See the [specification](../spec/owned-choices.md) for the exact contract and the
[continuation](../campaigns/20261002-owned-choices.md) for observed verification and
delivery. Development source support does not establish a public binary release.
