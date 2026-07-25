# Callable Linux x86-64 Baseline JIT Cycle: Platform And Tier Decision

[Authority](../callable-baseline-jit.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Platform And Tier Decision

Linux x86-64 is the only acceptance platform for this cycle. The runtime
hierarchy delivered by this cycle is:

```text
reference VM
  -> synchronous whole-function baseline JIT
```

The baseline tier is non-speculative. It does not require guards,
deoptimization, background compilation, compiler threads, loop OSR, a
persistent profile, or a persistent native-code cache. Offline PGO was rejected
for this implementation cycle; the later [Execution
Portfolio](../../execution/execution-portfolio.md) reclassifies optional explicit local PGO and
a content-addressed cache without making either Current. Later loop-triggered compilation and OSR,
proof-based optimization,
and only then justified guarded specialization remain separate cycles.

Generated-code execution speed is the primary performance objective after
exact semantics and safety. Compilation latency, memory, code size, and binary
size remain measured secondary costs. Emission or disassembly without a native
call is not completion.
## Explicit Executable Main

An executable root contains exactly one `main` form. Imported files contain
only `import`, function `def`, and `product` declarations and may not contain
`main`, top-level `do`, or runtime value definitions. The canonical form is:

```text
main/
sig/
->
Unit
/sig
body-expression
/main
```

The signature has no parameters and exactly one declared return type. The body
is one expression and its exact type must equal the declared return type.
Script arguments remain available through the typed `arg` operation. Functions
and products may be declared in the executable root, but all runtime effects
begin in `main`. A missing main, duplicate main, or imported main is a compile
error. Top-level `do` and arbitrary runtime global initialization are removed,
not retained as compatibility forms.
## Function-Local Mutation

The canonical mutable lexical form binds one explicitly typed local:

```text
var/
name/
state
/name
type/
Product EditorState
/type
initial-expression
body-expression
/var
```

The initializer is evaluated before the binding is in scope. The binding is in
scope only in the body. The initializer type must exactly equal the declared
type. `var` may occur in any function or main and returns the body's type.
Nested `var` forms express multiple mutable locals.

`set/ name value /set` resolves the nearest lexical binding in the same
function invocation. Its target must be a `var`; parameters and immutable
`let` bindings are rejected. Resolution never crosses a function boundary and
never targets a global. The value type must exactly equal the binding type and
`set` returns Unit. Mutable closure capture remains forbidden; this cycle does
not introduce cells or implicit references.

Top-level `def` declares only an immutable function. Program-global mutable
values, global stores, and arbitrary runtime global initializers are removed.
Function installation may remain implementation metadata, but it is not
source-observable mutable state.

lkjedit, terminal state, Brainfuck, and every other source-global workload move
state into immutable nominal products. Helpers receive state explicitly and
return a replacement product. Executable main owns the evolving value in one
or a small bounded number of local vars. A mutable object stored in a product
retains that object's existing explicit mutation semantics; it does not make
the product itself mutable.
## Inferred Effects

The compiler, not source declarations, computes deterministic function effect
summaries over resolved callees. The compact lattice contains at least:

- allocation;
- memory read;
- memory write;
- local mutation;
- host IO;
- possible trap;
- possible explicit exit or other process outcome change;
- possible divergence.

Each expression contributes its direct effects. A direct call contributes its
arguments, call operation, and resolved callee summary. Canonical generic
instantiations map to their resolved callee identity. Indirect or unresolved
call provenance remains conservatively all-effects.

Summaries are the least monotone fixed point over the finite bitset. Recursive
strongly connected components converge together and retain divergence without
inventing unrelated allocation, IO, trap, or write effects. Function and
summary order is stable by compiler identity, independent of hash iteration or
declaration order. No native movement or dead-code decision may drop an effect
absent proof.
## Validated Execution Boundary

Raw mutable bytecode construction is distinct from executable bytecode. One
validator consumes a raw chunk and returns an opaque `ValidatedChunk` or a
validation error. Ordinary VM, disassembly, SSA/native linkage, and tiering
paths cannot construct or execute a validated value without that validator.
Compiler-produced chunks pass through the same boundary as directly
constructed malformed test chunks.

Validation decodes every reachable and unreachable byte before effects occur
and checks at least:

- known, non-retired opcodes and complete operands;
- bytecode, table, metadata, and per-function size limits;
- constant, prototype/function, local, function-metadata, product, field, and
  any remaining implementation-global indexes;
- product identity, descriptors, field categories, and duplicate or
  inconsistent metadata;
- zero captures while source closures cannot capture;
- function arity, local count, main entry, and return shape;
- jump bounds and instruction-boundary targets;
- stack underflow and equal stack shape at CFG joins;
- definite local initialization on every path;
- statically checkable Option, Result, list, buffer, handle, and operation
  categories.

Validation failure is not a language trap and no bytecode executes before it is
reported. The VM and native tiers consume the same validated semantic object;
there is no backend-specific weaker validator.
## Structured Execution Outcomes

The execution core never terminates the Rust process. VM and native execution
use one structured terminal model with distinct categories equivalent to:

```text
Returned(value)
Exited(code)
Trapped(trap)
DeadlineExceeded
ResourceLimitExceeded(kind)
HostFailure(error)
```

A language `Option`, language `Result`, validation error, language trap,
explicit exit, deadline, resource limit, and host-service failure are never
conflated. Ordinary recoverable `sys-*` failures remain language Results when
the operation contract says so. Generated code and the VM never call
`std::process::exit`.

The outer execution owner stops execution, releases or transfers runtime
resources, restores terminal state exactly once, and flushes output according
to the language contract before the CLI translates a completed outcome into a
process exit status. Cleanup failure is reported without erasing the prior
outcome. A later VM instance is independent of earlier exit, trap, deadline, or
resource exhaustion.

Instruction fuel, stack/frame depth, aggregate heap/allocation, handle count,
output, bytecode, and native-code resources receive explicit bounded
configuration. A wall deadline is checked at calls, loop backedges,
allocations, host calls, polls, and tier transitions; blocking operations must
honor remaining time or report that the deadline contract is unsupported.
Forced native execution cannot claim support for a missing required limit.
