# Resource Profiles: Logical Metering

[Authority](../resource-budget-profiles.md)

## Purpose

Define deterministic source-semantic charges independently of optimizer- and
engine-dependent physical resource metrics.

## Status

**Accepted Target, not Current.** Current VM and compiler meters retain their
existing documented physical/work meanings.

## Logical Events

`logical_aggregate_constructions` counts each semantically evaluated product or
enum construction after child evaluation and before its result becomes
available. The charge is attached to verified HIR/SSA identity and survives
constant folding, inlining, scalar replacement, stack/region placement, object
reuse, allocation elimination, and proof optimization. A deterministic profile
exhausts at the same semantic point with the same structured outcome on the SSA
evaluator, VM, baseline JIT, and proof JIT.

Future logical IO or operation categories require a new closed profile version;
they cannot be hidden inside construction or physical-allocation counters.
Match usefulness and witnesses are deterministic compiler work categories, not
runtime semantic events.

## Physical Metrics

Normal performance modes report actual or explicitly estimated allocation
attempts, bytes, collection work, roots, barriers, runtime calls, code bytes,
and placement/materialization decisions under distinct physical category
identities. Such metrics can differ by target, tier, optimization, and run.
They are observations, not source semantics, equality, object identity, or
portable budget promises.

An initial boxed implementation may produce equal logical and physical counts,
but the counters, units, authorities, and reports remain separate. Ordinary
products and enums expose no identity, tag, address, null sentinel, allocation,
or placement choice.

## Preservation And Evidence

SSA verification and optimization proof checking require a one-to-one retained
logical charge for every reachable semantic construction, preserving control
order and multiplicity. Transformations may merge a charge only with a proof
that the original dynamic event multiplicity remains exact; otherwise they are
rejected. Malformed, missing, duplicated, reordered, or wrong-category charge
metadata fails closed.

Acceptance compares exact charge traces and exhaustion across unoptimized SSA,
optimized SSA, evaluator, VM, forced baseline, and forced optimizing execution,
including branches, loops, recursion, eliminated allocations, traps, and
limits. Physical metrics are checked separately and never substituted for this
differential.
