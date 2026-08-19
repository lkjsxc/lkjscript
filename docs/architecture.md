# Current architecture

The repository is an agent-native semantic application platform plus the complete `lkjwork` local
work-ledger product. One immutable typed representation owns accepted meaning at each authority
boundary; source documents, JSON, renderings, caches, indexes, compiled forms, and evidence remain
proposals or derived views.

## Authority domains

```text
development workspace revision
        |
        | exact release projection
        v
immutable reusable release graph
        |
        | exact application composition
        v
immutable application artifact
        |
        | instance creation + exact grants
        v
durable instance journal --HEAD--> current revision
        |                              |
        | application command          | pure query
        v                              v
immutable-blob adapter            typed query result
```

Workspaces own development identity/history. Releases own reusable immutable semantic identity.
Applications own runnable closure, exact stateful types and entries, cases, and host requirements.
Instances own mutable state continuity, journal/checkpoints, grants, pending commands, attempts, and
outcomes. Deployments own paths and process placement. Product locators merely find one exact
instance. Digests have only the narrow equality/integrity role assigned by their owner.

## Semantic workspace

`src/schema.rs` owns the closed type/operation/declaration vocabulary, including validated text and
nominal sequences. `src/graph.rs`, `src/validate.rs`, and `src/transaction.rs` own accepted snapshot
structure, whole-proposal validation, and validate/apply parity. `src/artifact.rs` owns canonical
workspace artifact 7; `src/persistence.rs` owns immutable revisions and HEAD9 publication.

Editable documents and context packets under `src/workbench/` normalize into or observe the same
transaction owner. They never bypass durable IDs, type/scope/dominance checks, publication, or exact
base selection.

## Release and application composition

`src/release/` projects one selected package closure into canonical release-format-2 identity. It
erases workspace identity, preserves exact nominal/release references, validates explicit dependency
bytes, and runs immutable release cases.

`src/application.rs` composes one complete exact release graph into application format 5. Its
stateful profile names exact mutation, resume, and query entries; application-owned response/query
types; and exact decision-variant mappings. It also owns public-value conversion and canonical
application-value binary encoding. The only built-in host requirement is `immutable_blob_v1`.

Application execution compiles the selected closure to independently verified Core IR and runs the
explicit-frame interpreter. Lowering, Core, ownership plans, managed handles, and runtime frames are
derived and disposable.

## Values and runtime memory

Bytes and text share the safe generation-checked managed byte store. Text validity is established at
every public/artifact boundary and cannot be invalidated by arbitrary byte slicing. Nominal sequences
are immutable ordered invocation-local objects containing safe `Arc<RuntimeValue>` elements. Append
and replace allocate one new sequence and may share immutable elements.

Logical visible cells/bytes, retained canonical backing, managed objects, depth, items, frames, and
fuel are bounded independently. Sharing cannot evade retained accounting. Test-only allocate-new
encoding is the differential oracle. No accepted value exposes allocation, handles, addresses,
reference counts, or representation identity; no tracing collector or unsafe Rust is present.

## Instance journal and current path

`src/instance.rs` is the sole owner of instance state selection, mutation/query evaluation,
idempotency, history, checkpoints, grants, host evidence, and publication. Every accepted publishing
transition creates one immutable hash-linked record. Full semantic state is stored only at revision
zero and every 64 revisions; intervening records store exact input, typed response, resulting state
digest/accounting, and optional command.

HEAD selects the exact current record, cumulative journal bytes, tombstone state, and digest of a
bounded current manifest. The manifest contains the exact current state and contiguous published
event-key index. Ordinary current queries, mutations, inspections, and host operations validate this
HEAD-selected closure and avoid replay. Published idempotency replay reads one indexed immutable
record. A missing/mismatched manifest falls back to complete chain validation and checkpoint replay;
queries do not write repairs.

Deep audit walks the complete HEAD-selected chain, validates every canonical link and checkpoint, and
reexecutes every transition from genesis. The manifest is acceleration and cannot authorize a state
that the deep oracle does not reproduce. Publication order is immutable record, derived manifest,
then HEAD. Failure before HEAD leaves the old authority; possible HEAD visibility is reported unknown.

## Host boundary

Applications emit one typed command only through a declared import slot. Instances bind that slot to
one exact immutable grant. The adapter cannot invent a command, response, state, or authority.

The immutable-blob production adapter receives bounded content from the application command and a
private namespace descriptor. It has no general filesystem capability. A put attempt is durable
before visibility-capable work. Known success/failure, possible visibility, and reconciliation are
distinct typed outcome classes. Unknown visibility never causes an automatic repeat. Deterministic
fake outcomes exercise the same instance owner and are permitted only by an exact fake grant.

## Runtime topologies

`src/runtime.rs` admits and observes application/instance operations without duplicating semantic or
persistence logic. `src/runtime_protocol.rs` and `src/bin/lkjscript/runtime.rs` provide strict
one-shot and foreground line-session process adapters. One store lock serializes instance operations;
there is no hidden queue or background work.

The product process calls the same `InstanceStore` owner directly. One-shot is the shell-composable
baseline. `lkjwork session` may retain one prepared application and current-state object keyed by
exact HEAD; HEAD is reread on every request and publication updates only after success. This measured
caller-owned reuse is neither a daemon nor persistent authority.

## lkjwork packaging and client

`applications/lkjwork/lkjwork.lkja` is the exact distribution application. `build.py` is the
reviewable public-command recipe; generated `bindings.json` names artifact-owned targets and is
checked against the embedded digest. `src/bin/lkjwork.rs` embeds and independently validates both at
startup.

`src/bin/lkjwork/project.rs` owns strict private locator/deployment handling, exact instance/grant
construction, explicit attachment routing, and staged backup/restore. `bindings.rs` constructs and
checks exact application values. `render.rs` maps typed product results to deterministic JSON and
terminal-safe human text. The client never reads instance state to compute readiness, dependencies,
filters, next work, context, or export.

A project layout is:

```text
PROJECT/.lkjwork/
  locator
  instance-store/
  blobs/
```

Project name and task data remain application state. The locator contains exact deployment facts and
checksum only. A moved exact backup remains one instance; restore deliberately creates a new instance
and rebinds its path-bearing blob grant while preserving semantic state.

## Backup and export

Backup holds the source store lock, deep-validates source history, copies one private closure into a
no-replace staging directory, deep-validates staged authority and blob references, synchronizes, then
renames and validates the public destination. It claims integrity under the local filesystem/OS
assumptions only, not power-loss proof or authenticity.

Export version 1 is a bounded pure application query view. It includes product entities and exact
revision/digest/omission facts but excludes raw attachment bytes, paths, grants, records, and runtime
handles. It is not import or restore authority.

## Trusted computing boundary and absences

The bootstrap trust boundary is the local operator/OS account, Rust product/runtime code, validated
application bytes, and narrow blob adapter. All text, JSON, artifacts, locators, paths, instance
records, manifests, checkpoints, outcomes, backups, and blobs are hostile input. Symlinks,
nonregular files, foreign identities, noncanonical forms, excess work, and digest disagreement fail
closed. Terminal safety is enforced independently of text semantics.

No process is described as a sandbox. There is no network, multi-user authorization, secrets system,
encryption, signature/provenance, broad filesystem interface, daemon, scheduler, worker pool, async
runtime, database, persistent query cache, bytecode/JIT/native tier, automatic migration, or hidden
compatibility path.
