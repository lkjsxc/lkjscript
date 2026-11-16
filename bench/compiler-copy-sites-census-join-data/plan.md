# Once-only offline raw-copy join

Root released this read-only join after accepted cohort 2 and its independent
audit. No compiler, sampled program, official reader, accounting helper or extra
detail page is run. The 20 selected function indices remain, in first-appearance
order: 4545,4546,4452,3101,5729,7492,4631,4445,7710,752,5719,2234,2615,2652,6179,6244,7372,690,1417,9782.

The frozen reader is join.pl SHA-256
d3583950adc72bbc895254727ed752dd5454c81618b23326395207c693f1e3e1.
The single invocation uses run-once.sh, which exclusively claims phase-1, copies
and verifies the reader, records argv/cwd/resource limits/load/time/status and
captures stdout/stderr. A refusal or timeout is retained and stops the run.
No retry or population substitution is implied. Limits are 4 GiB address space,
300 seconds and 10 seconds grace. The only data invocation is this offline reader.

## Accepted inputs

- Raw profile:
  /home/coder/workspace/lkjscript/target/compiler-copy-sites-run.20261116-2/producer-profile64.samples.00000000000002892325
  SHA 7849d4dd69906491d07664f493eb0f4fd8340cfde3760f6e4f168dbbde95f302.
- Extraction:
  /home/coder/workspace/lkjscript/target/compiler-copy-sites-census-output.20261116-1
  manifest SHA 63d59716e15c302d241d9e19ba792a0d715b0a0e25aeda75827b24be0799d480.
- Companion SHA 32c0f43dbde125e43f2ca5a0c6d8548ba0f1774ce3b51bc62d94a6488af0fdca.
- Detail SHA 06093752531ea752f7081548166f33cc95c71aad381e72d2713e35f5bcaab25d.
- Selection SHA 3dd145c8920a3169b322bf6fbee0e637ceb4e8989980229260b217c04e38d80c.
- Main 964be9a48491cbdee64b93fa27e132084077b688 acceptance:
  bench/compiler-profile-copy-sites-cohort-2-acceptance/acceptance.json,
  SHA adfb9a9837ec044cc1a03cec186a7ada643300bab7efe79b324df6e597bddcbe.
- Native layout source: src/native/layout.lkj,
  SHA f88d1ff9a50ce796d2fc10bfa04cd4e8d73a30d8284d5a9b3bcd3488f724daf0.

Every extraction-manifest entry is verified before use; selected complete bodies
and type ranges are checked against the accepted full companion. The reader
verifies the LKJPROF4 binary framing/counters, full sorted 200-row prefix, original
row/interval uniqueness, selected operand/parameter/subsite/type/role contracts,
and all selected widths against the pinned native layout. Official IR admission
remains the accepted prior command's evidence.

## Output and limits of interpretation

All selected raw rows are retained, including zero counts and unshown sites.
Rows retain original table index, row digest, interval, full numeric coordinates,
type, width, tags, origin, roles/flags, mapped symbol, samples and displayed rank.
Each also retains its exact instruction text or original parameter-local record,
the record kind and complete containing-body hash. Different intervals at one
IR instruction are not collapsed.

Per-function displayed/unshown/total counts and global outside-population,
unmapped and interrupted-handler totals are explicit. No possible in-flight
sample is assigned to an arbitrary row. The sole 200-row page is not a full
copy census.

The payload screen applies the frozen scalar-field shape rule: Copy IR Struct,
only Unit/Bool/Char/8/16/32/64-bit Int/F64 fields, at least one nonzero field, no
custom destructor and no native cleanup. Native padding/alignment uses
max(extent,1), maximum alignment and final padding. Passing output is labelled
flat-copy-ir-struct-shape-only: frontend tuples also lower to IR Struct, so
nominal origin remains unestablished by shape.

This is not a containing-family screen. A scalar constructor-field payload may
belong to a flat Copy destination aggregate. Every such row remains in the join
with its complete instruction available. Fresh roots, closed families, complete
uses, escape, loans, control flow and whole-body eligibility are not assessed.
There is no sample-times-width, traffic, removable-time or gain calculation.

Output includes selected-copy-rows.tsv, function-totals.tsv, payload-types.tsv,
payload-type-fields.tsv, payload-types.json, exact payload type declarations,
complete shared destructor records, accepted detail/selection, extraction
verification inventory, pinned layout source, result.json and a full output
manifest. Exact original inputs remain unchanged.

review-v1.pl, review-v2.pl and review-v3.pl are unexecuted prospective versions.
The nominal-origin and coordinate-record qualifications were added during static
review before the only data run. Syntax-only checks are separate from execution.
