# Local daemon and machine protocol specification

## Endpoint and framed JSON

`lkjscriptd` requires an explicit absolute state path and listens only on
`STATE_DIRECTORY/lkjscript.sock`. State-directory mode 0700, socket mode 0600, an exclusive daemon
lock, and OS filesystem ownership form the bootstrap local access boundary. There is no HTTP, TCP,
or public JSON listener.

Each connection carries exactly one request and one response and then closes. Protocol version 7
directly replaces prior versions; protocol-v6 success readers and writers are deleted and no
legacy reader remains. A control frame is:

```text
u32 little-endian body length | strict compact JSON version-7 envelope
```

Request bodies are limited to 8 MiB and response bodies to 32 MiB. The response repeats the nonzero
request ID. Length checks happen before allocation. Strict decoders reject truncated or oversized
frames, zero request IDs, invalid numeric/UTF-8/ID domains, unknown or duplicate fields and variants,
trailing JSON values, and any bytes after the one request or response frame. A response body is
exactly either a response envelope or a boundary-error envelope; an optional boundary-error
correlation ID must match the request when present. Artifact and HEAD bytes use separate decoders.

## Request and response families

The complete public request families are:

- `CreateWorkspace`;
- `ApplyTransaction`, carrying the typed transaction, commit/validate-only mode, optional commit
  idempotency key, and bounded response projection;
- `QueryBatch`, binding independent read items to one workspace and exact retained revision;
- `Run`, naming workspace, exact revision, entry function Node ID, ordered exact
  primitive/bytes/product/sum invocation values, and a closed positive bounded policy containing
  `fuel` and `maximum_frames`;
- `DescribeSchema`, carrying exactly one `manifest`, non-empty unique bounded `roots`, or `full`
  projection plus an optional canonical machine-schema digest;
- `Shutdown`.

Responses are `WorkspaceCreated`, compact `TransactionReceipt`, `QueryBatchResult`, typed `Run`,
`DescribeSchema`, `Acknowledged`, or structured `Error`. The schema result is exactly `unchanged`,
`manifest`, `roots`, or `full`. Errors include a stable code and typed
optional workspace, revision, public transaction operation index, offending exact `draft_symbol`,
bounded deterministic `draft_path` for private implied nodes, target, expected/actual kind or type,
at most 64 deterministically ordered related IDs, and
retryability; prose is presentation only. Exact larger blocker detail remains available through the
paginated blockers query. Run argument mismatch, execution-fuel exhaustion, and execution-frame exhaustion have distinct stable
error codes; byte encoding/literal/input/result limits, managed object/visible/retained policies,
byte index/slice bounds, and invalid internal handles also remain distinct. Arithmetic overflow
remains a runtime trap.

Initial public construction uses atomic `CreateProductType` and `CreateSumType`, structured
`CreateFunction` with an optional body, and base-block-only `InsertExpression`. Maintenance may use
`DefineFunctionBody` only with an existing function Node ID. Declaration members carry explicit draft symbols matching
`[a-z][a-z0-9_]*` in 1..=64 bytes. `NodeTarget` is exactly `existing` or `draft`; numeric draft targets
and the old `local` form reject. `TypeDraft` is a closed primitive (`unit`, `bool`, `i64`, or
`bytes`) or nominal target and resolves
existing or transaction-local declarations before canonical validation. Parameter, shared or selected bound-expression,
loop-index, loop-carried, and match-payload fields are symbols. Expression bindings are optional;
an omitted binding is private and unselectable. A `ValueDraft` is exactly a parameter reference,
block-argument reference, operation-result reference, or an `inline_expression` containing one
`ExpressionKindDraft`. Inline values accept only descriptor-complete, non-terminating,
single-result, regionless operations: `const_unit`, `const_bool`, `const_i64`, `const_bytes`,
`add_i64`, `lt_i64`, `call`, `construct_product`, `project_field`, `construct_variant`,
`bytes_len`, `bytes_at`, `bytes_slice`, and `bytes_equal`. Holes and region-owning
`if`, `for_i64`, and `match_sum` remain explicit, as do shared, selected, repairable, and maintenance
targets. Fine-grained maintenance requests reject inline values.

