# Independent scalar full-gate audit

Status: passed. The audit read the retained captures and exact top-level gate logs; no compiler, native image, repeat gate or benchmark was executed.

Source: 5b5ce7759c035ae0437e3512e18efc440aa50d2e. Producer: f85624e3cc694f6a3a3e10c33f822789ae6084e2d5a5b41c3b1fc8fcc9bd2d64 (32,761,734 bytes).

All 65 named phases and 123 log files agree with the pinned gate script. All 1,943 source-manifest paths were independently read and checked against both their before/after SHA-256 records and the exact Git blob IDs. The current emitter matches both retained producer hashes.

Gate elapsed: 754.526 s; GNU wall/user/system: 754.52/2016.49/35.51 s; peak RSS 1258256 KiB. Load: 1.32/1.44/1.42 to 5.01/4.81/3.26.

Fresh case requests: 7476; reused: 0. Conformance reports 813 source cases, 1,624 route checks and two pending cases per policy. Generated reports 805 source cases, 2,430 route checks and zero pending per policy; its request count is 1,215, not 1,215 distinct sources.

Scalar conformance: 149 attempts/149 admitted, 3,208 constants, 325 identities, 48 branches in 1,026 bodies. Scalar generated: 538 attempts/538 admitted, 3,862 constants, 294 identities, 54 branches in 1,595 bodies. Both have zero scalar rejections and message mismatches.

Both fixed mutation routes: 128 mutants, 104 rejected, 22 values, one trap, one nontermination, zero resource outcomes and zero findings. Scalar enabled: four attempts, four admitted, four executed, four branch rewrites in four bodies; zero other rewrites, rejections, guarded traps or message mismatches. Inline default remains zero in both routes.

The live semantic suite reports 36 native-observation contracts. Fixed generator/reducer controls, scalar CLI/cache/profile tests and both parser conflict orders passed in the gate.

## Limits

- Independent read-only audit; no compiler, native executable, gate rerun or benchmark was executed.
- All named phase markers and per-phase detail hashes were verified against the exact checked-in gate script; overall success is not inferred from a partial log.
- 7476 is the number of fresh case requests, not distinct source cases or route checks. Generated routes report 805 source cases and 1215 case requests each.
- Each conformance route retains two declared pending cases; gate success does not claim they ran.
- The scalar fixed mutation population exercised four admitted/executed branch rewrites. Executed uses the selector's existing post-invoke accounting; native outcome comparisons and zero findings are separately reported.
- The existing inliner rejected twelve conformance candidates in both inline-only and scalar routes, retaining admitted fallbacks. Scalar candidates in those routes report zero checker rejections and zero message mismatches.
- The cold-store designation follows the captured outer wrapper declaration; zero case reuse is independently recomputed. Initial store emptiness cannot be reconstructed from its later contents.
- GNU time, the gate's own elapsed marker and launch-inclusive nanosecond duration have different capture boundaries and are reported separately.
- Global key contents and store contents were not read. No compression, cleanup or process observer was used.
- Candidate CI and performance remain pending; this gate audit is not benchmark execution permission.
