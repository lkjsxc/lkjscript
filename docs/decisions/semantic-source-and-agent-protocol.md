# Semantic Source And Agent Protocol

## Purpose

Define the primary manipulation boundary for complete and incomplete programs,
the deterministic Edition 1 projection adapter, and the smallest complete
agent-facing transaction/query slice.

## Status

**Accepted Target.** No Semantic Source API, structured edit protocol, or typed
hole is Current until its implementation and gates land. The Current parser and
compiler behavior remain authoritative while this slice is built.

The first implementation may cover only the existing Edition 1 declaration and
expression vocabulary, but every supported operation must be complete and
non-placeholder. Unsupported schema nodes or edit operations fail explicitly.

## Problem

The Current compiler resolves a physical token/form tree directly into typed
HIR. Agents retrieve arbitrary file fragments and edit by string identity.
Formatting drift, repeated forms, stale revisions, and broad diagnostics can
therefore turn a semantically simple edit into an ambiguous text mutation. A
new language edition built on that interface would magnify context cost and
migration risk.

The source schema must not become a second trusted type system or a sibling
backend frontend. It must preserve exact source origin while keeping compiler-
derived type, ownership, effect, layout, and proof facts authoritative.

## Versioned Identities

The first registered identities are:

- Semantic Source schema: `lkjscript.semantic-source`, version `1`;
- agent protocol: `lkjscript.agent`, version `1`;
- structured diagnostic schema: `lkjscript.diagnostic`, version `1`;
- semantic edit schema: `lkjscript.edit`, version `1`.

Every serialized envelope carries a schema name and exact integer version.
Unknown schema names, versions, variants, and fields fail with structured
protocol diagnostics; they are not ignored. Canonical serialization uses
UTF-8, deterministic field order, deterministic list order, and no
floating-point numbers for identities or counters.

## Source Authority Boundary

A parser adapter accepts canonical Edition 1 `.lkjscript` and constructs a
private mutable builder. Validation checks marker matching, source limits,
declaration shape, stable-key uniqueness, source spans, and node-tree
well-formedness. Only successful validation yields opaque immutable
`ValidatedSourceTree` authority.

Consumers cannot construct a validated tree by deserializing an arbitrary
public struct. HIR analysis accepts the validated boundary or a mechanically
checked projection from it; no backend reads source spelling or serialized
claims.

The first cutover must replace the old parser/form authority rather than retain
two independently interpreted source trees. A temporary mechanically checked
adapter may feed unchanged analysis during the cutover. It is removed once HIR
consumes the validated source nodes directly.

## Schema V1

Schema V1 represents the complete Current Edition 1 source vocabulary:

- source unit, edition, schema version, canonical relative origin, and imports;
- top-level main, function, product, trait, and implementation declarations;
- declaration visibility as an explicit closed value, even where Edition 1
  permits only its default;
- names, type forms, parameters, generic bounds, product fields, trait markers,
  and implementation targets;
- literals, names, bindings, calls, operations, conditionals, loops, local
  mutation, product operations, ownership operations, and every other Current
  expression form;
- comments/documentation if and when the Current projection exposes them; until
  then their absence is explicit rather than reconstructed heuristically;
- exact byte/line/column source spans and migration/source origin; and
- expression holes in the supported development positions.

Closed enums represent node kinds. Generic untyped JSON objects are not
semantic nodes. Source nodes never serialize inferred types, resolved binding
IDs, effects, ownership results, layouts, or optimizer facts as authority.
Queries may attach those derived facts in separately versioned response fields.

## Two Identity Layers

### Stable declaration keys

A stable declaration key is derived deterministically from:

```text
schema version
+ edition
+ future package identity (an explicit Edition 1 root identity initially)
+ canonical relative source-unit identity
+ declaration kind
+ declared name or reserved main identity
```

Keys do not depend on byte offsets, declaration order, formatting, or dense
compiler IDs. Rename and move operations report old and new keys plus the
semantic relationship. Duplicate keys are rejected rather than disambiguated
by source order. Package and module identities replace the Edition 1 root
component when those contracts become Current.

### Revision-scoped node IDs

Every validated revision assigns dense preorder `NodeId` values. A node ID is
valid only with its exact repository/tree revision. It is compact compiler and
protocol data, not a cross-revision semantic identity. Transactions that refer
to a node ID from another revision fail as stale.

## Revisions And Preconditions

A snapshot carries:

- a server-issued monotonic revision counter within a daemon session;
- a deterministic whole-tree fingerprint over the canonical schema encoding;
- the base repository identity when available; and
- deterministic per-entity and per-node precondition fingerprints.

A transaction names the exact base revision and expected fingerprint for every
read/modified entity. Fingerprints are a stale-edit check, not authorization.
Commit also compares the exact canonical precondition value, so a hash
collision cannot authorize a different edit. A one-shot CLI snapshot derives
its initial identity from the complete loaded source closure rather than wall
clock, process ID, or filesystem enumeration order.