Inline children normalize iteratively, depth-first and left-to-right before their parent. Product
fields and match arms first normalize to declaration order. Anonymous inline nodes receive ordinary
persistent operation IDs but no selectable draft symbol; proposal nesting and diagnostic paths are
discarded after validation. Product
values key bindings by field identity and match arms key bodies by variant identity. Implied match
regions, blocks, payload arguments, and yield terminators, as well as the existing structured
scaffolding, are expanded once into canonical nodes and the draft is discarded. The displaced low-level
parameter/region/block/operation creation messages are not public variants. Expansion exhaustively
scans every `NodeTarget`, `TypeDraft`, and `ValueDraft`, rejects undeclared or wrong-category local
references before allocation, and allocates every explicit and implicit identity before applying
canonical edits. A transaction-local, non-persisted nominal catalogue permits product bindings,
variants, payload scaffolding, and match arms to reference declarations or members authored later in
the same transaction. Match arms and product fields normalize to declaration order before implied
identities are assigned. Forward and mutual function references continue to resolve deterministically.
Structured requests allow at most 16 combined inline-expression or operation-owned-body edges on one
proposal path and 65,536 exactly counted draft items; the conservative depth remains below the strict
JSON parser recursion boundary. The item total is the checked aggregate of top-level transaction
operations, function parameters, product fields, sum variants, function bodies, yielding bodies,
explicit or inline expressions, call arguments, product bindings, and match arms. Wrapper variants
are not double-counted. Deeper semantic graphs remain constructible by inserting into blocks retained
by prior transactions.

`const_bytes` uses the canonical `bytes_string` scalar. One decoded literal is limited to 4,096
octets and the checked aggregate of all explicit and inline literals in a transaction is 65,536
octets. This is independent of structured item count and the 8 MiB JSON envelope limit; violation
rejects before persistent identity allocation.

A transaction receipt never contains the full semantic diff or every allocation by default. It
contains bounded identity/publication/completeness facts, exact change count/digest, total explicit
and implicit created-node count, and only requested explicit symbolic bindings. The response
projection is part of the idempotency fingerprint. Validate-only returns an identical predicted
allocation/hash with `published=false` and does not publish.

A query batch contains at most 32 client-labelled items and an aggregate requested-item budget of
2048. Every item observes the same immutable revision and results remain in request order with the
same query IDs. A bad workspace, revision, duplicate query ID, batch shape, page limit, or aggregate
budget rejects the batch. Once the batch boundary is valid, a target-specific error affects only
that item and other items can succeed.

The closed query families are workspace summary, compact or exact node view, completeness blockers,
owner chain, block body slice, incoming value uses, incoming definition references, outgoing
dependencies, visible values, legal constructors, semantic diff, repair context, and exact nominal
type context. Nominal context pages declaration and member display names with exact identity/type facts. Derived layout and member offset, size, alignment, cell, and discriminant facts are optional and absent rather than fabricated when layout is unrepresentable; its cursor binds workspace, revision, and declaration. Pages contain
typed items, `next` when more exists, and total count when provided. Page size is 1..=256. Cursors
bind workspace, revision, query family/purpose, target and options as applicable, and deterministic
next position; cross-revision, cross-target, cross-purpose, malformed, or out-of-range cursors
reject. A diff query binds exact `from` and batch `to` revisions and repeats exact total
change-count/digest facts on every page. Repair context composes bounded structural facts using up to 64 items per category. Every constructor reports exact operand/member totals and whether its retained requirements are complete; requirement vectors retain at most 64 items. Fitting product declarations remain one-query repairable, while oversized products expose a typed nominal-type query continuation instead of materializing unbounded constructor payloads. Full scans and recomputation are the
correctness oracle.

`DescribeSchema` has one active payload contract. Its projection is `manifest`, `roots { roots }`,
or `full`; `known_digest` is absent, explicit `null`, or exactly 32 bytes represented in JSON as 64
lowercase hex characters. Root lists are non-empty, unique, contain at most 16 entries, and use a
closed lowercase vocabulary. Unknown and noncanonical roots reject during strict decoding before a
matching known digest can return `unchanged`.

The manifest advertises exact roots for all five control endpoints and every query endpoint as well
as broad request/response families, transaction and expression drafts, query families, runtime
values, errors, semantic nodes and operations, ID formats, limits, nominal facts, and schema
discovery. An endpoint definition binds either the shared control protocol template or the shared
query protocol template, exact selected leaf request/success variants and payloads, the shared
boundary-error envelope and typed error, protocol/JSON versions, ID-format definition, and limit
definition. The query template exactly defines the `query_batch` top request, batch request/item,
batch result/item outcome, success/error outcome, `query_batch_result` and top-level typed-error
response, and request/response envelope layers. Its explicitly declared contextual parameters bind
one inner query variant and matching result variant from the endpoint; they are not unresolved global
types. The templates and endpoint bindings are projected from executable broad record, variant,
envelope, error, and tagging descriptors rather than maintained as a second accepted-shape authority.

