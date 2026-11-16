# Prospective whole-body review of sampled copies

This freezes the review population and rules before the fixed-witness copy-site
cohort is opened. It is a separate, later offline analysis, conditional on that
cohort's complete success and independent retention acceptance. It authorizes
no compiler execution, optimization pass, additional sample, or extra detail page.
A later release must bind the extractor, exact accepted profile/companion/detail
bytes and output directory. No performance or eligibility result is asserted here.

## Population and bounds

Select the first **20 distinct selected-function indices** appearing in the sole
recorded 200-row detail page, in first-appearance order. A parameter-sentinel row
selects its complete function too. Use all returned functions if there are fewer
than 20. Do not substitute another function for a blocked or oversized one.
Numeric coordinates identify this exact selected companion, not source identity
across builds or an inline call chain.

| Bound per selected function | Limit |
| --- | ---: |
| Body instructions | 65,536 |
| Exact serialized fn/endfn block | 16 MiB |
| Distinct transitively referenced types | 16,384 |
| Direct call and relevant destructor signature records | 2,048 |
| Complete body plus required context material | 32 MiB |

These are explicit review bounds, not compiler defaults or language limits.
They allow large known compiler bodies; they do not promise that every selected
body fits. A missing fact, unsupported grammar or exceeded bound yields
**unassessed**, with the actual count or missing dependency. Never classify a
truncated body as eligible. One offline structural scan of the companion may
build offsets and indexes; this is not an authenticated-store query latency test.
Use one worker, an external 4 GiB address-space limit and a 300 s watchdog with
10 s grace for a released extraction. Check 3 GiB free space first and retain
raw argv/cwd/status/time/load captures. The read-only extractor may be validated
before release using only the already retained small role companion and detail.

## Complete extraction and facts

Preserve every selected function's exact fn/endfn block, declaration-order
function index, return-source/return-leaf records, all locals and all body
instructions. Instruction indices exclude local and framing records. Parse
quoted words and escapes with the pinned ir_text grammar. Preserve instruction
source coordinates. Review fragments retain original indices and are not a
newly admitted standalone program.

For each complete body, retain its result/local types and their full referenced
field, variant, element and pointee closure, with a visited set for cycles.
Retain every direct callee's full signature and returned-view contract. The
complete destructor declaration table and source records are shared exact-byte
records. Do not read the source paths named by those records. Any eligibility
decision depending on a destructor's behavior requires its complete body too;
an absent or bounded-out dependency remains unassessed.

The companion has no serialized per-instruction loan state or native offset
table. Do not present either as recovered data. For a supported candidate,
derive layout with the pinned native layout rules, including field alignment,
maximum alignment and final padding of max(extent,1), and cross-check any sampled
width validated by the official reader. An aggregate width is not generally the
sum of field widths. Missing required ownership/layout facts block a verdict.

## Initial eligibility rules

The unit is a fresh struct value and its closed family of same-body whole-value
copies. Every member and every use must be accounted for in the complete body.
The first subset requires:

- A nominal Copy struct with only Unit, Bool, Char, supported integer or F64
  fields, at least one nonzero field, no custom destructor and no native cleanup.
  Nested aggregates, arrays, enums, views/references, owning containers and
  capabilities are outside this subset.
- A Struct instruction defining a nonparameter local, with one whole definition
  and no earlier use. Incoming storage, call results and heap reads are not fresh
  roots for this subset.
- Whole-local Read copy/move edges may introduce fresh exact-type family members.
  Follow those as value transfers, not storage aliases. Permit direct scalar
  field reads; initially block partial field moves, writes, replacement and
  reinitialization. Drop/DropIf requires complete initialization facts and proven
  cleanup-free behavior.
- The whole family lifetime lies in one proven straight-line region. No live
  member crosses a join, loop/backedge, break/continue, early return or trap.
  Last textual use alone does not establish this property inside a loop.
- No borrow, slice/range view, indirect/indexed use, enclosing aggregate/container
  insertion, aggregate argument or aggregate return. Calls/intrinsics during a
  live family are barriers unless their irrelevance is explicitly established.

Constructors, assignments, calls and intrinsics consume operand locals even
when their types are Copy. A future transformation must preserve field snapshots
at construction, source order and copy/move initialization effects. Redirecting
later field reads to the original input locals is unsound if those slots change.
This review records obligations; it does not implement forwarding or a rewrite.

Classify each investigated family as eligible for this exact subset, blocked
with all applicable rules, or unassessed. Record defining instructions, locals,
complete uses, type/layout/cleanup provenance and relevant copy-site intervals.
Copies whose actual kind is outside existing KIND_COPY attribution remain out of
scope. Deduplicate intervals before any count; keep unreviewed functions/sites,
unmapped counts and the possible unlocated in-flight detail sample explicit.
No samples-times-width product, removed-time prediction or speedup follows from
an eligible family. A bounded population does not establish a global zero.

## Future transformation boundary

Any later proposal is untrusted IR-to-IR work and must be separately rechecked,
with original-IR interpretation retained as its behavioral oracle. The current
Selection API stores one optional optimized candidate; a second pass after
inlining must explicitly preserve the last admitted fallback or deliberately
specify a combined-candidate fallback. This preparation makes neither change.

The grammar and rules are grounded in src/ir_text.lkj, src/ir.lkj,
src/native/layout.lkj, src/check.lkj, src/optimizer.lkj, src/driver.lkj,
src/reuse.lkj and docs/ir.md at source d461c2d9. Their exact identities belong in
the later extraction/review release. The producer and semantic meaning still
come from the accepted selected companion, not a source-file approximation.
