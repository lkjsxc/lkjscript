# lkjscript

`lkjscript` is a typed semantic programming system designed for coding agents. An agent observes a
bounded slice of an immutable program revision, proposes an exact semantic document, and lets a
deterministic Rust engine validate, publish, compile, and run it.

Program meaning has one authoritative typed representation. Text documents, JSON, context packets,
reviews, Core IR, ownership plans, and executable state are proposals or derived views; none can
bypass the semantic validator.

## Primary workflow

The normal path is one `lkjscript` binary opening a local workspace directly under an exclusive
authority lock. No background service needs to be started.

```sh
cargo build --release --locked

STATE=$(mktemp -d)
chmod 700 "$STATE"

target/release/lkjscript agent orient
target/release/lkjscript agent create --state "$STATE"
target/release/lkjscript agent context \
  --state "$STATE" --workspace WORKSPACE --revision 0 --purpose orient
```

The remaining agent commands are:

- `agent view` and `agent diff` for deterministic human-readable review;
- `agent document` to render one packet-bound editable function document;
- `agent validate` and `agent apply` for the same bounded editable document;
- `agent run` for an exact revision and entry function.

`agent orient` embeds the active machine-contract digest. Normal work therefore needs no explicit
global schema request. `agent context --known-digest DIGEST` returns a 107-byte unchanged response
when the exact capsule is reusable. Full schema projection and strict protocol-v9 JSON remain
diagnostic surfaces.

An optional `lkjscriptd` Unix-socket adapter exercises the same `Engine`; it is not the primary
workflow or a separate semantic implementation. `lkjscript --state DIR session` keeps one direct
engine open for line-delimited JSON requests.

## Semantic model

The authoritative model uses two identity strata:

- durable entity IDs for the workspace root, named packages/modules/declarations/members/functions,
  parameters, and explicit repairable hole anchors;
- revision-bound function-local references for regions, blocks, block arguments, ordinary
  operations, implied returns/yields, and other body scaffolding.

A body replacement preserves its function entity while rebuilding local terms. Local references
never consume the durable allocator or tombstones. In the retained job-policy workload, 189 semantic
items require 48 durable identities. Thirty-two repeated body replacements allocate no durable ID,
create no tombstone, and each encode to the same 443-byte snapshot size.

The current language supports `unit`, `bool`, checked `i64`, immutable `bytes`, named immutable
records, fixed variants, calls, conditionals, counted loops, exhaustive lazy matching, exact
construction/projection, and typed placeholders. Compilation lowers only the selected entry's
complete dependency closure to verified Core IR and runs one explicit-frame interpreter.

## Documents and revisions

Editable semantic document version 1 declares its exact workspace, base revision, schema digest,
editable scope, edits, and optional packet digest. The parser is strict, bounded, location-aware, and
iterative for user-scalable nesting. Parsed syntax is discarded after normalization into the same
typed transaction used by JSON.

Published revisions are immutable full canonical snapshots under artifact format 6,
`lkjscript-tsm006`, `LKJTSM\0\x06`, and `LKJHEAD8`. Rejection and validate-only publish nothing and
consume no durable identity. Old artifact, HEAD, protocol, document, and context forms reject; there
is no compatibility reader.

Full snapshots remain deliberate: after identity stratification, the eight-revision maintenance
corpus grows from 8,354 to 9,457 bytes per revision. An incremental object store would currently add
publication, corruption, retention, and garbage-collection surface without a measured payoff.

Workspace snapshots are development authority, not publishable package artifacts. Package graphs,
dependencies, host effects, resource-owning values, remote operation, sandboxing, and native code
generation are not implemented.

## Build and verification

The supported bootstrap is stable Rust edition 2024 on Linux x86-64.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run the retained public-path applications with:

```sh
for example in \
  job-policy named-data release-channel release-manifest \
  binary-canonicalizer agent-maintenance
do
  "examples/$example/run.sh"
done
```

The crate forbids local unsafe Rust. This is not a formal proof, sandbox, or production-readiness
claim. Inputs and work are bounded, but the trusted computing base still includes Rust, Cargo,
resolved dependencies, the operating system, and the filesystem.

## Current documentation

- [semantic model](docs/spec/semantic-model.md)
- [language](docs/spec/language.md)
- [protocol and documents](docs/spec/protocol.md)
- [architecture](docs/architecture.md)
- [implemented status](docs/status.md)
- [current measurements and decisions](docs/performance.md)
- [future evidence gates](docs/roadmap.md)
