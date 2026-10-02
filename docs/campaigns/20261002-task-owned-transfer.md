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

## Fixed source and public deployment correction

Implementation `8279edca3fe05b0d258266bc2c15302c69e56bfe`, tree
`fa87a746d67bff8e15fb0fea0eb2b208d694b5e1`, fixes the imported constraint scope.
The fresh `focused-import-scope.log` reports **7 unit tests passed, 0 failed,
0 ignored** (1,061 unrelated filtered, 1.11 seconds), including the new exact-package
collision probe. The public selection reports 3 passed, 1 failed, exit 101.

The actual optimized source executable is copied before subsequent build producers:
`/tmp/lkjscript-task-owned-20261002/lkjscript-8279edca`, SHA-256
`0b9185c929c6c7476360ea8841559c9235ad69f24eb91ee75696eb2d2bbe37f0`.
The corresponding initial public harness SHA-256 is
`b660967c5c0ee90c9d6fb7c80e7668e917a41fec10fd117517654aa5592ad5e1`.
The enumerated 25-case copied-host selection runs outside the checkout with an
empty environment/PATH except the candidate path: **24 passed, 1 failed, 0 ignored**,
166 unrelated filtered, 44.64 seconds, exit 101. This is not passing acceptance.
`8279edca-copied-host.log` and the separately targeted
`8279edca-package-diagnostic.log` retain the complete failure.

The remaining failure is after successful publication, import checking and artifact
construction: the four-package fixture incorrectly uses source-only `run owned-tasks`
for an empty-row task target. The existing runner correctly returns
`normalized_runner_grants_required`. Correct the test, not the authority boundary:
explicitly assert that source-only execution fails without publishing a result,
then use the existing empty-grant deployment for both before-removal and source-free
artifact execution. No product code changes after `8279edca` for this correction.
The queued `full-8279edca.log` invocation was interrupted with exit 130 while waiting
for Cargo's lock, before its harness/DAG started, so it supplies no full-gate result.
The corrected test and frozen source require fresh public and full verification.

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

## Resumed full-suite diagnosis

The resumed workspace is clean at `0d850de52fda5baee66a37a97abf337028d69020`,
tree `bca467e6185d91e237496c2a5279f6148594b834`. The independently reread main is
`e887fad9f88da1cd016a40daba12bcccc7d77449`; neither implementation descendant has
been delivered to main yet. The retained full receipt
`.artifacts/lkjscript-dev/check/1790924635227406330-3049196-0/receipt.json`
reports stable source inputs, 25 freshly passed gates, zero reuse and no unrun gates,
but `workspace_tests` fails. Its complete workspace run has two library failures
(1,058 passed, two failed, eight ignored) and one public CLI failure (189 passed,
one failed, one ignored). Other workspace targets retain their own original results.
This is not full acceptance.

The product-negative test still expected a consuming task OwnedProduct parameter to
be forbidden. Preserve that exact newly valid shape as an independently classified
positive control, and test a task product **borrow** as the negative instead. The
witness test still expected predecessor memory and generic/witness feature versions;
it now asserts the explicitly selected updated feature inventory, including the
separate same-task feature. Historical proof rejection checks are preserved.

The resource-only generic attack remains invalid, but the new memory signature
checker intercepted its resource ordering before the existing affine resource-copy
diagnostic. Restrict that ordinary-parameter ordering check to signatures where an
owned-memory parameter has occurred. Resource-only admission and its exact diagnostic
remain at the affine owner; memory-after-resource and ordinary-after-memory still
reject. The original public test is unchanged, including its no-publication and
complete-project-inventory assertions.

A fresh full invocation was mistakenly started before this retained failed receipt
was found. Its owned checker was interrupted and joined with exit 130; no full pass
is inferred. Its run is `1790929589978816249-3271642-0`, with the original output in
`.artifacts/20261002-task-owned/full-0d850de5-fresh.log`. An earlier launch with an
incorrect background-shell entry point failed with exit 127 before checking; that
separate `full-0d850de5.log` is also retained. The corrected source now requires
focused proof followed by one dependency-complete full acceptance run.
