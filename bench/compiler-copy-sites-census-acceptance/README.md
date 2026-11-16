# Independent verification of the complete-body extraction

Verified the exact completed extraction at /home/coder/workspace/lkjscript/target/compiler-copy-sites-census-output.20261116-1. No discrepancy was found. Eligibility remains **not assessed**.

The original 345-record manifest is 63d59716e15c302d241d9e19ba792a0d715b0a0e25aeda75827b24be0799d480. Its complete 346-file population, including the manifest itself, was unchanged before and after this verification. The owned input snapshots match the accepted held/committed originals:

- Companion: 57,649,044 bytes, SHA-256 32c0f43dbde125e43f2ca5a0c6d8548ba0f1774ce3b51bc62d94a6488af0fdca.
- Sole detail page: 53,808 bytes, SHA-256 06093752531ea752f7081548166f33cc95c71aad381e72d2713e35f5bcaab25d.

## Independent checks

The verifier independently scans the exact canonical companion and builds structural byte ranges from the original input. It does not import or invoke the extractor. It compares complete output bytes and every recorded range/digest for:

| Material | Verified count |
| --- | ---: |
| Selected full fn/endfn bodies | 20 |
| Instruction indices, operations, source coordinates and byte ranges | 6,202 |
| Complete type blocks, counting occurrences across selections | 1,882 |
| Distinct type blocks | 701 |
| Callee/destructor signature excerpts, counting occurrences | 198 |
| Distinct callee/destructor signatures | 97 |
| Exact signature component ranges | 651 |
| Shared source declarations | 140 |
| Shared destructor declarations | 202 |

Every body includes its complete original header, returned-view contracts, locals, instructions and endfn. Enum blocks include all variants and endtype. Signature excerpts include the exact header, all return-leaf records, parameter locals and endfn in source order. Shared declarations match their complete original record sets. Instruction indexing excludes local/contract/framing records. No named source path from the companion was opened.

The first 20 distinct function indices and their first-row positions were independently recomputed from the sole existing 200-row detail page and match selection.tsv exactly. The required type closure was recomputed from each function's result and all local types, following every field/variant/element/pointee reference. Direct-call and relevant destructor signatures extend the same visited-set fixed point with their result/parameter types. All resulting identities, signature reasons, context bytes and exported tables agree exactly.

All selections meet the frozen export bounds. The 20,176-byte complete shared context is counted once per function's logical context although physically stored once. Required type material contributes 142,235 bytes and signature material 21,194 bytes across the selections, counting repetition.

## Size of the complete reading material

| Material | Total bytes | Median bytes | Maximum bytes | Minimum bytes |
| --- | ---: | ---: | ---: | ---: |
| Complete bodies | 203,690 | 2,644 | 56,134 | 1,206 |
| Bodies plus complete logical context | 770,639 | 23,613 | 122,246 | 22,762 |

The median is the mean of the two central values for the fixed 20-function population. These are exact retained serialized-byte counts; they are not authenticated query timings or evidence of optimization eligibility.

## Captures and proof

The original extraction captured exit 0, empty stderr, 26.43 s wall, 25.84 s user, 0.05 s system and 45,412 KiB peak RSS; load was 3.02 / 3.82 / 3.38 -> 2.89 / 3.72 / 3.36. Its exact command, working directory, stdout, status, time, UTC and load records are copied in captures/. The command binds the frozen extractor and exact input/output paths under 4 GiB / 300 s / 10 s.

This independent verifier ran once with exit 0 and empty stderr: 9.86 s wall, 9.58 s user, 0.05 s system, 103,988 KiB peak RSS; load 2.04 / 2.68 / 2.99 -> 1.88 / 2.62 / 2.97. Its raw argv/cwd/status/time/load/UTC, syntax result and gateway completion are retained. This was offline verification, not another extraction or compiler workload.

proof.json SHA-256: d4d2990af2fcbadda0dbd8b6b9046a74e5d8f0ff77cd876414b78999f1cf90f0.
verify.pl SHA-256: 8368b6c1a0f5c261fc3e05c0438cd9b2908369748a62e47cf938475e4a98319e.
Verified selection SHA-256: 3dd145c8920a3169b322bf6fbee0e637ceb4e8989980229260b217c04e38d80c.
Before/after full output inventory SHA-256: bfbe1f854c080b2ec24010f2bfc14f2da548651a5a0a858835869a6a13c37384.

The verifier checks this admitted canonical input structurally and preserves original byte identities. It does not re-admit IR, recover loan state, calculate layout, establish cleanup behavior, or classify any family. The first page selects functions; it does not enumerate every raw site within them. No later join output or additional page was opened. No extractor/compiler/official reader/frozen helper was rerun, and no cleanup was performed.
