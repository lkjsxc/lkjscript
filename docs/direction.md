# Direction

## Outcomes

lkjscript is a language whose types, generics, traits, ownership, lifetimes,
effects and concurrency compose soundly. Reliable development by agents is the
second priority. The project should own its compiler, runtime, tooling and
persistence, eventually without Rust. Applications exercise the language; they
are not the measure of success.

Remove project-defined capacity ceilings with the machinery that exists solely to
enforce them. Valid input grows until the machine or a useful outer caller control
stops it. Improve representations and algorithms across the complete lifecycle.
Use more of the machine when that is the simplest complete solution. Preserve
semantic checking, checked encoded extents, cycle detection and cancellation.

Reduce duplicated production implementations and repeated development work.
Ordinary execution runs once. Keep compact differential oracles in tests and an
explicit slow tier for fixed points and scale. Prefer changes that shorten the
next meaningful compiler edit. Compatibility with past formats is unnecessary.

## Selected route

1. Keep entry documents short and the feedback loop fast.
2. Remove complete capacity families, starting with execution stacks and depth.
3. Build idiomatic compiler components on the evaluator over current meaning and
   exact dependencies. Keep publication history in the store.
4. Consolidate the semantic core while extending the language features those
   components actually need.
5. Grow the new compiler path and retire each predecessor after corpus comparison,
   behavior-changing source mutations and its own fixed point.
6. Replace Rust responsibilities in complete steps: compiler, kernel, store, CLI.

A named bootstrap route must work throughout. Rust evaluation is the bridge for
language constructs not yet supported by a native path. Temporary Rust is justified
when it unlocks a named larger deletion. Record the source and target of any
self-built seed and how a clean checkout obtains and replaces it.

## Design hypotheses — targets to test

- A small explicit typed core checked without inference or witness search. An
  untrusted elaborator supplies explicit selections, substitutions and evidence.
- One data universe: records, variants, sequences and matching. Affinity, copying,
  moving and borrowing are type properties and operation modes. Test opaque affine
  scalar records, affine generic collections, borrowed matches, captures, recursive
  types and cleanup on early exit.
- Ownership may carry capability effects and structured task borrowing. Test
  `examples/owned-effects`, `examples/parallel-work`, escaping callbacks and stored
  task callables before removing effect rows or requirement parameters. Captured
  capabilities and latent callable effects must remain explicit and checked.
- Parameter conventions and result projections may express borrows; stored views
  require region brands. Test returned and selected views, captured borrows,
  disjoint mutable projections and structured tasks.
- Static witness values may replace separate contract machinery. Work through an
  associated type, prerequisite, generic adapter and ambiguous selection first.
- Revision-bound validated handles should carry admission proof without repeated
  validation. New bytes, changed revisions and untrusted transforms require checks.
- Dense conventional text may reduce edit tokens and retries. Keep exact identity
  in a machine-maintained declaration/member table and derive local structure.
  Retire committed packs only after every maintained project round-trips to the
  same semantic state.

Choose the smallest mechanism that passes its cases. These hypotheses are not
claims about current behavior. Further targets include syscall-based native
executables, demand-driven compiler queries and diagnostics with applicable deltas.
