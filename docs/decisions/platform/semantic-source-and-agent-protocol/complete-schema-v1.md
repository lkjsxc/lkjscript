# Semantic Source: Complete Schema V1

[Authority](../semantic-source-and-agent-protocol.md)

## Purpose

Close the complete Edition 1 source schema and define honest correlation to
compiler and execution facts before Schema 2 extends it for Edition 2.

## Status

**Current.** `lkjscript.semantic-source/1` is the closed Edition 1 schema used
by the bounded one-shot protocol. Agent Foundation V1 is historical.

## Closed Source Schema

Complete Schema V1 represents every Current Edition 1 concept with closed typed
nodes and no generic fallback object:

- source unit and edition identity, import, main, function, signature,
  parameters, type variables, bounds, and exact type forms;
- product, product field, marker trait, and implementation;
- Unit, Bool, I64, F64, string, empty-list, Option, and Result literals and
  constructors;
- name reference, let/bind, var/set, if, while, do, and quote;
- product construction, field access, and immutable replacement;
- list, Option, Result, and every Current built-in call form;
- ownership move, borrow, and borrow-mut;
- comment, blank line, source origin, exact span, and trivia attachment; and
- development typed holes once their separate contract is implemented.

Kinds are closed enums. The schema preserves one canonical tree and
deterministic Edition 1 projection. Its complete kind/field table is tested
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

`lkjscript.semantic-source/1` is emitted after complete vocabulary, schema/source
roundtrip, malformed-boundary, correlation, diagnostic, and transaction gates.
The older `lkjscript.agent-foundation/1` is historical evidence, not a competing
Current source schema. `lkjscript.agent/1` is emitted only when its selected
complete operation contract passes; identities are never relabeled.

Schema 2 reuses this closure discipline and has identity
`lkjscript.semantic-source/2`; it does not mutate or overload Schema V1.
