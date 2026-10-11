# Current status

Snapshot: 2026-10-11. Accepted meaning, source verification, mainline delivery,
finalized executable acceptance and public availability are separate boundaries.

## Nominal owned records in development

The nominal owned-record source is selected in
`/home/coder/workspace/lkjscript-generalized-sequences-20261011`, branch
`work/nominal-owned-records-20261011`, based on `72854313`. Local development is
**0.1.91**. It has no completed fresh source acceptance, mainline integration or
finalized-byte acceptance yet.

The implementation adds exact nominal owned records with module-private
representations and adopts the public `Fifo<T>` record in the native standard
FIFO. Public operations preserve queue order, returned custody and source-tied
front reads. The [record specification](spec/owned-records.md),
[FIFO guide](guides/native-owned-fifo.md) and [release notes](releases/v0.1.91.md)
own the semantics and intentional format cut. Five public nominal witnesses and
generation-8 required-inventory admission are implemented; dependency-complete
source acceptance and finalized-byte execution remain gates.

The accepted standard meaning is
`rev_6818785d8919028a69659b24df495b2d7a42f2676a95fd1aa0ae143a32623013`,
with **2,800 live owners**. Its accepted revision is retained in
`.artifacts/owned-nominals/standard-status-after-apply.log`; it has **291 compiler
units**. The fifth transport and seventh artifact are installed, and embedded
exact pins are updated.

Current verification: `public-nominals-fourth.log` passed **5/5** in **137.63s**
through the ninth public harness build (**3m09s**), after two fixture corrections.
The tested `candidate-fourth-lkjscript` SHA-256 is
`5e190171d51618379196d7a21917c03d0a74b7a63618658c28cbc0be7d54ca06`.
`public-fifo-first.log` passed **5/5** in **116.79s**; `focused-third.log` passed
**79/79**, covering all **35** earlier library failures. The new `BorrowCall`
unused-binding producer-edge regression subsequently passed in the first full
run's library suite (**1,838 passed, 0 failed, 8 ignored**). Fmt-fifth and clippy-fourth
passed, including the final public harness corrections. Maintained checks passed
**116 standard / 44 journal / 79 guides / 62 policy**, with production/reference
equality and the same exact installed artifacts, at `standard-check-eighth.log`,
`journal-check-second.log`, `guides-check-fifth.log` and `policy-check-fifth.log`.
Documentation generation and verification passed for **all eight pages** at
`docs-generate-second.log` and `docs-verify-second.log`. All named originals are
in `.artifacts/owned-nominals/`. The final development-tool build
`dev-build-first` passed in **4m59s**, and `release-build-first` passed in
**5m04s**; full source acceptance remains pending.

