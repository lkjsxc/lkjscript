# Fixed twenty-body copy-family census

No fresh Struct family in the frozen twenty complete bodies qualifies for the
initial scalar-replacement subset. All 64 constructors have an independent
blocking rule. This closes the proposed first subset for this population;
it establishes neither a whole-program zero nor a backend performance floor.
No optimizer or production source changed.

The [joint attribution cohort](compiler-profile-copy-sites-cohort-2.md) supplied
1,503 copy samples out of 14,658 total samples (10.253786%), with exact sites
and the selected checked IR. The [plan](compiler-copy-sites-census-plan.md)
fixed the first twenty distinct function indices in their order of appearance
on the sole 200-row detail page, before this inspection. No additional page or
replacement function was chosen. Complete bodies were necessary because a
copy's payload type alone does not describe its containing aggregate or lifetime.

## Exact input and complete context

The ordinary and instrumented producers built the same witness source at
26bf2c3c021dd79810a81fa1d9054d59f3768b06. Current producer bytes were
6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc.
The selected companion was 57,649,044 B, SHA-256
32c0f43dbde125e43f2ca5a0c6d8548ba0f1774ce3b51bc62d94a6488af0fdca;
the profile was 55,464,197 B,
7849d4dd69906491d07664f493eb0f4fd8340cfde3760f6e4f168dbbde95f302.
These identities bind the coordinate system; source names remain frontend
claims, not extra admission authority.

The [separately released extraction](compiler-copy-sites-census-extract-release-20261116.md)
ran once. All twenty functions fit the fixed export limits: 65,536 instructions,
16 MiB body, 16,384 types, 2,048 signatures and 32 MiB logical context per
function. A separate 32 MiB IR-line bound was accepted before execution.
Closure construction precedes some export-limit checks, so the external
4 GiB / 300 s / 10 s grace envelope remains material.

The [independent verifier](compiler-copy-sites-census-acceptance/README.md)
rescanned the original companion without invoking the extractor. It verified
all 6,202 instruction coordinates and complete bodies, 1,882 type occurrences
(701 distinct), 198 callee/destructor signature occurrences (97 distinct),
651 signature components, 140 source declarations and 202 destructor records.
No source path named by the companion was opened. Every original selection,
type closure, signature reason, byte range and recorded digest agrees.

| Complete reading material | Total B | Median B | Maximum B | Minimum B |
| --- | ---: | ---: | ---: | ---: |
| Twenty bodies | 203,690 | 2,644 | 56,134 | 1,206 |
| Bodies and logical context | 770,639 | 23,613 | 122,246 | 22,762 |

Logical context includes the complete 20,176 B shared source/destructor records
once per question, although physically stored once. The
[selection table](compiler-copy-sites-census-extraction-data/selection.tsv)
contains all twenty names, coordinates and sizes. These are offline review
sizes, not authenticated-query latency measurements.

## Every raw row in the selected functions

The [all-row join](compiler-copy-sites-census-join-data/README.md) read the
complete existing 68,092-row copy table and joined all selected-function rows
to exact whole-body instructions, roles, types, layouts and parameter records.
It did not rerun the compiler, profiler, official reader, accounting helper or
extractor. Instruction-level associations retain containing-family candidates;
they are not assumed to partition removable work.

| Population | Raw rows | Copy samples | Displayed samples | Unshown samples |
| --- | ---: | ---: | ---: | ---: |
| Selected twenty functions | 258 | 469 | 430 | 39 |
| Outside the selection | 67,834 | 1,034 | 395 | 639 |
| Complete profile | 68,092 | 1,503 | 825 | 678 |

Of the selected rows, 70 were displayed and 188 were unshown; 152 have zero
samples. The complete profile has zero unmapped copy samples and zero
detail-versus-kind or serial-versus-total discrepancy. No row was adjusted.
Counts are sampled observations, not bytes moved, removed time or a gain forecast.

The join exited zero but emitted 68,550 Perl portability warnings while parsing
hexadecimal masks above 32 bits: 12,887,400 stderr bytes, SHA-256
7ddb808824da96834af92393b1ce7fd82f37f7226f641b6b1240e55a3b6d436a.
The frozen helper explicitly requires at least 64-bit Perl integers; its plan
required stopping on failure or timeout and did not require empty stderr.
Root accepts the qualified offline result with the full original warnings.
No warning was suppressed, helper repaired or invocation repeated. Its elapsed
time includes those writes and is not a reader-performance result.

A payload-only screen found 78 types and four flat Copy-Struct payload rows,
all t41 diagnostic::Span, accounting for 21 samples. Scalar constructor payloads
can belong to an outer flat aggregate, so this result could not establish
whole-family eligibility. The next inventory therefore examined every Struct
instruction and every flat-shaped local, including unsampled cases.

## Complete-family result

The [full inventory and manual review](compiler-copy-sites-census-family-data/README.md)
cover all 64 Struct roots, 13 flat-shaped locals and three same-type whole-local
Read edges in the same complete bodies. Root independently recomputed the
disjoint constructor partition from the retained records.

| Roots | Independent blocker |
| ---: | --- |
| 60 | Destination is a Move Struct, outside the Copy subset. |
| 1 | Copy closure t3277 contains reference field t1286. |
| 2 | Copy IR tuple t3284 contains enum field t1325. |
| 1 | Fresh Span is consumed whole as an aggregate call argument. |

