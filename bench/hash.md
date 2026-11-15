# Fixed-byte producer hash attribution

The semantic question path must authenticate the exact executing compiler.
The first index implementation measured about 0.7 ms retrieving facts and about
243 ms hashing the producer. This experiment isolates that hash, including the
driver's 64 KiB file-read/update pattern. It does not replace producer identity
with an embedded identifier or metadata hint.

Run one worker:

```sh
bash bench/hash.sh --compiler target/lkjscript --input target/lkjscript \
  --repeats 20 --results target/compiler-hash.tsv
```

The harness copies the input, source library and workload before any comparison,
checks every digest against sha256sum, records the producer and source hashes,
and runs all builds and measurements sequentially. It records wall/user/kernel
time, RSS, image bytes and all three load averages. The program's internal clock
covers twenty fresh file rewinds and SHA states, excluding source compilation and
entry startup. File bytes are read anew each repetition, so an optimizing witness
cannot replace the repeated work with one digest. Temporary source trees, images
and sample files are removed on exit; the selected result files remain.

The default 2 GiB address-space ceiling is an experimental host-safety bound,
derived from the measured sub-40 MiB hash build and sub-200 MiB Rust witness build.
It is not a language resource policy. Free space is checked before copying.
`--memory-mib` changes that bound. No Python, Node or WebAssembly participates.

## Before changing code

The input was the 17,039,220-byte compiler at source baseline `1f48bd24`,
SHA-256
`944d5e663bc6f7f703b8a399656f774a785966af4029ac56294e4b4c0fc2d2e7`.
The unchanged `library/sha256.lkj` source hash was
`1cc3ea4cdde49659c4acc438211b7913dc6a274519b07c43a33d56dc8b0ee75c`.
The native default IR inline limit was zero. Rust was
`rustc 1.97.0 (2d8144b78 2026-07-07)`, one codegen unit, overflow checks on,
panic abort, edition 2024. Only the host namespace in the workload differs;
the SHA implementation is the same source file.

All rows hash twenty copies of the fixed input. Milliseconds/hash below divide
the program's measured interval by twenty; they are not independent one-shot
query timings. Loads are 1/5/15-minute averages before → after.

| Route | ms/hash | Wall / user / kernel s | RSS KiB | Load |
| --- | ---: | ---: | ---: | --- |
| Native baseline | 230.867 | 4.61 / 4.50 / 0.01 | 264 | 2.13/2.51/4.01 → 2.13/2.51/4.01 |
| Rust opt0 | 112.163 | 2.24 / 2.19 / 0.01 | 1,764 | 2.13/2.51/4.01 → 2.20/2.52/4.01 |
| Rust opt1 | 47.578 | 0.95 / 0.91 / 0.01 | 1,976 | 2.20/2.52/4.01 → 2.20/2.52/4.01 |
| Rust opt2 | 44.648 | 0.89 / 0.86 / 0.01 | 1,936 | 2.20/2.52/4.01 → 2.20/2.52/4.01 |
| Rust opt3 | 43.136 | 0.86 / 0.83 / 0.01 | 1,956 | 2.20/2.52/4.01 → 2.20/2.52/4.01 |
| Rust opt3, MIR and LLVM inlining off | 145.085 | 2.90 / 2.82 / 0.02 | 1,912 | 2.20/2.52/4.01 → 2.11/2.50/3.99 |

This hash is 5.352× Rust opt3 and 2.058× opt0. These are hash ratios; they do not
replace the earlier full-compiler ratios of 6.053× and 1.826×. Disabling inlining
globally also prevents ordinary Rust scalar helpers from being inlined; the
145.085 ms row is an intervention, not a claim that cross-library function calls
alone explain that amount of the compiler gap.

For the no-inline witness, scoped `RUSTC_BOOTSTRAP=1 -Zinline-mir=no` disables
MIR inlining; LLVM receives `--force-remove-attribute=alwaysinline` and
`--force-attribute=noinline`. Merely lowering the inlining threshold would not
disable always-inline functions. The harness also records an opt3/MIR-only-off
row to separate the two stages. No toolchain configuration is changed.

### Native profile

