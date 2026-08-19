# lkjscript

`lkjscript` is an agent-native typed semantic application platform. It stores accepted program
meaning as exact immutable revisions, projects reusable releases, composes validated application
worlds, and runs durable stateful instances with explicit host authority. The repository also ships
`lkjwork`, a complete local durable work ledger for humans and coding agents.

The verified bootstrap target is stable Rust 2024 on Linux x86-64. There is no network service,
daemon, database, package registry, unsafe Rust, or backward-compatibility layer.

## Use lkjwork

Build the release binaries and initialize a private project:

```sh
cargo build --release --locked
target/release/lkjwork init ./work --name product-next
target/release/lkjwork --project ./work add "Implement pure query" --priority 20 --label runtime
target/release/lkjwork --project ./work add "Ship product" --depends '#1'
target/release/lkjwork --project ./work next
target/release/lkjwork --project ./work context --maximum-tasks 5
```

`lkjwork` supports task editing and lifecycle, holds, priorities, labels, exact DAG dependencies,
append-only notes, immutable evidence attachments, activity/history, pure queries, deterministic
agent context, semantic export, exact backup/new-instance restore, corruption diagnosis, strict JSON,
and a bounded foreground session.

All task policy lives in the embedded lkjscript application. The native client owns only command
parsing, strict project discovery, bounded explicit attachment reads, host routing, terminal-safe
rendering, backup transport, and process lifecycle. It has no hidden JSON/SQLite/filesystem task
database and does not compute readiness or next work.

See [`applications/lkjwork/README.md`](applications/lkjwork/README.md) for the complete product
contract, workflows, recovery model, machine session, and reproducibility commands. Run
`target/release/lkjwork --help` for the command grammar.

## Platform model

The active semantic vocabulary includes validated UTF-8 text, nominal immutable homogeneous
sequences, products, sums, checked integers, booleans, bytes, direct calls, structured control, and
deterministic resource accounting. One explicit-frame interpreter and independently verified Core IR
remain the correctness route.

The artifact domains are deliberately separate:

- a workspace owns development identity and immutable revision history;
- a release owns one exact reusable workspace-independent semantic closure;
- an application owns one exact runnable release graph, typed entries/results/cases, and host
  requirements;
- an instance owns durable state continuity, mutation/query history, checkpoints, grants, commands,
  attempts, and outcomes; and
- a deployment owns paths, processes, local accounts, and resource placement.

Stateful applications return application-owned typed responses and distinguish declined, unchanged,
completed, and suspended decisions. Declined/unchanged publish no revision. Pure application queries
publish nothing. Instances retain a hash-linked journal with periodic full checkpoints and a
HEAD-bound current manifest; ordinary operations avoid full replay while `doctor --deep` remains the
genesis reconstruction oracle.

The sole built-in host interface is a bounded immutable blob namespace. A visibility-capable put
records an attempt first; possible visibility is reconciled and never silently retried. Applications
declare requirements, instances bind exact grants, and adapters cannot invent semantic state.

Current format and protocol identities, direct rejected predecessors, implemented limits, and exact
absences are maintained in [`docs/status.md`](docs/status.md). Normative contracts live under
[`docs/spec/`](docs/spec/).

## Platform CLI orientation

Create and inspect a semantic workspace:

```sh
target/release/lkjscript --state /absolute/state create
target/release/lkjscript --state /absolute/state inspect
target/release/lkjscript --state /absolute/state agent orient
```

The workspace protocol is strict versioned JSON. Agent context packets and editable semantic
documents are bounded proposals/views that normalize through the same typed transaction validator.
Reusable release, application, instance, and runtime commands use their own command-local closed
contracts so application/product schemas do not inflate the global workspace catalogue.

Use command help for exact syntax:

```sh
target/release/lkjscript help
target/release/lkjscript release help
target/release/lkjscript app help
target/release/lkjscript instance help
target/release/lkjscript runtime help
```

## Reproduce product evidence

The installed `lkjwork` binary embeds and independently validates one exact checked-in application
artifact. Reproduce the artifact through public platform commands and run the complete product story:

```sh
python3 applications/lkjwork/build.py target/release/lkjscript
python3 applications/lkjwork/acceptance.py --binary target/release/lkjwork
python3 applications/lkjwork/workload.py target/release/lkjwork --profile functional
python3 applications/lkjwork/workload.py target/release/lkjwork --profile representative
```

The representative retained corpus has 500 tasks, 2,500 core mutation requests, 1,000 dependency
edges, 1,000 notes, 100 attachments, and 2,000 queries. Exact measurements and selected/rejected
alternatives are in [`docs/performance.md`](docs/performance.md) and `docs/evidence/`.

## Develop and verify

The full repository gate is:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Read the root `AGENTS.md` before changing the repository. Specifications own accepted contracts;
status owns implemented reality; architecture owns components/trust; performance and structured
evidence own measurements; roadmap contains only evidence-gated reversal conditions.

## Trust and nonclaims

The bootstrap deployment assumes one trusted local operator and OS account. Native code and the
narrow immutable-blob adapter are trusted. Artifacts, text, JSON, paths, locators, records,
checkpoints, manifests, outcomes, backups, and blobs are hostile inputs and fail closed on malformed,
foreign, excessive, symlinked, noncanonical, or digest-mismatched forms.

The project does not claim a hostile-native-code sandbox, hostile-administrator isolation,
multi-user authorization, encryption, authenticity, provenance, power-loss proof, exact RSS
enforcement, provider-token savings, monetary savings, or cross-platform support beyond exercised
Linux x86-64 workflows.
