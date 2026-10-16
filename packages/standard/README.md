# standard

This directory is the maintained typed meaning authority for the exact built-in standard package.
Its 13 modules define deterministic core operations and typed interfaces for HTTP values, JSON,
canonical typed application data, the ordered `DataStore`, configuration, secrets, clocks, secure
randomness, identifiers, password hashing, byte streams, deployment-bound outbound HTTP, named
object storage, durable queues, and structured interactive-session events and decisions.

The core module exports ordinary F64 arithmetic (`f64-add`, `f64-subtract`, `f64-multiply`,
`f64-divide`, `f64-negate`, `f64-abs`, `f64-sqrt`), comparisons (`f64-less`, `f64-less-equal`),
predicates (`f64-is-finite`, `f64-is-nan`), and explicit text/integer boundaries
(`f64-from-i64`, `f64-to-i64-result`, `f64-parse-result`, `f64-to-text`). Exceptional arithmetic
produces full binary64 values. Algorithms and finite-only policies belong in graph libraries.

An external declaration is not arbitrary FFI. Its dotted implementation name must match the
closed semantic-validator/runtime intrinsic inventory before publication or execution. Capability
interfaces own typed operations, failure behavior, idempotency, possible visibility, and limits;
deployment grants select concrete adapters later.

The pure `buffer-empty`, `buffer-push`, `buffer-get`, `buffer-length`, `buffer-freeze`
and `buffer-discard` operations own a concrete affine `ByteBuffer`, with consuming
updates and scoped synchronous read loans. They require no capability or deployment
grant. The [native request](requests/20260930-owned-buffer.lkjc) adds four graph tests;
the [normative scope](../../docs/spec/owned-byte-buffers.md) defines exact parameter
modes, transfer, cleanup and unsupported boundaries. Ordinary immutable Bytes remains
unrestricted; freeze adopts the owned vector and can retain spare capacity.

The pure graph-owned `sequence-empty<T: Owned>`, `sequence-length<T: Owned>`,
`sequence-push<T: Owned>`, `sequence-pop<T: Owned>` and `sequence-discard<T: Owned>`
operations compose runtime-sized affine `(owned-sequence T)` values. Length borrows;
push consumes its element then sequence; pop consumes its sequence and returns
`empty: Sequence<T>` or `item: {rest: Sequence<T>, value: T}`. Both outcomes retain
the sequence allocation for reuse. Discard consumes and joins ordinary cleanup.
Elements must be eligible Owned values, including in empty sequences.

Indexed access uses the lexical `borrow-owned-item` language expression, which binds
a temporary read view and retains ancestor custody. It is not an escaping reference.
The [literal native request](requests/20261004-owned-sequences.lkjc) adds these five
wrappers, private ordinary graph helpers and seven fixed tests for empty reuse,
repeated indexed reads, LIFO drainage, replacement, nesting and populated cleanup.
The [normative contract](../../docs/spec/owned-sequences.md) defines admission,
transfer, reservation, cancellation and unsupported boundaries.

The same affine container now holds ordinary first-order data. Seven pure wrappers
use `T: Transferable`: `data-sequence-empty`, `data-sequence-length`,
`data-sequence-push`, `data-sequence-pop`, `data-sequence-get`,
`data-sequence-replace` and `data-sequence-discard`. Get returns an ordinary
immutable value that remains valid after later replacement, removal, destruction
or transfer of the source. Replace consumes the sequence and returns an owned
product with `rest` and the displaced `value`. The owned `sequence-replace<T: Owned>`
additionally consumes its exact replacement owner. Both forms retain allocation
and capacity; invalid indices trap, and failure joins cleanup.

The [wrapper request](requests/20261011-data-sequences.lkjc) and
[ten fixed tests](requests/20261011-data-sequences-tests.lkjc) are ordinary native
proposals. Tests cover empty reuse, repeated reads, signed scalar extremes, Bool
updates, retained immutable aggregate snapshots, populated cleanup and displaced
owned payloads. The [data-sequence guide](../../docs/guides/native-data-sequences.md)
defines argument order and the maintained planner's working-storage use. Accepted
graph meaning remains the authority; wrappers add no new runtime intrinsic.

The current package also owns exact-interface affine capability resources and canonical operation
parameter use. `DurableQueue` has nine operations: claim and heartbeat return the nominal
absent/live `QueueLeaseState`; `lease-info` borrows its live resource; heartbeat, complete, and fail
consume. `QueueLeaseInfo` exposes job ID, attempt number, lease deadline, and payload without raw
attempt or worker transition authority.

