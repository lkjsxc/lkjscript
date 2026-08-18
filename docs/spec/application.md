# Application-world artifact and invocation contract

This specification owns immutable runnable application worlds: exact release-graph selection,
exports, imported host-interface requirements, pure tests, canonical bytes, inspection, and pure
invocation. Reusable releases are specified in [reusable-release.md](reusable-release.md); mutable
instance operation and grants are specified in [instance.md](instance.md). An application is not a
workspace, release, instance, grant, deployment, adapter, executable cache, or provenance claim.

## Authority and build

Application command contract version 4 accepts one strict `ApplicationBuildRequest` containing:

- one exact root `ReleaseId` and exported entry target;
- one closed invocation profile;
- one exact semantic `RunPolicy`; and
- one or more named exact pure application cases, including the entry.

Every release in the complete graph is supplied explicitly as hostile artifact bytes. Build never
opens a workspace, resolves a coordinate, consults mutable HEAD, uses a registry, or fetches from a
network. The release graph independently decodes the exact reachable closure, validates dependency
slots, rejects cycles and private targets, flattens exact nominal identity, lowers through verified
Core IR, and runs all retained release and application tests. Only an all-pass prepared object may
publish.

Distinct releases remain distinct. A release reached twice through a diamond is decoded and mapped
once. Compiler IDs, Core IDs, layouts, ownership plans, runtime handles, and timing observations are
derived and absent from application authority.

## Invocation profiles

The version-4 profile is one closed tagged value:

- `typed` permits exact typed pure invocation;
- `bytes_stream` requires one `bytes -> bytes` entry; or
- `stateful` declares the application-world transition contract below.

The stateful profile names exact nominal `State`, `Event`, `Command`, `Outcome`, and `Decision`
types. Its entry has signature `(State, Event) -> Decision`; its resume entry has signature
`(State, Outcome) -> Decision`. `Decision` is an exact nominal sum with two exact payload products:

```text
Decision = completed { state: State, response: bytes }
         | suspended { state: State, response: bytes, command: Command }
```

The profile names the sum variants, payload products, and fields, so declaration order or a human
name cannot substitute for semantic identity. Complete and suspended results are structurally
distinct. There is no integer command discriminator, fixed target field, raw outcome tag, or opaque
continuation.

Evaluation is pure. Returning a suspended decision performs no host action and publishes no state.
The application artifact declares requirements only; it contains no grant, adapter descriptor,
instance identity, filesystem path, or ambient authority.

## Imported host interfaces

A stateful application declares zero or more imports in strict canonical slot order. Each import
binds:

- one canonical application-local slot;
- one closed built-in `HostInterface` and its derived exact identity;
- one exact nominal request sum and outcome sum;
- one exact wrapper variant in the application `Command` sum;
- one exact wrapper variant in the application `Outcome` sum;
- a complete request-variant-to-host-operation routing table; and
- the compatible host-operation/outcome-class-to-outcome-variant table.

The retained interface contracts are `application_activation_v1` and `immutable_blob_v1`.
`HostInterfaceId` is a domain-separated BLAKE3 digest of the closed contract name under
`lkjscript.host-interface.identity.v1`. It identifies that immutable interface contract only. It is
not a grant, adapter, publisher identity, signature, or mutable registry key.

Activation owns operations `validate_application`, `activate_application`, and
`reconcile_activation`. Immutable blob owns `put_blob` and `inspect_blob`. A route whose operation
belongs to another interface rejects. Duplicate variants, missing routes, foreign nominal targets,
non-sum request/outcome types, or a command selecting an undeclared slot reject before publication.

An application command is ordinary nominal data. The validator unwraps exactly one declared
command variant, validates its request payload against that import, and derives the slot, interface
identity, host operation, and typed request. Resume receives only the application `Outcome` value
formed by the exact compatible route. Runtime infrastructure metadata does not enter the resume
signature.