## Atomic Semantic Edits

Protocol V1 supports these complete operations for schema nodes that exist in
the first implementation:

- insert a top-level declaration at an explicit semantic position;
- replace a top-level declaration;
- delete a top-level declaration;
- rename a declaration and all resolved references in the loaded closure;
- replace an expression subtree;
- insert or delete an expression child only where the parent schema defines an
  ordered child collection; and
- fill or refine a typed expression hole.

Each transaction follows:

1. validate envelope/version, revision, limits, and all preconditions;
2. clone or persistently stage the complete affected semantic state;
3. resolve every target before applying any operation;
4. apply operations in declared order to the staged state;
5. rebuild dense node IDs and stable keys deterministically;
6. run structural validation and optional name/type/effect/ownership checks;
7. compute the semantic diff and diagnostics; and
8. atomically publish one new revision only on success.

Any failure discards all staged changes. Operations never search for an exact
text substring and never partially write source files. File publication uses a
same-directory temporary plus atomic replacement where the host guarantees it;
otherwise the protocol reports unsupported publication before changing files.

## Protocol V1

The initial CLI command is conceptually:

```text
lkjscript semantic <request.json>
```

A daemon may later expose the same request/response envelopes over a local
transport. Transport does not change semantic behavior.

Required requests in the first complete slice are:

- `snapshot`: source units, declarations, stable keys, revision, fingerprints,
  and compact summaries;
- `read_entity`: one complete declaration and descendants;
- `query_node`: source node plus available derived type/effect/ownership facts;
- `apply_transaction`: atomic operations and validation policy;
- `diagnostics`: structured diagnostics for the loaded revision; and
- `hole_context`: expected type, visible bindings, allowed effects, source
  origin, and bounded candidates for one hole.

Responses are deterministic for the same source closure, compiler build,
request, and profile. Lists define their sort keys. Requests and responses have
aggregate byte/node/work limits. Normal program stdout is never mixed with
protocol output.

## Structured Diagnostic V1

Every diagnostic contains:

- stable code, diagnostic schema version, severity, and category;
- primary semantic node and exact source span;
- related nodes/spans;
- declaration and binding identities when available;
- expected and actual types where applicable;
- effect/capability mismatch and ownership/move/loan paths where applicable;
- relevant control-flow path where available;
- resource-budget category where applicable;
- concise human and compact agent renderings;
- zero or more machine-applicable semantic edits; and
- explicit certainty: `guaranteed`, `conditional`, or `informational`.

The first converted codes are:

| Code | Meaning | Safe repair policy |
| --- | --- | --- |
| `LKJ-SRC-UNMATCHED-MARKER` | opening/closing marker mismatch | only offer an edit when the unique intended marker is structurally proven |
| `LKJ-DECL-DUPLICATE` | duplicate declaration key/name | no automatic rename without a requested name |
| `LKJ-NAME-UNKNOWN` | unresolved binding/declaration | offer only uniquely proven visible-name replacement |
| `LKJ-CALL-ARITY` | exact call arity mismatch | report missing/excess positions; do not invent effectful arguments |
| `LKJ-TYPE-MISMATCH` | expected and actual types differ | offer only semantics-preserving coercion/construction known to be exact |
| `LKJ-EDIT-STALE` | revision or precondition mismatch | refresh/rebase action; never apply the stale mutation |

Human text is a projection of this record. Existing message wording may remain
compatible during migration, but string parsing is never the agent API.

## Typed Expression Holes

A typed expression hole is a development semantic node, not an `Any`, `nil`, or
unchecked runtime value. V1 permits holes in let initializers, function/main
body positions, call arguments, and conditional branch positions.

A hole records:

- stable hole identity within its declaration plus revision-local node ID;
- source goal and origin;
- expected type when derivable, otherwise an explicit bounded unknown;
- visible bindings and their exact types;
- generic/trait obligations;
- allowed and already-required effects;
- available capabilities when that analysis exists;
- ownership/place/region constraints when that analysis exists;
- control-flow requirements and whether divergence is permitted; and
- material obligations preventing release acceptance.

Development analysis treats a hole as satisfying only its recorded expected
position and propagates an explicit incomplete fact; it does not prove arbitrary
traits, effects, ownership, or values. Independent surrounding declarations
continue to typecheck. Executable, release, AOT, cache, package, and component
artifacts reject every unresolved hole with a structured diagnostic.

`hole_context` returns bounded, deterministically sorted candidates from exact
literals, visible local bindings, available constructors, and directly callable
functions whose result and effect contract fit. Candidate enumeration is
resource bounded and explicitly reports truncation. It does not claim complete
inhabitation for the full language.

## Deterministic Edition 1 Projection

The formatter is total over every validated V1 tree representable in Edition 1.
It emits one canonical UTF-8 byte sequence with LF endings. Required laws are:

