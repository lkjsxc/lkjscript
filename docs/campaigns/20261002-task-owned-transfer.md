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

The first corrected focused run passes eight library controls and fails the remaining
witness golden-digest assertion; its public target is unrun because Cargo stopped at
the failed library target. The literal 0.1.64 digest becomes an explicit non-reuse
control, while the new literal digest pins the independently enumerated updated
feature inventory. The original `focused-continuation.log` remains a failure, not
an accepted run. The resumed focused command collects both test targets.

The corrected focused command completes with **9 library tests and 5 public CLI tests
passing**, zero failures and zero ignored selections. It includes the complete
four-package source-free task/witness lifecycle, concrete and generic resource/memory
composition, illegal ordering/effect rows, unchanged resource-copy rejection and the
new positive product-consumption control. Cargo-reported test executables are
`target/debug/deps/lkjscript-26a9856cbc97ee55` and
`target/debug/deps/public_cli-04f06cd51021249e`; the original log is
`.artifacts/20261002-task-owned/focused-corrected.log`. This feedback is not yet
full acceptance or a proof of the final optimized copied host.

## Accepted source and mainline delivery

Fresh full acceptance completes on exact source
`fbac03b256b351fba44cd94f1960acf042f000c7`, tree
`6f3a34d7a72ab7de8582980b71f7b0582efb023d`, using pinned Rust 1.98.0 on the Linux
x86-64 workspace, `CARGO_BUILD_JOBS=4` and checker `--jobs 2`. The original
`.artifacts/lkjscript-dev/check/1790931338572084651-3370767-0/receipt.json`
reports **26/26 freshly passed gates**, zero reuse, stable inputs and no unrun gates.
Initial and final input identities are both
`verification_9e136a687aad5e072fbe91bd29659cb5ced526900f07a49c6e0c908307137147`.
The checker and wrapper join with exit zero. The preserved outer log is
`.artifacts/20261002-task-owned/full-fbac03b2.log`.

The complete all-target/all-feature workspace command passes **1,511 tests** with
zero failures and 29 pre-existing ignored cases. This includes 1,060 library and
190 public CLI passes, the independent custody model, contributor tooling and the
native documentation-site targets. The separate optimized command lifecycle,
offline packages, HTTP/stateful/service owners, generated references, product
surface and maintained native package/artifact comparisons also pass. Historical
failed runs above remain failures; no quota or acceptance gate was waived.

A separately copied optimized executable and the Cargo-reported, source-matched
`target/debug/deps/public_cli-035dfc3eafbe52c5` harness execute outside the checkout
under `env -i`, with only `LKJSCRIPT_RELEASE_CANDIDATE` selecting the product.
All **25** selected owned-memory, task-owned, byte-range and resident-policy tests
pass, zero failures/ignored cases. Four new task-owned public cases extend the
previous 21-case supplement. The exact invocation uses `--test-threads=2` and
filters `native_owned_ native_task_owned_ native_byte_buffer_ native_byte_ranges_
resident_policy`; `selected-tests.list` independently records the 25 selections.

These originals remain in `/tmp/lkjscript-final-0165-fbac03b2-20261002/`:
`lkjscript-verified` is 28,640,936 bytes, SHA-256
`e9433ea17d84beb50f6034be28331eeaff46da96a40b888b0db988f2b2770860`;
`public-cli` is 528,370,800 bytes, SHA-256
`590ad07edcde289f3c8fee542e5a5a0f0adb541c20ef7af17bd0e7980497c3ef`.
The completed `copied-host.log` has SHA-256
`cf5b9c93aeb6a3ab69f3633889fb048cde86ba40d758435a82a658a73aa20a20`.
The final full-run product still compares byte-for-byte with that accepted copy.
This is optimized host proof, not final musl-archive acceptance or a public 0.1.65 release.

The first attempted product copy failed `cmp` at byte 813 before any execution;
its original `lkjscript` copy is retained in the same directory with SHA-256
`66bc5a86cd8fac54b9c0c3bc4097f3cb6f37a7fb2f9a6cbcd5ee178db27699eb`.
The source release output was being generated concurrently, but the exact cause of
the mismatch is not established. No claim is made for that unexecuted copy.
A later explicit byte copy binds matching source digests before/after copying and
an equal destination digest, then passes the public supplement above. It does not
rewrite the initial mismatch as success.

After refreshing main and its unprotected branch state, a normal fast-forward
integrates the accepted source from `e887fad9f88da1cd016a40daba12bcccc7d77449` and
pushes main to `fbac03b256b351fba44cd94f1960acf042f000c7`. An independent GitHub ref
read confirms that exact result; local main and origin agree. The two original
untracked main files, existing stash, other worktrees and unrelated services remain
untouched. Subsequent status/decision reporting must not relabel this tested source.

Public/latest is separately completed immutable **v0.1.64**, accepted from source
`0048ae1ee2e4678b409c782e02044b038bf60052` by original producer `36966111016/1` and
promotion `36984555849/1`. The [publication record](20261002-owned-choices.md#completed-v0164-publication)
retains its independently checked unchanged assets and anonymous installed verification.
Development 0.1.65 is not selected for a second immediate binary publication; it
remains integrated source. The next language experiment is the joined bounded
handoff, not an ambient memory origin or a detached task API. Its
[reservation refinement](../decisions/20261002-structured-owned-transfer.md#bootstrap-channel-reservations-are-not-acceptance)
records the independent library observations and their limits.
