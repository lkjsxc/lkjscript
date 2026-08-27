# Public CLI

Status: normative. CLI contract 10 is the current process boundary. Exact operations, grammar,
request/response models, limits, diagnostics, authority effects, and security nonclaims are owned
by the executable registry and generated into
[operations.md](../generated/operations.md) and [contracts.md](../generated/contracts.md).

## Registry and dispatch

`lkjscript capabilities` projects the complete current registry. Focused discovery uses
`capabilities COMMAND` or `--section SECTION`; exact-known digests may request an unchanged result.
`--generate-docs DIR` and `--verify-generated DIR` are the sole generated-document owner.

The public operation set is closed: `capabilities`, `new`, `status`, `inspect`, `query`, `change`,
normalized built-in `package`, `check`, `build`, `run`, and artifact-runtime `serve` and `worker`.
An unknown command or option returns `cli_usage`. There is no universal namespace, compatibility
alias, marker-selected alternate dispatcher, or fallback parser.

Global `--project PATH` selects a project for repository operations. Otherwise discovery walks
ordinary ancestors without following symlinks. A predecessor `.lkjscript` marker produces the
stable predecessor-authority diagnostic before cache, output, or mutation work.

## Finite response and error contract

Every finite operation emits deterministic bounded compact line records. A classified finite
outcome keeps stderr empty. Success begins with `result status=success|accepted|... command=...`;
failure begins with `result status=failure` and includes stable diagnostic class, code, boundary,
message, and safe identity/location fields.

Compact output has independent byte and record limits. Growing results paginate with a logical
continuation or write to an explicit bounded file. Output is never silently truncated. Project
reads name the exact observed revision. Large artifacts, logical plans, and logs are referenced by
path and digest rather than repeated in stdout.

Exit classes distinguish source/semantic rejection, capability or cancellation, resource
exhaustion, corruption, infrastructure, stale base, and invalid candidate according to the
executable registry. The same typed diagnostic classes cross repository, compiler, artifact,
runtime, and adapter boundaries.

## Project creation

```text
new DEST [--template minimal|command] [--name NAME]
```

The parent must be an ordinary existing directory. The destination may be absent or empty and may
not traverse a symlink. Creation validates the name and path before publication, constructs the
complete Graph 5 repository in a private sibling, synchronizes canonical data, and makes it visible
by one rename. Failed creation removes only its own stage and never changes an existing destination.

`minimal` creates an empty dependency-free package. `command` creates one useful pure command
application with an exact built-in standard dependency, application module, private function,
component, port, target `main`, and graph-owned test. The implementation calls an exact public
standard declaration and deterministically returns text `"hello"`. Both recipes are executable-
owned typed construction, not source templates or a general template language.

Creation through a copied release binary requires no Cargo, checkout-relative asset, network,
source file, or helper command.

## Status, inspection, and query

`status` reports project, repository, package, revision, state/root, validation evidence, receipt,
and semantic counts. `inspect owner KIND ID [--package PACKAGE]` reads one exact typed owner.

Normalized query supports exactly:

```text
query owners [--kind KIND] [--limit N] [--bytes N] [--continuation TOKEN]
query find CLASS NAME [--parent OWNER]
query relations OWNER|package --direction incoming|outgoing \
  [--kind KIND] [--limit N] [--bytes N] [--continuation TOKEN]
```

It reads canonical owners plus committed namespace and relation witnesses from one immutable
repository view. Stateless `qcont_` continuations bind repository, package, exact revision,
operation, normalized selector, order, and exclusive logical resume key. They do not persist a
cursor or session. Malformed, oversized, foreign, selector-mismatched, or stale tokens reject.

Query work reports map/store/canonical/witness/output dimensions separately. Context traversal,
generic impact, fuzzy search, historical query, predecessor JSON requests, and old callers/callees
aliases are unavailable.

## Reviewed change

Record input uses:

```text
change plan (--input RECORDS | --input-file PATH) [--output PATH]
change apply (--input RECORDS | --input-file PATH) --plan TOKEN
```

One direct adapter exists for exact owner rename. Its full usage and the exhaustive compact record,
type, expression, precondition, selector, and field vocabularies are registry-owned.