The reference-bearing constructor is fn2652 i646; the enum-bearing pair is
fn690 i232/i267. An inventory label on some already-blocked nonflat shapes,
no-nonzero-field, means no recognized nonzero scalar field. It does not mean
those aggregates occupy zero bytes. The table above relies on the independent
Move/reference/enum/call exclusions.

The one fresh flat-shaped constructor was missed by the payload-only screen:
fn2652 i1262 initializes l698 from five usize operands, and i1265 consumes it
as argument zero of f1295, diagnostic::new. The complete body establishes one
definition, one use, no earlier use and no whole-local Read descendant. There
is no branch between definition and use. Its enclosing loop is not evidence
that this value crosses a backedge. The aggregate-argument exclusion is sufficient.
No raw copy interval is recorded at this root or use; that proves neither
no execution nor no machine cost.

Span's retained type, destructor and pinned native-layout records give Copy
shape, five usize fields, width 40, alignment 8, offsets 0/8/16/24/32 and no
custom destructor/native cleanup. IR Struct also represents source tuples;
the name and shape do not independently establish nominal source origin.
No positive eligibility decision relies on that unresolved fact.

The remaining twelve flat-shaped locals start with projected reads, assignments
or intrinsic results rather than fresh Struct instructions:

- [fn5719](compiler-copy-sites-census-fn5719-data/README.md) has two chains from
  projected storage through Assign and Read into whole-Span call arguments.
  The retained values cross branches and joins, then are dropped before their
  loop backedges. The two raw rows account for 10 samples.
- [fn7710](compiler-copy-sites-census-fn7710.md) has projected reads, Assign,
  Read and Replace results, with no Struct instruction or branch in its
  29-instruction body. The saved old Span crosses replacements and a call;
  forwarding a later read to the original storage would not preserve that
  snapshot. Its two raw rows account for 11 samples.

Copy shape does not remove IR consumption: Assign and call arguments retire
their inputs, while Read copy preserves its source. The reviews use complete
definitions and uses; they neither invent per-instruction loan state nor assume
callee irrelevance from a signature.

## Execution, retention and next step

Each row below is a distinct, once-only offline phase. None is a rerun of the
fixed compiler workload.

| Phase | Wall / user / system s | Peak RSS KiB | Load before → after |
| --- | ---: | ---: | --- |
| Complete-body extraction | 26.43 / 25.84 / 0.05 | 45,412 | 3.02/3.82/3.38 → 2.89/3.72/3.36 |
| Independent extraction audit | 9.86 / 9.58 / 0.05 | 103,988 | 2.04/2.68/2.99 → 1.88/2.62/2.97 |
| All-row join | 3.52 / 3.33 / 0.10 | 290,780 | 1.16/2.04/2.61 → 1.15/2.02/2.61 |
| Complete-family inventory | 0.38 / 0.36 / 0.01 | 89,660 | 1.32/1.35/1.70 → unchanged |

All ran under the 4 GiB / 300 s / 10 s grace limits, with one worker and exit
zero. Only the join has nonempty stderr. The original full extraction
345-record manifest is 63d59716e15c302d241d9e19ba792a0d715b0a0e25aeda75827b24be0799d480.
The join and family output manifests are respectively
b69c1d21e9ac89fde114dda7a0abee592bc16698c94cb902a4de669193ddff31 and
7c14fc93ff46c4ad19ba9d20625d9572e25a599e73066d0b826df0a925c74e9a.

Root compared every archived file to live bytes before retention:

| Archive | Files / logical B | Compressed B | SHA-256 |
| --- | ---: | ---: | --- |
| [Extraction and captures](compiler-copy-sites-census-extraction-data/raw-extraction.tar.gz) | 356 / 59,163,875 | 10,260,424 | babfaf4cbcaebb16fb146e142d96b98890907bb47a6f0abe576df8bcb9d1ab46 |
| [Join, including full warnings](compiler-copy-sites-census-join-data/evidence.tar.gz) | 125 / 13,410,540 | 188,053 | 2656c2abff9149705e482d78bef8f3aec4c21781bc8114ffe77b4f35841bb29a |
| [Family census and exact contexts](compiler-copy-sites-census-family-data/raw-family-census.tar.gz) | 109 / 594,062 | 83,235 | 6f354d1d4ce488343cdc99a5f9a070c6ff6309175416e015e128315dadfb1cee |

Independent extraction proof and fn5719 review are retained as exact complete
file sets; fn7710 includes the exact full body and root review. Original
failures and warnings remain intact. These are historical coordinate proofs,
not current-store admission or source-freshness assertions.

The initial aggregate pass is not implemented: this fixed population supplied
no eligible family. Source review identifies a separate missing Bool/Int
simplifier after the admitted inline64 stage. It must preserve scalar
consumption, custom trap-message identities and the last admitted fallback,
and be independently rechecked against interpretation of the original IR.
Its coverage and gain are still unmeasured. The required ten-percent retention
threshold and the Rust-O milestones are unchanged.

No production or specimen .lkj lines changed in this census. The existing
source/seed gate remains applicable; [CI43884934240](https://github.com/lkjsxc/lkjscript/actions/runs/43884934240)
at 964be9a4 passed both the native gate and optional witness.
