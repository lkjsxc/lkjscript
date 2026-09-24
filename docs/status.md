# Current status

Status date: 2026-09-25 Asia/Tokyo. This page describes current product boundaries and
outstanding delivery. [Generated guides](generated/operations.md) own public
operations, [specifications](spec/) own semantics, and [campaigns](campaigns/)
retain detailed implementation and verification history.

## Native standard text composition

Development standard now provides ordinary pure `text-join(List<Text>, Text)`: exact
order and text, separators only between entries, and empty/singleton behavior without
coercion or escaping. The native balanced-range implementation adds twelve fixed tests;
all 63 standard tests agree between evaluators. No Rust intrinsic or format change is
added. The copied public example checks 2,197 independent runtime cases through
32,768 items both before and after source removal. Three deliberately wrong separator
implementations are caught by the existing literal graph tests. The
[native guide](guides/native-text.md) owns the copied-product usage.

The maintained eight-page reference tool adopts that standard function and retires
its duplicate range helper, preserving its public join identity. All 79 closure tests
agree. Other exact consumers and the published-v0.1.44 HTML library retain their
existing selections. These changes are development source, not an official release
or compiler self-hosting. The [campaign](campaigns/202609250013.md) owns final-source
acceptance, runtime-case evidence, measured costs and delivery state.

## Joined contributor processes and native HTML composition

Source `4ead1716` passes all 26 full-profile gates freshly, with zero result reuse
and stable input (677.449 seconds). The shared contributor process owner no longer
reaps before its final group signal or blocks indefinitely joining inherited output.
Nonblocking streams retain deadlines, output bounds and cancellation after direct-child
exit; sampled owned descendants are cleaned on every shared route. Git/toolchain
identity commands use the same owner. Eleven new regression tests include two finite
fixtures that fail on the predecessor. This is not hostile-process containment or an
application VM speedup. See the [execution record](campaigns/202609242248.md).

The public v0.1.44 [typed HTML library](guides/native-html.md) now has 88 independently
checked detached render cases, six rejected runtime inputs and successful article
recovery. The new 76-line [native HTTP composition](guides/native-html-http.md) connects
that library to the separate response library without a new intrinsic or host renderer.
Its 74 differential graph tests include four new composition tests. Nineteen successful
HTML responses, three expected 404s, eight simultaneous clients and two clean detached
server lifecycles are separately observed. Runtime files are only the executable,
bundle and deployment descriptor; exact authoring roots are not runtime dependencies.

The HTTP adapter still requires its explicit request-stream owner even for this GET
handler. The generated numeric resident execution policy remains unchanged. These
public-binary experiments and their documentation are separate evidence from the
Rust full-profile source above. No official binary release or production deployment
was performed, and frozen v0.1.44 assets are unchanged.

## Native byte inspection (development checkout)

Source `b7b0ff52` passes all 26 full-profile gates freshly, with zero result reuse and
stable input (728.556 seconds). A stale verification-count assumption and an existing
closed-output fixture race were fixed before this acceptance; failed runs remain in
the campaign evidence. Ten targeted public CLI tests also pass on the immutable final
product bytes, separately from earlier producer copies.

