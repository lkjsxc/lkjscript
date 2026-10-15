# Verification and evidence

Status: normative. Verify claims at their actual boundary with independent
expectations, complete admission and honest terminal outcomes. Only current HEAD
formats are supported; regenerate affected maintained inputs after format cuts.

## Verification profiles and receipts

At the canonical root of clean committed source:

```sh
verifier_root=$(bash tools/bootstrap-verifier.sh)
CARGO_TARGET_DIR="$PWD/target" "$verifier_root/lkjscript-dev" check changed --machine
CARGO_TARGET_DIR="$PWD/target" "$verifier_root/lkjscript-dev" check full --fresh --machine
"$verifier_root/lkjscript-dev" release target
```

[Bootstrap](../../tools/bootstrap-verifier.sh) retains exact immutable checker plus
original source/context/Cargo evidence in a separate cold target. The explicit
bootstrap profile uses opt-level 2 without LTO or debug data. Options:
--output ABSENT_ABSOLUTE_ROOT, --expected-source COMMIT, --with-controller.
Unsupported effective Cargo configs/compiler/flags/environment overrides refuse
without revealing secrets. Bootstrap neither certifies existing check outputs nor
authorizes cleanup. The checker remains bound to its compiled checkout. Set checker
--jobs and CARGO_BUILD_JOBS separately; join target owners before cleanup.

| Profile | Boundary |
| --- | --- |
| focused | Narrow format/library/public CLI/service development proof |
| changed | Current Git status and complete dependencies, not committed ranges |
| product | Release CLI/copied product/packages/maintained assets/discovery |
| service | Distributed/stateful/outbound HTTP and lkjournal service |
| full --fresh | Complete standalone 29-gate acceptance |
| release-source --fresh | 23 source gates plus required final-candidate union below |

[Registry](../../tools/lkjscript-dev/src/check/registry.rs) and
[selection](../../tools/lkjscript-dev/src/check/snapshot.rs) own exact commands, DAG,
bounds and changed-path widening. Preserve all gates: safe Rust/lints, workspace
all-target/all-feature/reference tests, checker/default release lifecycle, policy,
guides, maintained assets and native planner. Six behavioral owners excluded from
release-source run once against final bytes. Counts/labels never replace outcomes.

Selection covers every Git path state and mixed unions. Product/executable/example/
unclassified tool inputs widen to full; catalog prose includes site tests, checker
edits include library tests and other prose retains policy/diff. Preserve no-fail-fast
and optimized test guards; test-debug remains separate interactive evidence.
See [development verification](../development-verification.md).

Release production uses thin LTO and 16 codegen units. release_build owns optimized
product bytes; release_command_lifecycle uses the source-profile harness with an
explicit admitted release candidate. Its Cargo debug output and candidate proof
remain separate. Checker/site retain workspace Clippy/tests.

Receipts bind complete source/input/DAG/command/toolchain/environment/dependency/
verifier originals, bounded logs and immutable outputs. Honor producers: later Cargo
features may replace shared paths. Reuse needs identical complete context and a
permitting profile; final full/source evidence is fresh. Source admission accepts
only maintained full or release-source originals: re-admit every producer gate
under identical input/DAG/runtime bindings, then count the required 23-gate closure.
Never rerun source after full or rewrite its receipt; final-byte proof stays mandatory.
Drift, absent originals, skipped/unrun/failed/cancelled/unavailable work or unjoined
cleanup cannot pass.
Reporting descendants get actual scoped checks without relabelling product proof.
Keep logs/binaries in .artifacts, not tracked diaries.

## Independent correctness mechanisms

Complete typed meaning reconstruction checks sparse reads/changes/interfaces/
compiler impact/queries. Normalize only nonsemantic layout/cache/request spelling.
Production uses normalized bytecode; reference derives signatures/layouts/types/
targets/tests/blobs from canonical owners, independently of compiler/resolver tables.
Compiled-table removal must leave reference behavior intact; omitted compiled tests
cause inventory disagreement. Pure commands/graph tests need VM/reference equality
and independent expectations. Differential effects use neutral adapters; live effects
execute once.

