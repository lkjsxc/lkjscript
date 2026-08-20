# lkjscript

`lkjscript` is a meaning-first semantic software platform. A discoverable semantic project stores
one validated typed meaning graph, immutable development revisions, canonical revision records, and
first-class build targets. The public CLI is the normal authoring, history, inspection, build, test,
run, backup, and recovery interface. No source generator or separate semantic commit step is needed.

The repository also ships `lkjwork`, a complete local durable work ledger, and `lkjedit`, a
mouse-capable Vim-like tiled terminal editor with ordinary semantic-project tabs. Both applications
and their build definitions live in checked semantic projects. The verified bootstrap target is
stable Rust 2024 on Linux x86-64. There is no network service, daemon, database, package registry,
unsafe first-party Rust, or compatibility layer.

## Develop a semantic project

Create a project, then use it from its root or any descendant directory:

```sh
target/release/lkjscript init ./my-project
cd my-project
../target/release/lkjscript orient
../target/release/lkjscript status
../target/release/lkjscript query targets
../target/release/lkjscript context --purpose create
../target/release/lkjscript change validate < change.json
../target/release/lkjscript change apply < change.json
../target/release/lkjscript log --limit 10
../target/release/lkjscript diff --from 0 --to 1
```

Every accepted `change apply` publishes exactly one immutable revision and revision record and
returns a bounded exact continuation for the next query or affected-target action.
Validation, rejection, stale input, and semantic no-change publish nothing. Changes bind an exact
workspace and base revision; selectors are revision-bound and ambiguous names reject. An editable
semantic document can be submitted with `change validate|apply --document`, optionally using aliases
from one exact context capsule. The document is a proposal, never a second source of truth.

Useful project commands include:

```sh
lkjscript orient|status|inspect|query|proposal|context
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

## Build and use lkjedit

Reproduce the editor package from its maintained semantic authority:

```sh
target/release/lkjscript doctor --project applications/lkjedit --deep
target/release/lkjscript target test lkjedit --project applications/lkjedit
target/release/lkjscript target build lkjedit-application \
  --project applications/lkjedit --output /tmp/lkjedit.lkja
cmp /tmp/lkjedit.lkja applications/lkjedit/lkjedit.lkja
```

Ordinary launch uses the validated checked package without exposing an artifact path:

```sh
target/release/lkjedit .
target/release/lkjedit --root . README.md
target/release/lkjedit --project applications/lkjedit .
```

The application owns Normal, Insert, Visual, command-line, and search modes; buffers and independent
views; ordinary heterogeneous tabs; a normalized integer-weight split tree; keyboard and SGR mouse
layout operations; explorer and bounded recursive search; expected-base save, conflict, unknown
visibility, and reconciliation; and logical styled frames. One capacity-one native worker executes
typed host work while local input continues. Semantic orientation, proposals, validation/apply,
history/diff, and target work appear in the same tab model. See
[`applications/lkjedit/README.md`](applications/lkjedit/README.md) for exact commands, Unicode and
line-ending semantics, bounds, trust, replay, and acceptance details.

## Authority model

The domains are deliberately separate:

- a semantic project owns development identity, accepted graph meaning, targets, and immutable
  automatic history;
- a release owns one reusable workspace-independent semantic closure;
- an application owns one exact runnable release graph, typed or interactive role mappings, cases,
  and host requirements;
- an instance owns durable product-state continuity, mutation/query history, grants, commands,
  attempts, and outcomes; and
- deployment owns paths, processes, local accounts, output locations, and resource placement.

Build-target declarations are accepted graph meaning. Target names are revision-bound lookup
metadata; target edges use exact durable identities. A build selects one exact revision, runs all
accepted cases, and publishes a deterministic no-overwrite artifact. Building never creates a
development revision. Release/application artifacts remain independently validated distribution
authority and do not contain development paths or workspace identity.

The language includes validated UTF-8 text, immutable nominal sequences, products, sums, checked
integers, booleans, bytes, direct calls, and structured control. Text execution uses an unobservable
persistent UTF-8 piece tree with a canonical flat oracle and explicit byte, scalar, grapheme, line,
splice, and literal-search operations. One explicit-frame interpreter and independently checked
Core IR remain the correctness route. Interactive applications use explicit semantic-project and
selected-filesystem grants with separate publication and unknown-visibility reconciliation.

Current format identities, rejected predecessors, implemented limits, and exact absences are in
[`docs/status.md`](docs/status.md). Normative contracts live under [`docs/spec/`](docs/spec/).

## Reproduce product evidence

```sh
python3 applications/lkjwork/acceptance.py --binary target/release/lkjwork
python3 applications/lkjwork/workload.py target/release/lkjwork --profile functional
python3 applications/lkjwork/workload.py target/release/lkjwork --profile representative
python3 applications/lkjedit/acceptance.py --binary target/release/lkjedit
python3 applications/lkjedit/workload.py --binary target/release/lkjedit \
  --project-cli target/release/lkjscript --samples 5
```

The representative retained corpus has 500 tasks, 2,500 core mutation requests, 1,000 dependency
edges, 1,000 notes, 100 attachments, and 2,000 queries. Semantic-project migration, automatic
history, storage, authoring-economy, and TUI-readiness evidence is under `docs/evidence/` and
summarized in [`docs/performance.md`](docs/performance.md).

## Verify

Use `tools/check quick`, `tools/check product`, or `tools/check full`. The wrapper runs locked,
offline gates, retains complete bounded logs under `.artifacts/check/`, and prints aggregate success
or bounded failure evidence. `tools/check full` includes formatting, clippy, all-target tests,
optimized build, deep project doctors, product target tests, artifact reproduction, and acceptance.

The trust model is one local operator and OS account. Native code and the narrow blob adapter are
trusted. Graphs, JSON, documents, paths, locators, records, artifacts, manifests, outcomes, backups,
and blobs are hostile input and fail closed on malformed, foreign, excessive, symlinked,
noncanonical, or digest-mismatched forms.

The project does not claim a hostile-native-code sandbox, hostile-administrator isolation,
multi-user authorization, encryption, authenticity, provenance, power-loss proof, exact RSS
enforcement, provider-token savings, monetary savings, or cross-platform support beyond exercised
Linux x86-64 workflows.
