# standard

This directory is the maintained typed meaning authority for the exact built-in standard package.
Its 12 modules define deterministic core operations and typed interfaces for HTTP values, JSON,
canonical typed application data, the ordered `DataStore`, configuration, secrets, clocks, secure
randomness, identifiers, password hashing, byte streams, deployment-bound outbound HTTP, named
object storage, and durable queues.

An external declaration is not arbitrary FFI. Its dotted implementation name must match the
closed semantic-validator/runtime intrinsic inventory before publication or execution. Capability
interfaces own typed operations, failure behavior, idempotency, possible visibility, and limits;
deployment grants select concrete adapters later.

The current package also owns exact-interface affine capability resources and canonical operation
parameter use. `DurableQueue` has nine operations: claim and heartbeat return the nominal
absent/live `QueueLeaseState`; `lease-info` borrows its live resource; heartbeat, complete, and fail
consume. `QueueLeaseInfo` exposes job ID, attempt number, lease deadline, and payload without raw
attempt or worker transition authority.

Current identity:

- repository: `repo_c1358d64c351873b51c954b69d1ac988`;
- package: `pkg_10000000000000000000000000000001`;
- semantic revision: `rev_02a0f86cf6b5bf17bc5227b898afd59fe9d7f5228eb297f69e4acf12f79c0261`;
- package revision: `package_revision_03dba0a36440d6d0cba6fcc7d22b626db113a927aab40163500a322f1a666705`;
- package transport: `package_transport_8d351f69935443c3a4276605ffd86e9f5c91fa05511258ccf4414c6f2b8678e9`;
- artifact manifest: `artifact_manifest_a44c48ac71f99688c22b1fb3a39f9bafe36792e3a697546d8bd2e06644bcc55f`;
- artifact bundle: `artifact_bundle_bbdebe268c69488400dbd99d6da9056b6fa436e250910d8674ea64c431c03a05`;
- 431 live semantic owners, 83 compiler units, and 13 graph tests.

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

The `HttpClient` interface has exactly one idempotent, possibly externally visible `get` operation.
It accepts only ordered headers and returns status, ordered headers, and whole body bytes; endpoint,
DNS/address, TLS trust, retry/redirect, deadline, and cleanup policy remain deployment authority.

`generated/standard.lkjp` and `generated/standard.lkja` are deterministic derived owners for the
executable's embedded package transport and artifact bundle bytes. Product verification regenerates
these outputs from the typed meaning graph and compares both embedded exports byte for byte. These
files, artifact paths, and package transport are not another editable program authority.
