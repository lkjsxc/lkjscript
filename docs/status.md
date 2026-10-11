# Current status

Snapshot: 2026-10-11. Development **0.1.94** implements native package dependency
planning. Source `d01708a2` passed **29/29** fresh full gates with stable inputs,
**0 reused**, **0 unrun** and terminal exit **0**. Mainline integration and
finalized-byte publication remain separate gates. Main remains `8fb6230a` with the
generation-9 release controller.

## Selected source and capability

Worktree `/home/coder/workspace/lkjscript-native-package-planner-20261011`, branch
`work/native-package-planner-20261011`, has accepted source
`d01708a2d089af33e79747c097ca61b5c90d4fd7`, tree
`d08f9a0d64afca0a320d0d7a8b0b93a20f788036`. It contains the native planner
implementation, normal ancestry merge from main, and reviewed fixture corrections.
This status update is a docs-only reporting descendant; the receipt binds the
exact accepted source above. Preserve the worktrees and ignored original evidence;
[selection.json](../.artifacts/native-package-planner/selection.json) retains
earlier executable selections and proof scopes.

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

## Source acceptance and retained evidence

The [successful original full receipt](../.artifacts/lkjscript-dev/check/1791727236348792345-2336981-0/receipt.json)
records **29/29** fresh gates, **0 reused**, **0 unrun**, stable inputs and terminal
exit **0** after **2,896.017618526 seconds**. Receipt SHA-256 is
`c0912b08adfd985f487cad4461006563ffd211c5e3e11929c933a6e592c191c0`.
The [verified summary](../.artifacts/native-package-planner/full-second-summary.json)
and [71-file inventory](../.artifacts/native-package-planner/full-second-inventory.json)
retain exact originals. The fresh run includes the complete workspace/public
matrix, maintained standard/journal consumers, generated guides, native planner
tests and artifact regeneration/comparison. Library **1,912 passed / 8 ignored**,
native verifier **10 passed**, and public CLI **270 passed / 1 ignored**, with no
failures.

The [first failed full receipt](../.artifacts/lkjscript-dev/check/1791722983656536426-2131509-0/receipt.json)
records terminal exit 1 after **2,866.089739 seconds** on stable source `d1488367`.
It passed **28/29** fresh gates, **0 reused**, **0 unrun**. Only `workspace_tests`
failed: library **1,911 passed / 1 failed / 8 ignored**,
native verifier **8/1**, and public CLI **269/1/1 ignored** (1,551.61 seconds).
Original logs and receipt remain unchanged. The reviewed test-only corrections
account for standalone container input charges and update stale capacity/negative
precedence fixtures to the existing 10,000-node / 100,000-edge contract. They do
not change production behavior. The preceding [18 focused passes](../.artifacts/native-package-planner/tests/focused-sixth-20261011/summary.json)
and earlier executable-specific successes/failures retain their original scopes
in the selection record; fresh acceptance does not relabel them.

The maintained [native asset](../tools/native-package-plan/README.md), authored
through the copied predecessor, passes **170** native tests. Independent builds
produce identical 3,371,783-byte artifacts, SHA-256
`0115358c9dc9c9a91d0bac7755b619a12f36ca9b5e90b69d649639c8ccdb3a54`.
Fresh maintained regeneration and exact comparison passed in the accepted full run.

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
The thin variant is not adopted. This does not establish compiler phase or
end-to-end speed; the dominant cold cost remains unresolved.

## Remaining gates

Source acceptance is complete for the exact source above. Candidate generation
**10** additionally requires **23** release-source gates and finalized-byte
admission with the exact expanded public compiler inventory. Reconcile the
generation-9 release, refresh source ancestry, then integrate the accepted .94
source/reporting descendant normally, installing its generation-10 controller
on main. No .94 mainline delivery or publication is claimed.

The separate `/home/coder/workspace/lkjscript-associated-owned-types-20261011`
worktree holds the uncompiled **0.1.95** associated-owned-types foundation. It has
no feature acceptance and is outside this .94 receipt.

## Accepted predecessor and pending release

Extraction source `2396e69cc3eccd8971bdee092ccc47e7978cbd52` passed **26/26** fresh
full gates, **0 reused**, with stable inputs and exit 0. Its receipt is
`/home/coder/workspace/lkjscript-checked-extraction-20261011/.artifacts/lkjscript-dev/check/1791716965778033360-1879238-0/receipt.json`.
Its normal delivery and retained public proofs remain in that worktree's
`.artifacts/checked-extraction/delivery/` and `proofs/` directories. Main reporting descendant
`8fb6230aa92a43d82c5e45a093acdf58b7a5f7e4` changes only `docs/status.md`; its normal
push is independently verified in `delivery/reporting-20261011T115837243Z/`.
That receipt binds the extraction implementation, not its reporting descendant or
this successor.

The single v0.1.92 producer [38130920573/1](https://github.com/lkjsxc/lkjscript/actions/runs/38130920573)
selects accepted source `a7a3d778fe34df5ed9a8f0fb5595fbf2f858c1a5`.
The authenticated **14:50:10 UTC** snapshot records the original producer completed
successfully and five retained artifacts, with no competing active release run.
Original API evidence is
`/home/coder/workspace/lkjscript-mixed-storage-20261011/.artifacts/mixed-storage/delivery/recheck-20261011T145010Z/summary.json`.
The original generation-9 `candidate_accepted` terminal and exact
assets/verifier/handoff have since been authenticated; authorized unchanged-asset
promotion is in progress.
No publication terminal is yet verified. Reconcile this lineage before replacing
the main generation-9 consumer with generation 10.
Latest verified public release remains **v0.1.89**, immutable release 407859944.
[Release](release.md) owns the required `immutable_published_and_public_verified`
terminal; reconcile actual occupancy and source at the next delivery boundary.
