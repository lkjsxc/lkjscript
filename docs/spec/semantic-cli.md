# Public CLI

Status: normative. The distributed `lkjscript` executable is the current process boundary. Exact
operations, grammar, request/response models, limits, diagnostics, authority effects, and security
nonclaims are owned by its public capability projection and generated into
[operations.md](../generated/operations.md) and the other guides in
[`docs/generated/`](../generated/).

## Product identity, capabilities, and dispatch

Exact `lkjscript --version` writes `lkjscript <product-version>` plus one newline, writes no stderr,
and exits successfully before project discovery or runtime setup. Every alias, value-bearing form,
project-scoped form, or additional argument rejects through `cli_usage`.

`lkjscript capabilities` projects complete public capabilities. Every successful default or focused
response begins with the product name/version and an opaque capabilities digest. Focused discovery
uses `capabilities COMMAND` or `--section SECTION`; `--known-capabilities DIGEST` may request an
unchanged result. The predecessor registry-named cache spelling and removed contract-table section
reject without aliases.
`--generate-docs DIR` and `--verify-generated DIR` are the sole generated-document owner.

The public operation set is closed: `capabilities`, `new`, top-level operational `data`, `status`,
`inspect`, `query`, `change`, normalized built-in `package`, `check`, `build`, `run`, and
artifact-runtime `serve` and `worker`.
An unknown command or option returns `cli_usage`. There is no universal namespace, compatibility
alias, marker-selected alternate dispatcher, or fallback parser.

Global `--project PATH` selects a project for repository operations. Otherwise discovery walks
ordinary ancestors without following symlinks. A predecessor `.lkjscript` marker produces the
stable predecessor-authority diagnostic before cache, output, or mutation work.

## Finite responses and errors

Every finite operation emits deterministic bounded compact line records. A classified finite
outcome keeps stderr empty. Capability success begins with its product and digest records; other
success begins with `result status=success|accepted|... command=...`. Failure begins with
`result status=failure` and includes stable diagnostic class, code, boundary, message, and safe
identity/location fields.

Compact output has independent byte and record limits. Growing results paginate with a logical
continuation or write to an explicit bounded file. Output is never silently truncated. Project
reads name the exact observed revision. Large artifacts, logical plans, and logs are referenced by
path and digest rather than repeated in stdout.

Exit classes distinguish source/semantic rejection, capability or cancellation, resource
exhaustion, corruption, infrastructure, stale base, and invalid candidate according to the
public capability projection. The same typed diagnostic classes cross repository, compiler, artifact,
runtime, and adapter boundaries.

## Project creation

```text
new DEST [--template minimal|command|http|nostr-relay-info] [--name NAME] [--relay-url URL]
```

The parent must be an ordinary existing directory. The destination must be absent and may not
traverse a symlink; an existing empty directory is still a conflict. Creation validates the name
and path before publication. Every nonempty recipe is typed authored intent whose operations have
the same normalized meaning and public compact representation as reviewed changes. It passes
through ordinary normalization, allocation, preparation, logical planning, impact/test selection,
and complete validation; there is no recipe-specific owner, ID, snapshot, or validator path. The
resulting repository has exactly one initial accepted revision. Canonical repository data and any
bounded auxiliary inventory are synchronized in a private sibling and made visible by one rename.
Failed creation removes only its own stage and never changes an existing destination.

`minimal` creates an empty dependency-free package. `command` creates one useful pure command
application with an exact built-in standard dependency, application module, private function,
component, port, target `main`, and graph-owned test. The implementation calls an exact public
standard declaration and deterministically returns text `"hello"`.

`http` creates one exact-standard-dependent HTTP application. Typed meaning owns private pure
`response-text` and `status-code` functions, a task handler with the normative structural HTTP
request and response types, one byte-stream requirement, an HTTP port, target `serve`, and one
status-code test. The handler returns status 200, no headers, and bounded bytes obtained from the
editable response function through exact built-in standard declarations. The request is unused in
this initial recipe.

