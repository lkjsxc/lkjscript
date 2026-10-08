# Current status

Snapshot: 2026-10-08. Source acceptance, distribution acceptance and running
applications are separate boundaries. Product identifier components remain opaque.

## Selected worktree and unfinished work

The current worktree is `/home/coder/workspace/lkjscript`, branch `main`. The
resumption began at reporting source `6a95fcd27ee4957ff1678c084af1e2c4ac2d87ad`,
tree `09f77288edae71b6e2f9846b6b2209bbcaada3a2`, independently matching remote main.
Two pre-existing local test changes remain deliberately unintegrated:
`src/platform/execution/normalized/vm_map_tests.rs` and the untracked
`src/platform/execution/normalized/vm_map_entry_scan_tests.rs`. Neither is a runtime
fix. Preserve them and the original failed evidence; do not mistake this reporting
update for a tested implementation successor.

A new execution of `cargo test --locked -p lkjscript --lib map_ -- --nocapture`
used the existing pinned test-profile executable and passed 59 tests, failed one,
ignored zero and filtered 1,637. Only the new lookup-free entry-scan law failed.
Complete independently expected integer entries matched both evaluators at all
seven sizes, with unchanged retained input and zero live resources observed by the
test helper. Subtracting the same raw input's header-only length-query visits,
1,024 entries require 10,252 scan visits and 4,096 require 49,166 in both evaluators.
The new target is one production traversal visit per entry. These counters do not
establish a wall-time or application speedup, and this target is not a previously
established public complexity guarantee. Reverse input collection does not produce
different AVL shapes because construction first collects into an ordered host map.

The source-review request `lkjscript-resume-map-projection-20261008-9a751c` was
refused by the tool before execution and was not retried or bypassed. The runtime
optimization is not implemented. The next acceptance boundary is a lookup-free
production entry projection retaining exact parent/type admission, complete raw
validation, reservation/refusal/cancellation and cleanup, with independent and
detached public result/cost evidence followed by fresh full source acceptance.
No new full acceptance, producer dispatch, promotion or deployment occurred here.
Originals, the repeated test log, exact source/test hashes and the unfinished scope
are indexed at `.artifacts/20261008-map-entry-scan/resume-evidence-index.json`.
No test process from this resumption remains running.

## Accepted source

Accepted source: `567b4d3152fb51158b3b595362b5bdc0f065b854`, tree
`1e0b9fbe7cc694b0512469838d8fd07ed86fb266`. Its fresh full source verification passed
26/26 gates, with stable inputs, zero reused evidence and zero unrun gates.
Workspace tests passed 2,231, failed 0 and retained 29 existing ignored tests.
The 234 public CLI tests are included in that total; two filtered child probes
are separate. A status-only reporting descendant is not the tested source.

Receipt:
`.artifacts/lkjscript-dev/check/1791446554781283924-3129190-0/receipt.json`, digest
`verification_1b8c76cc1213a5265f7d5b737deb24699026e8241125c530db8fa8419f38a2a5`.
The complete run is additionally retained outside checker rotation in
`.artifacts/20261008-native-plan/full-source-verification/`.
The evidence index is `.artifacts/20261008-native-plan/evidence-index.json`.

## Native dependency-first planning

Five ordinary lkjscript modules extend the complete dependency-component witness
with distinct component dependencies and earliest dependency-first stages. For
A -> B, `successors` means that A depends on B: B's component appears earlier.
Cycles remain explicit mutually dependent units, not invalid graphs or a claim
that their members have a valid sequential execution order. Components, members
and stage peers preserve authored identity order; dependency lists preserve the
first distinct cross-component reference. Roots select reported reachability;
they do not narrow complete input validation or the all-component plan. See the
[decision](decisions/native-dependency-plan.md) and
[public authoring guide](../examples/dependency-plan/README.md).

The planner removes internal and repeated component edges, counts distinct pending
dependencies, propagates maximum dependency levels and groups the result separately
from queue order. Derived counter, range, duplicate and completion contradictions
cannot become a valid partial traversal. Internal helper signatures do not prove
arbitrary graph invariants; they require the exact validated proposal and complete
component partition. The inherited SCC pass still needs the selected worklist's
LIFO law, which ownership/type signatures alone do not prove.

