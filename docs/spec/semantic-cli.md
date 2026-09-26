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

The public operation set is closed: `capabilities`, `new`, top-level operational `data`, native
installation `runtime`, `status`,
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
new DEST [--template minimal|command|http|web|nostr-relay-info] [--name NAME] [--relay-url URL]
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
It atomically includes `command.deployment.json` and an empty `generated/` directory. The
descriptor selects `main` from `generated/application.lkja`, has no grants or listener, and
omits execution/runtime policies. Its next actions include check, build and foreground run.

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
recommended artifact output, target, runner, listener, and ordered next-action records. Command
creation reports `listener=none`; minimal creation reports no deployment.

`web` creates one exact-standard-dependent browser application using embedded ordinary native
requests. The maintained UI renderer and its 19 tests become editable local `ui` and `ui-tests`
modules; `web` owns a stateless GET page, labeled name input, light/dark submit controls, response
headers and 13 application tests. The UI library owns the fixed HTML/CSS and escapes text. No
browser script is emitted. This is creation-time vendoring, not a separately selected UI dependency,
a mutable registry or a new privileged renderer. Existing projects do not silently inherit later
recipe edits. Independent exact package imports retain their separate authority.

The web recipe uses the same HTTP deployment layout and loopback listener above, with one stream
grant and no data, filesystem, outgoing-network or secret authority. It accepts `GET /`; query
values remain read-only URL input, not saved state or secret storage. Only exact `dark` selects
the dark theme. The first duplicate value wins, including an empty string; an absent or empty
value list selects the application fallback. Other methods and routes retain normal routing
behavior. Authentication, mutation, Origin policy and persistent state are not implied.

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

The closed five-recipe set is executable-owned, not a general template language. Existing typed
operation lists and embedded native requests both lower through the public change machinery.
For a native input, only its exact first-line request base is bound to the preceding private
accepted revision; declaration text is neither substituted nor generated. Subsequent native
inputs can resolve prior accepted local declarations. All intermediate revisions remain inside
an owned private lowering repository; the visible destination still has one initial acceptance.
Malformed native syntax, invalid types or conflicting declarations fail rather than bypassing
validation or exposing a partial project. Unknown spellings, including `Web`, `server` and
`service`, reject through `cli_usage` and are not aliases. Invalid template/option combinations
fail before any project destination becomes visible.

Creation through a copied candidate binary requires no Cargo, checkout-relative asset, network,
source file, database, container, or helper command. Release availability is current distribution
state and is intentionally not part of this normative contract.

## Native declaration units and editable drafts

Change input contract 24 adds `declarations.begin` / `declarations.end` around a parenthesized
`(units ...)` collection. All such blocks in one request are collected before typed resolution.
The executable's change registry owns the concrete grammar. Complete units cover modules,
records, variants, functions, constants, tests, interfaces, externals, components, requirements,
ports and targets, including HTTP routes and ordered generic/effect/requirement/value parameters.
Inline structural types and scoped type aliases are notation; nominal references retain their
exact declaration identity. Bodies use the existing structural expression forms, with lexical
parameter/binding names and optional explicit binder labels for otherwise shadowed references.
Existing compact records and structural blocks remain supported in the same request.

Creation and editing are distinct: `(FAMILY create NAME ...)` allocates an owner, while
`(FAMILY edit EXACT_OWNER NAME ...)` requires the exact owner, family, displayed name and parent
at the bound base. A displayed name never grants editing authority or causes upsert. A module
edit is a patch; unselected declarations survive. Complete edited contracts must retain every
existing child unless a precise `delete.owner` accounts for its removal. Positional contract
children retain order, and additions explicitly create and append. Keyed fields/cases use the
existing canonical identity ordering; the language adds no stored source-order dimension.
Rename, move, deletion and exact dependency changes use the existing precise operations and
complete-candidate validation. A supplied repository/package binding must agree with the base.

`change draft --owner OWNER [--owner OWNER]... --output PATH [--bytes N]` reads one accepted
revision. Selectors are exact local module, declaration or target IDs. Module selection expands
only actual owned declarations; selected declarations include their complete contracts and
bodies, and selected targets include their routes. References outside the selection remain
exact typed locators. Canonical reads use the maintained aggregate definition admission, share
cancellation, and charge expanded types and output growth before allocation. Unsupported or
unrepresentable content is an error; the complete result must pass native input admission before
it can be exposed. The byte maximum is positive and at most the complete change-input maximum.
Output is atomically created at an absent destination outside accepted project storage; existing
files are not overwritten. No draft operation changes accepted meaning.

Generated drafts bind repository, package, revision, selected identities and contract children.
They deterministically intern repeated structural types and generate readable typed aliases.
Generated aliases avoid every selected declaration and contract-child name, preserving lexical
and generic resolution even when authored names resemble generated names or owner-ID prefixes.
Original comments, formatting and alias spellings are not recoverable because they are not
canonical meaning. Draft files are disposable proposals and introduce no synchronization owner.
An untouched plan reports `outcome=unchanged`, no semantic change, no owner recreation and no
publication token. Such a request has no exportable plan; no-op apply retains the existing
publication rejection policy. Real body replacements retain declaration and unchanged signature
owners while following existing expression/binding replacement and retirement rules.

All edits lower into the shared typed authored request and existing review/publication lock.
Stale bases, missing repairs and altered reviewed meaning reject before publication. Accepted
idempotent retries keep their immutable original result. Authored codec 18 adds complete constant,
test and expression-backed port updates to existing graph meanings; requests using only earlier
intent retain their original codec identity and commitment. Graph, artifact, transport, application
data, deployment grants and runtime semantics have no format migration for this notation.

See the generated [creation and re-entry example](../generated/change-grammar.md) for public
commands, exact supplier staging and the complete native grammar.

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

Compatible historical repositories remain inspectable when the current semantic validator rejects
their accepted program. `status` retains the original `evidence` record and separately reports
`current-validation status=valid|invalid|unavailable`, the current validator, diagnostic code, and
proof origin. `invalid` denotes semantic rejection; resource or other incomplete admission is
`unavailable`. Neither status nor revalidation rewrites HEAD, parents, or the catalog. Current proof
can be reused within the process only for its exact immutable root and dependencies; an on-disk
producer assertion alone is not a current admission token.

