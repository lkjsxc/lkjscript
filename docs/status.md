# Current status

Status date: 2026-09-25 Asia/Tokyo. This page describes current capabilities,
release state and limits. [Specifications](spec/) own semantics,
[generated guides](generated/operations.md) own discovery, and
[campaigns](campaigns/) retain exact implementation and verification history.

## Public binary release

**Available:** immutable [v0.1.44](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.44),
source `9ea93419f1c5e76126548ac5460752fe4cc4511b`. Its candidate acceptance,
publication and anonymous exact/latest installed verification are complete.
The [release procedure and history](release.md) and
[original release record](campaigns/202609222330.md#accepted-v0144-and-the-requested-release-stopping-point)
retain the real producer, corrected public lookup and earlier failed boundaries.
Those failures are not relabeled and no old immutable assets are replaced.

**Selected successor:** [v0.1.45](releases/v0.1.45.md), candidate source
`4d64abc70f88dcfd6d31b0e800f23ca8bfb1459d`, integrated on main.
Producer [36064706830/1](https://github.com/lkjsxc/lkjscript/actions/runs/36064706830)
was dispatched once at that exact source. Candidate acceptance and publication
are not yet complete. The [delivery campaign](campaigns/202609250650.md) owns
its observed state and resumption. A prepared version or running CI is not a release.

The supported binary target remains static `x86_64-unknown-linux-musl`, admitted
in the maintained pinned Alpine and Debian userlands. Immutable installation
slots retain exact runtimes; selecting another version neither changes running
processes nor migrates application data. Bundles require a compatible executable.

## Current authority and maintained consumers

The accepted typed meaning graph is the sole editable program authority. Native
units, targeted requests and canonical drafts are proposals. Plan/apply binds
review to validated effects, preserves intended identities and publishes
atomically. Invalid, stale or cancelled changes do not partially publish meaning.
Names, read-only projections and package availability confer no execution authority.

Revision-bound queries and complete local-function projections support inspection.
Exact offline packages transport the full implementation closure while retaining
types, effects, visibility and provenance after authoring sources are removed.
The [first native command](guides/native-command.md) and [library guide](guides/native-library.md)
show fresh public authorship, review, tests and detached use.

[Standard](../packages/standard/README.md) owns reusable ordinary functions and exact
platform interfaces. [lkjournal](../applications/lkjournal/README.md) owns its HTTP
service, interactive subscriptions, durable worker and local application-data policy.
Public build operations regenerate their deterministic assets; read-only contributor
inspection is not an accepted-graph writer.

## Accepted development increments

The combined implementation at `8242f3a71e71eab49c70857f76716c8cc30a72dd` passes
all 26 full-profile gates freshly, with zero reuse, in 740.134 seconds, and is on
remote main. The version-preparation descendant `4d64abc7` has its own successful
focused product lifecycle and generated-guide checks. Hosted source/finalized-target
acceptance belongs to that versioned candidate, not to a renamed predecessor receipt.

| Capability | Actual implementation and evidence owner |
| --- | --- |
| Explicit resident quotas | Four independent nullable cumulative quotas; legacy omissions preserve prior defaults. New HTTP recipes choose four nulls. [Guide](guides/resident-policy.md) and [accepted campaign](campaigns/202609250154.md). |
| Standard text composition | Ordinary native `text-join`, with balanced joining and caller-owned escaping. The guide tool retires its duplicate helper. [Guide](guides/native-text.md) and [campaign](campaigns/202609250013.md). |
| Direct byte inspection | `bytes-get` reads an unsigned octet and traps on invalid indices; the native policy consumes Bytes rather than expanded integer lists. [Campaign](campaigns/202609240603.md). |
| Native reference pages | All eight page renderers and exact-reference validation are ordinary lkjscript. Rust retains observation and publication. [Owner](../tools/native-guides/README.md) and [campaign](campaigns/202609240132.md). |
| Native repository policy | Native extension/shebang decisions, required no-Python gate retained, and one shared pure embedding boundary. [Owner](../tools/native-policy/README.md) and [campaign](campaigns/202609240414.md). |
| Joined contributor processes | Deadlines, output bounds and cancellation survive direct-child exit; final group signaling precedes reaping. Sampled descendants and executable identity retain honest limits. [Campaign](campaigns/202609242248.md). |

These are development-source capabilities, not additions to the frozen v0.1.44
executable. Native tool adoption is not compiler self-hosting. Rust remains the
supported kernel, parser, platform and contributor/release orchestration boundary.
[Performance evidence](performance.md) retains measured tool costs and slower cases;
none of these milestones establishes an application speedup or model-token savings.

Separately, the public-v0.1.44 [HTML library](guides/native-html.md),
[HTTP library](guides/native-http.md) and [HTML service](guides/native-html-http.md)
are ordinary native composition experiments. They do not add a framework intrinsic,
replace a maintained application, or imply that new development defaults applied
to those older executions. Their literal inputs and actual tested runtimes remain
at their own guide and campaign owners.

## Language and runtime boundary

The language supports explicit generic records/variants, finite recursive nominal
data, rank-one type/effect/requirement parameters, named pure/task callables, prefix
binding and capture-safe retained values. Persistent lists and ordered maps share
retained structure. Ordinary libraries provide folds, maps, composition, binary64
operations, typed data and transaction-completion reporting.

Project pure commands support independent evaluation. Deployment Command, HTTP,
worker and structured-session runners execute standalone artifacts without opening
an authoring project. Callable effects, current allowances and deployment grants
are distinct checks. Live effects are not replayed for differential verification.

Trusted execution need not have an invented cumulative work budget. Explicit quotas,
operational deadlines, cancellation, structural bounds and live limits are different
controls. Observed allocated bytes measure cumulative work, not RSS or live heap.
Owned work is joined; cancellation or cleanup failure does not prove rollback.

First-party local data and durable queues support transactions, conditional updates,
snapshots and operational backup/restore. Transaction outcomes concern the actual
store's decision; independent capability effects can survive a failed application
operation. Missing output does not establish that retrying is safe.

The inbound listener is plaintext; outbound HTTPS is bound to an exact deployment
endpoint and explicit trust/address policy. Static linkage, safe Rust and quotas
are not hostile-code containment, encrypted storage or multi-tenant isolation.

## Current limits and unproved properties

- Package dependencies are exact offline closures. A network registry/resolver,
  mutable package names and automatic upgrade selection are absent.
- Canonical `change draft` is available. Project history, backup/restore/doctor and
  arbitrary source queries are separate unimplemented facilities; operational
  `data backup|restore` does not provide them. General move/inline is not implied.
- Affine resources remain lexical and task-local, with the supported private,
  requirement-bound consume handoff. General resource results, borrowing signatures,
  cross-package transfer and asynchronous ownership remain unproved.
- Anonymous closures, automatic capture, generic inference, expanding nominal
  instantiation, JIT/AOT specialization and SIMD are not implemented by the current
  explicit abstraction facilities. A browser/Wasm backend is not provided.
- Arbitrary outbound URLs/methods, outbound WebSockets, Nostr event signing and
  reconnect/replay policy do not follow from the relay-information recipe.
- The admitted 100,100-owner lifecycle and million-independent-module capacity
  workload do not prove million-owner compilation, operational-data scale or long
  history. Graph/data garbage collection and compaction have not been selected.
- Data support is one trusted local Linux host. Replication, consensus, encryption,
  additional binary targets, minimum-kernel guarantees and universal portability
  remain unproved.

Exact finite bounds are discoverable through `lkjscript capabilities`. Current
admission limits are not demonstrated scale ceilings. [The roadmap](roadmap.md)
contains revisable choices driven by useful public workloads, not promises.

## Verification

[The specification](spec/verification.md) owns the proof obligations. Standalone
`check full --fresh` requires 26 fresh gates. Release acceptance combines 20 fresh
source gates, six behavioral owners against the executable extracted from the final
archive, both pinned userlands, installation/recovery and original readers.
Authenticated transfer and anonymous acquisition have separate small lifecycles;
passing a source test or uploading an artifact alone establishes no publication.

Native tests, copied-product workflows, maintained consumers and live adapters have
distinct scopes. Each campaign retains actual fresh, reused, failed, cancelled,
skipped and pending observations. Reporting-only descendants never relabel the
source accepted by an earlier receipt. Historical details remain at their existing
owners rather than becoming a second status chronicle.
