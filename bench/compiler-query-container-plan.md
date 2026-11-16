# Immutable semantic snapshot container: implementation and validation plan

## Declaration and scope

This plan is declared before implementation in the isolated
`compiler-query-container-20261116` worktree, based on
`a0da7d7fd98e0897856d7c94fe5622f949509c7f`.
The full gate currently owns all four compiler workers. Only this Markdown
protocol may be written until the owner of the compiler stream explicitly
releases implementation. No implementation, test or measurement result is
implied by this plan.

Text remains the stored repository projection. The semantic index remains an
authenticated producing-compiler claim; checked-IR facts retain their existing
level and source associations retain their source-claim qualification. This
change grants no current admission, image authority, gate reuse or filesystem
freshness. No new language/runtime primitive or v2 compatibility reader is
introduced.

The measured target is ordinary-store bookkeeping after an index has populated
the store: the independent 100k cohort scanned 70,173 complete 90-byte headers
and assigned 6.903 s of 7.936 s Store time to metadata
(`compiler-cache-index-condition.md`). One container replaces the page-file
population for each snapshot. Global fact construction, ordinary packs,
source discovery, checked graph and emitter work remain outside this remedy.
No cause or removal of the old million-line refusal is asserted.

## Canonical format

All numeric words below are little-endian unsigned 64-bit integers on disk.
Decode with checked conversion to host `usize`; file offsets/ends must also fit
nonnegative `i64`. No wrapping cast may turn a disk value into a seek/allocation.

| Object | Canonical filename or key | Authentication namespace / protocol |
| --- | --- | --- |
| Container | `semantic-container-v3-SLOT-SNAPSHOT` | namespace `semantic-container-v3`, raw producer, key SNAPSHOT |
| Embedded node | no separate file; key NODE_DIGEST | namespace `semantic-node-v3`, raw producer |
| Locator | `semantic-locator-v3-SNAPSHOT` | namespace `semantic-locator-v3`, protocol producer SHA256(`lkjscript-semantic-locator-3`) |
| Latest | `semantic-latest-v3-SLOT-SELECTOR` | namespace `semantic-latest-v3`, raw producer, key SELECTOR |
| Cursor | no file | namespace `semantic-cursor-3`, protocol producer SHA256(`lkjscript-semantic-cursor-3`) |

SLOT remains the existing SHA256 of the raw executable digest. SNAPSHOT and
NODE_DIGEST are lowercase 64-character hexadecimal SHA256 identities. Existing
selector version 1 is unchanged because source-selection semantics are unchanged.

The container reserves exactly 490 bytes at its front: the existing 90-byte
reuse envelope followed by a fixed 400-byte Snapshot-v3 payload. The payload is:

1. version word 3;
2. five existing length-framed, 64-byte hexadecimal fields in this exact order:
   raw producer digest, source digest, exact checked-IR digest, root-node digest,
   selector;
3. fact count;
4. root offset;
5. root envelope length;
6. complete container byte length.

Thus the payload is `8 + 5*72 + 8 + 3*8 = 400` bytes.
SNAPSHOT is SHA256 of these exact 400 bytes. The front envelope authenticates
that payload under the actual producing compiler and SNAPSHOT key.

Nodes follow the front in deterministic postorder. A node payload starts with
version word 3, kind, depth and count. Leaves preserve the current ordered
logical-key/hash/value records exactly. Branches preserve ordered radix digits;
each child contains its digit, existing framed 64-byte payload digest, offset
and envelope length. Logical fact-page version 1, its continuation word, line
order and UTF-8 fact bytes remain unchanged.

Each node is independently sealed through the existing `reuse::seal`. Its
reference is its payload digest plus physical offset and full envelope length.
The payload cap remains 32,768 bytes; the fixed node header is 32 bytes.
Branches have at most 16 children, leaves at most 64 entries, and radix depth
remains bounded by the 64-digit key hash.