Twenty profiled repetitions took 5.151 s internally (outer wall/user/kernel
5.15/5.04/0.02 s), load 2.11/2.50/3.99 → 2.42/2.56/4.00. The profile binary
collected 5,031 CPU samples at a requested 1,000 μs period:

| Observation | Samples | Share |
| --- | ---: | ---: |
| SHA compress self time | 4,983 | 99.0% |
| File-read leaf self time | 18 | 0.3% |
| Register-to-register moves | 2,320 | 46.1% |
| Runtime checks | 918 | 18.2% |
| Arithmetic/comparison/conversion | 652 | 12.9% |
| Immediate constants | 611 | 12.1% |
| Frame loads/stores | 359 | 7.1% |

The instruction categories overlap the function totals, not each other.
A profile changes code and adds sampling overhead; its elapsed time is not
substituted for the unprofiled baseline.

Inspection found that every shift, including a literal count, loaded the count
into CL and performed a width comparison and conditional trap. Every fixed-array
index, including a literal index, also performed a dynamic bounds comparison and
stride multiplication. These are direct measured candidates for instruction
selection; file I/O and hash protocol changes are not.

## Discarded source-only trials

Both trials hashed the same fixed bytes correctly. Neither is retained in the
library. Each twenty-repetition row has its own paired load observation.

| Trial route | ms/hash | Wall / user / kernel s | Load |
| --- | ---: | ---: | --- |
| Baseline before full unroll | 226.615 | 4.53 / 4.42 / 0.01 | 2.13/2.45/3.84 → 2.13/2.45/3.84 |
| Literal constants, 64 rounds unrolled, rotated variable names | 345.231 | 6.90 / 6.72 / 0.01 | 2.13/2.45/3.84 → 2.42/2.51/3.84 |
| Baseline paired repeat | 224.233 | 4.48 / 4.40 / 0.01 | 2.42/2.51/3.84 → 2.38/2.50/3.83 |
| Full unroll paired repeat | 341.374 | 6.82 / 6.68 / 0.02 | 2.38/2.50/3.83 → 2.43/2.51/3.82 |
| Baseline before buffer hoist | 226.694 | 4.53 / 4.44 / 0.01 | 2.57/2.50/3.73 → 2.53/2.49/3.72 |
| K table and schedule buffer initialized once per hash | 255.158 | 5.10 / 4.85 / 0.02 | 2.53/2.49/3.72 → 2.64/2.52/3.72 |
| Baseline paired repeat | 239.024 | 4.78 / 4.64 / 0.02 | 2.64/2.52/3.72 → 2.59/2.51/3.71 |
| Buffer hoist paired repeat | 230.734 | 4.61 / 4.52 / 0.01 | 2.59/2.51/3.71 → 2.63/2.52/3.71 |

The full-unroll workload image grew from 112,150 to 177,686 bytes, while literal
indices still retained dynamic bounds checks. The buffer-hoist image stayed
112,150 bytes and showed no repeatable ten-percent gain. The observations reject
these changes; they do not establish an instruction-cache or register-spill cause
without another profile.

## Instruction-selection experiments

The next experiments leave the checked IR unchanged. They select machine
instructions only after ordinary IR admission. Every result below hashes the same
17,039,220 bytes twenty times and matches the reference digest above. These are
**SHA workload** results; they neither replace the fixed-source compiler ratio nor
establish its 4x/3x/2x milestones. [compiler_attribution.md](compiler_attribution.md)
records the exact original compiler workload and its opt-level/no-inline matrix.

| Experiment | Before mean ms/hash | After mean ms/hash | Reduction | Decision | Paired load range |
| --- | ---: | ---: | ---: | --- | --- |
| Literals | 222.540 | 207.671 | 6.68% | Discard | 2.33/3.10/3.64 → 1.93/2.95/3.57 |
| Word32 alone | 228.764 | 212.271 | 7.21% | Discard | 5.18/3.00/3.39 → 4.71/3.00/3.39 |
| Literals + word32 | 224.773 | 190.536 | 15.23% | Superseded; removed before rotation trial | 4.66/3.39/3.40 → 4.20/3.36/3.39 |
| Rotation | 260.475 | 184.880 | 29.02% | Retain | 50.07/19.01/11.40 → 40.51/18.88/11.52 |
| Canonical results | 179.126 | 152.821 | 14.68% | Retain | 3.17/5.07/7.21 → 3.66/5.07/7.16 |
| Operand reuse | 167.857 | 158.903 | 5.33% | Discard | 9.13/6.37/6.30 → 8.88/6.45/6.33 |
| Scaled addresses | 156.532 | 145.781 | 6.87% | Discard | 6.10/5.94/6.13 → 6.20/5.98/6.14 |

