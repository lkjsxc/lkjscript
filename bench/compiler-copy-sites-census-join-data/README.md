# Completed offline copy-row join

This is a read-only join of accepted cohort 2, not a new sampled workload.
The independent Perl reader ran exactly once. It requested no extra detail page
and ran no compiler, source program, official profile reader or accounting helper.
All 20 frozen selected functions remain unchanged.

## Result

The complete raw profile contains 68,092 copy rows and 1,503 mapped copy samples.
The sole 200-row page contains 825 samples; it is not the full census.

| Partition | Rows | Raw copy samples |
| --- | ---: | ---: |
| Selected functions, displayed | 70 | 430 |
| Selected functions, unshown | 188 | 39 |
| Selected functions, total | 258 | 469 |
| Outside population, displayed | 130 | 395 |
| Outside population, unshown | 67,704 | 639 |
| Outside population, total | 67,834 | 1,034 |
| All rows | 68,092 | 1,503 |

There are 152 zero-sample rows among the selected 258 rows. They are retained.
The global unmapped counter is zero. Serial and kind total are both 14,658;
serial-minus-kind and detail-minus-copy are both zero. No row was decremented or
used to assign an unlocated interrupted sample.

All 345 files in the accepted extraction manifest verified. Every selected raw
coordinate, original constructor/parameter subsite, operand payload type, role/
flags and width matched the complete extracted body and pinned native layout.
Duplicate/overlapping intervals would have refused; none did. Each selected raw
row appears exactly once, preserving different intervals at the same instruction.

## Payload type screen, not family eligibility

The selected rows use 78 distinct payload types. Exactly one meets the flat Copy
IR-struct shape predicate: type 41, named diagnostic::Span in the companion.
Its five type-4 fields (file, start, end, line, column) have offsets
0, 8, 16, 24 and 32; storage is 40 bytes, alignment 8, with no custom destructor
or native cleanup.

Its four rows account for 21 samples:

| Original raw row | Function | Instruction | Samples | Displayed rank |
| --- | ---: | ---: | ---: | --- |
| 32811 | 5719 | 105 | 9 | 13 |
| 32818 | 5719 | 310 | 1 | unshown |
| 40165 | 7710 | 0 | 1 | unshown |
| 40166 | 7710 | 3 | 10 | 11 |

All four are Read contracts. This does not establish a fresh aggregate root,
closed family, nonescape, safe control flow, complete use classification or any
optimization gain. IR Struct also represents tuples; the screen records nominal
origin as unestablished by shape. A scalar constructor-field payload can belong
to a relevant containing aggregate, so rejection of its payload type is not
rejection of that family. Every row retains the exact instruction (or original
parameter-local record), record kind and complete containing-body hash for the
separately assigned family review. No whole-program zero is inferred.

## Once-only execution and warnings

- Reader SHA-256:
  d3583950adc72bbc895254727ed752dd5454c81618b23326395207c693f1e3e1.
- Wrapper SHA-256:
  191245e518d5705279a5114521c9248cfafd2a313019eae530c32737d50bda87.
- Prospective plan SHA-256:
  49c6b4a6916c77d4822298038e8256bda4fe8a17eeebe8cf8085e0508b54f882.
- Gateway launch: compiler-copy-tests-join-once-1249;
  completion: compiler-copy-tests-join-wait-1250.
- Status 0; GNU elapsed 3.52 s, user 3.33 s, system 0.10 s;
  peak RSS 290,780 KiB.
- Load 1.16 / 2.04 / 2.61 to 1.15 / 2.02 / 2.61.
- External limits: 4 GiB address space, 300 s, 10 s grace.

The reader emitted 68,550 identical Perl warnings about the tag mask's
hexadecimal value exceeding 0xffffffff. This is the 35-bit finite mask evaluated
under an explicitly required 64-bit Perl build; no count or descriptor mismatch
was reported. The entire 12,887,400-byte stderr is preserved, SHA-256
7ddb808824da96834af92393b1ce7fd82f37f7226f641b6b1240e55a3b6d436a.
The count equals 68,092 all-row checks + 200 displayed checks + 258 selected
checks. No warning suppression, source repair, output repair or data retry
followed. Execution time includes those writes and is not a performance result.

A later inspection command accidentally printed the repeated stderr to its
gateway capture; that inspection capture was truncated, but phase-1/stderr is
complete and unchanged. It was not a second data invocation.

## Proof locations

phase-1 contains the exact copied reader, argv, command, cwd, limits, status,
time, load, stdout and stderr. phase-1/output contains the full selected-row join,
per-function partitions, all type screens and fields, exact payload type records,
accepted detail/selection, full destructor declarations, extraction verification
inventory, accepted-measurement proof and pinned layout source. The .lkj layout
file is an exact captured input, not newly authored production code.

Output manifest SHA-256:
b69c1d21e9ac89fde114dda7a0abee592bc16698c94cb902a4de669193ddff31.

Important output identities:

- result.json / identical phase-1/stdout:
  0a3492d04958ec7c88474821e0ac0f6f26c7647dbc06d0271517383443465e5f.
- selected-copy-rows.tsv:
  cc15c9d93120dc5533c27ee1b8febf31d0182de0060fdb391083189f748b69be.
- function-totals.tsv:
  4a107a52bb75d592b01dfc22f7ae0da3593c08ce50166568b56f2d801d8b347e.
- payload-types.tsv:
  b0092a23cc0d861e8aa553b867b42e1bad5bf7aaa2e7307a02899bcd052e04e6.
- payload-type-fields.tsv:
  f129ef4f3e16e91e9c63cb6ca760247efbb56dc478358b0ba9f65db18fb26709.

Primary profile/companion/detail/manifest hashes remained unchanged after the
join. Original prospective versions and syntax checks are retained. There were
no runtime refusals or retries. Whole-body eligibility remains unassessed.
