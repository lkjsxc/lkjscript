# Semantic Source: Complete Schema

[Authority](../semantic-source-and-agent-protocol.md)

## Purpose

Close the complete the removed legacy source contract source schema and define honest correlation to
compiler and execution facts under the single canonical source contract.

## Status

**Historical identity; Current preserved base.** The exact closed the removed legacy source contract
node/value/type representation defined here is the unchanged base of Current
`lkjscript.semantic-source`. Input version 1 is rejected without an alias.
Agent Foundation legacy contract is also historical.

## Closed Source Schema

The preserved Complete Schema base represents every Current the removed legacy source contract
concept with closed typed nodes and no generic fallback object:

- source unit and edition identity, import, main, function, signature,
  parameters, type variables, bounds, and exact type forms;
- product, product field, marker trait, and implementation;
- Unit, Bool, I64, F64, string, empty-list, Option, and Result literals and
  constructors;
- name reference, let/bind, var/set, if, while, do, and quote;
- product construction, field access, and immutable replacement;
- list, Option, Result, and every Current built-in call form;
- ownership move, borrow, and borrow-mut;
- comment, blank line, source origin, exact span, and trivia attachment.

Schema appends the Current development typed-hole node/value and response
records without changing any base variant or field.

Kinds are closed enums. The schema preserves one canonical tree and
deterministic the removed legacy source contract projection. Its complete kind/field table is tested
against all 125 tracked `.lkjscript` files, including all 121 under `src/`.

Every union is closed. Unknown kind, field, enum variant, schema, version,
attachment position, duplicate field, malformed value, or trailing input fails.
Fields are required or explicitly optional by schema; absent never means
unknown extension. Inferred types, bindings, effects, ownership, control,
layouts, proofs, or backend data are derived facts and are never serialized as
source authority.

Comments and blank lines attach exactly as leading trivia to the next semantic
node in the same container, or as trailing trivia of that container when no
next node exists. Trivia order and exact bytes are preserved. Trivia never
changes semantic child order or source identity.

## Correlation Facts

Each available correlation fact states:

- producer identity and build identity;
- fact schema and version;
- pinned source revision and derived-artifact revision;
- certainty: `guaranteed`, `conditional`, or `informational`; and
- when unavailable, a typed reason rather than an invented value.

Facts map source nodes to exact resolved HIR, verified SSA, CFG block/edge,
frame, safepoint/root, layout/runtime-layout, proof/certificate, bytecode, and
native code/metadata identities only where the producing pipeline actually has
that mapping. One-to-zero, one-to-one, and one-to-many cardinality is explicit.
Optimization deletion or fusion is represented, not reverse-engineered.

## Registration

`removed Semantic Source contract` was emitted after complete vocabulary,
schema/source roundtrip, malformed-boundary, correlation, diagnostic, and
transaction gates. It is now historical and rejected. The older
Removed agent-foundation identities are historical evidence, not a competing
Current source schema. No generation-numbered agent schema is emitted.

Current Schema reuses this closure discipline and has identity
`lkjscript.semantic-source`. It preserves the legacy contract representation as its base
rather than mutating or overloading the legacy contract identity, and closes the Current
the canonical source contract enum, match, Never type-node, typed loop, return, break, continue,
value-bearing trap, and exit variants.
