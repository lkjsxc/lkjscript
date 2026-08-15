# Local daemon and machine protocol specification

## Endpoint and binary framing

`lkjscriptd` requires an explicit absolute state path and listens only on
`STATE_DIRECTORY/lkjscript.sock`. State-directory mode 0700, socket mode 0600, an exclusive daemon
lock, and OS filesystem ownership form the bootstrap local access boundary. There is no HTTP, TCP,
or public JSON listener.

Each connection carries exactly one request and one response and then closes. Protocol version 4
directly replaces prior versions; version 3 and older reject and no legacy reader remains. A binary frame is:

```text
u32 little-endian body length | u16 version=4 | nonzero u64 request ID | closed typed message
```

The response repeats the request ID. Frames are limited to 8 MiB and encoded collections to 100,000
items as transport policy. Checked decoders reject truncated or oversized frames, zero request IDs,
invalid lengths/counts/booleans/UTF-8/IDs/indexes, unknown stable tags, payload trailing data, and
any bytes after the one request or response frame. Artifact and HEAD bytes use separate decoders.

## Request and response families

The complete public request families are:

- `CreateWorkspace`;
- `ApplyTransaction`, carrying the typed transaction, commit/validate-only mode, optional commit
  idempotency key, and bounded response projection;
- `QueryBatch`, binding independent read items to one workspace and exact retained revision;
- `Run`, naming workspace, exact revision, entry function Node ID, ordered exact primitive/product/sum invocation values, and a closed positive bounded policy containing `fuel` and `maximum_frames`;
- `DescribeSchema`, carrying exactly one `manifest`, non-empty unique bounded `sections`, or `full`
  projection plus an optional canonical machine-schema digest;
- `Shutdown`.

Responses are `WorkspaceCreated`, compact `TransactionReceipt`, `QueryBatchResult`, typed `Run`,
`DescribeSchema`, `Acknowledged`, or structured `Error`. The schema result is exactly `unchanged`,
`manifest`, `sections`, or `full`. Errors include a stable code and typed
optional workspace, revision, public transaction operation index, offending explicit local handle,
target, expected/actual kind or type, at most 64 deterministically ordered related IDs, and
retryability; prose is presentation only. Exact larger blocker detail remains available through the
paginated blockers query. Run argument mismatch, execution-fuel exhaustion, and execution-frame exhaustion have distinct stable
error codes; arithmetic overflow remains a runtime trap.

Initial public construction uses atomic `CreateProductType` and `CreateSumType`, structured
`CreateFunction`, optional structured bodies, `DefineFunctionBody`, and base-block-only
`InsertExpression`. Declaration members carry explicit handles. `TypeDraft` is a closed primitive or
nominal `NodeTarget` and resolves existing or transaction-local declarations before canonical
validation. Parameter, expression, loop-index, loop-carried, and match-payload handles are explicit. Product
values key bindings by field identity and match arms key bodies by variant identity. Implied match
regions, blocks, payload arguments, and yield terminators, as well as the existing structured
scaffolding, are expanded once into canonical nodes and the draft is discarded. The displaced low-level
parameter/region/block/operation creation messages are not public variants. Expansion exhaustively scans every `NodeTarget`, `TypeDraft`, and `ValueDraft`, rejects undeclared or wrong-category local references before allocation, and allocates every explicit and implicit identity before applying canonical edits. A transaction-local, non-persisted nominal catalogue permits product bindings, variants, payload scaffolding, and match arms to reference declarations or members authored later in the same transaction. Match arms normalize to declaration order before implied identities are assigned, so equivalent arm permutations allocate the same canonical graph. Forward and mutual function references continue to resolve deterministically. Structured requests allow at most 16 nested
structured bodies and 65,536 exactly counted draft items; the conservative depth remains below the
strict JSON parser recursion boundary and binary decoding enforces the same value. The item total is the checked aggregate of top-level transaction operations, function parameters, product fields, sum variants, function bodies, yielding bodies, expressions, call arguments, product bindings, and match arms. Deeper semantic graphs remain constructible by inserting into blocks retained by prior transactions.

A transaction receipt never contains the full semantic diff or every allocation by default. It
contains bounded identity/publication/completeness facts, exact change count/digest, total explicit
and implicit created-node count, and only requested explicit-handle bindings. The response
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

`DescribeSchema` has one active payload contract. Its projection is `manifest`, `sections { sections
}`, or `full`; `known_digest` is absent, explicit `null`, or exactly 32 bytes represented in JSON as 64 lowercase hex
characters. The old tag-only request is truncated protocol input and rejects. Section lists are
non-empty, contain no duplicates, contain at most seven entries, reject unknown tags/names, and are
returned in canonical section order regardless of request order. The closed sections are
`identity_and_envelopes`, `semantic_types_and_nodes`, `nominal_declarations`,
`transactions_and_expressions`, `queries_and_repair`, `runtime_and_run`, and `errors_and_limits`.

The compact manifest identifies `lkjscript-machine-schema-v4`, its digest, protocol/JSON version 4,
artifact format 3, semantic schema `lkjscript-spg003`, the closed section catalogue and tags, the
seven-section policy, full availability, and frame/JSON output bounds needed to request a projection.
It does not contain the full payload catalogues. A matching known digest returns only typed
`unchanged { digest }` regardless of the requested projection. A mismatch is not an error and returns
the requested active projection. Section and full results bind the active digest; full output is
available only explicitly. Projection is recomputed without daemon caching or persistence.