The load-range column spans the observations used by each mean; each individual
row and its paired load follow below. Means are not fresh measurements.

The literal trial required a local to have exactly one literal write, no parameter
write, no address taking, and no projected or indirect write through an alias.
It selected immediate shifts only for positive/in-range counts and fixed-array
addresses only for proven valid indices and representable displacements. Invalid
constants retained the old traps. Its 6.68% result was below the continuation
rule, so the analysis and both selections were removed. The unsigned-32-only
trial likewise stayed below ten percent. Their combined trial exceeded ten
percent, but was removed in full before isolating the simpler rotation rule.
No constant-propagation helper survives in the retained implementation.

### Exact rotation selection

A seven-instruction checked-IR sequence copies one plain unsigned local, shifts
it by a literal count, copies the same local again, shifts it in the other
direction by the complementary count, and ORs the results. The selected counts
are both strictly between zero and the width and sum to 32 or 64. No instruction
in the sequence writes the common input, and there are no projections, effects,
calls or control edges. Checked consuming operands prevent an intermediate value
from escaping into a later use. The native selector replaces only this exact
sequence with one ROR instruction; signed values, different inputs, unequal
counts and invalid counts retain their original path. Integer temporaries have
no cleanup flags or custom drop effects to omit.

The selector loads the common input before writing the final destination and
uses the final OR's source location. Register reuse therefore cannot overwrite
the input before it is read. A separate code review found no alias or liveness
counterexample in this restricted form. The first rotation row's outer wall time
includes time outside the workload's clock; its 5.69 s outer wall is not treated
as 3.878 s of hashing. The host was unusually loaded during that pair.

### Canonical scalar results

A shared native write helper omits normalization only when the selected
instruction already produces the checked type's complete register representation.
Typed scalar loads retain their exact signed or unsigned width and load directly
into the assigned destination register. A signed narrow load still sign-extends.
Unsigned 32-bit wrapping arithmetic, bitwise operations and shifts use r32
instructions, whose results zero-extend; shifts keep the original width test
before the machine masks CL. Checked add/multiply retain their old paths. ROR
results use the same helper. All other writes continue to normalize normally.
This coherent selection reduced the next paired mean by 14.68%. It is distinct
from retaining the earlier sub-ten-percent word32-only experiment.

Direct use of assigned right-operand registers and constant-stride scaled
addressing were then measured independently. Neither reached ten percent, and
both were removed. The scaled-address trial kept every bounds check; its
multiply fallback and its new encoder were removed together. The original
place-address body and the scalar operand paths are restored.

### Individual observations

