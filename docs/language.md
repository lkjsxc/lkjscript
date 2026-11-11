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
diagnostics reject unsupported constructs.

Source bundles declare their root, source files and optional source-library root.
The owned loader handles inline modules, file modules (`child.rs` or
`child/mod.rs`) and nested directories. Missing, ambiguous and duplicate inputs
have distinct diagnostics. Source spans and K functions retain file IDs;
diagnostics identify the original file. The host supplies bytes only.

Resolution uses module/declaration identities, lexical scopes, imports/aliases,
qualified `crate`/`self`/`super` paths and prelude priority. Private, `pub(self)`,
`pub(super)`, `pub(crate)` and `pub` visibility is checked for items, construction,
fields, methods and re-exports. Unsupported visibility forms are rejected. Local
declarations shadow prelude items and intrinsic spellings. Aliases preserve
nominal identity; identically spelled sibling declarations remain distinct.

Types and instances use structural keys built from declaration/child-type IDs
and substitutions. Printable names are diagnostic labels with compact child IDs;
they are not expanded semantic identity keys.

Struct/enum fields have declaration order. Scalars and shared references are Copy.
Nominal types opt into Copy with `#[derive(Copy, Clone)]`, whose fields must all be
Copy; other values move even if their fields are scalar. Box and Vec own their
contents. Arrays contain their declared fixed number of elements. Partial moves
can be reinitialized. A type with custom Drop cannot be Copy or partially moved.
Custom Drop runs before owned fields, including on early return/break; temporary
receivers are cleaned at the end of their statement. Source cannot call Drop
directly. Traps terminate the program without cleanup or recovery.

References have local, parameter, or immutable static provenance. Mutable
references are exclusive; fields can be borrowed disjointly. A view aggregate
contains references from at most one dynamic source root, with static leaves
allowed alongside it. A projected result names one
borrowed parameter as its source, using explicit lifetime annotation or Rust
elision (including receiver elision). A result cannot select between unrelated
input sources. Static leaves can share a shorter dynamic source lifetime without
losing their individual provenance. Borrow endings and cleanup are explicit in K.
Whole-array/Vec slice projections, view structs and implicit mutable reborrowing
are implemented.

Every initialized reference leaf has checked source provenance. Inactive enum
alternatives carry explicit absence evidence. Moves, construction, assignment,
cloning and replacement preserve this inventory; return checks its completeness.
Replacement transfers the old value's provenance to its result and installs the
new value's provenance in the destination, including through local borrows.
Reference-field writes recheck the enclosing view's source and exclusivity.

K does not yet express cross-parameter mutation contracts. Writes through an
external parameter may retain the exact symbolic origin of each reference leaf
or clear it, but changing that origin is rejected with `E_BORROW`. The comparison
includes projections, not just the parameter root. This conservative restriction
prevents a callee from silently invalidating the caller's lifetime assumptions;
explicit write contracts remain required for more general view mutation.

## Immutable text

String literals produce shared UTF-8 text views with immutable module-long storage.
The unsized `str` referent is used through `&str`; bare or mutable text is not
implemented. Text provides byte `len`, `is_empty`, `as_bytes` and source-owned
`to_string`. Byte projections preserve the original static or borrowed origin,
including after a local reference carrier is destroyed. No byte mutation or owned
destruction can reach literal storage.

`std::str::from_utf8(&[u8])` validates bytes at the trusted runtime boundary and
returns the source Result with `Ok(&str)` or `Err(Utf8Error)`. The error is an
opaque marker; byte-position/error-length reporting is not implemented. Successful
conversion borrows the original storage. No unchecked source conversion exists.
String keeps its private Vec bytes and implements copying, `push_str`,
`to_string` and `as_str` using source functions and checked views. `as_str`
currently validates on conversion; retained UTF-8 evidence for avoiding that scan
remains future work.

The explicit static-lifetime subset covers wholly static function outputs,
including view aggregates. K records `ReturnSource::Static` and checks every live
reference leaf. Static constraints on inputs, local annotations, fields, aliases,
bounds, or mixed static/dynamic result leaves are named unsupported until per-leaf
lifetime contracts exist. Validation includes unused definitions and dead source.
Ordinary dynamic outputs may contain static literals with their lifetime shortened.

