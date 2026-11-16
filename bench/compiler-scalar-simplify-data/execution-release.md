# Scalar simplifier: one-cohort execution release

Root releases exactly one execution of the unchanged [protocol](../compiler-scalar-simplify-plan.md) and harness after the exact candidate's focused checks and complete local gate have passed. The fixed source, order, arithmetic-mean threshold, external limits and stopping rules remain unchanged. No performance observation exists at this release.

## Candidate and trust

The isolated source commit is `5b5ce7759c035ae0437e3512e18efc440aa50d2e`, above published prerequisite `e0b21e66659dc1830a5bebddf0c1f30fd2d8fa94`. Its independently reproduced executable is 32,761,734 bytes, SHA-256 `f85624e3cc694f6a3a3e10c33f822789ae6084e2d5a5b41c3b1fc8fcc9bd2d64`.

This opt-in scalar stage follows the last admitted native selection and is independently rechecked. It preserves the original interpreter oracle and falls back to the last admitted selection on rejection. It handles the pinned Bool identities/constants and exact integer bitwise/comparison subset; it adds no arithmetic, shifts, casts, ranges, aggregate pass, loop transform or dead-producer elimination. Address-taken exclusions, control/effect invalidation and the sorted unique trap-message inventory constrain its rewrites. Ordinary output and bootstrap policy remain unchanged.

## Exact validation

| Observation | Result | Wall and load before → after |
| --- | --- | --- |
| Candidate bootstrap | Two generations agree; library check passes | 77.47 s; 1.34/1.21/1.32 → 1.69/1.33/1.36 |
| Focused CLI/cache/profile/worker forwarding | Passed | 3.35 s; 1.64/1.53/1.44 → unchanged |
| Mutation argument conflict orders | Passed; no publication | GNU 0.00 s, separate nonzero ns capture; same load |
| Fixed generator/selection/codec contracts | Passed | 0.01 s; same load |
| Empty-store complete four-worker gate | 7,476 fresh requests, zero reused | 754.526 s internal, 754.52 s GNU; 1.32/1.44/1.42 → 5.01/4.81/3.26 |

The full gate uses 2016.49 user plus 35.51 system seconds and a largest-process peak RSS of 1,258,256 KiB. All 65 named phases, 123 top-level logs and 1,943 source files were independently checked against before/after captures and exact Git blobs. Current candidate bytes still equal the captured producer.

The scalar conformance/generated routes admit 149/538 candidates, respectively, with zero scalar rejections or message mismatches. The existing inliner retains twelve conformance fallback rejections in both policies. Each conformance policy has two declared pending cases; a passed gate does not claim those ran. Generated routes contain 805 source cases and 1,215 case requests apiece.

Both fixed 128-mutant routes produce 104 rejections, 22 values, one trap and one nontermination, with zero findings and zero resource outcomes. The scalar route actually attempts, admits and executes four branch rewrites in four bodies. Mutation remains at inline0. The semantic host passes 36 native-observation contracts, including the new original-IR comparison groups.

Focused evidence and full-gate evidence remain separate, directly readable files. The gate audit's nonfatal parser warning and a report-only finalization error are preserved; neither caused a compiler workload rerun. The earlier source-form, include setup and replay-scope refusals also remain recorded.

## Retention

- `source/` retains the exact candidate Git bundle and creation/verification records. Its advertised head is the candidate; its sole prerequisite is published `e0b21e66`. It is not standalone. Git bundle verification passed; no isolated fresh-repository reconstruction is claimed.
- `preparation/` retains 386 closed capture/proposal records, three exact desired-projection overlays, all three compressed artifacts (actual candidate emitter, preliminary old-emitter live image and tests-input tar), both original proposal directories and both auditor gateway outcomes. Every compressed member and artifact was verified against original bytes.
- `gate/` retains all 20 outer capture files and 123 top-level gate logs, the independent audit and a verified 143-member archive. Nested fixtures, stores and key contents are excluded.
- `retention-copy.json` and `gate-copy.json` verify every copied path, mode, byte count and digest, and unchanged inputs.
- `focused-summary.json` and `producer-pin.json` bind the actual passed evidence to the precise producer used by the harness.

The experimental source's retained 1,401 added/deleted language lines were independently audited as 30 semantic operations: 15 core, three test and twelve mutation operations. They are not part of main's operation-adoption numerator or denominator unless the candidate is adopted. Compressed source copies earn no additional credit.

## Execution and decision

Use the final pin and fresh main path `target/compiler-scalar-simplify.20261116-1`. One compute worker constructs A with inline64 and B with inline64 plus scalar, then runs exactly A1, B1, B2, A2, A3, B3. Each phase uses 4 GiB address space, 900 seconds and a ten-second TERM grace; the fixture needs 3 GiB free.

All six outputs must be byte-identical to the frozen `26bf2c3c` seed, 16,228,028 bytes and SHA-256 `8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132`. The primary decision is exactly `10 * sum(B wall ns) <= 9 * sum(A wall ns)`, from GNU elapsed values. Medians cannot reverse it. There are no warmups, profiling phases, cache resets, retries, outlier exclusions or adaptive changes. Failure ends the cohort and retains its partial results.

All compiler/gate/compression workers are finished before launch. The outer launcher records its invocation, cwd, status and full diagnostics, plus this release's Git commit. Both constructed producers, all six outputs, frozen source, expected image and emitter remain until independent acceptance. Cleanup is separate.

Candidate CI is pending until/if main adoption; the baseline run at `7716f0d5` is not candidate validation. A miss discards this producer optimization. A qualifying result still requires separate production integration, seed/fixed-point work, gate and CI. This release grants no automatic adoption, Rust-O milestone or backend-floor claim.
