# Status

Current snapshot: 2026-09-28 (Asia/Tokyo). This page describes current availability
and boundaries; historical measurements and failed attempts stay with their
[campaign owners](campaigns/) and [release records](release.md).

## Public binary release

**Public/latest is immutable [v0.1.51](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.51).**
It adds named canonical drafts and retains native web/editor workflows, identity-preserving scalar edits,
immutable deployment builds and corrected joined resident lifecycle. The
[delivery record](campaigns/202609280123.md) identifies the original producer,
unchanged assets, promotion and anonymous exact/latest verification. The withheld
v0.1.49 candidate was never a public release; its tag and original failures remain
unchanged.

Inspect the actual installed executable with `lkjscript capabilities`. Installing
a newer runtime does not silently replace accepted application definitions,
exact dependency selections, running processes or operational data.

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

## Development v0.1.52

The selected successor combines [typed form selection](guides/native-forms.md)
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
acceptance and terminal artifacts are unexpired. Publication is blocked at reading
the existing release-only selection variable: the current connection returns HTTP
404, which does not distinguish absence from unavailable permission. No variable
was created, no permissions changed, and no v0.1.52 tag or promotion selected.
The [continuation](campaigns/202609281010.md) owns this observation and exact
resumption boundary. This is **not a published v0.1.52 binary**.

## Development v0.1.53: recorded recent history

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
Source acceptance and integration are tracked at the [continuation](campaigns/202609281010.md);
this development feature is absent from the public v0.1.51 executable.

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

Native [reference renderers](../tools/native-guides/README.md) and
[repository policy](../tools/native-policy/README.md) are real ordinary-language
consumers. Rust still owns the supported kernel, parser, platform and
contributor/release orchestration. This adoption is not compiler self-hosting.

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
are not silently upgraded. Public/latest remains the unchanged v0.1.51 binary;
its embedded starter does not change with the current source examples.
The selection record retains its original failure, corrected source acceptance
and existing-runtime observations; the v0.1.52 record above owns combined-source
acceptance and candidate progress.

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
- Affine resources remain lexical and task-local, with the supported private,
  requirement-bound consume handoff. General resource results, borrowing signatures,
  cross-package transfer and asynchronous ownership remain unproved.
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