The focused public suite passes all three Rust tests. Its 4,635 proposals include
all 512 three-vertex directed graphs, all 4,096 loop-free four-vertex graphs,
cyclic diamonds, repeated cross-member edges, signed extremes, long chains,
root/order changes and inherited capacity/invalidity cases. Both maintained flat
and chunked carriers match complete independent results before and after deleting
source projects and transports: 18,540 proposal comparisons. The suite uses
19 input batches and 76 batch executions; the largest argument file is 687,760
bytes, below the unchanged 1 MiB runner limit.

Two single commands, two empty batches, four raw-type/resource refusals and four
subsequent valid recoveries also pass. Successful runs retain no live owned handles,
locals, operands, frames, transactions or type bindings and join all tasks/workers.
Eleven new fixed native tests supplement the public matrix; the complete authored
application reports 122 passing tests with agreement between both evaluators.
The 18,540 public comparisons are production runs, not differential evaluator pairs.
The independent oracle uses pairwise reachability and synchronous level relaxation,
not the native two-pass SCC and readiness-count algorithms.

Focused originals, including three failed development attempts, are retained in
`.artifacts/20261008-native-plan/`. The corrected suite divides input batches rather
than raising limits. Focused executable and final full-verification release producer
share SHA-256 `b14bd646f96061f172e0947f46c0743038a51c0344a1069019afb61441869f72`.
The focused artifact is
`e83ef4e0b6bccf02c58685ab83de0db9ea5268f84686b8cbdb0732ae89228f15`.
This addition changes no Rust runtime, intrinsic, dependency, encoding, existing
component-analysis API or product identifier; development remains `0.1.88`.

Stages express graph precedence, not parallel-execution or publication authority.
No production compiler pass or checker scheduler is replaced. The optional
experiment against the actual checker DAG was refused by the tool and was not
executed; it is not adoption evidence. Bounded graph visits do not establish
linear wall time, zero copying, bounded RSS, complete self-hosting or an application
speedup. Existing applications and deployed services are unchanged.

## Retained runtime frontier

The preceding [borrowed immutable metadata](decisions/borrowed-immutable-metadata.md)
implementation remains intact. It retains a live allocation's exact prepared-program
admission when selecting an ordinary immutable field through a live read. Raw input,
packing, consuming unpack, captures and ownership transfer retain their existing
admission. Its source, independent boundary probes and matched cost measurements
remain in `.artifacts/20261008-borrowed-metadata/evidence-index.json`; the retained
[native metadata workload](../examples/owned-metadata-costs/README.md) still runs in
the fresh full profile. This planner does not claim to fix the remaining consuming
admission costs. Additional consuming-boundary inspection was refused by the tool;
no optimization at that boundary was implemented.

## Publication and next boundary

The latest independently observed public release remains immutable `v0.1.83`.
Existing `v0.1.88` candidate producer `37643217328/1` completed successfully for
source `6a15fdab20cdbf89ebc8683e8aa1d8b0f64b7142`, not this accepted source. This
step does not replace its artifacts, dispatch another producer, promote a release
or switch running apps. Publication is deferred for this native-only addition;
its selected host executable is unchanged from the preceding accepted runtime.
When selecting a successor distribution after the next accepted runtime boundary,
bind that final source and its transferred executable through the ordinary release
procedure. The existing candidate must not be relabeled as current-source evidence.

The next compiler-integration boundary must retain exact source-scoped identities,
complete candidate validation, the fixed admitted analysis implementation and
independent result/authority checks. A generic worklist type does not grant freedom
to substitute an implementation that violates the needed algorithmic laws.
Prefer this concrete connection over accumulating disconnected compiler examples.
Separately, consuming extraction needs an inseparable exact-parent/field proof
before checked construction can reuse it. Neither a free-value classification nor
an address cache proves an exact extracted field type.
