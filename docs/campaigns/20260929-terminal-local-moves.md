# Terminal ordinary-local transfers — 2026-09-29

## Mandate and decision

The owner asked to advance `lkjsxc/lkjscript`, delegated present architectural
judgment and requested a long-term direction, including substantial changes where
justified. This is the language repository, not the separately corrected `lkjstr`
Rust implementation/domain request.

Select a small, broadly useful execution improvement before adding more surface:
remove ordinary-value duplication where the current activation provably cannot
read that local again. Keep immutable value semantics and exact resource contracts.
This can remain useful under future regions or different value carriers without
committing the language to pervasive reference counting, borrowing syntax or a
second semantic authority.

## Boundary

`MoveLocal` exists only in disposable prepared instructions. Strictly admitted
artifact code still contains the established `LoadLocal` and use mode. The analysis
runs after effect/type closure and derives instructions for graph code, port
expressions and native tests. No artifact format, kernel rule, package interface,
canonical owner, builtin package or release version changes.

For a given local, select only its last lexical read outside every interval crossed
by a backward jump. If a future read existed, it would either have a later lexical
index (contradicting last-read selection) or require a backward edge crossing the
selected instruction (contradicting interval exclusion). Explicit conditional and
unconditional jumps and variant dispatch participate. Transaction commit bindings
are implicit reads. All instruction variants are classified exhaustively.

The derivation is deliberately conservative: loop-carried locals, some branch-local
last uses and opportunities requiring definition-sensitive liveness stay copies.
It performs linear work in instructions, edges and local slots within existing
preparation work, cancellation and storage bounds. Strong or weak shared storage is
reserved before copy-on-write/detachment. Re-derivation can invalidate an earlier move.

The VM takes the existing checked value from the slot and retains the same prepared
origin and ordinary classification check. Borrow/consume instructions, exact resource
requirements and adapter admission do not change. Failure/cancellation unwind the
same owned invocation state. `local_value_moves` and `local_value_copies` count
ordinary local reads; neither is a copied-byte or allocator counter.

## Evidence design

- `local_moves_tests.rs` independently explores all successor paths for every
  selected move in 65,536 small control-flow inputs. This is an analysis oracle, not
  a claim that arbitrary bytecode is a valid language program. It also distinguishes
  reuse, backward branches/variant edges, implicit transaction reads, resource modes,
  shared-code preservation, repeat derivation, cancellation and capacity rejection.
  A weak-sharing capacity test must reject before detaching or rewriting storage.
- `local_moves_value_tests.rs` authors real recursive nominal data and functions,
  publishes canonical meaning and prepares its artifact. The optimized VM, a
  same-meaning copying control and the independent canonical interpreter must agree.
  The physical witness compares all 65 boxed payload addresses in a 64-wrap value:
  terminal transfer preserves every address; the copying control preserves none.
  Infinite tail transfer is cancelled, joins all owned frames/locals/operands and
  leaves the same prepared program reusable. Foreign prepared values reject before
  executing a local move.
- The literal `tests/fixtures/terminal-values.lkjc` and copied-binary
  `native_terminal_values` test combine generic forwarding, repeated nominal values,
  both branches, captured-callback iteration, unchanged canonical draft round-trip
  and detached artifact use. Both branch choices at depths 0, 1, 16 and 48 run before
  and after removing the owned project, draft and authoring input. All 16 invocations
  compare both retained outputs with independently constructed JSON values.
- Existing workspace and full-profile gates remain responsible for regression
  coverage, including resource suffixes, transactions, native consumers and the
  independent reference path. No live effects are replayed by the new pure tests.

## Development observations

Initial workspace compilation passed. The first focused analysis run passed four
tests but two early VM witnesses incorrectly supplied raw arguments to anonymous
code. The existing entry admission rejected them with `normalized_runtime_type`.
The runtime was not weakened: those test fixtures were replaced with real native
typed functions admitted through canonical publication.

The corrected focused run passed seven tests. The copied public CLI witness then
passed all 16 designed invocations (one Rust test), including source removal.
Subsequent strengthening adds the weak-sharing capacity negative and compares all
boxed payload identities rather than only the outer box. Final acceptance below
must identify the tested source rather than retrospectively relabel these runs.

Preliminary logs are retained in the workspace:
`/tmp/lkjscript-local-moves-check.log`,
`/tmp/lkjscript-local-moves-tests.log`,
`/tmp/lkjscript-local-moves-tests-typed.log`,
`/tmp/lkjscript-local-moves-public.log`,
`/tmp/lkjscript-local-moves-clippy.log` and
`/tmp/lkjscript-local-moves-final-focused.log`.

## Acceptance and integration boundary

The selected final source gate is:

```sh
cargo build --release --locked -p lkjscript-dev
target/release/lkjscript-dev check full --fresh --jobs 2 --machine
```

Retain the exact source, native optimized-executable result and full receipt before
mainline delivery. Full-profile acceptance is not yet claimed in this implementation
record; a following evidence-only update will record observed results.

Inherited catalog-aware site acceptance changes remain in the ancestry and must
participate in the same final source verification. The previously unstaged campaign
appendix was preserved in `1f2bb126`; unrelated untracked files, stashes and worktrees
were left alone. Normal mainline permission and current branch protection are checked
before delivery. No release, service replacement, domain change, credential change,
force push or protection change is part of this slice.

## Remaining questions

This is not general memory borrowing, a region/trait ownership contract, a uniqueness
proof, cyclic-data management or whole-program zero-copy execution. Persistent
collection updates, projections, repeated reads and serialization retain their own
costs. Local-read counts and this physical identity witness do not establish a
throughput percentage. Future work should measure real application allocation and
retention, then compare broader ownership and memory-region designs without treating
this optimization as their semantic foundation.
