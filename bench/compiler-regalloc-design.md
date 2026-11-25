# Native register and frame opportunities after Inline64

The rejected field-decomposition pass gained 2.722420% on the frozen compiler
computation. The CallSRA census found no eligible parameter or result families.
This design examines primitive local storage in the native backend for pass 2
of 3. Inline64 is pass 1. The reference for the decision is the historical
18.733333 s Inline64 control against 3.650 s Rust `-O` (5.13×). A fresh 10%
gain at those times would yield about 4.62×. [Rejected pass and fixed computation](compiler-scalar-replace.md)

## Static census and retained samples

The frozen computation is `26bf2c3c021dd79810a81fa1d9054d59f3768b06`. The
output oracle is the 16,228,028-byte image with SHA-256
`8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132`. The
selected checked IR and raw cohort-2 profile have SHA-256
`32c0f43dbde125e43f2ca5a0c6d8548ba0f1774ce3b51bc62d94a6488af0fdca` and
`7849d4dd69906491d07664f493eb0f4fd8340cfde3760f6e4f168dbbde95f302`,
respectively. The census reads those retained bytes. It checks the IR, uses the
historical `d461c2d9` layout and register planner, and checks the profile
function map, exact copy counts, exclusive kind totals and exclusive symbol
counts against the retained tables. [Historical Inline64 profile](compiler-profile-inline64.md)
[Exact cohort-2 map and sample accounting](compiler-profile-copy-sites-cohort-2.md)

Nominate only these opportunities on the existing allocation.

- **Register move.** Find whole-local primitive `Read` or `Assign` with
  distinct allocated registers in the same bank (volatile or saved), an exact
  source last read and destination first write in one straight-line block.
  The source also starts in that block. Its register must be free of other
  intervals for the destination's entire lifetime. Both locals have the same
  type. `copy_local` and `Assembler.mov` already omit a same-register move.
- **Frame spill.** Find a used, unaddressed, spilled primitive nonparameter
  whose entire interval fits one currently unused volatile register once
  three opcode-specific clobber masks replace the current all-volatiles
  barrier. Test each register against the baseline occupants and every
  barrier in the interval. These are individual opportunities. Simultaneous
  promotions may conflict, and a register operand may introduce a move.
  Count same-block, disjoint, equal-size/alignment spill homes for possible
  frame slot reuse separately. Reusing a slot alone deletes no load or store.
- **Fixed frame.** Find scalar leaves with no saved registers and every used
  non-unit local in a register. Check every opcode against the scalar
  selector and exclude any result staging, address materialization, call or
  cleanup. An unreferenced, nonparameter primitive declaration needs no slot.
  Under these conditions the unused scratch reservation is the only reason
  for a nonzero fixed frame. The capacity check and BP frame record remain.

The scan covers 10,072 checked IR functions. The profile maps 4,412 to machine
symbols, which carry 11,777 self samples. The remaining 2,881 samples belong
to unmapped symbols, including native runtime and platform helpers.

| Static opportunity | Whole selected IR | Emitted | Emitted functions | Functions with self samples |
| --- | ---: | ---: | ---: | ---: |
| Same-bank transfer edges with a free source register | 65 | 62 | 34 | 15 |
| Spilled locals with a free register through refined clobbers | 12,376 | 12,202 | 1,416 | 398 |
| Reused scalar spill homes, considered before any promotion | 18,524 | 18,406 | 902 | 221 |
| Proven scalar zero-frame leaves | 13 | 0 | 0 | 0 |

The emitted functions have 62,631 live unaddressed primitive spills. The
12,202 nominations test feasibility one local at a time. The 34 emitted
move-candidate functions have 311 self samples, of which 45 are known copies.
The frame-spill candidate functions have 4,265 non-copy self samples, so
their function totals alone cannot exclude a share of the 1,033 frame
samples. No emitted function satisfies the zero-frame rule. In particular,
the existing allocator can place scalar arguments in saved registers, and
stack-staged arguments or saved slots already require frame storage.

This is a constrained upper bound on exposed **samples**, based on exclusive
counts. The raw format keeps per-site samples for aggregate copy only. For
each symbol, subtract those exact copies from its self total. Intersect the
retained kind address intervals with the symbol's extent to determine which
kinds its remaining samples could occupy. Assign the residual samples to
compatible kinds while satisfying every global kind marginal. Maximize the
number assigned to the nominated symbol-kind pairs. This gives simultaneous
constraints across the three kinds. Adding separately maximized kind counts
may overstate their joint ceiling. Inclusive counts and static instruction
counts supply no weights. Even an eligible symbol-kind pair can contain only
samples of instructions outside the transformation, so zero is the supported
lower bound on removable samples.

