# Semantic Source And Agent Protocol: Protocol V1

[Authority](../semantic-source-and-agent-protocol.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

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
