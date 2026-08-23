# Public CLI

Status: normative.

Current contract identities and versions are executable-derived in the generated
[contract table](../generated/contracts.md) and [machine manifest](../generated/manifest.json).
This normative document does not maintain a parallel version catalog.

## Authority and executable owner

The registry in `src/platform/contract/` is the sole exhaustive owner of finite command names,
purposes, usage, project requirements, authority effects, request and response schemas, limits,
diagnostic classes, and schema digests. CLI dispatch consumes its closed operation enum. The
executable projects that registry through:

```sh
lkjscript capabilities
lkjscript capabilities COMMAND
lkjscript capabilities --known-schema DIGEST
lkjscript capabilities --section SECTION
lkjscript capabilities --known-section SECTION=DIGEST
lkjscript capabilities --output schema.json
```

The `--known-schema` form returns only the digest and `unchanged: true` when the caller already
knows the current registry. Documentation may describe workflows, but it does not define a second grammar.
An unknown command or option fails with the `cli_usage` diagnostic; no compatibility routing is
performed.

The public CLI reads and writes accepted meaning only through repository APIs. `new`, committed
`change`, published drafts, committed merges, and `restore` are the accepted-authority operations.
Package staging, review and artifact output, query indexes, logs, and runtime deployments are not
accepted program authority.

## Direct command groups

The current CLI has no universal namespace prefix. Its exact direct operations and schemas are in
the generated [operation table](../generated/operations.md).

Use `capabilities COMMAND` for the current subcommand and option grammar. One behavior has one
public name.

Global `--project PATH` selects a project for commands that require one. Otherwise repository
discovery begins at the current directory and walks ordinary ancestors. Discovery is deterministic
and rejects source-era project markers when no current meaning graph exists. Artifact, deployment,
built-in-package, project creation, and restore actions use their explicit paths instead.

## Binary-only project creation

`new DEST [--template minimal|command] [--name NAME]` creates fresh accepted authority without a
repository checkout, external artifact, network access, Cargo, or Rust toolchain. The parent of
`DEST` must exist. `DEST` may be absent or an empty ordinary directory; a nonempty destination,
non-directory, or symlinked destination or parent path rejects without publication.

Creation allocates fresh repository and package identities, constructs and fully validates an
initial graph in a private sibling stage, makes its canonical bytes durable, and exposes the
project through one filesystem rename. Its receipt names the template, project path, repository,
package, accepted revision, root, built-in dependency when present, allocated identities, and
publication evidence.

The `minimal` template contains one empty module and no dependency. The `command` template binds
the exact embedded standard package and uses the public change lowering path to add a function,
component, port, test, and command target named `main`. `check`, `build`, and `run main` operate on
the result as on any other current project.

The embedded package is derived data carried by the executable, not mutable accepted authority.
`package builtin inspect` exposes its graph contract, package ID, semantic revision, package
artifact digest, bundle digest, and byte count. `package builtin export --output PATH` writes its
exact bytes to a newly created file. Runtime loading verifies artifact integrity, and black-box
reproduction compares the exported bytes with the maintained standard artifact.

## Finite response contract

Each finite invocation writes one strict JSON value followed by a newline and writes no stderr for
a classified outcome. A success has this envelope:

```json
{
  "contract_version": 4,
  "ok": true,
  "status": "success",
  "command": "inspect.status",
  "result": {}
}
```

`status` is closed per command. A valid request whose requested semantic result is rejected, such
as a stale base or invalid candidate, uses the same envelope with `ok: false` and the exact typed
status. A decoding, corruption, capability, resource, cancellation, or infrastructure failure has
`status: "failure"` and one structured `error` containing class, code, message, and only bounded
optional location or notes. Normal output excludes stack traces, full schemas, passing-test lists,
secrets, and child logs.

The hard finite-response limit is 4 MiB. Large bodies require explicit selection; growing results
use budgets and continuations or publish bytes to an explicit output file. Project-bound reads
name their observed revision. Results and diagnostics use deterministic ordering.

Process exits are 0 for success, 2 for usage/source or semantic rejection, 3 for capability failure
or cancellation, 4 for resource exhaustion, 5 for corrupt authority, and 6 for infrastructure
failure. Transaction-style semantic outcomes additionally use 7 for stale base or HEAD and 8 for
an invalid candidate graph.