The root is emitted last. Canonical framing requires:

- every node starts at or beyond 490;
- checked `offset + length <= authenticated_total`;
- envelope length is between 90+32 and 90+32,768;
- the authenticated envelope-header length plus 90 equals the reference length;
- every child ends at or before its parent's offset; all descriptors, including
  unselected children, undergo these cheap checks before choosing a branch;
  this does not require reading unselected nodes;
- `root_offset + root_length == total`;
- all existing node kind/depth/order/prefix/key hashing and logical-page
  framing/continuation checks still pass.

The builder writes a contiguous stream. The reader need not read gaps or
unreferenced bytes to answer a question; all physical bytes are still charged.
The front authenticates a root reference and total, not every page's contents.

The locator payload remains exactly 104 bytes: version 3, raw producer32,
selector32 and root digest32. SNAPSHOT is bound by its envelope key. Latest
contains the same 400-byte manifest, authenticated under the current producer.
Cursor traversal payload and public query metadata become version 3. The cursor
question-key domain becomes `lkjscript-semantic-cursor-question-3`, retaining the
existing raw-producer/question binding and snapshot/question/depth validation.

## Concrete source APIs and ownership

Transport types and the fixed manifest codec live in `store.lkj`, which must
not import `semantic_store`. The names below are the implementation contract:

```text
store::QueryExtent { digest: String, offset: usize, length: usize }

store::QueryManifest {
    identity: String, producer: String, source: String, checked_ir: String,
    root: String, selector: String, facts: usize,
    root_offset: usize, root_length: usize, total_bytes: usize
}
QueryManifest::encode() -> Vec<u8>
QueryManifest::decode(bytes: &[u8]) -> Option<QueryManifest>
QueryManifest::root_extent() -> QueryExtent

Store::begin_query_container(&mut self)
    -> Result<QueryContainerWriter<'_>, String>

QueryContainerWriter::append_node(&mut self, payload: &[u8])
    -> Result<QueryExtent, String>

QueryContainerWriter::finish(
    self, source: &str, checked_ir: &str, selector: &str,
    facts: usize, root: &QueryExtent
) -> Result<QueryManifest, String>

Store::query_container_retained(&mut self, identity: &str, total_bytes: usize)
    -> bool

QueryAccess::query_node(
    &mut self, snapshot: &str, extent: &QueryExtent
) -> Option<Vec<u8>>
```

`QueryContainerWriter` exclusively borrows its Store and privately holds the
new temporary Directory and File, stream position and cleanup state. There is
no public conversion to a writer for an existing container. This borrow forbids
interleaved ordinary puts that could bypass the pending capacity reservation.
If implementation syntax needs adjustment, record the exact API revision before
applying it; preserve this exclusive ownership invariant.

`QueryAccess::query_page(digest)` is replaced, not retained as a compatibility
alias. Existing `query_producer`, `query_bound_root`, `query_root`,
`query_latest`, cursor and invalidation roles remain. `query_root` returns only
the authenticated 400-byte front payload. Store and QueryReader privately retain
a container reader bound to one exact snapshot while serving its extents.
query_node must check its snapshot argument before reuse of a retained handle;
a pinned QueryReader refuses another snapshot, and Store must open/validate a
new selection rather than accidentally serve a different Reader's old handle.

`semantic_store::Snapshot` retains its existing public fields and adds the three
extent/total fields above. Its codec delegates to the shared transport codec;
no semantic producer facts move into Store. `semantic_store::node` returns
`QueryExtent` and appends completed nodes. `publish_emitted` retains the current
lazy logical-page serialization and full fact collection; this change does not
claim to eliminate those global allocations.

`semantic_store::open` validates the actual container front even after a latest
selection, requiring the same authenticated manifest. `Reader::node/value`
traverse extents. The private immutable snapshot remains the lifetime boundary.
Validate an extent before a raw-node cache hit, or key that cache by its complete
snapshot/offset/length/digest. The existing logical Page/None cache remains;
missing/unauthenticated data is an error, never cached absence.

