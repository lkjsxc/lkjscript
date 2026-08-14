# Roadmap

The reset scalar vertical is the only implemented product path. Work advances by evidence gates,
not compatibility milestones.

## Next: agent query ergonomics

Add revision-bound direct uses/dependencies, bounded body slices, semantic-diff queries, exact legal
constructors for holes/operands, and continuation-bound pagination. Acceptance requires repairing a
type error and filling a hole without a whole-workspace dump, with measured request bytes, round
trips, failures, elapsed time, and model tokens. Full recomputation remains the oracle; no query
cache is added without measured repeated cost.

Before that gate closes, add property-generated transaction sequences and bounded artifact/protocol
fuzz smoke targets so compact query work does not outrun boundary hardening.

## Then: structured pure programs

Add direct calls, parameters, structured `if`, and loops through the same SPG, validator, Core IR,
verifier, and interpreter. Agents must not author predecessor lists or phi nodes. Acceptance is
interpreter-correct branching/calls/loops, deterministic dominance rejection, and stack/resource
policy for recursion.

## Later evidence gates

Nominal products/sums and matching precede explicit generics. Pure semantics precede effects and
exact daemon-granted capabilities. Real resources precede ownership, borrow, cleanup, and memory
strategy. Useful libraries precede multi-package dependency closure. Repeated measured work precedes
incrementality or caches. Stable representative interpreter workloads precede evaluation of a mature
baseline native backend. Isolation and daemon concurrency follow real workload pressure.

Native optimization, runtime cells, host I/O, package networks, cross-platform expansion, and
self-hosting are not current work. The active reset prompt in `prompts/202608141640.md` retains the
full campaign gates; it is an execution artifact, not a second semantic specification.