| Experiment / route | ms/hash | Wall / user / kernel s | RSS KiB | Load before → after |
| --- | ---: | --- | ---: | --- |
| Literals / native | 223.115 | 4.46 / 4.38 / 0.01 | 264 | 2.33/3.10/3.64 → 2.30/3.08/3.63 |
| Literals / literals | 207.562 | 4.15 / 4.07 / 0.02 | 264 | 2.30/3.08/3.63 → 2.20/3.05/3.61 |
| Literals / native | 221.193 | 4.42 / 4.35 / 0.01 | 264 | 2.20/3.05/3.61 → 2.10/3.01/3.60 |
| Literals / literals | 208.680 | 4.17 / 4.09 / 0.01 | 264 | 2.10/3.01/3.60 → 2.01/2.98/3.58 |
| Literals / native | 223.313 | 4.46 / 4.36 / 0.02 | 264 | 2.01/2.98/3.58 → 1.93/2.95/3.57 |
| Literals / literals | 206.770 | 4.13 / 4.08 / 0.01 | 264 | 1.93/2.95/3.57 → 1.93/2.95/3.57 |
| Word32 alone / native | 229.300 | 4.58 / 4.50 / 0.01 | 264 | 5.18/3.00/3.39 → 4.92/2.98/3.39 |
| Word32 alone / u32 | 211.241 | 4.22 / 4.16 / 0.01 | 264 | 4.92/2.98/3.39 → 4.77/2.98/3.38 |
| Word32 alone / native | 228.229 | 4.56 / 4.47 / 0.02 | 264 | 4.77/2.98/3.38 → 4.71/3.00/3.39 |
| Word32 alone / u32 | 213.300 | 4.26 / 4.17 / 0.02 | 264 | 4.71/3.00/3.39 → 4.71/3.00/3.39 |
| Literals + word32 / native | 227.684 | 4.55 / 4.42 / 0.02 | 264 | 4.66/3.39/3.40 → 4.53/3.39/3.40 |
| Literals + word32 / combined | 190.747 | 3.81 / 3.72 / 0.02 | 264 | 4.53/3.39/3.40 → 4.48/3.40/3.40 |
| Literals + word32 / native | 221.862 | 4.43 / 4.35 / 0.03 | 264 | 4.48/3.40/3.40 → 4.20/3.36/3.39 |
| Literals + word32 / combined | 190.325 | 3.80 / 3.72 / 0.02 | 264 | 4.20/3.36/3.39 → 4.20/3.36/3.39 |
| Rotation / native | 278.845 | 5.57 / 5.38 / 0.05 | 264 | 50.07/19.01/11.40 → 47.34/18.96/11.42 |
| Rotation / rotate | 193.903 | 5.69 / 3.75 / 0.03 | 264 | 47.34/18.96/11.42 → 47.15/19.39/11.60 |
| Rotation / native | 242.105 | 4.84 / 4.69 / 0.03 | 264 | 47.15/19.39/11.60 → 43.78/19.15/11.57 |
| Rotation / rotate | 175.857 | 3.51 / 3.45 / 0.03 | 264 | 43.78/19.15/11.57 → 40.51/18.88/11.52 |
| Canonical results / rotate | 180.940 | 3.61 / 3.53 / 0.03 | 2068 | 3.17/5.07/7.21 → 3.40/5.09/7.20 |
| Canonical results / canonical | 149.541 | 2.99 / 2.93 / 0.02 | 264 | 3.40/5.09/7.20 → 3.40/5.09/7.20 |
| Canonical results / rotate | 177.145 | 3.54 / 3.43 / 0.03 | 264 | 3.40/5.09/7.20 → 3.61/5.10/7.20 |
| Canonical results / canonical | 150.549 | 3.01 / 2.94 / 0.03 | 264 | 3.61/5.10/7.20 → 3.61/5.10/7.20 |
| Canonical results / rotate | 179.293 | 3.58 / 3.50 / 0.03 | 264 | 3.61/5.10/7.20 → 3.72/5.10/7.18 |
| Canonical results / canonical | 158.375 | 3.16 / 3.08 / 0.02 | 264 | 3.72/5.10/7.18 → 3.66/5.07/7.16 |
| Operand reuse / canonical | 169.011 | 3.38 / 3.30 / 0.03 | 264 | 9.13/6.37/6.30 → 9.13/6.37/6.30 |
| Operand reuse / operands | 144.625 | 2.89 / 2.80 / 0.04 | 264 | 9.13/6.37/6.30 → 9.04/6.39/6.31 |
| Operand reuse / canonical | 160.964 | 3.21 / 3.14 / 0.03 | 264 | 9.04/6.39/6.31 → 9.04/6.39/6.31 |
| Operand reuse / operands | 167.404 | 3.34 / 3.28 / 0.03 | 264 | 9.04/6.39/6.31 → 8.96/6.42/6.32 |
| Operand reuse / canonical | 173.598 | 3.47 / 3.37 / 0.04 | 264 | 8.96/6.42/6.32 → 8.88/6.45/6.33 |
| Operand reuse / operands | 164.679 | 3.29 / 3.17 / 0.04 | 264 | 8.88/6.45/6.33 → 8.88/6.45/6.33 |
| Scaled addresses / canonical | 163.156 | 3.26 / 3.17 / 0.01 | 264 | 6.10/5.94/6.13 → 6.10/5.94/6.13 |
| Scaled addresses / scaled | 148.859 | 2.97 / 2.90 / 0.02 | 264 | 6.10/5.94/6.13 → 6.26/5.98/6.14 |
| Scaled addresses / canonical | 152.668 | 3.05 / 3.01 / 0.01 | 264 | 6.26/5.98/6.14 → 6.48/6.03/6.16 |
| Scaled addresses / scaled | 143.196 | 2.86 / 2.81 / 0.02 | 2064 | 6.48/6.03/6.16 → 6.48/6.03/6.16 |
| Scaled addresses / canonical | 153.770 | 3.07 / 3.01 / 0.01 | 264 | 6.48/6.03/6.16 → 6.20/5.98/6.14 |
| Scaled addresses / scaled | 145.287 | 2.90 / 2.82 / 0.01 | 264 | 6.20/5.98/6.14 → 6.20/5.98/6.14 |