| Mechanism | Independent oracle |
| --- | --- |
| Affinity/ownership | Finite provenance/borrow/consume/branch/recursion model |
| Effects/grants | Finite-bitset rows, scripted traces and exact requirement tables |
| Packages | Complete graph/transport owners/types/private bodies/edges/interfaces |
| Queries/extraction | Typed preorder/relations/captures/movable-owner derivation |
| HTTP routes | Segment language/overlap/specificity/capture model and raw TCP |
| Lists/maps | Vec/BTreeMap histories, physical identities and reservation arithmetic |
| Catalog | Strict pack-footer enumeration and packing-independent commitment |
| Recursive proof | Simultaneous substitution/weighted closure, not production SCC |
| Native planner | Complete outcome checker, fixed graphs and false-plan controls |

Neutral decoding/digests may be shared; production algorithms cannot generate their
own oracle. Required faults must fail discriminating tests, then restored controls
pass. Consistent rehashing cannot launder semantic/authority/source-code violations.
The following rules apply to every feature and existing case indexed below:

- Admit full closures, including unused/untaken/phantom/inactive/last-invalid inputs.
  Preserve scope/order/types/witnesses/provenance/requirements/identity through every
  canonical/package/compiler/artifact/runtime boundary.
- Raw inputs/results/captures reject before callbacks/effects/secrets. Types alone
  certify no values. Strict independent loaders check canonical/code metadata and
  cleanup; neutral valid reconstructions remain accepted beside hostile controls.
- Reserve before growth/mutation. Exact fit passes, N-1 refuses. Semantic invalidity,
  preparation capacity, quotas, cancellation, deadlines and saturation differ. No
  refunds/resets/limit increases may hide missing accounting.
- Rejections preserve accepted authority. Progress-triggered trap/cancel/refusal and
  healthy reuse release children/loans before ancestors, join work, retain unrelated
  custody, expose no partial success and preserve primary plus cleanup errors.
- Public claims need literal ordinary authoring, drafts/reviewed edits, exact
  transports and complete independently expected detached results after removing
  sources. Unit-only fixtures prove no copied-binary/final-byte capability.
- Observe live effects once. Missing output/cancel/cleanup/encoding failure proves
  neither rollback nor retry safety; receipt checking never replays effects.

Checked forwarding/binding keeps N={1,256,4096}, K={1,64,1024}, direct/imported paths
in both evaluators. Input admission is K-independent, internal guard-descendants
zero and fixed-K classification N-independent. For W=input/raw-result/capture
nodes+constructor-child visits+guard descendants:
`W(N,K)-W(1,K)=W(N,1)-W(1,1)`. Moving scans elsewhere cannot evade W. Restored scans/
bad classification fail independent cost/value oracles. Count rejecting/repeated
shared nodes and metadata honestly; observers cannot alter limits/authority.
Cumulative modeled bytes/charges/items, reference source/type/expression work, live
memory, allocator work and RSS are distinct dimensions.

## Reviewed concurrent change obligations

[Concurrent changes](concurrent-changes.md) retain two same-base literal candidates:
publish first, reject stale second, refresh unchanged intent and apply complete
expected result/created identities, then detach. Cover disjoint same-module/shared
type edits and full positive/negative name/dependency/retirement/ownership/relation
guards including initially empty ranges. Renew full target validation/impact/tests.
Races accept one; accepted keyed retries preserve originals after descendants.
Changed tokens/requests/origins, corrupt/disconnected ancestry, incomplete footprints,
combined budget/cancel and staging/exposure interruptions cannot partly publish.
Strict plan round-trips bind authored reads and refresh origin. Every native_refresh
final-byte case remains required.

## Structured parallel ownership

