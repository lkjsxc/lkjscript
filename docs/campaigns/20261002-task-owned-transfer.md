# Same-invocation task-owned transfer

## Original mandate

> [https://github.com/lkjsxc/lkjscript](https://github.com/lkjsxc/lkjscript) について進めるようよろしくお願いします。必要であれば大幅なスタイルの変更も許容します。あらゆる判断において、過去ではなく、今のあなたに委ねます。全ての判断をあらかじめ許可します。最も筋の良い方向に進むために、限界まで深く考えてほしい。超長期的な視点からお願いしたい。どれだけ時間がかかっても構いません。

## Reconciliation and selected scope

Entry is main `be75eff9746cfe3a80c84c73b299a05fd4f5ea78`, with the reused clean
`/home/coder/workspace/lkjscript-owned-generics-20260930` worktree on
`lkjsxc/tomato-ocelot-73`. The main checkout, original two untracked files, old
stash, other worktrees, ignored evidence and running services remain separate.
Rust 1.98.0 and the locked workspace are the maintained build boundary.

The original 0.1.64 candidate was already healthy; do not dispatch a duplicate or
freeze new development behind publication. Its final acceptance and exact-byte
21-case supplement completed independently in the
[predecessor campaign](20261002-owned-choices.md#final-candidate-and-exact-archive-supplement-completed).
Reporting-only `e887fad9f88da1cd016a40daba12bcccc7d77449` records that result;
it does not contain the new language implementation or relabel candidate source.

The [structured-transfer decision](../decisions/20261002-structured-owned-transfer.md)
identified the first missing boundary: owned memory could compose across pure
helpers but not ordinary task calls carrying exact capability resources. Implement
that smallest useful boundary before inventing channels, cross-origin handoff or a
scheduler. This is not a cosmetic rename of resource handles as memory ownership.

Development identifier 0.1.65 introduces consume-only owned parameters/results on
named first-order task helpers. Parameter regions are ordinary data/callbacks,
owned memory, then exact capability resources. Generic task hosts can forward
explicit implementation witnesses; concrete contract methods remain pure and
monomorphic. Kernel and independent source classification, full package admission,
canonical preparation and both evaluators must agree. Semantic validator 22
invalidates proof reuse; graph/type/owner, request, instruction and artifact wire
formats do not change. The [specification](../spec/owned-task-transfers.md) owns
custody, effect and failure semantics.

## Failure-first evidence and corrections

The immutable predecessor executable is retained as
`.artifacts/20261002-task-owned/predecessor-lkjscript`. Against the two new public
native task/resource fixtures, `predecessor-verified.log` reports 0 passed, 2 failed,
exit 101: concrete memory is rejected by the pure-only signature restriction and
generic memory additionally by the old Owned owner restriction. These are semantic
failures, not malformed syntax. Initial shell/PATH and incomplete background-log
attempts are not counted as a predecessor acceptance run.

The first new implementation passes the complete concrete public task/resource
case. Its generic case and four-package witness case expose separate full-package
checks that still require pure function owners/operands. The independent compiled
and source interpreter tests also fail at full source reconstruction, before value
execution. These genuine failures are retained in
`/tmp/lkjscript-task-owned-20261002/initial-public.log` (1 passed, 2 failed) and
`.artifacts/20261002-task-owned/focused-initial.log` (0 passed, 2 failed).
Full validation now permits exact first-order task owners and task-hosted witness
operands without relaxing the purity of concrete owned-method implementations.
Two compile-only feedback failures (a missing test-observer count method and the
expanded validator-feature array length) were corrected; their logs remain intact.

The corrected runtime passes all six selected unit tests, including allocation
counts, cleanup sweeps, raw ingress/egress, unused parameter mutations and a
consistently rehashed untaken artifact attack. Concrete and generic public resource
cases both pass. The initial four-package fixture then correctly encounters an
ambiguous exported name: package aliases select a package export inventory, not
an implicit submodule filter. Its task exports now use unique `task-produce`,
`task-relay`, `task-attempt` and `task-recover` names. The product's exact-reference
ambiguity refusal remains unchanged. `.artifacts/20261002-task-owned/focused-corrected.log`
retains six unit passes and two public passes/one failed fixture; it is not relabeled
as all-green acceptance. The later namespace-only fixture and new resource-order/
missing-effect negative case require their own fresh public execution.

The fresh unique-name run (`public-namespaced.log`) passes the three concrete,
generic and order/effect public cases but fails the four-package consumer with
`kernel_affine_function_parameter_use`. Resource signature inspection correctly
reads the foreign parameter yet classifies its Owned type parameter in the caller's
package. The production classifier now reads the constraint at the exact defining
package; the disjoint resource oracle independently does the same. It never searches
arbitrary dependency inventories or substitutes a caller's same-ID declaration.
An isolated defining-package/caller collision test covers all four Owned/ordinary
constraint combinations and absent foreign metadata. This tests the classifiers,
not a fabricated accepted package: actual package publication and detached use
remain the four-package public test's responsibility. The original 3-pass/1-fail
run is retained, and the corrected source requires fresh execution.

## Acceptance boundary

The selected tests include actual resource use and source-free public artifacts,
four independently exported packages, exact same-Self implementation selection,
I64 extremes, products and success/rejection choices, independent allocation and
live-owner inventories, raw entry/return rejection, traps, cancellation/quota exits,
unused task-loan mutations and consistently rehashed untaken duplicate transfers.
The verification owner lists the obligations in
[verification](../spec/verification.md#same-invocation-task-owned-transfer).
Test design and a passing local plan are not completed acceptance. Fresh focused,
full-source, copied-host and mainline results will be recorded separately below.

No asynchronous channel, structured child task, receiver-origin rebinding, fairness,
parallel scheduler, transaction spanning adapters, automatic retry or rollback is
introduced. Public binary publication and running-service deployment are separate;
the accepted original 0.1.64 assets must not be replaced by a 0.1.65 local build.
