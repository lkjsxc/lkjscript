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
Normalized `new` and `change apply` are its current accepted-authority operations; `status`, exact
owner inspection, and normalized query are revision-pinned reads. Draft, merge, restore, package,
compiler, runtime, and deployment commands still target predecessor authority until direct
cutover; their output cannot alter a normalized repository. Package staging, review, artifact
output, query indexes, logs, and runtime deployments are not accepted program authority.

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

`capabilities`, normalized `new`, `status`, exact `inspect owner`, `query`, and `change` write
deterministic compact line records and no stderr for a classified finite outcome. Records use one
closed operation followed by unique `field=value` assignments and one escaping rule. Success
begins with `result status=... command=...`; failure begins with `result status=failure` and
contains bounded `diagnostic` records with class, code, message, and available source location.
Remaining finite commands retain their predecessor JSON envelope only until direct cutover.
Compact output excludes stack traces, complete schemas, passing-test lists, secrets, and child
logs.

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
reads one exact coarse owner summary at the observed revision. Normalized query contract 3 has this
exhaustive grammar:

```text
query owners [--kind KIND] [--limit N] [--bytes N] [--continuation TOKEN]
query find CLASS NAME [--parent OWNER]
query relations OWNER|package --direction incoming|outgoing [--kind KIND] [--limit N] [--bytes N] [--continuation TOKEN]
```

`capabilities query` is the executable-owned inventory for the three actions, accepted options,
owner kinds, namespace classes, relation kinds, directions, limits, response fields, selector
fields, and continuation metadata. `owners` reads live canonical owner bindings in encoded typed-ID
order. An optional exact kind filter may produce an empty page with a continuation; its resume key
is the last owner key visited so every page progresses. Named owner projections reproduce the
canonical namespace class, name, and package or exact owner parent. Unnamed owners receive no
synthesized name.

`find` performs one case-sensitive canonical `Name` lookup in the committed namespace witness.
Module and target classes are package-root and forbid `--parent`; every child namespace class
requires one exact local parent of an admissible typed kind. Absence is a successful `match=false`
result. A witness match is returned only after the live canonical owner reproduces the same owner
key, class, parent, and name. Missing required evidence or disagreement is corruption, not a cache
miss or no-match. Names remain mutable locators; stable typed owner identities express continuity.

`relations` requires one direction and selects either one live exact local owner or the current
package endpoint. Incoming reads use the committed reverse relation witness and outgoing reads use
the forward witness. An exact kind filter narrows the map prefix. Keys, endpoint, direction, kind,
and canonical empty values are revalidated while reading. Results retain separate package and
owner endpoint fields, including exact foreign endpoints already present in accepted relations;
query never opens an ambient foreign repository. A nonexistent local owner is a semantic failure,
not an empty page.

Every success names the project path and name, repository, package, exact observed revision,
normalized query digest, registry digest, logical summary, and dimension-separated work. Reported
work keeps map pages, map bytes, map entries, catalog lookups, store objects, store bytes,
canonical records, witness records, and rendered output bytes separate. There is no public or
internal scalar query work/fuel budget. The default page is 50 items and 64 KiB. Item limits are 1
through 10,000; output limits are 1,536 bytes through 4 MiB and remain subject to the global compact
response bounds. Resource exhaustion is typed and never alters repository state.

Paged order is the canonical logical owner or relation key order, independent of hash iteration,
insertion history, files, page coordinates, and cached indexes. A stateless `qcont_` continuation
is at most 320 bytes and canonically binds query and continuation versions, repository, package,
exact revision, operation, normalized selector and ordering digest, exclusive logical resume key,
and a distinct integrity digest. Item and byte limits are excluded so a resumed page may select new
valid limits. Malformed, padded, oversized, foreign, selector-mismatched, or stale tokens reject
before semantic map traversal. No continuation file, session, daemon, cache record, or mutable
cursor is created.

Ordinary normalized query never reconstructs the complete graph, invokes the full relation oracle,
rebuilds an index, repairs witness data, writes derived files, or advances HEAD. Complete canonical
reconstruction and relation extraction remain independent test and doctor oracles. The predecessor
actions `callers`, `callees`, `types`, `capabilities`, `context`, `impact`, and `request`, their JSON
or file request forms, and predecessor continuations are rejected. Context traversal, generic
impact, fuzzy search, historical revision query, and saved queries are not available under another
spelling.