The narrow driver edits are the two indexed-build paths: cached selection must
require the container and locator, and final publication must check the container
again after all later `Memo.remember` writes. The existing before-Memo explicit
pin dispatch remains, including zero current-executable hashing.

Expected production ownership is `src/store.lkj`, `src/semantic_store.lkj`,
`src/semantic_query.lkj` and coordinated small driver hooks. Fact generation,
source/IR admission and native/runtime implementation are unchanged. Every
retained .lkj change must be made through checked operations with request,
before/after manifests and complete apply output retained.

## Reader bounds, provenance and mutation

Open only through `Directory.open_regular`, retaining the descriptor. Front
inspection reads fixed-size bytes; a claimed length never drives that initial
allocation. A pinned reader already knows the authenticated raw producer from
its locator. Reconciliation can obtain the candidate raw producer from the
bounded manifest, but must then authenticate the front, verify its producer
slot/name and snapshot identity, and only then accept its fields.

Use `File.seek(0i64, 2u8)` to obtain exact physical EOF, then compare it with
the authenticated total. Its result and every reference must fit host `usize`
and nonnegative `i64`. Use exclusive seek/read access and require absolute
seek to return the requested offset. Do not duplicate handles that share the
seek position. Do not call the ordinary envelope reader's `at_end(file)` for
an embedded node; exact authenticated extent length provides that framing.

For a node, validate all cheap bounds before seek and allocation, read the
90-byte envelope header, authenticate it, require exact reference-length
agreement, allocate no more than the node cap, read exact owned bytes, verify
them and check the rooted digest. Existing VerifiedPayload construction remains
private to AuthenticatedRoot::verify. It covers one manifest or one node only.

Writer capacity and reader allocation policy are distinct. The writer charges
the complete container to --store-bytes. A reader may query a valid container
whose total exceeds its --store-bytes: each metadata/node allocation remains
bounded by the reader cap, and signed total is constrained by exact EOF and
representation bounds rather than that per-read cap. A >256 MiB container built
with a 2 GiB writer budget must remain queryable using default read settings.

Immutable-by-publication does not mean kernel sealed. A retained readonly inode
can be changed through a hardlink. Requested owned bytes always authenticate;
already verified cached bytes may still be returned for the fixed snapshot.
A mutation race may fail or return valid authenticated old bytes, not necessarily
be detected merely because a write occurred. A pathname swap must not redirect
the held descriptor. There is no filesystem-freshness or global-page-validation
claim. Every refusal precedes partial query stdout.

## Stream publication and accounting

Keep the ordinary writer lock and full directory reconciliation. No clean marker,
stat-only trust or skipped orphan discovery is introduced.

Claim a fresh temporary directory, open and retain it, and use its existing
exclusive `create_private("entry")` (0600). Retain both parent and file. Reserve
the 490-byte front, append nodes, backpatch only this newly owned file, check its
exact final EOF, and publish through the raw rename between the held temporary
Directory and held store Directory. Never reopen an existing container or an
attacker-supplied hardlink for writing. Do not re-resolve the temporary path at
the final rename.

Before the initial write and each append, checked growth includes retained
published bytes, the prospective full staged container, the prospective entry
and generation metadata, and max(current signed index bytes, prospective signed
index bytes). The final snapshot filename has a fixed length even before its
digest is known. Reserve a fresh entry conservatively; do not assume deduplication
or a replaceable old name. Existing oldest-generation eviction remains.

A stream that cannot fit aborts and cleans its stage. Do not first write a
600 MiB uncharged file and discover refusal at finish. Temporary checkpoint
copies and reader-held unlinked inodes are not newly claimed to be covered by
the published-content cap. If a smaller checkpoint is needed while staging,
preserve the current staged-byte reservation through that operation.

