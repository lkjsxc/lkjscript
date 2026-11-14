# Semantic contracts

`lkjscript test tests/semantics` runs 328 checked-IR acceptance/rejection cases
and 108 value programs. The value programs share functions while retaining 2,902
scalar assertions, including integer boundaries, aggregate layout, native register
preservation, projection/drop interactions and checked runtime operations. Both the
interpreter and the native executable must satisfy each expectation header.

The separate `src/semantic_main.rs` executable imports the production compiler
modules and checks 536 source/parser contracts plus 170 direct compiler/model
contracts. These cover diagnostic recovery and original source locations, include
reader confinement, generic owner identity, library demand, cyclic return contracts,
compact array repetition, sparse facts, stale references, malformed unchecked IR,
20,000 interpreter frames, and sealed native allocation against the independent
raw-memory model. Source bundles retain module boundaries; ordinary standard-library
modules come from the current embedded library. The native gate builds and shards
this executable automatically:

```sh
target/lkjscript build src/semantic_main.rs -o target/semantic-test
target/semantic-test 0 1
```

Arguments select the zero-based shard and worker count (at most eight); an optional
third argument prints unit names before execution. These tests need no Rust or Node
host. Add ordinary source examples with expectation headers to the appropriate
case suite; use the semantic executable for contracts on compiler data structures
which cannot be observed through a source program's result.