[Parallel](structured-parallel.md) independently checks callable kind, allowances,
grants, generic bounds and separate Transferable/Shareable. Ordinary results use
ordinary records; mixed/owned results use owned products. Joined reads require
Shareable/source custody and cannot return loans. Rehashed bounds/target/witness/
result/instruction attacks reject. Observe actual distinct production threads and
original allocation identity through generic/mixed/nested parent-child return;
scheduler fixtures/serial equality alone do not prove physical handoff.

Zero/one/full workers, inline fallback, nesting, exact resource bounds and all
prepare/adopt/result/trap/cancel/deadline/unwind paths join descendants before
capacity/completion/lender reclamation. No lock spans user code. Reusable workers
start lazily within demand/ceiling, retain physical identity and private job state.
Idle handoff requires result destruction, one join owner, bounded slots/balanced
custody counts and shutdown/recipient races. Former owners cannot cancel transferred
work/retain roots; destructor failure is no successful shutdown. Complete generic
library/worker/consumer, source-deleted capacity-one and shared-service witnesses
retain independent payloads, HTTP siblings, repeated dispatch/cancel/restart.
Custody observations prove no speedup/fairness/cross-process sharing.

## Command lifecycle requirements

Copied-product lifecycle outside source: discover/create→status/inspect/query→reviewed
plan/apply→check→deterministic build→pure run, with exact standard/test selection and
no Cargo/checkout/network/template semantic helpers. Read-only/failure routes preserve
HEAD. Clean/current/incremental builds match; bad caches select no wrong meaning;
post-acceptance derived failure remains distinct. Cold maintained copies omit only
LOCK/catalog: first recovery preserves preexisting bytes, creates only allowed lock/
catalog files; steady-state inventory stays unchanged. Each read entry has cold proof.

[Public CLI](../../tests/public_cli.rs) and offline owners retain every strict input,
output and filesystem boundary case. Argument files share inline JSON/types, exact
1 MiB/one-over, large Text, UTF-8/duplicates/trailing/selectors and regular-file
admission before context/secrets. Result files retain exact typed JSON, Unicode/F64/
large/limit cases, create-new/late-conflict ownership and no partial output/debris.
Completed effects survive output failure; broken stdout preserves published files.
Retain detached 33,333-key/99,999-item success versus next-key item refusal.

Discovery/native authoring covers resources, sessions, routes, complete paginated
definitions and extract.function. Independent digests/captures/effects/requirements/
movable identities, direct/file parity, reviewed body edits and invalid/stale tokens
remain required. Raw HTTP proves precedence/ordered captures/no-effect 404/restart.
Foreground transported iteration/bound callbacks retain recursive reports, exact
N(N-1)/2 results, once-only counters, old/new detached bundles, grants, rollback
versus committed prefixes, signals/deadlines/quotas/output faults. Trusted unmetered
312,499-step scalar execution exceeds ten million instructions; quota policy stays
separate. Live effects are never replayed.

## Maintained consumers and service boundary

Standard/lkjournal differential checks/deterministic builds match maintained assets
and exact dependency/interface/target/test inventories. Built-in product exports
match separately generated standard artifact/transport bytes. Prove tracked-source
cold reproduction of required packs through supported owners.

[Service owner](../../tools/lkjscript-dev/src/service.rs) runs copied bundle/descriptors
with isolated data, serve and two workers, never repository authority or external DB.
Retain complete auth/actor/resource/queue/recovery/failure/cleanup matrices and
independent raw transitions. Copied entry/consume/borrow/largest definitions preserve
exact provenance, single consumption, pagination and ≤40 helper body records.

## Exact release-target admission

[Target policy](../../tools/lkjscript-dev/src/release/target.rs) owns
x86_64-unknown-linux-musl pins/images/bounds. Independently admit regular non-symlink
little-endian ELF64 x86-64 without PT_INTERP/DT_NEEDED/GLIBC requirements; malformed/
trailing/dynamic/foreign/contradictory bytes reject without ldd. Extracted final bytes
pass both pinned musl/older-glibc Linux/amd64 userlands without network/host library
mounts, plus all six owners. Bind source/policy/bytes/mode/outcomes/cleanup. Host
proof cannot substitute; this proves neither universal portability nor sandboxing.

