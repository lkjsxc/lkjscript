# Ordinary generics with scoped capability resources

This guide describes development **v0.1.55**. Check [current availability](../status.md)
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

## Deliberate limits

There is one final direct resource parameter, exact concrete authority, and a
private same-package acyclic direct call. Ordinary arguments, callbacks and results
must remain resource-free. Effect/requirement-polymorphic resource signatures,
escaping views, indirect resource calls, cross-package resource transfer and
asynchronous borrowing are not supported. Automatic function extraction retains
its narrower nongeneric, consume-only eligibility.

See the [language contract](../spec/language.md#affine-capability-resources) and
[verification obligations](../spec/verification.md) for the owning rules. This
increment composes two existing language facilities; it is not a general lifetime
system, region allocator, trait system or a measured performance improvement.