| Cohort-2 kind | Observed samples | At most directly exposed | At least outside the candidate |
| --- | ---: | ---: | ---: |
| Register-to-register move | 1,909 (13.024%) | 266 (1.815%) | 1,643 |
| Frame load/store, including spills and reloads | 1,033 (7.047%) | 1,033 (7.047%) | 0 |
| Prologue/epilogue | 691 (4.714%) | 0 | 691 |
| Joint, exclusive | 3,633 (24.785%) | **1,299 (8.862%)** | **2,334** |
| All other kinds | 11,025 | 0 directly credited | 11,025 |

**NO-GO for these three changes as pass 2.** The direct exposure ceiling
requires every eligible move and frame sample to vanish. It still falls 167
samples short of the 1,466 needed for a 10% share. Treating samples as CPU
time and holding the retained run's remaining wall time fixed gives an
even lower optimistic wall proxy of 8.708% (`8.862% × 14.69/14.95`).
This rejects the proposed mechanism as a 10% opportunity on retained
attribution. It supplies no hard wall-time bound or forecast. Allowing
every target-kind sample anywhere in a nominated function would raise
the upper bound back to 24.785%, because marginal totals admit that
allocation. Such a relaxation assigns mandatory and unavailable work to
the candidate and cannot justify a performance trial. Reopen pass 2 if
a distinct native mechanism makes more direct opportunity visible on the
same artifacts. A concrete next question is whether direct operand selection
can remove moves in the hot scalar loops that have no eligible transfer edge.

The historical profile has 15,122 total samples. The three exclusive kinds
are 1,913 moves (12.650%), 1,045 frame loads/stores (6.910%) and 753
prologue/epilogue samples (4.980%), a gross 3,711 (24.540%). The cohort-2
profile records a different run with an exact selected-IR join. Its figures
must use its own 14,658-sample denominator. The 1,503 joint aggregate-copy
samples, allocator, runtime, checked branches and arithmetic, mandatory ABI
transfers, and nonspill frame accesses receive no direct credit. Callee saves,
stack checks, and frame bookkeeping may be labelled prologue. Spills and
reloads already within frame or prologue are counted once. Fixed-offset
compression, changed register-move traffic, instruction layout and cache
effects may affect wall time and remain unmeasured.

## Week 2 native lowering

Keep checked IR, its checker and independent interpreter, sealed runtime and
profile format as the input contracts. Work in `src/native/` and its
`src/native.lkj` emitter. Evaluate the change with a private emitter build so
ordinary compiler policy has no public flag or change during the design and
qualification work.

The following are bounded single-worker implementation slices if the
decision is reopened.

1. **Prefer a dead copy source.** In `registers.lkj`, use `Intervals` and the
   existing before/after points `2*i` and `2*i+1`. Build incumbent interval
   lists per register from `Registers.locals`. Process the same-block,
   same-bank transfer edges in instruction order. Change only the
   destination's assignment when the source's actual register remains free
   across the destination's range, and update that register's occupants
   immediately. Keep the baseline save set in the experiment to isolate the
   change. `Body::copy_local` already skips the resulting self move. Keep
   `scalar.lkj` selectors, including rotation fusion, the destructive binary
   destination/right alias guard, width normalization, signedness, division
   registers and NaN rules. When the peephole owns an instruction range,
   qualify the emitted machine sequence and count only the moves actually
   removed. Acceptance is one paired machine-code fixture check that compares
   against the interpreter and proves an eligible transfer emits no move
   while a live-source twin still emits its move.
2. **Promote a spill and share disjoint homes.** In `registers.lkj`, add a
   volatile clobber mask for the three verified refinements. `Discriminant`
   preserves the whole pool. `TagTest` reserves R10 from that pool.
   Projected primitive `Read` reserves R10 and RSI. The latter two use
   `Body::place_address`, whose other fixed temporaries are RAX, RCX and R11.
   Unknown forms retain the existing conservative barriers. Keep old
   assignments, then promote only a qualifying spill with a conflict-free
   register across its complete interval. Retain linear-scan loop extensions.
   In `storage_plan.lkj`, reuse a spill home for strictly disjoint,
   same-block, unaddressed plain primitive locals of equal width and
   alignment. Keep the aggregate coalescer rule separate, retain slot
   representative indices, and keep the frame offsets and normalized narrow
   load/store paths in `Body::read` and `Body::write`. Acceptance is one
   machine-code and interpreter check of a pressure fixture with a promoted
   spill and reused slot, plus a projected-address or call-clobber twin
   whose allocation remains conservative.