Before its one visibility rename, HTTP creation also synchronizes a strict deployment descriptor at
`service.deployment.json` and an empty `generated/` directory. The descriptor names
`generated/application.lkja`, `serve`, `127.0.0.1:0`, and one byte-stream grant. It is separate
mutable operator authority and is not part of semantic state. Creation returns its descriptor,
recommended artifact output, target, runner, listener, and ordered next-action records. Minimal and
command creation do not report a deployment.

`nostr-relay-info` creates an exact-standard-dependent HTTP application with the existing inbound
byte-stream requirement and one `HttpClient` requirement. Its graph-owned `GET /relay-info` route
sends one `Accept: application/nostr+json` GET through that capability and returns the exact bounded
status-200 document only for the same case-insensitive base media type; all transport, status, and
media-type failures become deterministic local 502 without remote detail. Two graph tests cover
the stable pure response policy. The starter descriptor binds the client requirement to the exact
normalized endpoint and includes the same separate inbound listener and empty generated directory.

`--relay-url` is required exactly once for `nostr-relay-info` and rejected for every other recipe.
It accepts lowercase `wss` or `https`; `wss` normalizes to the `https` NIP-11 endpoint. Lowercase
`ws` or `http` is admitted only for a lexical loopback destination and normalizes `ws` to `http`.
Authority, explicit port, and path are preserved; user information, query, fragment, ambiguous
authority, malformed port, noncanonical escape, and unsupported scheme reject before the
destination is visible. This normalization does not implement WebSocket.

All four recipes are executable-owned typed operation lists, not source templates or a general
template language. Internal recipe construction avoids a pointless text round trip but may use only
operations owned by the public compact grammar. Unknown template spellings, including `web`,
`server`, and `service`, reject through `cli_usage` and are not aliases. Invalid template/option
combinations fail before any project destination becomes visible.

Creation through a copied candidate binary requires no Cargo, checkout-relative asset, network,
source file, database, container, or helper command. Release availability is current distribution
state and is intentionally not part of this normative contract.

## Operational data lifecycle

```text
data initialize --root PATH
data verify --root PATH
data backup --root PATH --output PATH
data restore --backup PATH --root PATH
```

This top-level operation manages only first-party operational application-data authority. It never
discovers, reads, or advances a project repository. Roots and backup/output paths are strict bounded
arguments; creation and restore require an absent destination, backup requires an absent output,
and all paths reject symlink/non-regular/path-traversal surprises. Initialize is idempotent only for
one matching valid store. Verify is read-only and walks the complete retained accepted closure.
Backup pins one exact head and create-new publishes a canonical logical backup; restore creates an
equivalent store under a new physical identity, verifies it, and makes the root visible once. There
is no repair, overwrite, SQL import, query shell, implicit descriptor change, or project-meaning
backup alias. Exact behavior and limits are specified in
[data-capabilities.md](data-capabilities.md) and executable discovery.

## Status, inspection, and query

`status` reports project, repository, package, revision, state/root, validation evidence, receipt,
and semantic counts. `inspect owner KIND ID [--package PACKAGE]` reads one exact typed owner.

One local function definition has the additional exact form:

```text
inspect owner KIND ID --detail definition \
  [--package PACKAGE] [--limit N] [--bytes N] [--continuation TOKEN]
```

`KIND` must be `pure_function` or `task_function`, the owner must be live in the selected local
package, and it must have a body. Summary inspection is unchanged. A dependency package, a
non-function, a missing or retired function, or a dependency implementation request rejects.
`body`, `full`, `raw`, `source`, `recursive`, and JSON spellings are not aliases.

Definition detail is a revision-pinned derived projection, not an authoring format. It discloses
the complete accepted function contract; every structurally owned expression, binding, pattern,
construction, projection, list, match, call, transaction, and resource form; exact references at
named declaration, package, interface, operation, requirement, field, parameter, and type
boundaries; and the resolved summary and validation facts already bound to those body records.
Referenced declarations are not recursively expanded. Storage bytes and paths, indexes, caches,
compiler or artifact operands, runtime handles, deployment state, grants, secrets, environment,
operational data, queue transition tokens, object bytes, and evidence paths are never consulted or
reported.

