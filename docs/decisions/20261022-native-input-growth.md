# Growable native compiler input

Date: 2026-10-22 (UTC). Baseline: a8c4d1c4534c92a6c97a1d8cdbea0fb91ed8d59c.

## Decision and dependency

Remove the fixed 16 MiB stdin rejection from both Linux x86_64 binary-filter
emitters. The Rust and lkjscript emitters now own a separate, growable anonymous
input mapping. This is a runtime capability prerequisite for larger compiler
carriers. It does not change LKJCI1 profile 4, LKJSG001, their 8 MiB validation
bounds, or the compiler's semantic permission.

The former transport ceiling was a default policy quota. The initial 64 KiB
mapping is a tuning choice. Nonnegative I64 lengths, checked size arithmetic,
Linux mapping failure and caller-selected address-space limits are distinct
representation and platform boundaries. No numeric maximum substitutes for an
unmetered policy, and no telemetry counter establishes storage validity.

Both emitters retain their entry ABI, successful raw output, usage diagnostic,
compiler refusal and trap ordering. No compatibility reader or temporary runtime
bridge is introduced.

## Pointer lifetime and progress

Before EOF, only the reader owns the input base address. It retains a relative
length and actual mapped capacity; no language descriptor, slice or closure can
refer to that mapping. A successful moving remap replaces the base before any
append. A failed mapping operation reaches native_allocation before pointer use.

At a full mapping, a one-byte stack probe distinguishes EOF from further input.
The pending octet survives the growth syscall in R9 and is appended exactly once
to the new mapping. The scratch stack slot is restored before EOF, error or
growth handling. EINTR retries after reserving the slot, without another stack
decrement. MOVZX and LEA preserve the read-result comparison flags.

Growth doubles a positive capacity with signed overflow detection. The supported
capacity values are powers of two from 2^16 through 2^62; Linux may refuse much
earlier. Every ordinary read requests a positive remaining extent and advances
length only by a positive returned count. A finite stream progresses to EOF;
read errors reject, and an unending stream remains governed by its host process.
No semantic decoder fuel or cancellation mechanism is removed.

At EOF, the ordinary immutable Bytes descriptor is allocated in the language heap.
The separate input mapping stays stable through the entry call and stdout write,
including a result that aliases input. Remap replaces the old input mapping rather
than retaining previous capacities. The mapping and general heap live until process
exit. This is not transitive phase promotion, repeated-compilation reclamation or
a claim of bounded native heap RSS.

## Bootstrap sequence and authoritative adoption

1. The existing Rust compiler and native emitter build the first replacement
   executable from the admitted compiler project. The independent canonical
   interpreter and normalized VM supply comparison paths.
2. The replacement consumes unchanged LKJCI1 profile 4, with mode 0 for native
   lowering and mode 1 for artifact emission. Source input remains LKJSG001.
3. The Rust binary-filter capability passes standalone growth, read-error and
   allocation-refusal controls before adoption into the authoritative .lkjc graph.
   No new language intrinsic is needed.
4. An ordinary public change draft, plan and apply replace only stream::read,
   stream::invoke and stream::emit bodies. The existing module, four declarations
   including stream::exit, parameter identities, signatures and order are retained.
   The transaction creates 545 body owners, retires 321 and resolves 26 existing
   module/callable references against the exact base. Complete maintained-proposal
   agreement remains a mandatory selfhost test. Carrier and normalized inputs are
   regenerated from the resulting accepted graph.
5. The updated C1 compiles the maintained native closure to C2; C2 reproduces C3.
   Both emitted stages reproduce the artifact bytes and execute independent programs
   and compiler-source mutation controls. Strict Rust loaders independently admit
   the emitted canonical units and artifact. Separate reader tests exercise input
   beyond the former transport limit, which the still-bounded compiler formats
   cannot yet accept.
6. Temporary ELF-export and inventory instrumentation is removed before staged
   acceptance. There is no format bridge to retain or clean up.

The new revision is
rev_a52428252252cf7cb597e2316da6e1ee1cff948a446323484724b1cc5e146985;
semantic state is
semantic_state_dd7af6f9c105d7b87f3e47270713bfcda165e78280168e9579fa501289549a98.
The complete carrier SHA-256 is
4488fc83fc5ede9865069bab3c5063277cafd58ba458b1b844d24f85aad14955.
Semantic state alone does not bind unused supplied types; all recorded carrier
observations retain this complete digest.

## Independent controls

Both native emitters echo independent octet patterns at lengths 0, 1, 256,
65,535, 65,536, 65,537, 256 KiB, 16 MiB, 16 MiB plus 1, and 20 MiB plus 17.
The pattern distinguishes bytes on successive growth boundaries. Pipe writers
use short chunks. Directory-backed stdin produces native_input, exit 65 and
empty stdout. A child-only 8 MiB address-space limit admits 65,537 bytes and an
exact 4 MiB input, but rejects 8 MiB plus 1 with native_allocation, exit 70 and
empty stdout. Exact-fit EOF therefore does not require another mapping.

Supplemental GDB controls place the initial input mapping immediately below the
actual ELF mapping using MAP_FIXED_NOREPLACE. This makes in-place growth impossible.
For both emitters, the initial 0x3f0000 mapping moves successfully, reaches
131,072 bytes of capacity and preserves all 65,537 input bytes. The shared input
and output SHA-256 is
f34e38d1f6278a7d5a3bc3ece2b0419229a3b31cf0dc5c878544485954002936.
The debugger changes only syscall placement in this control; production has no
test hook and the normal gate has no GDB dependency.

