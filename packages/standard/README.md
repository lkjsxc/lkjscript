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
target/release/lkjscript --project packages/standard inspect status
target/release/lkjscript --project packages/standard inspect project --limit 20
target/release/lkjscript --project packages/standard check
target/release/lkjscript --project packages/standard build \
  --output /tmp/standard.lkja
target/release/lkjscript --project packages/standard doctor --deep
target/release/lkjscript package builtin inspect
target/release/lkjscript package builtin export --output /tmp/builtin-standard.lkja
```

Package identity is `10000000000000000000000000000001`; repository identity is
`repo_c1358d64c351873b51c954b69d1ac988`. The maintained graph-4 revision is
`rev_af36c21e869a22a992b982aafe959c6230311293094e9ded162e29872ce0afdf`; it owns 7 graph tests.
Its package artifact is
`artifact_cef17b4730c708a9e3dfdaa934af28fad58902fb011db1e1305fd840f459c57a`, and the current
executable bundle digest is
`artifact_b2f39efc64b987378a6abcb81ade2f14de354ace122dbea22f02a984de875cea`.
Consumers bind the exact semantic revision and package artifact digest. Artifact paths are
deployment or transport locators, not semantic identity.
The executable's built-in package is an integrity-checked copy of this maintained artifact;
inspection and export do not create another editable program authority.
