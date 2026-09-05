# Pure graph tail execution

The standard graph-owned fold and independently authored recursive programs need bounded live
control state beyond the existing call-depth admission. Raising limits or adding a Rust fold
would leave the language guarantee incomplete.

Preparation derives terminal-only continuations from strict bytecode in one iterative linear
instruction/edge traversal. Only return and unconditional jumps to terminal-only continuations
qualify; cycles and pending work do not. Exact canonical declaration kind and effect distinguish
pure graph functions from constants, externals, and empty-requirement tasks. Internal dispatch
may replace a pure activation after arguments and exact call validation, preserving its original
return continuation, operand base, and ancestor transactions. Artifact and compiler-unit bytes
remain unchanged.

The canonical reference independently derives tail context from expression owners and trampolines
between activations after lexical state has unwound. It does not inspect normalized eligibility,
compiler control flow, or VM transfer helpers. Public arithmetic expectations and bounded-stack
ownership observations discriminate shared mistakes and ordinary frame growth.

The guarantee is constant control space, with unchanged cumulative budgets. It does not extend to
task-tail transfer, affine resource return, callbacks into graph execution, or constant heap use.
Reconsider the mechanism only with independent evidence that an alternative preserves the complete
eligibility domain, exact evaluation order, failure accounting, and task/transaction ownership.
