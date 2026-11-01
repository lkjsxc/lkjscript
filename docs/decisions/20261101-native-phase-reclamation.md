# Native phase allocation and result ownership

Date: 2026-11-01 (Asia/Tokyo).

## Decision and acceptance boundary

Give a pure native binary-filter invocation two independent allocation contexts:
one for compiler scratch and one for the complete surviving result. Promote the
result into its output context before disposing scratch. A caller can retain
several output handles, invoke the compiler again in the same process, then
release each output independently.

This is the implementation decision for Milestone C. The
[native phase contract](../spec/native-phases.md) owns the current ABI; the
[self-hosting contract](../spec/selfhosting.md#separate-acceptance-milestones)
keeps Milestones A, B and C distinct. Focused checks, terminal acceptance and the
exact implementation tree must be identified separately in [status](../status.md).
Neither process exit nor a patched prototype establishes production reclamation.

The previous accepted mainline implementation,
`010ce014ae97e87df17d2f6acbea728f73bf14d1`, established the maintained compiler's
complete raw-source Milestone A. Its native allocator retained scratch until
process exit. A pinned compiler workload used approximately 16 GiB of resident
memory, so independent output ownership and scratch reclamation are an immediate
dependency for repeated native compilation. That observation is not an
allocation-traffic or peak-live-language-data measurement.

## Allocation and publication

The current `LKJPH001` interface uses caller-owned, stable handle storage. Each
handle contains a scratch context, an output context and terminal observations.
R14 identifies the active context; R15 identifies an optional invocation recovery
frame. Entry preserves the caller's callee-saved registers, including both
context registers. Ordinary command execution uses its own context with no
recoverable phase frame.

Each context owns an independent linked chain of mapped slabs. A chain link is
the payload address (`mapping_base + 64`), with zero denoting the empty chain.
Header reads and unmap bases use `link - 64`, so a valid mapping at base zero
can still be owned. A slab records its predecessor link, exact mapped extent,
owner and payload start in its 64-byte header.
The 64 KiB minimum mapping and page rounding are physical layout choices. Larger
allocations get larger slabs; neither sets a logical workload quota. Payload
alignment, signed byte extents, address addition and page rounding are checked
before allocation or indexing.

The selected boundary is exactly `(Bytes) -> {diagnostic: I64, image: Bytes}`.
Its complete surviving representation is small and explicit: an output-owned
24-byte record and an independently allocated copy of every image byte. Even
when a function returns a view of its input, the result does not retain an input
or scratch pointer. The diagnostic is an inline integer. This does not claim a
general deep copier for arbitrary language values.

Promotion allocates the result record first and payload second. Scratch remains
live during the copy. The output root is published only after scratch disposal
succeeds. A failure between these steps releases both contexts and publishes no
result. Allocating after scratch and then rewinding a shared cursor is not used.

Disposal marks a context closing, clears its cursor and unpublished root, reads
the next link and exact extent while the current slab remains mapped, and calls
`munmap` on that slab. The chain advances only after success. A failed unmap
leaves the remaining chain reachable from the caller-owned handle for retry;
cleanup failure cannot be reported as successful reclamation. Empty contexts
become closed, and release is idempotent on an empty handle.

## Recovery, cancellation and scope

Recoverable invocation is exported only for a pure callable closure. Task
callables with empty effect rows are still task callables and do not qualify.
Recovery discards memory owned by the phase; it does not promise rollback of
arbitrary external effects or foreign resources.

Cancellation is independent of resource admission. Generated call, branch and
tail paths, allocation and unbounded helper loops provide safe points. Bulk
copies check cancellation between bounded chunks. The disposal loop itself
completes cleanup without observing cancellation again. Runtime rejection,
allocation failure, cancellation and cleanup failure have distinct terminal
statuses, separate from a completed compiler's language diagnostic.

Nested callers retain their own allocator state while an inner invocation uses
its own handle and recovery frame. Surviving outputs remain valid until their
own handle is released. The caller must keep input, handle and cancellation
storage valid for the documented lifetime; the native ABI is not a hostile
pointer sandbox. Generated native CALL frames still use the OS stack. This
change does not remove recursive lifecycle or native stack boundaries.

Internal contexts are not language-level lexical Regions. No new intrinsic is
introduced into the maintained compiler's feature closure. Region capabilities,
relative pointers/slices, region aliases and cycles remain separate Tier 2
semantic work.

## Source-driven implementation

The Rust native backend and the maintained source backend implement the same
external handle and result contract while retaining their different private
value representations. Neither consumes host-precomputed custody or phase
certificates.

The source backend keeps phase machine code in its runtime area. The
`phase-asm` module uses an explicitly checked two-pass instruction table;
`phase-code` describes current routines and `phase` checks their physical
placement before emission. Current label and opcode checks reject an invalid
table rather than silently truncating an address. Function body layout remains
measured from actual emitted bytes.

All maintained source changes are authoring proposals until adopted through
public transactions, public readback and complete proposal replay. The active
profile inventory and raw-source fixed point must include the added modules.
An earlier graph's successful compilation does not cover a later proposal edit.

## Evidence and measurement rules

Maintained native tests must call emitted production entrypoints in one
long-lived process. An appended caller may change the ELF entry and enclosing
segment extents; it must not patch allocator, compiler or cleanup instructions.
The tests must exercise the complete maintained compiler, independent programs,
retained output, diagnostic rejection, cancellation, actual allocation refusal,
promotion refusal and recovery.

Allocator observations are primary ownership evidence and must be corroborated
by independent process mapping observations. Clearing counters without unmapping
must fail the control. Strict artifact loading and independent expected bytes
remain necessary after scratch disposal; a zero count alone proves neither
survivor validity nor semantic agreement.

Allocation attempts, live mapped slabs, live mapped bytes and their saturation
flags have separate meanings. Saturation never denies work and never supplies
an exact layout or performance bound. Actual file/mapping extents and links
remain exact even when a passive observation saturates. Exact empty teardown
resets live observations. Timing, mapped capacity and resident memory are
reported separately; modeled evaluator allocation, native requested allocation,
copied bytes and peak live language data are not interchangeable metrics.

The former modified-binary experiments are design evidence only. Focused
production checks do not replace the staged-tree bootstrap, complete selfhost
selection, named source/inventory/native controls, 29 fresh full gates and
release-target validation required for integration.


## Completed focused production evidence

The 2026-11-01 working candidate passed four focused selections under the shared
target lease: 19 native ELF tests, one complete source profile inventory, one
maintained-native-closure control and one whole-maintained raw-source fixed-point
control. Every selected test passed, with zero failed or ignored tests; the two
expensive explicit controls were invoked with `--ignored`. Whole command times
were respectively 581.884431769s, 39.198342628s, 55.722975938s and 1315.264746685s.
The [status record](../status.md#focused-milestone-c-evidence) gives each command
filter and terminal receipt hash.

The test binary has SHA-256
`a8cc9f82ee4f13ee0932cc1010c1b4dcd2e89f0e239fa9bd16af3e6bb4c1be5a`.
Its source base is `114f7764638a40e4239038b865eb2b265d785219`.
The receipts explicitly identify working-tree controls; neither the base commit
nor the test-binary hash is a substitute for a candidate Git tree seal.

The complete compiler phase sequences exercised 17 observation frames in one
C1-Rust native process and 17 in one C2-source native process. Both reported
unchanged production instructions, empty scratch ownership, passed independent
process-map checks and preserved caller callee-saved registers. Artifacts
retained across later invocations remained byte-identical to the host oracle.
Releasing the final output left zero owned mapped bytes and zero anonymous
writable bytes attributed by the observer. C3 was byte-identical to C2; this
evidence does not claim another independently executed C3 phase process.

Native controls also covered cancellation in an allocation-free tail loop,
actual mapping and promotion refusal, runtime rejection and retry, cleanup
refusal with retained ownership and successful release retry, malformed handles,
passive observation saturation, and task-empty exclusion from the pure ABI.
The independent mapping observer rejected the forgotten-unmap control even when
its visible counters had been cleared. These outcomes exercise ownership and
physical disposal, rather than treating counter reset as proof.

The raw-source run re-established `C2 = C3` and `Ahost = A1 = A2 = A3`
for the current complete graph, independently expected program behavior and
source mutations, layer-reaching negative controls, complete inventory checks
and strict loading of each artifact generation. Its final repository check was
`original_repository_unchanged=true`. The focused log's old
`same_process_reclamation=false` footer label was stale: the explicit production
phase records above identify the checks that ran. The label source has since
been corrected without rewriting historical logs; the full seal must cover that
correction and the subsequent Rust guard completion-status fix.

### Graph adoption and storage identity

The accepted phase source is
`rev_4738276e107d5ba43239a48a5c93ad852c76ab850e8bbf43346ef4ac520c7d59`,
with public HEAD SHA-256
`46e8489a1033b25a7f4373016619b58172bf7b4069084123c781267454ac6eec` and
semantic state
`semantic_state_ac4cf31448b764ce6b563ba84cc1d2f0a241d00c5eaed0057657ed4799a43a46`.
The inventory covers 121 modules, 62,978 owners, 88 supplied types and
1,117 callables. Public readback covers all owners in 16 pages.
The accepted proposal snapshot SHA-256 is
`61aa185c50f233fde194e9a339de44b7a9042d9fec34a07a5e74fda24c37e5bf`.

Final public GC changed 98 packs / 272,587,289 bytes to ten packs /
269,216,542 bytes, preserving HEAD and the complete 23,628,696-byte transport,
SHA-256 `212b52be4f04c0c849dc6d4d3a36a4eb25647eca789145630ac03bb07bca1208`.
The GC receipt SHA-256 is
`c5393dc67d72b0cae7e7c8fb32d2581b70a33ccbff4abb62a195bd60a6097b0d`.
Storage compaction and native reclamation are separate observations.

### Remaining acceptance boundary

Formatting, Clippy, current CLI construction and generated-discovery checks
passed during final preparation. Their receipt SHA-256 is
`bd7f57070e50d773da6f62b58468d936c306454402ee8faf33dd122dec8fc1a5`.
Fresh staged-tree bootstrap, complete selfhost verification, named controls,
all 29 fresh full gates, release-target validation and integration are still
pending for this candidate. The historical Milestone A seal covers its earlier
implementation tree only. Until the new terminal seal is bound to the exact
committed tree, this decision records completed focused Milestone C evidence,
not accepted mainline completion.
