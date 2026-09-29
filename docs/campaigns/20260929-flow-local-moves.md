# Control-flow-aware ordinary local transfers — 2026-09-29

## Owner request (unchanged)

[https://github.com/lkjsxc/lkjscript](https://github.com/lkjsxc/lkjscript) について進めるようよろしくお願いします。必要であれば大幅なスタイルの変更も許容します。あらゆる判断において、過去ではなく、今のあなたに委ねます。全ての判断をあらかじめ許可します。最も筋の良い方向に進むために、限界まで深く考えてほしい。超長期的な視点からお願いしたい。どれだけ時間がかかっても構いません。

## Selection and entry

This is the language repository, not `lkjstr`. GitHub and the shared Coder checkout
both selected `19230d36bd5bbe272ecc8431d317accfa59313f5` on main. Tracked files were
clean. The two unrelated untracked paths, existing stash/worktrees and running
services remain untouched. Rust 1.98.0 is the pinned toolchain.

The existing terminal-local proof observes only the last lexical read outside
backward-edge intervals. Extend that disposable analysis instead of installing a
second resource or ownership mechanism. This is useful execution groundwork, not
completion of the roadmap's owned-region/scoped-view semantic boundary. No public
syntax, graph/package/artifact encoding, authority or deployment contract changes.
No release or running application is selected for replacement by this campaign.

## Predecessor failure sensitivity

Before production edits, two newly added tests ran against the predecessor code.
`exclusive_branch_reads_are_both_terminal` failed because the first exclusive
branch still used `LoadLocal` instead of `MoveLocal`. The native
`branch_terminal_moves_preserve_both_paths_payload_identity` returned the expected
value but failed on the true branch: its 65 recursive payload boxes had different
addresses. Each focused command exited 101 with one failing test. These observations
are retained in the execution transcript; no missing baseline log is manufactured.

## Implementation boundary

A bounded bitset fixed point computes the lifetime of each current local value.
A store kills the old value; a match payload write kills it only on that edge.
A use may move when no successor path reads that value before replacing it.
Loops with a live carried value still copy. Re-analysis treats old derived moves
as reads and demotes them when later flow requires reuse. Resource borrow/consume
instructions and runtime origin/class checks remain unchanged.

The optional matrix and rewrite plan are admitted before allocation. The precise
pass has an 8 MiB scratch/plan ceiling and one million advisory work steps; an
unfinished proof changes no instruction and falls back to the predecessor linear
proof. Global preparation capacity and cancellation remain separately enforced.
Weak/shared instruction storage is reserved before copy-on-write detachment.

Review of actual VM dispatch caught an over-strong assumption in the uncommitted
candidate: `TailInvoke` can resume its caller after an external invocation. Both the
analysis and its independent oracle now retain that continuation. Exact graph
`TailCall` and `Return` remain exits. This is a candidate correction, not a claim
that predecessor main had this optimization defect.

## Verification obligations and retained evidence

The original 65,536 control-flow fixtures remain. Another 234,256 small fixtures
include local stores and path-specific payload writes; an independent per-read
reachability traversal checks soundness and last-use precision. Explicit cases
cover joined values, loop redefinition versus carried values, multiple bitset words,
implicit transaction access, dynamic-call continuation, stale moves, cancellation,
shared storage, invalid dead operands and advisory fallback. These are analysis
fixtures, not a claim that arbitrary bytecode is an admitted program.

Native graph publication constructs recursive data and exercises both branches.
Both branches must retain all 65 payload boxes and agree with independent canonical
interpretation. Existing forced-copy, cancellation cleanup, foreign-origin and
copied-executable `native_terminal_values` witnesses remain required. Local-read
counters are not byte-copy, total-memory or whole-program speed measurements.

Owned raw logs are under `.artifacts/20260929-flow-local-moves/`. The initial focused
run passed 10 cases; the expanded pre-dispatch-review run passed 16. Final corrected
focused, full-profile and copied-executable results are recorded below only after
observation. Complete acceptance must use stable source, fresh gates and zero reuse;
subsequent reporting commits must not be mislabeled as the tested product source.
