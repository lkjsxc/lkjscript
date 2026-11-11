# Direction

Language design comes first: types, generics, trait evidence, ownership, views,
capabilities and structured concurrency that compose soundly. Reliable agent
editing comes second. Applications probe the language. The project eventually
owns its compiler, runtime, library and tools without Rust or Node.

Generation two is the working foundation. Rust-compatible source resolves to
explicit meaning and checked K; the current backend emits memory64 WebAssembly.
These are selected strategies, not permanent owner requirements. A replacement
needs a causal reason, meaningful evidence and a working transition. Source and
history live in Git. The generation-one store, services and formats are frozen.

The near-term foundation is complete reference provenance, real module/declaration
identity and privacy, structural interning, and generic-definition contracts. Each
change must strengthen a semantic invariant, close a production compiler dependency,
remove an incidental constraint, or replace obsolete machinery. Preserve meaningful
operations while repairing their rules. Missing provenance must never grant access.

Use the same production compiler/library components under rustc and lkjscript.
The first ownership boundary is a complete source-to-artifact compiler core taking
declared source/library inputs and returning artifacts or diagnostics through a
small host-buffer adapter. Compile and execute real components, then close the
whole production dependency graph. Accepted syntax alone is not self-hosting.

A scoped WebAssembly fixed point means C0's language pipeline emits W1 from the
exact compiler closure and W1, executed on the declared host, emits identical W2.
Conformance and behavior-changing source mutations accompany the equality. Node
and the host adapter may still be required at that milestone. Later close the CLI,
memory, host capability and execution dependencies deliberately. Choose an owned
machine or native route from dependencies and measurements; an interpreter is not
a mandatory predecessor to native code. Native N1=N2 and an isolated rebuild
without Rust/Node establish different claims, and both need explicit bootstrap
inputs. Never remove shared-host tools to simulate independence.

Remove arbitrary ceilings by choosing growing representations and suitable
algorithms. Keep semantic invariants, target encodings and external operational
controls distinct. Address practical emission, engine loading, execution and
cleanup failures alongside compiler work; check-only scale is not execution
capacity. External Rust and Wasm implementations remain useful test oracles.

The implemented-language specification is `language.md`; working state, concrete
next actions and evidence are in `status.md`; actual representation and engine
constraints are in `limits.md`. Pending designs remain visibly pending.