The sixth FIFO cost run completed **432 executions across 18 workloads and
seven measured samples per lane/workload**, with every result, joined-cleanup and
unchanged-hash check true. Originals are
[cost-sixth/measurement-1791693814286](../.artifacts/owned-nominals/cost-sixth/measurement-1791693814286/summary.json).
Both executables used checked matching workspace release profiles and features.
Current release SHA-256 is
`1d95cb7a214b9228be1832a5e86b428cd024b0c5d0a97b40515d78c4566c1657`;
the retained release baseline from accepted source
`99232e109758f646c88d1f36f59d93bf1e1b8b38` is
`daf81df9850ad7db14c71fb61f9b113d13ad3eafcd2c944b4adaf9a7bcf8190b`,
bound at
[baseline-workspace-release-binding.json](../.artifacts/owned-nominals/baseline-workspace-release-binding.json).
The [cost report](../examples/owned-fifo/costs/README.md#observed-results-2026-10-11)
retains all favorable and unfavorable results: nominal/structural median
invocation ratios were **0.8596–0.9893**, while current structural/predecessor
ratios were **0.9123–1.0638**. Seven samples on one host do not establish portable
speedups or an isolated internal cause. Modeled value work and allocation charges
were unchanged; the cost report owns the per-preparation metadata observations.
The [fifth run](../.artifacts/owned-nominals/cost-fifth/measurement-1791692906183/summary.json)
remains a functional and same-current-binary witness; its cross-generation timing
is confounded by predecessor release level **3** versus current test level **2**.

Failed and cancelled originals remain retained separately: the
[earlier focused run](../.artifacts/owned-nominals/focused-second.log),
[full library run](../.artifacts/owned-nominals/library-first.log),
[first](../.artifacts/owned-nominals/public-nominals-first.log),
[second](../.artifacts/owned-nominals/public-nominals-second.log) and
[third public nominal runs](../.artifacts/owned-nominals/public-nominals-third.log),
and [initial cost preparation](../.artifacts/owned-nominals/cost-prepare-first.log).
The fourth build remains cancelled with exit **130**, not failed.

The first frozen full run at source
`fbe7a54378ec5c4a3957f683d2f3e9820ae52a37` completed in **28m12s** with stable
inputs, **23/26 fresh gates**, zero reuse and zero unrun gates. Its failed receipt is
`verification_fd60498bf7fc72bd13d0b23a805abd894fd93dff1dae172aad39783917649f30`,
at `.artifacts/lkjscript-dev/check/1791694354031048775-938064-0/receipt.json`;
selection, checker and aggregate outcomes are retained in
`.artifacts/owned-nominals/full-first/`. Workspace outcomes were **2,452 passed,
3 failed, 29 ignored**, plus two separately filtered passing child probes.
All five nominal and all five FIFO public tests passed. The failures were stale
service artifact/discovery inventory assertions, the distributed HTTP projection
expectation and the service artifact hash pin. The current correction updates
those exact expectations and aligns continuation magic/envelope/payload version 3
with discovery, with explicit predecessor rejection. No failed proof is relabelled
as acceptance; the correction awaits a new full run.

Next: freeze all tracked inputs for fresh **26-gate full acceptance**, including full
workspace tests, clippy and independent-loader checks. No current source
acceptance, integration or publication is claimed. Integrate the accepted exact
source normally and independently verify remote main before dispatching a
generation-8 candidate; finalized-byte acceptance and unchanged-asset promotion
remain separate gates. Do not edit tracked status during the frozen proof.

## Accepted baseline and superseded intermediate release

The preceding generalized-sequence source
`99232e109758f646c88d1f36f59d93bf1e1b8b38`, tree
`57eef72efdb6678d5e0f71a474ce7af1bc0f801d`, passed fresh **26/26 gates** with stable
inputs, zero reuse and zero unrun gates: **2,372 passed, 0 failed, 29 ignored**,
plus two separately filtered passing child probes. Its receipt is
`verification_40eb2d40302de639cc773b1f85aa426bd51b37fc23d09f1b1e09bb1642b6c5de`,
at `.artifacts/lkjscript-dev/check/1791680514015518854-558889-0/receipt.json`;
immutable checker and result summary remain in
`.artifacts/data-sequences/full-99232e10/`. This proof belongs to that baseline,
not the nominal implementation.

The accepted baseline reached remote main through [PR #14](https://github.com/lkjsxc/lkjscript/pull/14),
merge `d8e62956791fd649e0a6bfd2722067375cacce83`, with the same accepted tree and
exact-source ancestry. Integration proof is
`.artifacts/data-sequences/delivery/integration.json`. Latest observed remote main
is the reporting descendant `7285431327104ea539e74858064ec735d71216be`.

Original v0.1.90 candidate
[38103217658/1](https://github.com/lkjsxc/lkjscript/actions/runs/38103217658)
was dispatched once at **2026-10-11 01:51:09 UTC**, selecting exact
product/controller source `d8e62956791fd649e0a6bfd2722067375cacce83`. A fresh
identity check at **03:28:14 UTC** found it already completed with **failure**;
no cancellation request was needed. Hosted source acceptance passed **19/20 fresh
gates**, then failed with a **workspace_tests timeout** at **03:21:45 UTC**.
The receipt is
`verification_99e504c868c666ac25fe89e27bea6ad318d6c4abe6fc63d7db3b20e2f60a3ac1`,
at `.artifacts/lkjscript-dev/check/1791684008776929279-6526-0/receipt.json`
inside its original diagnostic artifact. The invocation's terminal is
**incomplete**, with candidate failure and publication/public verification
skipped; no finalized candidate assets or verifier were accepted.

The retained workspace log shows **13m05s cold compilation**, **21m29s preceding
test binaries**, then **200/260 public-CLI outcomes with no FAILED outcomes**
before its 3,600-second aggregate limit. Output was still being written in the
timeout minute. The same accepted baseline completed locally in **22m01s**;
the hosted evidence establishes incomplete progress, not a proved deadlock or
semantic test failure. Current nominal source raises only this aggregate to
**7,200 seconds** and the hosted candidate outer limit to **360 minutes**,
preserving every test and other owner deadline. New acceptance remains required.

This generation-7 intermediate milestone is **deliberately superseded by the
nominal v0.1.91 milestone**. No v0.1.90 publication is planned. Its genuine failed
history is retained separately and does not attest the new source. Raw service
responses, the original job log and the service-digest-verified terminal ZIP/JSON
are in
`.artifacts/data-sequences/delivery/candidate-dispatch/supersession-20261011032814/`.
The diagnostic artifact is **11691130148**; the terminal artifact is
**11689664511**, both with observed service expiry **2026-10-25**. The [release
owner](release.md) governs the future generation-8 candidate and unchanged-asset
promotion after nominal source acceptance and mainline integration.

Latest observed immutable public release is **v0.1.89**, release **407859944**.
The v0.1.90 and v0.1.91 tags/releases were both unoccupied at **03:16:58 UTC**.
No v0.1.90 or v0.1.91 public acceptance or publication is claimed. Retain the
sequence/FIFO worktrees and unique ignored evidence through the nominal handoff;
deployment and operational data remain separately owned.
