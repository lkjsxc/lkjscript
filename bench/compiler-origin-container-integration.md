# Combined container and generated-origin integration

The integrated source at `62fa76f4f863a169b906094f0dd9998c8421e77a`
passes the complete four-worker gate from a new empty store. This combines
[authenticated semantic containers](compiler-query-container.md) with
[validated generated-source provenance](compiler-generated-source.md).
The separate focused failures, measurement cohorts and 29 historical replay
proofs retain their original identities; none is retroactively replaced.

## Source, fixed point and seed

| Record | Value |
| --- | --- |
| Gated commit | 62fa76f4f863a169b906094f0dd9998c8421e77a |
| Source tree | 00d1109fcdc49041d13d7406ca74c8f5b4394e8d |
| Library tree | 25bdb8ff8567b941790d3951c78e896b23e26e8e |
| Runtime blob | 2fed59cd33d8a2ab80d562f160648b05f24b8c4c |
| Canonical executable SHA-256 | 34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377 |
| Canonical executable bytes | 32,196,802 |
| Source/library/tools/tests inventory | 1,860 files; SHA-256 4b4b74b2dbfa418a2ecfaddd3ae7e2ecb2ea891f09880c4163e4f2fec9511b05 |
| Seed-only commit | 72a259e8889cff6e7c4ac712d7940b77343a9f5b |
| Compressed seed SHA-256 | 0f2b0b0f0d62dc31e625eea80cfb37e317f007f6fcb4a7e8ecf6ef5249d51522 |
| Compressed seed bytes | 6,466,085 |

The versioned bootstrap recipe uses relative `src/main.lkj`,
`--library-dir library`, `--runtime-source library/runtime.lkj`,
inline limit 64 and no source-directory prefix, from the main repository.
Two consecutive current-emitter generations compare byte-for-byte; no bridge
was needed. The copied bootstrap script and its successful detail log are
retained. The expanded random temporary executable/output path was not logged
and is not reconstructed as a literal argv.

Root copied the resulting executable to an owned read-only canonical file.
Seed compression used `gzip -n -9` twice with exact output comparison;
decompression also compared byte-for-byte with that canonical executable.
Only the seed changed in its commit. Neither this refresh nor these comparisons
run a new performance experiment.

## Complete local gate

The command was `LKJSCRIPT_GATE_JOBS=4 bash tools/check.sh`, with an explicitly
new empty authenticated store, the existing private compiler key outside source
grants, a 4 GiB inherited address-space ceiling and a 2,400-second watchdog
with ten-second termination grace. Available disk before launch was
389,439,467,520 bytes. The gate began at 10:36:31 UTC and ended at 10:46:38 UTC
on 2026-11-16.

| Observation | Result | Wall s | Load 1m before → after |
| --- | --- | ---: | --- |
| Full gate | Passed; 5,448 fresh case requests, zero skipped/reused | 606.794 | 1.15 → 3.70 |
| Bootstrap | Consecutive generations identical; no bridge | 76.437 | 1.15 → 1.63 |
| Semantic query | Passed | 5.752 | 1.59 → 1.86 |
| Generated-source regression | Passed | 19.812 | 1.86 → 3.68 |
| Authenticated query-store cases | Passed | 2.061 | 1.86 → 2.36 |
| Deferred semantic index pages | Passed | 21.831 | 2.36 → 3.79 |
| Checked meaning | Passed | 4.414 | 4.51 → 4.51 |
| Existing history-replay suite | Passed | 2.896 | 4.51 → 4.46 |
| IR mutation suite | 128 mutants; zero findings | 2.413 | 3.76 → 3.70 |

The outer GNU time record is 606.79 s wall, 1,522.09 s user, 21.90 s kernel,
and peak RSS 1,382,056 KiB. The enclosing load triples are
1.15 / 4.30 / 5.26 before and 3.70 / 4.27 / 4.72 after.
RSS is GNU time's reported process/child maximum, not simultaneous aggregate
resident memory across the four workers. These gate observations are validation
costs, not a speedup claim against earlier host-load cohorts.

The mutation detail separates 104 rejections, 22 values, one trap, one bounded
nontermination, zero resource outcomes and zero findings. Its internal interval
is 2.410 s, within the 2.413 s enclosing suite at the same load pair.
The ordinary and inline64 conformance/generated suites both reran their cases.
The new generated-source suite also passes in the integrated v3 store context.

All 114 fresh suite/detail logs, outer status, metadata and timings are retained
under [integration-data/local](compiler-origin-container-integration-data/local/).
The complete gate log SHA-256 is
`2aeaa17467a5fe6019efac9073b79e0d4f8520e9d5e88c87a5ed615dd53cedf7`.
The new gate store held 318,819,150 apparent bytes at completion. It is a
temporary owned fixture, retained for acceptance/cleanup; no key bytes are
copied into evidence.

## Boundaries and follow-up

This gate does not rerun the refused historical case, measure the current
compiler's query latency, resolve the old 1M missing metrics, or execute the
blocked new syscall/race observer. The existing parser/authentication and
publication suites retain their own tested scope.

The next query cohort is separately pinned to this exact producer/source and
requires both indexed compiler images to equal the canonical executable.
The isolated historical replay follows that cohort; its new pass count remains
conditional. A distinct ordinary-build 1M protocol may then localize refusal
under a predeclared ceiling while preserving the old incomplete observation.

Both GitHub jobs for this integrated source/seed remain pending at this record.
The source, seed and all local proof bytes are ready for publication; CI results
will be added with their exact run/head identities.