## Transferable application evidence

`lkjscript-dev offline-packages --binary PATH --evidence-root ABSENT_ABSOLUTE_PATH
--machine` uses copied product/closed environment outside source, no Cargo/semantic
helpers. Its 3,600-second process allowance includes cleanup/evidence/self-admission;
parent original-reader admission is additional. Focused --case is not aggregate.
The five-package/eight-edge diamond retains 11 after D2 staging, mixed-closure refusal,
12 after paired replacement and 28 tests once. Pre-export graph equals transport.
All existing offline language/visibility/repair/source-deletion matrices remain.

Six owners remain independently required: distributed-http, outbound-http,
offline-packages, pure-tail, stateful-http, service. Transferred roots are absent
absolute paths outside checkouts, without source discovery/helpers; bind verifier/
candidate/copy/context/commands/logs/cleanup and label source-only probes.
Stateful native authoring cannot use fixture topology. Independent raw HTTP/WebSocket/
TLS clients retain complete existing protocol/effect/authority/bounds/recovery
matrices, never application policy, live relays or automatic GET replay.
Contributor-only PostgreSQL 16.15 comparison remains separate: warm-up/three samples,
>5x wall/>2x RSS/>4x durable bytes reject; it is no product/release dependency.

Pure-tail retains copied author/export/check/build/detached execution, eight-frame
VM/reference ceiling through 8,192 items, independent canonical trampoline and 2 MiB
thread failure probes. Genuine nontail/frame/fuel controls, allocation/operand exact
limits, cancellation and transaction rollback/recovery discriminate false reuse.
Exact-self-tail slot reuse needs physical buffer identity, dirty-slot clearing,
complete pre-mutation argument/callee admission and excluded task/borrow/transaction/
read-view fallback. Preserve independent ledgers and no-refund accounting; physical
reuse proves no throughput/RSS claim. The pure-tail owner retains 900-second/1 GiB
owned-file bounds, complete mapping/forwarding/capture matrices and original readers.

## Package and public-release evidence

Native planning must own real cycle refusal/compilation order; keep independent
artifact host topology checks and nonrecursive bootstrap. Complete 4,635 outcomes,
false plans, unreachable invalid input, ordered cycles/frontiers/chains and inclusive
10,000 nodes/roots/100,000 edges remain. Large JSON uses explicit bounded embedding,
never raised CLI defaults. Native input cancel, concurrent preparation/init/encoding/
cleanup failures recover. Reuse rereads complete private context under current bounds;
fresh/reused inventories/bytes/work/exact-N-1 agree, invalid entries cannot publish.
Separate operations/containers plan fresh. Native source check/build/exact asset and
public detached diamonds prove actual compiler adoption.

### Source and finalized-byte coverage

Fresh release-source plus exact-final-candidate admission is release acceptance;
full remains standalone complete. Main integration can precede hosted completion
only with that release gate explicit.

| Claim | Required owner |
| --- | --- |
| Source safety/reference/checker/default lifecycle/assets/discovery | Fresh release-source DAG and original reader |
| Native public semantic/ownership/Map/fold/compiler/storage behavior | Source-matched exact-final-byte public harness |
| HTTP/offline/pure-tail/service | Six owners against extracted executable |
| Static target/pinned userlands | Independent ELF/exact target |
| Slots/locking/install/recovery | Native workspace plus candidate installation tier |
| Transfer/anonymous exact/latest/small installed lifecycle | Authenticated pair/exact readers and public acquisition |
| Whole candidate/publication completion | Original readers and authenticated terminal controller |

