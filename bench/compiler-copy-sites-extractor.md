# Complete-body extractor for the frozen copy census

This is an offline extraction tool for the [prospective census](compiler-copy-sites-census-plan.md).
It does not execute the program, admit IR, infer layout or loans, or decide
scalar-replacement eligibility. The exact successful official detail/companion
pair and independent retention acceptance are prerequisites for future use.
A separate release must bind those inputs and the absent output directory.

## Fixed implementation

[compiler-copy-sites-extract.pl](compiler-copy-sites-extract.pl) is 31,883 B,
SHA-256 `9ca679c6f24cfe1ad9fcfd14a54f27be0edef56a1c9a0faeb7c455ab76536517`.
It implements plan `8a1f3e3d5354b3eca7fd9adb68bfee892f94360130a19b265f3f9ffb73c4dc15`
against `src/ir_text.lkj` at source d461c2d9, SHA-256
`af4c78b51063bce8a24e2a35bd364a48cbb3a888d01c1fd6219faa3d5983d4f8`.

The first twenty distinct functions in the sole first detail page are selected
in first-appearance order. Exact fn/endfn blocks, instruction ranges and original
indices are retained. Result/local type references, recursive type members,
direct callee signatures and relevant destructor signatures extend to a visited-set
fixed point. Shared source and destructor declarations are exact byte records.
Named source paths are never opened. Destructor bodies are not extracted; a
decision needing their behavior remains unassessed until separately supplied.

Root and an independent static review found no completeness blocker for that
scope. Constructor, assignment, call and intrinsic operands remain in the full
body bytes; their consumption and snapshot obligations still belong to the later
census. The tool makes no liveness or eligibility claim.

## Bounds and refusal

The plan's per-function bounds are 65,536 instructions, a 16 MiB body,
16,384 types, 2,048 direct/destructor signatures and 32 MiB of complete context.
Oversized or unsupported functions are marked unassessed without partial body
or context export. The extractor also has an explicit 32 MiB single structural
line guard. Root accepts this additional scan guard prospectively: even an
unrelated oversized line can make extraction unassessed.

These are export bounds. Type/signature/context limits are checked after the
complete closure traversal; the full companion snapshot and shared tables also
precede per-function verdicts. They do not bound all intermediate allocation.
The separate release must enforce one worker, 4 GiB address space, 300 s and
10 s grace, with 3 GiB free space before extraction. A resource termination is
an unassessed result, never evidence of a complete body.

## Retained validation

Only the existing 59,965 B role companion and its 13,319 B detail page were used.
The successful final fixture contains six distinct functions and 58 zero-sample
rows: exit 0, 0.06 s, peak RSS 12,016 KiB, load 2.33/3.36/2.72 unchanged.
The original input hashes remain unchanged. Initial syntax and buffered-read/
seek failures and their candidate versions remain in the archive. The first
oversized gateway write changed no file. No compiler, profiler or official
reader was rerun for extractor validation.

The small fixture does not exercise the twenty-function cutoff, large bounds,
continuations or every quoted escape. Independent bounded reads found the
retained body, enum and returned-view signature byte ranges consistent; root
then verified all 104 archived files against their complete inventory and live
originals. This is retention verification, not new extraction validation.

The [preparation archive](compiler-copy-sites-extractor-data/candidate.tar.gz)
is 129,300 B, SHA-256
`39e748fc1364a2ab1a3120ff8f0372d2c0885faf083e8b21e5fc77e6a62f76ad`.
It contains 104 regular files, 460,223 logical bytes and 25 directories,
including all earlier candidates and raw validation captures. Its sorted tar
metadata is normalized and gzip has no timestamp. The complete inventory and
[root verification](compiler-copy-sites-extractor-data/root-retention.json)
are retained beside it. Packaging took 0.01 s at load 1.49/2.44/2.65 unchanged;
it did not execute the extractor or open future census inputs.