The digest is BLAKE3 domain-separated and covers the canonical protocol schema-facts encoding of the
complete executable `SchemaDescription`, excluding projection digest output itself. It binds machine,
protocol, JSON, artifact and semantic-schema identities; stable vocabularies/tags; payload and field
shapes; semantic types and nodes; operation and dynamic-region contracts; nominal invariants;
transactions and structured expressions; query and repair DTOs; Run/runtime invariants; errors; ID
formats; and limits. Set-like catalogues are sorted for hashing while semantic field/member order is
preserved. The seven exact typed sections partition the complete description, and their canonical
union reconstructs full output.

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
stable draft field tag and name exactly once to its machine type expression.

The executable full description exposes stable names/tags, exact adjacent/external/string-enum tagging conventions,
the complete named-NodeKind set, the 1-byte minimum and 1 MiB artifact/decode maximum for names,
and all seven exact sibling-name uniqueness groups. `semantic_types_and_nodes` carries that name
contract; `errors_and_limits` carries the exact 64 MiB complete-artifact and 1 MiB artifact-name
limits. Both are also present in full output and covered by its digest. It also exposes
operation contracts including the closed `match_variants` dynamic-region rule, exact structured
expression/maintenance-operation/value variants, query input/result/outcome/nominal-member/cursor
DTOs, depth-first allocation and implicit-node facts, transaction/query/error/request/response
vocabularies, unit/newtype/record payload shapes, exact required and optional field names with stable
type expressions, request/response/boundary envelopes, transaction and receipt records, required Run
and RunPolicy fields, RunResult, runtime field-value records, and all five runtime value tags. Product payloads are
`{ty: node, fields: [{field: node, value: runtime_value}]}`; sum payloads are
`{ty: node, variant: node, payload?: runtime_value}`. Product input names every exact owned field
identity once in arbitrary order and normalizes to declaration order; output uses declaration order.
Sum input names one exact owned variant and has payload exactly when required at its exact type.
Only semantic Node IDs cross the boundary; Core IDs, discriminants, and layout offsets never do.
The generated runtime schema states these closed invariants and the scope of each policy.

Runtime limits are 1,024 ordered arguments, 10,000,000 fuel units, 100,000 frames, 65,536 peak
materialized cells, runtime-value depth 24, 4,096 runtime-value items aggregated across all Run
arguments, and 64 KiB of encoded runtime-value bytes aggregated across all Run arguments and used for
mandatory-result preflight. Binary and strict JSON Run envelopes enforce the same depth, item, byte,
and argument-count constants. Protocol writers stop before vector growth beyond the 8 MiB frame
ceiling. Binary runtime decoding checks counts before allocation and depth/items/bytes while decoding.
Artifact load and commit preflight reject artifacts over 67,108,864 bytes and names over 1,048,576
UTF-8 bytes with `policy_exceeded`; semantic validation rejects empty names or duplicates within an
advertised sibling category. A policy or semantic rejection after a valid nonzero request ID returns
a typed error under that exact request ID and publishes nothing.
Malformed tags, truncation, trailing bytes, foreign members, duplicate fields, and
payload mismatches reject. This specification does not duplicate the remaining exhaustive payload
catalogue.

## Strict generic JSON CLI

`lkjscript --state DIRECTORY rpc [--pretty]` reads exactly one strict JSON version-4 request envelope
from stdin, projects it to the same typed binary request, invokes the private daemon, and writes
exactly one JSON response envelope to stdout. `lkjscript schema` emits the compact manifest locally;
repeatable `--section NAME` requests exact sections, `--full` requests the complete description,
`--known-digest HEX` enables `unchanged`, and `--pretty` only changes formatting. `--full` and
`--section` are mutually exclusive; missing, mixed, duplicate, unknown, malformed, or excessive flags
produce a bounded usage error. Local output and daemon `DescribeSchema` use the same projection
functions. JSON is transport only and is never persisted as program authority.

The envelope fields are `version`, nonzero `request_id`, and typed `request`. Tagged variants use
stable lowercase snake-case names. Workspace and idempotency IDs are exactly 32 lowercase hex
characters; Node IDs are `workspace:nonzero-canonical-decimal-serial`; hashes/digests are
fixed-width lowercase hex; revisions, query IDs, handles, indexes, and counts are JSON integers in
their checked domains. There is one canonical representation.

Unknown fields and variants, duplicate fields, wrong case, malformed or uppercase hex, zero node
serials, invalid numeric domains, invalid UTF-8, excessive nesting, trailing JSON values, and input
over 8 MiB reject locally. JSON output is streamed through a 32 MiB limit rather than first
allocating an unbounded value. Boundary error messages are bounded; schema flag, digest, section,
and projection failures use the same compact boundary-error envelope. Semantic and policy validation
that belongs to the daemon is returned as typed `Response::Error`, not reclassified as JSON syntax
failure.

Machine stdout contains exactly one compact JSON value plus newline (pretty output is explicit).
Diagnostics do not contaminate stdout and belong on stderr. Exit status is:

- `0`: a syntactically valid daemon response, including typed semantic rejection;
- `2`: CLI usage, stdin, or JSON boundary error;
- `3`: daemon transport failure;
- `4`: response conversion, output-limit, serialization, or write failure.

## Connection behavior

A clean close before a frame is ignored. After writing its request, the production client shuts down
the connection write half. The daemon proves request-side EOF before dispatch, so a second frame or
any connection-level trailing byte cannot reach mutation. The client likewise proves response-side
EOF before accepting the reply. The server enforces one absolute five-second connection deadline;
the client uses a 30-second I/O timeout. Every well-formed request
echoes its nonzero request ID. A malformed binary request receives a structured error with
correlation ID zero. A dropped response does not stop the daemon. `Shutdown` exits only after
acknowledgement is written. The client owns no mutable graph, artifact writer, compiler, or
interpreter.
