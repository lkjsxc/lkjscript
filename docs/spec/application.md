# Application artifact, release-test, and invocation contracts

This specification owns the standalone application domain: exact closure selection, immutable
release cases, canonical artifact bytes, inspection, publication, and pure invocation. Workspace
semantics remain owned by the typed semantic program model. An application artifact is neither a
workspace revision, a reusable package, nor an executable cache.

## Authority and lifecycle

One accepted workspace `Snapshot` remains semantic authority. Application build version 1 names an
exact workspace, revision, entry, invocation profile, Run policy, and nonempty release-test set.
The builder:

1. opens the named immutable revision without consulting implicit HEAD;
2. validates and canonically orders the release cases;
3. computes the union of the entry and test dependency closures;
4. rejects incomplete, foreign, profile-incompatible, or unrelated content;
5. constructs a run-only semantic snapshot projection;
6. encodes and independently decodes that projection;
7. compiles the entry to independently verified Core IR;
8. runs every release case under its exact policy;
9. constructs the bounded receipt; and
10. either returns validate-only results or publishes one complete artifact.

A failed, skipped, invalid, incomplete, resource-exhausted, or engine-failed case blocks build
publication. Application build and test never publish a workspace revision or consume durable
semantic identity.

The application CLI JSON contract version is 1. Build requests and typed invocations carry
`version: 1`; every JSON success or error result reports `contract_version: 1`. Unknown or older
versions reject with `protocol_version`. Raw stream input has no JSON envelope; its contract is
bound by the validated artifact format and `bytes_stream` profile.

## Exact semantic closure

Closure roots are the selected entry plus every release-case target. Closure contains:

- each complete selected function, its parameters, body, and function-local structure;
- every transitively called function;
- every nominal declaration and member referenced by signatures, bodies, arguments, or expected
  values; and
- the exact workspace/package/module owner chain needed to validate containment.

The artifact contains no unrelated workspace node. Package entry fields are cleared because the
application manifest owns the sole selected entry. Reachable holes, missing bodies, dangling
references, incomplete tests, and incompatible entry signatures reject. The same closure walk owns
build, decode validation, inspection counts, test compilation, and application compilation.

## Identity domain

Version 1 retains the source workspace-qualified durable and function-local IDs. This preserves
nominal equality and exact diagnostic targets without an inferred name or structural remap. The
artifact also records the source revision because function-local IDs are revision-bound.

This is a run-only identity contract:

- it does not make workspace IDs package coordinates or application release identity;
- it does not promise import, vendoring, fork, merge, or cross-artifact continuity;
- equal program behavior under different workspace IDs may produce distinct canonical content;
- names and filenames remain presentation metadata; and
- the application digest is an integrity/content key, not entity identity, authorship,
  authorization, provenance, or a signature.

Sparse durable serial gaps are not serialized as workspace history. The decoder deterministically
constructs bounded validator scaffolding for absent lower serials; it has no import or continuity
meaning. The highest durable serial is limited to 262,144.

## Canonical application artifact version 1

The envelope is:

- magic `LKJAPP\0\x01`;
- little-endian application format `1`;
- exact semantic schema `lkjscript-tsm006`;
- checked little-endian payload length;
- one canonical payload; and
- a 32-byte BLAKE3 digest derived under `lkjscript.application-artifact.v1`.

The payload contains, in order, source workspace and revision, exact entry, invocation profile,
entry Run policy, canonical release cases, and semantic nodes in strict identity order. Each case
contains its canonical name, exact target, policy, ordered typed arguments, and exactly one expected
value or stable typed trap. Runtime values use strict canonical compact JSON inside a bounded binary
field; reserialization must reproduce the exact stored bytes. Semantic nodes reuse the accepted
semantic schema's canonical node codec, not the workspace history envelope.

The artifact is limited to 64 MiB, 100,000 semantic nodes, 256 tests, 64 bytes per ASCII test name,
and a highest durable serial of 262,144. Test names match `[a-z][a-z0-9_]*`, are unique, and are
stored in strict lexical order. Runtime values retain their existing depth, item, and byte limits.
The sum of declared per-case fuel is at most 100,000,000.

The decoder checks all lengths and counts before corresponding large allocation, rejects unknown
magic/version/schema/profile/tags, duplicate or noncanonical order, invalid IDs, wrong ownership,
foreign references, malformed values, incomplete closure, digest mismatch, truncation, and trailing
bytes. It reconstructs a validated `Snapshot`, recomputes exact closure, and re-encodes the complete
application; unequal bytes reject. Compilation and execution always start from that independently
validated state. Core IR remains derived and is independently verified on every run.

The payload excludes workspace history, HEAD, allocator frontier as authority, idempotency records,
context aliases or packet digests, proposal text, caches, absolute paths, temporary paths,
compiler-private IDs, ownership plans, runtime handles, timestamps, randomness, and provider data.