A roots result repeats the canonical sorted roots, documents `list<T>`, `optional<T>`,
`tuple<T,...>`, and `page<T>` constructors, and returns each named definition in lexical order exactly
once. An iterative worklist follows every named payload and draft-field type expression transitively.
`page<T>` includes both `page` and `T`; `type_parameter` is bound by the page definition. Every
dependency resolves within the result.

The compact manifest identifies `lkjscript-machine-schema-v7`, its digest, protocol/JSON version 7,
artifact format 4, semantic schema `lkjscript-spg004`, the closed root vocabulary, the 16-root request
policy, the type constructors, full availability, and frame/JSON output bounds. It does not contain
the full payload catalogues. A matching known digest returns only typed `unchanged { digest }` after
root validation. A mismatch is not an error and returns the requested active projection. Root and
full results bind the active digest; full output is available only explicitly. Projection is
recomputed without daemon caching or persistence.

The digest is BLAKE3 domain-separated and covers the canonical protocol schema-facts encoding of the
complete executable `SchemaDescription`, excluding projection digest output itself. It binds machine,
protocol, JSON, artifact and semantic-schema identities; stable lowercase vocabularies; payload and field
shapes; semantic types and nodes; operation and dynamic-region contracts; nominal invariants;
transactions and structured expressions; query and repair DTOs; Run/runtime invariants; errors; ID
formats; and limits. Set-like catalogues are sorted for hashing while semantic field/member order is preserved. Root
projection does not change or partition the complete description; it derives closed definitions from
that same complete executable contract.

The executable descriptors are the sole machine-schema authority; there is no second hand-maintained
schema file. Strict serde-agreement tests guard that the JSON transport implementation accepts and
rejects exactly the described record and variant shapes. The full description includes one closed
code-owned scalar catalogue stating each scalar or ID's JSON kind and exact signed or unsigned range,
UTF-8 domain, lowercase hexadecimal width, or canonical workspace-qualified Node ID format. Every
non-primitive named type expression in the executable full description resolves exactly once to a
described record, variant, scalar, or generic list/page/tuple/optional contract. `optional<T>` means
the field may be omitted or supplied as
explicit JSON `null`; all other field type expressions are required and non-null. Structured-draft
fields state `required` and `nullable` separately, and the closed draft-field-type catalogue maps every
draft field name exactly once to its machine type expression.

The executable full description exposes stable lowercase names and exact adjacent/external/string-enum
tagging conventions without obsolete numeric control-plane tags. It includes the complete named-NodeKind
set, the 1-byte minimum and 1 MiB artifact/decode maximum for names, and all seven exact sibling-name
uniqueness groups. Name, limit, ID-format, and nominal policy facts are first-class root definitions
and remain in full output and its digest. It also exposes
operation contracts including the closed `match_variants` dynamic-region rule, exact structured
expression/maintenance-operation/value variants, query input/result/outcome/nominal-member/cursor
DTOs, depth-first allocation and implicit-node facts, transaction/query/error/request/response
vocabularies, unit/newtype/record payload shapes, exact required and optional field names with stable
type expressions, request/response/boundary envelopes, transaction and receipt records, required Run
and RunPolicy fields, RunResult, runtime field-value records, and all six runtime value forms. Product payloads are
`{ty: node, fields: [{field: node, value: runtime_value}]}`; sum payloads are
`{ty: node, variant: node, payload?: runtime_value}`. Product input names every exact owned field
identity once in arbitrary order and normalizes to declaration order; output uses declaration order.
Sum input names one exact owned variant and has payload exactly when required at its exact type.
Only semantic Node IDs cross the boundary; Core IDs, discriminants, and layout offsets never do.
The generated runtime schema states these closed invariants and the scope of each policy.

A public byte scalar is one JSON string containing canonical unpadded URL-safe base64. The alphabet
is `A-Z`, `a-z`, `0-9`, `-`, and `_`; padding and whitespace reject, a length congruent to one modulo
four rejects, and unused trailing bits must be zero. Decode length is checked before allocation and
decode followed by re-encode must reproduce the input. Empty bytes are `""`. `ConstBytes` drafts and
`RuntimeValue::Bytes` use this same scalar; no hexadecimal, padded, standard-alphabet, or integer-list
alias is accepted. Output contains visible octets only, never arena backing, views, or handles.

