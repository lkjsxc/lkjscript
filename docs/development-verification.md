# Opt-in pre-main source verification

The [Verify workflow](../.github/workflows/verify.yml) executes the existing
`lkjscript-dev check full --fresh --jobs 2 --machine` profile against an exact
candidate commit before mainline integration. It is a source-acceptance execution
path, not a new verifier, a release workflow, or permission to merge.

## Selecting a candidate

Prepare and review one coherent candidate, then normally push its commit to an
owned branch under `verify/`, for example:

```sh
git push origin HEAD:refs/heads/verify/my-change
```

A Git-object client can create a branch at the base commit and then update it to
the candidate with `force=false`. The candidate must include this workflow.
The push selects its immutable event SHA; the workflow checks out that exact SHA
rather than a moving branch name. It also checks the workflow/source relationship,
clean inputs and the commit's whitespace delta before building.

Manual `workflow_dispatch` is an alternative once the workflow is on the default
branch. Select the intended branch or tag through GitHub's ordinary workflow UI
or API; the event SHA remains the accepted source. This document does not imply
that every connected client exposes workflow dispatch.

Ordinary `main` pushes, unrelated topic pushes and pull-request updates do not
trigger this expensive profile. There is no schedule, push/PR duplicate run,
automatic merge or automatic publication. Existing local focused profiles remain
appropriate for iteration and proportionate acceptance. This opt-in path does not
make hosted full acceptance compulsory for every change and does not implement
the broader branch/PR CI item in the roadmap.

## Execution and trust

The runner is `ubuntu-24.04`. Rustup honors `rust-toolchain.toml`, and dependency
resolution/builds use `--locked`. An immutable copy of the candidate's contributor
executable runs the maintained full gate DAG. Two check workers and two Cargo
build jobs bound concurrency; there is no cross-run build or acceptance cache.
The workflow does not substitute clean-worktree `check changed` selection.

The workflow grants only `contents: read`, does not retain checkout credentials,
and supplies no repository secrets or publishing token to candidate commands.
Checkout and artifact actions use the repository's existing exact action pins.
There are no deployments, environments, release commands, ref writes, privileged
`pull_request_target` execution, or downstream publication consumers. Do not add
a privileged artifact consumer without a separately reviewed trust design.

Code capable of changing its own verifier still needs substantive review. A green
workflow is evidence about the inspected candidate, not an independent security
certificate or an authorization to execute arbitrary code with greater authority.

## Acceptance and retained evidence

Success requires the verifier's zero exit status, a nonempty fully passed fresh
`full` summary, and a matching receipt for the event SHA with stable inputs, no
unrun/reused/failed gates and no failure summary. Pipeline failure is not hidden by
`tee`; a missing or inconsistent receipt fails the job. Gate ownership and detailed
semantic admission remain in `tools/lkjscript-dev/src/check/` and
[the verification specification](spec/verification.md), not a second shell test suite.

Each attempt uploads `verify-SOURCE-RUN-ATTEMPT` with 14-day retention. It contains
execution context, environment observations, verifier identity, compact summary,
the original receipt when admitted, and the original `.artifacts/lkjscript-dev/check`
run directories, manifests, logs and retained outputs. Upload runs on failure too;
early runner/setup failure or cancellation can prevent evidence from being
produced or uploaded. Absence is not a pass. GitHub's actual artifact expiry, not
this retention request alone, determines availability. Download needed originals
before expiry. This artifact is diagnostics, not release assets or release authority.

Read the actual workflow run, attempt and job steps, not only commit status.
A skipped/deleted-ref job, pending run or successful upload is not acceptance.
Record the accepted SHA, tree, verifier, profile, result and run/attempt. The full
profile currently owns 26 gates; its Rust registry, rather than a duplicated count
in this workflow, remains authoritative. This host run does not establish a new
static-target release, anonymous acquisition or production deployment.

## Integration, failure and recovery

Refresh `main`, its rules and the candidate's completed checks before a normal
non-forced update or permitted merge. Revalidate material intervening changes.
Then independently read `main` and verify the intended ancestry/content. A passed
candidate branch is not completed integration.

The workflow has a 180-minute job bound. Runs for the same ref do not cancel a
running attempt automatically; GitHub may replace an older pending run when a
new one is queued. Do not push repeatedly as a polling mechanism. Keep the
selected lineage and inspect the actual attempt before retrying an ambiguous
operation. A rerun of the same source is a new attempt, not a relabeled old pass.
For a code defect, commit the correction normally on the owned branch and accept
the corrected source. Preserve failed attempts. If only execution authorization
or runner capacity is missing, report that gate rather than merging untested code.

After confirmed mainline delivery and terminal jobs, delete an owned temporary
branch only after rereading its tip and preserving unique work/evidence. A client
without a safe branch-deletion operation must report the retained branch instead
of rewriting it, force-updating it, or treating cleanup as a reason to hide delivery.
