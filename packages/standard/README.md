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
`rev_1af582dbebc01b43cd1050349f208b7c71c92ca4efd3f6b65624745f7d9c988e`; it owns 7 graph tests.
Its package artifact is
`artifact_6ea73654d153ac4410ff4aaad329373dce27a58bb0d8c61eaa31cd6d66bcb3f6`, and the current
executable bundle digest is
`artifact_3648f87daea0164ef6e94ea6e731dd687db590b8889583f63cac6587f5e7a4d1`.
Consumers bind the exact semantic revision and package artifact digest. Artifact paths are
deployment or transport locators, not semantic identity.
The executable's built-in package is an integrity-checked copy of this maintained artifact;
inspection and export do not create another editable program authority.