Runtime limits are 1,024 ordered arguments, 10,000,000 fuel units, 100,000 frames, 65,536 peak
materialized cells, runtime-value depth 24, 4,096 runtime-value items aggregated across all Run
arguments, and 64 KiB of structural runtime-value accounting aggregated across all Run arguments.
Bytes add a 65,536-octet limit per public value and across all decoded byte arguments, 1,048,576
cumulative visible arena bytes, 262,144 distinct retained backing bytes, 4,096 combined backing and
view objects, and a 65,536-octet public result limit. The strict JSON Run envelope enforces the same depth, item, byte, and
argument-count constants through the CLI and daemon. Request writers stop before growth beyond the
8 MiB input ceiling, response writers stream through the independent 32 MiB output ceiling, and
framing checks declared lengths before allocating.
Artifact load and commit preflight reject artifacts over 67,108,864 bytes and names over 1,048,576
UTF-8 bytes with `policy_exceeded`; semantic validation rejects empty names or duplicates within an
advertised sibling category. A policy or semantic rejection after a valid nonzero request ID returns
a typed error under that exact request ID and publishes nothing.
Malformed variants, truncation, trailing bytes, foreign members, duplicate fields, and
payload mismatches reject. This specification does not duplicate the remaining exhaustive payload
catalogue.

## Strict generic JSON CLI

`lkjscript --state DIRECTORY rpc [--pretty]` reads exactly one strict JSON version-7 request envelope
from stdin, sends the same closed typed JSON request through the private framed Unix connection, and
writes exactly one JSON response envelope to stdout. `lkjscript schema` emits the compact manifest locally;
repeatable `--root NAME` requests exact transitive definition closures, `--full` requests the complete
description, `--known-digest HEX` enables `unchanged`, and `--pretty` only changes formatting.
`--full` and `--root` are mutually exclusive; missing, mixed, duplicate, unknown, malformed, or excessive flags
produce a bounded usage error. Local output and daemon `DescribeSchema` use the same projection
functions. JSON is transport only and is never persisted as program authority.

`lkjscript --state DIRECTORY session` is a same-vocabulary process-lifecycle projection. It reads one
version-7 request envelope per LF-delimited line, bounded to the same 8 MiB before decoding, opens one
ordinary single-request daemon connection for that line, writes one compact response followed by LF,
and flushes before reading the next request. A final nonempty line at EOF is processed; EOF after a
response exits cleanly. Blank, malformed, or oversized lines produce one boundary-error value and
the session continues after draining that line. A daemon transport failure is also line-local, so a
later request can reconnect; the failed request is never retried. In particular, no possibly
published unkeyed mutation is silently replayed. `Shutdown` is an ordinary request: after its
acknowledgement, later lines receive transport failure unless a daemon is started again. Stdin read
failure is fatal with status 2, and stdout write or flush failure is fatal with status 4. Clean EOF
returns status 0 even when earlier lines carried structured boundary or transport errors. Session
mode adds neither a request variant nor a multi-frame daemon connection.

The envelope fields are `version`, nonzero `request_id`, and typed `request`. Tagged variants use
stable lowercase snake-case names. Workspace and idempotency IDs are exactly 32 lowercase hex
characters; Node IDs are `workspace:nonzero-canonical-decimal-serial`; hashes/digests are
fixed-width lowercase hex; revisions, query IDs, indexes, and counts are JSON integers in their
checked domains. Draft symbols are canonical bounded strings. There is one canonical representation.

Unknown fields and variants, duplicate fields, wrong case, malformed or uppercase hex, zero node
serials, invalid numeric domains, invalid UTF-8, excessive nesting, trailing JSON values, and input
over 8 MiB reject locally. JSON output is streamed through a 32 MiB limit rather than first
allocating an unbounded value. Boundary error messages are capped at 1,024 UTF-8 bytes; schema flag, digest, root, and
projection failures use the same compact boundary-error envelope. Semantic and policy validation
that belongs to the daemon is returned as typed `Response::Error`, not reclassified as JSON syntax
failure.

Machine stdout contains exactly one compact JSON value plus newline (pretty output is explicit).
Diagnostics do not contaminate stdout and belong on stderr. Boundary messages refer to roots and
projections rather than deleted sections. Exit status is:

- `0`: a syntactically valid daemon response, including typed semantic rejection;
- `2`: CLI usage, stdin, or JSON boundary error;
- `3`: daemon transport failure;
- `4`: response conversion, output-limit, serialization, or write failure.

## Connection behavior

A clean close before a frame is ignored. After writing its request, the production client shuts down
the connection write half. The daemon proves request-side EOF before dispatch, so a second frame or
any connection-level trailing byte cannot reach mutation. The client likewise proves response-side
EOF before accepting the reply. The server enforces one absolute five-second connection deadline;
the client uses an absolute 30-second connection deadline. Every well-formed request echoes its
nonzero request ID. A malformed frame receives a structured boundary error without a fabricated
correlation ID; malformed JSON may carry a strictly recovered matching ID. A dropped response does
not stop the daemon, and an exact keyed retry returns the retained replay receipt. `Shutdown` exits only after
acknowledgement is written. The client owns no mutable graph, artifact writer, compiler, or
interpreter.
