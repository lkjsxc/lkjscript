# Independent native consumer exercise — success

On 2026-09-22, an independent consumer created and edited a small packing-advice program through public lkjscript operations. Both versions passed four graph tests, built artifacts, and ran successfully. The native request was authored directly from product discovery and the supplied generated guides. No implementation sources, existing authoring fixtures, Orbloam, semantic generators, or private storage edits were used.

The program defines a nominal `Parcel` record with `fragile` and `chilled` Boolean fields, a pure `recommend` function returning packing advice, four graph tests covering the Boolean truth table, a `PackingTools` component, and a `packing-advice` command target. It starts from the minimal template and has no dependencies.

| Step | Observed result | Evidence |
|---|---|---|
| Discover capabilities and create minimal authority | Success | `transcripts/001-*.log` through `009-*.log` |
| Plan and apply original native request | Prepared, then accepted | `010-create-plan.log`, `011-create-apply.log`; `original-inputs/create-packing.lkjc` |
| Check, build, run initial behavior | 4 passed, 0 failed, differential equal; build 20,891 bytes; fragile+chilled yields `Use an insulated carton.` | `012-initial-check.log` through `014-initial-run.log` |
| Remove original request and outputs from mounted work area | Original request and outputs moved to unmounted sibling directories; no top-level files remained in `work` | `sequester-initial-files.sh`, `transcripts/015-sequester.log` |
| Locate module/target and publicly draft both | Canonical draft produced, 3,402 bytes | `016-find-module.log` through `018-draft.log`; `work/draft-untouched.lkjc` |
| Plan untouched draft | `outcome=unchanged`, `semantic-change=false`, `owner-recreation=false`, `publication=none`; revision unchanged | `019-plan-untouched.log`, `020-status-after-untouched.log` |
| Edit reconstructed behavior and matching test together | Fragile+chilled now requests a padded insulated carton | `work/edit-packing.lkjc`, `transcripts/021-edit.diff` |
| Plan and apply edited request | Prepared, then accepted; two stored declaration changes plus expression replacement/retirement | `022-edit-plan.log`, `024-edit-apply.log`; `work/edit-plan.txt` |
| Check, build, run edited behavior | 4 passed, 0 failed, differential equal; build 21,960 bytes; same arguments yield `Use a padded insulated carton.` | `025-edited-check.log` through `027-edited-run.log` |
| Final status | Current validation valid | `029-final-status.log` |

All transcript filenames in the table are under `transcripts/`. Every product invocation retains the full literal Docker command, output and exit status. Run arguments were exactly `[{"fragile":true,"chilled":true}]` in both runs. Both artifacts are retained: `original-outputs/packing-initial.lkja` and `work/packing-edited.lkja`. Runs used the project command route; deployment-descriptor execution was not exercised.

The initial accepted revision was `rev_b115ebd38b2b7ca7360bf7d1c082864ebe3efe0762bb32930d23da37ec41b71c`. Drafting and untouched planning retained that revision. The edited and final revision is `rev_a975ee70134d6d68cede147335b977eb6e25f85890f3717d700dcfea2818ad6e`.

One consumer misunderstanding occurred during plan review. A host assertion assumed every non-expression `logical-plan.owner` row represented changed stored content. It reported seven owners and five unexpected entries (exit 1); the plan also includes affected dependents whose `before` and `after` object values are identical. Comparing those public values showed exactly the intended function and graph test changed. The incorrect assertion ran inline and did not prevent the later product commands from running because that shell block did not use `set -e`. Its original inline output was observed in the tool transcript; `review-plan-assertions.sh` explicitly replays it against the retained, unchanged plan and then checks the correct condition. Both results are retained in `transcripts/028-plan-assertions.log`. No product command failed, and no compiler knowledge was used to resolve this misunderstanding.

The main authoring reference was `candidate-dev-5/guides/change-grammar.md`, including its native creation, draft, type, component, port, and target grammar. Also consulted: generated operations and function-definition guides, a deployment-guide search, and public `capabilities` output. The native creation syntax and nominal JSON arguments both worked on the first attempt. No functional usability blocker was found in this bounded exercise.

Product identity: supplied `candidate-dev-5/lkjscript`, version `0.1.39`, SHA-256 `e45b354856426ed8241a86fcacb402b323b1752c38359c76e6922fd9c5d341f7`, capability digest `311d1f6f2ce937279b819418191149aac91c0bf4ee7f2253b3bd98b3c36dabab`. Compiler source/revision was deliberately not inspected.

Every product process ran with a cleared environment in the existing Docker image `debian@sha256:70509c95d1857a3704c0a5d92ee2e0adac95f612a9386889d70760bfd7c1ebba`, network disabled, read-only container filesystem, no capabilities, and user 1001:1001. The only task mounts were the supplied binary read-only, generated guides read-only, and this exercise's `work` directory writable. The original-input and transcript directories were never mounted. No publishing credentials were passed. Each foreground container exited and was removed; there are no retained running jobs. Only this owned consumer directory was changed.
