# Native raw-I64 graph slots

## Decision and acceptance state

The Rust x86_64 emitter implements signature-defined raw I64 direct graph calls
and derived scalar slot lowering. The implementation is based on `7f7cdc77`,
which includes the [allocation baseline](20261109-native-scalar-allocation.md).
The tested tree is `9d71c3e6`; implementation commit:
`acf74027`. Acceptance: accepted and integrated into main.
Delivery: Pushed to main; advertised and fetched `origin/main` independently matched `acf74027` at 2026-11-09T11:07:09.686Z.
The [acceptance record](#acceptance-evidence-and-delivery) binds the frozen source,
completed controls, both allocation comparisons and the final verification and
delivery evidence. Earlier source-directory and measurement receipts retain their
original trees and scopes.

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

The implementation requires public authoring and independent VM/reference expectations
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

## Acceptance evidence and delivery

Acceptance state: accepted and integrated into main.
Implementation commit: `acf740271957297971e379efb90c74ef392316a7`.
Delivery: Pushed to main; advertised and fetched `origin/main` independently matched `acf74027` at 2026-11-09T11:07:09.686Z.
Publication state: no release archive or production deployment published.

The completed focused and raw-source evidence binds implementation tree
`9d71c3e61cd7d5ce8c3116b126fd965bd86d4550`, based on
`7f7cdc776cea74b9f6bedf42ffde10bb261f2dd1`.
The complete frozen source inventory is
`.artifacts/capacity-unbounding/native-i64-20261109/freeze-1/source-after.json`,
SHA-256
`2d677b2f90fe2e74d7cb2399697e925bfc5acd19588f660fdbad12ff8b25835e`.
The aggregate acceptance and actual delivery records below must retain this
implementation binding. A subsequent Markdown-only checkpoint has separate
source, bootstrap, bridge and changed-check evidence.

### Capability and proof scope

The Rust x86_64 emitter keeps exact I64 Code arguments and results raw across
ordinary and tail direct calls. It derives internal scalar representations from
the expanded optimizer stream; a complete declined plan retains boxed internal
slots while using the same direct ABI. Checked adapters preserve external,
callable, List, Record and Variant boundaries. Raw/empty local facts exclude
non-pointers from tail-owner scans without weakening checks on uncertain boxed
slots. Bytecode Jump(0) bypasses one-time parameter adaptation.

The focused run passed 23 controls and the separate ignored allocation probe.
Publicly authored fixtures supply independent VM/reference expectations for
extreme I64 values, scalar/boxed boundaries, ordinary/tail/callable transitions,
nominal payloads and complete boxed fallback. Machine controls additionally
exercise frame reuse, callee-saved state, cleanup, traps, cancellation and retry.
The Jump(0) supplement preserves authentic admitted Record/Field metadata while
rewriting derived normalized code. Its original public VM/reference and rewritten
VM/native comparisons are evidence for entry/backedge wiring; they are separate
from the complete raw-source compilation proof.

The complete raw-source control genuinely generated C1, C2 and C3, checked strict
canonical artifact loading and independent behavior/mutation/negative controls,
and exercised both C1-Rust and C2-source in long-lived native phase processes.
Each phase process retained 17 observed frames, output survival, independent
mapping checks, callee-saved preservation and final owned mappings zero.
C3 byte equality does not establish a separate C3 phase process.

### Raw-source identities

The unchanged complete maintained carrier has 26,640,313 bytes and SHA-256
`acea4485c9b3e83198903ad69ad9ebe294499f9808bfc2064696cecec06b6cf2`.
It includes 81,458 live owners, 97 supplied types and 61,470 retirement records.
This implementation changes neither accepted graph meaning nor maintained source
proposals, source/compiler carrier profiles, canonical artifact schema or the
LKJPH001 layout.

The C1 Rust-emitted compiler has SHA-256
`05370f26bfa34d341a678db2cff5ba0174314ea7d5a83ae683b3b348a58f4cbe`.

| Output | Bytes | SHA-256 |
| --- | ---: | --- |
| C2 = C3 | 22,005,155 | `aa8c368b224fb4b31bb6b34e001c11d5f821fbd7f85bb6f961869139ab9db474` |
| Ahost = A1 = A2 = A3 | 22,909,104 | `3b5fa68fb045b971062e106093a120835fa69b6528c406b92fd1c75fe032b996` |

### Scalar fixture allocation comparison

The unchanged publicly authored fixture uses Tier 1 normalized input. The two
explicit counts are 4,096 and 8,192, with seed 4,096 and step 1,024. Each measured
run covers three modes, two counts and both emitters: 12 native processes,
36 independently reset invocations and 132 observed frames. The caller-selected
deadline is 120 seconds per process and the address-space allowance is `None`.

| Rust path | Before, N=4,096 | After, N=4,096 | Before, N=8,192 | After, N=8,192 |
| --- | ---: | ---: | ---: | ---: |
| Scalar loop | 12,297 | 6 | 24,585 | 6 |
| Ordinary identity-call loop | 12,297 | 6 | 24,585 | 6 |
| Bytes-slice loop | 16,393 | 4,102 | 32,777 | 8,198 |

The Rust reduction is `3*N+3` requests per invocation in all three paths. The
maintained backend is unchanged: 6 requests for scalar/identity-call paths and
`N+6` for Bytes slicing. All three invocations in each process agree; independent
VM/reference outputs match, all observed saturation flags are zero and final
owned mappings are zero. Complete maintained source is bound as context for this
probe; the probe does not itself establish raw-source self-compilation.

### Complete compiler allocation comparison

The second comparison consumes the completed prior and current raw-source
receipts on identical complete carrier, ELF request and artifact request bytes.
The matched explicit caller policy is a 3,600-second deadline and a 32 GiB
(34,359,738,368-byte) address-space allowance. It compares successful artifact,
ELF and artifact-retry invocations in one long-lived process per compiler.

| Compiler / invocation | Before requests | After requests |
| --- | ---: | ---: |
| C1-Rust artifact, frame 0 | 872,780,680 | 97,298,484 |
| C1-Rust ELF, frame 1 | 1,123,766,130 | 66,443,017 |
| C1-Rust artifact retry, frame 8 | 872,780,680 | 97,298,484 |
| C2-source artifact, frame 0 | 63,427,596 | 63,427,596 |
| C2-source ELF, frame 1 | 47,655,928 | 47,655,928 |
| C2-source artifact retry, frame 8 | 63,427,596 | 63,427,596 |

Both comparisons count passive LKJPH001 allocator requests per independently
reset invocation, including scratch and output promotion. The pinned compiler
observer requires a zero saturation flag before emitting each trace record; its
text trace does not serialize the raw flag. Cancellation and forced-refusal
frames are incomplete invocations and are excluded from this comparison.
Requests are distinct from allocated bytes, mmap calls, RSS and peak scratch.
These observations establish neither allocation-category shares nor subpass
attribution or wall-clock speedup. Passive counters do not impose an execution
quota.

### Verification iteration measurements

Both checks ran all 29 gates fresh, with zero reused gate receipts. All nine
complete workspace input records matched: the product CLI and eight harnesses.
The recorded Cargo preparation context also matched; the two check profiles
retain their distinct overall profile and gate fingerprints.

| Observed elapsed time | Changed | Full |
| --- | ---: | ---: |
| Release-build gate | 609.866 s | 2.597 s |
| Workspace-test gate | 4,607.407 s | 4,607.498 s |
| Outer root harness | 4,605.23 s | 4,604.95 s |

Inside those fresh gate executions, Cargo reused 1 of 16 recorded workspace
units in changed and all 16 in full. Gate durations can overlap; they are not
added into a serial duration or treated as exclusive cost shares. These
observations do not isolate per-test phases or establish a causal speedup.

The next measurement targets repeated per-entry native-image generation and
normalized-payload export in the canonical-unit control. Source inspection
establishes that repeated preparation; its cost remains unmeasured. Sharing
preparation must preserve each unit's VM, reference, both native executions,
canonical-byte comparison and strict decoding.

### Retained evidence

All paths below are relative to
`.artifacts/capacity-unbounding/native-i64-20261109/`.
The audited evidence set binds complete inputs, source and images to actual
joined terminals. Comparison receipts preserve the original prior execution
trees.

| Evidence | SHA-256 |
| --- | --- |
| `focused-1/receipt.json` | `d8ee1584e968d89ca1e233c71b17bd4bc4085735d3938022768c5031f5d571b2` |
| `raw-1/receipt.json` | `f1835c8250300642486a7bb63940bfc3437d04b721439a41919e1193ca8d29b6` |
| `raw-audit-1/receipt.json` | `0e7464e54ceed988932ecf16fd221533f22e87b23fa4600475f62b311d162f58` |
| `allocation-comparison-1.json` | `2277ba207d9f1066ea87916f65787bd9529dd519372e8bb2a9153ed6bf1d375b` |
| `compiler-allocation-comparison-1.json` | `d1d1f0c8ae8d2362808587eed8805f61ac1b4cbb02795836583f737b69dc056c` |
| `gate-timing-comparison-1.json` | `7b99d112fd037a0b75bbabc88aa5fb252ae7552e468f2f6b86057dcde163a304` |
| `acceptance-1/receipt.json` | `e1c62429a1286214171ff5945e22edd3c8bc1be162ee5899c628e825c241e43b` |
| `implementation-delivery-1.json` | `73e33026ae0af0e642eeabfed787465125c5a8cb50d6471a1b49c090e2f5f585` |

Tier 2 Milestone B remains open for affine OwnedRecord/OwnedChoice deconstruction,
general `List<T>`, owned witnesses and source-tied read views, lexical Regions,
generic/recursive nominal values, general specialization, task/effectful callables
and resource/owned captures. This implementation does not close general boxed
value construction/copy/comparison/codec/destruction/failure-recovery lifecycles,
default call/value-stack bounds, deep non-tail native CALL growth or
arbitrary-value phase promotion.

### Completed verification

| Validation | Recorded outcome |
| --- | --- |
| Focused controls | 23 passed; separate ignored allocation probe passed |
| Combined raw-source control and independent audit | Passed; C2=C3, Ahost=A1=A2=A3 and independent behavior/mutation/negative controls |
| C1/C2 same-process native phase controls | Passed; 17 observed frames per process, output survival, final owned mappings zero |
| Staged bootstrap | Passed; `bootstrap-1/receipt.json` |
| Typed verifier bridge | Passed; `bridge-1/receipt.json` |
| Changed check | 29 passed fresh; 0 reused/unrun; root 3,093 passed, 0 failed, 22 ignored |
| Fresh full check | 29 passed fresh; 0 reused/unrun; root 3,093 passed, 0 failed, 22 ignored |
| Release-target verification | Passed; target policy only |
| Aggregate source/index/image/actual-terminal audit | Accepted; complete source/index seal and actual joined terminals bound |
| Implementation commit and actual remote delivery | `acf74027` integrated into main; remote ancestry independently verified |
