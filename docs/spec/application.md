# Application worlds

This specification owns immutable runnable application closure, application public values, invocation
profiles, embedded cases, and application-declared host requirements. Release identity is specified in
[`reusable-release.md`](reusable-release.md); durable state and grants are specified in
[`instance.md`](instance.md).

## Authority and contract

Application contract version 8 owns one artifact formed from an exact root release, an exact
exported entry, one closed invocation profile, one `RunPolicy`, and at most 256 immutable cases. In a
semantic project these inputs are accepted contract-1 application-target meaning and are lowered
mechanically from one exact revision. The artifact embeds the complete exact release graph.
Coordinates, paths, workspaces, mutable resolvers, grants, deployment state, caches, and compiled
code are not application authority.

Preparation validates every release, exact dependency, export, type, profile target, import route,
case, resource bound, flattened closure, lowering, and Core program before bytes can publish. Every
embedded release case and application case must pass. Target testing and target publication consume
the same prepared bytes and report; neither changes project history.

## Public values

The closed `ApplicationValue` set is `unit`, `bool`, `i64`, `bytes`, `text`, nominal product, nominal
sum, and nominal sequence. Nominal targets are exact `(ReleaseId, ReleaseItemId)` pairs. Products
carry every exact field once in declaration order; sums carry one exact variant and its declared
payload; sequences carry one exact sequence declaration and ordered values of its exact element type.
Foreign but shape-equal nominal values reject.

Strict JSON uses `null`, JSON booleans, signed JSON integers, unpadded URL-safe base64 byte strings,
JSON strings for validated UTF-8 text, closed nominal objects, and ordered arrays inside nominal
sequence values. The internal application/instance value codec is canonical binary and independently
bounded; it is not a second public schema. Unknown kinds or fields, duplicates, invalid UTF-8,
noncanonical IDs, wrong nominal members, excessive depth/items/bytes, truncation, and trailing input
reject.

## Invocation profiles

The profile set is closed:

- `typed`: the exported entry accepts exact typed arguments and returns one exact typed value;
- `bytes_stream`: one bounded byte input and byte output adapter over an exact bytes entry; and
- `stateful`: distinct mutation, resume, and pure-query entries plus exact application-owned types;
  and
- `interactive`: pure foreground state with exact initialize, update, resume, and render roles.

Typed and byte-stream profiles remain independent public consumers. They do not acquire state or host
authority.

### Stateful profile

A stateful profile names exact `State`, `Event`, `Response`, `Query`, `QueryResult`, `Command`,
`Outcome`, and `Decision` nominal types. The application entry is the mutation entry; the profile also
names exact resume and query entries. Their required signatures are conceptually:

```text
mutation_entry : (State, Event) -> Decision
resume_entry   : (State, Outcome) -> Decision
query_entry    : (State, Query) -> QueryResult
```

`Decision` has four exact variants and payload mappings:

- `declined { response }`: expected domain conflict, no next state and no command;
- `unchanged { response }`: accepted semantic no-change, no next state and no command;
- `completed { state, response }`: one proposed next state and no command; and
- `suspended { state, response, command }`: one proposed next state and one command.

Response and query-result types are application-owned. Infrastructure corruption, authority denial,
and execution/resource failure remain outside them. The decoder rejects a decision whose variant,
payload, state, response, or command does not match its profile mapping.

The query entry runs the same verified pure language route as mutation code. It cannot return a
command and has no grant, instance, or publication primitive. Instance query publication rules are
specified in [`instance.md`](instance.md).

### Interactive profile

Interactive profile version 2 maps exact release items to application-owned `State`, `Event`,
`UpdateResult`, `Frame`, `Action`, and `Outcome` types and to initialize, update, resume, and render
functions. The conceptual roles are:

```text
initialize : (rows, columns) -> State
update     : (State, Event) -> UpdateResult
resume     : (State, Outcome) -> UpdateResult
render     : State -> Frame
```

`UpdateResult` contains an exact next state, changed/exit booleans, and one closed action. The
application action vocabulary is `none`, bounded semantic-project reads/mutations/target actions,
and bounded selected-filesystem list/read/save/reconcile actions. Action payloads are data only;
the application receives no path authority, project handle, terminal handle, file descriptor,
thread, or OS object. The native runner may execute at most one action at a time, then supplies one
typed outcome through `resume`. Input while an action is unresolved rejects; possible external
visibility remains an explicit outcome and is never retried by application semantics.

