# Prospective LKJPROF4 accounting helper

This untracked proposal implements the offline phase specified in
compiler-profile-copy-sites-measurement-plan.md, reviewed draft SHA-256
6bd73c58ce10a36cdc5fed7ba4fd5b1bf96f83e6550e7a572852b2884b955e37.
The prospective cohort has not run. After static preparation, a separately
root-authorized fixture check read the retained small role profile and a
synthetic counter-state copy, as recorded below. No new sampled program ran.
The helper is not an admission checker or an execution attestation.

## Exact interface

    perl HELPER --profile SAMPLE --companion I --summary SUMMARY --detail DETAIL --output DIR

Every option is required exactly once. DIR must be absent and is exclusively
created at entry; failure leaves that claim and any partial output visible.
The launcher invokes this helper once only after the successful official
default and bounded detail reports, with their capture-complete/exit records
kept separately. The copied helper digest belongs in the prelaunch release.
Use the same external 4 GiB address-space, 900 s deadline, 10 s grace envelope.

The current fixed protocol uses the first detail page only, no cursor, limit
200 and max reply bytes 1048576. This helper validates that exact policy.
It verifies returned byte/row ceilings and the exact first-page prefix, and
preserves its continuation. The actual requested limits are proved by the
launcher's separately retained argv; this helper does not attest execution
or reconstruct the next omitted type-name string to prove maximal page filling. It does not fetch another page.

## Input and format boundary

The implementation was read from current main src/native/profiling.lkj,
SHA-256 59a6bdc97617130af9b7a0fbd3964952b31a48f2aa11dfac73416a0dad22fbe5,
with the finite tag matrix in src/x86.lkj. The later gated production/source
pins still have to be supplied and checked before measurement.

All inputs must open as regular files without following the final path
component. The profile is read into owned bytes; the companion is streamed
only for byte length and SHA-256. Neither matching metadata nor a digest
asserts filesystem freshness or kernel sealing.

The decoder validates:

- LKJPROF4, 160-byte header, exact 1 ms requested period, reserved zeros;
- all contiguous 64-byte symbol rows/names and code ranges, valid UTF-8,
  per-symbol count bounds and checked function-self sum;
- all ordered 16-byte kind intervals, 16 counters, canonical section offsets;
- every 72-byte site, address order/nonoverlap, whole function/kind containment,
  finite origin/role/flag matrix and synthetic-parameter sentinel;
- the full 16-byte selected-function map, unique emitted symbols, explicit
  all-ones unemitted sentinel, and exact EOF;
- exact selected-companion digest and full raw-profile digest;
- S-T in {0,1}, D-K in {0,1}, with D-K=1 only when S-T=1, and all u64 sum
  overflow refusals. No convenient row is decremented.

The native u64 words are exact only on the explicitly required 64-bit Perl.
All field offsets are bounded by signed file length before multiplication or
indexing. Counter aggregates use Math::BigInt, including an unbounded
overlapping inclusive sum. Output observations are decimal strings, not JSON
floating values. The separate arithmetic probes check these host assumptions
without reading profile/IR bytes or invoking a compiler.

The helper cross-checks all ordinary report integer counts and the exact
top-function selection/order, while treating its displayed f64 percentages as
syntax/range-checked display values only. It cross-checks the detail header,
scope, accounting, every visible row's raw numeric/role fields and function
symbol name, then proves the page is the exact deterministic first prefix.
That proof scans C rows and does not materialize/sort C copies. The official
detail command alone independently parses/checks IR and validates all operand
roles, type identities, layout widths, selected names and function count. This
helper deliberately does not recreate or replace those semantic checks;
stdout's checked-ir label is not by itself proof that a command ran.

## Outputs

Successful completion produces exactly these eight files:

- accounting.json: input hashes/lengths, complete counters, section offsets/
  lengths, C/F, precise official-output correspondence, scope limitations;
- kinds.tsv: all 16 exact integer marginals, with interval counts;
- functions.tsv: every symbol including zeros, self/inclusive/last-seen serial,
  ranges/frame, selected slot or none, and JSON-escaped name;
- function-map.tsv: every selected slot and explicit unemitted sentinel;
- widths.tsv: exact observed widths, site/nonzero-site counts and sample sums;
- roles.tsv: origin/source/destination/place-flag buckets;
- width-roles.tsv: joint exact width plus those role fields;
- manifest.sha256: hashes of the other seven files.

