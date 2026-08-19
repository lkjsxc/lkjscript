# lkjscript

`lkjscript` is a meaning-first semantic software platform. A discoverable semantic project stores
one validated typed meaning graph, immutable development revisions, canonical revision records, and
first-class build targets. The public CLI is the normal authoring, history, inspection, build, test,
run, backup, and recovery interface. No source generator or separate semantic commit step is needed.

The repository also ships `lkjwork`, a complete local durable work ledger whose application meaning
and build definitions live in a checked semantic project. The verified bootstrap target is stable
Rust 2024 on Linux x86-64. There is no network service, daemon, database, package registry, unsafe
Rust, or compatibility layer.

## Develop a semantic project

Create a project, then use it from its root or any descendant directory:

```sh
target/release/lkjscript init ./my-project
cd my-project
../target/release/lkjscript orient
../target/release/lkjscript status
../target/release/lkjscript context --purpose create
../target/release/lkjscript change validate < change.json
../target/release/lkjscript change apply < change.json
../target/release/lkjscript log --limit 10
../target/release/lkjscript diff --from 0 --to 1
```

Every accepted `change apply` publishes exactly one immutable revision and revision record.
Validation, rejection, stale input, and semantic no-change publish nothing. Changes bind an exact
workspace and base revision; selectors are revision-bound and ambiguous names reject. An editable
semantic document can be submitted with `change validate|apply --document`, optionally using aliases
from one exact context capsule. The document is a proposal, never a second source of truth.

Useful project commands include:

```sh
lkjscript orient|status|inspect|context
lkjscript change validate|apply
lkjscript log|show|diff|restore
lkjscript target list|show|build|test|run
lkjscript doctor [--deep]
lkjscript backup DESTINATION
lkjscript session
```

`lkjscript session` is a caller-owned foreground JSON-lines session over the same owners. It reduces
process and engine-open cost, but owns no authority: restart recovers from the project, and aliases
expire when the revision changes. `lkjscript --state … rpc|session` remains a distinct low-level
engine conformance and embedding transport; it is not the project authoring workflow. The former
`agent`, command-local release-build, and command-local application-build paths reject.

## Build and use lkjwork

Build the release binaries and reproduce the checked application from semantic authority:

```sh
cargo build --release --locked
target/release/lkjscript doctor --project applications/lkjwork --deep
target/release/lkjscript target test lkjwork --project applications/lkjwork
target/release/lkjscript target build lkjwork \
  --project applications/lkjwork --output /tmp/lkjwork.lkja
cmp /tmp/lkjwork.lkja applications/lkjwork/lkjwork.lkja
```

No Python process constructs application meaning. The retained Python files are product acceptance
and workload harnesses that exercise public binaries.

Initialize and use a work ledger:

```sh
target/release/lkjwork init ./work --name product-next
target/release/lkjwork --project ./work add "Implement pure query" --priority 20 --label runtime
target/release/lkjwork --project ./work add "Ship product" --depends '#1'
target/release/lkjwork --project ./work next
target/release/lkjwork --project ./work why '#2'
target/release/lkjwork --project ./work context --maximum-tasks 5
```

`why TASK` is an application-owned pure query. It reports exact phase, archive and hold state,
actionability, and deterministically ordered blocker IDs without publishing instance state.
`lkjwork` also supports editing and lifecycle, labels, dependencies, notes, attachments, activity,
history, export, backup/restore, corruption diagnosis, strict JSON, and a bounded foreground session.

The native client discovers and validates artifact-owned interface identities at startup. It owns
argument parsing, typed boundary conversion, narrow immutable-blob routing, terminal-safe rendering,
and deployment lifecycle; it does not reconstruct task policy or carry generated semantic bindings.
See [`applications/lkjwork/README.md`](applications/lkjwork/README.md) for its complete product
contract.

## Authority model

The domains are deliberately separate:

- a semantic project owns development identity, accepted graph meaning, targets, and immutable
  automatic history;
- a release owns one reusable workspace-independent semantic closure;
- an application owns one exact runnable release graph, typed interfaces, cases, and host
  requirements;
- an instance owns durable product-state continuity, mutation/query history, grants, commands,
  attempts, and outcomes; and
- deployment owns paths, processes, local accounts, output locations, and resource placement.

Build-target declarations are accepted graph meaning. Target names are revision-bound lookup
metadata; target edges use exact durable identities. A build selects one exact revision, runs all
accepted cases, and publishes a deterministic no-overwrite artifact. Building never creates a
development revision. Release/application artifacts remain independently validated distribution
authority and do not contain development paths or workspace identity.

The language includes validated UTF-8 text, immutable nominal sequences, products, sums, checked
integers, booleans, bytes, direct calls, and structured control. One explicit-frame interpreter and
independently checked Core IR remain the correctness route. The sole built-in host interface is a
bounded immutable blob namespace with explicit grants and unknown-visibility reconciliation.

Current format identities, rejected predecessors, implemented limits, and exact absences are in
[`docs/status.md`](docs/status.md). Normative contracts live under [`docs/spec/`](docs/spec/).

## Reproduce product evidence

```sh
python3 applications/lkjwork/acceptance.py --binary target/release/lkjwork
python3 applications/lkjwork/workload.py target/release/lkjwork --profile functional
python3 applications/lkjwork/workload.py target/release/lkjwork --profile representative
```

The representative retained corpus has 500 tasks, 2,500 core mutation requests, 1,000 dependency
edges, 1,000 notes, 100 attachments, and 2,000 queries. Semantic-project migration, automatic
history, storage, authoring-economy, and TUI-readiness evidence is under `docs/evidence/` and
summarized in [`docs/performance.md`](docs/performance.md).

## Verify

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

The trust model is one local operator and OS account. Native code and the narrow blob adapter are
trusted. Graphs, JSON, documents, paths, locators, records, artifacts, manifests, outcomes, backups,
and blobs are hostile input and fail closed on malformed, foreign, excessive, symlinked,
noncanonical, or digest-mismatched forms.

The project does not claim a hostile-native-code sandbox, hostile-administrator isolation,
multi-user authorization, encryption, authenticity, provenance, power-loss proof, exact RSS
enforcement, provider-token savings, monetary savings, or cross-platform support beyond exercised
Linux x86-64 workflows.
