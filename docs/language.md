# Language

This file describes the implemented generation-two subset. Status names the
unimplemented parts of the intended language.

## Source and values

The surface is Rust-compatible. Integer literal fallback is i32; annotations and
suffixes retain distinct types, including usize/u64 and isize/i64. Arithmetic is
checked. Signed minimum literals are accepted; out-of-range integer and non-finite
floating literals are rejected. Explicit casts follow Rust conversion behavior.
Named wrapping operations remain source-library work. Booleans are distinct from integers. Expressions evaluate
left to right; logical operators short circuit. There is no unwinding.

Functions, structs, enums, constants, type aliases, generic declarations, trait
implementations, matches, if, while, loop and for are implemented. For-loop
continue advances the iterator; an inclusive maximum endpoint does not overflow.
Assertions, panic and the implemented vec forms are fixed syntax. Named
diagnostics reject unsupported constructs. Source modules are not loaded yet.

Struct/enum fields have declaration order. Scalars and shared references are Copy.
Nominal types opt into Copy with `#[derive(Copy, Clone)]`, whose fields must all be
Copy; other values move even if their fields are scalar. Box and Vec own their
contents. Arrays contain their declared fixed number of elements. Partial moves
can be reinitialized. A type with custom Drop cannot be Copy or partially moved.
Custom Drop runs before owned fields, including on early return/break; temporary
receivers are cleaned at the end of their statement. Source cannot call Drop
directly. Traps terminate the program without cleanup or recovery.

References are local/parameter conventions and projected results. Mutable
references are exclusive; fields can be borrowed disjointly. A view aggregate
contains references from exactly one source root. A projected result names one
borrowed parameter as its source, using explicit lifetime annotation or Rust
elision (including receiver elision). A result cannot select between unrelated
input sources. Borrow endings and cleanup are explicit in K. Whole-array/Vec slice
projections, view structs and implicit mutable reborrowing are implemented.

## Generic declarations and traits

Traits describe methods and associated types. Implementations must be unique;
overlap is rejected. Full implementation signatures, prerequisite/supertrait
bounds, associated types and instantiated arguments are validated. Send, Sync and
Data predicates are derived structurally; Data excludes capabilities. Generic
parameter-flow strongly connected components reject expanding instantiation.
Aliases participate in these checks. Bounds with extra type arguments remain
unsupported. Method calls resolve to concrete function applications.

The current first vertical instantiates generics and resolves witnesses before K.
K therefore checks concrete programs, not yet the generic/witness contracts.
Moving those explicit contracts into K and instantiating after the gate remains
necessary to complete the intended single-checker architecture.

## K and checking

K is a monomorphic program of explicit types, function signatures, local slots,
places, instructions and structured control. It distinguishes copies, moves,
borrows and drops. A function declares the input source of a returned view.
Concrete trait method selection is represented by an explicit function call.
K has a readable/writable text form accepted by `check` for direct tests.

The checker validates all type references, signatures and operation shapes,
including dead code, then checks ownership through each function. Variant payload
access needs a proven tag. A loop back edge must reproduce entry ownership state.
Return and break edges must carry correct live owners and loans; no fixed-point
iteration is used. Local storage or a local reference carrier cannot become the
source of an escaping view. The checker performs no source inference or trait
search. Only CheckedProgram reaches the backend; later stages rely on that gate.

## Memory and execution

The backend emits memory64 WebAssembly. Integer and pointer slots are i64; f64
uses its bit representation in slots and canonical NaNs after arithmetic.
Collection indexing checks bounds. Traps report a code and source line. The
reusable first-fit allocator and structural/custom drop release owned fields,
arrays, boxes and vector elements. Recursive helpers use the engine stack.

Structs use field slots; enums add a tag. Vec and array storage has data, length
and capacity slots. Whole-storage slice projections borrow that header without a
separate allocation. Moved heap slots are cleared so partial cleanup skips them.
These layouts are implementation details, not an exposed source API.

Generated functions export as `fn:<K-name>`, avoiding a collision with the
`memory` export. Node is currently the only execution path for generated code.
The host prints scalar entry results; floating display formatting is Node's.
No generated engine implements a step counter or per-operation allowance.

## Source library, trusted modules and imports

The source library implements Option/Result, byte operations, UTF-8 validation,
String, vector ordering for integer/bool elements, and part of the BTreeMap API.
BTreeMap uses sorted Vec entries with logarithmic lookup and linear insertion or
removal. Constructors validate String bytes; full privacy enforcement is pending.
`std::mem::replace` transfers ownership through an exclusive reference. Vec/Box
and memory operations remain trusted intrinsics until their source layer lands.

`src/check.rs` is the K checker; `src/k.rs` defines its inputs and checked result.
`src/wasm.rs` contains lowering/emission plus allocation and collection internals.
`bridge/main.rs` supplies bootstrap file/process/command-line access and the trusted Linux main-stack shim;
`bridge/run.mjs` instantiates on Node. `lkjscript trusted` lists these boundaries.

Generated modules import only `lkjscript.trap(code: i64, line: i64)`.
Ordinary source cannot declare host imports or construct capabilities from scalars.
Ambient filesystem, environment and print operations are rejected. World and its
derived capabilities, closures and structured task scopes remain unimplemented.