The later witness run separates the inlining stages while using the same SHA
source and bytes. Opt3 was 42.070 ms/hash (outer 0.84/0.81/0.01 s,
load 1.95/3.46/3.80 → 1.95/3.46/3.80). Disabling MIR inlining alone gave
48.596 ms/hash (0.97/0.93/0.01 s, the same paired load); disabling both MIR and
LLVM inlining gave 143.130 ms/hash (2.86/2.78/0.01 s,
load 1.95/3.46/3.80 → 1.88/3.42/3.79). These interventions include ordinary
library scalar helpers and are not an additive attribution of the compiler gap.

The rotation-only profile collected 3,732 samples: compress 3,668 (98.2%),
register moves 1,587 (42.5%), checks 541 (14.4%), arithmetic 706 (18.9%), frame
traffic 352 (9.4%), and constants 306 (8.1%). It ran twenty hashes in 3.790 s
internally, outer 3.79/3.72/0.02 s, load 7.99/13.02/10.60 → 7.67/12.87/10.57.
That profile motivated the subsequent experiments; their failures remain in the
table rather than being hidden by a preferred repeat.

### Eight-round source trial after native selection

After restoring the retained native selector, one bounded source trial grouped
eight SHA rounds per loop and rotated the names of the eight state variables.
Each round computed the same two sums, assigned only the two changed words,
and left all array bounds and arithmetic rules intact. After eight rounds the
variable names aligned again. This avoided the full-unroll image growth while
testing the measured register-move cost. It was applied as one body operation.

| Route | ms/hash | Wall / user / kernel s | RSS KiB | Load before → after |
| --- | ---: | --- | ---: | --- |
| Retained selector / original SHA 1 | 164.525 | 3.29 / 3.22 / 0.03 | 264 | 6.17/4.69/4.82 → 6.39/4.76/4.85 |
| Eight rounds 1 | 146.943 | 2.93 / 2.86 / 0.03 | 264 | 6.39/4.76/4.85 → 6.39/4.76/4.85 |
| Retained selector / original SHA 2 | 161.555 | 3.23 / 3.17 / 0.02 | 264 | 6.39/4.76/4.85 → 6.44/4.80/4.86 |
| Eight rounds 2 | 140.918 | 2.81 / 2.70 / 0.04 | 264 | 6.44/4.80/4.86 → 6.44/4.80/4.86 |
| Retained selector / original SHA 3 | 161.673 | 3.23 / 3.17 / 0.02 | 264 | 6.44/4.80/4.86 → 6.33/4.80/4.86 |
| Eight rounds 3 | 155.544 | 3.11 / 3.02 / 0.03 | 264 | 6.33/4.80/4.86 → 6.22/4.81/4.86 |

The paired means were 162.585 → 147.802 ms/hash, a **9.09%** reduction.
The workload image grew from 109,566 to 113,662 bytes. The trial stayed below
ten percent and was discarded; no preferred repeat was selected.

Before editing, the original checked IR for SHA-256, HMAC-SHA256 and HMAC boundary
cases was saved and independently interpreted. All returned 42. The modified
native cases agreed with those saved original-IR outputs, and every workload
digest agreed with the host SHA-256 reference. Original-IR SHA-256 identities were:

| Case | Saved original checked IR SHA-256 |
| --- | --- |
| SHA-256 | `3088370bb939752d7170958d1087cc48f6004e96aa7d8d1ea94b745d18afe9a4` |
| HMAC-SHA256 | `0c0ddfcd02de5abd792a6f1f0fb75abeefb74b680aad4ddbb9b4bf299bd03908` |
| HMAC boundaries | `245cdbde9d9ff09d38b9d93b8d30506756297fa5333a750a118f0784e8aa1a0d` |

A second body operation restored the SHA source byte-for-byte to
`1cc3ea4cdde49659c4acc438211b7913dc6a274519b07c43a33d56dc8b0ee75c`.
The fixed workload's original checked-IR identity was also restored. Neither
this rejected source change nor its execution speed is part of the retained
native result.

### Oracles and operation use

The focused native/original-IR cases cover both widths of rotation, edge counts,
signed patterns, unequal inputs/counts, mutable counts, borrows, control flow,
scalar loads at every signed/unsigned width, nested arrays, zero-sized elements,
and non-power-of-two aggregate strides. Out-of-width shifts, overflow, and invalid
array/slice projections still trap. The final follow-up rebuilt the retained producer and executed twenty-three
cases and forty-six native/original-IR routes with no pending cases; six optional
Rust value/output witnesses independently agreed. This includes SHA-256, HMAC
and HMAC boundary cases as well as scalar and trap cases. The SHA workload is
also checked against the host SHA-256 implementation on every measured run.
The original-IR oracle is for these bounded cases, not an interpreted compiler
self-build.

| Final validation | Wall / user / kernel s | Peak RSS KiB | Load before → after |
| --- | ---: | ---: | --- |
| Retained producer build | 20.16 / 19.63 / 0.20 | 517,396 | 24.45/10.26/7.24 → 18.82/9.90/7.19 |
| 23 cases / 46 native and original-IR routes | 12.54 / 12.42 / 0.04 | 44,236 | 18.82/9.90/7.19 → 16.61/9.72/7.16 |
| 6 Rust value/output witnesses | 0.24 / 0.26 / 0.05 | 198,044 | 16.61/9.72/7.16 → 16.61/9.72/7.16 |

All later compiler body/item changes and experimental rollbacks were submitted
as checked identity transactions. Rejected scope syntax, a too-small external
2 GiB address-space ceiling, and an incorrectly typed identity left the source
unchanged. A successful compiler transaction used a 3 GiB experimental ceiling;
that ceiling is not a language policy.

Net retained source additions plus deletions from the saved operation-activation
snapshots are shown below. This measures source adoption after the CLI became
available, excluding earlier edits and canceled experiment churn.

| Source | Net changed lines through operations |
| --- | ---: |
| native.lkj | 50 |
| native/scalar.lkj | 152 |
| x86.lkj | 2 |
| native_u32_selection.lkj | 45 |
| native_literal_selection.lkj | 49 |
| **Total through operations** | **298** |

Two new trap files added six lines by ordinary file creation; the available
operation CLI did not create a new source file. The retained source share is
**298/304 = 98.03%** in that measured interval. One transient projection-only
EOF blank line outside every item span needed an ordinary text deletion after
remove-item. That cleanup cancels out of the final net delta and is disclosed
separately. Shell and benchmark documentation edits are outside this source
measure. The library source change and rollback cancel exactly.

Use `--previous COMPILER` with the hash harness to build both producers from the
same copied source and interleave three old/new pairs. Profile and optional Rust
rows remain separately labelled. The original fixed-byte workload's retained
canonical-result mean is still above 100 ms; no query-latency target or whole
compiler optimization milestone is claimed by this table.

## Integrated producer follow-up and discarded copied-operands fusion

After integration of image authority, questions and structural operations, the
compiler itself is 19,584,049 bytes, SHA-256
`54db01f51ca7b5802f932a8f4b76737ee2671cbe306f229c62e3c3ccae54c1e3`.
It was emitted for one extra generation so its own hashing code uses the
retained selectors. This is a larger input than the original 17,039,220-byte
hash workload; the two input sizes are not silently treated as the same trial.