[Public inventory](../../tools/lkjscript-dev/src/release/candidate/public_inventory.rs)
and siblings own exact required identities. All current/future Map, native-fold,
incremental-unit, FIFO, data-sequence, owned-nominal, indexed-data, associated-BlockRead/
read-view namespaces and native-owned/buffer/range/policy/parallel/read/refresh/topology
families remain required. Inventory must exist before execution; absent/substituted/
duplicate/ignored/failed/unrun cases reject even with exit 0/forged summary. Notices
supply no pass credit; similar names/cost predicates cannot replace complete matrices.
Cargo JSON selects source-matched public_cli, immutably copied and executed outside
source under closed environment with explicit final-candidate path; no fallback.
Check source/candidate/harness before/after, exact outcomes/logs and joined cleanup.
[Candidate owner](../../tools/lkjscript-dev/src/release/candidate.rs) owns the sole
current acceptance/workload pair; mixed terminal identities reject.

Finalize/extract/admit without rebuilding. [Release](../release.md) owns independent
producer/source/run/artifact authentication, current authority, unchanged promotion
and retention. Readers reject omitted/substituted/rehashed false evidence without
effect replay; required ignored faults count only after execution. Transfer/public
rebuild no product and replay no broad owners. Installed exact/latest retain literal
structural/F64 review results 42→43, 1.75→2.75, -0.0/hello, wrong-review rejection and
detached 43; project pure results compare reference. Simulated URLs cannot prove
anonymous acquisition; exact-only cannot prove latest or execute different bytes.

## Contributor process completion

Direct-child exit and pipe EOF differ. Every contributor/probe/transferred route
retains nonblocking bounded both-stream reads, deadline/controls through inherited
writers, exact/one-over/final-burst output admission and preserved exhaustion.
Keep child waitable through signalling, then reap. Every path attempts descendant
termination/EOF/log sync with separate five-second cleanup; missing EOF/survivors
fail while preserving primary/cleanup errors. Track owned PID/start through
reparenting; observation errors retain ownership, recycled PID grants none and
signals require identity/pidfd. Unique traversal stays within 4,096 processes/depth 64;
thread/procfs bounds differ. Unsampled escaped children/blocking kernel calls prove
no hostile containment/hard host deadline. Git/toolchain probes retain 30 seconds.
Existing inherited-stream/post-exit/reaping/groups/reparent/log/pre-spawn faults and
healthy recovery tests remain required.

## Performance evidence

Compare exact meaning/artifact/input/workload, recording source/executable/context/
warmth and preparation/invocation/encoding/sample order. Retain all required create/
check/build/current/incremental=clean/run/standard/lkjournal observations, separating
wall/CPU/RSS/live/cumulative allocation/compiler units/bytes/I/O/publications and
failures. Current-cache work must decrease; incremental respects impact/locator
bounds. Measurements prove no API savings and replace no gates. Share immutable
complete-context fixtures, never effects/grants/quotas. Preserve full matrices and
real boundary smokes; SLA claims require complete local/hosted measurements.

## Native installation and release bootstrap

Independent current containers exercise public native parser/member/mode/checksum/
ELF/identity/slot/prefix/lock/symlink/I/O/process faults without release preparation.
Failures preserve slots/live-runtime selection. Genuine final assets retain deterministic
bootstrap/both userlands/installed authoring/retained-process recovery. Anonymous
exact/latest independently admits triple/attestations and runs original script/small
lifecycle. Original/nested readers retain omission/substitution/rehashed faults and
restored controls before terminal. Installation grants no authority/migration/fallback;
published history stays immutable, obsolete readers are no development requirement.

## Language and runtime witnesses

The current feature specs and executable owners below retain their complete
positive/negative, boundary, independent-oracle and fault-sensitivity matrices.
This index applies the common obligations above; it does not narrow those matrices.

