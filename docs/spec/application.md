# Application artifact, exact-graph test, and invocation contracts

This specification owns the runnable application domain: exact release-graph selection, immutable
application cases, canonical bytes, inspection, publication, and pure invocation. Reusable-release
semantics are owned by [reusable-release.md](reusable-release.md). An application is not a
workspace, release, resolver state, executable cache, provenance statement, or deployment.

## Authority and build

Application command contract version 2 accepts an `ApplicationBuildRequest` containing:

- one exact root `ReleaseId`;
- one exported function target as `(ReleaseId, ReleaseItemId)`;
- `typed` or `bytes_stream` invocation profile;
- one exact `RunPolicy`; and
- one or more lexically named application invocation cases, including the entry.

Every release in the complete exact graph is supplied explicitly as hostile artifact bytes.
Application build never opens a workspace, consults HEAD, resolves a coordinate or user version,
uses a mutable store, or fetches from a network. `ReleaseGraph` independently decodes every object,
requires exactly the reachable closure, validates exact dependency slots and imports, rejects
cycles, privately flattens the graph, compiles the entry through the verified Core-IR path, and runs
every embedded release case and application case. Only an all-pass prepared object can publish.

Distinct exact releases remain distinct through flattening. One exact release reached through a
diamond is decoded once and mapped once. Imported nominal proxy declarations redirect to the exact
dependency release and item; compiler-dense IDs and runtime tags remain derived and never become
artifact identity.

## Canonical application artifact version 2

The envelope contains, in order:

- magic `LKJAPP\0\x02` and little-endian format `2`;
- semantic schema `lkjscript-tsm006`;
- a checked little-endian payload length;
- one canonical payload; and
- a 32-byte BLAKE3 digest derived under `lkjscript.application-artifact.v2`.

The payload contains the root release ID, exact entry, profile, policy, canonical application
cases, and every exact release artifact once in strict `ReleaseId` order. Embedded release bytes
remain independently valid release authorities; the application container does not reinterpret
their coordinate, exports, dependencies, tests, or nominal identities. The application graph
digest uses the separate `lkjscript.application-release-graph.v1` domain.

An application artifact is limited to 256 MiB. The embedded graph retains the release limits of
256 releases, 4,096 edges, depth 64, and 256 MiB aggregate release bytes. Application cases are
limited to 256, 64-byte names matching `[a-z][a-z0-9_]*`, 1 MiB per encoded public value, and
100,000,000 aggregate declared fuel. Runtime values, arguments, frames, fuel, and stream input have
their independently reported interpreter limits.

The decoder checks sizes before allocation; rejects wrong magic, old format 1, wrong schema,
unknown tags, invalid local IDs, duplicate or noncanonical ordering, missing or extra releases,
cycles, private targets, malformed values, digest mismatch, truncation, and trailing bytes;
reconstructs and validates the exact graph; and requires byte-identical re-encoding. Corruption is
rejected before compilation or execution. There is no compatibility reader.

The payload excludes workspace identity and history, revision identity, mutable resolution input,
filesystem paths, idempotency state, aliases, proposal text, caches, Core IR, ownership plans,
runtime handles, timestamps, provenance, signatures, attestations, and provider data.

## Application cases

Application cases are immutable typed data, not semantic declarations and not a second assertion
language. Each has one canonical name, exact exported function target, ordered exact public-value
arguments, one expected value or stable trap, and one policy. At least one case targets the entry.
Only exact value or trap equality passes; skipped, invalid, incomplete, cancelled, exhausted, or
engine-failed cases do not pass.

Public nominal values carry exact release/item pairs for their type, fields, and variants. A value
from R2 does not satisfy an R1 type even when coordinate, user version, display names, local
ordinals, and structure match. Two paths to the same exact release do satisfy one nominal type.

Application test digests use `lkjscript.application-test-case.v2`. They are bounded review keys,
not durable identities. Test runs do not mutate semantic authority. The test report includes every
embedded release case followed by application cases under closed status variants.

## Invocation

`typed` accepts strict version-2 JSON:

```json
{"version":2,"arguments":[{"kind":"bytes","data":"YWJj"}]}
```

Each value is checked against the exact flattened entry signature before execution. Output reports
contract version 2, application digest, exact public value, and compile/execute observations.

`bytes_stream` is legal only for an entry accepting one `bytes` and returning `bytes`. It reads at
most 65,536 uninterpreted standard-input bytes and writes exactly the result bytes. It grants no
filesystem, environment, network, clock, randomness, process, or signal authority. Both profiles
use the artifact policy and the explicit-frame interpreter.

## Files and publication

Artifact paths are absolute lexically canonical Unix paths no longer than 4,096 bytes. Dot, parent,
empty, repeated-separator, symlink-parent, symlink-input, and non-regular input forms reject.
Publication uses a private mode-0600 temporary file in the destination directory, complete write,
file synchronization, atomic no-replace hard link, temporary removal, and directory
synchronization. It never overwrites.

A failure before the public link is a known failure and leaves no destination. Any modeled failure
after the link is `artifact_publication_outcome_unknown`; the caller must reconcile the exact path
and must not silently retry. Deterministic tests inject every retained write/sync/link/cleanup edge.
Concurrent hostile directory administration and the operating-system/filesystem implementation
remain in the trusted local-host boundary; this is not a sandbox claim.

## Public commands

```text
app build --release FILE [--release FILE ...] (--validate-only | --output FILE)
app validate --artifact FILE
app inspect --artifact FILE
app test --artifact FILE
app run --artifact FILE
app stream --artifact FILE
```

Build and typed run consume strict bounded JSON. Validate and inspect never run code. Test executes
the complete embedded suite. Artifact operations need only the one application file.

Exit classes are `0` success, `2` usage/JSON, `3` transport or filesystem I/O, `4` output failure,
`5` corrupt artifact or unknown publication outcome, `6` application contract/profile rejection,
`7` trap or failed test verdict, and `8` resource exhaustion. Diagnostics are bounded and
terminal-safe.

## Explicit absences

Application format 2 embeds a target-neutral semantic release graph. It contains no external-store
dependency, resolver, lockfile, network registry, native code, bytecode, serialized Core IR,
executable cache, platform ABI, host permission, external resource, signature, provenance, or
deployment contract. Application format 1 and `LKJAPP\0\x01` reject directly.
