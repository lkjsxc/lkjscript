# Native package dependency planning

Production logical package admission delegates cycle detection and dependency-first
ordering to an ordinary native program. The compiler consumes that program's
checked plan. This replaces the two former host topology algorithms in logical
admission and source reconstruction; it is a compiler boundary takeover, not a new
standalone graph command.

## Authority boundaries

Rust still owns immutable reads, hash and canonical decoding, exact revision and
package unification, complete edge bindings, and source/interface admission. A
private collected-input carrier is explicitly unplanned. It cannot confer
readiness. Every admitted package and edge enters the native proposal, including
nodes that would be unreachable in a general graph request. Positions correspond
to the complete revision-digest order; dependency lists retain authored order.

The maintained [native package planner](../../tools/native-package-plan/README.md)
imports an independently exported complete-outcome checker. The producer computes
components and earliest dependency stages. The checker verifies source outcomes,
partition connectivity, exact edges and stage equations without reusing the
producer's SCC or readiness algorithm. Both still share the native source
validator, runtime and trusted typed compilation boundaries.

The host strictly decodes the complete checked result and maps its positions back
to exact package revisions. It checks coverage and ranges without computing a
replacement topology. Cyclic components refuse package admission. A contradictory
native negative, malformed result or internal failure fails the boundary; no
fallback or partial plan becomes success. The sealed operation-local plan moves
through logical, transport and source admission. It does not grant execution,
parallelism or publication authority.

Source collection may reuse that operation's plan only after rereading the complete
canonical logical closure under its own collection budget and exactly matching the
private root, revision inventory and edge count. Transport, body, interface,
witness and composed-callable admission still run. Mutable reporting projections
cannot supply this proof. Compiler preparation consumes the resulting admitted
source directly; an independently loaded container performs its own admission and
native planning.

## Independent artifact loading

Loading the planner's artifact cannot invoke the planner itself. The strict artifact
reader uses a separate exact-closure entrypoint and retains the host topology
check. Its result has a distinct type and cannot substitute for a production
planned closure. Artifact hash, source, interface and code correspondence checks
remain mandatory. There is no recursion flag, unchecked loader or global bypass.

Native code preparation may be shared after successful independent admission.
Input/result values, cancellation and the dependency plan remain invocation-local.
Cancelled or failed preparation cannot permanently poison later initialization.
Pure invocation explicitly joins its execution owner on failure as well as
success; cleanup failure prevents a successful output from escaping.

CLI observation 46 records the successful invocation's artifact/input/outcome/order
SHA-256 identities and complete package/edge counts. The receipt is constructed
only after native execution and strict result mapping, then carried with the plan
to compilation reporting. Hashes identify evidence; they do not independently
establish behavioral correctness or grant authority.

## Capacity and canonical identity

The native source validator admits at most 10,000 nodes, 10,000 roots and 100,000
edges, matching the compiler closure ceiling. The compiler embedding supplies its
own bounded JSON envelope: 4 MiB, 500,000 aggregate entries, depth 32 and 256 bytes
per string. Ordinary CLI defaults stay independent. Complete strict JSON decoding,
typed raw admission, live memory limits and source-admission budgets all still
apply. A graph count ceiling is not permission for arbitrary package bodies or
unbounded working memory.

Compilation order is stage-major, with revision-digest order within a stage. The
former Kahn pass selected the smallest available digest after each individual
package. These valid orders can differ. Canonical compiler units, dependency
bindings and final artifact inventories remain sorted by their semantic owners;
an exact-byte comparison on a deliberately differing order proves that boundary.
Failure-first observation order is not a canonical artifact identity.

No language, graph, interface, compiler-unit, bytecode or artifact encoding change
is intended. New native assets, host code and candidate obligations require fresh
acceptance. Matched public build measurements include cold preparation, complete
projection/admission, native production and independent checking. The public
compiler reports end-to-end build time; separate phase timings remain unavailable.
Retain warmups and cases slower than the host predecessor. No speed advantage is
presumed.

## Acceptance evidence