```text
parse(format(tree)) == tree, excluding revision-local NodeIds
format(parse(canonical_corpus_file)) == canonical_corpus_file
format(parse(format(tree))) == format(tree)
```

The exact 109-file canonical `src` corpus, all 113 tracked `.lkjscript`
sources/fixtures/workloads, and benchmark identities recorded in Current State
are immutable migration evidence. If existing canonical files reveal more
than one accepted spelling, the decision records whether the formatter
preserves one spelling field or mechanically migrates the corpus; it does not
silently normalize benchmark input.

Comments require explicit stable attachment rules before the formatter accepts
them. A parser that discards source bytes it cannot reproduce does not satisfy
the roundtrip gate.

## Constrained Generation Boundary

Legal-next-action or token-mask generation is **Deferred** until Schema V1,
typed holes, and query contexts are Current. A future constrainer states the
exact supported subset and soundness/completeness evidence. Unsupported states
fail open to ordinary generation followed by compiler validation, or return an
explicit unsupported result. They never silently exclude valid full-language
programs while claiming complete coverage.

## Dependency Selection For The Protocol

A general JSON parser will not be hand-written merely to preserve a
zero-dependency count. The retained isolated experiment at baseline `f6410a22`
evaluated `serde 1.0.229` and `serde_json 1.0.151` with exact version tags,
unknown-field rejection, malformed versions, Unicode, and roundtrip tests.

The derive candidate grew the lock from 9 to 20 packages: `serde`,
`serde_core`, `serde_json`, `itoa`, `memchr`, `zmij`, `serde_derive`,
`proc-macro2`, `quote`, `syn`, and `unicode-ident`. Licenses reported by Cargo
were MIT OR Apache-2.0, Unlicense OR MIT for `memchr`, MIT for `zmij`, and
(MIT OR Apache-2.0) AND Unicode-3.0 for `unicode-ident`; bundled notices were
present. These are compiler/tooling TCB dependencies; the derive chain is
build/proc-macro TCB. They are not linked into the guest VM/runtime solely by
this boundary and are not lkjscript language-package dependencies.

On the same Linux/Rust 1.96 host, three fresh compiler-check trials measured a
baseline median of 0.937567 s and derive median of 2.425041 s (+1.487474 s,
2.587x). No-op medians were 0.010903 and 0.015098 s. Fresh target data grew from
25,782,273 bytes/37 files to 105,088,318 bytes/187 files; compiler metadata grew
55,766 bytes. A no-derive manual visitor reduced clean median to 1.450214 s and
target data to 67,265,361 bytes, but one strict request alone required 145
production lines and the lock still retained all eleven packages. Direct
`serde_core` compiled one fewer active package but retained the same lock and
license surface and showed no check-time improvement.

**Accepted dependency selection, not yet added:** use derive-enabled `serde` and
`serde_json` only at the bounded versioned protocol/serialization boundary when
that boundary is implemented. The protocol has enough request/response types
that the measured proc-macro cost is preferable to many hand-written visitors.
A `serde_json::Value` object boundary is rejected because ordinary map parsing
does not provide the closed typed schema or desired duplicate-field behavior.
A hand-written general JSON parser is rejected due disproportionate Unicode,
escape, duplicate, trailing-input, and evolution risk.

Permanent addition still requires locked-version/advisory and bundled-license
checks, malformed/duplicate/Unicode/trailing-data tests, pre-parse request byte
limits, post-parse node/depth/work limits, fuzzing targets, and notice retention.
No vulnerability audit or legal approval was performed by the experiment; those
are explicitly untested boundaries. Until the dependency lands with those
checks, implementation may build the in-memory validated schema and transaction
engine, but it may not expose an unbounded or incompletely parsed JSON mutation
endpoint and call Protocol V1 complete.

## Acceptance Gates

The first slice is Current only when:

- every exact baseline corpus file parses and formats byte-identically;
- parse-format-parse and format-format structural laws pass;
- malformed source/schema/protocol inputs fail under aggregate bounds;
- stable declaration keys ignore formatting and order but detect duplicates;
- node IDs are rejected across revisions;
- stale and mismatched-precondition edits deterministically fail;
- every failed multi-operation transaction leaves memory and files unchanged;
- insert/replace/delete/rename and expression replacement have positive,
  malformed, adversarial, and resource-boundary tests;
- the six diagnostic codes have schema golden tests and human projections;
- supported holes expose exact contexts and release compilation rejects them;
- unchanged Edition 1 compile/evaluator/VM/JIT semantics pass the canonical and
  runtime gates; and
- an initial retained harness compares raw text, entity edits, and hole filling
  without making a general superiority claim.

## Not Current And Deferred

Schema V1 does not itself provide Edition 2, modules/packages, semantic merge,
multi-agent task ownership, general proof holes, complete type inhabitation,
constrained decoding, a network daemon, or a selected replacement text syntax.
Those remain later measured slices.
