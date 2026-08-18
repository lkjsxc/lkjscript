# lkjscript

`lkjscript` is a local typed semantic application platform designed for coding agents. Agents
author immutable workspace revisions, publish workspace-independent releases, build exact
single-file application worlds, and operate durable application instances. Accepted meaning,
mutable state, host intent, host authority, execution, and operational observations remain separate
typed domains.

A stateful application is pure. Its transition returns either:

```text
completed { state, response }
suspended { state, response, command }
```

Commands and outcomes are application-owned nominal sums routed through explicitly imported host
interfaces. Application bytes declare requirements but contain no grants. An instance publishes
state and a pending command before a separately granted adapter acts. The adapter records one typed
known, failed, unknown, or reconciled outcome; only a later pure transition can consume it. Possibly
visible non-idempotent work is never silently retried.

## Complete durable applications

Two public examples exercise independent interfaces and authority shapes:

```sh
examples/durable-controller/run.sh
examples/durable-blob-publisher/run.sh
```

The durable controller validates, activates, and reconciles one exact application in one granted
local slot. The blob publisher creates content-addressed immutable objects in an independently
bounded private namespace and reconciles unknown publication by exact digest. Both applications:

- own their workflow state machine in lkjscript semantics;
- run after source workspaces and standalone releases are deleted;
- survive process restart and deterministic history replay;
- exercise production and disjoint deterministic-fake adapters;
- reject stale bases, duplicate-key conflicts, foreign grants, corrupt retained bytes, and
  identity reuse; and
- expose bounded state/history inspection and explicit unknown visibility.

These grants are narrow trusted local capabilities, not general filesystem APIs. A content digest
is not provenance or authority. The in-process adapters, local OS account, executable, kernel, and
POSIX-like filesystem remain trusted; a process boundary is not a sandbox.

## Public contracts and commands

Reusable release format 1 is `LKJREL\0\x01`. Application-world format/contract 4 is
`LKJAPP\0\x04`; instance format/contract 2 uses `LKJINS\0\x02` records. Runtime session contract
version 1 is line-delimited strict JSON. Superseded application v3 and instance v1 inputs reject
directly; there are no compatibility readers or migrations.

```text
lkjscript release build|validate|inspect|test ...
lkjscript app build|validate|inspect|test|run|stream ...

lkjscript instance create ...
lkjscript instance validate-event|apply-event ...
lkjscript instance execute-host|fake-outcome ...
lkjscript instance validate-resume|resume ...
lkjscript instance inspect|history|delete ...

lkjscript runtime orientation
lkjscript runtime inspect --store DIRECTORY
lkjscript runtime session --store DIRECTORY
```

One-shot application/instance commands and the caller-owned foreground runtime session use the same
topology-neutral runtime kernel. The session amortizes process lifecycle while every request still
names exact authority and retains an independent publication boundary. It keeps bounded stage and
resource counters, but no application/Core cache, queue, worker, scheduler, profile, or hidden
current application/instance. There is no resident daemon or socket supervisor.

## Agent authoring

Normal semantic development uses one `lkjscript` binary and task-scoped bounded context:

```sh
STATE=$(mktemp -d)
chmod 700 "$STATE"

target/release/lkjscript agent orient
target/release/lkjscript agent create --state "$STATE"
target/release/lkjscript agent context \
  --state "$STATE" --workspace WORKSPACE --revision 0 --purpose orient
```

`agent view` and `agent diff` provide deterministic review. `agent document`, `validate`, and
`apply` use one exact-base, schema-bound editable document. `agent run` selects an exact revision
and function. The workspace line session retains one `Engine` for dependent authoring calls without
an implicit current workspace. Release, application, instance, and runtime commands own compact
command-local contracts instead of expanding the global workspace schema.

## Language, execution, and resources

The language supports `unit`, `bool`, checked `i64`, immutable `bytes`, nominal immutable products
and sums, direct calls, conditions, counted loops, exhaustive matching, construction/projection,
typed holes, returns, and yields. Stateful host interfaces required no direct effect primitive,
collection, clock, randomness, thread, or opaque continuation. Expected workflow outcomes are
ordinary nominal data; corruption, traps, resource exhaustion, authority rejection, and unknown
host visibility remain distinguishable.

Only a complete selected closure enters independently verified Core IR. The explicit-frame
interpreter is the sole execution route and correctness oracle. Runtime telemetry separates
application decode, graph validation, flattening, lowering, Core verification, execution, instance
open/replay, publication, grant validation, adapter work, and response encoding. These are bounded
observations, never semantic values.

Semantic fuel, frames, cells, values, state, events, responses, evidence, history, and replay work
have exact owners. Runtime request/response bytes and active transition/adapter/compilation/store
slots have separate deployment admission. Current cache, profile, compiled-unit, and queue budgets
are exactly zero. Logical accounting is not claimed as exact process RSS enforcement.

There is no registry, mutable resolver, network or child-process capability, general filesystem
access, secret store, encryption, multi-user authorization, hostile-host sandbox, worker, daemon,
database, compaction, bytecode, JIT, native application image, or dynamic plugin ABI.

## Verification and documentation

The verified bootstrap is stable Rust edition 2024 on Linux x86-64. The crate contains no local
unsafe Rust.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

- [semantic model](docs/spec/semantic-model.md)
- [language](docs/spec/language.md)
- [reusable releases](docs/spec/reusable-release.md)
- [application worlds](docs/spec/application.md)
- [durable instances and adapters](docs/spec/instance.md)
- [runtime kernel and session](docs/spec/runtime-kernel.md)
- [protocol and editable documents](docs/spec/protocol.md)
- [architecture](docs/architecture.md)
- [implemented status](docs/status.md)
- [measurements and decisions](docs/performance.md)
- [future evidence gates](docs/roadmap.md)
