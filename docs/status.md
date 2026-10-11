# Current status

Snapshot: 2026-10-11. Development **0.1.94** implements native package dependency
planning. Its first full run failed `workspace_tests`: **28/29** fresh gates passed
with stable inputs, **0 reused** and **0 unrun**. The three reviewed fixture
corrections pass all **18** focused cases with the sixth all-target build;
complete fresh acceptance remains pending. Mainline integration and finalized-byte
publication are separate gates.

## Selected source and capability

Worktree `/home/coder/workspace/lkjscript-native-package-planner-20261011`, branch
`work/native-package-planner-20261011`, contains implementation `e9564f2d` and normal
ancestry merge `77decedce1cdb188c8712fd9854996332b15cb4f`. The merge incorporates main's
reporting descendant without changing implementation. First full source
`d1488367758226e02a3c5522476b7058fd881ce2` remains distinct from test-only corrected
source `5a9629ef986ed2dc63b8befbc8ad081a10c04b28` and the accepted predecessor.
This remains a local development candidate. Preserve the worktrees and their
ignored original evidence;
[selection.json](../.artifacts/native-package-planner/selection.json) records live
process handles and subsequent development evidence.

Production logical admission invokes the checked native planner for cycle refusal
and dependency-first order. Its operation-local private plan passes through
transport and source admission into compilation after complete budgeted canonical
rereads and exact context matching. Standalone admission remains independent.
Artifact loading uses a distinct host topology reader to avoid recursive native
initialization. Actual operation cancellation reaches native computation, and
pure-tool invocation/encoding exits explicitly join cleanup. Cold artifact loading
still has no internal cancellation checkpoints; bounded cold-load cancellation
latency is not established.