`serve` and `worker` are resident modes rather than finite commands. They emit bounded JSON event
records under resident protocol version 1 for ready and stopped observations and continue until
shutdown or failure.

## Inspection, queries, and bounds

`inspect` covers status, bounded project orientation, one exact owner with optional body, targets,
one revision, artifacts, and deployment descriptors. `query` covers owners, exact or broad name
search, relations, callers, callees, type uses, capability uses, task context, impact, and a closed
structured request. Owner selection uses typed stable identities; a name is a locator, not
continuity.

Growing queries have item, byte, work, depth, and fanout budgets. Defaults are 50 items, 64 KiB,
100,000 work, depth 4, and fanout 1,000. Current hostile/resource maxima are 10,000 items, 4 MiB,
10,000,000 work, depth 32, and fanout 10,000. Exhaustion is explicit and does not alter meaning.

Ordering is independent of hash iteration and physical index position. A continuation binds the
query contract, exact revision, normalized query, and cursor with a domain-separated integrity
check. Changed or malformed inputs reject. Query indexes are disposable state. The broad relation
index is revision-bound; exact owner/name queries use content-addressed local-index v3 shards named
by a revision/root-bound manifest. Local accepted changes update touched exact shards by delta.
Missing or corrupt state rebuilds from canonical authority.

## Public change protocol

`change (--request JSON | --request-file PATH) [--dry-run|--commit]` accepts one strict change
contract v3 request. With no mode flag or with `--dry-run`, it normalizes and validates without
publication. `--commit` lowers the same request to the exact internal transaction protocol and may
publish at most one revision.

The request envelope contains:

- `contract_version` equal to 3;
- optional `base_revision` and `idempotency_key`;
- ordered `preconditions` and `changes`;
- a bounded transaction `budget`;
- optional bounded nonsemantic `intent`.

An omitted base is resolved to the observed current revision once. An idempotency key requires an
explicit base. One commit prepares semantic validation once, carries the exact result into
publication, rereads HEAD under the repository write lock, and verifies the base, result root,
root delta, changed modules, summary delta, semantic certificate, and validation-fact bindings
before publishing. A precondition-free request may prepare locally when it contains only eligible
pure-function body replacements, only independent empty-module creations, only module renames, or
only declaration renames. Body replacement validates selected modules and their recursive local
import dependencies and carries removed nested-identity tombstones in the same delta. Module and
declaration rename validate owning modules plus outgoing imports without rewriting importers or
targets. Preconditions, mixed operations, and every other request use complete candidate
preparation. A separate dry-run and later commit are separate invocations and each prepare
independently; no reusable public prepared handle exists. Rejection, validation, no-change, and
dry-run publish nothing.

The exact accepted change, type, and expression form catalogs come from `capabilities --section
change`, `capabilities --section type`, and `capabilities --section expression`. The complete nested
change schema is available through `capabilities --output schema.json` and is retained as
[generated JSON Schema](../generated/protocol.schema.json).

Declaration references accept a typed request-local symbol, a local `decl_` identity resolved at
the selected revision, or a fully exact
`exact:PACKAGE_HEX/mod_HEX/decl_HEX` selector. The last form is required for direct dependency
references that cannot be derived from a local declaration identity. Every form lowers to the same
typed package/module/declaration reference before validation; the selector string is not graph
authority.

`capabilities` publishes every current high-level change, top-level type form, concise expression
form, owner kind, relation role, and declaration-reference form through digest-addressed sections.
Unknown request-envelope fields and trailing input reject at the owning boundary. Executable
decoder conformance remains an independent oracle for generated schema strictness.

A created construct uses an `as` symbol beginning with `$`. The remaining 1–64 bytes contain only
ASCII alphanumeric characters, `-`, or `_`. A symbol is request-local, unique, defined before use,
and valid only in its typed identity domain. Allocation is deterministic from repository, exact
base, and request content. The result returns the complete symbol-to-domain-and-stable-ID map plus
the exact lowered transaction digest. Repeating a dry-run against the same base yields the same
map; exact idempotent replay preserves it.

Preconditions can bind a root digest, owner existence or absence, or an expected owner name. A
transaction result names requested base, observed current revision, transaction digest, semantic
diff and predicted or published revision when applicable, diagnostics, and affected-owner count.
At most 64 affected owners are inline; an accepted receipt gives `history show REVISION` as exact
expansion.

