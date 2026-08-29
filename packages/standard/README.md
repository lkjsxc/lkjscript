# standard

This directory is the maintained Graph 5 authority for the exact built-in standard package. Its
12 modules define deterministic core operations and typed interfaces for HTTP values, JSON,
PostgreSQL, configuration, secrets, clocks, secure randomness, identifiers, password hashing,
byte streams, named object storage, and durable queues.

An external declaration is not arbitrary FFI. Its dotted implementation name must match the
closed semantic-validator/runtime intrinsic inventory before publication or execution. Capability
interfaces own typed operations, failure behavior, idempotency, possible visibility, and limits;
deployment grants select concrete adapters later.

Current identity:

- repository: `repo_c1358d64c351873b51c954b69d1ac988`;
- package: `pkg_10000000000000000000000000000001`;
- semantic revision: `rev_b7e85425b4d2a15c6e7cbdc2c9128addeaebf24b9cb3dd626f2570ba47da23ee`;
- package revision: `package_revision_b133c038d2997b440d5a6ec3fe9ec326e6c7c2c75259be7499aa234313bd6515`;
- package transport: `package_transport_9326e2744a3bfe401ef03750c162d32c1e3d4151a9b384fdd8fb28261601464a`;
- artifact manifest: `artifact_manifest_48e18403aec9c5c74db8c4a0d75633cbe4f38648218c2e58fe5d7d3d1ca267a0`;
- artifact bundle: `artifact_bundle_47b5cc49c1ca833538933091b6648ef953eaa546337a63974f1aed6467c17f1b`;
- 381 semantic owners, 72 compiler units, and 11 graph tests.

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
`list-length<T>`, `list-get<T>`, and
`list-fold-left<Item, State>(List<Item>, State, Function(State, Item) -> State) -> State`.
The fold and its private recursive helper are ordinary Graph meaning; no fold-specific intrinsic
or runtime opcode exists. Empty, singleton, ordered multi-item, and distinct i64/bool
instantiations are graph-owned tests, and the maintained stateful HTTP workflow passes its header
predicate as a named function value. Exact type parameters, signatures, implementation-free
references, and the rest of the public interface are executable-generated in
[`docs/generated/builtin-standard.md`](../../docs/generated/builtin-standard.md).

`generated/standard.lkjp` and `generated/standard.lkja` are deterministic derived owners for the
executable's embedded package transport and artifact-10 bytes. Product verification regenerates
the Graph 5 outputs and compares both embedded exports byte for byte. These files, artifact paths,
and package transport are not another editable program authority.
