# Contributor descendant inventory correction — 2026-09-30

## Selection and entry state

The owner requested continued development of `lkjsxc/lkjscript`, delegated design
choices and normal mainline integration, and prioritized the strongest long-term
direction over preservation of incidental styles. This continuation chooses a
demonstrated delivery blocker before expanding the already-unreleased language work.
It does not replace the owned-region/scoped-view direction or add a new language feature.

Entry main and origin were `8c7b36f0fec79f2ef287bddafea22aee0351b1ad`.
The existing unrelated untracked campaign and `change_review.rs`, the old history
stash, other worktrees and running applications remain outside this change.
Rust 1.98.0 and Cargo 1.98.0 were observed in the existing Linux Coder workspace.

## Original failed candidate

The original v0.1.60 producer [36605595167/1](https://github.com/lkjsxc/lkjscript/actions/runs/36605595167)
used source `89241cd33c88d27ef1b6f774e24ab9b0726ff093`. It ended in failure on
2026-09-30 at 03:23:21 JST, rather than remaining in progress.
Its source profile passed 19 of 20 selected gates freshly, with no reuse.
`workspace_tests` failed with `owned descendant traversal exhausted`; publication
and public verification were skipped and the terminal remained `incomplete`.
The verifier interrupted the workspace command during public CLI tests. This is
not evidence that every interrupted application test passed or that the candidate
was accepted.

Authenticated diagnostics artifact `11052529513` and its original receipt/logs
are retained under `.artifacts/20260930-descendant-inventory/failed-candidate-complete/`.
The receipt is below
`lkjscript/lkjscript/.artifacts/lkjscript-dev/check/1790703593360944789-6523-0/`.
An earlier interrupted download remains separately in `failed-candidate/`; it was
not overwritten or used as a complete artifact. Terminal artifact `11052909885`
was observed separately. Both artifacts were unexpired at that observation.

## Demonstrated mechanism and correction

The previous Linux sampler retained every historically observed child identity.
It also enqueued an already-known child again when reaching the same child through
its parent, charging repeated discovery paths against traversal capacity.
A long sequence of small process trees could therefore exhaust the verifier's
inventory without thousands of simultaneously live children.

Refresh the known inventory before each discovery. Remove only confirmed absent,
terminated or replaced PID/start identities. A failed observation retains that
identity for cleanup and returns the error; it is never converted into absence or
successful completion. A replacement process is not adopted merely because it has
a formerly known PID. Live sampled branches remain roots after reparenting.

The traversal owner counts each queued/visited PID once. It exhausts a connected
tree depth-first before falling back to retained roots, so a previously known child
does not silently reset its actual connected depth to zero. The existing
4,096 distinct per-sample process and depth-64 bounds remain; thread/read bounds,
exact PID/start checks, pidfd signaling, group ownership, EOF handling and joined
cleanup are unchanged. Confirmed historical departures are not a lifetime quota.

These are contributor-verifier changes only. Product implementation, language
semantics, application data, dependency locks and running processes are unchanged.
Sampling still cannot account for a separate-session descendant created and
reparented entirely between observations; this is not hostile-process containment.

## Focused evidence

Before changing production code, three new regressions were run against the
predecessor implementation. All three failed: 4,096 confirmed-absent retained
identities exhausted admission, a recycled unrelated identity was not retired,
and a reaped child remained alongside the live owned branch. The literal original
output is retained as `predecessor-tests.log` in the campaign artifact directory.

After correction, `cargo test --locked -p lkjscript-dev process:: -- --nocapture`
passed all 32 selected process tests, with no failed or ignored tests, in 0.79 s
(test execution only). `process-tests.log` retains the output.
The nine new tests additionally cover 8,192 simulated serial departures, failed
observations and recovery, duplicate edges at the exact process bound, one-over
process/depth rejection, and disconnected roots with later children.
The existing real separate-group/reparenting, output/EOF, cancellation, timeout,
reaping and healthy-recovery fixtures remain active.

The large-count inventory and traversal cases are deterministic controlled states,
not measurements of 4,096 concurrent real processes. Small real child fixtures
retain finite fallbacks and a kill/reap guard. No runtime speedup or memory-use
measurement is claimed.

## Acceptance and delivery

Focused results above precede complete source acceptance. Record the exact frozen
source, full-profile result and normal remote-main integration below after they
are observed. The failed original producer remains failed and must not be promoted.
A corrected verifier needs a new exact-source candidate, not a relabeling of the
old source or its evidence. Public/latest remains v0.1.55 until separate final
archive acceptance and unchanged-asset publication succeed.
