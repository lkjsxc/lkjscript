# Status

Current snapshot: 2026-10-01 (Asia/Tokyo). Availability, source acceptance and
running deployments are separate. Historical measurements and failed attempts
belong to the [campaigns](campaigns/) and [release records](release.md), not to a
second list of pending obligations.

## Public binary release

**Public/latest is immutable [v0.1.60](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.60).**
Release `400357606` was published on 2026-10-01 at 04:10:53 JST. Original accepted
producer `36617982924/1` supplies source
`2962c43f0617bda2c5726a96249c8fe53f571747`; promotion `36763094941/1` completed at
04:12:24 JST with `immutable_published_and_public_verified`. Its authenticated
original three assets match the independently read public inventory. Anonymous
exact/latest acquisition, release attestations and the actual installed lifecycle
passed. The [completed delivery](campaigns/20260930-descendant-inventory.md#completed-v0160-publication)
retains the source, annotated tag, terminal result and immutable asset identities.

The [v0.1.60 notes](releases/v0.1.60.md) consolidate the unreleased v0.1.56–v0.1.60
increments; those intermediate numbers are not separate public releases. The
public executable now includes recursive and cross-package exact resource helpers,
contiguous resource-parameter suffixes, ordinary type parameters and effect-generic
resource-free callbacks. Exact deployment grant selectors distinguish same-name
obligations without merging their authority. Terminal ordinary-value transfers,
shared immutable map-key payloads, reusable byte concatenation storage and bounded
affine metadata admission are also included. These are not general memory borrowing
or a whole-program zero-copy or timing/RSS claim.

v0.1.60 does **not** include the v0.1.61 owned-memory and byte-range development
below. Earlier immutable releases and genuine failed or superseded candidates remain
unchanged. Inspect the installed executable with `lkjscript capabilities`.

## Development v0.1.61

The [v0.1.61 release notes](releases/v0.1.61.md) select owned data across ordinary
libraries as the next binary milestone. Its own candidate/final-archive acceptance
remains separate from source integration; public/latest is still v0.1.60.

The following capabilities are integrated source development, not a public v0.1.61
binary or an automatic application upgrade.

### Immutable byte ranges

`bytes-slice(bytes, start, end)` selects a strict half-open view without payload
copying in the production checked evaluator. Nested views retain one original
backing; empty views retain none. `bytes-copy(bytes)` explicitly detaches visible
bytes when a small view would otherwise keep a large parent alive. Aliases, ordering
and encoded contents remain immutable. See the [guide](guides/native-byte-ranges.md)
and [scope/evidence](campaigns/20260930-byte-ranges.md).

### Owned data and exact static implementations

Concrete `ByteBuffer` has pure creation, consuming push/freeze/discard and scoped
synchronous read borrowing, separate from capability-resource authority. Direct
owned results and a final borrow/consume suffix compose with ordinary generic
data and callbacks across packages. The [guide](guides/native-byte-buffer.md)
and [specification](spec/owned-byte-buffers.md) own the exact rules.

Explicit `Owned` parameters support symbolically checked direct affine parameters,
locals and results. Nominal method contracts use exact static implementation
operands; independently sealed `OwnedI64Cell` supplies signed-scalar storage
alongside ByteBuffer. A generic-only library can compose producer, transformer
and consumer packages without choosing one global implementation. See the
[guide](guides/native-owned-generics.md), [specification](spec/owned-generics.md)
and [accepted predecessor](campaigns/20260930-owned-generics.md#observed-completed-acceptance--2026-10-01).

General traits, owned containers, escaping or mutable borrows, asynchronous owned
transfer and task memory signatures are not supplied by this increment. Raw and
durable boundaries do not admit these process-owned memory tokens.

### Owned storage admission

ByteBuffer creation reserves its modeled owner token, synchronized storage, loan
bookkeeping and shared-control metadata at the storage owner. Both evaluators use
that admitted constructor. Cancellation during capacity reservation stops before
vector allocation. On the inspected Linux x86-64 host, both predecessor evaluators
charged 48 bytes instead of the independently modeled 80 bytes for creation.
No quota was raised to make the correction pass.

The storage-admission correction changes neither language meaning nor canonical
encoding. A fixed allocation policy can now refuse previously uncounted work.
These are cumulative admission charges, not allocator size classes, RSS or
live-heap measurements. The [continuation](campaigns/20261001-owned-storage-admission.md)
retains the failed predecessor controls and exact-source verification.

### Structural owned products (integration pending)

The current isolated implementation adds explicit fixed named products, consuming
construction from live locals and complete lexical decomposition. Products compose
one or more Owned payloads with closed ordinary metadata, including nested products,
whole synchronous borrows and exact implementation witnesses. Product ownership is
explicit; ordinary records and containers retain their existing contracts. Open
ordinary metadata parameters, partial moves and field borrows remain outside this
slice. See the [specification](spec/owned-products.md),
[native guide](guides/native-owned-products.md) and
[implementation evidence](campaigns/20261001-owned-products.md).

This work has not been integrated or released. Its derived compiler/artifact cut
requires rebuilding retained source; historical source and immutable releases remain
unchanged. Final full acceptance belongs to the main integrator.

## Current authority and maintained consumers

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
This is not a Rust-only mandate or a permanent boundary against eventual self-hosting.
The [language-first direction](direction.md) remains in force. The independent
[Rust documentation site](guides/rust-site.md) is presentation, not program authority.

## Native web and deployment

[`new --template web`](guides/native-web.md) creates an editable stateless GET
application with ordinary typed UI modules, tests and loopback deployment.
[`new --template web-editor`](guides/native-web-editor.md) adds an authenticated
single-note editor with explicit data/credential setup and transactional conditional
saves. [Forms](guides/native-forms.md) preserve ordered duplicates and bounded UTF-8;
typed missing/present/repeated selection is not HTTP-header adaptation.
No Node.js, downloaded template or application-authored HTML/CSS/JavaScript is
needed for these starters. They are not a browser backend, multi-user document
service or complete web framework; loopback defaults are not public HTTPS deployment.

Command, HTTP, worker and structured-session deployments execute standalone
artifacts. The [shared service runtime](spec/shared-runtime.md) shares immutable
prepared code for exact artifacts while retaining per-instance configuration,
secrets, grants, adapters, task accounting and cancellation. Static admission precedes
live preparation; readiness follows listener setup; termination joins owned work.
It is not a subprocess launcher, dynamic supervisor or hostile-tenant sandbox.

Installing a runtime does not replace accepted application definitions, exact
dependency selections, running services or operational data. Deployment builds
preserve the original artifact/descriptor and return an unselected immutable result.
There is no automatic migration or live reload. Code rollback does not roll back
data; a missing response does not prove that an effect was rolled back or is safe
to retry. See the [deployment contract](spec/deployment-security.md).

## Current limits and unproved properties

- Offline dependencies are exact closures, not a network registry or mutable-name
  resolver. Recorded recent history is bounded to 100 entries, not general historical
  query, project backup/restore or rollback; operational data backup is separate.
- Plan summaries do not show literal before/after text. Retain the original draft
  and compare the proposal and exact plan. After a runtime upgrade, re-plan the
  unchanged request rather than rewriting its base or reusing an incompatible token.
- Capability resources remain lexical and task-local. Public v0.1.60 supports exact
  cross-package borrow/consume contracts; general resource returns,
  requirement-polymorphic resource transfer and asynchronous borrowing remain absent.
  Ambiguous name-only grants reject; exact package/requirement selectors disambiguate.
- Owned aggregates and structured asynchronous memory transfer remain future work.
  Anonymous closures, automatic capture/generic inference, a native-code language
  backend, SIMD and a browser/Wasm backend are not established by current features.
- Arbitrary outbound URLs/methods, outbound WebSockets and Nostr signing/replay are
  not provided by the relay-information recipe. Callable effects, current allowances
  and deployment grants remain distinct; live effects are not differentially replayed.
- Local durable data and queues support transactions, conditional updates, snapshots
  and operational backup/restore. Independent capability effects can survive a failed
  application operation. Replication, consensus, encryption and general migration are
  unproved; graph/data garbage collection and compaction have not been selected.
- The supported binary target is Linux x86-64 musl. Other targets, a minimum-kernel
  guarantee, hostile-code containment and universal portability remain unproved.
  Work quotas, deadlines, structural bounds and live limits are not interchangeable.
  Admitted owner-count workloads do not prove arbitrary graph or data scalability.

Exact bounds are discoverable through `lkjscript capabilities`.
[Performance evidence](performance.md) retains slower cases and tradeoffs without
universal speed, memory or model-token claims. The next preferred language boundary
is useful owned-data composition before more dispatch or supervision machinery;
the [roadmap](roadmap.md) contains revisable choices, not promised implementations.

## Verification

Source `0dadaed890d7425509a37d746088500650be1fb7`, tree
`48e1d8382c3c254cf59e19214a35352ea042a425`, passed all 26 full-profile gates freshly,
with stable inputs, zero reused results and no unrun gates. Workspace execution
passed 1,461 top-level tests, with zero failures and 29 existing ignored cases.
Two nested one-test subprocess controls are not counted twice.
The copied optimized v0.1.61 executable separately passed all 18 selected owned-memory,
byte-range and resident-policy cases from `/tmp` with an empty environment/PATH
except its explicit candidate selector. An independently authored packet/shadowing
witness passed both branches through project and source-moved detached execution.
These results bind the [owned-product source](campaigns/20261001-owned-products.md#accepted-frozen-source),
not a finalized distribution or another source revision.

[Verification obligations](spec/verification.md) distinguish full source checks,
20-gate release-source acceptance, six final-archive behavioral owners, pinned
userlands, installed recovery, authenticated transfer and anonymous public acquisition.
The [current campaign](campaigns/20261001-owned-products.md) binds the exact
source, retained original logs and copied binaries. Reporting-only descendants do
not relabel a tested source. The [previous detailed snapshot](https://github.com/lkjsxc/lkjscript/blob/9afa799794ac26fa90bd4b2413e6e4d26886ccb6/docs/status.md)
preserves milestone chronology; current instructions live in the linked guides.
