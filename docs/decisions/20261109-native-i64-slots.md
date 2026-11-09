# Native raw-I64 graph slots

## Decision and acceptance state

Implement signature-defined raw I64 direct graph calls and derived scalar slot
lowering in the Rust x86_64 emitter. This candidate is on
`compiler/20261109-native-i64-slots`, based on `7f7cdc77`, which includes the
[allocation baseline](20261109-native-scalar-allocation.md). Fresh implementation
acceptance is pending; earlier source-directory and measurement receipts retain
their original trees and scopes.

The baseline isolated three per-iteration Rust allocations in the scalar and
ordinary identity-call paths: reading a word, adding the accumulator and advancing
the cursor. Keeping graph call boundaries boxed would leave scalar traffic in the
tail-recursive loop. The complete slice therefore includes a direct graph ABI,
callable adapters and typed ownership-scan exclusions.

## Representation contract

[Native scalar representation](../spec/native-elf.md#native-scalar-representation)
owns the implementation contract. Accepted NormalizedProgram meaning remains
authoritative. No graph adoption, maintained source proposal, LKJSG002/LKJCI001
carrier, canonical artifact or LKJPH001 layout is changed.

| Boundary | Contract |
| --- | --- |
| Exact I64 Code parameters/results | Raw direct ABI, independent of internal optimization success |
| Proven local/operand slots | Raw I64; boxed pointer and empty local are distinct states |
| Missing or conflicting representation proof | Complete function retains boxed internal slots |
| External function ABI | Existing boxed arguments/results |
| Callable prefix and ordinary Invoke | Boxed; graph entry/result adapters perform conversions |
| List, Record and Variant payloads | Boxed; payload zero never denotes absence |
| Direct/tail entry versus bytecode zero | Entry adaptation occurs once per activation, not on Jump(0) |
| Fast tail return | Caller and target direct result representation must agree |
| Tail owner scan | Proven raw/empty locals excluded; uncertain boxed slots still checked |

The analysis runs over expanded optimizer output and binds the complete prepared
function/application. Reused instruction storage cannot substitute for that
context. Block-entry states use a finite-height lattice and fallible flat vectors;
there is no arbitrary iteration, instruction or count quota. A declined plan is
an optimization result, while allocation failure remains a resource diagnostic.

Rust-private callable metadata is 40 bytes with a fifth word identifying raw-I64
results. Ordinary wrappers reserve a complete outgoing argument window, preserve
the caller's boxed arguments and callee-saved registers, call the direct graph
entry and box a raw result. Boxed-input tail adapters enter the same direct-tail
destination used by direct calls. No predecessor reader or migration is needed
for this private, self-contained machine image.

## Arithmetic, custody and cancellation

The raw intrinsic paths recognize exact pure signatures and parameter-use modes.
They share checked arithmetic with boxed external entries and preserve byte/buffer
range checks. Operand tuples remain on the stack through polls and helpers.
Fallback bodies adapt both parameters and results; an unlike tail-result ABI uses
the retained ordinary continuation.

Raw/empty facts never grant ownership or borrow authority. Existing source and
normalized admission still establish affine custody and provenance. Compound
storage and inactive-branch cleanup retain their existing representation. The
native phase still promotes its complete Bytes output before scratch reclamation.

Intrinsic-entry polling and the former out-of-pool integer-result polling points
remain observable to cooperative cancellation. Removed scalar allocations do not
increment the passive allocation-request counter. The -1..255 boxed pool is a
physical optimization, with all signed I64 values still supported.

## Verification obligations

The candidate requires public authoring and independent VM/reference expectations
for runtime I64 extremes, direct and callable transitions, nominal payloads,
whole-body boxed fallback, live-owner cleanup, overflow rejection and retry.
Focused machine controls cover frame reuse, callee-saved state, allocation-free
cancellation and bytecode backedges. Tests distinguish live-owner tail fallback
from a complete declined representation plan.

Repeat the existing explicit 4,096/8,192 allocation probe unchanged on the actual
Cargo-selected image, with separate per-invocation counters and clear saturation
flags. Requests are not bytes, RSS, mmap calls or whole-compiler allocation shares.

Renew the combined raw-source compiler test once: genuine C1/C2/C3 generation,
C2=C3, strict Ahost=A1=A2=A3, independent behavior/mutation/negative controls and
C1/C2 same-process output survival and reclamation. Then bind fresh changed/full
checks and release-target verification to the frozen implementation tree before
integration. The complete ordinary test matrix retains its explicit caller-policy
and physical-framing controls.

This slice does not establish deep non-tail native stack safety, general boxed
value lifecycle closure, arbitrary-value phase promotion, lexical Regions or full
Tier 2 Milestone B.