The event vocabulary is key, paste, resize, and close. A key contains one closed code, optional
character scalar, control/alt/shift flags, and repeat state. Paste is one event. Rows and columns are
1 through 1,000, a paste contains at most 65,536 Unicode scalars, a frame at most 131,072 scalars,
and status at most 4,096 UTF-8 bytes. Invalid scalars, foreign nominal values, excessive dimensions,
malformed action routes, or wrong function signatures reject during preparation or event decoding.

A frame contains bounded rows/columns, semantic Unicode scalars, cursor row/column/visibility, and
status. It contains no raw terminal escape sequence. Rendering is a pure application query over
foreground state and publishes nothing. The runner's cell width, clipping, escaping, and terminal
lifecycle are specified in [`terminal.md`](terminal.md).

An event step or action resume is transactional within the ephemeral session: the runtime computes
and validates a candidate state, pending-action state, and rendered frame before replacing the prior
session state. Update, resume, policy, or render failure therefore preserves the prior state and any
prior pending action. This is not durable rollback and has no effect on an external project or file
publication that already occurred.

Interactive state is explicitly ephemeral. Events do not create development revisions or durable
instance revisions. Process exit loses unsaved state. Semantic-project publication and filesystem
publication occur only through separate host action owners and cannot be rolled back by a later
frame or output failure. Headless replay contract 3 invokes the same functions sequentially for at
most 10,000 events and outcomes and returns exact frame/action digests.

Historical profile version 1 is accepted only while validating immutable development snapshots that
predate the direct cutover. Current target normalization, application-format-8 preparation, and
interactive execution require version 2. This is history reconstruction, not an artifact or mutation
compatibility path.

## Host requirements

An application import declares one canonical slot, the exact built-in interface identity, nominal
request/outcome/command wrapper targets, and a complete closed operation-to-variant route. Imports
declare requirements only; an application contains no grant or path.

The only retained interface is `immutable_blob_v1`:

- `put_blob` has success, already-present, known-previsibility failure, possible-visibility, timeout,
  cancellation, and cleanup outcome classes under the closed compatibility table;
- `inspect_blob` has reconciliation-present, reconciliation-absent, and
  reconciliation-indeterminate classes.

Interface identity is derived from the immutable contract name. Unknown interfaces, operations,
classes, duplicate routes, incomplete routes, or mismatched nominal wrappers reject. The superseded
application-activation interface has no current consumer and is absent.

## Embedded cases

Each case names a bounded stable case name, exact target, exact arguments, expected value or stable
trap class, and explicit run policy. Stateful applications must include at least one query case, a
resume case when imports exist, and cases covering `declined`, `unchanged`, `completed`, and
`suspended`. Cases are immutable distribution checks, not mutable test history.

Suite fuel is capped at 100,000,000. A skipped, incomplete, exhausted, malformed, or engine-failed
case does not pass.

## Artifact version 8

The sole successful application encoding is:

- magic `LKJAPP\0\x08` and little-endian format `8`;
- semantic schema `lkjscript-tsm008` from the workspace artifact owner;
- bounded canonical release graph, exact root/entry/profile/policy/cases; and
- a domain-separated digest over the canonical payload.

The decoder checks lengths before allocation, requires canonical re-encoding, reconstructs and
validates the exact graph/profile, compiles every required entry, and runs the immutable cases.
Application format 7 and every predecessor reject directly; no reader, migration, edition, alias,
or fallback remains.

## Derivation, self-description, execution, and inspection

`target show|test|build|run` selects accepted target meaning at one exact project revision. Target
build preflights its receipt, publishes one no-overwrite artifact, and publishes no development
revision. The command-local `app build` predecessor rejects. `app validate|inspect|test|run|stream`
remains an independent immutable-distribution consumer.

Interface-description contract 1 is derived directly from validated artifact bytes. It exposes the
exact root release, entry, profile, imports, and release export descriptors needed by a native
client. A client resolves names only inside that exact artifact, checks kinds and complete nominal
shapes, and retains the application digest. Generated binding constants are neither required nor
trusted; a stale or foreign view cannot silently authorize execution.

Inspection exposes exact digest, graph/release facts, profile, cases, policy, limits, and explicit
absence of signatures/provenance. Run receipts contain the exact typed result plus bounded
operational stage observations. Observations are neither semantic values nor authority.

Application execution is deterministic for exact bytes, entry, arguments, and policy. It publishes
nothing. Output failure has no rollback meaning. Artifact publication uses no-replace immutable file
publication and reports possible visibility rather than silently retrying after its visibility
boundary.

No application artifact contains a registry, grant, deployment locator, instance state, runtime
handle, live resource, ambient time/randomness, filesystem or network access, signature, provenance,
native code, or compatibility metadata.
