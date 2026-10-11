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
