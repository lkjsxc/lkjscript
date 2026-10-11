# Current status

Snapshot: 2026-10-11. Accepted meaning, source verification, mainline delivery,
finalized executable acceptance and public availability are separate boundaries.

## Nominal owned records on main

Development **0.1.91** adds exact nominal owned records with module-private
representations and adopts public `Fifo<T>` in the native standard FIFO. Queue
order, returned custody, source-tied front reads and one-envelope runtime storage
are preserved. The [record specification](spec/owned-records.md),
[FIFO guide](guides/native-owned-fifo.md) and [release notes](releases/v0.1.91.md)
own semantics, limitations and the intentional format cut. Rebuild experimental
derived artifacts for Graph 32, interface 20, compiler 32 and artifact 39.

Accepted source is **`ccbed8cf132a633a81182d62192031d38b03b9ca`**, tree
`af7d52353bcc7019fed0e52915dd83ffbadaec28`. It reached remote main by an ordinary
fast-forward from `7285431327104ea539e74858064ec735d71216be`; independent Git and
GitHub reads confirmed the exact commit, tree and ancestry. Original integration
evidence is [.artifacts/owned-nominals/delivery/integration.json](../.artifacts/owned-nominals/delivery/integration.json).
This status update is a reporting descendant, not a replacement tested source.

The selected worktree remains
`/home/coder/workspace/lkjscript-generalized-sequences-20261011`, branch
`work/nominal-owned-records-20261011`. Keep it and its ignored evidence through
release closure. No local build or acceptance process remains active.

The accepted standard meaning is
`rev_6818785d8919028a69659b24df495b2d7a42f2676a95fd1aa0ae143a32623013`,
with **2,800 live owners, 291 compiler units and 116 tests**. Required native packs,
transport, artifact, embedded pins and maintained consumer artifacts are tracked.
The journal retains its accepted meaning and exact historical standard selection.

## Fresh source acceptance

The corrected source passed **26/26 fresh full gates** in **28m23.5s**, with stable
tracked inputs, **zero reused gates and zero unrun gates**. Workspace results were
**2,455 passed, 0 failed, 29 ignored**, plus two separately filtered passing child
probes. All five nominal public witnesses and all five FIFO behavior witnesses
passed, including detached execution, private representations, complete layout and
phantom-argument admission, borrowed reads, transfer, traps, budgets and joined
cleanup. The continuation codec now matches discovery and explicitly rejects its
predecessor. Formatting, clippy, complete source/reference tests, release lifecycle,
all six external behavioral owners, generated documentation and maintained
package/artifact comparisons passed.

Receipt:
`verification_e49147fb91bb8fb98bda8f0b2593be5fc47ad3f665cb77e707a35ebea5caaa28`,
at [.artifacts/lkjscript-dev/check/1791696537485078079-1099956-0/receipt.json](../.artifacts/lkjscript-dev/check/1791696537485078079-1099956-0/receipt.json).
The immutable checker SHA-256 is
`74646bdf8013f9148dc9bb4db381d52e5b5df41f712a774e6626eeb271d5a260`.
Selection, executable, output, exit status and counted outcomes remain in
[full-second](../.artifacts/owned-nominals/full-second/accepted-summary.json).