Use the same `change plan --input-file REQUEST` and `change apply --input-file REQUEST --plan TOKEN`
to repair a newly invalid historical body. Request names and local preconditions are resolved from
authenticated canonical facts. Without a current valid base witness, the entire post-change candidate
must pass full validation and receive fresh proof before any pure candidate test or publication.
An ordinary compact dependency replacement can participate in that repair when its exact source and local
preconditions can be established. Supplier bodies receive independent current admission before
publication. Check, build, current export, and executable loading reject an
invalid current program. No unsafe option, implicit source migration, or alternate validator exists.
Historical idempotent retries retain their original result; incompatible retries report
`change_historical_request_incompatible` with that result and require a new reviewed repair.
Old prepared tokens require re-planning under the current capability and validation context.

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
transactions. Nested shapes may use ordered flat fragment records with explicit parent/index
edges, or structural expression bodies. Request-local labels are notation only; normalized authored
intent owns stable allocation and request commitment.

### Evolving callable inputs

`set.parameter-type parameter=OWNER type=TYPE` changes one existing callable input through the
ordinary reviewed typed change. `parameter` accepts an exact local `param_...` owner or a typed
request alias, including `reference.owner ... class=parameter`; `type` accepts the existing type
reference forms, including declared `@` types. Same-request symbols follow ordinary ordered
authored-change rules. Only the type changes: parameter identity, parent, name, positional
membership/order, use mode and exact requirement binding are preserved. `set.function-contract`
instead changes a function's result and effect. Neither operation grants execution authority.

The common parameter owner covers local functions, task functions, external functions and
interface operations. Existing intrinsic signatures, interface admission, resource use, lexical
scope and generic substitutions remain authoritative. A type parameter must belong to the
parameter's defining context, regardless of other declarations in the input. Imported immutable
parameters cannot be mutated. Complete final-candidate validation includes unused arguments and
unreachable syntax; a retained identity does not make old direct calls, bound callable types,
tests or ports compatible. Repair them explicitly in the same request, then review and publish
the complete candidate once. Unknown, duplicate or missing fields reject with located diagnostics.

For example, after binding an existing module, F64 function input and function-backed port as
`$module`, `$function`, `$input` and `$port` through `reference.owner`, this body changes the input
to a public structured reading. Prepend the actual `request base=REVISION` and the bindings:

```text
create.record as=$Reading module=$module name=Reading visibility=public
add.field as=$value record=$Reading name=value type=f64
add.field as=$enabled record=$Reading name=enabled type=bool
type.named as=@Reading declaration=$Reading
set.parameter-type parameter=$input type=@Reading
expression.block as=$body
  (if (field (local $input) $enabled)
    (field (local $input) $value)
    (f64 0.0))
expression.end
replace.body function=$function body=$body
type.function as=@Input result=f64
type.argument parent=@Input index=0 type=@Reading
set.port-contract port=$port type=@Input
```

Use `change plan --input-file PATH`, inspect the changed owner/type and affected contracts, then
apply the identical request with its plan token. Existing request commitments, exact owner
bindings, stale-review rejection, under-lock validation and immutable idempotent retry results
apply. Invalid, cancelled or exhausted edits cannot partially publish the new signature.

A library must evolve in its own repository and export a new exact package revision. An old
consumer pin retains the old contract. Replacing that dependency may require a single coordinated
edit to the consumer's input, body and port. No implicit coercions, wrappers or call-site repair
are introduced. Old artifacts, running processes, deployments and application data retain their
own identities and contracts; preserve matching older bundles and descriptors for recovery.
Restoring an old contract requires a new accepted successor, not rewriting history.

The compact compatibility identity changes when this record is exposed. The existing typed
`SetParameterType` encoding (authored codec tag 25) and graph/compiler/artifact/data formats do not
change. Older adapters reject the unknown spelling. Changed capabilities invalidate old review
tokens under the existing re-plan rules below, including accepted retries; they do not silently
reinterpret a reviewed request or allocate another accepted result.

Input evolution also relies on current validation of the exact affected ports and closed external
signatures. The validator contract detects corrections to those checks independently of storage
formats. Historical acceptance remains immutable: current validation may reject an old invalid
signature, and a complete reviewed successor can repair it without changing the parameter identity.
Old acceptance evidence cannot authorize present execution of invalid meaning.

### Structural expression bodies

A change request may define an expression root with one block:

```text
expression.block as=$body
  (let
    (binding value (type i64) (i64 2))
    (binding value (type i64) (call $add (local value) (i64 3)))
    (in (local value)))
expression.end
```

With its ordinary declaration/reference prelude, `create.function ... body=$body` or
`replace.body function=FUNCTION body=$body` attaches this root. Both `change plan` and `change apply`
accept blocks through the same input and input-file path. A saved request does not change meaning
until reviewed publication succeeds. The definition projection remains descriptive output and is
not accepted as request input.

The header has exactly the `as` field. The body has exactly one parenthesized expression over one
or more physical lines. `expression.end` is an otherwise empty standalone line, with surrounding
whitespace allowed. Stray or missing markers, nested headers, duplicate roots, extra header fields,
a second body expression, trailing tokens and an incomplete expression at the closing marker
reject. Marker spelling inside a quoted string is literal text. Outside blocks, the existing
compact-record framing, escaping and error behavior apply; compact responses remain strictly flat.

Inside a block, ASCII whitespace separates atoms and parentheses delimit expressions and syntax
wrappers. `;` begins a comment ending at the physical newline outside strings. Text and static-text
literals require double quotes and use compact escapes, including Unicode validation. Unescaped
physical newlines or control characters in strings reject. Names use the existing portable `Name`
rules without quoted identifiers. Typed selectors, type references and effect rows keep their
existing spellings and exact resolution. There are no bare literals, arithmetic precedence,
implicit applications or function-name lookup.

