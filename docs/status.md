# Current status

## Source accepted and integrated

The selected product identity is **0.1.89**. Checked linear Map-entry projection
and the final-byte verification correction are integrated through
[PR #7](https://github.com/lkjsxc/lkjscript/pull/7) and
[PR #8](https://github.com/lkjsxc/lkjscript/pull/8).
The correction's accepted source is
`4fba5833c7d7354f17e9413b73bb8935600a85d4`, tree
`e964347611be1705c9f5d0849e1b86304810f53d`.
Normal merge `440ff34d30760a125d4c766b85899cd8900fe3e7` has that source as its
second parent and the identical tree; fetched Git objects and GitHub main were
independently checked. Reporting descendants are not new product acceptance.

Fresh `check full --fresh --jobs 1 --machine` completed with original-process
**exit code 0**, all **26/26 gates fresh passed**, zero reused or unrun gates,
and identical initial/final input digests. Its original receipt was independently
read. The selected checkout remains clean and frozen at `4fba5833`.
The public CLI target passed 237 tests, failed 0 and retained one existing ignored
case. Its complete Map matrix and candidate-binding regression passed. Standalone
Map tests passed 2/2; tooling passed 259 with no failures and 19 existing ignored
cases. The original library target passed 1,694 with 8 existing ignored cases.

Evidence checkout: `/home/coder/workspace/lkjscript-map-public-20261009`.
Original full run:
`.artifacts/lkjscript-dev/check/1791523334550838046-1232059-0/receipt.json`.
Receipt digest:
`verification_63bc0512ab1d5ec7fe9ad8751abf13d896eac86dcf8d0c6e194de85d2f39e843`.
An unchanged copy is retained in `.artifacts/map-release-20261009/full-originals/`.
Original terminal observation: `release-full-terminal-n3`.

## Required final-byte Map coverage

The complete Map suite is shared by `public_cli` and the standalone entry. The
final owner requires the exact detached matrix and every enumerated
`native_map_entries::` case. The validated release-candidate path reaches the copy
helper explicitly; component/development fallbacks cannot substitute other bytes.
Executable identities are checked around copying and execution. A cost-only suite,
similar name, wrong namespace, omitted/duplicate/ignored/failed test or forged
success summary cannot satisfy admission.

The matrix retains 192 complete production results over 96 paired cases, independent
ordered-map expectations, full raw input validation, source/transport deletion,
resource refusal, recovery and joined cleanup. Acceptance contract and workload
advance together to generation 4; old or mixed 3/4 terminals reject. No semantic
encoding generation changes accompany this distribution-acceptance correction.

The original diagnostic tests were rerun before the fix: 0 passed, 3 failed,
exit 101. All three pass after the correction, alongside candidate identity,
contract and controller regressions. New focused originals are in
`.artifacts/map-release-20261009/`; earlier failed and independent runtime evidence
remain in `.artifacts/map-public-acceptance/` at the same checkout.

## Distribution

The last observed public/latest release remains immutable **v0.1.83**.
Candidate [37892791430/1](https://github.com/lkjsxc/lkjscript/actions/runs/37892791430)
selected source `440ff34d30760a125d4c766b85899cd8900fe3e7` but failed source
acceptance: 19/20 fresh gates passed and `workspace_tests` reached its unchanged
3,600-second deadline. Source inputs remained stable. The diagnostic ZIP
`candidate-diagnostics-37892791430-1` (artifact `11601692753`) and original logs
are retained; no finalized candidate was accepted or published.

The workflow follow-up serializes source gates with `--jobs 1`, avoiding overlap
between workspace tests and the release lifecycle Cargo build. Workloads, Cargo
parallelism, test cases and deadlines remain unchanged. Its fresh full check and
new integrated-source candidate remain required before annotation, promotion and
anonymous public verification. No release tag or scoped publication control was
changed. Failed attempt 1 is not reusable acceptance.

## Map behavior and measurement limits

`core.map.entries` projects the actual children of one exact admitted immutable
parent in key order. It preserves complete raw admission, exact origins, persistent
versions, bounded reservations and joined cleanup. The reference evaluator retains
its independent lookup-based path. This is private runtime machinery, not a new
native borrowing interface.

Earlier matched observations used the same artifact and literal inputs under
preceding main `f0fe59e9` and Map implementation `b6a262f2`, with all 384 complete
results matching independent expectations. At 4,096 entries and eight enumerations,
additional modeled tree visits fell from about 393,328 to 32,768. Additional modeled
cumulative allocation remained 19,919,104 bytes in both products. These counters
are not elapsed speed, RSS, actual total allocation traffic or application gains.
The comparison predecessor is preceding main, not public v0.1.83. See the
[performance guide](performance.md) and [Map workload](../examples/map-entry-projection/README.md).

## Retained originals

The original implementation/evidence checkout `/home/coder/workspace/lkjscript`
remains at `b6a262f2`, with its old full run
`.artifacts/lkjscript-dev/check/1791491548750073678-760500-0` preserved. Its original
command exit 0 was previously recovered; its detailed receipt was not independently
reread after the earlier tool refusal. The new correction acceptance above is a
separate fresh run and does not relabel those historical observations.

The handoff `.artifacts/map-public-acceptance/HANDOFF.md`, independent
`fresh-product-proof/`, `matched-modeled-work/`, original diagnostic failures,
source-deleted `/tmp/lkjscript-components-3PGENt`, and unused `implement.pl` remain
preserved. Older candidate `37643217328/1` belongs to different source and is not
this release's producer. Existing stashes, other checkouts and running applications
remain outside this release task. No application deployment was performed here.
