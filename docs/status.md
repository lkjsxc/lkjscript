# Current status

Status date: 2026-09-26 Asia/Tokyo. This page describes current capabilities,
release state and limits. [Specifications](spec/) own semantics,
[generated guides](generated/operations.md) own discovery, and
[campaigns](campaigns/) retain exact implementation and verification history.

## Public binary release

**Available:** immutable [v0.1.48](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.48),
release 397062403, published on 2026-09-26 at 12:33:55 Asia/Tokyo and independently
observed as latest. Original producer
[36210073260/1](https://github.com/lkjsxc/lkjscript/actions/runs/36210073260) accepted exact
product source `e2c0d1f58ed0b347c7956d2af248b944ccf422b9`. Promotion
[36214860067/1](https://github.com/lkjsxc/lkjscript/actions/runs/36214860067), controlled
by `0e9160930217ae24e4034ddd9afa7ed408ef467a`, completed authenticated selection,
immutable publication, anonymous installed verification and its successful terminal.
The three original assets were not rebuilt. This release adds immutable deployment
builds to the editable native web starter. The [delivery campaign](campaigns/202609261118.md)
records exact publication lineage and keeps later v0.1.49 source acceptance separate.
Earlier v0.1.47 remains unchanged; its [continuation](campaigns/202609260831.md) retains
that release's original evidence rather than being rewritten as v0.1.48 proof.

The [v0.1.44 record](campaigns/202609222330.md#accepted-v0144-and-the-requested-release-stopping-point)
and [v0.1.45 promotion history](campaigns/202609250903.md) retain their actual earlier
failures and then-pending observations. They are historical evidence, not current
release status, and no frozen assets are replaced.

The supported binary target remains static `x86_64-unknown-linux-musl`, admitted
in the maintained pinned Alpine and Debian userlands. Immutable installation
slots retain exact runtimes; selecting another version neither changes running
processes nor migrates application data. Bundles require a compatible executable.

## Immutable deployment builds in public v0.1.48

`build --deployment PATH` derives an unselected content-addressed artifact and a
sibling descriptor from current accepted meaning and one strict operator template.
Only the artifact value changes. Original files and running processes remain
unchanged, relative local data roots retain their meaning, and matching files are
reused only after exact-byte checks. New descriptors are owner-only from their first
staged byte; broader existing permissions reject without chmod or overwrite.
Static target/grant admission opens no adapters,
secrets or listeners; output paths inside declared local data/object/queue roots
reject. Ordinary `build --output` still requires an absent file.

The initial prototype was withheld after an independent permissions counterexample.
Combined corrected source `e5847f1e568ad823325dd3a1e56c7a6fd64fa3fa` also rejects
ambiguous configuration-map keys, including equal values and escaped duplicates,
through the shared descriptor reader. Valid signed/unsigned 64-bit values remain supported.
It passes 24 local deployment unit tests and six copied-executable public workflows
in an unoptimized focused profile, plus unchanged independent permission and
JSON-ambiguity oracles and a late-publication/recovery trial. These observations
are not finalized-archive or source-wide acceptance.

The [relocation continuation](campaigns/202609261015.md) adds a seventh public workflow:
complete-directory relocation retains exact snapshots and store identity; source-free
execution from an unrelated working directory succeeds, while a transfer missing its
data directory rejects without initializing a substitute. All seven focused workflows
and Clippy pass at test/docs descendant `8da50e4ea6d7bf5e9a96f68d413dd7d84128d5d4`.
An independent script-free Chromium session also passes against moved old/new web
snapshots, with ordinary form interaction and joined shutdown. Product source is unchanged.

The original [hosted full verification](https://github.com/lkjsxc/lkjscript/actions/runs/36206083541),
at exact corrected source e5847f1e and attempt 1, completed successfully on
2026-09-26 at 10:50:22 Asia/Tokyo. All 26 gates passed freshly, zero were reused,
inputs remained stable and elapsed execution was 3334.373119559 seconds.
The original receipt and artifact were downloaded and their identity checked.
Together with the distinct relocation supplement, these changes reached remote
main at `e2c0d1f58ed0b347c7956d2af248b944ccf422b9` through a normal fast-forward;
an independent GitHub read confirmed delivery.

The original v0.1.48 candidate [36210073260/1](https://github.com/lkjsxc/lkjscript/actions/runs/36210073260)
was selected from that exact main source at 10:56:21 Asia/Tokyo and completed its twenty
source gates, six finalized-target owners and two pinned userlands. Its successful
promotion above then verified anonymous public acquisition and the installed lifecycle.
These are distinct actual results, not publication inferred from source acceptance.
The ordinary form-library increment below and v0.1.49 editing behavior remain separate
from that frozen producer's source.
Original failures remain in the [original campaign](campaigns/202609260831.md),
relocation evidence in its [continuation](campaigns/202609261015.md), and current
integration/candidate state in the [delivery record](campaigns/202609261045.md).
See the [editing guide](guides/native-web.md) and [build contract](spec/semantic-cli.md#build).

## Durable web starter in development v0.1.49

`new --template web-editor` creates a locally editable authenticated note app from
one executable. It vendors the maintained ordinary UI, form codec, editor and tests,
not a second application implementation. Creation reports explicit operator steps
for `notes.lkjdata` initialization and `LKJSCRIPT_EDITOR_AUTHORIZATION`; it opens no
adapters, reads no secrets and initializes no data. The `.lkjdata` path is a directory
using the existing store format. Existing paths and the stateless `web` template
remain unchanged. The [starter guide](guides/native-web-editor.md) covers native
editing, immutable builds, safe data placement and private loopback operation.

Focused evidence includes 15 creation unit tests, copied-executable old/new editor
workflows, unchanged stateless web behavior, ordinary/immutable artifact equality,
180 graph tests before and after editing, concurrent conditional saves, source-free
restart and rejection of untrusted requests and corrupt storage. Workspace/all-targets
Clippy and generated discovery checks pass. Independent JavaScript-disabled Chromium
also passes real two-tab saves/conflicts, keyboard review, a 360-pixel layout check
and source-free restart after moving the whole deployment and data directory.
These browser observations use the unoptimized source-built executable.

Exact source `6536fea15635933d8d91eef5ca5e2d930ae5c3f6` completes full verification:
26 fresh gates, zero reused, stable inputs and 1124.976458823 seconds. It reached main
by normal fast-forward and an independent GitHub read confirmed delivery. The
[campaign](campaigns/202609261600.md) retains the original receipt, failed prototype
expectations and separate browser evidence. One nonpublishing v0.1.49 candidate,
[36227761390/1](https://github.com/lkjsxc/lkjscript/actions/runs/36227761390), selects that
exact source and was observed in progress. Finalized-asset acceptance and publication
are not yet confirmed. This new template is not in immutable public v0.1.48; existing
public assets, tags and release-selection controls remain unchanged.

## Ordinary form serialization

The [native form library](guides/native-forms.md) now supplies bounded UTF-8 encoding
as well as strict decoding, without adding a form-specific runtime primitive.
It preserves ordered duplicate names, charges escaped output and separators before
growth, and distinguishes invalid input from an execution-resource failure.
The separate structural consumer exposes individual and batch command targets.

Focused public workflows compare 674 independent cases through both project and
source-free paths, then 205 detached roundtrips. Twelve live encoder/receiver
requests, malformed input, review-token binding, a deliberate encoding mutation
and the unchanged durable editor are checked separately. These library/test
observations do not relabel the deployment full receipt or the v0.1.48 binary
candidate. The unchanged official v0.1.47 binary also completes fresh native
authorship and source-free execution for 642 cases against an independent
`URLSearchParams` oracle; no compiler rebuild or new runtime dependency is required.
The [campaign](campaigns/202609261045.md) owns exact proof and scope.

## Identity-preserving native edits in development v0.1.49

Same-structure function edits now update only changed scalar values after a complete canonical
intent comparison and independent live-ownership admission. Expression and binding identities
survive; structural edits retain ordinary replacement. In the unchanged web-starter workload,
one button label reduces the complete plan from 80,402 to 8,613 bytes: one updated expression,
no recreated or retired owners, and all review evidence retained. This measures graph/proof churn,
not runtime speed, model tokens or monetary cost. Repeated exact owner selections in complete
native declarations reject before normalization, even across different scopes or blocks; they
cannot combine two complete proposals into an unrequested mixture. Eleven new unit tests and
three copied-binary workflows cover exact values, review, ownership, overlap rejection and old/new
detached snapshots. Corrected source `e6688adc14151b8ca9a2271e3305b72a8f3e3ea8` passes all 26
full-profile gates freshly with zero reuse and stable inputs in 1062.324654199 seconds.
The [campaign](campaigns/202609261118.md) retains the earlier failed counterexample and exact
receipt. This new runtime behavior is not in public v0.1.48; no v0.1.49 finalized archive or public
release is claimed. No graph, artifact or application-data format migration is introduced.

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

The byte-conversion and ordinary form source at
`0ce651cad480e4d62668da45fb273d2539441233` passes all **26 full-profile gates freshly**,
with zero result reuse, stable inputs and 1397.481 seconds elapsed. Normal mainline
integration and an independent GitHub branch read confirm that exact accepted
source. The [campaign](campaigns/202609250926.md) preserves the earlier interrupted
run, the genuine 25-of-26 failed attempt and its exact-supplier assertion repairs.
Subsequent reporting changes do not relabel that full acceptance with another SHA.

The durable-editor source `b25bde2d92c1882470fc4444c94319a68fe9a10c` additionally
passes all 20 release-source gates freshly with stable inputs and no result reuse,
and is integrated into remote main. This is not standalone full-26 acceptance.
Its selected v0.1.46 candidate [36097087012/1](https://github.com/lkjsxc/lkjscript/actions/runs/36097087012)
completed successfully at that exact product/controller source, including final-archive
acceptance and the candidate terminal. Its separate promotion now completes public
v0.1.46 as recorded above. The [editor campaign](campaigns/202609251211.md) retains
original receipts, prior failures and integration; later reporting does not relabel them.

| Capability | Actual implementation and evidence owner |
| --- | --- |
| Explicit resident quotas | Four independent nullable cumulative quotas; legacy omissions preserve prior defaults. New HTTP recipes choose four nulls. [Guide](guides/resident-policy.md) and [accepted campaign](campaigns/202609250154.md). |
| Standard text composition | Ordinary native `text-join`, with balanced joining and caller-owned escaping. The guide tool retires its duplicate helper. [Guide](guides/native-text.md) and [campaign](campaigns/202609250013.md). |
| Direct byte inspection | `bytes-get` reads an unsigned octet and traps on invalid indices; the native policy consumes Bytes rather than expanded integer lists. [Campaign](campaigns/202609240603.md). |
| Native reference pages | All eight page renderers and exact-reference validation are ordinary lkjscript. Rust retains observation and publication. [Owner](../tools/native-guides/README.md) and [campaign](campaigns/202609240132.md). |
| Native repository policy | Native extension/shebang decisions, required no-Python gate retained, and one shared pure embedding boundary. [Owner](../tools/native-policy/README.md) and [campaign](campaigns/202609240414.md). |
| Joined contributor processes | Deadlines, output bounds and cancellation survive direct-child exit; final group signaling precedes reaping. Sampled descendants and executable identity retain honest limits. [Campaign](campaigns/202609242248.md). |

These capabilities shipped in v0.1.45; they are not retroactive additions to the
frozen v0.1.44 executable. Native tool adoption is not compiler self-hosting. Rust remains the
supported kernel, parser, platform and contributor/release orchestration boundary.
[Performance evidence](performance.md) retains measured tool costs and slower cases;
none of these milestones establishes an application speedup or model-token savings.

Separately, the public-v0.1.44 [HTML library](guides/native-html.md),
[HTTP library](guides/native-http.md) and [HTML service](guides/native-html-http.md)
are ordinary native composition experiments. They do not add a framework intrinsic,
replace a maintained application, or imply that new development defaults applied
to those older executions. Their literal inputs and actual tested runtimes remain
at their own guide and campaign owners.

The original [native UI library](guides/native-ui.md) and GET-only application were
proved on public v0.1.44. Typed cards, stacks, labels, buttons and themes need no
application-authored HTML/CSS/JavaScript. The library owns fixed HTML/CSS and emits
no script. That historical HTTP/offline-browser proof remains distinct from its
unavailable live-browser proof. The [unselected join-consolidation experiment](campaigns/202609250903.md)
remains unselected. The current ordinary UI package extends controls for the durable
editor below; old exact imports are not automatically upgraded.

A development-v0.1.45 [paged-list workload](guides/native-list.md) combines those
libraries with byte inspection, ordinary text joining and list windows. Its native
parser rejects ambiguous/overflowing pagination before arithmetic; 98 graph tests,
1,860 independent number cases and 72 detached HTTP requests pass. This is an
executable example, not a new runtime primitive or part of the frozen candidate
source. The [delivery campaign](campaigns/202609250650.md) records its distinct proof.

## Native form boundary in v0.1.46

Public v0.1.46 adds general byte construction and a checked UTF-8
result. An ordinary [strict form library and separate consumer](guides/native-forms.md)
use those primitives for bounded URL-encoded inputs while preserving duplicate order.
Its source, public authoring and acceptance boundaries belong to the
[campaign](campaigns/202609250926.md). This is not another released v0.1.45 binary.
The codec does not itself supply authentication, CSRF policy, persistent mutation,
file uploads or a complete stateful UI framework.

## Native durable editor

The [new editor](guides/native-editor.md) uses the UI/form libraries and existing
HTTP, configuration, secret verification and transactional data capabilities.
It has explicit POST saves, labeled multiline fields, hidden base revisions,
strict domain validation, shared Basic authentication and configured Host/Origin
admission. It adds no kernel primitive, browser script, external database or
application-authored markup. Its operator-facing descriptor is also a test input.

A save compares the displayed revision and conditionally writes within one
completed transaction. Concurrent writers get visible conflicts retaining their
drafts; review and another explicit action are required. GET is read-only. Malformed
stored values and version exhaustion are not silently overwritten. There is no
automatic live-effect retry, and a missing response does not imply rollback.

Development authoring/check/build passes 165 combined graph tests, including 38 new
editor tests. Independent HTTP checks include 8 competing writers across 2 processes
(one commit, seven conflicts) and detached restart. Chromium 153 with page JavaScript
disabled performs actual authentication, form POST/redirect/GET, two-tab conflict and
review; leading LF/Unicode round-trip and 360px layout pass. This browser trial found
and corrected the no-referrer/Origin-null incompatibility without weakening admission.
Maintained tests and the [campaign](campaigns/202609251211.md) own complete source evidence.

This is one bounded shared note, not accounts, sessions, a public production deployment,
rich text, multiple documents or a complete framework. The UI extension is an explicit
package update; exhaustive consumers of its closed node variant need the new cases.
The reference runtime remains Rust, and native application composition is not compiler
self-hosting or a Wasm/browser backend.

## Native web starter

Public v0.1.47 adds [`new --template web`](guides/native-web.md). One copied
executable creates the ordinary UI modules, shared tests, a stateless GET application
and its loopback deployment without downloaded sources or manual import identities.
The application composes typed controls; the shared native library owns HTML/CSS
and emits no browser script. Vendored UI declarations remain locally editable and
do not silently update with another installed runtime. Separate exact library
imports retain their own authority.

Focused public-CLI verification passes 107 graph tests (19 UI, 13 application and
75 built-in standard tests), title editing with stable declaration identity,
wrong-type rejection without publication, standalone HTTP behavior, shutdown and
restart after removing the authoring graph. Chromium 153 with page JavaScript
disabled passes actual form submission, Unicode/markup-as-data input, keyboard
submission, refresh and a 360px viewport without horizontal overflow. The
[campaign](campaigns/202609251450.md) retains original evidence and the earlier
incorrect local-test-count assertion. Corrected source
`511f48c41bc45374492051181e5334fa018b0d71` now passes all 20 release-source gates
freshly, with stable inputs and zero reuse. Normal mainline integration is complete
through documentation descendant `18e04bc0e6611ff0b79a49da1e5ce5da3632da35`.
That source profile is not standalone full-26 or finalized-archive acceptance. The
selected v0.1.47 candidate [36198402289/1](https://github.com/lkjsxc/lkjscript/actions/runs/36198402289)
subsequently completed finalized-archive and terminal acceptance at that exact
18e04bc0 product/controller source. Its unchanged assets are now publicly available
through the completed promotion recorded above.

This template is not part of the immutable public v0.1.46 executable. It has no
saved state, authentication, POST mutation, account system, client-side event
framework or Wasm backend. GET values appear in URLs and are not
secret storage. The existing durable-editor example remains a separate demonstration
of explicit admission and completed transactions.

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