## Release-test contract

Tests are immutable application-local invocation cases, not durable semantic entities and not a
second executable language. Their lexical name is the selection and review key within one artifact;
the enclosing application digest binds the full set. At least one case must target the application
entry. Version 1 always builds and runs the complete set and has no tag, glob, skip, or subset mode.

Expected success uses exact typed public-value equality. Expected failure is limited to stable
runtime trap codes `runtime_trap`, `byte_index_out_of_bounds`, and `byte_slice_out_of_bounds`, with
an optional exact semantic target. Allocation counts, handles, Core IDs, diagnostic prose, and wall
time are not test semantics.

Each case also has a derived 32-byte digest under `lkjscript.application-test-case.v1`; it binds the
workspace domain, case name, target, policy, arguments, and expected outcome without exposing or
truncating a large value in normal inspection. It is a review/content key, not test identity.

Test reports use the closed statuses `passed`, `unexpected_value`, `unexpected_trap`, `missing_trap`,
`wrong_trap`, `invalid_case`, `incomplete`, `resource_failure`, and `engine_failure`. Only exact
outcome equality produces `passed`. Reports are bounded, ordered exactly like the cases, and include
an observed stable trap when one exists. Test execution cannot mutate workspace authority and one
trap cannot poison later cases or a reusable Engine.

Inspection derives from validated artifact facts. It exposes the artifact and semantic digests,
source domain, entry, profile, policy, exact artifact/path/runtime limits, node/identity counts, and
each test's case digest, target, argument/result types, expectation kind, optional expected trap, and
policy. A build receipt combines that inspection with the freshly executed complete test report.
Inspection alone never claims a current pass result.

## Invocation profiles

`typed` accepts one version-1 `ApplicationInvocation` containing the exact ordered public values.
Every value is validated against the artifact's retained identity and entry signature before
flattening. Successful output is a version-1 receipt containing the application digest and the
normal bounded Run result.

`bytes_stream` is valid only when the entry accepts exactly one `bytes` value and returns `bytes`.
Standard input is read as uninterpreted bytes up to the semantic 65,536-byte value limit. There is
no implicit newline, UTF-8, locale, terminal, environment, current directory, filesystem, network,
clock, randomness, process, or signal input. Success writes the exact returned bytes to standard
output; terminal-safe JSON diagnostics use standard error. Equal artifact bytes, input bytes,
runtime contract, and policy produce equal semantic output or trap.

Both profiles use the exact policy stored in the artifact. They compile only the complete selected
entry closure and run the explicit-frame interpreter. They publish no state. The process adapter is
a pure invocation boundary, not a host-effect system or sandbox.

## Filesystem publication

Artifact input and output paths are explicit absolute canonical Unix paths of at most 4,096 bytes.
Empty, dot, parent, repeated-separator, and trailing-separator components reject from their original
byte spelling. Every existing parent and input component must be a non-symlink directory or regular
file as applicable. Output never derives from semantic names and never overwrites an existing
destination.

The CLI encodes and bounds the exact success receipt before publication. Only the opaque
`PreparedApplication` returned after independent decode, compile, and release-test success can call
the publisher. Publication creates a private mode-0600 temporary file in the destination directory,
writes and synchronizes the complete canonical bytes, establishes the destination through one
atomic no-replace hard link, removes the temporary name, opens the parent directory, and
synchronizes it. A failure before the destination link leaves no destination. A failure after the
link reports `artifact_publication_outcome_unknown`; the exact destination may already exist and no
retry or overwrite is attempted. All modeled edges have deterministic failure-injection coverage.

The filesystem and concurrent directory administration remain part of the trusted local host; this
contract is not a hostile-kernel or sandbox claim.

## Public commands and exits

The standalone lifecycle is:

```text
app build --state DIR (--validate-only | --output FILE)
app validate --artifact FILE
app inspect --artifact FILE
app test --artifact FILE
app run --artifact FILE
app stream --artifact FILE
```

Build and typed run consume strict bounded JSON from standard input. Validate and inspect decode
without compiling or running. Test runs all cases. Artifact commands never consult source state or
implicit HEAD.

Process exit classes are: `0` success, `2` command usage, `3` authority/transport/filesystem I/O,
`4` output failure, `5` artifact corruption or publication uncertainty, `6` application request or
profile rejection, `7` program trap or failed test verdict, and `8` resource exhaustion. Program
traps never masquerade as artifact corruption. Machine output is bounded to 32 MiB; build and typed
input are bounded to 8 MiB. Stream output remains limited by the semantic byte-value policy.

## Explicit absences

Application artifact version 1 is target-neutral semantic content. It carries neither serialized
Core IR nor bytecode, native code, compiler cache, signature, attestation, package dependency graph,
multiple exports, user release version, package coordinate, host permission, or external resource.
Reusable package distribution and executable caches require distinct future contracts and cannot
reinterpret this run-only format.
