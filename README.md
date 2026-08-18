# lkjscript

`lkjscript` is a local typed semantic programming system designed for coding agents. Agents author
immutable workspace revisions, publish workspace-independent reusable releases, build single-file
applications, and operate exact durable application instances. Accepted meaning, durable state,
host intent, and host authority remain separate typed domains.

The retained stateful model is deliberately small: a pure application transition returns the next
typed state, a bounded response, and either completion or one typed command. The instance store
publishes that state atomically. A separately granted host executor records success, known failure,
or unknown visibility; only another pure transition can consume the result. Possibly visible work is
never retried automatically.

## Durable controller proof

The public `durable-controller` example authors a release controller through workspace RPC, builds
release and application artifacts, deletes the source workspace and standalone release, and then
operates two isolated instances after repeated process restart.

The primary instance validates and atomically activates one exact application in one granted local
slot using the production executor. The second is bound to the deterministic fake executor and
proves denied cross-executor authority, unknown activation outcome, explicit reconciliation,
known failure, retry, cancellation, duplicate delivery, stale-base rejection, bounded history,
corruption rejection, tombstoned identity, and no reuse.

```sh
cargo build --release --locked
examples/durable-controller/run.sh
```

The slot capability is not a general filesystem API or sandbox. It accepts one explicit regular
application file below one granted source directory and can replace only one exact activation slot.
The local OS account, executable, kernel, and POSIX-like filesystem remain trusted.

## Public artifacts and commands

Reusable release format 1 is `LKJREL\0\x01`. Application format 3 is
`LKJAPP\0\x03`; it embeds the complete exact release graph and may declare `typed`,
`bytes_stream`, or `stateful` invocation. Both remain usable after all source workspaces are
removed. Application format 2 and older forms reject directly.

```text
lkjscript release build|validate|inspect|test ...
lkjscript app build|validate|inspect|test|run|stream ...

lkjscript instance create ...
lkjscript instance validate-event|apply-event ...
lkjscript instance validate-application|execute-activation|reconcile-activation ...
lkjscript instance fake-outcome ...
lkjscript instance validate-resume|resume ...
lkjscript instance inspect|history|delete ...
```

Instance contract and durable format version 1 use full canonical typed state records, a validated
HEAD, exact event-key receipts, and deterministic replay. State, event, history, response, command,
evidence, retry, and replay work have independent bounds. An instance embeds its exact application,
retains every revision and unresolved outcome, and never reuses a deleted identity.

## Agent authoring

Normal development uses the one `lkjscript` binary under an exclusive local state lock. No daemon
or background service is installed.

```sh
STATE=$(mktemp -d)
chmod 700 "$STATE"

target/release/lkjscript agent orient
target/release/lkjscript agent create --state "$STATE"
target/release/lkjscript agent context \
  --state "$STATE" --workspace WORKSPACE --revision 0 --purpose orient
```

`agent view` and `agent diff` provide bounded deterministic review. `agent document`, `validate`,
and `apply` use one exact-base, schema-bound editable document. `agent run` selects an exact revision
and function. A line-delimited `session` amortizes Engine startup without introducing an implicit
current workspace, application, or instance. Release, application, and instance commands own small
command-local contracts rather than expanding the global workspace schema.

## Semantic and runtime model

The language supports `unit`, `bool`, checked `i64`, immutable `bytes`, nominal immutable products
and sums, calls, conditions, counted loops, exhaustive matching, construction/projection, typed
holes, returns, and yields. Stateful operation added no direct host effect, capability value,
collection, clock, randomness, thread, or opaque continuation. Expected workflow outcomes are
ordinary nominal data; corruption, traps, resource exhaustion, and authority rejection remain
distinct.

Only a complete selected closure enters independently verified Core IR. The explicit-frame
interpreter is the correctness oracle. Managed immutable bytes retain an allocate-new differential
oracle. Full workspace snapshots, full instance state snapshots, deterministic scans, and embedded
application release graphs remain because current workloads do not cross their replacement gates.

There is no registry, online resolver, mutable dependency store, network or process capability,
general filesystem access, daemon, database, scheduler, secret store, encryption, bytecode, JIT,
native application artifact, or hostile-host isolation.

## Verification

The supported bootstrap is stable Rust edition 2024 on Linux x86-64. The crate contains no local
unsafe Rust.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Retained public workflows are under `examples/`; `durable-controller`, `reusable-release`, and
`binary-canonicalizer` are the principal complete workloads. This repository does not claim formal
verification, cross-platform operation, crash safety beyond its documented local-filesystem model,
or a production multi-user deployment system.

## Current documentation

- [typed semantic program and identity model](docs/spec/semantic-model.md)
- [language](docs/spec/language.md)
- [reusable semantic releases](docs/spec/reusable-release.md)
- [application artifacts](docs/spec/application.md)
- [durable instances and activation](docs/spec/instance.md)
- [protocol and editable documents](docs/spec/protocol.md)
- [architecture](docs/architecture.md)
- [implemented status](docs/status.md)
- [measurements and decisions](docs/performance.md)
- [future evidence gates](docs/roadmap.md)
