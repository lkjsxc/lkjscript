# Public semantic CLI

Status: normative. CLI contract version: 2.

## Executable command owner

The command registry in `src/platform/cli.rs` is the only command-reference owner. Run
`lkjscript semantic help` for its names and schema digest, and `lkjscript semantic help COMMAND`
for exact usage. Documentation may group workflows and show tested examples but must not maintain
a second exhaustive grammar.

The CLI is the ordinary interface for reading, changing, validating, reviewing, testing, building,
backing up, and restoring lkjscript meaning. A caller does not read packed objects, use Rust enum
layouts, or edit a maintained program file.

## Response contract

Every ordinary invocation writes one strict JSON value and no stderr on a classified result.
Success and semantic rejection share this envelope:

```json
{
  "contract_version": 2,
  "ok": true,
  "status": "success",
  "command": "semantic.status",
  "result": {}
}
```

`ok` says whether the requested semantic outcome succeeded. `status` is closed per command;
transaction and merge statuses appear at the outer level as well as in the typed result. An
unrepresentable request, corrupt store, or infrastructure failure returns `ok: false`,
`status: "failure"`, and one structured diagnostic. Stack traces, schemas, passing-test lists,
secrets, and child logs are absent by default.

The hard response limit is 4 MiB. Normal defaults are much smaller. Large bodies require explicit
`--body`, fields in a closed query, a continuation, or an out-of-band output file. An all-pass
package test returns aggregates only.

Process exits are 0 success, 2 rejected source/semantic request, 3 capability or cancellation, 4
resource exhaustion, 5 corrupt authority, 6 infrastructure failure, 7 stale base/head, and 8
invalid candidate graph.

## Revision pinning and selection

Read commands return the exact observed revision. `--revision REV` selects retained history where
the command supports it. Mutations always name `repository_id` and `base_revision`; there is no
implicit "latest" write.

Owners are selected by typed stable ID or bounded exact/name filters. IDs and names may appear
together, but names do not substitute for continuity. Zero result, multiple result, truncation,
stale continuation, foreign identity, exhaustion, and corruption are distinct.

`status` shows authority health. `orient` returns the smallest package/module/dependency/target map.
`owners`, `find`, and `show` select owners. `refs`, `callers`, `callees`, `type-uses`, and
`capability-uses` traverse exact semantic relations. `context` returns a task slice with inclusion
reasons; `impact` returns conservative affected meaning. `query` accepts the closed declarative
query request rather than host scripting.

## Query budgets and continuations

Every growing query has item, byte, work, depth, and fanout budgets. Defaults are 50 items, 64 KiB,
100,000 work, depth 4, and fanout 1,000. Hard maxima are 10,000 items, 4 MiB, 10,000,000 work,
depth 32, and fanout 10,000.

Ordering is independent of hash maps and physical shard position. Truncation is explicit.
Continuation handles bind query contract, exact revision, normalized query digest, and cursor,
plus a domain-separated integrity check. A changed query, changed revision, malformed handle, or
tampered cursor rejects; pagination neither omits nor duplicates an item.

The production query path uses a revision-bound disposable broad relation index plus 256-way local
owner/name shards. The independent path reconstructs owners and relations from canonical
roots/module tables. Missing or corrupt manifests or shards rebuild automatically. A warm exact
name or ID query reads the relevant local shard; an exact body query additionally reads only the
owning canonical module table.

## Mutation workflow

`id-allocate` creates opaque IDs in an explicit domain. `plan`, `validate`, and `apply` consume the
same strict transaction JSON from `--request` or `--request-file`. `dependency-stage` verifies a
graph-native artifact and makes its exact package objects available before the transaction that
adds or replaces the canonical binding. Staging cannot change HEAD.

Ordinary transaction results inline at most 64 canonically ordered affected owners, state the full
count and truncation, and include an exact `revision-show` expansion for the durable receipt. The
receipt retains the complete bounded affected-owner set out of band.

Draft commands create, inspect, append through transaction requests, rebase, publish, or drop a
non-executable draft. Draft publication rejects unresolved holes or conflicts. The current CLI
exposes the generic transaction form as the complete writer; convenience commands must compile to
that same form and may not become alternate writers.

`diff` classifies stable-owner additions, removals, renames, moves, and semantic modifications.
`merge` previews or atomically publishes a three-way identity merge from one exact base. A merge
with conflicts publishes nothing.

## Build, execution, review, and recovery

`build` lowers the exact graph revision and dependency closure directly to a deterministic packed
graph artifact. `test` compares prepared bytecode with the implementation-disjoint semantic
reference interpreter. `run` supports pure command, batch, and test targets; resident HTTP and
worker runners use deployment commands over the same prepared artifact path.

`text-project` and `export-text` write the same deterministic, span-free, explicitly
non-authoritative JSON review projection and return its digest and counts. That projection cannot
be applied or imported. `backup` and `export-bundle` write the same independently verifiable
canonical recovery bundle. `restore` verifies every entry before publishing a new repository
directory. `artifact-inspect`, `history`, `revision-show`, and `doctor --deep` expose bounded exact
evidence.

## Filesystem behavior

Global `--project PATH` selects a repository root or a descendant from which discovery succeeds.
Discovery rejects source-era markers when no current graph exists. Output publication rejects
symlinks and non-regular existing targets, writes a unique sibling stage, syncs it, renames it, and
syncs the parent. Equal existing bytes return `unchanged`.

## Machine help stability

The registry schema digest commits to command name, purpose, usage, and mutation classification.
Clients should cache help by this digest, request only the needed command detail, and reject an
unknown CLI contract. JSONL is reserved for an explicitly named future streaming command; no
ordinary command emits multiple values.
