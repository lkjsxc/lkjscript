# Status

Current snapshot: 2026-09-30 (Asia/Tokyo). This page describes current availability
and boundaries; historical measurements and failed attempts stay with their
[campaign owners](campaigns/) and [release records](release.md).

The owner-selected [language-first direction](direction.md) now prioritizes advanced
type/ownership/effect composition, region-based memory research and a shared runtime.
The exact-code shared service host and scoped capability borrowing are public.
Ordinary generic composition is public in v0.1.55. Recursive helpers and exact
cross-package resource contracts, exact deployment grant selectors and multi-resource
parameter suffixes and effect-generic resource callbacks are development
capabilities below; none is supplied by that public executable.
Advanced ownership/trait and region-memory work remains a research selection,
not a capability conferred by a roadmap or by sharing existing prepared code.

## Public binary release

**Public/latest is immutable [v0.1.55](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.55).**
It adds ordinary type-generic scoped borrowing and consumption, retaining recorded
history, the shared service host, typed forms and native web/editor workflows.
Release `398379899` was published on 2026-09-29 at 00:26:30 JST. The
[delivery observation](campaigns/202609290026.md#completed-v0155-publication)
identifies accepted producer `36419055364/1`, unchanged source `320dacc0`,
promotion `36442849973/1` and successful anonymous acquisition/installed smoke.
The earlier immutable releases and genuinely failed/withheld candidates remain unchanged.

Inspect the actual installed executable with `lkjscript capabilities`. Installing
a newer runtime does not silently replace accepted application definitions,
exact dependency selections, running processes or operational data.

## Development v0.1.60: unique byte concatenation storage

Ordinary immutable `Bytes` concatenation now reuses a uniquely held vector's spare
capacity and grows it geometrically when needed. Empty operands retain the other
payload. Shared prefixes, map keys and captures remain immutable; contents, ordering,
types and encoded bytes do not depend on the storage representation. New payload
allocations are admitted before copying, with cancellation checkpoints during copies.

The [campaign](campaigns/20260930-byte-buffer-reuse.md) records physical-identity,
allocation-boundary, independent-value and native generic/recursive consumer evidence.
The change may retain spare capacity and adds vector metadata, so it is not a claim
of universally lower memory use or measured speedup. Production byte-copy counters
cover this concatenation path only. This is not a new owned type, scoped memory borrow,
region feature, whole-program zero-copy guarantee or binary publication.

Source `0f0f4d92` reached main normally after all 26 full-profile gates passed freshly,
with zero reuse, no unrun gates and stable inputs. The retained final optimized product
also passed both native byte/map tests from an unrelated directory; the campaign
records the 1,024-concatenation observation without claiming a timing or RSS result.
A consolidated [v0.1.60 successor](releases/v0.1.60.md) is selected for candidate
acceptance. This includes the affine-proof correction below; public v0.1.55 is unchanged.
Candidate `36605595167/1` uses notes/reporting descendant `89241cd3`; its source was
read back and its last observed state is in progress, not accepted or published.
The candidate and final-archive cases remain separate from the local source proof.

## Development v0.1.60: affine validation admission

Affine checking now charges metadata reads as well as expression visits before work
occurs. All phases propagate budget exhaustion separately from invalid meaning and
from diagnostic-sink exhaustion; whole-snapshot checking no longer drops the affine
exhaustion result. Shared counters stop before overflow, and underlying read/cancellation
behavior remains intact. The [campaign](campaigns/20260929-affine-validation-work.md)
owns the predecessor failures and exact-source acceptance observations.

Validator feature 9 invalidates predecessor affine proof reuse. Language meaning,
canonical encodings and runtime resource/effect behavior are unchanged. A fixed request
budget can now exhaust on previously uncounted metadata work; this does not make the
program semantically invalid. This is not region ownership, a new execution quota,
a linear-time type-analysis claim or a binary publication.

## Development v0.1.60: shared immutable map keys

Byte and text map keys now retain the same immutable payload as ordinary values.
Conversion, key cloning, replacement and entry projection no longer copy those
payload buffers. Ordering, retained versions, type admission and serialized bytes
are unchanged. Raw inputs still count every logical occurrence, including aliases;
new map nodes, entries and projected records/lists still reserve their own storage.

The [campaign](campaigns/20260929-shared-map-keys.md) records the independent
pointer-identity, allocation-boundary and copied-binary consumer evidence. This is
a runtime representation improvement, not a new ownership or region feature, a
whole-program zero-copy claim, a timing/RSS result or a binary publication.

Source `48237793d718aa8d34338807b54357e35799b2e1` passed all 26 full-profile gates
freshly, with zero reuse and stable inputs. Its workspace tests passed 1,319 cases
with 29 existing ignored cases kept separate. The copied release executable passed
all four Text/Bytes project/detached consumer routes. The map-focused suite passed
79 tests, and predecessor pointer-identity witnesses demonstrated the removed copies.

## Development v0.1.60: control-flow local transfers

Ordinary local values can transfer at the final use on each branch, not only at the
last lexical occurrence. Bounded fixed-point liveness distinguishes loop-carried
values, redefinitions and edge-specific match bindings; dynamic external-call
continuations retain live values. Advisory exhaustion falls back to the existing
conservative proof, while global preparation limits and cancellation still apply.
Exact resource modes, runtime origin/class admission and canonical formats are unchanged.

The [continuation](campaigns/20260929-flow-local-moves.md) records predecessor failure
sensitivity, independent control-flow checks and native boxed-payload observations.
This is an execution optimization, not general memory borrowing, a region type system
or a whole-program zero-copy claim. It does not publish a binary release or change
running applications.

Corrected source `b6794d9cc4c72f13b377210ded8c54746343b83e` passed all 26 full-profile
gates freshly with zero reuse and stable input. Focused transfer tests passed 20/20;
the copied release executable passed the source-removal public CLI witness. Both
branches retained all 65 original recursive payload boxes. Precise and fallback
proofs preserve transaction slot-occupancy checks; the original failing witnesses
and the first frozen full run are retained separately in the campaign.

## Development v0.1.60: effect-generic resource callbacks

Exact-resource helpers may declare effect parameters for resource-free task callbacks,
while every borrowed/consumed resource retains an explicit concrete requirement. Explicit
empty and nonempty effect applications compose with ordinary generics, recursive/public
forwarders and repeated-borrow suffixes. A callback effect cannot replace the resource
binding; requirement-polymorphic resource signatures and escaping views remain unsupported.

The [literal library](../tests/fixtures/effect-resources-library.lkjc),
[consumer](../tests/fixtures/effect-resources-consumer.lkjc) and
[guide](guides/native-resources.md#effectful-callbacks-without-rebinding-resources) describe
normal authoring and detached execution. The
[campaign](campaigns/20260929-effect-resource-callbacks.md#accepted-source) records
source `f129c2fd`: all 26 full-profile gates passed freshly with zero reused results and
stable inputs. The workspace tests passed 1,302 cases, with 29 existing ignored cases
kept distinct. The exact retained final optimized producer also passed all six new
callback cases and the raw CLI handoff regression. Both source commits were normally
pushed to main and independently re-read from GitHub.

This source extension does not publish v0.1.60, replace running applications or establish
general memory borrowing or zero-copy execution.

## Development v0.1.59: terminal ordinary-value transfers

The prepared VM can move an ordinary value out of its local slot at a proved terminal
read instead of duplicating it. A conservative linear analysis excludes backward-edge
regions and retains earlier reads; exact resource borrow/consume rules and runtime
origin/class checks are unchanged. The [campaign](campaigns/20260929-terminal-local-moves.md)
owns the control-flow, physical-identity, independent-value and copied-binary evidence.
This changes neither canonical meaning nor package/artifact contracts and does not
provide general memory borrowing, region ownership or zero-copy payload processing.
It does not publish a release or replace running applications.

## Development v0.1.59: composing exact resource parameters

Direct graph-authored task helpers may take a contiguous final suffix of resources,
with a concrete requirement and borrow/consume mode for each parameter. Repeated
shared borrows are accepted; a repeated owner with any consuming occurrence rejects
in either order. Ordinary generic values, exact package contracts and synchronous
recursion compose with this boundary. Different resources can retain different
requirements even when their interfaces match.

The [literal two-queue workload](../tests/fixtures/resource-suffix.lkjc) and
[resource guide](guides/native-resources.md#multiple-resources-in-one-helper) describe
the public authoring and detached execution path. The
[continuation record](campaigns/202609291100.md#complete-accepted-source) records
source `9daca43e`: all 26 full-profile gates passed fresh, zero reused results,
with stable inputs. The exact retained optimized executable also passed the six
suffix cases and aligned legacy handoff case.

This is not general memory borrowing, an atomic transaction across resources,
asynchronous borrowing or a resource-return facility. Automatic function extraction
keeps its narrower eligibility. This source integration does not publish v0.1.59
or replace the public executable or running applications.

## Development v0.1.58: exact deployment grant selection

A grant may select the existing exact package/requirement identity instead of a
name. Distinct requirements named `jobs` can consequently retain independent
adapters, authority and durable roots. A legacy name still works when unique in
the complete component; name/ID duplicates and ambiguity-by-elimination reject
before secret lookup or preparation. Redacted observations retain both selectors.

This is a deployment composition extension, not general memory borrowing,
resource returns, requirement polymorphism or zero-copy payload processing.
Deployment discovery contract 6 changes no graph, artifact, package or data format.
The [continuation record](campaigns/202609290337.md) owns actual test/source results;
[the specification](spec/deployment-security.md#exact-requirement-selection) owns
the selection rules. No v0.1.58 distribution is claimed by this development section.
The independently selected v0.1.57 source and candidate remain unchanged.

Exact source `a571dc01` completed all 26 full gates freshly, with zero reused
results and stable inputs, in 836.675248583 seconds. The retained optimized
executable then passed all 20 generic/recursive/package-resource native tests
from an unrelated working directory in 57.10 seconds. The campaign records the
source/tree, raw receipts, four new public cases and trial corrections separately
from this evidence-only status update and any future distribution acceptance.

## Named canonical drafts in public v0.1.51

The public executable supports `change draft --module NAME`, `--declaration MODULE::NAME` and
`--target NAME`, alongside the existing `--owner ID`. A known declaration can be
drafted in one command instead of two namespace queries followed by an ID-selected
draft. Mixed/repeated selectors are bounded and deduplicated; names resolve and
are rechecked at one immutable revision under shared admission. The output still
binds exact owner and base identities. Names are not mutation authority.

The [source record](campaigns/202609270517.md) retains the named-selector evidence
and compatible-reader comparison. [v0.1.51 notes](releases/v0.1.51.md) and the
[completed delivery](campaigns/202609280123.md) own its accepted producer, original
assets and public verification. The linked [test-profile](campaigns/202609271910.md)
and [cold-lifecycle](campaigns/202609272130.md) continuations retain their earlier
failures and measured corrections; they are not pending delivery obligations.

The original [recent-history prototype](campaigns/202609280123.md#unintegrated-prototype--not-main-or-a-release)
remains preserved as an unaccepted historical attempt. The separate v0.1.53
continuation below corrects and extends it; neither v0.1.51 nor v0.1.52 includes
`inspect history`.

## Public v0.1.52

The published successor combines [typed form selection](guides/native-forms.md)
with coherent catalog/HEAD observation after recovery. Healthy reads remain shared;
recovery retains exclusive ownership until the caller finishes its observation,
rather than relying on atomic lock conversion. No storage schema changes or automatic
application upgrades are introduced. The [v0.1.52 notes](releases/v0.1.52.md) and
[recovery record](campaigns/202609280350.md) distinguish regression proof, complete
source acceptance and delivery.

Implementation `60f793eb` is integrated on main and passed all 26 full-profile
gates freshly, with zero reuse and stable inputs. Candidate
[`36344022937/1`](https://github.com/lkjsxc/lkjscript/actions/runs/36344022937)
uses that exact source and completed with `candidate_accepted`; its original
acceptance and terminal artifacts are unexpired. The initial apparent selection
read blocker was an operator API-path error (missing `/actions/`), not unavailable
permission. A correct read reconciled the existing v0.1.51 selection and terminal
release. Annotated tag `4f324e10` now selects the unchanged accepted product source;
its verbatim notes and remote object were compared before the existing release-only
selection was updated and read back. Promotion `36367325740/1` was dispatched once
from controller `8c00123f` and completed successfully. Its retained terminal is
`immutable_published_and_public_verified`, with latest selecting exact source
`60f793eb`. All three published assets match the accepted candidate inventory.
The [continuation](campaigns/202609281010.md) retains the original mistake and
corrected delivery evidence rather than treating a tag alone as publication.

## Public v0.1.53: recorded recent history

[`inspect history [--limit N]`](guides/native-history.md) projects existing accepted
revision and receipt records without a second history database, old-code execution,
rollback or application-data access. It captures one coherent HEAD/catalog view,
then releases the lock before bounded traversal. The default is 20 entries and
the maximum is 100; cumulative record/receipt admission also limits reads to 200
objects and 8 MiB. Truncation identifies an explicitly unread parent, not a cursor
or proof that the unvisited suffix is intact.

Each entry distinguishes recorded change intent, changed-owner counts and
selected/executed/passed tests. `current-validation=not-run` prevents old acceptance
from masquerading as a fresh check. Missing or inconsistent visited links fail the
complete response. Graph, artifact, package and operational-data encodings are
unchanged. Discover this capability with `capabilities --section inspection`.
Implementation `8570bfa4` is integrated on main and passed all 26 full-profile gates
freshly, with zero reuse and stable inputs. The [continuation](campaigns/202609281010.md)
retains the original failed run, corrected acceptance and copied-executable native
web exercise: 107 passing application tests and distinct old/new HTTP responses
without changing either immutable deployment. Candidate `36368771474/1` was
selected once from that exact implementation. It later failed the source profile:
19 gates passed; the closed-stdout fixture did not complete successfully within
its inherited 80 ms deadline. The [shared-host continuation](campaigns/202609281356.md)
retains that original failure and the scoped test correction. Its accepted successor
was published as v0.1.53; the earlier failed producer was not promoted.

## Public v0.1.53: shared service runtime

`serve --deployment A --deployment B` hosts a finite group of HTTP and interactive
services in one process. Exact complete artifacts share immutable prepared code;
configuration, secrets, grants, adapters, task accounting and cancellation remain
per instance. Different versions can coexist without a mutable global selector.
All static admissions precede live preparation, all listeners precede readiness,
and group termination joins each started service. Single-descriptor serving retains
its existing interface. This is not a subprocess launcher.

The [contract](spec/shared-runtime.md) defines the 64-instance and summed resident
capacity bounds, structural observations, code lifetime and failure domains. The
[campaign](campaigns/202609281356.md) owns pointer/reclamation tests and copied-binary
editor, version, isolation, busy-peer, WebSocket and startup-failure evidence.
There is no dynamic CLI load/reload/individual stop, cross-instance ownership
transfer or hostile-tenant sandbox. Encoded object-byte counts are not RSS, complete
private-memory accounting or measured speedup. Public v0.1.52 is unchanged.

Corrected source `7c6fec6b` is integrated on main and passed all 26 full-profile
gates freshly, with zero reuse and stable inputs. It also fixes a demonstrated
resident admission/stop race: accepted captures are counted before shutdown may
observe idle. All 16 focused runtime tests and five shared-service public tests
against a copied optimized executable passed. The [campaign](campaigns/202609281356.md)
retains both the original failure and exact-source proof. Producer `36386561026/1`
subsequently passed and its unchanged archive was promoted as immutable v0.1.53.
Final-archive shared-service proof and normal public verification are complete.

## Public v0.1.54: scoped resource borrowing

[v0.1.54](releases/v0.1.54.md) adds one private, same-package, exact-requirement
borrow parameter in the final argument position. Borrowing and nested reborrowing
preserve caller ownership; a borrowed helper cannot consume or retain the view.
Its accepted source `7e6101f8` passed 26 fresh gates. Producer `36395986112/1`
was accepted and subsequently promoted unchanged as the immutable public release.

## Public v0.1.55: ordinary type-generic resource helpers

[v0.1.55](releases/v0.1.55.md) composes ordinary type parameters with both borrow
and consume helpers. A native queue consumer decodes through an ordinary callback,
forwards the type through nested borrowing and returns ordinary data after final
consumption. Concrete resource interfaces/requirements, direct-call and escape
rules remain unchanged. Corrected source `320dacc0` passed all 26 full-profile
gates freshly, with zero reuse and stable inputs, and reached main normally.
The three public generic-resource cases also passed against a copied optimized
executable. The [campaign](campaigns/202609281735.md#accepted-corrected-source-and-mainline-delivery)
retains the original failed check and its scoped test correction. Candidate
`36419055364/1` was accepted. All three native cases passed against its final
archive executable before promotion. The same three assets are now immutable
public v0.1.55; no development v0.1.56/v0.1.57 source was substituted.

## Development v0.1.56: recursive resource contracts

[v0.1.56](releases/v0.1.56.md) admits direct and mutual synchronous recursion for
private same-package helpers with one final exact borrow/consume parameter.
Every body retains independent affine admission; cycles do not permit consuming
borrowed views, using moved owners, or escaping resources. Validator feature 5
separates this contract from earlier validation evidence.

The [continuation](campaigns/202609282124.md) records the old explicit cycle
rejection, its bounded implementation, independent execution/cleanup cases, and
literal native countdown and mutual-consumption cases. Focused gates pass,
including 100 VM/reference resource invocations and seven native public cases.
Implementation `d61dafaf3e1fc5d12cee6aaf11fdc4c239a961f3` passed all 26 full
source gates fresh, with zero reuse and stable inputs, in 815.368798381 seconds.
The same optimized development executable separately passed all seven native
public cases. The v0.1.57 successor below includes this recursion increment; no
separate v0.1.56 producer or public release is selected.
The original v0.1.55 producer `36419055364/1` subsequently passed final acceptance
and was promoted unchanged. The development recursion increment remains separate.

## Development v0.1.57: exact resource contracts across packages

[v0.1.57](releases/v0.1.57.md) admits public resource helpers and exported
forwarders with one final borrow/consume parameter bound to exact concrete
authority. Ordinary type parameters and recursive implementations compose across
package boundaries. The consumer's port explicitly declares imported requirements;
implementation bodies and unused supplier requirements never imply deployment grants.
Private/package members remain hidden, and same-name requirements are not aliases.

The [campaign](campaigns/202609290026.md) owns source acceptance, copied native
execution and delivery evidence. The literal library/consumer workloads exercise
transport staging, canonical drafts, detached two/three-package execution, failed
borrows, HTTP serving and pre-effect grant rejection. Affine feature 6 and qualified
task feature 2 invalidate predecessor validation evidence. General resource returns,
requirement/effect-polymorphic transfer and asynchronous borrowing remain unsupported.

Corrected source `3438e2ed` reached main normally after all 26 full-profile gates
passed freshly, with zero reuse and stable inputs. The same optimized executable
has passed all 16 generic, recursive and package native cases; its byte identity
survives the final test-only correction. Static grant matching additionally rejects
ambiguous imported requirements before secret lookup. The campaign retains the
original failed full run and the expanded independent visibility/ownership oracle.
Candidate `36461408308/1` completed with `candidate_accepted` at this exact source.
Its original successful evidence remains intact, but publication is superseded by the
[consolidated v0.1.60 selection](campaigns/20260930-byte-buffer-reuse.md#revised-publication-selection-consolidate-v0160),
which includes the later affine-proof correction and subsequent language/storage work.
The additional 16 final-archive native cases were not run in this resumption; no
v0.1.57 tag or publication is claimed.

## Current authority and maintained consumers

The accepted typed meaning graph is the sole editable program authority. Native
units and canonical drafts are proposals. Reviewed changes preserve intended
identities and publish atomically; invalid, stale or cancelled requests do not
partially publish meaning. Names, projections and package availability confer no
execution authority. Exact offline packages retain implementation, types, effects,
visibility and provenance after authoring sources are removed.

The [first native command](guides/native-command.md) and
[library guide](guides/native-library.md) cover authorship, review, tests and
detached use. [Standard](../packages/standard/README.md) owns ordinary functions
and exact platform interfaces. [lkjournal](../applications/lkjournal/README.md)
is the maintained HTTP/session/worker and durable-data consumer.

The owner corrected the 2026-09-29 request: complete Rust migration and hosting
at `lkjstr.lkjsxc.com` concern `lkjsxc/lkjstr`, not this language repository.
The existing language-first and eventual self-hosting direction remains in force.
Rust owns the kernel, parser, platform, contributor/release orchestration and the
no-Python classifier. The [native policy predecessor](../tools/native-policy/README.md)
remains an executable differential-test fixture, not a production fallback.
The [reference renderers](../tools/native-guides/README.md) execute an ordinary
language bundle. That is a supported implementation choice, not unfinished work
under a Rust-only mandate for this repository. Native
standard-library/application programs, authored test inputs and historical shell
evidence remain present; no all-files-or-dependencies-are-Rust claim is made.

The independent [Rust documentation site](guides/rust-site.md) serves eight explicitly
selected embedded documents, bounded server-side search and source-revision metadata.
It has no runtime dependency on the compiler, Node.js, Python or an external template
engine, and exposes no workspace directory or execution API. TLS, public DNS and
front-door traffic limits require deployment infrastructure. This presentation server
is separate from the still-native capability-reference generator.

## Native web starter

[`new --template web`](guides/native-web.md) creates a locally editable stateless
GET application, ordinary UI modules, tests and a loopback deployment from one
executable. The application composes typed controls; its library owns HTML/CSS
and emits no browser script. No Node.js, downloaded template or application-authored
HTML/CSS/JavaScript is needed. GET values are visible in URLs, not secret storage.

The UI is vendored at creation. Existing exact library imports do not automatically
upgrade with it. Same-structure scalar edits preserve internal expression/binding
identities as well as declaration identity; structural edits use ordinary replacement
and explicit review. The [editing guide](guides/native-web.md) and
[original evidence](campaigns/202609261118.md) retain that distinction.

## Native form selection

The ordinary form library adds `lookup(fields, name)` with typed `missing`,
`present(Text)` and `repeated` outcomes. Empty values remain present; decoded names
compare exactly, and even equal repeated values cannot silently select a winner.
The maintained editor uses it without adapting form values into HTTP headers.
Body decoding, actual header admission, domain validation and conditional saves
retain their separate responsibilities. The
[selection record](campaigns/202609280200.md) distinguishes source verification,
existing-runtime use and publication. Existing accepted suppliers and applications
are not silently upgraded. The public v0.1.52 executable embeds the updated
starter; an already installed older executable does not change with source examples.
The selection record retains its original failure, corrected source acceptance
and existing-runtime observations; the v0.1.52 record above owns combined-source
acceptance and completed public delivery.

## Native durable editor

[`new --template web-editor`](guides/native-web-editor.md), available in public
v0.1.50, creates an editable authenticated note application from shared ordinary
UI, form, editor and test definitions. The [separate-package editor](guides/native-editor.md)
remains another supported composition path. The strict [form library](guides/native-forms.md)
preserves ordered duplicates and bounded UTF-8; serialization is also ordinary lkjscript.

Data initialization and private credential configuration are explicit.
`notes.lkjdata` is an operational **directory**, not a source file or new data
format. GET is read-only; POST performs a conditional update in one completed
transaction. Conflicts retain the submitted draft and require explicit review.
Missing data, malformed values or revision exhaustion are not silently replaced.
The loopback default is not public HTTPS deployment; follow the guide's
Host/Origin/authentication and forwarding policy.

This is one bounded shared note, not per-user accounts, sessions, autosave,
multiple documents or a complete web framework. The
[starter evidence](campaigns/202609261600.md) and
[composition/browser evidence](campaigns/202609251211.md) have distinct scopes.
The [current public-reader observation](campaigns/202609270625.md) checks the
existing public v0.1.50 executable without rebuilding it.

## Deployment and process lifecycle

`build --deployment TEMPLATE` publishes an unselected immutable artifact and
sibling descriptor, with exact-byte reuse and owner-only descriptor publication.
The original descriptor, artifact, data and running process stay unchanged.
Relative roots retain their meaning. Use the returned path, not a guessed
content-addressed filename; the [build contract](spec/semantic-cli.md#build)
and [web guide](guides/native-web.md) own the workflow.

Command, HTTP, worker and structured-session deployments run standalone artifacts
without opening authoring projects. Public v0.1.50 joins owned resident work,
including cancellation and process-owned SIGINT/SIGTERM shutdown. A successful
stop must establish drained tasks and completed cleanup, not just a vanished
parent PID. See the [termination contract](spec/runtime-http-streams.md#process-owned-termination).

There is no automatic version selection, live reload, general supervision service
or data migration. Code rollback does not roll back data. A missing response or
cleanup failure does not prove that a previous effect rolled back or is safe to retry.

## Language and runtime boundary

Explicit generic records/variants, finite recursive nominal data, rank-one
type/effect/requirement parameters, named pure/task callables, prefix binding and
capture-safe retained values are supported. Persistent lists and ordered maps
share retained structure. Ordinary libraries provide composition, binary64
operations, typed data and transaction-completion reporting.

Pure project commands have independent evaluation. Live effects are not
differentially replayed. Callable effects, current allowances and deployment grants
are distinct checks. Cumulative work quotas, operational deadlines, cancellation,
structural bounds and live limits are not interchangeable. Allocated-byte
observations are not process RSS or live-heap guarantees.

Local data and durable queues support transactions, conditional updates, snapshots
and operational backup/restore. Independent capability effects can survive a failed
application operation. Inbound HTTP is plaintext; outbound HTTPS has exact endpoint
and explicit trust/address policy. Static linkage, safe Rust and quotas do not
establish hostile-code containment or multi-tenant isolation.

## Current limits and unproved properties

- Package dependencies are exact offline closures, not a network registry,
  mutable-name resolver or automatic upgrade service.
- Canonical drafting and development recent-history inspection are available.
  General historical queries, paging beyond the recent 100 entries, project
  backup/restore/doctor and general move/inline remain separate unimplemented
  facilities; operational `data backup|restore` does not provide them.
- Plan summaries and complete logical plans do not show literal before/after text.
  Retain and compare the unchanged draft with the edited proposal as well as its
  exact plan. A selected-test count is not proof of a passing test. Review tokens
  bind executable capabilities too; after an upgrade, re-plan the original request
  rather than modifying its base or reusing an incompatible token.
- Affine resources remain lexical and task-local, with requirement-bound
  borrow/consume helpers and ordinary type parameters in public v0.1.55.
  Development v0.1.57 adds public cross-package contracts with exact authority.
  General resource results, effect/requirement-polymorphic transfer and asynchronous
  ownership remain unproved. Name-based deployment cannot address two distinct
  obligations with the same name; one grant is never shared implicitly.
- Anonymous closures, automatic capture, generic inference, expanding nominal
  instantiation, JIT/AOT specialization, SIMD and a browser/Wasm backend are absent.
- Arbitrary outbound URLs/methods, outbound WebSockets, Nostr event signing and
  reconnect/replay policy do not follow from the relay-information recipe.
- The admitted 100,100-owner lifecycle and million-independent-module workload do
  not prove million-owner compilation, long history or operational-data scale.
  Graph/data garbage collection and compaction have not been selected.
- Data support targets one trusted local Linux host. Replication, consensus,
  encryption, additional binary targets, minimum-kernel guarantees and universal
  portability remain unproved.

Exact finite bounds are available through `lkjscript capabilities`; admission
limits are not demonstrated scale ceilings. [Performance evidence](performance.md)
retains slower cases as well as improvements, without universal speed or model-token
claims. The [roadmap](roadmap.md) contains revisable choices, not promises.

## Verification

[Verification obligations](spec/verification.md) distinguish standalone
`check full --fresh` (26 fresh gates), release-source acceptance (20 fresh gates),
and six behavioral owners against the executable extracted from the final archive.
Pinned userlands, installation/recovery and original readers are additional release
boundaries. Authenticated transfer and anonymous public acquisition remain distinct;
source tests or an uploaded artifact do not establish publication.

Campaigns preserve actual fresh, reused, failed, cancelled, skipped and pending
results. Reporting-only descendants never relabel an earlier tested source.
The [previous detailed snapshot](https://github.com/lkjsxc/lkjscript/blob/60f793eb2eaa38c7dc87adabcbe82d33342e06d8/docs/status.md)
retains the older milestone chronology; current instructions live in the linked
guides rather than being duplicated here.