## Public change protocol

`change plan (--input RECORDS | --input-file PATH) [--output PATH]` and
`change apply ... --plan TOKEN` accept flat UTF-8 records under the current Change contract named
by the generated contract table. A request begins with exactly one
`request base=REVISION` record and may add bounded `idempotency` and nonsemantic `intent` fields.
Every later record is a closed semantic precondition, operation, type fragment, expression
fragment, or indexed edge. There is no indentation meaning, implicit scalar typing, duplicate
field, macro, include, or JSON fallback.

The same public command also provides one direct single-operation adapter:

```text
change plan rename.owner --base REVISION --owner OWNER --name NAME [--idempotency KEY] [--intent TEXT] [--output PATH]
change apply rename.owner --base REVISION --owner OWNER --name NAME [--idempotency KEY] [--intent TEXT] --plan PLAN
```

Direct `OWNER` is one exact typed owner identity. It is not a request-local symbol or name lookup.
Direct flags construct the same typed authored request and publication options as an equivalent
`request` plus `rename.owner` record pair; they do not synthesize or reparse compact text.

`plan` parses, resolves fragments, lowers to the typed authored model, allocates request-local
identities, and prepares the candidate exactly once through canonical delta construction, impact
analysis, validation, and required test selection. It returns one canonical `plan_` token followed
by 128 lowercase hexadecimal characters. The first 64 characters are the request commitment to
normalized authored intent, exact declared budgets, idempotency, and intent; the second 64 are the
prepared-plan commitment to the canonical logical-plan records. The former 64-character
request-only token is a predecessor contract and is rejected.

`apply` strictly parses the token, renormalizes the authored request, and compares the request
component before project discovery or repository access. Only an equal request is opened and
reprepared against its exact base. Apply renders the same logical plan and compares the prepared
component before calling `GraphRepository::publish`; the existing publication-lock base recheck
then either advances HEAD once or reports stale authority. Request and prepared-plan mismatches are
distinct diagnostics and never publish. Both adapters converge before either commitment,
repository access, preparation, response rendering, or publication.

For apply with an idempotency key already bound to the same exact base, the repository may reopen
that immutable historical base for retry preparation. The CLI still reparses the complete request,
recomputes and compares both reviewed token components, and enters the publication lock; only the
exact existing binding returns `already-accepted`. A different request, token, base, or key cannot
use this recovery path, and replay never creates a second revision.

With `--output`, plan streams contract `lkjscript-logical-change-plan-1` to an external canonical
file while hashing the same pre-trailer records. Its closed records bind interpreting contracts,
repository/package/base/result/state identities, request controls, transaction and semantic-diff
summaries, typed allocation ordinals, every exact owner/type/dependency/retirement change, every
removed and added relation, structural and semantic validation owners, selected tests, sorted
logical impact reasons, and derived counts. The final non-hashed record repeats both commitments
and the complete token. Exhaustive record and field vocabulary is executable-owned by
`capabilities --section change`, not duplicated here.

Witness edit programs, summary refresh, compiler units, cache/storage layout, staged objects,
receipt/revision object digests, timing, filesystem paths, output status, and request-local symbol
spelling are operational or presentation facts and do not enter the prepared-plan commitment.
The strict streaming decoder rejects unknown, duplicate, noncanonical, out-of-order, malformed,
foreign-domain, overflowing, truncated, trailing, or digest-inconsistent input.

Plan output has an independent ceiling of 740,018 records and 303,377,551 bytes, with at most
65,536 bytes per physical record. Those ceilings cover every logical fact selectable under the
default change admissions; a request using a larger internal admission can prepare successfully
but its explicit output fails rather than truncates if this separate boundary is exceeded. The
writer rejects targets at or below the project root, symlinks and non-regular targets; uses one
private sibling stage, synchronization, atomic rename, and parent synchronization; reports
`unchanged` for byte-identical existing output; and removes only its own failed stage. Output path
and publication status affect neither commitment. Planning and output failure leave repository
content and HEAD unchanged, and the plan file is never an apply input or accepted authority.

