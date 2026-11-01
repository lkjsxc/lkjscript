# Recoverable native binary-filter phases

This contract defines `LKJPH001` for standalone Linux x86_64 ELF produced by the
Rust and maintained lkjscript backends. It applies only to a pure entry with
signature `(Bytes) -> {diagnostic: I64, image: Bytes}`. A task callable with an
empty effect row does not qualify. The [native ELF contract](native-elf.md) owns
the surrounding image; [self-hosting](selfhosting.md) keeps raw-source compiler
reproduction, full `P_self` support and same-process reclamation distinct.

## Discovery and calls

An eligible image contains this 64-byte descriptor at file offset 256. Integer
fields are little-endian u64. Entry addresses are virtual addresses in the
mapped image, not file offsets. The current contract has no predecessor reader.

| Offset | Field | Current value |
| --- | --- | --- |
| 0 | Magic | Eight bytes `LKJPH001` |
| 8 | Descriptor bytes | 64 |
| 16 | Caller handle bytes | 160 |
| 24 | Result bytes | 24 |
| 32 | Invoke address | Image-specific native entry |
| 40 | Release address | Image-specific native entry |
| 48 | Flags | 1: recoverable pure binary filter |
| 56 | Reserved | 0 |

Invoke uses the System V AMD64 argument registers: RDI is the input byte pointer,
RSI its nonnegative signed-I64 length, RDX the handle pointer, and RCX an optional
cancellation-word pointer. RAX returns a status from the table below. The result
is published through the handle, not returned as a language value in RAX.
Release receives the handle pointer in RDI and returns its status in RAX.
Both entries preserve the caller's callee-saved registers. Internal generated
language calls retain their backend-specific convention.

| Status | Meaning |
| --- | --- |
| 0 | Invocation completed, or release completed |
| 1 | Invocation cancelled at a safe point |
| 2 | Allocation or checked allocation-extent failure |
| 3 | Runtime rejection or invalid native result |
| 4 | Cleanup failed; owned mappings may remain |
| 5 | Invalid invocation or release arguments |

A completed compiler diagnostic is a field of a status-0 result. Diagnostic 12,
for example, is not native runtime status 12. Trap and allocation detail values
are backend-specific observations; callers must not infer a shared diagnostic
namespace from them. Invalid-argument refusal does not establish that arbitrary
pointers were safe to dereference or that an already live handle was released.

## Caller ownership and handle layout

The caller supplies zero-initialized, writable, eight-byte-aligned handle storage.
Its address must remain stable until all its mappings have been released. The
caller must own the handle exclusively during invocation and release, and keep
it disjoint from immutable input and cancellation storage. Do not copy, move,
concurrently reuse or zero a live handle. Reuse is permitted after successful
release or completed failure cleanup.

Input must remain immutable and readable for the entire invocation. Nonempty
input requires a nonnull pointer and checked nonwrapping pointer-plus-length.
The optional cancellation word is a separately owned, eight-byte-aligned u64,
kept readable throughout the invocation. Zero permits continuation; any nonzero
value requests cancellation. A concurrent writer must provide an atomic aligned
store under the caller's memory model. Numeric extent and alignment checks do
not make this ABI an arbitrary-pointer sandbox.

| Handle offset | Field |
| --- | --- |
| 0–63 | Scratch allocation context |
| 64–127 | Output allocation context |
| 128 | Terminal native status |
| 136 | Backend-specific terminal detail |
| 144 | Allocation-attempt observation |
| 152 | Allocation-attempt saturation flag |

Each 64-byte context contains the current slab payload-address link at offset 0,
allocation cursor at 8, allocation end at 16, observed live slab count at 24,
observed live mapped bytes at 32, observation saturation flags at 40, lifecycle
state at 48, and published result pointer at 56. A zero link denotes an empty
chain; a nonzero link equals its slab's mapping base plus the 64-byte header.
States are 0 for open, 1 for closing and 2 for closed. Saturation bit 0 covers
slab count and bit 1 covers mapped bytes. These fields are observable runtime
state; the caller must not edit live contexts or infer permission from counters.
Allocation attempts count allocator requests, not successful mappings, bytes
copied or allocator traffic.

