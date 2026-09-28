# Ordinary generics with scoped capability resources

This guide describes development **v0.1.57**. Check [current availability](../status.md)
before selecting an executable; an older public runtime does not gain these
semantics by reading this document.

The [complete literal queue program](../../tests/fixtures/type-generic-resources.lkjc)
uses one reader with both integer-list and Unicode-text decoders. It is ordinary
lkjscript authoring input, not a Rust generator or another execution engine.
The [native command workflow](native-command.md) explains binding a request to
the current revision, planning, applying, checking and building accepted meaning.
The literal here supplies the `declarations.begin`/`declarations.end` block; prefix
it with `request base=REVISION` from the actual new project's status.

## Separate data types from authority

`read-lease<T>` receives an ordinary `Bytes -> T` decoder and a final borrowed
DurableQueue resource. `relay<U>` forwards both U and the borrowed view to that
reader. Each helper is private and its task effect names the exact `queue::jobs`
requirement. The resource itself is not T, and T does not select a deployment grant.

The native signature spells those independent choices explicitly:

```text
(type-parameter create T)
(parameter create decode (type (function (Bytes) T)))
(parameter create lease (type (resource std::DurableQueue))
  (use borrow) (requirement queue::jobs))
(returns T)
(effect (task (requirement queue::jobs)))
```

These are clauses inside a function declaration, not a complete request. A direct
nested call forwards the type with `(types U)`. The complete program also has a
`finish<V>` helper: it receives ordinary V, consumes the final owned lease using
`(use consume)`, and returns V after the queue completion operation.

Borrowing preserves the caller's ownership. It does not promise purity, read-only
memory, or zero-copy payload decoding. Only the declared non-consuming capability
operations are available through the view; a borrowed view cannot be promoted
into the consuming argument of `finish`.

## Observable behavior

The [public integration tests](../../tests/public_cli/native_generic_resources.rs)
author this input, draft it back without its original request, re-plan the unchanged
meaning, and build a detached artifact. Its first `numbers` invocation returns
`[7, 42, -3]`; its first `text` invocation returns `"日本語 + generic"`. Repeated
invocations return their absent defaults because those same jobs are completed.
The tests separately inspect durable job bytes, not just the helpers' return values.

The [failure test](../../tests/public_cli/native_generic_resources_failure.rs)
replaces the numeric decoder with division by zero inside the nested borrow. It
requires a runtime error, a still-leased durable job without a completion result,
and successful processing of a different ready job. Local resource cleanup cannot
undo an already committed queue claim and does not authorize blind replay.

## Recursive helpers

A direct or mutually recursive helper can now retain the same exact borrow or
consume contract. There is no new loop primitive or unchecked transfer. Each
function body is checked once under its declared use modes; every recursive edge
borrows a view or moves the owner in the same way as an ordinary direct call.

The [recursive native input](../../tests/fixtures/recursive-resources.lkjc) adds an
ordinary `remaining: I64` before the decoder and final lease. `read-lease<T>`
counts down through 4,097 tail calls; `relay<U>` and `alternate<U>` mutually
reborrow, then read the caller's still-live lease after the recursive return.
`finish<V>` moves the owner recursively and completes the durable job only at its
base case. Its ordinary result returns through pending non-tail activations.
The [public cases](../../tests/public_cli/native_recursive_resources.rs) also
exercise mutual consuming helpers, recursive decoder failure, detached execution,
and rejected post-transfer reuse and borrow escalation.

Recursion is not a termination proof. Pending non-tail calls remain subject to
the existing live-depth limit, and tail transfers still charge execution work and
check cancellation. Failure unwinds invocation-owned resources without completing
or retrying a durable job. The capability borrow is not a general memory reference
and does not promise payload copies have been eliminated.

## Public libraries and exact authority

The [library input](../../tests/fixtures/package-resources-library.lkjc) exports a
recursive `read-lease<T>` borrower and a `finish<V>` consumer. Export its accepted
package with `package current export --kind transport --output library.lkjp`, then
stage that exact transport in a fresh command project. Use the exported package,
semantic revision and package revision in `add.dependency`; `(use lib PACKAGE
PACKAGE_REVISION)` binds public names. The [consumer input](../../tests/fixtures/package-resources-consumer.lkjc)
then calls `lib::read-lease` and `lib::finish`, with its own public forwarders.
The [package workflow](native-library.md) and [executable integration
fixture](../../tests/public_cli/native_package_resource_fixture.rs) retain the full
export/stage/author sequence.

In that consumer, a resource parameter uses `(requirement lib::queue::jobs)` and
its task effect names the same exact imported requirement. The entry port explicitly
includes `(row (requirement lib::queue::jobs))`; its component need not invent a
new local queue requirement. The deployment still needs a `jobs` grant. The
supplier's unused `unused-jobs` requirement does not become a consumer obligation,
and an extra grant for it is rejected. A same-name local queue is not equivalent
to this imported authority. The current name-based deployment descriptor cannot
address two different obligations with the same name; use distinct names.

The [package cases](../../tests/public_cli/native_package_resources.rs) repeat the
list/text workload after removing both source projects, and after forwarding
through a third independently exported package. The failed-borrow case retains
the leased durable job rather than completing or retrying it. An HTTP case exercises
the same imported authority across server startup, shutdown and restart. Ordinary
use of these libraries requires no Rust or Python generator and no compiler API.

## Deliberate limits

There is one final direct resource parameter, exact concrete authority, and a
direct call admitted by the normal package visibility rules. Ordinary arguments,
callbacks and results must remain resource-free. Effect/requirement-polymorphic
resource signatures, escaping views, indirect resource calls and asynchronous
borrowing are not supported. Automatic function extraction retains
its narrower nongeneric, consume-only eligibility.

See the [language contract](../spec/language.md#affine-capability-resources) and
[verification obligations](../spec/verification.md) for the owning rules. This
increment composes two existing language facilities; it is not a general lifetime
system, region allocator, trait system or a measured performance improvement.
