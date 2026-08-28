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
- semantic revision: `rev_856f4cab0ddd42b2719694a3d1b23553f248b3bd887c54dc19b90d506724b234`;
- package revision: `package_revision_be57b4a64f267a5ffb64bc576e6546ac04d51aa79fd059ebecdac3558386a665`;
- package transport: `package_transport_dd83db89a5a492b9195e439b759eafe259911e967a254d6f0dfaba36442bec4c`;
- artifact manifest: `artifact_manifest_844aed53e4be165ab6907147831a7b751e244b66dcd048000a7df2e65868d98b`;
- artifact bundle: `artifact_bundle_e5f346fa99ea4346cfa76e4a7bc5e605dbbba72770ef65701ca1911756c6aa12`;
- 298 semantic owners, 64 compiler units, and 7 graph tests.

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
`list-length<T>`, and `list-get<T>` declarations used by the maintained public stateful HTTP
workflow. Their exact type parameters, signatures, implementation-free references, and the rest
of the public interface are executable-generated in
[`docs/generated/builtin-standard.md`](../../docs/generated/builtin-standard.md).

`generated/standard.lkjp` and `generated/standard.lkja` are deterministic derived owners for the
executable's embedded package transport and artifact-10 bytes. Product verification regenerates
the Graph 5 outputs and compares both embedded exports byte for byte. These files, artifact paths,
and package transport are not another editable program authority.