| Feature | Contract and maintained public owner |
| --- | --- |
| Buffers | [Contract](owned-byte-buffers.md), [public tests](../../tests/public_cli/native_byte_buffer.rs): custody, freeze identity, bounded builders and drops |
| Owned generics | [Contract](owned-generics.md), [public tests](../../tests/public_cli/native_owned_witnesses.rs): symbolic complete admission and exact same-Self witnesses |
| Parameterized Worklist | [Contract](owned-contract-parameters.md), [parameters](../../tests/public_cli/native_owned_parameters.rs), [worklists](../../tests/public_cli/native_owned_worklists.rs): flat/chunked boundary and graph matrices |
| Sequences | [Contract](owned-sequences.md), [public tests](../../tests/public_cli/native_owned_sequences.rs): LIFO, bounded control, reservations and disposal |
| Parallel reads | [Contract](structured-parallel.md), [public tests](../../tests/public_cli/native_parallel_reads.rs): exact leases, symmetric conflicts, joined custody and bounded seals |
| Structural syntax | [Contract](language.md), [public tests](../../tests/public_cli/native_forms.rs): independent flat/block parity, scope, locations and bounds |
| F64/effects/requirements | [Language](language.md), [effects](owned-effects.md), [offline owner](../../tools/lkjscript-dev/src/offline_packages.rs): arithmetic, codecs, transactions, grants and visibility |

Transaction completion distinguishes commit, condition abort and conflict. Empty/
read-only/false-valued success is not an abort. Barrier-synchronized two-writer
updates from one base yield exactly one commit/conflict, two callbacks and one
physical revision; only winning bytes remain after reopen. Canonical aliases may
participate only in their live owner scope; malformed/missing authority rejects
before callbacks. Failed conditions permit further staging but no publication.
Other-store completed effects survive parent abort. Credit/stock, DurableQueue,
exact byte/store observers and reviewed transported old/new bundles remain required.

## Same-invocation task-owned transfer

[Task transfer](owned-task-transfers.md) and [public tests](../../tests/public_cli/native_owned_tasks.rs)
retain imported/generic custody, resource suffixes and exact witnesses/method contracts.
Synchronous borrows pass; bad order, unrestricted owners, duplicate moves, wrong
witnesses and pure→task calls reject, including inactive/raw/rehashing attacks.
Allowances/grants remain independent; all exits join custody and recover. This
proves neither asynchronous transfer nor exactly-once effects.

## Generic owned-implementation obligations

[Generic schemes](generic-owned-implementations.md) and [public tests](../../tests/public_cli/native_owned_generic_implementations.rs)
require full symbolic mapping admission before carriers, without item-specific
wrappers. Preserve stable ties, distinct same-Self readers, chunk boundaries,
513-item results and larger aggregate refusal. Growing cycles reject even untaken;
finite/permuting controls pass. Witnesses grant no effects; forged mappings reject.

## Mixed storage-contract obligations

[Mixed contracts](owned-contract-parameters.md) admit every category/bound/scope/
phantom actual: ordinary formals cannot conceal owners or borrow/consume them.
IndexedData exports before suppliers. Retained/displaced values survive disposal;
evaluation order, signed indices, module privacy and reservations remain exact.
All four integer/Bool planner combinations retain independent outcomes/capacities.
Checked indexing cannot scan/copy prefixes. Require complete indexed-data final
inventory, including category/mode/source forgeries and cleanup.

## Scoped owned-child read obligations

[Lexical reads](owned-borrows.md) independently derive source/ancestor guards and
complete symbolic/unused admission. Detached results preserve consumable originals.
Scopes retain activation through reborrow/recursion. Protected consumption, escapes,
invalid arms/methods and extraction reject, as do rehashed guard/source/cleanup/tail
mutations. Release youngest loans before custodians without user-code storage locks.

## Source-tied borrowed result obligations

[Borrowed results](owned-read-results.md) and [public tests](../../tests/public_cli/native_owned_read_results.rs)
retain exact source/method position, mode/type/guard chain. Four flat/chunk32 cell/
buffer witnesses preserve stable first ties, identity observation, LIFO drain/reuse
and complete detached boundary results. Wrong-source/owning/escaping/ordinary/codec
or task-loan results reject. Guard reservation/cancel/adoption failures retain one
cleanup owner; child loans end before ancestors and activation replacement cannot
erase cleanup. Empty access traps and healthy reuse remain required.

