# lkjscript

Build a coherent language with composable types, generics, traits, ownership,
lifetimes, effects and concurrency. The project should own its compiler, runtime,
tools and persistence, eventually without Rust. Language design comes first;
reliable development by agents comes next. Applications are useful probes.

## Continue

Read `docs/status.md`, inspect Git and live jobs, then work on the recorded next
action. Search before reading; read relevant ranges. `docs/direction.md` states
outcomes and hypotheses; specs describe implemented behavior. Git is the history.
Keep status short: what works, what is broken, bootstrap, next action.

Use English. The owner delegates architectural and implementation decisions,
including breaking changes and deletions. Keep only current formats. Work inside
this checkout; preserve unrelated work and running deployments. Never force-push,
rewrite history, change credentials/protection settings, buy services or publish
releases. Integrate verified changes directly into main, or merge a PR if required.

## Engineering

Remove arbitrary admission and execution ceilings and their unused policy,
accounting and saturation machinery. Fix the whole lifecycle, including equality,
encoding, cloning, destruction, failure and cancellation. Keep semantic checks,
checked encoded arithmetic, cycle detection and useful caller cancellation.
Chunk sizes, caches and worker counts tune speed; they do not define validity.
Use physical memory and measured work to choose representations and stacks.
Report observed allocation/I/O failures and process termination honestly.

Keep one production implementation per responsibility. Differential oracles belong
in explicit tests. Ordinary execution runs once. Keep a working source-to-executable
bootstrap path while replacing its parts. Validate new compiler components on the
Rust evaluator before extending native support. Fixed points need independent
semantic comparisons and source mutations that change emitted behavior.

Preserve type, visibility, identity, evaluation-order, ownership and effect rules.
Admit external bytes before effects. An admitted revision-bound handle carries its
proof; internal consumers should not re-admit unchanged meaning. Publication keeps
stale-base detection, atomic visibility, crash safety and inspectable HEAD after a
lost response. Cancellation and rejection cannot partly publish.

## Feedback and delivery

Use `bash tools/check.sh` for the routine suite; use its focused and slow modes
as documented in `docs/spec/verification.md`. Run one broad routine suite per
completed change. Run relevant bootstrap/scale controls explicitly when their
owners change. Say what ran, failed and was skipped; do not create evidence files,
seals, receipts or session diaries. Keep `.artifacts/` untracked and disposable;
check for live jobs and unique inputs before cleaning it. Share the Cargo target.

Keep a lint when it earns its place. Safe Rust and no-Python remain current house
choices. Stage intended files, review the diff, commit small and push often.
Rewrite stale guidance when it conflicts with these outcomes. Every simplification
must make a working capability easier to build or maintain.
