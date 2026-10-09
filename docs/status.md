# Current status

## Source integrated; publication held

The selected product identity is **0.1.89**. Its checked linear Map-entry projection
is integrated through [PR #7](https://github.com/lkjsxc/lkjscript/pull/7).
Integration commit `91d3a9b36a1d9fb2156de1114f238839dffd4f69` has the exact accepted
source `b6a262f2e1ff95cb6f3889b0d10734a9ea93e078` as a parent and the identical tree
`d9d0f42beccae454480a51d25bcab734b8e8dc25`. GitHub and the fetched Git objects were
independently checked. A later status-only descendant is not a newly tested runtime.

The last independently observed public release is immutable **v0.1.83**. No release
candidate was dispatched, no immutable tag/assets were replaced, and no application
was deployed in this continuation. Mainline source availability and public-release
closure are separate. Do not promote the Map successor before the final-byte correction below passes
its fresh source and finalized-candidate acceptance.

## Accepted behavior and observed verification

`core.map.entries` now walks the actual entries of one exact admitted immutable
parent, rather than listing keys and restarting a tree search for each key. It
preserves complete raw admission, exact origins, immutable sharing, retained Map
versions, bounded reservation and joined failure cleanup. The independent reference
evaluator keeps its lookup-based implementation. The cursor is private runtime
machinery, not a new native borrowing interface.

The official frozen-source `check full --fresh --jobs 1 --machine` completed with
**exit code 0**, collected directly from its original process. The checker was built
with the pinned locked source and `CARGO_BUILD_JOBS=4`. Its zero return requires all
26 full-profile gates to pass, stable initial/final source inputs, no reused full
proof and no final snapshot error. The source stayed at the accepted HEAD/tree and
was clean after completion.

A scoped tool-policy denial prevented independent rereading of the detailed
receipt/logs. That denied inspection was not repeated through another route. The
completed command status and its maintained return conditions were observed; no
individual workspace test total, receipt digest or hosted acceptance is claimed.
The original source-check evidence remains retained, not replaced with a summary.

A separately compiled, source-matched and copied executable harness passed **2/2
tests**, including fresh public authoring/check/build, source/transport removal,
**192 complete-result executions (96 paired cases)**, malformed-input/resource
refusals, recovery and joined cleanup. Every complete result is checked against an
independent ordered-map oracle. Candidate executable SHA-256:
`f9d8d5aff6f40c391ce2e46be1b4ce7f804fc30197630acff891c861cb054e43`.

A second experiment used the same detached artifact and literal inputs under both
preceding main source `f0fe59e90068cd2465bfb70af6e3f2feedf2b25a` and the candidate.
All **384 complete-result executions** matched the independent expected bytes.
The exact input domain, each version/control combination, complete raw input-node
admission, unchanged input identities and joined cleanup were checked separately.

For shape 0 and eight repeated enumerations, subtracting each version's matched
non-enumerating control gives:

| Map entries | Preceding main extra tree visits | Candidate extra tree visits |
| ---: | ---: | ---: |
| 1 | 16 | 8 |
| 32 | 1,336 | 256 |
| 1,024 | 82,016 | 8,192 |
| 4,096 | 393,328 | 32,768 |

At 4,096 entries, shape 2 needs 393,336 preceding-main visits; all four candidate
shapes need 32,768. Additional modeled cumulative allocated bytes remain equal in
all 96 groups: 19,919,104 bytes in both products at 4,096 entries and eight passes.
These are operation counters and accounting, **not elapsed speed, RSS, actual total
allocation traffic or whole-application gains**. The predecessor is the preceding
main implementation, not the public v0.1.83 release. No comparison with C, Rust or
Bun was performed here. See the [performance guide](performance.md) for stage and
measurement boundaries.

## Final-byte correction under acceptance

[PR #8](https://github.com/lkjsxc/lkjscript/pull/8) now carries the correction to
final-archive Map coverage. Its original diagnostic source
`493f58e4cf74aa4e048cca64287aa98090f7d7e3` and failing originals remain preserved.
The three inventory regressions again failed before the correction (0 passed,
3 failed, exit 101); all pass with the complete matrix required.

The full Map suite is shared by `public_cli` and the standalone test entry. The
public owner requires the exact detached execution matrix and every enumerated
`native_map_entries::` case. Its validated release candidate reaches the copy
helper explicitly; component-candidate and development-binary fallbacks cannot
replace it. Candidate/copy identities, complete results, full raw admission,
source deletion, refusal/recovery and joined cleanup remain required.

Final-candidate acceptance contract and native-public workload generation 4 reject
predecessor and mixed generation 3/4 terminals. No unrelated encoding changes.
The correction's focused inventory/runner checks pass, and the copied public
harness passes the full matrix, its independent cost oracle and isolated candidate
binding regression (3 passed, 0 failed/ignored). This is focused development
proof, not fresh full-source or finalized-archive acceptance.

Fresh full-source verification, normal main integration of the correction, a new
candidate from integrated source, unchanged-asset promotion and anonymous public
verification remain the release gates. The original correction proposal remains
unused; the current change was reviewed against the actual source. Old candidate
`37643217328/1` belongs to different source and cannot attest this successor.
New originals are in `.artifacts/map-release-20261009/` in the diagnostic checkout;
old proof and failures remain at their original paths below.

## Retained owners and continuation

The frozen runtime/evidence checkout remains `/home/coder/workspace/lkjscript` at
accepted source `b6a262f2`. New source-check originals are in
`.artifacts/20261009-map-fresh-acceptance/`; the exact checker run is
`.artifacts/lkjscript-dev/check/1791491548750073678-760500-0`. Launch request:
`lkj-new-independent-full-20261009-b3`; completed original-session observation:
`lkj-source-terminal-final-observation-20261009-o1`.

The diagnostic worktree is `/home/coder/workspace/lkjscript-map-public-20261009`.
Its evidence owner is `.artifacts/map-public-acceptance/HANDOFF.md`, with independent
copied-product originals in `fresh-product-proof/`, matched originals in
`matched-modeled-work/`, and negative controls in `negative-inventory.*` and
`committed-negative.*`. The retained source-deleted native project is
`/tmp/lkjscript-components-3PGENt`. Completion comments on PR #7 record the exact
executable, harness, artifact and result-file identities. Preserve these originals,
the earlier denied/failed evidence, unrelated stashes/worktrees and running apps.

For the next performance step, measure actual consumers before introducing a new
representation: list/entry-record construction remains a separate modeled cost.
A traversal/fold or producer-consumer fusion may avoid a materialized intermediate
when semantics permit; that is a hypothesis, not implemented capability or measured
application speed. Language-first design, complete admission and independently
checkable semantics remain the [project direction](direction.md).
