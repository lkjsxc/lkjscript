# Current status

Snapshot: 2026-10-11. Development **0.1.93** selects checked consuming extraction.
Implementation and focused proof are present; complete fresh source acceptance,
mainline delivery and finalized-byte publication remain separate gates.

## Checked consuming extraction

Production `unpack-owned` preserves the exact live parent allocation's admission
for ordinary immutable fields. Owned children retain exact-type, domain, custody
and allocation-admission checks. A sealed extraction owner holds every undelivered
field through cancellation or failure. It reuses the original vector and removes
an unused field-type vector allocation. Raw ingress, packing, transfer and the
independent reference evaluator retain their admission boundaries.

The [decision](decisions/checked-owned-extraction.md),
[verification obligations](spec/verification.md#checked-consuming-extraction) and
[release notes](releases/v0.1.93.md) own the mechanism and limits. The maintained
[indexed-data application](../examples/indexed-data-storage/README.md) adds complete
nested displaced values after Flat/Chunk32 destruction and multi-field nominal
extraction. No language, interface, compiler, bytecode or artifact format changes
are required. All version components remain opaque identifiers.

Selected worktree:
`/home/coder/workspace/lkjscript-checked-extraction-20261011`, branch
`work/checked-owned-extraction-20261011`, based on remote main
`19055ba03640b7e13fd1150cdef20b2908cf17c3`. Preserve its ignored evidence and the
mixed-storage worktree until their separate acceptance and release gates close.

## Verification selection

The complete workspace test build, release build and strict workspace clippy pass.
All **15** new adversarial extraction unit tests pass. The copied-product extraction
witness passes **18** detached invocations. Its accepted predecessor fails the
intended mode-2 cost ceiling at N=0, K=1 while retaining complete input admission;
this expected negative is not candidate failure. Original logs, immutable selected
executables, Cargo output bindings and the negative control remain under
`.artifacts/checked-extraction/` in the selected worktree.

Both metadata cost tests pass against development and release-profile copies,
including the complete **100-point** matrix, malformed unused input, resource
refusal and healthy recovery. At N=1,024 and K=128, mode-2 raw admission falls from
262,656 to 131,328 nodes; complete ingress remains 1,027. These are modeled work
counts, not copied bytes, RSS or API billing. Standard, journal, native-guide and
native-policy checks pass **116/44/79/62** tests respectively with equal production
and reference results. Generated guides are refreshed through product discovery.

Matched release-profile measurements pass all **288 executions** across 18
workloads, with exact counts, complete results, joined cleanup and unchanged bytes.
At 128 repetitions, pack/unpack invocation ratios are 0.517 at 1,024 items and
0.479 at 16,384 items. Shared-host conditions, preparation costs and slower controls
remain in the [workload report](../examples/owned-metadata-costs/README.md#matched-extraction-measurements-2026-10-11)
and original `costs/measurement-first/` evidence. These are not portable speedups.

The three expanded indexed-data public tests remain live under their original
process/log evidence at `tests/public-indexed-first.log`; record the terminal before
claiming success. Next: freeze the reviewed source, run the checkout-bound immutable verifier with
`check full --fresh --machine`, require every dependency-complete gate and stable
inputs, then refresh remote main/protections and integrate normally. A focused
pass, local commit or candidate executable is not full source acceptance.

## Existing mixed-storage release

The separately accepted 0.1.92 source remains
`a7a3d778fe34df5ed9a8f0fb5595fbf2f858c1a5`, integrated into main before this work.
Its **26/26 fresh gates**, source and integration bindings, retained failures and
measurement originals remain at the
[previous status](https://github.com/lkjsxc/lkjscript/blob/19055ba03640b7e13fd1150cdef20b2908cf17c3/docs/status.md)
and `/home/coder/workspace/lkjscript-mixed-storage-20261011/.artifacts/mixed-storage/`.
Those proofs do not attest the new extraction implementation.

Reuse the single non-publishing v0.1.92 producer
[38130920573/1](https://github.com/lkjsxc/lkjscript/actions/runs/38130920573).
At **10:59:35 UTC**, authenticated workflow 343592338, official repository/head
repository and exact source matched; the run remained live in fresh source
acceptance, started at 10:18:30 UTC. No terminal or artifacts existed yet and no
failure was observed. Original API evidence is in
`delivery/recheck-20261011T105935Z/` under the mixed-storage evidence root.

At the next meaningful boundary inspect that exact run/attempt. Require original
generation-9 `candidate_accepted`, authenticate assets/verifier/handoff, then follow
[release](release.md) for exact-source tag/control selection, unchanged-asset
promotion and `immutable_published_and_public_verified`. Do not restart healthy
work or rebuild promoted assets. No tag, scoped-control change or promotion has
occurred. Latest verified public release remains **v0.1.89**, immutable release
407859944. Defer a successor candidate until this producer reaches its terminal
and the successor has complete fresh source acceptance; reconcile current source
and occupancy at that trigger instead of dispatching competing identities.