Twenty unprofiled hashes of the integrated producer took 178.761 ms each,
outer 3.57 / 3.50 / 0.02 s wall/user/kernel, RSS 264 KiB,
load 7.13/6.04/5.81 → 6.96/6.02/5.80. The separate profiled route took
3.50 / 3.42 / 0.02 s, RSS 272 KiB, load
6.96/6.02/5.80 → 6.96/6.02/5.80; its elapsed interval does not replace the
unprofiled result. Of 3,429 samples, 3,383 (98.6%) were in SHA compression.
Arithmetic/conversion accounted for 988 samples (28.8%), checks 664 (19.3%),
register moves 628 (18.3%), frame traffic 405 (11.8%), constants 341 (9.9%),
branches 159 (4.6%), other memory 122 (3.5%), address formation 91 (2.6%) and
runtime 22 (0.6%). The exact one-shot query measurements are in
[the final compiler report](compiler-query-final.md); all forty commands still
exceeded 100 ms.

The checked SHA IR contains exact sequences of two plain copying Reads
followed by one consuming Binary. One trial fed the original source locals
directly to the existing scalar selector. It accepted only identical unsigned
32- or 64-bit operand types, preserved the original Binary's source location,
and excluded sources or final destinations that alias either temporary/source
in a way that changes the original sequencing. In particular, the first Read's
destination cannot be the second Read's source: that would make the omitted
write semantically observable. Borrowed temporaries cannot be consumed by the
original checked Binary. Unsigned primitive temporaries have no custom Drop
or ownership cleanup to omit.

Independent review found no counterexample under those restrictions. The
focused interpreter/native suite was expanded to 27 cases and 54 routes,
including adversarial checked-IR dependency aliases, final-destination reuse,
a surviving shared view of a source, checked overflow, division by zero and an
out-of-width shift. All passed. The trial was then measured against the same
retained selector on the exact same integrated producer bytes, three interleaved
pairs of twenty hashes:

| Route | ms/hash | Wall / user / kernel s | Load before → after |
| --- | ---: | --- | --- |
| Retained selector | 177.100 | 3.54 / 3.45 / 0.02 | 7.58/8.26/7.63 → 7.61/8.26/7.63 |
| Copied-operands trial | 172.375 | 3.44 / 3.38 / 0.02 | 7.61/8.26/7.63 → 7.73/8.27/7.64 |
| Retained selector | 179.636 | 3.59 / 3.54 / 0.02 | 7.73/8.27/7.64 → 7.59/8.23/7.63 |
| Copied-operands trial | 176.920 | 3.53 / 3.49 / 0.02 | 7.59/8.23/7.63 → 7.59/8.23/7.63 |
| Retained selector | 179.504 | 3.59 / 3.52 / 0.02 | 7.59/8.23/7.63 → 7.54/8.21/7.63 |
| Copied-operands trial | 171.617 | 3.43 / 3.39 / 0.02 | 7.54/8.21/7.63 → 7.58/8.21/7.63 |

Each unprofiled row has 264 KiB peak RSS and a 110,152-byte workload image.
Equal image lengths do not establish equal machine-code bytes. All digests
match the independent host reference. The paired means are
178.747 → 173.637 ms/hash, a **2.86%** reduction. The trial is discarded under
the ten-percent rule. The original scalar-copy path already selects direct
assigned-register moves, but no additive attribution of the remaining cost is
claimed from that observation alone.

| Trial validation / rollback | Wall / user / kernel s | Peak RSS KiB | Load before → after |
| --- | ---: | ---: | --- |
| Apply checked trial transaction | 39.57 / 38.56 / 0.31 | 920,188 | 7.62/8.76/7.65 → 6.63/8.35/7.56 |
| Build trial producer | 21.41 / 20.81 / 0.17 | 534,012 | 6.63/8.35/7.56 → 8.64/8.66/7.67 |
| 27 cases / 54 native and original-IR routes | 13.04 / 12.77 / 0.07 | 39,860 | 6.84/8.15/7.59 → 7.58/8.26/7.63 |
| Restore original source transaction | 42.08 / 40.84 / 0.39 | 921,764 | 8.72/8.56/7.90 → 8.96/8.68/7.97 |

The restoration returns the compiler's canonical checked IR to
`b677ad116d98b50f4d952d0dcf0b733e7ae0edc50716f73525c803aa093b5a0c`
and its production source bytes to the committed retained selector. Removing
the added helper left one extra EOF blank line outside item spans; guarded
projection cleanup removed that line. Both source operations and their
rollbacks cancel from the retained adoption count. Four hand-authored IR
regression fixtures remain; the source-operation CLI does not edit IR
fixtures. There is no retained fusion implementation or source-library change.