## Separated maintained-closure measurements

The preceding column is the compiler graph retained at a8c4d1c; the current column
includes the ordinary stream transaction. These are maintained-closure observations,
not an old/new implementation benchmark on identical input.

| Measurement | Before | After |
| --- | ---: | ---: |
| Modules / callables including externals | 62 / 465 | 62 / 465 |
| Live canonical owners | 28,326 | 28,550 |
| Structural type objects | 27 | 27 |
| Current retirement records | 9,881 | 10,202 |
| Complete LKJSG001 bytes | 6,793,590 | 6,898,767 |
| Mode-1 LKJCI1 bytes / units | 7,690,384 / 467 | 7,746,716 / 467 |
| Selfhost canonical artifact bytes | 8,126,517 | 8,183,095 |
| Mode-0 LKJCI1 bytes / functions / nodes | 794,236 / 244 / 16,111 | 830,072 / 244 / 16,417 |
| Tier 1 C2=C3 ELF bytes | 2,519,420 | 2,584,632 |
| Public rebuilt current-validation semantic work | 9,097,803 | 9,173,945 |
| Physical pack files / bytes | 13 / 334,241,707 | 51 / 343,959,058 |
| Reachable historical canonical payload bytes | 416,184,233 at prior GC | Not remeasured |
| Native allocator-request bytes / retained heap / RSS | Not measured | Not measured |

Public status supplies the comparable aggregate semantic-work counter. It sums
local full validation and dependency/composed-source validation, which have distinct
admission counters. A separate direct full-validator observation is 2,572,299 units;
it must not be compared with the aggregate column as a speedup or subtracted from
the artifact admission ceiling to infer remaining capacity. The physical storage
total includes 37 new
authoring packs and one 245,107-byte pack from the ordinary reference artifact
build. These are reviewed additions owned by this slice; prior packs and immutable
history are unchanged. Physical bytes are not transitive reachable payload bytes.

The inventory measurement took 12,566 ms including source reconstruction, carrier
and normalized-input export, and a separate semantic inventory read. The focused
compiler fixed-point and mutation test passed in 1,489.59 seconds, with complete
command wall time 24 minutes 49.732 seconds. Acceptance logs record their own
wall times. VM/reference cumulative allocation charges are modeled observations,
not native allocation requests or RSS. No same-input speedup or multi-iteration
native reclamation is claimed.

## Tier 1 identities and acceptance

| Object | SHA-256 |
| --- | --- |
| Mode-0 input | c3c7e792dca48cc4cbad184bd6f8ae47684f8a98988f1b177beeec531ad4c863 |
| C1 native compiler | 95cd468ae9e5dfdbad823f7de8af39ee4a125cef7b7cc2eba00f58bbe7869c70 |
| C2=C3 native compiler | 621b3433a5c94dddbb2f140c8aea8df28e11bfcad67812b84a3f0b2f07409fa3 |
| Mode-1 input | ee48cd09ceac6649d25ac024a72cf49f2c46f4e2795fc78a4e1af27f07232b19 |
| VM/reference/C1/C2/C3 selfhost artifact | 09c681e06f4269d8f10adaf525e573d5bd4061e3eaa593874d9856209353505c |

These identities bind the Linux x86_64 Tier 1 normalized-input context and the
complete maintained carrier stated above. Rust still supplies normalized input.
They are not the source-driven E2/E3 C2=C3 or A1=A2=A3 proof. The separate ordinary
Rust CLI artifact build is 8,243,974 bytes, SHA-256
ebb3598b4de722b3823f99849cd91206c7267eb29b450063bc6d7fe866f170d9;
it is not the selfhost artifact identity in this table.

Publication requires exact staged-tree bootstrap, tools/selfhost/verify.sh, all
29 fresh full gates and release-target inspection. The focused script covers
the selfhost and native-ELF test families with input hash stability; the complete
checker additionally owns workspace, CLI, lint, product, package and service gates.
Changed selection widens to the same full matrix for these paths. Run the
complete checker matrix once. Preserve terminal receipts under
.artifacts/selfhost/native-input-capacity and the compact latest checker receipt.
No public product release or deployment is included.

## Next dependency

Mode-1 input has only 641,892 bytes of remaining 8 MiB headroom. Close that
producer/consumer path next: checked host narrowing and extents, consumer range
proofs before multiplication, finite scanning progress, caller-selected allowance
semantics and independent inputs beyond the old aggregate byte bound. Retain the
700-unit guard until the flat canonical unit-map page is replaced; it has its own
64 KiB representation restriction. Audit each downstream unit/object encoder before
claiming expanded capacity.

LKJSG001 has 1,489,841 bytes of remaining headroom and minimum allowance 922,755,968.
Its u64 allowance field is still restricted by the current reader to a positive
low word below 2^31. Local full validation and artifact semantic admission each
retain a 10-million work policy; composed source admission has a separate
16-million policy. The public aggregate does not establish headroom under any one
of them. These are separate follow-up closures, not reasons to combine every
representation into one rewrite. Native byte operations still have a 256 MiB
implementation bound and
generated images a 32 MiB bound.

The active source profile still needs whole-inventory E2 admission and custody-aware
E3 lowering, followed by source-driven fixed points and full Tier 2 expansion.
This slice removes one executable capacity prerequisite without claiming those
semantic or memory-reclamation milestones.
