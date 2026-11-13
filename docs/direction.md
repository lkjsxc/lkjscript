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
are implemented; initialization joins now weaken, possible loan holders unite, and loop backedges remain conservative.

World-derived output and an independent K machine are available. Prioritize
front-end width over new trusted rules: patterns and question-mark first, then
source-library text and collections, then closures and iteration behind multi-root
view and origin-variable call contracts. Remove measured lookup and loan scans
as those constructs multiply their use. Preserve
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

## Owned execution

Checked K lowers once to an explicit target-independent layout and operation
form. Copy, destruction and equality glue are functions in that form. Natural
scalar widths and one source runtime supply allocation, growth and UTF-8; raw
memory operations belong only to the runtime module. WebAssembly and x86-64
encoders consume the same lowered operations. The native Linux ELF uses no libc;
system calls derive from capabilities. Stack and heap exhaustion have defined
traps and must not require retries or an engine-imposed extent.

Native conformance, generators and the requested capacity probes now agree.
The stack/worker bridge is deleted and the routine gate uses native and K.
WebAssembly remains only as a thin encoder. The million-function chain,
non-tail recursion and large list are observations, not acceptance ceilings.

After compiler closure, retain the previous native compiler as a committed seed
that builds its successor. The hand-over observation is three builds of the same
declared inputs producing identical bytes: the Rust-built compiler, the compiler
running on K, and the self-built native compiler. This is a future contract, not
a claim that compiler closure or the fixed point has been reached.

File access should derive from a World directory capability restricted beneath
its root. Compiler inputs remain an explicit declared set. The trust ledger in
status tracks what must be correct for accepted concrete programs to be safe;
physical size is an audit aid rather than a proof.
