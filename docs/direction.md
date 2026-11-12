# Direction

Language design comes first: types, generics, trait evidence, ownership, lifetimes,
effects and concurrency that compose soundly. Reliable agent development is second;
applications are probes. The project eventually owns its compiler, runtime,
library and tools without Rust or Node. No compatibility with past versions is owed.

Work by dependency, then general usefulness, corpus frequency and cost. Compiler
file order is not a roadmap. A census locates missing constructs and blocked
regions; acceptance counts do not establish self-compilation. The milestone is
execution of the real compiler and library graph with declared host inputs.

Resolve uses by identity. Infer and check definitions under declared assumptions,
record choices once, then instantiate and lower the recorded form. Migrate each
responsibility behind compiler::compile while retaining one runnable route. The
independent K checker remains a trust boundary. The separately implemented K machine
is an execution route and a counterexample search for checking and lowering.

Ownership facts need distinct joins: initialization weakens to maybe, possible
reference holders unite, and authority originates only from checked borrows.
Conditional cleanup preserves destructor order. Loop invariants are supplied by
construction and checked by inclusion. Origin-variable call contracts must precede
general reference-bearing mutation. These are designs until language.md says they
are implemented; current equality joins are still conservative.

First remove measured lookup/state-copy exponents and make output available through
World-derived capabilities. Widen inference, patterns, question-mark, closures,
iteration, text and source collections according to dependency and census. Preserve
per-leaf provenance, exact versus bounded sources and reborrow-parent distinctions.
No arbitrary acceptance ceilings or source-depth cutoff. Recursive and explicit
work stacks are both implementation choices, judged on acceptance and measurements.

Keep the Rust bootstrap and Node route explicit. The source compiler boundary takes
declared source/library bytes and returns artifacts or diagnostics. Resource setup
belongs to host adapters. A scoped fixed point requires C0's language pipeline to
emit W1 from the actual closure, then W1 to emit identical W2 with the same inputs,
plus conformance and source-sensitivity checks. Native closure and a rebuild without
Rust/Node are later, separate observations. Do not remove shared-host tools.

Conformance compares bytes and termination; seeded generators search combinations;
forged K reaches the safety gate directly; shared source-sensitivity probes protect
source ownership. Keep reduced failures and exact commands. Measure each available
stage and name fused stages honestly. Unexplained growth is a defect. External test
process envelopes never become language semantics.