Every bucket file ends with an explicit unmapped row. Its mapped-row sum equals
the complete raw mapped table; including unmapped sums to D, which may be K+1
for the one explicitly unlocated in-flight detail sample. No sample-by-width
product, byte traffic, removable time, SROA eligibility or causal effect is
computed. Zero-count sites remain in bucket site totals and in the retained
raw table. Inclusive function counts overlap and are not total CPU samples.

The 72*C + 16*F + 80-byte increment is a calculation against an otherwise equal
LKJPROF3 schema, with zero table alignment padding, not an observed old/new
producer delta. Both complete raw LKJPROF4 and selected IR remain separately
retained according to the launcher protocol.

## Preparation and fixture validation

The initial syntax check of draft6d8d619a succeeded but warned about a >32-bit
hex literal. Draft764923d2 constructs that exact mask from two LE u32 words;
it passes clean syntax and the original bounded arithmetic checks. Both drafts
remain as review-v1.pl and review-v2.pl.

Root then authorized one bounded fixture validation, distinct from the future
cohort. validate-fixture.sh b296703f runs only on owned copies of:

- retained zero-sample role profile, 28,522 B,
  SHA-256 1f0912222954ccfc998e2ee27f8164c0e631381701b6ae1b04d1ae43907a27a1;
- retained companion, 59,965 B,
  SHA-256 b323c6407a905ef3f7097406bd4659f728b9c252b27a70452789da7233b0154b;
- gated canonical reader, 32,584,542 B,
  SHA-256 6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc.

The fixture has C=58, F=39, 69 symbols and 1,092 kind intervals. Its only
additional admitted state changes counters, not code/map/IR: S=10, T=9,
K=5, mapped=6, unmapped=0, D=6, function self=5, unattributed=4, and one
explicitly unlocated in-flight detail count. These synthetic integers are
not observations from execution.

validation-1 preserves four official reads (default/detail for both states),
two helper acceptances and six expected helper refusals: reserved header,
u64 counter overflow, impossible copy offset, wrong companion bytes, false
detail mapped count, and false ordinary kind count. Every command used
4 GiB/60 s/10 s grace with argv/cwd/time/load/status/stdout/stderr captured.
Official reads took 0.00/0.01 s and helper reads/refusals 0.02–0.03 s, with
load 2.58/1.69/2.01 unchanged throughout. No sampled program ran.

Offline inspection then exposed an output-schema defect: Math::BigInt::bstr
retained numeric flags for some small aggregates, so accounting.json encoded
numbers despite its string-only declaration. The integer values and all six
TSVs were correct; the first validation's status=0 is therefore narrower
than full schema acceptance. Its bytes and original helper are preserved.

Final helper SHA-256
0f938e92e961d94581f4424804f798c67cb168269af3de0734f4e7703d8fcb57
wraps each BigInt result at the JSON boundary in the same fresh decimal-string
function already used for native words. The extended arithmetic-contract-v2
also checks BigInt values 0, 1, 9, u64::MAX and u64::MAX+1 serialize as strings.

revalidate-helper.sh 55e0d18b performs no official decode. validation-2 reuses
the already captured profile/companion/official outputs for two acceptances
and the same six refusals. Both positive JSON outputs now pass a recursive
scalar-type assertion, and all six TSV files per state remain byte-identical
to validation-1. Refusals return exactly E_COPY_ACCOUNT with no derived files;
the exclusively claimed empty directories remain. Every prior validation file
is rehashed unchanged.

The two accepted helper reads each took 0.03 s with
load 2.39/1.93/2.05 unchanged. The six refusals took 0.02–0.03 s with
individual load pairs retained, spanning 2.39/1.93/2.05 to 2.36/1.94/2.05.
These tiny validation costs are not cohort performance evidence.

Remaining limits: this fixture exercises a full 58-row first page, not a
nonterminal helper continuation; it does not independently exercise every
malformed UTF-8/tag/map condition. The official implementation's gate has
separate parser/companion tests. No future measurement input was processed,
no compiler was built, and there was no new observer/race/kill experiment.
The later cohort still requires its exact gated pins, committed bundle and
explicit release. All proposal/validation files remain untracked under target.