The first full run of `fbe7a54378ec5c4a3957f683d2f3e9820ae52a37` remains a genuine
failure: **23/26 gates**, stable inputs, **2,452 passed, 3 failed, 29 ignored**.
Stale discovery/artifact assertions, the HTTP projection expectation and service
artifact hash pin were corrected before the new run. Its original receipt is
`verification_fd60498bf7fc72bd13d0b23a805abd894fd93dff1dae172aad39783917649f30`,
at [.artifacts/lkjscript-dev/check/1791694354031048775-938064-0/receipt.json](../.artifacts/lkjscript-dev/check/1791694354031048775-938064-0/receipt.json).
[Earlier development evidence](https://github.com/lkjsxc/lkjscript/blob/ccbed8cf132a633a81182d62192031d38b03b9ca/docs/status.md)
retains the focused checks, failed originals and cancelled fourth build separately.

## FIFO cost evidence

The [matched-profile report](../examples/owned-fifo/costs/README.md#observed-results-2026-10-11)
retains **432 executions across 18 workloads**, seven measured samples per lane
and one warmup. Every result, joined-cleanup and unchanged-hash check passed.
Nominal/current-structural median invocation ratios were **0.8596–0.9893**;
current-structural/predecessor ratios were **0.9123–1.0638**, including unfavorable
results. Modeled value work and allocation charges were unchanged; the nominal
lane saved **9,488 modeled metadata bytes and 362 derivation steps** per preparation.
These are exact-host observations, not allocator traffic, portable speedups or API
billing evidence.

Originals are [cost-sixth/measurement-1791693814286](../.artifacts/owned-nominals/cost-sixth/measurement-1791693814286/summary.json).
Both copied executables used matched workspace release profiles and features:
current SHA-256 `1d95cb7a214b9228be1832a5e86b428cd024b0c5d0a97b40515d78c4566c1657`,
predecessor `daf81df9850ad7db14c71fb61f9b113d13ad3eafcd2c944b4adaf9a7bcf8190b`.
Those cost measurements remain bound to their original bytes; the later
continuation correction receives separate source acceptance. The retained
[fifth run](../.artifacts/owned-nominals/cost-fifth/measurement-1791692906183/summary.json)
has a cross-generation build-profile confound and is not a generation timing proof.

## Finalized candidate pending

Candidate [38116583723/1](https://github.com/lkjsxc/lkjscript/actions/runs/38116583723)
was dispatched once at **2026-10-11 05:59:37 UTC**. Product and controller source
both select accepted commit `ccbed8cf132a633a81182d62192031d38b03b9ca` on main,
workflow **343592338**, event `workflow_dispatch`, repository and head repository
`lkjsxc/lkjscript`. Its initial observed state is **in progress**, with the
`Accept final candidate` job running. No finalized-byte acceptance or publication
is claimed. Raw service responses, exact selection and dispatch evidence are in
[delivery](../.artifacts/owned-nominals/delivery/candidate-selection.json).

Next: inspect that exact run and attempt. Require generation-8 **candidate_accepted**,
its authenticated original assets/verifier/acceptance inventory and complete
finalized-byte nominal public witnesses. Then follow the [release owner](release.md):
refresh occupancy, select the candidate's exact product commit with an annotated
`v0.1.91` tag, compare/set/read back the existing scoped tag-object control, and
promote the same producer/attempt with unchanged assets. Closure requires
**immutable_published_and_public_verified**. Do not dispatch a duplicate candidate
or treat a green child job as publication. Source is mainline-ready; the remaining
gate is hosted release acceptance and subsequent authorized promotion.

At **05:56:03 UTC**, v0.1.91 tag/release occupancy was empty, including drafts,
and no other Release run was active. Latest observed immutable public release is
**v0.1.89**, release **407859944**. No v0.1.91 tag, scoped-control change or promotion
has occurred. The intermediate v0.1.90 milestone remains deliberately superseded.
Its original failed producer [38103217658/1](https://github.com/lkjsxc/lkjscript/actions/runs/38103217658)
passed **19/20 source gates** before a **3,600-second workspace timeout**; it was
already failed when checked, not cancelled. The nominal correction gives only
that aggregate **7,200 seconds** and the candidate job **360 minutes**, retaining
all coverage and other deadlines. The failed diagnostic artifact **11691130148**
and terminal artifact **11689664511** had observed expiry **2026-10-25**; originals
remain in `.artifacts/data-sequences/delivery/candidate-dispatch/supersession-20261011032814/`.
Their evidence does not attest the nominal source or finalized bytes.