The projector pins one immutable repository view and validates the complete closure before emitting
success. Sections are ordered as the definition/revision header, function contract in semantic
field order, structural body preorder using canonical slots and explicit indexes, external
references by `(role, typed target)`, and validation facts in body order. Missing, foreign, shared,
cyclic, duplicate-slot, noncanonical, or summary/fact-inconsistent ownership is corruption rather
than partial output.

The complete logical definition admits at most 4,096 body records, 16,384 combined structural and
reference edges, 32,768 fact reads, depth 256, and 8 MiB of canonical logical encoding. Literal
fragments are at most 8 KiB. Existing compact page bounds remain 50 default and 10,000 maximum
emitted records, 1,536 minimum, 65,536 default, and 4 MiB maximum output bytes. Executable
discovery reports the separately derived canonical-record, ownership, persistent-map, object-store,
continuation, and output admissions.

Every page repeats repository, package, exact revision, function, projection contract, complete
definition digest and counts, ordering, and the page range. Stateless `icont_` continuations bind
all of those identities plus the section and exclusive resume record key; they carry no body,
frontier, cursor, cache, or process state. Resume reconstructs and validates the complete definition
from immutable authority. Only page item and byte budgets may change. Malformed, padded, truncated,
oversized, predecessor, foreign, stale, selector-mismatched, projection-mismatched, or impossible
tokens reject without writes. Projection records are unknown compact-change input and cannot
advance semantic authority.

Normalized query supports exactly:

```text
query owners [--kind KIND] [--limit N] [--bytes N] [--continuation TOKEN]
query find CLASS NAME [--parent OWNER]
query relations OWNER|package --direction incoming|outgoing \
  [--kind KIND] [--limit N] [--bytes N] [--continuation TOKEN]
query context OWNER --direction incoming|outgoing|both --depth N \
  [--limit N] [--bytes N] [--continuation TOKEN]
```

It reads canonical owners plus committed namespace and relation witnesses from one immutable
repository view. Stateless `qcont_` continuations bind repository, package, exact revision,
operation, normalized selector, order, and exclusive logical resume key. They do not persist a
cursor or session. Malformed, oversized, foreign, selector-mismatched, or stale tokens reject.

Context requires one live local owner, an explicit direction, and a canonical depth from 1 through
8. The root has depth zero. Breadth-first traversal expands only local owners whose minimum depth is
below the requested maximum. It selects every unique canonical incoming, outgoing, or both-direction
relation encountered during that expansion. Package and foreign-package endpoints remain relation
output boundaries: they receive no local owner record and are never expanded. Cycles, self-edges,
diamonds, and equal-length paths neither duplicate owners nor change minimum distances.

A complete context request admits at most 4,096 unique local owners, 16,384 unique selected
relations, and 32,768 visited relation witnesses. Canonical record decoding has the same 4,096-owner
bound. The executable derives and advertises the separate map and object-store admissions from
those logical maxima and the current bounded persistent-map/object encodings. It constructs the
complete admitted neighborhood before rendering a successful page. Exceeding any logical or
physical dimension is an atomic resource failure, never a partial neighborhood.

Context output first emits existing `owner` records with required `depth`, ordered by
`(depth, canonical owner key)`, then existing `relation` records in canonical forward-edge order.
Every page repeats exact total owner, relation, and expanded-owner counts plus separate map, store,
decode, witness-visit, selected-result, and rendered-byte work. The item limit counts emitted owner
and relation records. Byte and item page limits may change on resumption.

A context continuation additionally binds root, direction, depth, the owner/relation section, and
its exclusive canonical key. Every page recomputes the same complete neighborhood from the pinned
view; no token carries a frontier and no query file, cursor, index, cache, session, or process-local
state is created. Query-3 tokens and the removed repeated-seed, scalar work/fanout, `--continue`,
relation-filter, package-root, and JSON request forms reject without an adapter. Context success,
failure, exhaustion, corruption, stale continuation, and cancellation perform zero repository
writes.

