# Application artifact, exact-graph test, and invocation contracts

This specification owns immutable runnable applications: exact release-graph selection, entry
interfaces, application cases, canonical bytes, inspection, publication, and pure invocation.
Reusable releases are specified in [reusable-release.md](reusable-release.md); mutable operation is
specified in [instance.md](instance.md). An application is not a workspace, release, instance,
capability grant, deployment slot, resolver state, executable cache, or provenance statement.

## Authority and build

Application command contract version 3 accepts one strict `ApplicationBuildRequest` containing:

- one exact root `ReleaseId`;
- one exported entry target as `(ReleaseId, ReleaseItemId)`;
- one closed invocation profile;
- one exact `RunPolicy`; and
- one or more named exact application cases, including the entry.

Every release in the complete graph is supplied explicitly as hostile artifact bytes. Build never
opens a workspace, resolves a coordinate or user version, consults mutable HEAD, uses a store, or
fetches from a network. `ReleaseGraph` independently decodes the exact reachable closure, validates
imports and dependency slots, rejects cycles and private targets, flattens nominal identity, lowers
through verified Core IR, and runs all embedded release and application cases. Only an all-pass
prepared object may publish.

Distinct releases remain distinct. A release reached twice through a diamond is decoded and mapped
once. Compiler IDs, Core IDs, layout tags, and runtime handles are derived and absent from the
artifact.

## Invocation profiles

The version-3 JSON profile is a closed tagged object:

- `{"kind":"typed"}` permits exact typed invocation;
- `{"kind":"bytes_stream"}` requires one `bytes -> bytes` entry; or
- `{"kind":"stateful","data":{...}}` declares the stateful interface below.

The stateful profile names one exported event entry, which is also the application entry, one
exported resume entry, one nominal `Decision` product, and its exact four fields. The event entry
has signature `(State, Event) -> Decision`; `State` and `Event` must be nominal. Resume has signature
`(State, i64, bytes) -> Decision`. `Decision` has exactly these fields in declaration order:

```text
state: State
response: bytes
command: i64
target: bytes
```

The command tag is `0` completed, `1` validate an exact application, `2` activate an exact
application, or `3` reconcile activation. Tag zero requires an empty target. Every nonzero tag
requires exactly one 32-byte application digest. Unknown tags and impossible tag/target
combinations reject. The result is ordinary typed semantic data; evaluating either entry performs
no host action and publishes no state. No stack, continuation, compiler ID, live handle, or
capability is persisted.

Host outcome tags and the durable suspension protocol are owned by
[instance.md](instance.md). Application format 3 declares required semantic intent, not authority;
an instance supplies the exact grant.

## Canonical application artifact version 3

The envelope contains, in order:

- magic `LKJAPP\0\x03` and little-endian format `3`;
- semantic schema `lkjscript-tsm006`;
- a checked little-endian payload length;
- one canonical payload; and
- a 32-byte BLAKE3 digest derived under `lkjscript.application-artifact.v3`.

The payload contains the root release ID, entry, complete profile, run policy, canonical cases, and
every exact release artifact once in strict `ReleaseId` order. The embedded release bytes remain
independent release authorities. The graph digest uses
`lkjscript.application-release-graph.v1`.

An application artifact is limited to 256 MiB. Its graph is limited to 256 releases, 4,096 edges,
depth 64, and 256 MiB of aggregate release bytes. Cases are limited to 256, names to 64 bytes
matching `[a-z][a-z0-9_]*`, public-value JSON to 1 MiB, and aggregate declared suite fuel to
100,000,000. Runtime values, arguments, frames, fuel, and stream input have independent interpreter
limits.

The decoder checks length before allocation and rejects wrong magic, application format 2 and all
other versions, wrong schema, unknown tags, invalid IDs, duplicate or noncanonical order, an
incomplete or excessive graph, private targets, malformed public values, digest mismatch,
truncation, trailing bytes, and non-byte-identical re-encoding. Corruption rejects before compile or
execution. There is no compatibility reader.

The payload excludes workspace and revision identity, instance identity and state, filesystem
paths, grants, event keys, mutable resolution, proposal text, Core IR, ownership plans, runtime
handles, clocks, provider data, signatures, and attestations.

## Application cases

Cases are immutable exact inputs and expected values or stable traps, not semantic declarations and
not a second assertion language. At least one case targets the entry. A stateful application also
requires a case targeting resume. Only exact value or trap equality passes; skipped, invalid,
incomplete, cancelled, exhausted, and engine-failed cases do not pass.

Public nominal values carry exact release/item identities. Structurally equal types from distinct
releases remain distinct. Test digests retain domain `lkjscript.application-test-case.v2` because
the case representation did not change; they are bounded review keys, not durable identity.
Application tests are pure and cannot execute production host actions.

## Pure invocation

Typed invocation accepts strict version-3 JSON, for example:

```json
{"version":3,"arguments":[{"kind":"bytes","data":"YWJj"}]}
```

Arguments are validated against the exact flattened signature before execution. The response binds
contract version 3, application digest, exact public value, and nonsemantic compile/execute
observations. Invoking stateful entries this way is only pure evaluation; the returned `Decision`
does not become instance authority.

`bytes_stream` reads at most 65,536 uninterpreted standard-input bytes and writes exactly the
result bytes. It grants no filesystem, environment, network, clock, randomness, process, or signal
authority. All profiles use the artifact policy and explicit-frame interpreter.

## Files, publication, and commands

Artifact paths are bounded absolute lexically canonical Unix paths. Dot, parent, empty,
repeated-separator, symlink-parent, symlink-input, and non-regular forms reject. Publication writes
and synchronizes a private same-directory temporary, atomically creates the destination without
replacement, removes the temporary, and synchronizes the directory. Failure before the public link
is known no-change. Failure after the link is `artifact_publication_outcome_unknown`; callers must
reconcile and must not silently repeat publication.

```text
app build --release FILE [--release FILE ...] (--validate-only | --output FILE)
app validate --artifact FILE
app inspect --artifact FILE
app test --artifact FILE
app run --artifact FILE
app stream --artifact FILE
```

Build and typed run consume bounded strict JSON. Validate and inspect never execute program code.
Test executes the embedded pure suite. One application file is sufficient after every source
workspace and standalone release artifact is removed.

Exit classes are `0` success, `2` usage/JSON, `3` filesystem/authority I/O, `4` output failure,
`5` corrupt artifact or unknown publication, `6` application contract/profile rejection, `7` trap
or failed case, and `8` resource exhaustion.

## Explicit absences

Application format 3 contains no dependency store, resolver, registry, native code, bytecode,
serialized Core IR, executable cache, host permission, live resource, opaque continuation,
scheduler, signature, or provenance. Application format 2 (`LKJAPP\0\x02`) and older forms reject
directly.
