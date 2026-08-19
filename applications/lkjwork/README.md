# lkjwork

`lkjwork` is a private local work ledger for humans and coding agents. A project stores task,
dependency, lifecycle, label, note, attachment, activity, and readiness meaning in one exact
lkjscript application instance. The native client parses commands, renders results, discovers the
project, reads explicitly selected attachment files, and performs backup; it does not keep a second
task database or recompute domain policy.

## Quick start

Build and use the installed product binary:

```sh
cargo build --release --locked --bin lkjwork
target/release/lkjwork init ./work --name product-next
target/release/lkjwork --project ./work add "Implement query" --priority 20 --label runtime
target/release/lkjwork --project ./work add "Ship client" --depends '#1'
target/release/lkjwork --project ./work next
target/release/lkjwork --project ./work start '#1'
target/release/lkjwork --project ./work note '#1' add "Focused tests pass" --actor agent
target/release/lkjwork --project ./work finish '#1'
target/release/lkjwork --project ./work context --maximum-tasks 5
```

From a descendant directory, `--project` may be omitted. Discovery walks parents for one strict
`.lkjwork/locator`, then revalidates the named instance and embedded application. A path remains a
locator, not project identity.

Run `lkjwork --help` for the complete command grammar and examples. Task IDs use `#N` in human
commands; strict machine output uses JSON integers.

## Product rules

Tasks have nonreused, monotonically allocated IDs and contain validated UTF-8 title and description,
phase, optional manual hold, signed priority, labels, prerequisites, append-only notes, immutable
attachment metadata, and archive state. Task order in accepted state is ascending ID. Labels and
prerequisites preserve insertion order and reject exact duplicates.

The application owns these lifecycle rules:

- a nonarchived, unheld planned task is actionable only when every prerequisite is done;
- `start` moves an actionable planned task to active;
- `stop` moves active back to planned;
- `finish` moves an unheld active task to done while prerequisites remain done;
- `cancel` moves planned or active to cancelled;
- `reopen` moves an unarchived done or cancelled task to planned;
- only done or cancelled tasks may be archived, and unarchive preserves phase.

Readiness is derived. No client-side ready flag exists. Dependency insertion rejects missing
endpoints, self-edges, duplicates, and cycles through bounded iterative application semantics.
Actionable order is higher priority first, then lower task ID.

Expected conflicts and semantic no-change are typed application results. They publish no revision.
Successful state changes publish one revision. Attachment requests publish a pending state before
host work and one further revision after an exact outcome is resumed. A stale base rejects before
application evaluation and is never silently retried.

## Pure observations

`show`, `list`, `next`, `summary`, `context`, and `export` invoke the application-declared pure query
entry. `history` reads bounded retained semantic history. None publishes a state revision, event key,
command, host attempt, outcome, checkpoint, or HEAD change.

Pages use a zero-based `after` offset, a positive limit of at most 100, filters before pagination,
and explicit `total`, `omitted`, and `next_after` facts. `context` selects active tasks first,
actionable tasks second, and orders each class by priority/ID. Its task, note, dependency, and UTF-8
byte limits are enforced by application semantics; omitted counts cover the complete eligible set.

Every query receipt binds the exact application, instance, selected revision, record and state
digests, input, and typed result. Supplying `--known-result-digest` for the same exact query may return
an `unchanged` receipt after recomputation. The digest is equality evidence, not authority.

## Attachments

`attach TASK FILE` accepts one explicitly selected regular file from 1 through 65,536 bytes. The
client checks metadata before bounded reading. The application validates task and metadata, then the
immutable-blob adapter publishes content in the project's private namespace. Final state retains only
the exact content digest, display name, actor, and byte length.

A possibly visible put is never repeated automatically. The retained host attempt is inspected and
resumed as present, absent, or indeterminate. Deterministic fault coverage is available only in a
project created with `--deterministic-fake`; for example:

```sh
lkjwork init ./fault-work --name fault-work --deterministic-fake
lkjwork --project ./fault-work add "Reconcile evidence"
lkjwork --project ./fault-work attach '#1' ./evidence.bin \
  --fake-put unknown --fake-inspect absent
```

The last command returns a typed conflict after exactly one put attempt and one inspection. Fake
outcomes reject on production projects.

## Machine mode and foreground session

`--json` emits exactly one versioned JSON value and never ANSI or progress text. Stored text remains
JSON text. Unknown or duplicate fields, invalid UTF-8, malformed versions and IDs, trailing input,
and excessive requests reject.

`lkjwork session` accepts one strict line-delimited request per line:

```json
{"contract_version":1,"request_id":1,"project":"/absolute/work","arguments":["next","--limit","3"]}
```

Every project operation names an explicit project. Request IDs are nonzero and cannot repeat during
one session. Malformed independent lines return bounded errors without desynchronizing later lines.
The caller-owned process may hold one validated application/current-state cache keyed by exact HEAD;
each hit revalidates HEAD, mutations update only after publication, and eviction or restart is
semantically invisible. The session is not a daemon, queue, scheduler, or authority.

## Layout, validation, and recovery

A project uses this private layout:

```text
PROJECT/
  .lkjwork/
    locator
    instance-store/
    blobs/
```

The checksummed locator contains only deployment facts: product contract, exact instance,
application digest, adapter, and checksum. Project name and work state remain in application state.
Symlinked markers or parents, nonregular authority files, traversal, foreign application/instance
bindings, malformed text, noncanonical envelopes, missing records, and digest disagreement reject.

`doctor` validates current authority and the attachment closure. `doctor --deep` additionally
reexecutes every retained transition from genesis, checks every periodic checkpoint, and compares the
final state. Ordinary current open validates the exact application, HEAD-bound `CURRENT` manifest,
current record/immediate prior, state digest/accounting, event-key index, and applicable checkpoint;
it does not claim complete-history audit. Missing or corrupt `CURRENT` validates the complete chain
and reconstructs from the latest checkpoint plus at most 63 transitions without writing a repair.

## Backup, restore, and export

`backup --to PATH` holds the source lock, copies the exact deployment to private staging, verifies the
application, chain, current state, and referenced blob closure, then publishes without replacement.
Success follows file and parent-directory synchronization on the observed Linux filesystem. A failure
after rename is reported as possibly visible and must be inspected before retry; this is not a general
power-loss guarantee.

`restore BACKUP --to PATH` validates the backup, creates a new exact instance from its current semantic
state, and explicitly rebinds the immutable-blob grant to the new deployment. Task and note IDs and
state digest are preserved; instance and grant identities deliberately change. Restore never replaces
an existing project and does not claim old-instance continuity.

`export` version 1 is a bounded deterministic JSON domain view from pure application query semantics. It omits
attachment contents, paths, grants, runtime handles, and storage records. It is not restorable
authority and no import contract exists.

## Reproducibility and acceptance

The installed executable embeds `lkjwork.lkja` and validates it at startup. The checked-in application
artifact is distribution authority; `build.py` is a reviewable public-command recipe, not runtime
authority. Reproduce both the artifact and generated bindings with:

```sh
cargo build --release --locked --bin lkjscript
python3 applications/lkjwork/build.py target/release/lkjscript
target/release/lkjwork version
```

Run the complete public acceptance story and frozen product corpora with:

```sh
python3 applications/lkjwork/acceptance.py --binary target/release/lkjwork
python3 applications/lkjwork/workload.py target/release/lkjwork --profile functional
python3 applications/lkjwork/workload.py target/release/lkjwork --profile representative
```

The bootstrap trust model is one trusted local operator and OS account. The client and narrow blob
adapter are trusted native code. No network, encryption, signature, provenance, multi-user
authorization, hostile-administrator isolation, native-code sandbox, background service, database,
automatic migration, or cross-platform guarantee is claimed.
