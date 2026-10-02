# Status

Current snapshot: 2026-10-02 (Asia/Tokyo). Public availability, source acceptance
and running deployments are separate. Detailed chronology and original failures
belong to the linked campaigns, not a second list of pending obligations.

## Public binary: v0.1.64

**Public/latest is immutable [v0.1.64](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.64).**
Release `401655238` was published on 2026-10-02 at 17:39:02 JST. Original accepted
producer `36966111016/1` supplies source
`0048ae1ee2e4678b409c782e02044b038bf60052`. Promotion `36984555849/1` completed with
`immutable_published_and_public_verified`: authenticated selection, unchanged-asset
publication, anonymous exact/latest acquisition and installed lifecycle verification.
The independently read public three-asset inventory matches the accepted originals.
The [publication record](campaigns/20261002-owned-choices.md#completed-v0164-publication)
retains the annotated tag, controller/product identities, terminal and exact assets.

This release consolidates the unpublished 0.1.61–0.1.64 language work. Those
intermediate development identifiers were not separate public releases.
All `A.B.C` components remain equally opaque identifiers, not compatibility,
stability or feature promises. Discover actual support with `lkjscript capabilities`.

The public executable supplies:

- **Owned storage and libraries.** ByteBuffer, independently sealed OwnedI64Cell,
  first-order Owned parameters and exact static implementation witnesses compose
  producer, transformer and consumer libraries. Pure helpers support consuming
  parameters, direct owned results and scoped synchronous reads.
- **Owned products and recoverable choices.** Fixed named products combine owned
  payloads with closed ordinary metadata. Complete decomposition transfers fields;
  metadata reads preserve the parent owner. OwnedChoice combines ordinary success
  with a still-owned rejected payload and requires complete consuming case analysis.
  Ordinary Option/Result and unrestricted containers do not become ownership escapes.
- **Immutable byte ranges and bounded admission.** Production byte views share
  backing storage; explicit byte copying detaches retained backing. Storage creation
  accounts for modeled control metadata, and ownership proof state copying/joins
  consume finite proof work. These are not universal zero-copy, RSS or speed claims.

See the [release notes](releases/v0.1.64.md), [buffer guide](guides/native-byte-buffer.md),
[generic guide](guides/native-owned-generics.md), [product guide](guides/native-owned-products.md),
[choice guide](guides/native-owned-choices.md) and [byte-range guide](guides/native-byte-ranges.md).

Public v0.1.64 retains the earlier recursive cross-package exact resource helpers,
resource-parameter suffixes, effect-generic resource-free callbacks, exact deployment
grant selectors, ordinary terminal-value transfers and reusable immutable-value storage.
The shared in-process host, native web/editor templates and offline libraries remain
available. Immutable v0.1.60 and genuinely failed 0.1.61/0.1.63 producers are unchanged.

Graph 20 retains supported historical graph readers. Compiler 18, bytecode 14 and
artifact 25 require rebuilding predecessor derived artifacts from retained accepted
meaning. Authored request 23, compact discovery 27 and semantic validator 21 expose
the relevant public contracts. Operational data and running services are not migrated.
Installing this release does not replace accepted applications or exact dependencies.

## Development v0.1.65: same-task owned calls

Named task helpers can consume and return owned buffers, cells, products and choices
within one invocation. First-order Owned parameters and exact static implementation
witnesses compose with tasks. Ordinary arguments precede owned memory, and exact
capability resources form a final suffix. Task memory parameters are consume-only;
pure synchronous helpers still provide scoped read borrowing. Resource grants remain
independent, and even an empty-effect-row task requires explicit task execution.

The [specification](spec/owned-task-transfers.md) and
[continuation](campaigns/20261002-task-owned-transfer.md) distinguish this call boundary
from asynchronous handoff, channels and a shared scheduler. Semantic validator 22
invalidates old proof reuse; graph, request, instruction and artifact wire formats
are unchanged by this increment. No ambient shared memory origin was introduced.

Source `fbac03b256b351fba44cd94f1960acf042f000c7` passes all **26 full-source gates
freshly**, with stable inputs, zero reuse and no unrun gates. Its all-target/all-feature
workspace suite passes **1,511 tests**, zero failures and 29 existing ignored cases.
The final optimized copied host separately passes all **25** selected ownership,
task-owned, byte-range and resident-policy cases outside the checkout with a cleared
environment. Normal fast-forward delivery and an independent GitHub ref read confirm
that exact accepted source on main. The [acceptance record](campaigns/20261002-task-owned-transfer.md#accepted-source-and-mainline-delivery)
retains exact identities, failed predecessors and the unexecuted mismatching copy;
reporting-only descendants do not change the source actually tested.

Coverage includes both evaluators, exact values and allocation counts, four-package
source-free execution, explicit witnesses, raw boundaries, unused-loan mutations,
traps, cancellation/quota cleanup and consistently rehashed untaken duplicate transfers.
The original failed full run is not relabeled: stale test expectations and resource
diagnostic precedence were corrected before fresh acceptance. Public/latest remains
v0.1.64. No 0.1.65 binary publication or running-service deployment is implied.

The next distinct boundary is a bounded structured in-process handoff: reserve
capacity before irrevocable acceptance, return ownership only on an actual refusal,
and define one cleanup custodian through cancellation and receiver failure.
The [decision and finite model](decisions/20261002-structured-owned-transfer.md)
are design evidence, not an implemented channel or a parallelism/fairness proof.

## Program authority and maintained consumers

The accepted typed meaning graph is the sole editable program authority. Native
units and canonical drafts are proposals. Reviewed changes preserve intended
identities and publish atomically; invalid, stale or cancelled changes do not
partially publish meaning. Names, projections and package availability confer no
execution authority. Exact offline packages retain types, effects, visibility,
implementation and provenance after authoring sources are removed.

The [first command](guides/native-command.md) and [library guide](guides/native-library.md)
cover authorship, review, tests and detached execution. The
[standard library](../packages/standard/README.md) owns ordinary functions and exact
platform interfaces. [lkjournal](../applications/lkjournal/README.md) remains the
maintained HTTP/session/worker and durable-data consumer.

Rust implements the current kernel, platform and contributor/release tools; native
standard/application programs and reference renderers remain ordinary lkjscript.
Eventual complete self-hosting remains the [long-term direction](direction.md).
The separate [Rust documentation site](guides/rust-site.md) presents documentation;
it is not another program-meaning authority.

## Native web and shared deployment

[`new --template web`](guides/native-web.md) creates an editable stateless GET
application with typed UI modules, tests and loopback deployment.
[`new --template web-editor`](guides/native-web-editor.md) adds an authenticated
single-note editor with explicit data/credential setup and transactional conditional
saves. [Forms](guides/native-forms.md) preserve ordered duplicates and bounded UTF-8.
These starters require no Node.js, downloaded template or application-authored
HTML/CSS/JavaScript. They are not a browser backend, multi-user document service
or complete web framework; loopback defaults do not establish public HTTPS deployment.

Command, HTTP, worker and structured-session deployments execute standalone artifacts.
The [shared service runtime](spec/shared-runtime.md) shares immutable preparation for
exact artifacts while retaining per-instance configuration, secrets, grants, adapters,
task accounting and cancellation. Whole-group static admission precedes live preparation;
readiness follows listener setup; termination joins owned work. It is not a subprocess
launcher, dynamic supervisor, CPU-parallelism proof or hostile-tenant sandbox.

Deployment builds preserve the original artifact/descriptor and produce an unselected
immutable result. Installation does not change accepted application definitions, exact
dependencies, running services or operational data. There is no automatic migration
or live reload. Code rollback does not roll back data; a missing response does not
prove an effect was rolled back or is safe to retry. See the
[deployment contract](spec/deployment-security.md).

## Current limits and unproved properties

- Offline dependencies are exact closures, not a network registry or mutable-name
  resolver. Recorded recent history is bounded to 100 entries, not general historical
  query, project backup/restore or rollback; operational data backup is separate.
  Plan summaries omit literal before/after text. Retain the proposal, compare the
  exact plan, and re-plan unchanged requests after an executable upgrade.
- Capability resources remain lexical and task-local. General resource returns,
  requirement-polymorphic resource transfer and asynchronous borrowing remain absent.
  Ambiguous name-only grants reject; exact package/requirement selectors disambiguate.
  General owned containers, partial moves, field borrows, mutable or escaping memory
  references and structured asynchronous memory transfer remain future work.
- Anonymous closures, automatic capture/generic inference, native-code compilation,
  SIMD and browser/Wasm execution are not established. The Nostr relay-information
  recipe does not supply arbitrary outbound URLs/methods, outbound WebSockets or
  signing/replay. Callable effects, allowances and grants remain distinct, and live
  effects are never replayed merely for differential verification.
- Local durable data and queues support transactions, conditional updates, snapshots
  and operational backup/restore. Independent capability effects can survive failure.
  Replication, consensus, encryption and general migration remain unproved; graph/data
  garbage collection and compaction have not been selected.
- The supported binary target is Linux x86-64 musl. Other targets, a minimum-kernel
  guarantee, hostile-code containment and universal portability remain unproved.
  Work quotas, deadlines, structural bounds and live limits are not interchangeable.
  Bounded demonstrations do not establish arbitrary graph/data scalability.

[Performance evidence](performance.md) retains slower cases and tradeoffs without
universal speed, memory or model-token claims. The [roadmap](roadmap.md) records
revisable choices, not promised implementations.

## Verification and historical source milestones

Public v0.1.64 source `0048ae1e` passed all 26 full-source gates freshly, with stable
inputs, zero reuse and no unrun gates; its workspace suite passed 1,496 tests,
with zero failures and 29 existing ignored cases. The original finalized candidate
passed the separate source/target, pinned-userland and installed-recovery owners.
Its exact extracted executable then passed all 21 required supplementary public cases
outside the checkout with a cleared environment, followed by the completed unchanged
publication and anonymous installed verification. These proofs do not certify later
0.1.65 source merely because it descends from that commit.

Earlier [owned-storage](campaigns/20261001-owned-storage-admission.md),
[owned-product](campaigns/20261001-owned-products.md),
[numbering and cold-source reproduction](campaigns/20261001-version-policy.md),
[metadata-read](campaigns/20261001-owned-product-metadata.md) and
[owned-choice](campaigns/20261002-owned-choices.md) campaigns retain exact acceptance,
original failures and source identities. The
[previous detailed snapshot](https://github.com/lkjsxc/lkjscript/blob/e887fad9f88da1cd016a40daba12bcccc7d77449/docs/status.md)
preserves the old chronology without presenting its publication checkpoints as current.

[Verification obligations](spec/verification.md) distinguish source checks,
final-archive behavior, pinned userlands, installed recovery, authenticated transfer
and anonymous acquisition. Current development proof belongs to the
[task-owned continuation](campaigns/20261002-task-owned-transfer.md). Reporting-only
descendants never relabel the source or bytes that were actually tested.