The [decision](decisions/native-package-planning.md),
[verification obligations](spec/verification.md#native-package-dependency-planning)
and [release notes](releases/v0.1.94.md) own the contract. No language, graph,
interface, compiler-unit, bytecode or artifact encoding cut is intended. CLI
observation **46** adds invocation-bound planning records; candidate generation
**10** requires the expanded maintained/public coverage.

## Current development evidence

The [original full receipt](../.artifacts/lkjscript-dev/check/1791722983656536426-2131509-0/receipt.json)
records terminal exit 1 after **2,866.089739 seconds** on stable source `d1488367`.
Only `workspace_tests` failed: library **1,911 passed / 1 failed / 8 ignored**,
native verifier **8/1**, and public CLI **269/1/1 ignored** (1,551.61 seconds).
Original logs and receipt remain unchanged. The reviewed test-only corrections
account for standalone container input charges and update stale capacity/negative
precedence fixtures to the existing 10,000-node / 100,000-edge contract. They do
not change production behavior; corrected focused evidence follows.

[The sixth all-target/all-feature build](../.artifacts/native-package-planner/build/selected-sixth.json)
returned exit 0 on `5a9629ef`. All **18** focused cases pass,
**0 failed/ignored**:
standalone accounting, native capacity, the twelve reuse/budget/canonical/control
cases, three host-outcome cases and the corrected public worklist witness;
[original summary](../.artifacts/native-package-planner/tests/focused-sixth-20261011/summary.json).
This is focused evidence; the complete verifier/public matrix still belongs to
fresh full acceptance. Earlier fifth-executable evidence remains separate:

- All **4** cleanup cases pass;
  [original summary](../.artifacts/native-package-planner/tests/pure-artifact-runner-fifth-summary.json).
- Standard **116/116** and journal **44/44** pass with equal production/reference
  results and unchanged semantic HEADs. All **8** generated guides verify with
  unchanged bytes; [original summary](../.artifacts/native-package-planner/maintained/development-fifth-20261011T123533Z/summary.json).

Earlier executables retain their narrower proof scopes. The third library suite
passed **1,906**, failed **0**, ignored **8**, before reuse. The fourth copied
product passed both public planner witnesses, including exact native rebuild and
source-deleted execution, before the final diagnostic corrections. The first
bridge selection passed all **11**, including **10,000 packages / 100,000 edges**.
All **68** checker-contract tests passed with explicit pinned toolchain homes.
Their original summaries, the two fourth-binary failures and the invalid-zero-quota
cleanup fixture failures remain linked from the selection record; later passes do
not relabel those originals.

The maintained [native asset](../tools/native-package-plan/README.md), authored
through the copied predecessor, passes **170** native tests. Independent builds
produce identical 3,371,783-byte artifacts, SHA-256
`0115358c9dc9c9a91d0bac7755b619a12f36ca9b5e90b69d649639c8ccdb3a54`.
Fresh maintained regeneration remains part of complete source acceptance.

The [release-profile rebuild](../.artifacts/native-package-planner/build/selected-release-second.json)
passed on `5a9629ef`; its CLI and verifier are byte-identical to the prior release
binaries from `77decedc`. The following
[matched comparison](../.artifacts/native-package-planner/costs/run-20261011T124526Z/summary.json)
passes exact artifact, unchanged-HEAD and ten complete detached-result checks.
Five-sample build medians are **16.060 → 231.065 ms** for the tiny project,
**26.792 → 239.821 ms** for the four-package diamond, and
**9,337.967 → 10,336.357 ms** for the maintained planner. The candidate is slower
in all three observed cases. The [decision](decisions/native-package-planning.md#observed-public-build-costs)
retains scope and shared-host limitations; separate compiler phase timings remain
unavailable. The failed first harness run and its unfavorable samples are preserved.

Keep the production planner asset unchanged. The retained thin-consumer experiment
passes **156** checks and **16** matched functional runs, reducing units **575 →
543** and bytes **3,371,783 → 3,272,121**. Seven idle paired samples per lane show
aggregate public preparation medians **213.824 → 206.996 ms** (empty) and
**217.872 → 208.483 ms** (diamond), about **3–4%** lower with overlapping ranges;
[original timing evidence](../.artifacts/native-package-planner/thin-consumer-experiment/timing-idle-01/summary.json).
Thin remains an experiment. This does not establish compiler phase or end-to-end
speed; the dominant cold cost remains unresolved.

## Remaining gates

Complete source acceptance still requires all **29** full gates freshly with
stable inputs. Candidate generation **10**
additionally requires **23** release-source gates and the exact expanded public
compiler inventory. Refresh source ancestry and reconcile the pending generation-9
release before installing the generation-10 controller on main. Local commits and
focused passes establish neither mainline delivery nor publication.

## Accepted predecessor and pending release

Extraction source `2396e69cc3eccd8971bdee092ccc47e7978cbd52` passed **26/26** fresh
full gates, **0 reused**, with stable inputs and exit 0. Its receipt is
`/home/coder/workspace/lkjscript-checked-extraction-20261011/.artifacts/lkjscript-dev/check/1791716965778033360-1879238-0/receipt.json`.
Normal fast-forward delivery and independent exact-source ancestry are retained in
that worktree's `.artifacts/checked-extraction/delivery/mainline-20261011T115417228Z/summary.json`.
All three indexed-data public tests passed; verified archives remain under its
`proofs/` evidence directory. Main reporting descendant
`8fb6230aa92a43d82c5e45a093acdf58b7a5f7e4` changes only `docs/status.md`; its normal
push is independently verified in `delivery/reporting-20261011T115837243Z/`.
That receipt binds the extraction implementation, not its reporting descendant or
this successor.

The single v0.1.92 producer [38130920573/1](https://github.com/lkjsxc/lkjscript/actions/runs/38130920573)
selects accepted source `a7a3d778fe34df5ed9a8f0fb5595fbf2f858c1a5`.
At **13:56:06 UTC**,
source acceptance and final asset construction had passed; final candidate
admission remained live at step 10 (since 12:33:41), with no observed failures,
uploaded artifacts or acceptance terminal. Original
API evidence is
`/home/coder/workspace/lkjscript-mixed-storage-20261011/.artifacts/mixed-storage/delivery/recheck-20261011T135606Z/summary.json`.
Reuse that producer; require original generation-9 `candidate_accepted` and
authenticated retained assets/verifier/handoff before unchanged-asset promotion.
Close or explicitly supersede generation 9 before replacing its consumer with the
generation-10 controller. No tag, scoped-control change or promotion has occurred.
Latest verified public release remains **v0.1.89**, immutable release 407859944.
[Release](release.md) owns the required `immutable_published_and_public_verified`
terminal; reconcile actual occupancy and source at the next delivery boundary.
