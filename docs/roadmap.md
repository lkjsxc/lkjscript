# Roadmap

Work advances through dependency-closed evidence gates, not compatibility milestones or promises.

## Complete: agent-native repair and structured pure programs

The source-free vertical now lets an external agent use the real generic CLI and daemon to:

- discover the closed runtime schema and submit one structured transaction for multiple functions,
  parameters, calls, conditionals, loops, holes, and entry selection;
- inspect bounded nested repair context, including persistent loop-index and loop-carried identities;
- reject invalid repairs atomically and refine a scalar hole while preserving identity and uses;
- query exact retained-revision semantic diffs and execute ordered scalar arguments;
- run calls, lazy branches, loops, and bounded recursion through one verified Core IR and explicit
  interpreter frame vector;
- restart and query both incomplete and repaired immutable revisions with unchanged IDs;
- observe distinct argument, policy, trap, fuel, and frame failures without losing daemon usability;
- measure real JSON and binary bytes, round trips, artifact size, CLI wall time, repeated product-path
  latency, fresh build/test cost, and deterministic malformed-boundary mutation.

The retained representative example and principal integration produce `main() = 5050`,
`normalize_and_sum(-3) = 0`, and `normalize_and_sum(11) = 55`. Full scans remain the correctness
oracle and implementation; current measurements do not justify an index, cache, journal, database,
async runtime, or second executable route. No model tokens were measured and no token claim is made.

## Next gate: nominal immutable products, sums, and matching

The next evidence gate is the smallest closed extension needed to express ordinary immutable domain
values without introducing effects, ownership, or a general type framework. Acceptance requires:

- code-owned nominal product and closed sum declarations with stable semantic identity;
- exact construction, field projection, variant construction, and exhaustive structured matching;
- deterministic layouts as derived state, with no serialized compiler indexes;
- structured authoring and bounded repair context that do not expose CFG or layout scaffolding;
- direct lowering through the existing Core IR route and explicit-frame interpreter;
- malformed schema, scope, type, match-exhaustiveness, artifact, protocol, and runtime evidence;
- one representative application that demonstrates reduced agent scaffolding and measures bytes,
  round trips, compile/execute cost, and artifact growth;
- a human-first README/status update only after that vertical is verified.

Do not add generics, effects, capabilities, heap ownership, native code, package infrastructure, or a
pattern framework beyond the closed consumers required by this gate.

## Later evidence gates

Explicit generics follow real repeated nominal-type consumers. Pure semantics precede effects and
exact daemon-granted capabilities. Real external resources precede ownership, borrow, cleanup, and
memory strategy. Useful libraries precede multi-package dependency closure. Repeated measured work
precedes incremental validation, indexes, caches, journals, or databases. Stable representative
interpreter workloads precede evaluation of a mature native backend. Isolation and daemon
concurrency follow real workload pressure.

Native optimization, runtime cells, ambient host I/O, package networks, public networking,
cross-platform expansion, and self-hosting are not current work.