`TransactionOutcome<T>` is the ordinary nominal variant `Committed(T) | Aborted(TransactionAbortReason)`;
the ordinary abort reason is `ConditionFailed | Conflict`. Both types are constructible data.
The lexical `transaction-outcome` expression binds these exact declarations and cases and returns
the store's completed decision. It adds no capture or serialization permission to `T`. Ordinary
requirement-parametric libraries expose both caller-owned participation and standalone completion;
the transported typed-cell witness is maintained by the
[requirement-library observation](../../tools/lkjscript-dev/src/offline_packages/requirements.md).

`data-cell-update-in-transaction<T;E;R>(space, fallback, transform, key) -> T` stages one update
inside the caller's matching canonical transaction. It first invokes exact
`DataStore.require-transaction() -> Unit`, then reads the transaction view, invokes the task
transform once, encodes and conditionally writes the candidate. The required operations are
`get`, `put` and `require-transaction`; its result is tentative even after a failed condition.
`data-cell-try-update<T;E;R>` additionally requires `transaction`, owns `transaction-outcome`,
calls the same participant and returns `Committed(T)` only after successful finalization or
`Aborted(ConditionFailed|Conflict)` without T. Same-canonical nested ownership still rejects.

Both bodies are ordinary native-authored graph functions. Their
[literal reviewed request](requests/20260922-data-cells.lkjc) retains the public authoring input;
the accepted graph remains the editable authority. Fallback on missing or incompatible typed
encoding is explicit caller policy, not migration or conversion of operational failure into a
value. No blanket capture-safe constraint applies to T. Binding and encoding retain their own
admission rules. Argument effects may precede the guard, and independent callback effects can
survive an abort. No automatic retry occurs. The
[normative contract](../../docs/spec/data-capabilities.md#explicit-participation-and-typed-cells)
defines the authority, accounting and completion limits.

Current identity:

- repository: `repo_c1358d64c351873b51c954b69d1ac988`;
- package: `pkg_10000000000000000000000000000001`;
- semantic revision: `rev_2c307fbdefe23a178a2762148447fdac853f8e9b198b53294da8302f28270ced`;
- package revision: `package_revision_438773e160af5a00bdd72e4c576286d9add6bf5e9948e7b013f37b3228f9b591`;
- package transport: `package_transport_a0646e9a61973a84b9b29bebf2d3f520a5cb981921b5637e3a38f3956b93bbc7`;
- artifact manifest: `artifact_manifest_2e7c931f77bae42d3eb538aa69d1d1b8c057411154cd92747b13c440a1cec6c5`;
- artifact bundle: `artifact_bundle_cdd6d3d95261b1304c49bbf69591eb9fcf60571e281cc0b1eb5cfda9974d086e`;
- artifact bundle size: 1,278,543 bytes;
- 2,800 live semantic owners, 291 compiler units, and 116 graph tests.

The public check requires agreement between production and reference execution for
all 116 tests. Source-full and finalized-archive acceptance remain separate
obligations recorded by [status](../../docs/status.md).

Graph-owned `pair<First,Second>`, `pair-new`, `pair-first`, `pair-second` and `pair-map` compose
ordinary parametric records with pure functions. Mapping invokes the first callback then the second,
once each; a trap stops later callbacks. Three maintained tests cover construction/projection and
ordered heterogeneous mapping. No pair-specific host intrinsic or runtime opcode implements them.

Inspect and reproduce it from the repository root:

```sh
target/release/lkjscript --project packages/standard status
target/release/lkjscript --project packages/standard query owners --limit 20
target/release/lkjscript --project packages/standard check
target/release/lkjscript --project packages/standard build \
  --output /tmp/standard-current.lkja
target/release/lkjscript package builtin inspect
target/release/lkjscript package builtin query owners --name json-decode-or
target/release/lkjscript package builtin inspect owner external decl_...
target/release/lkjscript package builtin export --kind transport \
  --output /tmp/builtin-standard.lkjp
target/release/lkjscript package builtin export --kind artifact \
  --output /tmp/builtin-standard.lkja
```

The current package includes generic strict `json-decode-or<T>`, `json-encode<T>`,
`data-encode<T>`, `data-decode-or<T>`,
`list-length<T>`, `list-get<T>`, and
`list-fold-left<Item, State>(List<Item>, State, Function(State, Item) -> State) -> State`.
The fold and its private recursive helper are ordinary typed meaning; no fold-specific intrinsic
or runtime opcode exists. Empty, singleton, ordered multi-item, and distinct i64/bool
instantiations are graph-owned tests, and the maintained stateful HTTP workflow passes its header
predicate as a named function value. Exact type parameters, signatures, implementation-free
references, and the rest of the public interface are executable-generated in
[`docs/generated/builtin-standard.md`](../../docs/generated/builtin-standard.md).

`list-map<Input,Output>(List<Input>, Function(Input)->Output) -> List<Output>` binds its mapper
into a private generic fold step. The graph invokes the mapper once per item in ascending index
order, appends successful results, and stops at the first failure. Empty input invokes no mapper.
Four additional graph tests cover empty, singleton, configured multi-item and heterogeneous maps.
The shared runtime carrier preserves old aliases through bounded tail/spine copying and leaves
existing list types, encodings, callback eligibility, and default execution limits unchanged.

`list-window<Item>(items: List<Item>, start: I64, count: I64) -> List<Item>` selects values in
their original order. Start is clamped to zero through the list length; count is clamped to zero
through the remaining length before computing the end. Negative count or a start past the end
returns an empty list. Both signed I64 extremes are valid inputs. The pure body and its private
tail-recursive helper use ordinary indexed reads and persistent append, without a new intrinsic.
The [literal native request](requests/20260923-list-window.lkjc) and eight graph tests cover ranges,
empty input, extreme bounds, Unicode and structural records. The maintained transported aggregation
consumer uses it for ordered entry pages and consumer-owned nominal events.

`task-fold-left<Item,State;E>(List<Item>, State, TaskFunction(State,Item)->State ! E)->State ! E`
and `task-map<Input,Output;E>(List<Input>, TaskFunction(Input)->Output ! E)->List<Output> ! E`
are graph-owned tasks. The private sequential fold advances an index and state over the original
persistent list. Map binds its mapper into a private
task step and uses persistent append. Every callback runs once in input order; empty input invokes
none. Additional traversal control space is constant, while indexed reads retain their existing
cost. The library holds no grants and declares no concrete consumer requirements. Caller allowances,
component bindings, canonical grant counters and lexical transactions remain invocation boundaries.
The [offline package owner](../../tools/lkjscript-dev/src/offline_packages.rs)
verifies transported configuration/data composition separately from maintained
application behavior.

`iteration-step<State,Output>` is the ordinary public nominal variant `continue(State) | done(Output)`.
`task-iterate<State,Output;E>(State, TaskFunction(State)->iteration-step<State,Output> ! E)->Output ! E`
invokes the callback once per decision, continuing with its next state or returning its output.
Immediate completion invokes once. State and output are unconstrained ordinary types. The function
has no implicit iteration limit, scheduler or exception conversion; execution budgets and effect
admission still apply. Its implementation and the fold loop use ordinary task tail calls in both
evaluators, with invocation resources and ancestor transaction continuations preserved.

`function-compose<A,B,C>(outer: Function(B)->C, inner: Function(A)->B) -> Function(A)->C`
uses `bind` over its private generic graph helper. Four maintained graph tests fix composition
order and heterogeneous `I64 -> Bool -> Text` instantiation. Runtime bound prefixes may capture
other checked pure callables or recursively safe immutable data; bare unconstrained stored type
parameters, secrets, streams, and resources are rejected.

The `HttpClient` interface has exactly one idempotent, possibly externally visible `get` operation.
It accepts only ordered headers and returns status, ordered headers, and whole body bytes; endpoint,
DNS/address, TLS trust, retry/redirect, deadline, and cleanup policy remain deployment authority.

The canonical session family owns `SessionEvent`, `SessionMessageKind`, `SessionDecisionKind`,
`SessionOutbound`, `SessionReject`, and `SessionClose`. A structural `SessionDecision<State>`
reuses one exact closed ordinary state type across callbacks; no connection, stream, capability,
function, secret, or runtime handle can enter retained state. The normative relation and phase
protocol are specified in
[`docs/spec/structured-sessions.md`](../../docs/spec/structured-sessions.md).

`generated/standard.lkjp` and `generated/standard.lkja` are deterministic derived owners for the
executable's embedded package transport and artifact bundle bytes. Product verification regenerates
these outputs from the typed meaning graph and compares both embedded exports byte for byte. These
files, artifact paths, and package transport are not another editable program authority.

`function-constant<Value: capture-safe, Argument>(value: Value) -> Function(Argument)->Value`
binds its runtime value into the private ordinary `function-constant-first<Value,Argument>` helper.
Argument and both helper parameters stay unconstrained. Runtime Text, lists and safe nominal data
can be retained; an empty `List<Secret>` still rejects. Two maintained tests fix scalar behavior
and nested `List<Text>` results through standard map. No constant-function intrinsic is involved.

`bytes-get(bytes: Bytes, index: I64) -> I64` observes the unsigned octet at a
zero-based byte offset. It does not decode UTF-8, copy the complete buffer or alter
aliases. Negative, past-end and empty-buffer indexing traps, including both signed
I64 extremes; a caller can guard with `bytes-length`. The closed representation
primitive is `core.bytes.get`. Its [native declaration and six fixed graph tests](requests/20260924-bytes-get.lkjc)
cover NUL, ASCII and individual UTF-8 octets. Independent evaluator/public tests cover
all 256 octets, invalid encodings and bounds. The maintained repository policy
inspects Bytes directly.

`text-join(items: List<Text>, separator: Text) -> Text` is an ordinary pure function.
Empty input returns empty text; a singleton returns its item; separators occur exactly
between items, including empty ones. Text, order, Unicode and control characters are
preserved without quoting, escaping or normalization. A private balanced-range helper
prefixes nonfirst leaves and combines halves; empty separators retain leaf values
directly. This has logarithmic recursive depth but still copies concatenated text and
retains the existing list-index cost. It introduces no intrinsic or output-limit change.
The [native request](requests/20260924-text-join.lkjc) adds twelve fixed graph tests;
the [native text guide](../../docs/guides/native-text.md) demonstrates ordinary mapping,
caller-owned HTML escaping and detached execution. The maintained reference-page
tool uses this function.

## Byte ranges and explicit copies

`bytes-slice(bytes: Bytes, start: I64, end: I64) -> Bytes` selects `[start, end)` with
strict `0 <= start <= end <= bytes-length(bytes)` bounds. Invalid bounds trap;
valid empty ranges return empty Bytes. Offsets count bytes, not UTF-8 characters.
The production checked evaluator shares immutable backing without copying the
selected payload. Nested views stay flat, aliases are unchanged and empty ranges
do not retain the original allocation. A small nonempty range may retain a large parent.

`bytes-copy(bytes: Bytes) -> Bytes` creates an independent nonempty backing containing
only the visible bytes. Use it when a small retained result should not keep its
original parent alive. Other aliases still own that parent. Copying has an explicit
allocation/copy cost; it does not promise secure erasure or immediate RSS reduction.
Both operations remain ordinary immutable Bytes and introduce no region or borrow type.

The closed primitives are `core.bytes.slice` and `core.bytes.copy`. The
[literal native request](requests/20260930-byte-ranges.lkjc) adds ten fixed graph
tests. The [native guide](../../docs/guides/native-byte-ranges.md) covers exact library
composition, captures, map keys and source-free command execution.

## Byte construction and UTF-8 results

`bytes-from-list(octets: List<I64>) -> Bytes` constructs the exact unsigned byte
sequence. Empty input is valid. Values outside 0 through 255 trap; they are never
clamped, wrapped or interpreted as Unicode scalar values. The input list and its
aliases are unchanged. Checked execution reserves scratch and retained output before
each allocation and checks cancellation during traversal.

`bytes-to-text-result(bytes: Bytes) -> {valid: Bool, value: Text}` accepts only
complete valid UTF-8. Success preserves text exactly, including BOM, NUL, line endings
and supplementary scalars. Invalid encoding returns false and empty text, never a
valid prefix or replacement characters. Empty valid text returns true and empty text.
Allocation, cancellation and other operational failures remain errors, not false.
The existing trapping `bytes-to-text` is unchanged.

The closed primitives are `core.bytes.from-list` and `core.bytes.to-text-result`.
The [literal public request](requests/20260925-byte-conversion.lkjc) adds two exact
externals and twelve fixed graph tests, without changing graph or data encodings.

## Derived assets

Public check/build/export regenerate the maintained bundle and transport from
accepted meaning. All canonical formats admit only `SCHEMA_EPOCH = 1`; see
[the schema owner](../../build.rs). Source changes bind validation and compiler
caches to the current implementation without changing the schema epoch.
Regenerate affected assets after a schema change.
Rebuilding derives new bytes without changing semantic HEAD, running deployments
or operational data. [Status](../../docs/status.md) owns actual acceptance and
publication; graph tests alone do not establish either.

## Native owned FIFO

The standard exports `fifo-empty<T: Owned>`, `fifo-length`, `fifo-push`,
`fifo-pop`, `fifo-front` and `fifo-discard` as ordinary pure graph functions.
Their public nominal `Fifo<T: Owned>` contains incoming and outgoing owned
sequences, with representation access private to the exact core module.
Consumers name `(std::Fifo T)` and use those functions; a structural product
with the same field names cannot substitute for the nominal queue.
Push and pop transfer custody; both pop outcomes retain the queue for reuse.
Front returns an exact source-tied borrowed element without removing it, and an
empty front traps. Existing lexical loan rules protect every ancestor owner.

The private refill moves each enqueued element between stacks at most once;
this is an amortized sequence-transfer bound, not a constant VM-time or
bounded-latency claim. No FIFO-specific runtime primitive or encoding change is
introduced by the FIFO operations. The
[native migration request](requests/20261011-nominal-owned-fifo.lkjc) updates the
existing callable and test identities through ordinary draft/plan/apply.
The [native guide](../../docs/guides/native-owned-fifo.md) records
representation, allocation, lifetime and resource limits. Ten fixed
graph tests cover order, empty reuse, mixed stacks, repeated front reads and
nested cleanup. The [independent consumer](../../examples/owned-fifo/README.md)
exercises transported generic code and source-free execution over three distinct
owned element representations. Publication remains separately accepted.