3. **Drop unused scratch from a proven scalar leaf.** In `emit_function`,
   construct the frame after storage assignment. Skip unused, nonparameter
   primitive home slots and the default eight general scratch and
   `Ownership.depth + 2` partial scratch slots only when the complete scalar
   leaf whitelist and location plan prove they cannot be referenced. Require
   no saved registers, no real scalar spill, no ownership flags, no staged
   aggregate ingress or result and no call or frame-using lowering. Keep
   `prologue`, `stack_available`, the BP chain, the common return and
   `epilogue`. The zero-size case omits the frame subtraction and keeps a
   minimum 16-byte growth check. Acceptance is one disassembly and
   differential check showing that the scalar leaf has no frame subtraction,
   its non-leaf twin retains one, and the existing stack refusal fixture
   produces its located R0008.

### Constraints and hot-function failure modes

The allocatable volatile pool is R8, R9, R10, RDI and RSI. The saved pool is
RBX, R14 and R15. RAX, RCX, RDX and R11 are fixed scratch, RSP/RBP are stack
registers, R12 is the platform context and R13 is the stack floor. R14/R15
also carry trap line/file on a terminal edge. Model returning clobbers
separately from cold nonreturning diagnostic writes. Save and restore every
selected saved register on all normal return paths. Keep 16-byte call-site
stack alignment, register arguments RDI, RSI, RDX, RCX, R8, R9, incoming
stack words at `[BP+16+8*n]`, the hidden aggregate result pointer in the
first argument word, and RAX scalar results. Direct calls, runtime calls,
destructor calls, outgoing stack staging, copy helpers and all unmodeled
intrinsics retain conservative clobbers. Keeping parameters out of volatile
allocation in this slice avoids incoming argument permutations and races
with aggregate ingress copies.

A value's storage must hold the last reaching definition on every path where
the checker permits its use. Read points precede write points and loop-carried
intervals extend through backedges and multi-level continue. A plain scalar
with a projection, address, loan, capability, cleanup or address-producing
use stays out of the new rules. Never assign two overlapping live intervals
to one location. Shared spill homes do not share ownership flags or scratch.
Keep call, trap and effect order and all original diagnostic spans. Compute
frame size with existing alignment and signed-displacement checks. Keep
existing resource policy and test the located exhaustion cases. Preserve
the BP prologue pattern used by `profiling.lkj` for sampled stack walks.
`layout.lkj` still owns type layout, `storage.lkj` emits filesystem helpers
and has no allocator work, and unit/object entry points share
`emit_function`. Use the existing native-build identity for authenticated
objects and test a new producer against prior store contents. If a proposed
allocation cannot satisfy a pre-emission invariant, retain the original
function plan before emitting any code.

The hot `std::sequence::__lkjscript_slice_order<T=t5>` has a loop,
pointer-based reads and checked index work. Its 648 self samples include 13
known aggregate-copy samples, and its body qualifies for none of the three
direct sample categories. The mapped `builtin:$order:t271` likewise
qualifies for none (231 self samples). Checker and elaborator
hot bodies are dominated by matches, aggregate owners, cleanup, long live
ranges and calls. New masks may find gaps but hidden result pointers,
fixed scratch and addressable state limit register placement. Artificially
shortened intervals can corrupt reaching definitions only on a backedge or
one arm. Recoloring can replace a spill with multiple moves, consume a
register needed by another temporary or lengthen the instruction stream.
Hand-emitted runtime allocation, byte validation and platform helpers
sit outside the function-plan change.

## Qualification and kill switch

Freeze 18 focused specimens and their authored outcomes before candidate
construction. Execute each admitted specimen through baseline interpreter
`K`, baseline native `B0/B64` and candidate native `C0/C64`. Use the original
checked IR for `K`, `--no-opt` for policy 0, and `--inline-limit 64` for
policy 64. Use fresh inputs and `--no-store`. For repeatable effects compare
stdout and stderr bytes, process status, exact scalar bits, and the full
located trap payload or effect transcript. For source fixtures replay their
exact checked text IR too. For refusals compare admission diagnostics in
both builds and require no output artifact. Include a separate image/link
check for an ordinary build, a checked unit with an authenticated imported
call, and a transfer-only child. Compare candidate and baseline build
admission as well as observable outcomes.