The budget remembers the complete physical extent, not the front's 490 bytes.
A specific container branch in writable_inner checks bounded authenticated
metadata and exact EOF against the signed StoredEntry length. A valid-looking
front with shorter/longer physical extent is rejected and disposed. This is
stronger for containers than current ordinary header-only accounting; do not
claim ordinary namespaces have gained physical-EOF validation.

Publication order: complete container rename and accounting, locator, latest,
later semantic/input memo writes, final retained-container plus authenticated
front/locator check, then printed snapshot identity. Lock failure, tight budget,
generation eviction or failed final checks refuse publication. query_pages_retained
is replaced by the container retention check; no obsolete alias remains.

A failed checkpoint may leave a valid-looking unindexed container. It is an
orphan, not an adoptable proof. Replayed/missing budget records cannot undercharge
an extant container. Cleanup and invalidation preserve newer latest selections
and unrelated entries under the existing rules; historical QueryReader still
does not delete latest without the ordinary writer lock.

## Migration and retained-snapshot cost

This is v3-only. A v2 pin/cursor is unsupported; no fallback is attempted after
a v3 failure. Same-v3 producer A to reader B historical pins remain supported,
without reuse of A's current-check/image/gate proofs by B.

Retire only exact canonical v2 namespace/name families under the existing writer
lock: semantic-page, semantic-root, semantic-latest, and the old locator name.
Parse namespace plus exact fixed suffix fields; never erase all semantic-* or
semantic-latest-* paths. New v3 names are distinct. Budget::decode and owned_entry
must continue recognizing canonical v2 locator names for signed accounting and
retirement only. Replacing that validator with a v3-only validator would make a
valid old Budget undecodable and cause unrelated ordinary entries to be treated
as orphans. A mixed valid-budget control must preserve an ordinary check record
and v3 entries while retiring the old locator and exact old semantic families.
This accounting recognition is not a v2 query reader. A damaged/unauthenticated
Budget retains the existing disposable-orphan policy. Ordinary input/cache hints
may fail and rebuild normally; they are not a basis for adopting v2 snapshots.

The first writable open of a mixed store still decodes/scans/removes the old
population. It may cost more than a header-only scan. No trusted migration marker
skips later discovery if legacy names reappear. Steady-state scan work remains
proportional to retained containers plus ordinary entries and disposable debris.

Old-format writers sharing the directory are unsupported: their budget/name
rules may reject a v3-containing budget or discard containers. Record this
availability limit explicitly; do not build a compatibility shim.

Each new snapshot owns a full stream. Cross-snapshot physical page deduplication
is lost. Unchanged-build reuse should keep the same snapshot; changed indexed
builds may consume more persistent bytes and evict earlier. Measure two distinct
snapshots, not only one empty-store publication. This representation is not fact
compression and cannot be credited with removing the global fact/IR cost.

## Counter semantics

Preserve exclusive io_begin/io_end timing; never time the builder's complete
lifetime as Store I/O, because caller-side fact construction occurs between
appends.

- Existing seal_ns/seals/sealed_payload_bytes count each real bounded node or
  front seal, with actual payload bytes.
- New stream_write_ns and stream_completed_write_bytes count write duration
  and fully completed requested write buffers, including reservation/backpatch
  as explicitly labeled operations; they are not all attempted syscall traffic.
- Publication time covers final publish/rename work, excluding separately timed
  seals, stream writes and nested preparation/metadata.
- Existing preparation/metadata spans cover capacity and locked reconciliation.
  The front/EOF check stays in metadata timing.
- budget_header_bytes remains exactly 90 per completed fixed header.
  Add budget_manifest_bytes for completed 400-byte manifest reads,
  budget_container_authenticated for valid authenticated fronts, and
  budget_container_extent_checks for completed EOF comparisons.
- Normal range reads report only successfully verified manifest/node payload and
  envelope bytes. Never count the EOF value as bytes read/authenticated.
