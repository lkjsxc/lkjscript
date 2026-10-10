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

A Git-object client should create a new branch directly at the prepared candidate
SHA. Branch creation itself is a push event: creating it at the base and then
updating it starts an unnecessary base run before the candidate. For an existing
owned branch, update to a real corrective descendant with `force=false`.
The candidate must include this workflow. Each push selects its immutable event
SHA; the workflow checks out that exact SHA rather than a moving branch name.
It also checks the workflow/source relationship, clean inputs and the commit's
whitespace delta before building.

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

## Local capacity and test concurrency

Check workers, Cargo build jobs and Rust test workers are distinct controls.
`check --jobs` schedules gates; `CARGO_BUILD_JOBS` schedules compilation. Neither
limits the number of independently running libtest cases or threads created inside
a case. Rust's [default test concurrency](https://doc.rust-lang.org/rustc/tests/index.html#--test-threads-num_threads)
uses `available_parallelism`. On a many-core host, simultaneous copied executables
and disposable projects can exhaust the test filesystem even when compilation fits.

Observe both the build filesystem and the actual temporary filesystem before a
large run. Moving a build to tmpfs does not move every test's `/tmp` workspace.
Keep target paths as real directories: a symlinked target is not a reason to weaken
the existing non-symlink artifact-admission boundary. A complete owned tmpfs checkout
can provide build capacity, but its original verification evidence is volatile.

For a capacity-constrained local full run, an explicitly selected subset of the
process's allowed Linux CPUs can bound default test concurrency without editing
source, removing tests or changing gate deadlines. Set `ALLOWED_CPUS` from the
actual allowed CPU list, retain that selection and its observed affinity beside
the original receipt, then run the source-matched immutable verifier:

```sh
CARGO_BUILD_JOBS=2 taskset -c "$ALLOWED_CPUS" "$VERIFIER" check full --fresh --jobs 1 --machine
```

This does not cap test-internal thread creation, establish a memory quota or prove
unrestricted-host performance. The workflow's runner configuration is unchanged.
The [process owner](../tools/lkjscript-dev/src/process.rs) forwards only its approved
child environment. Check that owner before relying on parent-only overrides such
as `CARGO_INCREMENTAL` or `RUST_TEST_THREADS`; merely supplying a variable to the
outer verifier is not evidence that a supervised child received it.

An out-of-space assertion or linker failure is a failed attempt, not a test pass.
Retain its original result. Join owned processes before reclaiming only their
recomputable build outputs, and preserve source, original receipts, logs, retained
executables and consumer evidence. Do not clear a shared temporary directory or
another project's files to obtain capacity. After remediation, accept a new complete
run; do not rename the failed attempt or silently reduce its coverage.

Inactive Rust `target/debug/incremental` data in a task-owned target is
recomputable build state. It is not lkjscript project state under
`derived/compiler`: reclaiming the former after joining its build owners does
not disable or weaken the latter incremental-compilation behavior. Preserve
completed executables and acceptance evidence before any broader Cargo cleanup.

A copied Cargo target can retain a contributor executable compiled for its old
checkout even when Cargo reports the build fresh. Rebuild relocated workspace
packages before selecting that verifier, and confirm that its receipt belongs to
the intended checkout and selects the actual working delta. A passing clean-tree
check of another checkout is not acceptance of the requested source or report.

## Executable observations and compatibility

Check/source receipt contract 7 and cache contract 3 bind each selected command to
an `ExecutableProof`. Its `entry` retains the ordinary file or symlink observation;
`resolved` additionally binds a symlink's regular target bytes and mode. Plain files
have no duplicate target proof. General source FileProofs still do not follow final
symlinks. Both observations use the logical command label, so different immutable
copy directories do not alone defeat valid reuse. The existing digest and cache
owners remain authoritative; no new hash algorithm or acceptance store is added.

On Linux, explicit relative and empty PATH entries resolve against the child cwd,
and lookup skips candidates that the effective identity cannot execute. A bare
command without an explicit PATH fails observation rather than certifying a guessed
search path. Missing programs retain missing observations; a selected broken,
cyclic or nonregular linked target cannot be admitted as executable file evidence.
The runtime reader re-observes this same binding, including consistently rehashed
forgeries, and predecessor check/cache records cannot become current acceptance.
Frozen releases keep their original verifier and contracts.

A gate's fingerprint and fresh execution share one selection. Execution supplies
its absolute selected pathname to the existing process owner while preserving the
declared argv[0], arguments, cwd, environment, logs and bounds. It does not repeat
PATH search. In particular, an executable script whose interpreter is missing is
unavailable; execution must not fall through to a later unobserved candidate.
A missing selection never launches the original bare command as a fallback.
General process helpers keep their ordinary lookup semantics. The selected path
remains an alias rather than being replaced with its canonical target name.

This binds the observed command leaf and selection, not all interpreters, dynamic
libraries, plugins, rustup-selected backends or other transitive dependencies. It
does not pin an inode against concurrent replacement or provide a hostile-filesystem
sandbox. Independent fixtures exercise actual PATH execution, prevention of an
unobserved fallback, declared argv[0], inherited process bounds and a cached
successful program changed to fail behind the same link, with restored valid reuse.

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
run directories, manifests, logs and retained outputs. The separate
`.artifacts/lkjscript-dev/service` directories retain the service owner's original
receipts and diagnostics, including failures before any runner starts; its summary
in the check logs is not a replacement for those originals. Upload runs on failure too;
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
