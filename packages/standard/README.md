# standard

The canonical graph in this directory defines reusable lkjscript interfaces and pure helpers used
by `lkjournal`. Its 12 modules cover deterministic core operations, HTTP values, typed database
values, configuration, secrets, clocks, secure randomness, identifiers, password hashing, byte
streams, named object storage, and durable queues.

An external declaration is not arbitrary FFI. Its implementation name must match the closed
semantic-validator-owned intrinsic registry before a revision can publish. Capability interfaces
own typed operations, failure classes, idempotency, possible visibility, and limits; deployment
grants select concrete adapters later.

```sh
target/release/lkjscript --project packages/standard semantic status
target/release/lkjscript --project packages/standard semantic orient --limit 20
target/release/lkjscript --project packages/standard semantic test
target/release/lkjscript --project packages/standard semantic build \
  --output /tmp/standard.lkja
target/release/lkjscript --project packages/standard semantic doctor --deep
```

Package identity is `10000000000000000000000000000001`; repository identity is
`repo_c1358d64c351873b51c954b69d1ac988`. Consumers bind the exact semantic revision and artifact
digest. Artifact paths are deployment or transport locators, not semantic identity.