- Container successful published_envelope_bytes is its full stream length;
  published_payload_bytes is the sum of its individually sealed payloads.
  published counts one container publication. Explain this aggregation.
- Optional counters retain saturation/failure behavior and cannot create a trap
  in an otherwise valid build. The sum of exclusive times remains auditable.

## Focused validation contract

All failures, setup refusals and resource refusals are retained. No automatic
retry converts a failed case into a success. Correctly authenticated malformed
fixtures are distinct from raw corruption: fixture-only encoders may use the
private test key so bounds/order checks are actually reached.

| Area | Required observations |
| --- | --- |
| Serialization | Independent test-only eager v3 versus streamed v3 logical values, nodes/root and queries on empty, 16-line, 16 KiB, maximum-fact and multibyte boundaries |
| v2/v3 parity | Independently authenticate and enumerate both formats; compare complete logical key/value maps and fact/continuation bytes; physical root/snapshot/cursor identities legitimately differ |
| Pins | A/B producing executable identities, same-source distinct IDs, historical v3 read, no current-executable hash, no admission/image reuse across producers, wrong selector/pin/cursor refusals and direct query_node wrong-snapshot/retained-handle controls |
| Bounds | Signed overflow, high-bit i64 values, front offsets, wrong range length, forward/self/cyclic refs, root not last, prefix/digit/depth/order faults, short/oversized/trailing extents; invalid unselected child descriptor; two lookups on one Reader with a cached digest followed by an invalid descriptor, alongside valid repeated-descriptor reuse |
| Read cap | Valid multi-node total above per-reader cap succeeds; default-cap query of measured >256 MiB container succeeds with bounded allocation/read evidence |
| Mutation | Held-fd pathname swap; hardlink alteration/truncation before later selected node; valid old-owned bytes or refusal only; no partial stdout |
| Missing/hostile | Absent, readonly, symlink, dangling link and FIFO container/locator paths; no-create behavior and retained authentication failures |
| Capacity | Exact full extent, signed budget length disagreement, old/new index headroom, staged growth refusal, oldest-generation eviction, busy writer refusal |
| Recovery | Deterministic interruption at create, front reserve, child/root append, backpatch, rename, locator, latest, final Memo and checkpoint; valid completed pin or disposable miss, never partial answer |
| Migration | Exact legacy cleanup, mixed old/new names, replayed budget/orphan container, legacy reappearance without clean marker, newer latest/unrelated proof preservation |
| I/O evidence | One bounded front/EOF probe per retained container in reconciliation; only requested bounded node ranges read during query |

Migrate existing tests rather than delete their protection:
query_store_cases.lkj, semantic-query-test.sh, query-store-test.sh,
cache-read-test.sh, semantic-index-page-test.sh and query-publication-test.sh.
The final-Memo eviction witness must still reach that late boundary, not pass
because an earlier new-format operation refused. The old eager/lazy oracle must
be explicitly adapted to v3, while retained historical benchmark files stay
unchanged.

No broad unbounded corruption campaign is added. The independent reviewer
prepares exact fixture/observation design before source implementation.

## Fixed 10k/100k measurement protocol

Only after functional correctness and a frozen candidate, one worker uses:
4,294,967,296-byte address-space limit, 900-second command timeout with 10-second
kill grace, and 2,147,483,648-byte writer budget per store. No 1M/10M run.

The producing baseline is an immutable copy of the successful coherent landing;
record its actual SHA256, source revision and full source manifest before any
measurement. The candidate is likewise copied and pinned after implementation.
No producer, threshold or source change is allowed within this experiment.
This identity addendum is required before launch, not a result-dependent choice.

Use bench/scale.lkj at this declared base, shape revision
0b30ca8987bd4ba343ab45ab01751b6721c7151a, seed 20261116, and rungs 10,000 then
100,000. Retain the shape output (expected prior shape digest
0457f8f9424882d2e3160a568f280193d49b9b046a5fae2d6e2d0b6da047072c),
generated input manifests and both producers. A mismatch is a recorded setup
refusal, not permission to change the generator silently. Use the same frozen
library and explicit runtime source for both compilers; --inline-limit 0.