| Specimens | Cases |
| --- | --- |
| Transfers | Dead-source read, dead-source assign, a live source used after a copy, and an aliased binary right operand under pressure |
| Paths and spills | Branch join with conditional assignment, nested backedge/continue with a carried scalar, eligible tag/projected read with at least five competing values, and a spilled value across a call/opaque intrinsic |
| ABI | Six mixed-width register arguments, seven and eight arguments to cover both stack padding cases, hidden aggregate result plus seven explicit arguments, and recursive saved-register preservation with sentinels around the call |
| Representation | Narrow signed/unsigned loads and casts, and NaN signs/payloads plus a real rotation candidate |
| Observable behavior | Cleanup and returned view with ordered stream writes, projected bounds trap from a distinct file, signed-minimum division and exact custom location/payload via a second entry, and malformed IR with a forged uninitialized read |

Run each test that claims allocation coverage with independent emitted-code
assertions or a private plan dump. An unchanged function checks fallback only.
Inject four temporary native-only faults, one per build. Coalesce a
live source, incorrectly allow R10 across a projected/tag read, overlap
two live spill homes across a backedge, and omit one saved-register restore.
Require a targeted fixture to change result, trap or effects before the
differential check can count that mutant as caught. Include a zero-frame
negative control that removes needed scratch in a non-leaf and require the
frame check or differential to catch it. Run the ordinary existing resource
test with its one heap and one stack refusal. Run independent original-IR
mutation (`mutate 1701 128 --jobs 4 --no-store`) and the full four-worker
gate if the pass survives timing. Require the promoted emitter's bootstrap
fixed point and the normal native and unit/child code paths to pass. Direct
IR mutation tests admission. Native-only faults test the new target
lowering, whose semantics the IR checker cannot establish.

### Nine uninstrumented execution rows

Use the audited `d461c2d9` emitter generation as the common historical base.
Construct emitter twins from identical inputs except for the private
allocation patch. Use both to construct **B** (historical native lowering) and
**C** (candidate native lowering) from the same exact frozen witness with
`--inline-limit 64 --no-store`. Require identical rechecked post-Inline64 IR
across construction arms and the retained digest, and distinct producer
ELFs with planned machine-code differences. B must match the retained
unprofiled cohort-2 producer (SHA-256
`20cfe7aae7dddba36da8edca9b56e17f8983bed7e48eed965ce0a01d43b40856`).
Record construction time, CPU,
peak RSS, producer size and recheck counts separately. Build **R** from the
frozen `witness/Cargo.toml` with the existing locked offline release
procedure (`opt-level=3`, one codegen unit, one Cargo job, and the pinned
Rust toolchain). Freeze emitter, runtime, trees, toolchain and manufactured
image hashes before rows start.

Each row runs its producer on literal `src/main.lkj` under the same frozen
source root, explicit frozen library/runtime, and `--inline-limit 0
--no-store`, using a new output file. C applies the candidate to the
construction of the producer executable. At execution the producer emits
the frozen output via the unchanged witness source. Verify its exact
expected bytes, zero status and empty stdout/stderr after timing each row.
Run serially in **B, C, R | C, R, B | R, B, C** order, on one host and
within one reserved performance window. Use the historical witness's
external 4 GiB address-space envelope and 900 s watchdog with 10 s kill
grace for all rows, scoped to this historical comparison. Do not put those
settings into the compiler, runtime or normal defaults. Use GNU time's
elapsed hundredths as the primary observation. Record user/system time,
RSS, load before/after, output hash and endpoints. Time no profile
instrumentation, construction, oracle, hashing or output comparison.
Keep all nine rows and no warmup, discarded measurement, rerun or
outlier substitution. An artifact, status, or identity failure invalidates
the cohort and stops promotion.

Let `sum_B`, `sum_C`, and `sum_R` be the exact three-row wall totals.
Performance eligibility requires `10*sum_C <= 9*sum_B`. Report the three
paired blocks, mean wall and CPU, and `sum_B/sum_R` and `sum_C/sum_R`.
Below 10%, discard the candidate under the current pass rule. At or above
10%, run the four-worker gate and fixed-point qualification before
proposing pass 2 for retention. If three landed passes still exceed 3×
Rust `-O`, document the remaining wall in ten status lines and stop this
optimization thread.