F64 is explicit in both notations: `type.f64 as=@number` declares a local type alias,
`expression.f64 as=$value value=-0.0` declares a flat literal, and `(f64 -0.0)` is its structural
form. Integer-form decimal tokens inside an F64 literal still have F64 type. The scalar
[language contract](language.md#binary64-computation) owns decimal/special-token parsing,
nearest-even rounding, signed zero and fallible conversions. Both notations preserve the same
normalized bits and reviewed meaning for equivalent decimal spellings. An I64 expression does
not become F64 without an explicit conversion.

`capabilities --section change` advertises all structural forms as `change.expression-syntax`
records and their framing, scope, ownership and application rules. The generated
[change grammar](../generated/change-grammar.md) retains the same executable-owned inventory.
Direct calls and function values accept zero or one each of `(types TYPE...)`, `(effects ROW...)`
and `(requirements REQUIREMENT...)`, in that order before value arguments. Omitted and explicit
empty clauses both mean an empty vector, without inference. Duplicate, unknown, misplaced or
out-of-order clauses reject. Nominal record and variant construction accept only the optional
`(types TYPE...)` clause. Concrete requirements and `parameter:$R` remain distinct. Existing
owners decide arity, constraints, visibility, empty-sequence validity, field uniqueness,
exhaustiveness and affine eligibility.

Both notations expose `transaction-outcome` with one explicit body type and exact ordinary
outcome/reason declaration and case references. The generated grammar owns their complete field
and clause inventory. Named references resolve those identities through the same typed inventory
as other nominal expressions; matching shape or spelling is insufficient. Complete inference
checks the body/result substitution before publication. The
[completion contract](effects-capabilities.md#completed-lexical-transaction-outcomes) owns the
result branches, legacy projection, exact requirement allowance and independent visibility rules.

`(local NAME)` selects the nearest binding in the current block's lexical environment. It never
falls back to a public label or an exact identity. `(local $parameter)` uses ordinary typed public
reference resolution. `(local (exact LOCAL_SELECTOR))` explicitly selects an accepted `param_...`
or `bind_...` identity, subject to the existing kind and scope checks. A bare name resembling one
of those identities is still a lexical name. Structural field projection uses `(field EXPR (name
NAME))`; `(field EXPR FIELD)` selects a nominal member.

Lets permit sequential and nested shadowing. Each initializer resolves before its new binder is
introduced, so a same-named earlier binding is available during that initializer. Every binder has
a distinct identity, and leaving the scope restores the enclosing binding. Unbound self or forward
locals reject. Match payload names exist only in their own arm body; a transaction binding exists
only in its body and retains an absent type annotation. No cross-block capture or implicit closure
is introduced. Public parameter, function, type, row and requirement aliases may still be declared
later in the whole request. Complete validation includes unused bindings and untaken branches.

A block exports only one ordinary request-local expression root. An outer change or a genuine flat
parent may consume it exactly once. Unused or multiply consumed roots reject. Nested expressions
and binders remain private even when user labels resemble generated symbol spellings. Outer
argument/type/binding/field/arm or other edge records cannot extend a block root or reach private
children. A block cannot splice a separately defined flat expression into its interior. Each
syntactic occurrence owns a distinct expression; only let provides evaluated-once reuse. Authored
field, map-entry, argument and sequence order, callee-before-arguments order, lazy branches,
explicit substitutions and lexical transaction meaning are preserved.

Both notations lower into `AuthoredChangeSet` and use the same complete candidate validation,
logical planning, token checks and atomic publication. For the same normalized authored tree,
base, operations, references, Names, annotations, budgets and publication options, flat and block
input produce identical canonical intent, request commitments and prepared candidates. A plan
from one notation can be applied with the other under the same executable. Comments, formatting,
empty application clauses and local label spelling do not alter the normalized inputs. Changed
literals, semantic Names, order, selectors or applications keep their commitment differences.
An exact-addressed flat graph may select an earlier binder after shadowing; bare structural names
select the nearest binder. The flat notation remains available for that exact editing.

Body replacement preserves the function declaration and unchanged value/type/effect/requirement
parameter owners. Replaced expression and binding owners follow the existing retirement rules.
A malformed or invalid block rejects its entire request without advancing `HEAD`; stale or changed
plans, cancellation, idempotent retries and historical-current repair retain their existing rules.
Requests containing F64 types or literals select authored intent generation 17. Transaction-outcome
requests without F64 retain generation 16 and its codec-16 commitment; compatible intent 14/15
retains the codec-15 commitment and original allocation identities. F64 meaning uses Graph 17,
compiler 13/bytecode 9 and Artifact 21, with a disjoint F64 type envelope. Base TypeObject 10 remains
unchanged; supported Graph 14/15/16 and Artifact 18/19/20 inputs retain their original meaning.
Older executables reject unsupported syntax or artifact generations before accepted publication
or adapter invocation. An older project reader may rebuild its disposable physical catalog before
reporting an unsupported HEAD; accepted HEAD and immutable pack bytes, and application data, remain
unchanged. Review tokens remain bound to their executable's capabilities and verifier, so a changed review context
requires a new plan.

Complete input remains bounded at 4 MiB. Outer flat-record bounds remain in force. Structural
token admission is 2,560,000 across all blocks, derived from the existing 10,000-record by 256-field
format dimensions. Each parenthesis and atom costs one token. Syntax admission is 1,000,000 nodes
across blocks, counting list wrappers and atoms. Resulting expression/binder/public identities keep
the existing 100,000-identity authored default. Complete expanded type/effect input, cache and copy work
is separately bounded at 1,000,000 nodes. The public request's impact ownership-step default is
2,000,000 (hard ceiling 10,000,000). A 1,024-deep expression chain exhausted the former 1,000,000
default between derivation and impact planning; the revised default admits that same useful
boundary. It changes the default review budget commitment, not canonical meaning or identities.
Change processing uses a joined worker with a 64 MiB stack for the already bounded recursive
semantic owners; framing, mixed-depth preflight and symbol collection are iterative. This keeps
the declared depth usable in debug and production builds. Checked charges precede growth. Physical line count is
not structural node admission, and budgets do not reset for each block. The authored identity
allowance remains unchanged.

Combined normalized expression depth, including flat ancestors around block roots, is admitted
before recursive construction; the root has depth one and syntax wrapper lists add no semantic
expression depth. The expression/type depth limits remain 1,024/256. These are finite preparation
limits, not additional language meaning. Syntax and representative reference, type, scope and capacity
diagnostics retain original input path, byte, line and column locations, with downstream owner/path
context where useful.

### Reviewed named references

Names address exact accepted meaning through typed request bindings. They do not import a package,
allocate an owner, confer mutation rights or grant execution authority. The compact forms are:

```text
reference.package as=$ALIAS source=builtin
reference.package as=$ALIAS package=PACKAGE_ID package-revision=EXACT_PACKAGE_REVISION
reference.owner as=$ALIAS package=local|$PACKAGE_ALIAS class=CLASS name=NAME [parent=$OWNER_ALIAS]
reference.owner as=$ALIAS package=$PACKAGE_ALIAS class=declaration owner=DECLARATION_ID
```

The package forms are exclusive. Builtin selection normalizes to the executable's immutable
embedded package, semantic revision and logical package revision before commitment, independently
of a project. Every selected package must agree with the request's explicit dependency closure.
The existing dependency owner still owns add/replace/delete; compact dependency records expose
staged add/replace. Binding never fetches, stages,
imports, upgrades or executes anything. An exact declaration alias excludes name/parent and admits
only a declaration exposed by that same selected interface.

| Class | Parent | Local coverage | Dependency coverage |
| --- | --- | --- | --- |
| `module` | Package root | Existing module | Rejected; interfaces expose no modules |
| `declaration` | Local module; foreign interface root | Existing declaration | Unique exported name, or exact exported declaration ID |
| `type-parameter`, `effect-parameter`, `requirement-parameter` | Declaring declaration | Existing named parameter | Only parameters exposed by the selected interface |
| `field` | Record declaration | Exact field | Exposed exact field |
| `case` | Variant declaration | Exact case | Exposed exact case |
| `operation` | Interface declaration | Exact operation | Exposed exact operation |
| `parameter` | Function, external declaration or operation | Exact parameter of that parent | Only exposed parameters; no local-read scope is conferred |
| `requirement`, `port` | Component declaration | Exact component member | Only legally exposed interface members |
| `target` | Package root | Existing target | Rejected |

An alias is accepted in every existing reference or local mutation selector position of its
corresponding type: declaration/type application, call/function value, field/case,
operation/requirement/effect row, type/effect parameter, existing function parameter, and port.
Local bindings also address owner/parent preconditions and existing-owner deletion. An
owner-absent precondition on a successfully resolved alias necessarily fails: selecting a missing
owner does not create an identity for the precondition.
Foreign aliases cannot select local mutations; ordinary subtype, ownership, lexical/generic scope,
visibility, effect and complete-candidate validation remain authoritative. Expressions, lexical
bindings, match payload bindings, HTTP routes, annotations and documentation are not new named
namespaces. Their existing access remains; literal names, text and documentation never expand.

Bindings may precede or follow uses. Reference aliases share `$` collision detection with creator
and expression symbols but have distinct typed values. All bindings, even unused ones, must resolve.
Package/parent edges must form a finite DAG, with exact same-package parents and the class rules
above. Multiple aliases may select one owner. Normalized selectors and their complete inventory
are canonical independently of alias spelling or binding order; meaningful edit order is preserved.
A binding-only request still lacks a semantic operation. Raw admission charges precede deduplication.

Local selectors resolve in the exact accepted base before any same-request rename/create/delete.
Dependency selectors resolve in the explicit candidate dependency selection regardless of record
order. No fallback to an older revision, hidden declaration, fuzzy match or global name table is
permitted. Duplicate exported declaration names are ambiguous because the interface carries no
module metadata; the exact declaration-alias form disambiguates without widening visibility.

Prepared review binds the complete canonical selector and exact-resolution inventory, including
class, owner kind/identity, exact parent/package root, package/semantic/logical revision and foreign
interface identity. Apply rederives it through preparation; presentation labels and displayed IDs
are not trusted resolution inputs. Legacy requests retain their intent bytes, commitment domain,
allocation and accepted-idempotency behavior. A changed embedded supplier requires an explicit
re-plan or exact-package request; it cannot silently alter a reviewed selection or reuse an
unrelated success receipt. Flat expression/type fragments, explicit application and expression
symbol uniqueness remain unchanged.

The prepared token also binds executable capabilities and the verifier registry. Changes to that
review context reject an older prepared token, including one for a legacy request, with
`change_prepared_plan_mismatch`. Re-plan the original request/base/idempotency inputs and review the
new token. Both plan and apply may reopen the matching accepted-idempotency owner's retained
historical base; publication returns the original acceptance instead of another revision. A changed
or invalid request never inherits success from a matching key alone. Missing historical inputs or
an embedded supplier mismatch require the classified recovery path; there is no implicit
conversion of an accepted request to a new supplier.

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

The higher-order slice uses these public spellings: `add.type-parameter` adds one ordered
stable parameter to a pure or task function; `expression.function-value` names one exact function and
receives all ordered `type.argument` and `effect.argument` children; and `expression.invoke` receives one function-valued
expression plus ordered `expression.argument` children. `expression.bind as=$bound callee=$callee`
uses the same ordered argument children to bind an exact parameter prefix. A function value is
monomorphic after complete explicit substitution; its immutable runtime prefix contains only
capture-safe values and checked pure or task callables. Callee and capture expressions run once in order;
binding never executes the target. Empty binding preserves the callable and complete binding
returns a zero-argument callable. Bare stored type parameters, secrets, streams, capability resources,
and aggregates containing them reject. Function signatures are callable leaves for capture safety.
Definition, relation, context, reviewed-change, and eligible extraction projections include binding
children. Runtime environments are absent from all semantic and external encodings.
Missing, excess, duplicate, foreign, undeclared task invocation,
nonfunction, arity, and argument-type cases reject before publication. `function-ref`, `lambda`,
`closure`, and `apply` are not aliases. The dependency-closed data cutover also adds exact
`create.interface`, `create.external`, interface `add.operation`, operation parameters,
`set.requirement-contract`, and `replace.dependency`; these remain reviewed typed graph changes and
do not form a private builder.

`add.effect-parameter as=$E declaration=FUNCTION name=E` adds a function-owned ordered effect
parameter. `effect.row as=@E` declares a row fragment, with ordered `effect.requirement` and
`effect.parameter` children naming exact requirements or effect parameters. Authored duplicate atoms
normalize to one canonical set member. A task declaration or `set.function-contract` accepts these
same row children. `type.task-function as=@Callback result=TYPE effect=@E` takes ordered
`type.argument` parameter children. A call or named function value receives ordered
`effect.argument parent=EXPRESSION index=INDEX effect=@E` records, exactly matching the callee's
effect arity. An empty row still describes a task callable. These fragments are ordinary authored
meaning, with complete definition/interface pagination and exact relation/impact participation.

`set.port-contract port=PORT type=TYPE` replaces a port's exact callable contract in the same reviewed
candidate as its implementation and dependent edits. Effect/signature changes require complete
candidate validation; body-only local certification does not authorize them. Omitted effect arguments
preserve previous request meaning only for zero effect arity. Project `run TARGET` remains
pure-only; foreground `run --deployment` uses exact artifact Command ports and checked grants.

The public exact-dependency and topology slice is:

```text
add.dependency package=PKG semantic-revision=REV package-revision=PACKAGE_REVISION
create.component as=$COMPONENT module=MODULE name=NAME visibility=private|package|public
add.port as=$PORT component=COMPONENT name=NAME type=TYPE function=DECLARATION
add.port as=$PORT component=COMPONENT name=NAME type=TYPE value=EXPRESSION
create.target as=$TARGET name=NAME component=DECLARATION [port=PORT] runner=command|http|interactive|batch|worker|test
add.http-route as=$ROUTE target=TARGET method=METHOD path=EXACT_PATH port=PORT
add.http-route as=$ROUTE target=TARGET method=METHOD pattern=PATH_PATTERN port=PORT
set.http-route route=HTTP_ROUTE method=METHOD path=EXACT_PATH port=PORT
set.http-route route=HTTP_ROUTE method=METHOD pattern=PATH_PATTERN port=PORT
```

`add.dependency` accepts an exact package/semantic/logical binding after its complete immutable
source closure has been validated and staged. It performs no network, registry, ambient-directory,
or unchecked-file lookup; unavailable, duplicate, stale, foreign, conflicting, and mismatched
bindings reject before publication.
`create.component` creates an empty component. Requirements and ports are separate independently
budgeted operations. `add.port` requires exactly one function reference or expression implementation;
its explicit function type must exactly agree with that implementation. `create.target` binds an
exact component and requires one exact port for every non-HTTP runner, but forbids that field for
`http`; an HTTP target instead owns its
nonempty finite `add.http-route` set. Each route binds an exact method and exactly one typed exact
path or whole-segment capture pattern to a component-owned function-backed HTTP port;
`set.http-route` changes that binding without replacing route identity. Pattern captures index the
handler's same-named ordered unrestricted `Text` parameter suffix, while an exact route retains the
single `HttpRequest` parameter. Duplicate match languages and incomparable overlaps reject, and an
exact or strictly more-specific selector wins without authored priority. For `interactive`, the
exact port shape is `(Option<State>, SessionEvent) ->
SessionDecision<State>` with one repeated closed ordinary concrete state type; streams,
capabilities, functions, secrets, unresolved parameters, and other live values cannot enter
retained state. Expression-backed ports, `SetTarget`, dependency removal, arbitrary transports,
and additional runner values are not exposed.

Request-local forward references work across the complete request. These records participate in
the same strict decoder, canonical request commitment, allocation, logical plan, impact/relations,
validation/test selection, budgets, idempotent re-preparation, stale-base/token behavior, and atomic
publication as every other authored operation. Direct and input-file forms normalize identically.

Task effect requirements are an ordered exact set of component-local requirement references. A
new requirement names one exact built-in interface, an ordered admitted operation set, and separate
named resource limits. Pure functions cannot call tasks or capabilities or open transactions;
task capability calls must be admitted by both the function effect and component requirement.
Foreign domains, duplicate requirements, interface/operation mismatch, escaping transaction
bindings, shared owned fragments, unused fragments, and fragment cycles reject before publication.
Both evaluators reject reentry into an active transaction on the same canonical requirement,
including aliases and callback-mediated entry. Separately granted stores may own independent
nested transactions; one store's abort cannot undo another store's completed publication.

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
storage growth cannot change the logical plan. Plan and apply reopen an exact matching accepted
request's retained base; other stale-base requests reject. Current revalidation cannot rewrite the
historical publication: an accepted retry reports its original receipt and revision-record identities.

When apply accepts, its semantic records are final before any compiler-cache handoff. A
`derived-cache` record reports `updated`, `not-available`, `not-attempted-replay`, or `failed`, plus
manifest/work or diagnostic data as applicable. `failed` still accompanies a successful accepted
semantic result; it is never mapped to a failed change.

## Offline packages and the embedded supplier

```text
package builtin inspect
package builtin query owners [--kind KIND] [--name NAME] [--parent OWNER] \
  [--limit N] [--bytes N] [--continuation TOKEN]
package builtin inspect owner KIND ID
package builtin export --kind transport|artifact --output PATH
package current export --kind transport --output PATH
package dependency stage --transport DIGEST --input-file PATH
package dependency inspect [owner KIND ID] --package-revision REVISION
package dependency query owners --package-revision REVISION \
  [--kind KIND] [--name NAME] [--parent OWNER] [--limit N] [--bytes N] [--continuation TOKEN]
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

Current export creates one code-complete immutable source container for the root and its exact
transitive closure. Dependency stage validates all canonical graphs, private bodies, interfaces,
and closed intrinsic signatures, then atomically installs closure-ready operational material without
changing semantic HEAD. Only reviewed `add.dependency` and `replace.dependency` install accepted
bindings. The logical review contains ordered `package-before` and `package-after` inventories
binding package, semantic revision, logical revision, and interface. Its bounded response summarizes
added/removed/changed members and interface changes; the plan commitment binds the complete file.
Apply rechecks the exact union, source availability, and validation contract under the publication lock.

Dependency inspection requires the exact logical package revision, including for staged closure
members. It uses the same public-interface projection, filters, budgets, ordering, and continuation
rules as the embedded supplier. Root inspection exposes direct dependency identities and closure
counts/commitment, not an unbounded catalog. Staging or inspecting a transitive member grants no
ambient visibility. `--project` selects the importing/exporting repository under ordinary parsing;
`package builtin` remains project-independent. Source containers include private implementations;
interface-only inspection is not a source-confidentiality claim.

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
build (--output PATH | --deployment PATH)
```

Exactly one selector is required, once only. `--output` retains the existing artifact-only
create-new behavior. `--deployment` derives an immutable deployment snapshot from one observed
operator descriptor and does not select or run it.

Both selectors use the same artifact preparation and exact dependency closure as check and run.
Equal authority, dependencies, compiler compatibility, and options yield identical artifact bytes.
The deployment selector additionally emits the descriptor described below.

With `--output`, publication is create-new: validate a bounded absent path and ordinary parent, write and
synchronize an owned sibling stage, create the visible file without overwrite, synchronize the
parent, and remove only the owned stage. Existing file/directory/symlink, symlinked parent, missing
or invalid parent, byte exhaustion, interruption, or publication failure leaves no partial new
artifact and preserves existing data. Build does not alter accepted authority.

With `--deployment`, relative input paths use the invocation directory, independently of the
selected project. Input and resolved output paths are bounded to 4096 UTF-8 bytes. Read one
ordinary non-symlink descriptor under the existing 1 MiB descriptor limit, reject linked parent
components and `..`, and strictly reject unknown/duplicate fields before project preparation.
The old `artifact` file need not exist and is not read. Prepare the current accepted graph using
the ordinary lifecycle, then statically admit the exact target, runner policy and grants against
that program. This does not run graph tests; use `check` explicitly before delivery. It does not
read secret values, open adapters, initialize data, bind listeners or probe runtime availability.

The artifact is `build-<BLAKE3 of complete artifact bytes>.lkja` in the original artifact's parent
directory. The new descriptor is `build-<BLAKE3 of complete emitted descriptor bytes>.deployment.json`
beside the observed descriptor. Only the `artifact` JSON value changes; other values and omitted
fields are preserved, although JSON formatting/key order need not be. The descriptor stays beside
its template so relative data, local-object and durable-queue roots retain their original meaning.
Reject output paths inside those declared local roots. Existing ordinary output parents are
required; snapshot publication creates no output directories and never rewrites the operator template.

New deployment descriptors use an owner-only POSIX stage (`0600`, further restricted by the
process umask) before any bytes are written. Reusing a descriptor also requires that its effective
owner is the current user and that no group/other permission bits are set. Reject broader or
foreign-owned existing descriptors before publishing the artifact, without chmod, chown or
rewriting them. Deliberate access by another runtime account requires separate operator action;
build never broadens access automatically. Artifact-only publication retains its existing mode
policy. These POSIX permissions are not protection against the same user or a privileged process.

Preflight both destinations before publishing either. An existing ordinary file is reusable only
after bounded exact-byte comparison; a matching digest-shaped filename alone is insufficient.
Different bytes, directories, links and invalid parents reject without overwrite. Publish the
complete artifact first, then the descriptor, using the existing create-new owner. Identical
cooperating builders can converge on the same exact pair. A late conflict or write failure can
retain a complete inert artifact; it never exposes a partial new descriptor or removes another
writer's file. Local owner mutation, deletion or hostile concurrent filesystem changes are not
prevented by the content-addressed naming contract. Rebuild detects observed conflicting bytes;
this is not a filesystem immutability or hostile-writer sandbox claim.

The existing `output` record describes the artifact. A `deployment` record reports the new
absolute `path`, observed `source`, bytes, visibility, durability and stage-cleanup, with
`admission=static-only`, `selection=unchanged`, `application-data=untouched` and `access=owner-only`.
Newly published files report `visibility=created`; exact reuse reports `visibility=reused-exact` and
`stage-cleanup=none-retained`. Durability is observed independently for each file. A failed
owned-stage cleanup is not hidden behind successful reuse. Output delivery failure does not undo
visible files; rerunning unchanged input can reuse the complete pair.

Explicitly pass the returned descriptor path to `serve --deployment` or `run --deployment`.
No running process, default deployment or accepted graph is selected or changed by the build.
A snapshot can itself be a template; unchanged inputs converge on the same pair. Keep older pairs
for deliberate selection. Selecting an earlier program does not roll back or migrate operational
data. Retention is explicit: build never garbage-collects older outputs, and disk exhaustion does
not authorize deletion or overwrite. `--output` still rejects even an identical existing file.

## Run

```text
run TARGET [--arguments JSON | --arguments-file PATH] [--result-file PATH]
run --deployment PATH [--arguments JSON | --arguments-file PATH] [--result-file PATH]
```

The two argument selectors are mutually exclusive and may occur at most once; omission
selects `[]`. `--arguments-file` reads one ordinary file relative to the invocation's
working directory, independently of the project or descriptor location. Final-component
symlinks and nonregular files reject. Metadata inspection and bounded nonblocking opening
admit at most the discovered application JSON byte limit (currently 1,048,576), including
whitespace; growth beyond that limit also rejects. This is a transport alternative for
input that cannot fit in the host's process argument vector. It does not increase JSON,
typed-value or execution limits, stream values, or interpret `-` as standard input.
Both selectors feed the same application decoder and typed input admission. Conflicting
selectors, duplicate options and missing option values reject before argument-file,
project or descriptor reads.

`--result-file` selects one create-new output independently of either argument selector.
It writes the exact complete typed JSON bytes, without a newline or re-encoding, under
the same application JSON limits. Default output remains the compact `execution.value`.
With a result file, `execution.result-file` replaces `value` and names the absolute
output; a separate `output` record reports `path`, `bytes`, `visibility`, `durability`
and `stage-cleanup`. This permits a bounded typed result larger than the compact
record's display capacity; it does not raise either format's limits.

The selector occurs at most once and accepts a nonempty UTF-8 path of at most 4096
bytes, including its resolved absolute form. Relative paths use the invocation directory;
`-` is an ordinary filename. Before context reads, secrets or adapters, inspect the absent
destination and its existing ordinary parent. Existing files, directories and dangling
symlinks, symlinked parents and `..` traversal reject. Inspection creates or reserves
nothing, and does not promise later absence, writability or disk capacity. Encoding and
owned execution cleanup complete before publication. The existing create-new output owner
rechecks the destination, writes and synchronizes an owned sibling stage, and exposes the
complete file atomically without replacing a competing output. It reports actual directory
durability and stage cleanup. Preparation, invocation or encoding failure publishes no
result file. A later conflict or write failure can occur after application effects;
neither failure nor missing stdout proves rollback or safe retry. A successfully exposed
file remains visible if later receipt delivery fails. Result-file publication and application
transactions are separate authorities.

Pure `run` execution records include `production-peak-call-frames`,
`reference-peak-call-frames`, `production-tail-transfers`, and `reference-tail-transfers` as
bounded unsigned integer scalars. Discovery's runners section describes their units, bounds, and
the derived pure-tail guarantee. Eligible pure graph calls retain constant control space under
the existing budgets. Eligible terminal task calls also replace their control activation while
preserving exact allowances, grants, resources and transaction continuations; pending non-tail
work retains ordinary call admission.
Fuel, allocation, cancellation, and argument order remain binding. Failed execution emits a
diagnostic without a successful value record or semantic `HEAD` change.

The same record exposes `production-` and `reference-` observations with suffixes
`input-admission-nodes`, `raw-result-admission-nodes`, `constructor-child-visits`,
`guard-descendants`, `classification-decisions`, `allocated-bytes`, `allocation-charges`, and
`collection-items`.
Discovery lists the finite fields and their units. Admission counts include rejected-boundary
progress in first-party failure receipts; they do not create a successful public result. Internal
eligibility checks use construction-controlled, preparation-bound classifications and never walk
admitted descendants. These observations do not assert constant total call cost: exact generic
types, effects, live capability authority and accounting still apply. Normative counting coverage,
independent fault sensitivity and the forwarding matrix are owned by the verification specification.

The argument adapter accepts one strict bounded JSON array and converts it to typed runtime values.
Application numeric admission retains complete decimal tokens and correctly rounds F64 integer,
fraction and exponent spellings. It deliberately preserves `-0` and `-0.0`. I64 admission remains
exact, including values above `2^53`, and rejects fraction/exponent spellings such as `1.0` and
`1e0`. Decimal overflow such as `1e400` rejects. Control schemas, manifests and security inputs
retain their strict integer policies. Byte/depth/item/string limits, duplicate fields, invalid
strings and trailing input reject under both policies. Typed JSON built-ins and project/artifact
commands use the same application policy.

Typed output checks the emitted JSON representation before returning success. The
root has depth zero; each array member or object value adds one level. Every array
member and object field consumes one item, including generated typed wrappers.
A Map entry `[key,value]` consumes three items and places its key/value two levels
below the Map array; nested values consume their additional items and levels.
Bytes' `$bytes` value and variants' `case`/`value` fields also count. String-byte
limits include object field names, case labels and base64 text. Whole-type
directional eligibility still visits unused arguments and inactive cases separately.

Before constructing JSON text, the encoder reserves its aggregate UTF-8 byte
length against the existing encoded-byte bound. Repeated strings, object keys,
case labels and generated base64 text all count before copying or allocation.
This lower bound can reject an already oversized result with
`normalized_json_output_bytes`; escaping, punctuation and scalar encoding still
receive the final exact serialized-byte check. It is a boundary-output bound,
not a whole-process memory cap or an internal language execution quota.

Current defaults are 1 MiB of encoded bytes, 100,000 JSON items, depth 128 and
1 MiB per decoded string. The pinned strict reader additionally admits at most
127 nested array/object containers; the output encoder respects that same guard.
`execution.foreground-limits` exposes these independent representation bounds.
For example, a flat primitive-key/primitive-value Map fits the item bound at 33,333
entries and exceeds it at 33,334. Byte-identical encodings within the bounds remain
compatible. Previously emitted over-limit shapes now reject with
`normalized_json_type` before result publication. This correction changes no graph,
artifact or typed-data encoding and imposes no new internal execution quota.

`capabilities --section runners` advertises the F64 arithmetic, comparison, parse/format,
conversion and transport policies in `execution.f64`, `execution.f64-conversion` and
`execution.f64-transport`. Ordinary standard declarations use the existing built-in package
discovery and exact signature inspection, for example `package builtin query owners --name
f64-parse-result`. These records add no execution operation or application-specific primitive.

Finite F64 output is a JSON number. NaN and infinity fail explicitly with
`normalized_json_nonfinite`, never null, strings or partial successful JSON. F64 is potentially
encodable and therefore passes type preflight; encoding checks the actual value. The entire
bounded result is buffered before success. JSON's finite boundary follows
[RFC 8259 section 6](https://www.rfc-editor.org/rfc/rfc8259.html#section-6) and does not limit
internal or typed-binary values. A transaction committed before later output failure remains
committed; missing output supplies no rollback or retry evidence.

Run selects an exact root target by current public name, requires command runner kind and a pure
entry, executes once in the normalized VM and once in the canonical reference interpreter, and
rejects disagreement. It emits the typed result plus bounded production/reference observations.
Effectful or non-command targets receive an exact unsupported/grants-required diagnostic; effects
are not duplicated. Run never advances authority.

The deployment form selects only the descriptor's exact artifact Command target, with a closed
pure or task function-backed port. It rejects positional targets, `--project`, duplicate/unknown
options and other runners before deployment setup, and never discovers a project. Arguments
default to `[]`. Strict artifact/descriptor, exact component/grant, directional input/output codec
eligibility and typed argument admission precede secret loading or adapter construction. All
branches of nominal result types must support output encoding, including unselected branches.
Intrinsic Option/Result boundary forms remain unsupported.

Deployment execution invokes production exactly once for either callable kind and reports
`execution-mode=production` and `verification=not-performed`. Application error variants remain
ordinary typed returned values. Success is staged within the existing response bounds until
encoding, joining invocation work and adapter shutdown succeed. Errors after task admission
conservatively disclose possibly visible earlier effects and cleanup evidence without automatic
retry or implicit whole-command rollback. SIGINT/SIGTERM select joined cancellation; when
completion and cancellation are already ready, completion wins. Once cancellation is selected,
later completion cannot turn that outcome into success.

Omitted execution policy selects the named trusted foreground profile: no cumulative instruction,
allocation, collection-work or invocation capability-call quota. Omitted runtime policy supplies
no invocation deadline, while cancellation and bounded cleanup grace remain. Supplied complete
numeric objects retain their legacy bounded meaning, including cumulative defaults of 256 MiB,
1,000,000 collection items and 100,000 invocation capability calls. Structural call/stack,
single-value/container, preparation/codec and adapter limits and canonical per-grant quotas
remain binding. Saturated observation counters are lower bounds; real storage arithmetic and
explicit quota overflow still reject. Project differential execution retains bounded defaults.
Resident descriptors require complete execution and runtime objects, but each cumulative quota
may explicitly be null. Fresh HTTP recipes select four nulls; omitted new quota fields in older
objects retain the legacy defaults above. The named trusted foreground profile is reported only
when all four cumulative quotas are absent. See [deployment security](deployment-security.md)
and the [resident-policy guide](../guides/resident-policy.md). An installed executable runs
independent bundles in separate processes;
`.lkja` files require that runtime and do not embed it.

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

Legacy top-level `draft`, `history`, general package staging, `review`, `backup`, `restore`, and
`doctor` are absent from discovery and dispatch. Native `change draft` is specified above.
The top-level `data backup` and `data restore`
operations are distinct operational-data lifecycle commands, not compatibility aliases for removed
project behavior. Predecessor repositories and binary formats reject.

The CLI does not expose arbitrary storage-record writes, automatic predecessor migration, a general
package manager, a remote registry, source/graph synchronization, generic impact, an agent daemon,
inbound TLS, arbitrary network destinations, outbound WebSocket clients, NIP-01, sandboxing, or
multi-tenant isolation.

The rank-one constraint authoring forms are:

```text
add.type-parameter as=$T declaration=DECLARATION name=NAME [constraint=none|capture-safe]
set.type-parameter-constraint parameter=OWNER_SELECTOR constraint=none|capture-safe
```

Both lower to typed changes with the same closed constraint set as canonical/JSON authoring.
The setter requires an exact type-parameter owner, preserves its identity and declaration order,
and constitutes an interface edit. Plan and apply validate the entire final candidate: strengthening
can invalidate callers, and clearing can invalidate a capturing body. One coherent request can change
constraint, body and callers together. Review reports affected dependents. Stale reviewed apply,
malformed constraints, cancellation and exhausted validation cannot partially advance accepted HEAD.
Owner queries, full function definitions and built-in/staged interfaces expose `constraint=none` or
`constraint=capture-safe`; definition continuations remain revision-pinned and reject after edits.

The declaration selector accepts pure and task functions, records and variants. Nominal applications use
`type.application as=@Applied declaration=DECLARATION` followed by ordered
`type.argument parent=@Applied index=INDEX type=TYPE` records. Positive arity is mandatory; use
`type.named` for zero-arity declarations. Record and variant constructors accept the same ordered
`type.argument` records on their expression symbol. Arguments remain explicit in discovery and
complete definitions.

`set.field-type field=OWNER_SELECTOR type=TYPE` and
`set.case-payload case=OWNER_SELECTOR [payload=TYPE]` edit exact nominal members. Omitting payload
makes a case payloadless. These operations, parameter constraints, and dependent constructor/signature
changes share normal plan/apply validation and atomic accepted publication.

## Explicit requirement-parameter authoring

`add.requirement-parameter as=$R declaration=FUNCTION name=R interface=INTERFACE` adds an
ordered function-owned parameter with an empty minimum operation set. Its stable ID is distinct
from concrete requirements, type parameters and effect parameters.
`requirement-parameter.operation parent=$R index=INDEX operation=OPERATION` supplies contiguous
ordered fragments that normalize to a canonical set. Interface and operation references are exact;
empty constraints remain interface-constrained.

`set.requirement-parameter as=%constraint parameter=PARAMETER interface=INTERFACE` replaces the
complete constraint through normal review. Operation children use %constraint as their parent.
Changing a constraint affects the owning signature, callers and exported exact package interfaces;
replacing a dependency rechecks consumers. Rejected edits leave accepted HEAD unchanged.

Direct calls and named function-value expressions supply contiguous
`requirement.argument parent=$call index=INDEX requirement=OPERAND` children. Concrete operands
retain existing exact or named-reference forms. Symbolic operands have an explicit  `parameter:`
prefix:  `parameter:$R` for an allocated or discovered parameter, or
`parameter:pkg_ID/reqparam_ID` for its exact identity. The prefix selects a typed reference kind;
failed concrete lookups never trigger reinterpretation. The same operands are accepted by
`effect.requirement`,  `expression.capability-call` and  `expression.transaction`.

Named discovery uses class  `requirement-parameter` with the exact owning function as parent.
Definition inspection includes formal order, interface and operation constraints, rows and ordered
applications. Relation queries expose parameter uses and requirement arguments. No implicit
requirement selection or ambient deployment binding follows from name resolution. Complete candidate
validation checks unused and unreachable applications as well as reachable bodies. Old request
commitments remain unchanged when the new forms are absent; successor commitment/decoder tags
explicitly identify new meaning.