Keep all four rung/producer stores until oracle/report acceptance. Before
expanding trees or creating stores require available bytes >=
4*2,147,483,648 + 536,870,912 and record the actual free-space observation.
Refusal is an outcome. Keys are private and outside every source/library/runtime
directory grant. Every compiler/control runs with a relative root path and
explicit narrow --source-dir (or an equivalent narrow owned CWD); never grant /
by using an absolute root without --source-dir.

For each rung the fixed ordering is:

1. baseline ordinary cold build of S0 in an empty store;
2. candidate ordinary cold build of the identical S0 in its empty store;
3. baseline indexed build S0, then candidate indexed build S0;
4. baseline unchanged indexed build S0, then candidate unchanged indexed build S0;
5. materialize S1 by the exact guarded benchmark-input body change
   edit::target `value ^ 1u64` -> `value ^ 2u64`, retaining old/new input hashes;
6. baseline ordinary body build S1 without --index, then candidate equivalent;
7. baseline indexed build S1, then candidate indexed build S1.

The input transition is owned-fixture generation outside measured commands,
not a tracked production-source edit. Each step executes exactly once. A
failure is recorded and stops dependent phases; no enlarged limit or silent
rerun. Cold means empty disposable store, not flushed host filesystem caches.
Record GNU wall/user/system/peak-RSS/exit, wrapper wall separately, all load
averages before/after, complete stdout/stderr and LKJSCRIPT_STATS/STORE_IO.

After each command record static namespace census: file counts and physical
bytes, signed budget/index size, semantic container/page/locator/latest sizes,
snapshot IDs and source/IR IDs. Census is outside timed commands. Record which
S0/S1 pins survive after final writes. Compare single-snapshot and two-snapshot
semantic-family retention explicitly, including duplicated payload bytes and
eviction; do not mix this with ordinary pack changes.

Only after timed phases, run the full rooted logical-value oracle for every
retained corresponding S0/S1 pair. Authenticate every extent under its own
producer; compare sorted logical keys and exact value bytes. Verify source and
checked-IR identities and native image equality. Retain all mismatches.

Then make one bounded query of each following question for each available
S0/S1 pin and producer, in the listed order:

| Kind | Identity |
| --- | --- |
| outline | edit |
| signature | edit::target |
| text | edit::target |
| slice | edit::target |
| uses | edit::target |
| callers | edit::target |
| callees | main |
| impls | std::sequence::VecMutate |
| instances | edit::target |
| type | edit::target |
| layout | u64 |
| authority | edit::target |
| dependents | edit::target |

Use explicit pins, depth 1, limit 96, output bytes 32,768 and default query
read-cap settings. Preserve empty/continued answers, never replace or retry a
question to improve the table. Authenticate and compare complete fact bytes;
only independently verified protocol version, producing digest, snapshot/root
identity and cursor envelope differences may be normalized. Any continuation
must have its authenticated decoded traversal state compared; do not blindly
strip cursor strings. Full logical-value parity supplies coverage beyond a
bounded answer. Focused tests separately exercise cursor resumes/cross-pin
refusals. These queries are correctness observations after the measured build
sequence, not a fresh forty-question latency claim.

Report both baseline/candidate absolute results and the measured metadata
change. Do not infer a compiler-speed milestone, constant-time edit, 1M success
or overall memory improvement from fewer files. Snapshot duplication and first
legacy retirement remain explicit costs. Existing benchmark cohorts and failures
are never overwritten or substituted.

Retain fixture/store/key until report and proof-manifest audit acceptance, then
remove only owned trees, keys, stores, images and compiler copies. Keep intentional
text evidence, source operation requests/manifests, protocols and reports in Git.