The closed infrastructure classes are `succeeded`, `already_present`,
`known_failure_before_visibility`, `outcome_unknown`, `reconciliation_present`,
`reconciliation_absent`, `reconciliation_indeterminate`, `cancelled_before_action`,
`timeout_before_action`, `timeout_after_possible_visibility`, and `cleanup_failure`. An application
declares only compatible routes it uses. Expected workflow meaning remains its own nominal outcome
variants; corruption, authority denial, resource exhaustion, and unavailable infrastructure remain
outside ordinary application data.

## Canonical application artifact version 4

The envelope contains, in order:

- magic `LKJAPP\0\x04` and little-endian format `4`;
- semantic schema `lkjscript-tsm006`;
- a checked little-endian payload length;
- one canonical payload; and
- a 32-byte BLAKE3 digest under `lkjscript.application-artifact.v4`.

The payload contains the root release ID, entry, complete profile and import routing, run policy,
canonical tests, and every exact release artifact once in strict `ReleaseId` order. Embedded release
bytes remain independent release authorities. The graph digest remains
`lkjscript.application-release-graph.v1` because graph meaning did not change.

An application is limited to 256 MiB. Its graph is limited to 256 releases, 4,096 edges, depth 64,
and 256 MiB aggregate release bytes. Tests are limited to 256, names to 64 canonical bytes, public
value JSON to 1 MiB, and aggregate declared suite fuel to 100,000,000. Runtime arguments, values,
frames, fuel, and byte-stream input retain their independent interpreter limits.

The decoder checks lengths before allocation and rejects wrong magic or schema, every unsupported
format including version 3, unknown tags, invalid IDs, duplicate/noncanonical order, incomplete or
excessive graphs, private targets, malformed values, invalid world routing, digest mismatch,
truncation, trailing bytes, and non-byte-identical re-encoding. There is no compatibility reader.

Application identity is the exact artifact digest within this domain. It implies neither continuity,
publisher, provenance, authorization, freshness, nor grant.

## Pure tests and invocation

Application cases are immutable exact inputs plus an exact value or stable trap expectation. At
least one case targets the entry; a stateful application also requires a resume case. Stateful tests
use ordinary typed commands and fake typed outcomes but never call an adapter. Only exact equality
passes; skipped, incomplete, cancelled, exhausted, invalid, or engine-failed cases do not pass.

Public nominal values carry exact release/item identities. Structurally equal types from distinct
releases remain distinct. Test digests use `lkjscript.application-test-case.v2`; they are bounded
review keys, not durable identity.

Typed invocation accepts strict version-4 JSON:

```json
{"version":4,"arguments":[{"kind":"bytes","data":"YWJj"}]}
```

Arguments are validated against the exact flattened signature before execution. The receipt binds
contract version 4, application digest, exact public value, and bounded nonsemantic lowering, Core
verification, and execution observations. Invoking a stateful entry directly is pure evaluation;
its returned decision is not instance authority.

`bytes_stream` reads at most 65,536 uninterpreted stdin bytes and writes exactly the result bytes.
It grants no filesystem, environment, network, clock, randomness, process, or signal authority. All
profiles use the verified Core route and explicit-frame interpreter.

## Files, publication, and commands

Artifact paths are bounded absolute lexically canonical Unix paths. Dot, parent, empty,
repeated-separator, symlink-parent, symlink-input, and non-regular forms reject. Publication writes
and synchronizes a private same-directory temporary, atomically creates the destination without
replacement, removes the temporary, and synchronizes the directory. Failure before the public link
is known no-change. Failure after it is `artifact_publication_outcome_unknown`; callers inspect and
must not silently repeat.

```text
app build --release FILE [--release FILE ...] (--validate-only | --output FILE)
app validate --artifact FILE
app inspect --artifact FILE
app test --artifact FILE
app run --artifact FILE
app stream --artifact FILE
```

One application file remains sufficient after source workspaces and standalone release artifacts
are removed. These one-shot commands and the runtime kernel use the same application owner.

## Explicit absences

Application format 4 contains no dependency store, resolver, registry, interface artifact,
interface registry, dynamic plugin ABI, native code, bytecode, serialized Core IR, executable cache,
grant, live resource, scheduler, signature, or provenance. Application format 3
(`LKJAPP\0\x03`) and all other formats reject directly.