The complete required policy command improves from median 198.502 to 72.962 ms in
seven alternating matched pairs against the predecessor native implementation. The
contributor executable grows by 34,080 bytes. This is not a Rust-predicate comparison,
zero-copy claim or a general application speedup; see [measured costs](performance.md#native-byte-inspection-2026-09-24).

The current standard adds `bytes-get(Bytes, I64) -> I64`: zero-based unsigned octet
observation without text conversion, clamping or mutation. Out-of-range indices
trap. Six new graph tests bring the standard to 51. Independent evaluators and
copied public CLI tests cover every octet, both I64 extremes, invalid input, empty
buffers and detached execution after source removal. The closed external-signature
validator feature changes, but graph/artifact/data encodings do not.

The required native repository policy now receives `List<Bytes>`, not expanded
integer lists, and passes 62 differential tests. Its nine host tests retain the
independent 1,600-case rule oracle and all file/read/order semantics. Unchanged guide
and lkjournal selections are not upgraded: the maintained guide still has 61 tests
and its bundle rebuilds byte-identically. New guide authorship explicitly selects
the actually exported standard and passes 67 tests. The old policy bundle works on
the new runtime; the predecessor runtime rejects the new intrinsic before execution.
The [byte campaign](campaigns/202609240603.md) records exact native requests, migration,
focused evidence and acceptance. This is development source, not an update to the
frozen public v0.1.44 assets or a compiler-self-hosting claim.

## Native development tooling (earlier acceptance)

The [native guide tool](../tools/native-guides/README.md) now owns all eight reference
pages in ordinary lkjscript, including prose, layout, escaping and required-reference
selection. Missing, duplicate or empty exact references reject in native meaning.
Rust retains read-only metadata observation, typed admission, pure execution and
derived-file publication; the six remaining Rust page renderers are removed.
The maintained program passes 61 differential graph tests; ten adapter tests and
two copied-product authoring/regeneration/edit/detached tests pass. Independent
comparison preserves all registry/interface payloads, 20 required references and
ten numeric limits. Six page layouts change to escaped HTML; two remain byte-identical.

This is native tool adoption, not compiler self-hosting. Public v0.1.44 remains
frozen. Source `ee2c6079` passes all 26 full-profile gates freshly, with stable
inputs and zero reuse. The [current campaign](campaigns/202609240132.md) records
the exact source evidence and public-binary reproduction. Matched eight-page
generation rises from median 389.797 to 458.215 ms; the host binary grows by
254,272 bytes. There is no speedup or application-runtime performance claim.
That accepted history reached remote main at reporting descendant `41861c85`.
The earlier workspace Git-authentication blocker is resolved; source acceptance
and mainline integration remain separate from frozen public-release publication.

The [native repository-policy tool](../tools/native-policy/README.md) now supplies
extension and raw-shebang decisions to the existing required no-Python gate.
Rust keeps Git/path observation, bounded filesystem reads and report/exit handling;
the old Rust decision implementation is removed. Both maintained native tools use
one grant-free pure artifact embedding boundary. The initial native policy meaning passed
11 policy and 45 exact-standard tests; focused host tests compare 1,600 case/byte
variants plus first-line, prefix-boundary, link and multi-batch cases.
Implementation `9e5e192b` passes all 26 full-profile gates freshly, with zero
reused results and stable before/after inputs (750.277 seconds). The required
policy command is slower: alternating seven-pair medians are 7.287 ms before and
190.284 ms after; the contributor executable grows by 770,424 bytes. This is
a measured native-tool adoption cost, not an application speedup claim. The
[campaign](campaigns/202609240414.md) retains scope, evidence and delivery boundaries. This does not add an application requirement, language
intrinsic, filesystem capability or compiler-self-hosting claim.

## Standard list windows

Public v0.1.44 adds ordinary pure `list-window<Item>(items, start, count)`.
It preserves order and clamps start/count without overflow at the signed I64
extremes. The native standard request passes all 45 package tests, including
eight new range/type cases. The existing transported aggregation consumer now
uses it for structural entry pages and nominal events; its 72 copied-product
commands and original reader pass with joined cleanup. Generated standard transport,
artifact and discovery include the function. lkjournal keeps its prior exact supplier:
checks pass and the complete rebuilt artifact is byte-identical. There is no
data-format migration.

Corrected source `9ea93419` passes all 20 fresh source gates with stable inputs
and zero reuse and reached remote main unchanged. Candidate
[35833374673/1](https://github.com/lkjsxc/lkjscript/actions/runs/35833374673)
is accepted at that exact source: six target owners, two pinned userlands,
installation/recovery and original-reader admission all pass with joined cleanup.
Authorized promotion
[35838470851/1](https://github.com/lkjsxc/lkjscript/actions/runs/35838470851)
publishes the same assets as immutable
[v0.1.44](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.44).
After correcting the controller's public metadata lookup, resumption
[35841466243/1](https://github.com/lkjsxc/lkjscript/actions/runs/35841466243)
passes anonymous exact/latest acquisition, attestations, installed lifecycles and
the terminal decision using the original accepted producer and verifier.
The [release record](campaigns/202609222330.md#accepted-v0144-and-the-requested-release-stopping-point)
retains authenticated originals, the first candidate's stale-count rejection and
the first public-boundary failure. The requested release stopping point is complete.
v0.1.43 remains complete at its separate frozen source.

## Command result files

Public v0.1.43 provides optional `--result-file PATH` on both Command routes.
It publishes the exact bounded typed JSON through the existing create-new output
owner, allowing values larger than one compact display record. Destination
inspection precedes context/secret/adapter reads, and publication rechecks after
encoding and joined cleanup. A late output failure does not undo application
effects. Focused tests pass byte/limit, preflight and post-commit conflict cases.
Fresh copied native counting returns complete maps through 32,768 distinct keys,
and all 49 commands in the numerical owner pass with the new output bindings.
The [guide](guides/native-library.md#save-a-command-result) documents the interface.
Graph, artifact and data encodings remain compatible; numerical acceptance schema 3
adds result-file bindings while frozen producers keep their original verifiers.

The successor correction counts actual JSON Map pair arrays and synthetic fields,
checks object-key/tag/base64 string bounds, and respects the unchanged parser's
127-container guard. Fixed-shape codec tests, the copied CLI and fresh native
authoring outside the checkout pass adjacent 33,333/33,334-key success/rejection
on both routes. The unchanged predecessor bundle also returns the exact smaller
result. Over-limit output now fails before file publication; valid bytes and
typed-data limits remain unchanged.
Aggregate JSON text is reserved before copying it. Focused boundary tests and a
copied native shared-text program pass; its 64 MiB logical
result now rejects before full expansion, with exact valid bytes preserved.
The [measurements](performance.md#rejected-json-text-materialization) retain all
cases and their scope. Combined source `9394c0ea863823f0ce068e1e956253d92ae80471`
passes all 20 fresh source gates with stable inputs and zero reuse and reached
remote main unchanged. Candidate [35820435256/1](https://github.com/lkjsxc/lkjscript/actions/runs/35820435256)
passes final-candidate, installation and original-reader acceptance. Promotion
[35824461469/1](https://github.com/lkjsxc/lkjscript/actions/runs/35824461469) completes
immutable publication and anonymous exact/latest installed verification. Public
[v0.1.43](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.43) contains the same
accepted assets. Candidate/publication originals and the genuinely rejected first
promotion remain retained. The
[framing record](campaigns/202609222330.md#typed-json-framing--newly-observed-output-boundary)
preserves the original defect and earlier evidence at its actual scope.
The [native summary guide](guides/native-summary.md) also passes fresh authoring,
checking and detached use, including a four-field result over 99,999 distinct keys.
The [paging guide](guides/native-pagination.md) returns all 99,997 independently
checked entries in three bounded files and reuses its ordinary generic window
function with a separately authored consumer's nominal record.
The [ranking guide](guides/native-ranking.md) adds ordinary generic bounded
selection alongside full merge sort. Its fresh native and transported nominal
consumers pass, including 2,548 small input/count combinations. Matched
[measurements](performance.md#native-bounded-selection-2026-09-23) retain both
improvements and slower cases. The optional `select-buffered-by` function and
`top-buffered` command reuse the same native merge helpers and preserve stable
results while avoiding per-item replacement. Fresh authoring and transported
nominal use pass; descending input remains a measured slower case, so range
selection stays available. The earlier full-sort consumer also runs on v0.1.43
after reviewed exact standard selection; conflicting pins reject before acceptance.

## Command input files

Public v0.1.43 includes `--arguments-file PATH` on project and standalone
command execution. It removes the observed OS argument-vector barrier for larger
JSON inputs while retaining the existing 1 MiB byte limit, strict decoder, typed
admission and effect ordering. The selectors are mutually exclusive; omitted input
remains `[]`. Relative files use the invocation directory and must be regular with
no final symlink. The [native library guide](guides/native-library.md#read-arguments-from-a-file)
shows both forms. Candidate [35810382909/1](https://github.com/lkjsxc/lkjscript/actions/runs/35810382909)
completes source/final-candidate, installation and original-reader acceptance.
Publication is withheld for the subsequently observed shared JSON framing defect;
no tag or scoped-selection change occurred. The accepted lineage is retained,
and its corrected successor v0.1.43 is publicly complete.

The successor also shares immutable pack indexes across validated object reads.
It uses one accepted source view for preparation and bounded reuse of fully
admitted catalog blocks, retaining authority rechecks and per-object integrity and
request allowances. Source `8e9628ad3c8e71f886cd64aa750e032b20104bfa` passes all 20
fresh source gates with stable inputs and zero reuse, and reached remote main
unchanged. The [matched public measurements](performance.md#bounded-catalog-block-reuse)
retain slower cases and establish no general speedup or history-scale guarantee.

## Composable typed-cell updates

The [selected milestone](campaigns/202609221952.md) is publicly delivered in
v0.1.41. Ordinary native-authored standard tasks expose two contracts:
`data-cell-update-in-transaction` stages a tentative value in its caller's matching
transaction; `data-cell-try-update` owns a transaction and returns its completed
outcome. Exact `DataStore.require-transaction` rejects missing canonical scope
before the helper reads data or invokes its callback. Same-store nested owners
still reject. Independent callback effects may survive abort; retries remain
application policy.

The transported requirement-library witness adopts these ordinary tasks. lkjournal
only replaces its exact standard dependency and generated artifact. Its application
meaning and data format are unchanged. Native creation/drafting of generic
`transaction-outcome` also includes the exercised syntax repair. The
[evidence owner](evidence/202609221952-cells/README.md) distinguishes focused public
authoring, maintained adoption, official predecessor compatibility and release proof.

The [autonomous continuation](campaigns/202609222330.md) repaired a stale service
artifact pin and required-nullable deployment discovery while preserving original
failed/cancelled evidence. Corrected source `346c0366` is the accepted and published
product. Its [installed participation observation](evidence/202609221952-cells/README.md#immutable-publication-and-installed-participation)
passes two participants under one owner, separate-process reads, absent-owner
rejection before a trapping callback and unchanged data HEAD. The
[native library guide](guides/native-library.md) is exercised on published v0.1.40;
its generic library, exact import and standalone execution need no compiler checkout.

## Public binary release

The current published product is immutable [v0.1.44](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.44).
The [standard-window delivery record](#standard-list-windows) owns its exact
accepted source, publication and public verification. Native guide-tool development
is separate from that frozen release.

### v0.1.41 delivery

The historical v0.1.41 product is immutable
[v0.1.41](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.41), source
`346c0366bde29151952a19332cb540681ad7bc7c`. Producer `35801102943/1` passed source,
final-target and installation acceptance. Promotion `35809096928/1` published
the same assets and passed anonymous exact/latest installed verification and its
terminal. The [delivery record](evidence/202609221952-cells/README.md#immutable-publication-and-installed-participation)
retains original identities and expiry. The earlier bounded
[public native-authoring observation](evidence/202609220955-native/public-v0.1.40/README.md)
on v0.1.40 remains valid for its recorded scope; it is a designed witness, separate
from maintained adoption or an independent-agent experiment.

The supported distribution is `x86_64-unknown-linux-musl`, with the exact static
binary admitted in the pinned Alpine and Debian userlands. The native installer
maintains immutable version slots and an explicit local selection. Runtime
selection grants no application authority and performs no data migration.
[Release procedure](release.md) owns current delivery and recovery requirements.

### v0.1.39 delivery

The [v0.1.39 continuation](campaigns/202609210911.md) completed immutable publication
and anonymous public verification with producer `35508727722/1` and resumption
`35668854407/1`. It includes callable-input evolution, persistent ordered maps and
the recorded HTTP/data corrections. No delivery or rebuild remains due for it.

### Release delivery cutover

The [release cutover](campaigns/202609180007.md) and
[consumer correction](campaigns/202609180528.md) separate final-candidate acceptance
from authenticated unchanged-asset promotion. Their current publication path has
been exercised by v0.1.39 and v0.1.40. The earlier v0.1.38 delivery is complete;
its separate historical manual-reader gap and genuine v0.1.36/v0.1.37 failures
remain in the [milestone record](campaigns/202609162154.md). They are not new
current delivery obligations.

## Current authority and maintained consumers

The accepted typed meaning graph is the sole editable program authority. Native
declaration units, structural/flat requests and canonical `change draft` output
are proposals. `change plan` and `change apply` bind review to validated meaning,
preserve intended identities and publish atomically. Failed, stale or cancelled
changes do not partially advance accepted meaning. Names and read-only projections
grant neither mutation nor execution authority.

Public operations provide revision-bound inspection, bounded owner/relation/context
queries, complete local-function definition projection, checked changes, graph tests,
builds and execution. Exact offline package export/staging transports the complete
implementation closure. Libraries retain exact dependencies, visibility, types,
effects and callable provenance after their authoring projects are removed.

The maintained [standard package](../packages/standard/README.md) owns reusable
ordinary graph functions and exact platform interfaces. [lkjournal](../applications/lkjournal/README.md)
owns its HTTP service, interactive subscriptions, durable worker and application
data policy. Their maintained owners record exact current identities and generated
assets. Public builds regenerate those assets; contributor inspection does not
write accepted meaning.

Native user authoring and all eight native reference-page targets are implemented.
Rust remains the supported parser/kernel/platform boundary and implements
contributor/release orchestration and host observation. Guide rendering and the
required repository-policy decisions now have native owners. This is native-tool adoption,
not compiler self-hosting or a claim that development already runs entirely in
lkjscript.

## Language and runtime boundary

Current language facilities include explicit generic records/variants, finite
recursive nominal data, rank-one type/effect/requirement parameters, named pure/task
callables, prefix binding and capture-safe retained values. Standard libraries
provide graph-owned folds, maps, composition, binary64 operations and typed data.
Persistent lists and ordered maps share retained structure while preserving logical
values. [Performance evidence](performance.md) separates execution, preparation,
allocation accounting and measured workloads; it makes no model-token or cost claim.

`run TARGET` executes pure project commands. `run --deployment PATH` executes pure
or task command bundles once under exact grants; service, worker and structured
session runners load standalone artifacts without opening an authoring project.
Current allowances, callable effects and deployment grants are checked separately.
Owned processes/tasks are joined on exit, and live effects are never replayed for
differential verification. Optional quotas and deadlines are distinct from ordinary
trusted execution, which has no invented cumulative instruction budget.

First-party ordered data and durable queues provide local transactions, conditional
updates, snapshots and operational backup/restore. `transaction-outcome` returns
the actual completed decision for its store. Object storage and independent
capabilities retain their own effect boundaries. Cancellation, missing output or
cleanup failure does not imply rollback or safe retry.

The inbound listener is plaintext. Outbound HTTPS uses an exact deployment endpoint
and explicit trust/address policy. Safe Rust, static linkage and resource limits do
not establish a hostile-code sandbox, encrypted storage or tenant isolation.

## Current limits and unproved properties

- Dependencies are exact offline closures. Network publication/resolution, mutable
  package registries and automatic upgrade selection are absent.
- Canonical `change draft` is available; project history/backup/restore/doctor and
  arbitrary source-query workflows remain separate, unimplemented facilities.
  Public declaration/signature edits and extraction have their discovered contracts;
  general move/inline and broader transformations are not implied.
- Affine resources remain lexical and task-local, with the supported exact
  requirement-bound private consume handoff. General resource results, borrowing
  signatures, cross-package transfer and asynchronous ownership are unproved.
- Anonymous closures, automatic free-variable capture, generic inference, expanding
  nominal instantiation, JIT/AOT specialization and SIMD are not implemented by the
  current explicit abstraction facilities.
- Arbitrary outbound URLs/methods, outbound WebSockets, Nostr event signing and
  reconnect/replay policy are not provided by the closed relay-information recipe.
- The admitted 100,100-owner lifecycle and one-million independent-module capacity
  workload do not establish million-owner compilation, operational-data scale or
  long-history retention. No graph/data garbage collection or compaction is selected.
- Data support is one local trusted Linux host. Replication, consensus, encryption,
  additional binary targets, minimum-kernel guarantees and universal portability
  remain unproved.

Exact finite input, output, preparation and runtime resource bounds are discoverable
through `lkjscript capabilities`. Admission limits are not demonstrated scale ceilings.
[Roadmap](roadmap.md) contains contingent directions, each requiring a concrete workload.

## Verification

[Verification specification](spec/verification.md) owns proof obligations. Standalone
`check full --fresh` has 26 gates. Release acceptance uses the dependency-complete
union of 20 fresh source gates, six behavioral owners against the executable extracted
from the final archive, both pinned userlands, and installation/recovery. Transferred
and anonymous acquisitions require identity/admission and a small installed lifecycle;
they do not repeat broad semantic acceptance merely because bytes crossed a boundary.

Source/reference, pure differential tests, public copied-executable workflows,
maintained consumers and live adapter observations have distinct scopes. The
[selected evidence owner](evidence/202609221952-cells/README.md) reports fresh,
failed, reused and pending proof at its actual source. Reporting descendants do not
inherit a predecessor's tested SHA or turn a failed candidate into acceptance.
Detailed older measurements, failed histories and release identities remain at
their linked campaign/evidence owners; this overview does not duplicate them.
