# Current status

Status date: 2026-09-23 UTC. This page describes current product boundaries and
outstanding delivery. [Generated guides](generated/operations.md) own public
operations, [specifications](spec/) own semantics, and [campaigns](campaigns/)
retain detailed implementation and verification history.

## Command input files

Successor development v0.1.42 adds `--arguments-file PATH` to project and standalone
command execution. It removes the observed OS argument-vector barrier for larger
JSON inputs while retaining the existing 1 MiB byte limit, strict decoder, typed
admission and effect ordering. The selectors are mutually exclusive; omitted input
remains `[]`. Relative files use the invocation directory and must be regular with
no final symlink. The [native library guide](guides/native-library.md#read-arguments-from-a-file)
shows both forms. Final-candidate acceptance and delivery of this successor remain
pending; the accepted v0.1.41 candidate retains its frozen source and scope.

The successor also shares immutable pack indexes across validated object reads.
It uses one accepted source view for preparation, removing a redundant repository
opening while retaining subsequent authority rechecks. Source
`a1c462b521bdeab7c00797a0ebf329f1b3af57d2` passes all 20 fresh source gates with
stable inputs and zero reuse, and reached remote main unchanged. The
[matched public measurements](performance.md#shared-pack-index-comparison) retain
slower cases and establish no general speedup or history-scale guarantee.

## Composable typed-cell updates

The [selected milestone](campaigns/202609221952.md) is implemented on main for
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

Candidate
[35725872480/1](https://github.com/lkjsxc/lkjscript/actions/runs/35725872480) failed
at a stale service-verifier artifact pin. The [autonomous continuation](campaigns/202609222330.md)
repairs that pin, adds early regression coverage and retains diagnostic originals.
Replacement [35798767360/1](https://github.com/lkjsxc/lkjscript/actions/runs/35798767360)
was cancelled during source acceptance after a native library tutorial reproduced
a discovery defect: four required nullable deployment fields were advertised as
omittable. The correction preserves decoder/runtime behavior; original failed and
cancelled evidence remains at the continuation owner.

Corrected source `346c0366bde29151952a19332cb540681ad7bc7c` passes all 20 fresh
source gates with stable inputs and zero reuse, and reached remote main unchanged.
Candidate [35801102943/1](https://github.com/lkjsxc/lkjscript/actions/runs/35801102943)
completed source, finalized-candidate and original-reader acceptance at that exact
product/controller source. The annotated v0.1.41 tag and scoped selection bind it.
Promotion [35809096928/1](https://github.com/lkjsxc/lkjscript/actions/runs/35809096928)
is running; immutable publication and anonymous public verification remain pending. The new
[native library guide](guides/native-library.md) is exercised on published v0.1.40;
its generic library, exact import and standalone execution need no compiler checkout.

## Public binary release

The current published product is immutable
[v0.1.40](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.40), source
`1cdaf72888a1f46359a6d38956050747335f8e32`. Producer `35685667968/1` passed source,
final-target and installation acceptance. Promotion `35711837606/1` published
the same assets and passed anonymous exact/latest installed verification. The
[delivery record](campaigns/202609221813.md) retains original identities and expiry.
The bounded [public native-authoring observation](evidence/202609220955-native/public-v0.1.40/README.md)
also passes creation, canonical re-entry without the original input, reviewed
identity-preserving edit and detached old/new execution. It is a designed witness,
separate from maintained adoption or an independent-agent experiment.

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

Native user authoring is implemented. Rust remains the parser/kernel/adapter and
contributor/release-tool implementation boundary. This does not establish native
tool implementation or compiler self-hosting.

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
