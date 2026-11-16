# Committed-source closure of reference-capture adoption

The closure audit passed for commit `bd8a9050a3786de9f227c6c4037f5acd43205f71`. All **1,743 adopted final files** match exact committed Git blobs: 185 under src, 32 under library and 1,526 under tests, totaling 9,380,526 bytes. The committed source SHA-256 inventory is byte-identical to the prior frozen adoption inventory.

The source tree is `626c9c1f234f4d003a08820d9b69840535eff8b8`. Library remains `25bdb8ff8567b941790d3951c78e896b23e26e8e`; tests remains `e2b30ac0f2fa8f897f366293189d6af552a41980`. The immediate parent's source/library/tests trees equal the original adoption baseline at `0b0b27803d27b932a5b91cde2d766bd3c8bb82b9`.

## Retained language delta

Both the immediate-parent-to-commit and adoption-baseline-to-commit Git numstats yield exactly **564 added + 21 deleted = 585 retained .lkj lines**, across the same ten audited paths. The prior 24-operation proof therefore closes at **585 / 585 credited retained lines**, with no uncovered delta.

The sole added .lkj path is `src/semantic_tests/reference_capture.lkj`. No other .lkj evidence file was added, modified or deleted in this commit. The complete changed src/library/tests path set also equals the ten audited language paths. Shell tools, documentation, JSON/text captures and archives are not language-line credit.

The committed copies of the original adoption acceptance, source inventory and manifest are byte-identical to the frozen originals. This closure leaves the original source-publication proof, projection qualifications and query-audit package unchanged.

## Programs and publication record

The complete Programs section of docs/status.md is **4,146 bytes**, SHA-256:

`235aa6c283d334688289ab73e472cae920d25b1e8cbad76cb4461f30d18cc9f4`

It is byte-identical at the adoption baseline, immediate parent and audited commit. Its exact bytes are retained as Programs-preserved.md.

The commit also preserves the first whitespace-check stop and its explanation. The raw stdout is 1,700 bytes, SHA-256 `70ba5693a68e592ddc162311870c776b844f0bac87d4de2bd4f971e42e5ed324`. That stop concerned immutable inspection/diff captures; this audit neither rewrites those captures nor treats it as a compiler/source failure.

The committed bootstrap metadata records candidate `b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db`. No live executable or seed was read; seed refresh is outside this proof.

## Execution and limits

The offline audit exited 0 in **0.12 s**, using **12,888 KiB** maximum RSS. Load was **2.58 / 2.62 / 3.07** before and after. It ran once after syntax checking, under 4 GiB AS / 300-second timeout / 10-second termination grace.

One initial metadata inspection guessed a nonexistent target source-publication directory and exited 2; its complete stdout/stderr are preserved under reads/. The actual publication records were then located in the committed bench tree. The audit itself passed on its first execution.

No compiler, query, native workload, compression, key/store access or source mutation occurred. Only this new target package was written. This is exact committed-byte closure of the existing adoption proof, not a new admission, test result or performance claim.
