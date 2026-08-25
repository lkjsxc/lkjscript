# Public CLI

Status: normative.

Current contract identities and versions are executable-derived in the generated
[contract table](../generated/contracts.md) and [operation table](../generated/operations.md).
This normative document does not maintain a parallel version catalog.

## Authority and executable owner

The registry in `src/platform/contract/` is the sole exhaustive owner of finite command names,
purposes, usage, project requirements, authority effects, request and response schemas, limits,
diagnostic classes, and schema digests. CLI dispatch consumes its closed operation enum. The
executable projects that registry through:

```sh
lkjscript capabilities
lkjscript capabilities COMMAND
lkjscript capabilities --known-registry DIGEST
lkjscript capabilities --section SECTION
lkjscript capabilities --known-section SECTION=DIGEST
lkjscript capabilities --output registry.records
```

The `--known-registry` form returns only the digest and `unchanged=true` when the caller already
knows the current registry. Documentation may describe workflows, but it does not define a second grammar.
An unknown command or option fails with the `cli_usage` diagnostic; no compatibility routing is
performed.

The normalized public slice reads and writes accepted meaning only through `GraphRepository` APIs.
Normalized `new` and `change apply` are its current accepted-authority operations. Draft, merge,
restore, package, compiler, runtime, and deployment commands still target predecessor authority
until direct cutover; their output cannot alter a normalized repository. Package staging, review,
artifact output, query indexes, logs, and runtime deployments are not accepted program authority.

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

`new DEST [--template minimal] [--name NAME]` creates fresh accepted authority without a
repository checkout, external artifact, network access, Cargo, or Rust toolchain. The parent of
`DEST` must exist. `DEST` may be absent or an empty ordinary directory; a nonempty destination,
non-directory, or symlinked destination or parent path rejects without publication.

Creation allocates fresh repository and package identities, constructs and fully validates an
initial graph in a private sibling stage, makes its canonical bytes durable, and exposes the
project through one filesystem rename. Its receipt names the template, project path, repository,
package, accepted revision, root, built-in dependency when present, allocated identities, and
publication evidence.

The normalized `minimal` template contains an empty package and no dependency. The predecessor
command template is rejected by normalized `new`; a normalized command template remains a cutover
blocker. Consequently `check`, `build`, and `run` do not yet consume projects created by this path.

The embedded package is derived data carried by the executable, not mutable accepted authority.
`package builtin inspect` exposes its graph contract, package ID, semantic revision, package
artifact digest, bundle digest, and byte count. `package builtin export --output PATH` writes its
exact bytes to a newly created file. Runtime loading verifies artifact integrity, and black-box
reproduction compares the exported bytes with the maintained standard artifact.

## Finite response contract

`capabilities`, normalized `new`, `status`, exact `inspect owner`, and `change` write deterministic
compact line records and no stderr for a classified finite outcome. Records use one closed
operation followed by unique `field=value` assignments and one escaping rule. Success begins with
`result status=... command=...`; failure begins with `result status=failure` and contains bounded
`diagnostic` records with class, code, message, and available source location. Remaining finite
commands retain their predecessor JSON envelope only until direct cutover. Compact output excludes
stack traces, complete schemas, passing-test lists, secrets, and child logs.

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

Normalized `status` covers project orientation, and `inspect owner KIND ID [--package PACKAGE]`
reads one exact coarse owner summary at the observed revision. Other inspect and query actions are
currently rejected for normalized repositories rather than falling back to predecessor readers.
The eventual bounded relation, name, context, and impact query surface remains a cutover blocker.
Owner selection uses typed stable identities; a name is a locator, not continuity.

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

`change plan (--input RECORDS | --input-file PATH)` and `change apply ... --plan DIGEST` accept
flat UTF-8 records under contract `lkjscript-change-records-1`. A request begins with exactly one
`request base=REVISION` record and may add bounded `idempotency` and nonsemantic `intent` fields.
Every later record is a closed semantic operation, type fragment, expression fragment, or indexed
edge. There is no indentation meaning, implicit scalar typing, duplicate field, macro, include, or
JSON fallback.

`plan` parses, resolves fragments, lowers to the typed authored model, allocates request-local
identities, performs impact analysis and validation, and returns a `plan_` digest plus the predicted
revision, semantic diff, compact counts, validation work, allocation map, and predicted receipt and
revision-record identities. It publishes nothing and reports no durable receipt path. `apply`
reparses and reprepares the input through the same path, rejects a mismatched reviewed digest before
repository access, and then atomically publishes or reports a stale base without partial visibility.
Both actions require an explicit exact base.

The executable sections `capabilities --section change`, `type`, and `expression` are the only
public vocabulary owner. The current compact subset includes module, record, variant, pure
function, constant, and test creation; field, case, and function-parameter addition; owner rename;
declaration move; and complete function-body replacement. Types include the advertised primitive,
named, parameter, collection, result, stream, and function forms. Expressions include unit,
boolean, integer, text, local/constant references, conditional, sequence, and direct call. Broader
typed engine operations remain private until their compact workflows are complete.

`$name` identifies request-local semantic owners and expressions; `@name` identifies notation-only
type fragments. `expression.argument` and `type.argument` records use zero-based contiguous indexes
to keep trees flat and deterministic. Exact local declaration selectors use `decl_...`, qualified
selectors use `MODULE/NAME`, and dependency declaration references use `pkg_.../decl_...`.
Selectors lower to typed exact references before validation; record spelling is never accepted
graph authority.

Allocation is deterministic from repository, exact base, normalized typed request, and optional
idempotency key. Plan and apply return equal complete symbol maps. Function-body replacement
retires the exact old expression/binding ownership closure in the same semantic change. Raw JSON,
the former `--request`/`--request-file` grammar, and `--dry-run`/`--commit` are rejected before
publication. Preconditions, large external value files, direct single-operation flags, and broad
result export are not yet exposed by this compact subset.

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