Query work reports map/store/canonical/witness/output dimensions separately. Generic impact, fuzzy
search, historical query, predecessor JSON requests, and old callers/callees aliases are
unavailable.

## Reviewed change

Record input uses:

```text
change plan (--input RECORDS | --input-file PATH) [--output PATH]
change apply (--input RECORDS | --input-file PATH) --plan TOKEN
```

One direct adapter exists for exact owner rename. Its full usage and the exhaustive compact record,
type, expression, precondition, selector, and field vocabularies are capability-owned.

The second and only other direct change adapter is `extract.function`. It requires an exact base,
one existing local nongeneric function selected exactly or by an unambiguous module/name, one exact
live expression that is a proper structural subtree of that function, one request-local helper
symbol, and one absent helper name in the same module. Record and direct forms normalize to the same
typed operation. A request admits at most one extraction, and another operation may not edit the
target, selected closure, parent edge, inferred capture sources, or generated helper.

Extraction materializes the complete base definition before planning. The movable set is the
selected expression and every structural descendant under its unique incoming edge. Free function
parameters, lexical bindings, and match payloads become private-helper parameters ordered by first
canonical use and then typed owner key. Repeated source names receive one bounded deterministic
owner-derived spelling. Declaration, type, constant, operation, requirement, and package references
remain exact references. The helper result is the inferred exact resource-free subtree type. Its
effect is pure when the subtree needs no task authority; otherwise it contains the least exact
subset of caller requirements in caller order.

At most one free capability resource is admitted. It must have direct resource type, exact
acquiring-requirement provenance, exactly one consuming use inside the subtree, no later caller
use, and the existing private same-package acyclic affine handoff shape. It becomes the final
consume parameter and final local-read call argument. Resource containers or results, borrowed or
multiple resources, ambiguous or mismatched provenance, escaping bindings, transaction captures,
closures, generic or recursive targets, and cross-package helpers reject before review.

The rewrite retains the target declaration and every movable owner identity. It reparents the
selected root as the helper body, rewrites only captured local-reference records to generated
parameters, and replaces the exact parent slot with one generated direct call whose arguments are
effect-free local reads. The logical plan binds the base definition and moved-owner digests,
capture/use mapping, inferred contract and affine provenance, moved/preserved/changed/generated
owners, caller/helper body counts, ordinary semantic diff, impact, selected tests, and prepared
commitment. Apply rederives all facts against the exact base before the publication lock; stale,
invalid, malformed, cancelled, exhausted, or interrupted work does not advance authority.

Compact change records support `pure` and exact-requirement `task` function effects. Their public
dependency-closed stateful slice includes `add.requirement`, `set.function-contract`, structural
record types, lexical bindings, structural and nominal record construction/projection, typed lists,
variants and matches, exact built-in calls, requirement-scoped capability calls, and lexical data
transactions. Nested shapes use ordered flat fragment records and explicit parent/index
edges. Request-local labels are notation only; normalized authored intent owns stable allocation
and request commitment.

`type.capability-resource as=@TYPE interface=INTERFACE` authors one exact-interface resource type.
`add.parameter ... use=unrestricted|borrow|consume [requirement=REQUIREMENT]` authors canonical
parameter use and the optional exact function-resource binding. Omission means unrestricted and no
binding for an ordinary nonresource parameter. Resource operation parameters require explicit
borrow or consume and no binding. One private same-package nongeneric task function may instead
have one final direct resource parameter with `use=consume` and `requirement` naming the same exact
requirement in its effect and interface in its resource type. Its preceding parameters and result
must be resource-free, and only a direct named acyclic call may transfer the final resource owner.
Direct and input-file records lower to identical intent. Missing, extra, foreign, interface-
mismatched, borrowed, unrestricted, multiple, nonfinal, public, package, generic, pure,
cross-package, recursive, indirect, resource-result, and caller-reuse forms reject before plan
publication. Unknown predecessor type/use spellings also reject.

