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
- semantic revision: `rev_27c3a79c798fe402d114e0000fefa0d628916808062d63d1782a6d9ed5e5aa83`;
- package revision: `package_revision_4290e78132570943c17a9cd800af0742dfc8c16baa6f471354792dab1d0db981`;
- package transport: `package_transport_76566ff6df6024e573d3fc7f868cbc74760170dbd2111805c4c8c30a3a95b154`;
- artifact manifest: `artifact_manifest_1d2b53b867cbe1027d4b537f34ecf93007ea28ce54f28bd6674ebdba0b15fe6e`;
- 284 semantic owners, 60 compiler units, and 7 graph tests.

Inspect and reproduce it from the repository root:

```sh
target/release/lkjscript --project packages/standard status
target/release/lkjscript --project packages/standard query owners --limit 20
target/release/lkjscript --project packages/standard check
target/release/lkjscript --project packages/standard build \
  --output /tmp/standard-current.lkja
target/release/lkjscript package builtin inspect
target/release/lkjscript package builtin export --kind transport \
  --output /tmp/builtin-standard.lkjp
target/release/lkjscript package builtin export --kind artifact \
  --output /tmp/builtin-standard.lkja
```

`generated/standard.lkjp` and `generated/standard.lkja` are deterministic derived owners for the
executable's embedded package transport and artifact-10 bytes. Product verification regenerates
the Graph 5 outputs and compares both embedded exports byte for byte. These files, artifact paths,
and package transport are not another editable program authority.
