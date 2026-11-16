# Independent acceptance of the original fingerprint goldens

The captured baseline is accepted for the declared seventeen cases. Both captured commands completed successfully, the complete dump parses without omitted or extra records, and all ten outcome/value relations pass. This acceptance does not install the goldens or validate a candidate fingerprint implementation.

The original capture is target/compiler-fingerprint-index-golden-1, launched by root1605 and completed at final1606. The fixed controls are committed at 042bce952af30959b1ecdff2fc686033f789c497. This audit used only Perl/Git readers and the retained bytes. It did not execute capture.pl, the compiler, the native image, or a reconstruction of the fingerprint algorithm; it did not decompress, archive, delete or modify the original root.

## Source and image correspondence

All 45 preparation records match their exact committed Git blobs and the preparation manifest. The source population contains 1,743 original tracked files from source commit 9326dbaab1bf6832b9f35dc156e12d0d62eccd9e, with exactly two declared overlays: src/main.lkj is replaced, and src/fingerprint_fixture.lkj is added. Every one of the resulting 1,744 regular files, totaling 9,390,696 bytes, was compared in full with its Git blob or committed overlay, including modes and the complete path set. The file-by-file proof is results-v2/source-audit.tsv.

The fixture directly calls elaborate::incremental::fingerprint. Its expected() remains the pending-capture panic. The original src/elaborate/incremental.lkj is byte-identical to Git at SHA-256 760485a5a12d97ce320ba9113797370df584c154084ee4a1f9689da28c7d1127. The explicit runtime source remains eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14.

All four before/after source manifests match that independently checked live population. All four producer records match the actual retained compiler bytes; both dump-side image records match the actual image and the successful build record.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Executing baseline compiler | 32,588,975 | b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db |
| Built golden image | 1,568,827 | 99a179be5fd5f6cd9c8e7d3e4466468ff4b519a406303e77e935c1d27dcade89 |
| Complete raw dump | 4,531 | 346340880cf8b9bd247c0caf30f75ed534a702c8b0257dc81fcea2cb5849e4cd |

The image has the temporary main/fixture overlay and an explicit runtime selector. No equality with the canonical compiler image is claimed.

## Captured command and timing checks

The build uses the owned baseline compiler, source entry src/main.lkj, the exact source grant, cwd-relative physical library and runtime, --inline-limit 0, --no-store and --stats. The dump executes only the resulting owned image. Exact argv, wrapper argv, cwd, environment, stdin, raw wait/exit/signal fields, GNU time, UTC, integer nanoseconds and load are retained in results-v2/raw and summarized in phase-audit.json. Each command has a 4 GiB address-space limit, 900-second timeout and 10-second TERM grace; their intervals do not overlap.

| Phase | Exit | GNU wall / user / system (s) | GNU peak RSS (KiB) | Launch-inclusive wrapper | Load before → after (1/5/15 min) |
| --- | ---: | --- | ---: | ---: | --- |
| Build | 0 | 13.77 / 13.36 / 0.13 | 470,112 | 13.779799846 s | 1.55/3.20/2.89 → 1.58/3.13/2.87 |
| Dump | 0 | 0.00 / 0.00 / 0.00 | 3,692 | 6.771441 ms | 1.58/3.13/2.87 → 1.58/3.13/2.87 |

GNU time rounds to centiseconds; the dump's 0.00 is not a zero-runtime claim. The wrapper includes process/resource launch and capture overhead. Full input verification is outside each timed interval.

Build stderr contains exactly one completed compiler stats row and no diagnostic; build stdout and dump stderr are empty. The build, local/proof, emitted, frontend and test reuse fields, and all inline counters, are numeric zero. emitted_bytes equals the actual image length. No store_writes counter is emitted under --no-store; the audit records that absence instead of claiming a physical-I/O count.

## Complete golden relations

The independently parsed wire retains every case label, shape, body slot, empty slot and terminator. Fingerprint values are compared separately from the case-name wrapper.

| Cases | Verified relation |
| --- | --- |
| 0–16 | Exactly 17 records: 11 Some; None only at 4, 5, 7, 8, 9 and 10. Header, ordinal order, footer and final newline are complete. |
| 2 vs 0 | Identical shape and complete six-slot body vector. Labels intentionally differ. |
| 3 vs 0 | Identical shape; only alpha::left body slot 5 changes. |
| 1 vs 0 | Complete body vector is reversed, including its empty slot; shape differs. This is a fingerprint-coordinate fixture, not an admitted reordered-AST claim. |
| 6 vs 0 | Shape differs. The ignored first function makes slot 1 empty; the existing bodyless slot 0 remains empty. Slots 2–5 match. |
| 11 and 12 vs 0 | Each shape differs; each complete six-slot body vector matches. |
| 14 vs 13 | Shape and child body slot 1 differ; root body slot 0 matches. |
| 15 and 16 vs 13 | Each shape differs; each complete two-slot body vector matches. |

results-v2/parsed-goldens.json is derived solely by parsing the complete raw dump. It equals every recorded golden value and exact_record string in the capture's goldens.json. results-v2/relations.json contains the ten independent relation checks. The recorded outcome agrees with the two successful phases and these exact populations.

## Retained audit-only correction

The first independent audit exited 255 after passing source/Git and binary verification. Its assertion required byte equality between the root release and capture.pl's reserialized execution copy. That was an auditor mistake: the capture helper intentionally writes canonical pretty JSON. No workload or original capture failed, and no compiler/native command was repeated.

Both exact release files remain separately pinned:

| Release record | Bytes | SHA-256 |
| --- | ---: | --- |
| Original root release | 1,046 | 9d40835d790473ab8771f47ee96ae2a33c722add32a57493305a9276277b3dba |
| Execution's canonical JSON copy | 1,064 | 46a2235397dadb497280dc8491fcbcb2c16c2ac81608de78b45d653155fc44a2 |

audit-v2.pl checks complete decoded-field equality and the exact declared serialization, while retaining both byte identities. The first script, partial source audit, raw exit and gateway capture remain untouched. The correction diff and the separate successful v2 capture are included.

The audit-only first run took 0.42 s GNU wall at load 1.29/1.75/2.28 → 1.27/1.73/2.28. The corrected audit took 0.51 s at 2.44/1.96/2.33 → 2.44/1.96/2.33, exit 0. These are validation costs, not compiler-performance observations.

The accepted machine-readable summary is results-v2/summary.json. The bundle's manifest binds the scripts, both audit captures, the exact closed execution records, full source correspondence and independently parsed values. No expected() installation or production-source edit is part of this acceptance.