## Recursive callable proof

[Context proof](../decisions/context-scoped-callable-proof.md),
[demanded provenance](../decisions/demanded-callable-provenance.md) and
[operation-local inputs](../decisions/operation-local-callable-inputs.md) retain exact
SCC/provenance: equal shapes cannot merge authority. Only irrelevant growth edges
may be omitted. Independent unpruned/eager/weighted, exact/N-1 and cancellation
controls remain under unchanged limits. Read each demanded mapping once despite
convergence, never project undemanded inputs or share operation/context entries.
Reserve first; failed entries cannot publish and final handles reclaim storage.
Mandatory compact/demanded public witnesses retain generic-first authoring,
review/edit/restore, unsafe-unused rejection and complete detached behavior.

## Checked Map-entry projection

The exact native_map_entries::native_map_entries_preserve_results_and_linear_projection_work_after_detachment
matrix and every Map case are mandatory. Explicit admitted candidate selection and
before/after copy/execution checks exclude fallbacks. Public author/check/build then
source/transport removal precedes 8 sizes {0,1,31,32,33,256,1024,4096}, 4 insertion/update
patterns, 3 repetition counts {0,1,8}, enumeration/control routes and 192 complete
comparisons. Compare every ordered key/payload/replacement, complete overwritten/
unused raw inputs and independent BTreeMap/work bounds, refusal/recovery/cleanup.
Cost predicates/similar names cannot substitute.

## Native blocked fold composition and final-byte evidence

Exact native_fold family retains all five scalar/detached, wrong-contract,
complete-input-before-callback, nested-history and unused-initial-history cases.
Export supplier before consumer; ordinary graph folds are no intrinsic. Independent
prefix slicing/standard-fold controls cover empty/seeded state and block/remainder/
32-way boundaries; later appends preserve old prefixes, empty preserves full seed.
Malformed unused/deep input rejects even empty; traps expose no partial result.
Repeat detached matrices with unchanged artifacts after both sources/transport vanish.

## Exact incremental compiler-unit acceptance

Impact is before∪candidate enclosing units, not all checked owners; ports belong
to components, moved children affect both parents and deleted children keep before
paths. Retain domain/work/cycle guards and full incremental manifest/artifact=clean
across add/rebind/move/delete. Public exact counts/updated cache/exact-current checks
cannot silently clean-recover. Mandatory incremental_units cases cover one/two-port
Command detached results, malformed port authority/cache preservation/recovery and
native-planner exact assets/independent detached dependency diamonds.

## Declared Cargo executable origin

[Origin owners](../../tools/lkjscript-dev/src/check/cargo_origin.rs) authenticate complete
selected workspace units of direct Cargo build/test declared-output producers.
JSON precedes --; exactly one root non-test binary artifact per output matches
executable/filenames and successful build-finished. Units bind package/manifest,
target/kind/profile/test/features/all file proofs. Missing/ambiguous/malformed records
and unsupported build-script/proc-macro/generated closures refuse. Test stdout
cannot supply artifacts.

Only actual fresh:false establishes cold origin. Ordinary fresh:true units need exact
cold originals matching source/lock, checkout/physical target, runtime/compiler,
profile/features/config and live bytes. Warm references point directly cold; a cold
binary cannot bless unknown warm libraries. Unknown/changed/foreign/evicted originals
refuse without automatic cleaning/retry. Bootstrap seeds no check origins.

The root schema generator is a narrow derived-input exception: retain its selected
executable and literal freshness without minting a cold origin. Require its exact
source-bound Cargo execution record, empty linker/cfg/environment outputs and one
canonical_schema.rs in its target-owned OUT_DIR. Compare generated bytes literally
with the immutable checker's compiled source at capture and replay; retain that file
and Cargo's original script stdout with every root unit. Missing, foreign, extra or
tampered inputs refuse. Other workspace generators remain unsupported.

