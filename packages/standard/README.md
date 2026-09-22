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
- semantic revision: `rev_811d0d7a7f1792d6934eaa62defd6ea03c4306e01bcd7e463eb8c1cf68a0443e`;
- package revision: `package_revision_3b195535785762a7dcdec5df40080cd51e1abbb592bd2edfd2c26a05bcdd0a92`;
- package transport: `package_transport_2cc869f1c3049806d7481e4934d9ad575c0fbe88ae45cc412d1d13a1d0e6e676`;
- artifact manifest: `artifact_manifest_e4df6c4e62c4cddf973e69eed3856fcbf75a856762880ce819cc64b9b2fab826`;
- artifact bundle: `artifact_bundle_56689036f3927a117963e54ffd091bf81b71e2a828f2e5a64a287fc81afd4325`;
- 1,012 live semantic owners, 165 compiler units, and 37 graph tests.

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

`task-fold-left<Item,State;E>(List<Item>, State, TaskFunction(State,Item)->State ! E)->State ! E`
and `task-map<Input,Output;E>(List<Input>, TaskFunction(Input)->Output ! E)->List<Output> ! E`
are graph-owned tasks. The private sequential fold advances an index and state over the original
persistent list. Map binds its mapper into a private
task step and uses persistent append. Every callback runs once in input order; empty input invokes
none. Additional traversal control space is constant, while indexed reads retain their existing
cost. The library holds no grants and declares no concrete consumer requirements. Caller allowances,
component bindings, canonical grant counters and lexical transactions remain invocation boundaries.
The fresh transported configuration/data workload is recorded separately from maintained application
behavior in the [effect campaign](../../docs/campaigns/202609111843.md).

`iteration-step<State,Output>` is the ordinary public nominal variant `continue(State) | done(Output)`.
`task-iterate<State,Output;E>(State, TaskFunction(State)->iteration-step<State,Output> ! E)->Output ! E`
invokes the callback once per decision, continuing with its next state or returning its output.
Immediate completion invokes once. State and output are unconstrained ordinary types. The function
has no implicit iteration limit, scheduler or exception conversion; execution budgets and effect
admission still apply. Its implementation and the fold loop use ordinary task tail calls in both
evaluators, with invocation resources and ancestor transaction continuations preserved. Fresh
transported adoption is tracked by the [task iteration campaign](../../docs/campaigns/202609121214.md).

`function-compose<A,B,C>(outer: Function(B)->C, inner: Function(A)->B) -> Function(A)->C`
uses `bind` over its private generic graph helper. Four maintained graph tests fix composition
order and heterogeneous `I64 -> Bool -> Text` instantiation. Runtime bound prefixes may capture
other checked pure callables or recursively safe immutable data; bare unconstrained stored type
parameters, secrets, streams, and resources are rejected. The Graph 14 cutover retains these bodies;
unchanged TypeObject 10 bytes and typed-data layout identities remain current.

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
