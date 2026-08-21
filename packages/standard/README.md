# standard

This exact source package defines reusable lkjscript interfaces and pure helpers consumed by
`lkjournal`. Its modules cover deterministic core operations, HTTP header helpers, strict typed
database values, configuration, secrets, clocks, secure randomness, identifiers, password hashing,
byte streams, named object storage, and durable queues.

An `extern` declaration is not arbitrary FFI. Its implementation name must match the closed,
semantic-validator-owned intrinsic registry before a project revision can publish. Capability
interfaces describe operations, idempotency, possible visibility, and types; deployment grants
select concrete adapters later.

```sh
target/release/lkjscript --project packages/standard project orient
target/release/lkjscript --project packages/standard package test
target/release/lkjscript --project packages/standard package build --output /tmp/standard.lkja
```

Package identity is `10000000000000000000000000000001`. Consumers bind its exact semantic revision
and artifact digests in their package descriptor. The artifact path is only a checkout locator.