The strict record decoder rejects unknown or duplicate records/fields, invalid UTF-8 or escaping,
foreign identity domains, noncanonical order, overflow, missing edges, trailing input, and exhausted
admissions. Raw JSON and predecessor request/dry-run/commit forms are not alternate inputs.

Plan and apply both normalize to one typed authored request. Plan prepares a complete candidate and
returns a `plan_` token binding request intent and logical semantic effects. Optional plan output is
synchronized external evidence. Apply checks the request commitment before project access,
reprepares against the exact base, checks the prepared commitment, and calls the sole publication
boundary. A stale base, mismatch, invalid candidate, cancellation, or resource failure publishes
nothing.

When apply accepts, its semantic records are final before any compiler-cache handoff. A
`derived-cache` record reports `updated`, `not-available`, `not-attempted-replay`, or `failed`, plus
manifest/work or diagnostic data as applicable. `failed` still accompanies a successful accepted
semantic result; it is never mapped to a failed change.

## Built-in package

```text
package builtin inspect
package builtin export --kind transport|artifact --output PATH
```

Inspection reports exact package, semantic revision, logical package revision, transport,
interface, artifact manifest/bundle, counts, and byte sizes. Export strictly validates the embedded
material and creates one absent output file. Existing paths, symlinks, directories, and invalid
parents reject without replacement. No project, checkout lookup, mutable package source, or network
registry is consulted.

## Check

`check` opens only Graph 5 authority, validates its supported exact dependency closure, prepares an
exact-current or clean normalized compilation, links and strictly loads artifact 10, then runs all
graph-owned tests through production and canonical reference execution. It reports authority,
cache profile and unit work, artifact closure, aggregate test results, tier work, and differential
equality. It never advances `HEAD`.

A missing cache is ordinary clean work. A stale current revision is not reused. A corrupt cache is
reported through `cache=clean-recovery`, rebuilt, and cannot cause wrong semantics.

## Build

```text
build --output PATH
```

Build uses the same preparation and exact dependency closure as check and run. It emits artifact
contract 10 only. Equal authority, dependencies, compiler contracts, and options yield identical
bytes.

Output publication is create-new: validate a bounded absent path and ordinary parent, write and
synchronize an owned sibling stage, create the visible file without overwrite, synchronize the
parent, and remove only the owned stage. Existing file/directory/symlink, symlinked parent, missing
or invalid parent, byte exhaustion, interruption, or publication failure leaves no partial new
artifact and preserves existing data. Build does not alter accepted authority.

## Run

```text
run TARGET [--arguments JSON]
```

The argument adapter accepts one strict bounded JSON array and converts it to typed runtime values.
Run selects an exact root target by current public name, requires command runner kind and a pure
entry, executes once in the normalized VM and once in the canonical reference interpreter, and
rejects disagreement. It emits the typed result plus bounded production/reference observations.
Effectful or non-command targets receive an exact unsupported/grants-required diagnostic; effects
are not duplicated. Run never advances authority.

## Serve and worker

`serve --deployment DESCRIPTOR` and `worker --deployment DESCRIPTOR` are resident artifact-runtime
operations, not current graph build commands. Their descriptors reference an explicitly isolated
artifact-10 bundle. Loading reads descriptor, artifact, environment, and named host resources only;
it does not discover a repository. Preparation resolves the exact target and grants before
readiness, and `artifact_digest` is the domain-tagged artifact bundle identity. Resident events are
bounded and resources are released on failure, cancellation, exhaustion, and shutdown. The
plaintext HTTP and `NoTls` PostgreSQL adapters require an external trusted encryption boundary.

## Removed behavior and non-goals

`draft`, `history`, general package staging, `review`, `backup`, `restore`, and `doctor` are absent
from discovery and dispatch. They are not compatibility aliases and have not been silently moved to
another spelling. Graph 4 repositories and predecessor binary contracts reject.

The CLI does not expose storage records as authoring syntax, arbitrary Graph 4 migration, a general
package manager, remote registry, source language, context traversal, an agent daemon, TLS,
sandboxing, or multi-tenant isolation.