The higher-order slice has exactly three public spellings: `add.type-parameter` adds one ordered
stable parameter to a pure function; `expression.function-value` names one exact pure function and
receives all ordered `type.argument` children; and `expression.invoke` receives one function-valued
expression plus ordered `expression.argument` children. A function value is monomorphic after
complete explicit substitution, carries no capture or capability authority, and is evaluated once
before invocation arguments are evaluated left-to-right. Missing, excess, duplicate, foreign, task,
nonfunction, arity, and argument-type cases reject before publication. `function-ref`, `lambda`,
`closure`, and `apply` are not aliases. The dependency-closed data cutover also adds exact
`create.interface`, `create.external`, interface `add.operation`, operation parameters,
`set.requirement-contract`, and `replace.dependency`; these remain reviewed typed graph changes and
do not form a private builder.

The public exact-dependency and topology slice is:

```text
add.dependency package=PKG semantic-revision=REV package-revision=PACKAGE_REVISION
create.component as=$COMPONENT module=MODULE name=NAME visibility=private|package|public
add.port as=$PORT component=COMPONENT name=NAME type=TYPE function=DECLARATION
create.target as=$TARGET name=NAME component=DECLARATION port=PORT runner=command|http|interactive
```

`add.dependency` accepts only the exact current built-in binding after its immutable transport has
been staged through public package export. It performs no network, registry, ambient-directory, or
unchecked-file lookup; unavailable, duplicate, stale, foreign, and mismatched bindings reject.
`create.component` creates an empty component. Requirements and function-backed ports are separate
independently budgeted operations. An `add.port` explicit function type must exactly agree with its
function implementation. `create.target` binds exact component and port identities and checks that
the port is owned by the component, the function's requirement closure is present on that component,
and the port shape matches `command`, `http`, or `interactive`. For `interactive`, the exact shape
is `(Option<State>, SessionEvent) -> SessionDecision<State>` with one repeated closed ordinary
concrete state type; streams, capabilities, functions, secrets, unresolved parameters, and other
live values cannot enter retained state. Expression-backed ports, `SetTarget`, dependency removal,
arbitrary transports, and additional runner values are not exposed.

Request-local forward references work across the complete request. The four records participate in
the same strict decoder, canonical request commitment, allocation, logical plan, impact/relations,
validation/test selection, budgets, idempotent re-preparation, stale-base/token behavior, and atomic
publication as every other authored operation. Direct and input-file forms normalize identically.

Task effect requirements are an ordered exact set of component-local requirement references. A
new requirement names one exact built-in interface, an ordered admitted operation set, and separate
named resource limits. Pure functions cannot call tasks or capabilities or open transactions;
task capability calls must be admitted by both the function effect and component requirement.
Foreign domains, duplicate requirements, interface/operation mismatch, escaping transaction
bindings, nested transactions, shared owned fragments, unused fragments, and fragment cycles reject
before publication.

The strict record decoder rejects unknown or duplicate records/fields, invalid UTF-8 or escaping,
foreign identity domains, noncanonical order, overflow, missing edges, trailing input, and exhausted
admissions. Raw JSON and predecessor request/dry-run/commit forms are not alternate inputs.

Plan and apply both normalize to one typed authored request. Plan prepares a complete candidate and
returns a `plan_` token binding request intent and logical semantic effects. Optional plan output is
synchronized external evidence. Apply checks the request commitment before project access,
reprepares against the exact base, checks the prepared commitment, and calls the sole publication
boundary. A stale base, mismatch, invalid candidate, cancellation, or resource failure publishes
nothing.

An apply retry carrying the same valid idempotency key, normalized request, exact base, and reviewed
plan is reprepared against that request's historical base and reconciles to the one already accepted
revision. The retry path hides physical type objects introduced by the accepted child so append-only
storage growth cannot change the logical plan. `change plan` still observes the current revision and
rejects a stale base; idempotency is not a historical planning cursor.

