# lkjscript

`lkjscript` lets a coding agent author a typed program, publish immutable revisions, attach exact
release cases, seal one dependency-closed application artifact, inspect it, and run it after the
development workspace is gone.

Program meaning has one authoritative typed representation. Editable documents and JSON are
untrusted proposals; context and review text are bounded views; Core IR, ownership plans, and
runtime values are derived. Deterministic Rust validation decides acceptance.

## A complete application lifecycle

The retained `binary-canonicalizer` application accepts arbitrary bytes on standard input, removes
its marker and zero padding, and writes canonical bytes on standard output. Its production driver
creates and repairs the program through public commands, checks historical revisions, validates a
three-case release suite, builds twice, compares canonical artifact bytes, deletes the workspace,
then validates, inspects, tests, invokes, and corrupts the transferred artifact.

```sh
cargo build --release --locked
./examples/binary-canonicalizer/run.sh
```

The standalone command family is:

```text
lkjscript app build --state DIR --validate-only
lkjscript app build --state DIR --output /absolute/path/application.lkja
lkjscript app validate --artifact /absolute/path/application.lkja
lkjscript app inspect --artifact /absolute/path/application.lkja
lkjscript app test --artifact /absolute/path/application.lkja
lkjscript app run --artifact /absolute/path/application.lkja
lkjscript app stream --artifact /absolute/path/application.lkja
```

Build reads a strict version-1 JSON request naming the exact workspace, revision, entry, invocation
profile, Run policy, and immutable invocation cases. Typed run reads a strict version-1 invocation.
The `bytes_stream` profile reads up to 65,536 uninterpreted bytes from standard input and writes only
the exact returned bytes to standard output. It grants no filesystem, environment, network, clock,
randomness, or process authority.

Application artifact version 1 (`LKJAPP\0\x01`, semantic schema `lkjscript-tsm006`) contains only
the selected entry/test dependency closure, exact profile and policy, and release cases. It excludes
workspace history, HEAD, idempotency records, aliases, caches, paths, unrelated declarations, and
executable IR. Every load independently validates canonical bytes and semantics before compile or
run. A content digest proves integrity and exact reuse; it is not application identity, provenance,
authorization, or a signature.

An application artifact is run-only target-neutral semantic content. It is not a reusable package,
native executable, sandbox, or production deployment unit.

## Agent authoring workflow

The normal development path is one `lkjscript` binary opening a local workspace under an exclusive
authority lock. No background service is required.

```sh
STATE=$(mktemp -d)
chmod 700 "$STATE"

target/release/lkjscript agent orient
target/release/lkjscript agent create --state "$STATE"
target/release/lkjscript agent context \
  --state "$STATE" --workspace WORKSPACE --revision 0 --purpose orient
```

The remaining authoring commands are:

- `agent view` and `agent diff` for deterministic bounded review;
- `agent document` for one packet-bound editable function document;
- `agent validate` and `agent apply` for the same exact-base document;
- `agent run` for an exact workspace revision and entry.

Editable semantic document version 1 declares its workspace, base revision, machine-schema digest,
scope, and optional packet digest. Parsing is strict, bounded, location-aware, and iterative; syntax
is discarded after normalization into the same typed transaction used by diagnostic JSON.

`agent orient` embeds the active machine-contract digest. Normal authoring needs no global schema
dump. `agent context --known-digest DIGEST` returns a 107-byte unchanged response only after
rebuilding and matching the exact packet. Full schema projection and strict protocol-v10 JSON remain
diagnostic surfaces.

`lkjscript --state DIR session` amortizes engine startup for line-delimited diagnostic RPC without
combining publication boundaries. The optional `lkjscriptd` Unix-socket adapter remains for its
framing, correlation, disconnect, deadline, and lock-integration diagnostics; it calls the same
Engine and is not the primary workflow or a second semantic implementation.

## Semantic and runtime model

The authoritative program uses two identity strata:

- workspace-qualified durable IDs for continuity-bearing packages, modules, declarations, members,
  functions, parameters, and explicit repairable holes;
- revision-bound function-local IDs for regions, blocks, binders, ordinary operations, and implied
  control scaffolding.

A body replacement preserves its function while rebuilding local terms, allocating no durable ID
or tombstone. Application artifacts retain exact source identity so nominal values remain valid
after transfer; they do not promise import or cross-artifact continuity.

The language currently supports `unit`, `bool`, checked `i64`, immutable `bytes`, named immutable
records, fixed variants, calls, conditions, counted loops, exhaustive lazy matching, exact
construction/projection, and typed holes. Only a complete selected-entry closure enters verified
Core IR. One explicit-frame interpreter is the executable oracle.

The current managed-byte planner remains an implementation optimization, not language ownership. On
the retained 512-octet loop-carried append control it reduces copied backing bytes from 131,840 to
1,024 and peak backing from 1,024 to 513 bytes while the allocate-new route provides a differential
oracle. Logical fuel and memory limits do not depend on reuse.

Workspace revisions remain full canonical snapshots under artifact format 6 and `LKJHEAD8`.
Application artifacts use a separate format and publication path. Reusable packages, dependency
resolution, host effects, external resources, branches, bytecode, native code, and cross-platform
execution evidence are absent.

## Verification

The supported bootstrap is stable Rust edition 2024 on Linux x86-64.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run all retained public-path applications with:

```sh
for example in \
  job-policy named-data release-channel release-manifest \
  binary-canonicalizer agent-maintenance
do
  "examples/$example/run.sh"
done
```

The crate forbids local unsafe Rust. This is not a formal proof, sandbox, portability, or
production-readiness claim. The trusted computing base still includes Rust, Cargo, resolved
dependencies, the operating system, filesystem, and CPU.

## Current documentation

- [typed semantic program model](docs/spec/semantic-model.md)
- [language](docs/spec/language.md)
- [application artifact and release tests](docs/spec/application.md)
- [protocol and editable documents](docs/spec/protocol.md)
- [architecture](docs/architecture.md)
- [implemented status](docs/status.md)
- [current measurements and decisions](docs/performance.md)
- [future evidence gates](docs/roadmap.md)
