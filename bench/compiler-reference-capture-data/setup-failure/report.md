# Failed reference-capture setup: independent boundary audit

The audit accepts the **observed preparation failure**, not a successful setup. The original attempt remains unchanged: recover-inputs exited **255** with exactly `unexpected/duplicate archive file`. Its separate capture validator exited 0 because the failure's metrics were complete; that validation does not turn the failed command into a success.

## Failure boundary

The released controls are exactly manifest `edb63ca8c50e2659c12e057225f3adb7a774aa3fa9091ee277e872770b4d2d58`, committed at `a276bf3748810dbe7b4dc461ecee4ad17e6ec51b`. All five copied control files match their exact committed bytes. The launch's before/after manifest records also agree.

| Phase | Original command status | Capture validation | Recorded outcome |
|---|---:|---:|---|
| recover-inputs | 255 | 0 | failed |
| prepare-factory | unavailable | unavailable | not-run |
| generator-build | unavailable | unavailable | not-run |
| generate-100k | unavailable | unavailable | not-run |
| freeze-artifacts | unavailable | unavailable | not-run |

The original outer setup took **0.40 s**, user 0.33 s/system 0.06 s, maximum RSS **128,436 KiB**. The failing recover command took **0.32 s**, user 0.28 s/system 0.04 s, the same recorded maximum RSS. Both load captures are **2.07 / 2.24 / 2.84 → unchanged**. These are preparation-failure timings, not compiler performance observations.

The audit verified the exact Perl recovery argv, 4 GiB / 900-second / 10-second resource wrapper, actual and declared cwd, stdin, free-space record, original failure, timing, load and phase table. The only four recorded metadata subcommands are three Git blob reads and a Git source-tree listing. There is no compiler stats row.

The fixed input root contains exactly 46 files and four child directories; the outer capture root contains exactly 15 files and no child directory. All **61 original files / 50,006,352 bytes** were inventoried and checked unchanged. The compiler-input, factory, generator, generated-100k, setup-summary and artifact-manifest outputs are absent with ENOENT, as are all four later phase directories. The frozen helper validates the archive before creating the extraction destination. No source extraction, generator construction, generation or A/B build occurred in this attempt.

This conclusion uses the fixed control flow, retained commands/statuses and actual artifact population. It is not a syscall observation or a new diagnosis of the archive-path issue.

## Artifacts created before refusal and their recovery

| Original artifact | Bytes | Recovery |
|---|---:|---|
| producer-A | 32,584,542 | Exact baseline identity `6cd52308…bdacc`; recover from the committed seed gzip below |
| artifacts/baseline-A.gz | 6,547,345 | Byte-equal to `7d0731e9:seed/lkjscript.gz`, SHA `505be947…ffc3e2` |
| artifacts/compiler-source.tar | 10,741,760 | SHA `79aa24a9…71d806`; existing committed preliminary archive member |
| artifacts/scale-generator-source.lkj | 20,625 | Byte-equal to `0b0b2780:bench/scale.lkj`, SHA `557d6b12…ff1e9` |
| artifacts/shape | 68,884 | Byte-equal to the pinned `0b0b2780` shape blob, SHA `0457f8f9…072c` |

The actual producer-A bytes match the frozen complete SHA-256 and size. The audit did not decompress it again. Compiler source recovery is the member `target/compiler-reference-capture-preparation-1/source.tar` in committed `bench/compiler-reference-capture-data/preliminary/preliminary-roots.tar.gz` (7,839,018 bytes; SHA `35ced5f584174c20a884dd3feeb988c188bf4350d0276831a6c52ac15e7088bc`). Its existing committed byte-equality proof matches the actual copied tar; this audit also checked the exact committed recovery archive digest. No duplicate archive, gzip or extraction was produced.

Full hashes and recovery locations are in acceptance.json. All 56 small original control/capture files are copied byte-for-byte under raw/; the five artifacts are represented by exact inventory and committed recovery mappings, without duplicate binary/source copies.

## Separate inspection and audit controls

Root's read-only inspection `compiler-reference-capture-setup-refusal-v1509` exited 1 after printing the failure and trying the nonexistent filename `records/recover-inputs/argv`. The actual `argv.json` is present and verified. That diagnostic is retained under reads/ and is separate from the original status-255 setup refusal.

This audit ran once after syntax checking, exiting 0 in **0.57 s**, maximum RSS **106,360 KiB**, load **1.49 / 1.85 / 2.54 → unchanged**. It read saved files and exact Git recovery objects only. No original file changed; no compiler, generator, native workload, setup phase, key/store access, decompression or compression ran. There is **no performance data and no A/B execution** from this failed setup. Archive diagnosis and any revised setup require separate work and release.