When apply accepts, its semantic records are final before any compiler-cache handoff. A
`derived-cache` record reports `updated`, `not-available`, `not-attempted-replay`, or `failed`, plus
manifest/work or diagnostic data as applicable. `failed` still accompanies a successful accepted
semantic result; it is never mapped to a failed change.

## Built-in package

```text
package builtin inspect
package builtin query owners [--kind KIND] [--name NAME] [--parent OWNER] \
  [--limit N] [--bytes N] [--continuation TOKEN]
package builtin inspect owner KIND ID
package builtin export --kind transport|artifact --output PATH
```

Inspection reports exact package, semantic revision, logical package revision, transport,
interface, artifact manifest/bundle, counts, and byte sizes. Export strictly validates the embedded
material and creates one absent output file. Existing paths, symlinks, directories, and invalid
parents reject without replacement. No project, checkout lookup, mutable package source, or network
registry is consulted.

Owner query and exact inspection expose only the current implementation-free package interface.
They report canonical compact references, declaration type parameters, ordered parameters, result
types and effects, and interface operation signatures, idempotency, and external-visibility class.
Results use deterministic owner-key order and bounded output. A `bcont_` continuation binds the
exact package revision, normalized selector, order, and exclusive resume key. Malformed, oversized,
foreign, selector-mismatched, or stale tokens reject. No private body, intrinsic implementation
name, or artifact string scan is exposed or required.

`capabilities --section deployment` and
[`deployment.md`](../generated/deployment.md) project the deployment descriptor from the same closed
descriptor inventory exercised by strict decoding. They enumerate top-level and nested fields,
every adapter tag, required/optional status, scalar form, range, secret-name classification, and
nested limit shape without secret values. The generated
[`stateful-http-authoring.md`](../generated/stateful-http-authoring.md) walkthrough composes that
schema with current built-in references and compact grammar. The generated
[`nostr-relay-info-authoring.md`](../generated/nostr-relay-info-authoring.md) guide records the
closed recipe lifecycle, exact `HttpClient` references, response policy, and conservative defaults.
Both are guidance, not program authority.

## Check

`check` opens only typed meaning authority, validates its supported exact dependency closure,
prepares an exact-current or clean normalized compilation, links and strictly loads an artifact
bundle, then runs all graph-owned tests through production and canonical reference execution. It reports authority,
cache profile and unit work, artifact closure, aggregate test results, tier work, and differential
equality. It never advances `HEAD`.

A missing cache is ordinary clean work. A stale current revision is not reused. A corrupt cache is
reported through `cache=clean-recovery`, rebuilt, and cannot cause wrong semantics.

## Build

```text
build --output PATH
```

Build uses the same preparation and exact dependency closure as check and run. It emits only an
artifact bundle. Equal authority, dependencies, compiler compatibility, and options yield
identical bytes.

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
artifact bundle. Loading reads descriptor, artifact, environment, and named host resources only;
it does not discover a repository. Preparation resolves the exact target and grants before
readiness, and `artifact_digest` is the domain-tagged artifact bundle identity. `serve` admits only
exact HTTP or interactive topology. Interactive preparation reconstructs its relational state type
and every session bound before binding the listener; each connection belongs to one structured
parent whose finite callbacks cannot retain transport resources. Resident events are bounded and
resources are released on failure, cancellation, exhaustion, and shutdown. The HTTP/1.1 and RFC
6455 listener is plaintext and requires an external trusted encryption boundary when network
encryption is required. The local first-party data root is a trusted-host boundary and is not
encrypted.

## Removed behavior and non-goals

Project-scoped `draft`, `history`, general package staging, `review`, `backup`, `restore`, and
`doctor` are absent from discovery and dispatch. The top-level `data backup` and `data restore`
operations are distinct operational-data lifecycle commands, not compatibility aliases for removed
project behavior. Predecessor repositories and binary formats reject.

The CLI does not expose storage records as authoring syntax, arbitrary predecessor migration, a general
package manager, remote registry, source language, full owner-body projection, generic impact, an
agent daemon, inbound TLS, arbitrary network destinations, outbound WebSocket clients, NIP-01,
sandboxing, or multi-tenant isolation.