## Drafts, history, and packages

A draft is non-executable authority bound to an exact repository, base revision, and generation.
`draft` creates, inspects, appends an exact draft-bound transaction, rebases, publishes, or drops
it. Append and validation cannot alter accepted HEAD. Publication rejects stale or incomplete
draft state.

`history` lists bounded revisions, shows one revision and receipt, compares two exact revisions, or
previews/commits a three-way merge. Stable identities distinguish addition, removal, rename, move,
and modification. A conflicting merge publishes nothing. Persistent typed conflict resolution is
not implemented in the current CLI.

`package stage PATH` verifies a graph-native artifact and stores its exact package objects for a
later dependency-binding change. Staging is operational and cannot alter HEAD. Built-in package
inspection and export do not require a project.

## Check, build, run, review, and recovery

`check` executes graph-owned tests through prepared bytecode and an implementation-disjoint
semantic reference interpreter. All-pass output contains aggregate counts, tier identities,
revision, work observations, and differential equality rather than every passing test.

`build` lowers the exact accepted revision and dependency closure to a deterministic graph-native
artifact. `run` invokes a selected pure command, batch, or test target through both execution tiers
and rejects a mismatch. `serve` starts a bounded plaintext HTTP deployment; `worker` starts a
bounded worker deployment. Deployment descriptors contain external grants and secret bindings,
not accepted program meaning.

`review` produces deterministic span-free JSON marked non-authoritative and not importable.
`backup` writes a contract-4 segmented directory containing a bounded manifest, bounded index
segments, and individually copied canonical object files; it does not accumulate the complete
backup payload in one in-memory value, but it retains an O(object-count) sorted key index. It is not
a bounded-memory object pack. `restore` requires an existing destination directory without current
authority, verifies each segment and entry in a private stage, runs deep
structural/history doctor, and then makes the restored store visible. It does not currently rerun
the complete cross-package semantic validator as part of restore. Backup and restore preserve
repository identity. Disposable indexes rebuild. `doctor --deep` is the explicit exhaustive
retained revision/root/page/module/receipt walk within the history bound; it does not currently
walk dependency artifacts or drafts or rerun cross-package semantics. `doctor cleanup` is a
read-only retention-contract-1 preview rooted at HEAD's parent DAG plus every live draft base DAG.
It reports retained/reclaimable candidate counts and bytes, derived counts/bytes, unknown-entry
counts, and a plan digest; it always reports `destructive_ready: false` because revision pins,
active-reader leases, and registered backup roots are not represented. It has no delete mode.

Output files are created or replaced only through bounded publication rules. Ordinary derived
output rejects symlinks and non-regular targets, writes a unique sibling stage, synchronizes it,
renames it, and synchronizes the parent; an equal existing value reports `unchanged`. Built-in
export requires a new output file.

## Concurrency, recovery, security, and non-goals

Every accepted write observes or names one exact base and has one visibility point. Newly written
canonical data becomes durable before HEAD. After an uncertain external interruption, callers
reconcile by reading `inspect status` and retained history rather than blindly retrying.

The CLI treats request files, artifacts, backups, continuations, deployments, paths, and runtime
inputs as hostile decoding boundaries and applies checked byte/count budgets before publication.
Ordinary output and retained excerpts must not expose deployment secrets.

The HTTP server is plaintext and the PostgreSQL adapter uses `NoTls`. There is no CLI path for TLS,
certificates, or ACME, and no such implementation is planned. Encrypted deployments require an
appropriate external trusted transport boundary. That boundary does not provide hostile-code or
multi-tenant isolation.

The current CLI can author explicit rank-1 generic pure functions, direct calls with exact type arguments,
named pure function values, and invocation. It does not claim constraints, type-argument inference,
generic task functions, closure capture, persistent conflict resolution, fully incremental
validation or compilation, garbage collection, live-store packing, bounded-memory backup key
enumeration, or an agent session protocol. Those mechanisms require separate completed contracts
and consumers; physical storage records never become public authoring syntax.

Compatibility policy is direct cutover. Unknown contracts and predecessor commands, artifacts,
stores, source-era projects, and schemas reject rather than selecting a migration reader.