## Generic declarations and traits

Traits describe methods and associated types. Implementations must be unique;
overlap is rejected. Full implementation signatures, prerequisite/supertrait
bounds, associated types and instantiated arguments are validated. Send, Sync and
Data predicates are derived structurally; Data excludes capabilities. Generic
parameter-flow strongly connected components reject expanding instantiation.
Aliases participate in these checks. Bounds with extra type arguments remain
unsupported. Method calls resolve to concrete function applications.

Generic function, impl-method and trait-default bodies are checked even when
never instantiated. Definition checking reuses production elaboration and K
ownership/type checking under rigid parameter identities and declared trait
contracts. It does not sample concrete parameter types. Bounds, associated-type
relationships, method signatures and explicit method type arguments are checked;
unreachable expressions must still type-check. User traits named Copy, Clone or
Drop do not acquire builtin semantics by spelling. Derived Copy on a generic
layout is conditional on its fields.

The emitted program remains monomorphic. The generic validation context names
declared evidence, but a complete independently replayable generic/witness K
contract is unfinished. Reference-bearing content hidden inside arbitrary type
parameters is not fully modeled at this generic boundary; concrete specializations
still receive full K provenance checks. This is not complete generic lifetime or
external mutation-contract closure.

## K and checking

K is a monomorphic program of explicit types, function signatures, local slots,
places, instructions and structured control. It distinguishes copies, moves,
borrows and drops. A function declares a parameter or immutable static source for
returned views. Literal instructions carry bytes that the K checker validates as
UTF-8, independently of source parsing. Text K serializes those bytes as hexadecimal
and static result contracts as `from static`.
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
arrays, boxes and vector elements. Structural cleanup uses an explicit growable
DFS stack, preserving custom-destructor-before-children and declaration/element
order. Moved fields are skipped. Copy helpers and explicit recursive source calls
still use the engine stack.

Lowering reuses physical Wasm locals across nonoverlapping lifetimes. Additional
live values spill into a growing memory64 frame; address-taken values have stable
frame slots. The physical register pool is an optimization, not an acceptance
limit. Source-local count does not determine the permanent Wasm-local count.

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
removal. String constructors validate bytes and its private representation is
protected across modules. The library loads through ordinary module scopes.
Bootstrap byte provisioning lives in `bridge/library.rs`; a compiler source
bundle supplies its own library files without hidden fallback.
`std::mem::replace` transfers ownership through an exclusive reference. Vec/Box
and memory operations remain trusted intrinsics until their source layer lands.

`compiler::compile(&SourceBundle)` is the source-to-Wasm compiler-core entry.
`compiler::check_bundle` uses the same production checking pipeline. These entries
perform no host I/O. Their existence and input-mutation tests do not establish
self-compilation.

The production unsigned/signed LEB128 encoders live in `src/encoding.rs` and are
used directly by the emitter. That exact source also compiles and executes under
lkjscript, with byte-level boundary vectors and behavior-changing source mutations.
The same production canonical-name helper in `src/names.rs` is used by resolution,
generic validation and elaboration, and executes with the source String library.
Its tests compare complete output bytes and require valid/invalid source mutations
to change behavior or produce an attributed diagnostic. These are executing
compiler components; the full dependency closure and fixed point remain unfinished.

`src/check.rs` is the K checker; `src/k.rs` defines its inputs and checked result.
`src/wasm.rs` contains lowering/emission plus allocation and collection internals.
`bridge/main.rs` supplies bootstrap file/process/command-line access and the trusted Linux main-stack shim;
`bridge/run.mjs` instantiates on Node. `lkjscript trusted` lists these boundaries.

Generated modules import only `lkjscript.trap(code: i64, line: i64)`.
Ordinary source cannot declare host imports or construct capabilities from scalars.
Ambient filesystem, environment and print operations are rejected. World and its
derived capabilities, closures and structured task scopes remain unimplemented.
