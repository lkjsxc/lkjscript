# Direction

Language design comes first: types, generics, witnesses, ownership, views,
capabilities and structured concurrency that compose soundly. Reliable agent
editing comes second. Applications are probes. The project eventually owns its
compiler, runtime, tools and persistence without a Rust toolchain.

Rust-compatible text elaborates to explicit K. One checker verifies K. Backends
trust checked K and emit a WebAssembly subset or native code. Source and history
live in Git. No semantic-store migration, resource accounting or repeated runtime
proofs carry over from generation one.

Implement each stage once, keep it readable, and use rustc and existing
WebAssembly engines as independent test implementations. Unsupported features get
named diagnostics. Never call a pending design case a semantic pass. Remove
arbitrary ceilings by choosing representations that grow; document remaining
representation and engine constraints in limits.md.

The route is a runnable compiler, an owned standard library, self-check and a
WebAssembly fixed point, our machine, then a native fixed point. Cargo and Node
are named bootstrap bridges. Exact behavior and remaining work live in status;
the implemented language has one specification in language.md.
