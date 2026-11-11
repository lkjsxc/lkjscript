# Language

Generation two uses Rust-compatible source. Integer literal fallback is i32;
annotated and suffixed integer values retain their type. Arithmetic is checked;
explicit wrapping methods will be added by name. Booleans are distinct from
integers. Evaluation proceeds left to right, and logical operators short circuit.

K is a monomorphic program of explicit types, local slots, places, instructions
and structured control. It distinguishes copies, moves, borrows and drops. Its
checker validates type references, signatures, operations and ownership before a
backend can receive a CheckedProgram. Enum payload access requires a tag test.
Each control edge must preserve the declared live owners and loans. The checker
performs no source inference or trait search.

Struct and enum fields occupy ordered slots. Scalar types and shared references
are Copy. Nominal structs/enums opt into Copy with `#[derive(Copy, Clone)]`, and
all their fields must be Copy. Other values move. Boxes and vectors own their
contents; mutable references are exclusive. References may project from a single
borrowed input. View aggregates need explicit source tracking before acceptance.

The first backend emits memory64 WebAssembly. Integer and pointer slots are i64;
f64 values use their bit representation in slots and canonicalize NaNs after
arithmetic. Collection indexing checks bounds. Traps stop execution with a code
and source line. There is no unwinding. An allocator reuses freed blocks, and
explicit drop recursively releases owned fields and collection elements.

The implementation recognizes assertions and explicit panics as syntax. Ambient
I/O is rejected. World and derived values are the planned capability interface;
closures, structural concurrency predicates and task scopes remain implementation
work until status records passing design cases.

## Trusted modules and host imports

`src/check.rs` is the K checker. `src/wasm.rs` implements memory allocation,
collection layout, instruction lowering and emission. `bridge/main.rs` supplies
bootstrap files, processes and the command line. `bridge/run.mjs` instantiates
modules on Node. `lkjscript trusted` lists these boundaries.

Generated modules currently import `lkjscript.trap(code: i64, line: i64)`.
Ordinary source cannot declare host imports or construct a capability from scalars.
Library and host boundaries will shrink as ordinary source can own those parts.

The source library provides `Option<T>`, `Result<T,E>`, checked UTF-8/byte helpers,
lexicographic vector order for integer/bool elements, and the implemented subset
of `BTreeMap<K,V>` over sorted entries. `std::mem::replace` transfers ownership
through an exclusive reference. Collection operations remain trusted intrinsics
until their memory layer can be expressed by accepted source.

Generic parameter-flow SCCs reject expanding recursion. Trait implementations
must be unique, and instantiated arguments satisfy declared structural or explicit
trait bounds. Generic methods lower to explicit concrete calls. Full witness
values and all unused-implementation signature checks remain implementation work.