The [successful original full run](../../.artifacts/lkjscript-dev/check/1791727236348792345-2336981-0/receipt.json)
accepts source `d01708a2d089af33e79747c097ca61b5c90d4fd7`, tree
`d08f9a0d64afca0a320d0d7a8b0b93a20f788036`: **29/29** fresh gates, **0 reused**,
**0 unrun**, stable inputs and terminal exit **0** after **2,896.017618526 seconds**.
Receipt SHA-256 is
`c0912b08adfd985f487cad4461006563ffd211c5e3e11929c933a6e592c191c0`.
The [verified summary](../../.artifacts/native-package-planner/full-second-summary.json)
and [71-file inventory](../../.artifacts/native-package-planner/full-second-inventory.json)
retain exact original evidence. This is source acceptance; mainline delivery and
finalized-byte publication remain separate gates. Later docs-only reporting does
not change the receipt's source binding.

The [first full run](../../.artifacts/lkjscript-dev/check/1791722983656536426-2131509-0/receipt.json)
on `d1488367758226e02a3c5522476b7058fd881ce2` failed after 2,866.089739 seconds:
**28/29** fresh gates passed, **0 reused**, **0 unrun**, inputs stable. Only
`workspace_tests` failed, on three reviewed fixtures: library **1,911/1/8 ignored**,
native verifier **8/1**, public CLI **269/1/1 ignored**. Original evidence remains
intact. Test-only corrections preserve standalone input accounting and the current
capacity/negative precedence contract. The
[sixth all-target build](../../.artifacts/native-package-planner/build/selected-sixth.json)
returned exit 0 on corrected source `5a9629ef986ed2dc63b8befbc8ad081a10c04b28`;
[all 18 focused cases](../../.artifacts/native-package-planner/tests/focused-sixth-20261011/summary.json)
pass with 0 failed or ignored before the successful full acceptance above.
The native asset and production behavior remain unchanged by those corrections.

## Observed public build costs

The retained 2026-10-11 comparison uses release-profile 0.1.93 and 0.1.94
executables, identical accepted source clones, one cold compiler-cache warmup per
lane and five alternating measured builds per lane. Its candidate release binary
binds earlier source `77decedc`. The
[release rebuild](../../.artifacts/native-package-planner/build/selected-release-second.json)
on corrected source `5a9629ef` reproduces both CLI and verifier bytes exactly;
no corrected-release timing rerun is claimed. Every invocation starts a new
process; native code preparation is therefore cold even when the project's derived
compiler cache is warm. All complete artifacts match across lanes and repetitions;
the planner also reproduces its maintained artifact. Ten detached result checks
pass with complete expected values and joined cleanup.

| Project | 0.1.93 median | 0.1.94 median | Candidate / predecessor |
| --- | ---: | ---: | ---: |
| Dependency-free identity | 16.060 ms | 231.065 ms | 14.388 |
| Four-package diamond | 26.792 ms | 239.821 ms | 8.951 |
| Nine-package native planner | 9,337.967 ms | 10,336.357 ms | 1.107 |

These are complete public build-process costs, not isolated planner timings.
The candidate is slower in all three observed cases; this decision delegates the
compiler boundary without a speed claim. Owned build/test jobs were
idle, but unrelated Java/server/editor processes remained on the shared host.
Five samples do not establish portable performance, asymptotics, memory costs or
monetary savings. Preserve cold warmups, ranges and all unfavorable samples in the
[original comparison](../../.artifacts/native-package-planner/costs/run-20261011T124526Z/summary.json).
The earlier failed run remains intact: its untimed baseline execution correctly
rejected an absolute artifact path in the harness's deployment descriptor. The
corrected run uses canonical relative paths and the same accepted source roots.

## Thin consumer experiment and asset selection

The 0.1.94 thin-consumer decision retained its existing production asset. The
later maintained BlockRead replacement is a separate language-adoption change;
it does not select the thin variant or renew the costs below. The
[thin-consumer experiment](../../.artifacts/native-package-planner/thin-consumer-experiment/summary.json)
passes **156** native checks and **16** matched functional runs with complete
results and joined cleanup. It reduces compiler units **575 → 543** and artifact
bytes **3,371,783 → 3,272,121**, without replacing the production supplier.

With owned build/test jobs idle, seven paired samples per lane show aggregate
public preparation medians **213.824 → 206.996 ms** for empty input and
**217.872 → 208.483 ms** for the diamond. The roughly **3–4%** lower medians have
overlapping ranges on the shared host;
[retain all samples](../../.artifacts/native-package-planner/thin-consumer-experiment/timing-idle-01/summary.json).
Preparation combines deployment, strict loading, program and input work; it is
neither a compiler phase nor a separate pure-tool loading measurement. No compiler
end-to-end speed claim follows. The thin variant stays experimental, and the
dominant cold preparation cost remains unresolved.