Context binds approved environment, Cargo/compiler entry/resolution/sysroot rustc
bytes and config/config.toml contents/absence on actual cwd ancestors/Cargo home,
without exposing secrets. Closed grammar refuses wrappers/custom targets/overrides/
unsupported routing. All Cargo writers except fmt use canonical owned target;
external regular targets pass, aliases reject. Physical-target lease outside removable
tree plus same-run writer mutex span observation→durable dirty→execute/join→retain/
recheck→commit. Failed/abandoned/drifted/incomplete writers cannot publish clean state.
Nonproducer Clippy/workspace_tests serialize/recheck but do not admit compiled
unit origins; empty governed targets supply their final operational cold start.

Pin original runs with shared ACTIVE.lock; deletion needs exclusive lease. Admit
full independently hashed cold/warm sidecar closure and retained bytes, even when
live paths were replaced. Bounds: 2 GiB/file, 8 GiB/live, 16 GiB/retained reads, 256 units,
1,024 history records, 16 MiB sidecar/ledger, 64 MiB closure metadata, 256 originals,
depth 32, 64 MiB/process stream, 256 MiB closure logs; charge before I/O. These bounds
prove no adequacy/performance. This cooperating-host contract does not cover
uncoordinated writers, hostile builds or arbitrary prepopulated compiled tests.

## Generalized sequence storage and exact final-byte families

[Sequences](owned-sequences.md) stay affine with ordinary elements. Old immutable
values survive storage replacement/disposal/transfer; displaced owners retain exact
custody. Index runs once, bounds/reservations precede swap, raw mutation invalidates
admission and checked updates cannot scan prefixes. Independent planner and exact
FIFO/data-sequence inventory owners retain all outcomes, capacities, negative cases,
detached results, peek/LIFO adversaries and failure cleanup.

## Nominal owned records and final-byte composition

[Owned records](owned-records.md) retain intrinsic affinity, full/phantom layout and
depth/cycle admission, exact defining-module privacy and source-independent witnessed
abstraction. Shape or transported private bytes grant no field rights. FIFO keeps
exhaustive interleavings, guarded views, drain/reuse and every final-byte witness.
Preserve child allocations, one custody envelope, no nominal prefix scan and full
failure cleanup; count repacking/reversal separately.

## Checked consuming extraction

Extraction requires exact admitted parent program/type/layout/current owner/zero
loans. Raw/stale/foreign/same-shaped/inert/borrowed/protected/mutation-restored parents
and interrupted custody reject. Ordinary fields survive parent destruction without
loans/custody/descendant rescans; owned fields retain exact checks. Partial delivery/
cancel releases pending children and installed locals; reference admission remains
independent. Detached mode 2 raw-result work ≤ K*(N+2) with complete inputs must reject
doubled work and underchecking; pack/drop and borrowed projection are distinct
controls. Indexed-data witnesses retain full displaced nested values after disposal.

## Associated Owned BlockRead and final-byte coverage

The exact native_declarations::native_associated_blockread::native_associated_blockread_consumer_first_four_literals_three_targets_and_detached_bytes
case exports contract/consumer before flat/private nominal suppliers. Public
transport/draft/replan/artifact equality preserves witness identity; 4 literals × 3
targets × attached/detached require 24 complete results. Protected consume/escape/private
foreign-field refusals preserve HEAD; detached cleanup leaves no handles/effects.
Require exact identity and every future namespace case.

The [read-view inventory](../../tools/lkjscript-dev/src/release/candidate/public_inventory_read_views.rs)
additionally requires all four exact consumer-first five-literal/parent-drop/detached,
source-alias/path/category refusal, joined/projected/passive custody and trap/fuel/
subsequent-invocation witnesses. Independent complete source/category admission,
exact result-source/witness and parent/ancestor custody cannot be replaced by host
results, similarly named cases or source/full proof instead of finalized bytes.