A successful invoke leaves scratch closed and publishes its output root at
handle offset 120. The root is an output-owned 24-byte record: diagnostic I64
at offset 0, image length at 8, and image pointer at 16. The complete image is
copied into the output context; an empty image may have a null pointer. Neither
the record nor its payload may retain an interior pointer into scratch or input.
The result remains valid until its handle is released. Multiple live handles
may retain independent results. A retained image may serve as a later
invocation's immutable input while its owning handle remains live.

## Allocation, promotion and teardown

Each context owns a separate linked chain of mapped slabs. Every slab has a
64-byte header containing the predecessor payload-address link, exact mapping
extent, owner context and payload start. The context head and predecessor links
use the payload address, `mapping_base + 64`, rather than the mapping base.
Header access and unmapping recover the base as `link - 64`. Thus a successful
mapping at address zero has the nonzero link 64 and remains owned; zero alone
is the empty-chain sentinel.

Payload allocations retain their required alignment. The minimum slab size
and page rounding are physical allocation choices; a larger request creates
a larger slab. Neither is a logical workload quota. Checked arithmetic precedes
extent, alignment, pointer and mapping operations.

A successful `mmap` reserves virtual address space; Linux overcommit does not
promise physical backing for every future write. Status 2 covers checked extent
failures and allocation failures reported by the syscall. Later physical-memory
exhaustion or OS process termination cannot be converted into a recoverable
status by this ABI.

Invocation allocates compiler temporaries only in scratch. Promotion switches
to the independent output context, allocates the result record and complete
image, and copies while scratch is still live. Only after successful scratch
disposal is the output root published. This boundary has an explicit finite
result shape; it does not provide a general deep copier for arbitrary values.

Disposal closes allocation, clears unpublished cursor/root state and walks
only the context's owned chain. For each nonzero link it recovers the mapping
base, reads the predecessor link and exact mapping extent, then calls `munmap`
on that base and extent. The chain advances only after successful unmapping.
An unmap failure preserves the remaining chain in the stable handle for retry
and reports status 4. Such a handle must not be abandoned or relabelled as
reclaimed. Empty teardown resets live observations exactly and closes the
context. Release disposes output and scratch, and is idempotent for an empty
handle. Partial promotion, cancellation and runtime rejection attempt both
teardowns and publish no result; cleanup failure takes precedence over claiming
successful recovery.

Observation overflow saturates with an explicit flag and never denies an
allocation. Exact extents and links continue to drive mapping and teardown.
A saturated count cannot establish an index, buffer size or exact memory bound.

## Cancellation and recovery scope

Cancellation is cooperative and separate from proof allowance or resource
admission. Native call/control-flow paths, allocation, extent-scaled helper
loops and bounded bulk-copy chunks provide safe points. Fixed-size arithmetic
and hash compression work need not poll every instruction. A last successful
poll may race with cancellation, so completion can win that race. The contract
does not promise a wall-clock response bound.

Teardown does not poll cancellation again: once recovery starts it attempts to
finish releasing owned scratch/output. An active recovery frame restores the
invocation stack and outer allocator state after rejection. Nested invocations
require separate handles and preserve the outer allocation context. This
recovery applies to pure compiler work; it does not roll back foreign effects,
external resources or arbitrary host actions.

The leaf poll routine preserves its incoming flags and all registers except
reserved volatile R11. A surrounding backend call wrapper may itself clobber
flags; emitters must place the call before the comparison whose flags a later
branch consumes, or explicitly preserve those flags. Generated native CALL
frames still consume the OS stack. This ABI neither removes native stack limits
nor establishes language-level lexical `Region`, relative-pointer, region-slice
or region-capability semantics.

## Required verification

Production evidence invokes emitted, unmodified compiler and allocator bodies
repeatedly inside one long-lived native process. A test caller may adjust ELF
entry/segment framing without replacing those bodies. Controls cover retained
outputs, repeated ELF/artifact compilation, compiler diagnostics, cancellation,
actual mapping refusal, partial promotion refusal, cleanup failure/retry and
successful invocation after recoverable failure. Surviving output bytes must
remain correct after scratch unmapping and subsequent invocations.

Exact ownership observations must be corroborated by independent process
mapping observations. A negative control that clears counters while skipping
unmapping must fail. RSS is corroboration, not the sole reclamation proof.
Strict artifact loading, independent behavior and source fixed-point checks
remain required. Focused results and full staged-tree acceptance are recorded
separately in [status](../status.md); implementation alone is not Milestone C
acceptance.