The executable sections `capabilities --section change`, `type`, and `expression` are the only
public vocabulary owner. The current compact subset includes module, record, variant, pure
function, constant, and test creation; field, case, and function-parameter addition; owner rename;
declaration move; complete function-body replacement; and exact owner deletion with either
`policy=reject` or `policy=owned-closure`. Types include the advertised primitive, named,
parameter, collection, result, stream, and function forms. Expressions include unit, boolean,
integer, text, local/constant references, conditional, sequence, and direct call. Broader typed
engine operations remain private until their compact workflows are complete.

`change.operation-field` discovery records expose every registered field's operation, name,
required status, and typed form for all 13 public operations. `change.field-form` records resolve
each emitted form token to its syntax; focused discovery also enumerates visibility, function
effect, deletion policy, and selector/reference values. `change.direct-operation` reports
`rename.owner` as the sole direct adapter and gives its exact plan and apply usage.
`change.precondition` and `change.precondition-field` records likewise expose the complete current
precondition set and field forms. The operation descriptor inventory is the sole field-set and
required/optional authority used by both decoding and discovery.

The current preconditions are exact owner existence, absence, name, and semantic parent; namespace
absence and exact owner binding; and an exact dependency binding containing package identity,
semantic revision, and logical package revision. `package` denotes the package parent; every other
parent is an exact owner. Owner-parent guards derive from canonical owner and exact parent records,
without treating the ownership witness as authority. Present namespace entries are checked against
canonical owner meaning before they can satisfy caller intent. Physical semantic-root digests,
encoded owner digests, derived summary digests, dependency-object digests, and retirement digests
are not caller intent; their predecessor record names are unknown input.

`$name` identifies request-local semantic owners and expressions; `@name` identifies notation-only
type fragments. `expression.argument` and `type.argument` records use zero-based contiguous indexes
to keep trees flat and deterministic. Exact local declaration selectors use `decl_...`, qualified
selectors use `MODULE/NAME`, and dependency declaration references use `pkg_.../decl_...`.
Selectors lower to typed exact references before validation; record spelling is never accepted
graph authority.

Allocation is deterministic from repository, exact base, and normalized authored intent;
operational budgets, idempotency, and intent are bound by the request commitment without perturbing
allocated identities. Plan and apply return equal complete symbol maps. Function-body replacement
and public owned-closure deletion use one bounded semantic-ownership selector. Raw JSON, the former
`--request`/`--request-file` grammar, and `--dry-run`/`--commit` are rejected before publication.
Large external value files, direct forms for the other 12 operations, and broad result export are
not yet exposed by this subset.

`delete.owner` accepts one exact live local non-expression owner and requires an explicit policy.
`policy=reject` is exact leaf deletion: any candidate-owned child rejects with
`change_delete_owned_children`. `policy=owned-closure` selects the root plus every transitive
semantic child from the post-mutation, pre-deletion candidate. Canonical aggregate membership and
canonical external parent relationships are the only ownership edges. Arbitrary references,
package dependencies, persistent-map reachability, objects, artifacts, and caches are not followed.
Bindings and expressions can enter a selected closure but remain invalid public roots.

Multiple roots form one deterministic typed-owner union; ancestor/descendant overlap is valid while
an exact duplicate root is not. Every root independently satisfies its policy. A request-local
owner, including a newly created descendant under a selected root, cannot be created and deleted in
one request. Only selected roots whose direct parent survives detach from that parent; every
deleted accepted owner receives one retirement bound to the exact base facts.

Any surviving local or foreign source whose final candidate relations still target the deletion
set rejects with `change_delete_live_reference`. The engine never rewrites, nulls, rebinds, repairs,
or deletes a referrer implicitly. An earlier explicit mutation may remove the relation, and an
additional explicit deletion root may remove the referrer, in the same reviewed request. The
logical plan lists every removed owner, retirement, surviving-parent edit, relation change,
validation owner, selected test, and impact reason selected by the existing contract; apply
reparses, reprepares, compares both token components, and publishes once or not at all. There is no
direct flag adapter for deletion. Missing policy, `cascade=true`, and policies named `cascade`,
`recursive`, `deep`, or any other alias are predecessor or unknown input and reject.

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