This concrete follow-up leaves the exact-producer hashing wall named, not
solved. It establishes neither the 100 ms question target nor a whole-compiler
-O milestone, and it is not a proof of a permanent backend performance floor.

### Remaining work indicated by the existing profile

The 178.761 ms observation is not a lower bound. Hashing alone needs about
44.1% less elapsed time to reach 100 ms, and a whole command also needs startup
and fact-retrieval headroom. Two measured bounded slices already spend about
89–90 ms in the index phase, so merely bringing hashing just below 100 ms would
not complete the command-latency target.

A rough Amdahl calculation illustrates the scale of the remaining work.
Deleting every sampled check would leave approximately 144 ms; deleting every
register move and frame access would leave approximately 125 ms. These are
optimistic calculations from sampled instruction categories, not measured
speedups or rigorous performance bounds. Some checks, copies and memory
accesses are necessary. The three sampled call instructions concern this SHA
profile; they do not support ordinary native inlining as its next dominant
hot-path intervention. Whole-compiler workloads need separate inlining
measurements.

The concrete general work is stronger control-flow range and value analysis.
The SHA loops expose specific proofs: the input loop has i below 16 and
i*4+3 below 64; the schedule loop has i from 16 through 63 and therefore safe
i-2, i-7, i-15 and i-16 offsets; the round loop has i below 64. A transformation
must preserve the original overflow, shift and projection traps whenever those
facts are not established. Eliminating initial schedule zeroing additionally
requires a complete overwrite and no-escape proof. A source name or an
unverified index fact cannot provide any of these guarantees.

Copy equivalence and liveness across loop updates are another concrete
opportunity. The current allocator assigns registers to local intervals; the
smaller direct-operand and copied-read trials did not deliver ten percent.
An IR pass cannot simply replace copied temporaries with their original
locals when Binary consumes its operands: the resulting ownership/use
structure may fail the checker. A backend transformation can retain the
original checked IR, but needs a more complete value and alias analysis than
the rejected three-instruction pattern. Any new proof representation must be
validated at the existing trust boundary.

A narrower untried candidate is non-destructive wrapping-add selection through
LEA. The aggregate profile does not establish that it would exceed ten percent,
and it provides no credible measured route for that selector alone to the
required reduction. No further experiment is claimed here. The next supported
work item is a larger range-analysis and register-allocation project with
independent differential oracles and paired measurements. These observations
name the remaining wall; they do not establish a permanent backend floor.

### Production-only adoption under the landed-commit denominator

The final status uses a separate, conservative denominator: additions plus
deletions in retained `src/` and `library/` .lkj files in the landed change,
excluding benchmark files and tests/conformance. For source commit
`5ba8e48f` (main's `f9e8eb6e`), that denominator is 152 lines:

| Production file | Landed additions + deletions | Conservatively credited to operations |
| --- | ---: | ---: |
| native.lkj | 30 + 4 = 34 | 33 |
| native/scalar.lkj | 101 + 0 = 101 | 100 |
| x86.lkj | 17 + 0 = 17 | 0 |
| **Total** | **152** | **133 (87.50%)** |

The credited native hunks are the typed scalar-load replacement (4 additions,
2 deletions), rotation dispatch loop (9 additions, 2 deletions), and new
canonical-write implementation (16 additions). The scalar count includes the
four-line final dispatch hook and 96 lines of added rotation/emission/word32
items. A boundary blank line in each file is excluded from the lower bound.

All x86 additions are excluded conservatively. Its encoder helpers were first
written before operation activation. Two were later removed during the
rotation-only trial and reintroduced through the canonical-result transaction,
and the final projection also contains two added blank lines. This measure
does not credit the whole rewritten assembler implementation or conflate
reapplication with initial authorship. The earlier 298/304 table remains the
different activation-snapshot-and-fixture measure; it is not substituted for
this production-only fraction.

The later copied-operands experiment retains no additional production .lkj
delta after rollback. Its four IR regression fixtures add 137 text lines
outside this production-source denominator and outside the source operation API.
