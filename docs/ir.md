# lkjscript IR reference

The lkjscript IR is an explicit, monomorphic program of types, functions, local
slots, places and structured instructions. Producers may construct arbitrary IR.
Only `check::check` or `check::check_all` can produce the `CheckedProgram`
accepted by a backend. Parsing text does not grant that authority.

## Text and identity

Files use `.lkjir` and begin with the exact first line `lkjscript-ir 1`.
The revision describes the text format. Unsupported or absent headers are errors.
`lkjscript ir SOURCE` writes it; `check`, `build`, `run` and `test` accept it.

Types and functions are indexed from zero in declaration order. A function's
locals are indexed from zero; its first `params` locals are parameters.
Names identify diagnostics and function entry selection. Type names do not grant
authority or change a type's shape. Function names must be unique.

Outside quoted words, whitespace separates tokens and `#` starts a comment.
Quoted words use JSON-style string escapes. Instructions begin with
`@line:column`; `@line` means column 1. A function may specify its file index.
Text-format errors use E_IR_TEXT; checker errors retain instruction locations.

A complete scalar program:

```text
lkjscript-ir 1
type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
fn "probe" 0 3
local "answer" 3 let
@1:1 int 0 42
@1:1 return 0
endfn
```

The first seven types must match this primitive prefix exactly, including names.

## Types

| Text after `type NAME` | Meaning and checked constraints |
| --- | --- |
| `unit`, `bool`, `char`, `f64` | Unit, booleans, Unicode scalar values, binary64. |
| `int BITS signed\|unsigned` | BITS is 8, 16, 32 or 64. |
| `struct copy\|move NAME TYPE ...` | Ordered named fields. A copy declaration requires every field to be Copy. |
| `enum copy\|move` ... `endtype` | Ordered alternatives written `variant NAME FIELD TYPE ...`; Copy requires Copy payloads. |
| `ref shared\|mut TYPE` | Shared or exclusive reference to a sized value, slice or text. |
| `slice shared\|mut TYPE` | Unsized sequence shape; values require a reference type. |
| `array TYPE LENGTH` | Fixed-length owned elements. |
| `vec TYPE`, `box TYPE` | Owned growable sequence or owned indirection. |
| `str` | Unsized immutable UTF-8 text; values require a shared reference. |
| `world`, `stdout`, `stdin`, `stderr`, `directory`, `file` | Affine authority-bearing values. |

All type indices must exist. Recursive by-value layouts reject; introduce Box or
another indirection. Unsized text cannot be stored by value or as a container
element. Mutable text references reject. Box, Vec and owned arrays cannot
contain views, including nested reference-bearing aggregates. Struct/enum views
may live in locals, parameters and results.

Scalars and shared references are Copy. Exclusive references and capability
values are affine. Aggregate Copy declarations are independently checked.
A custom destructor is declared `destructor TYPE FUNCTION`; it belongs to one
non-Copy struct/enum and has signature `fn(&mut T) -> ()`. Ordinary calls cannot
invoke destructor functions directly.

## Functions and places

```text
fn NAME PARAMETER_COUNT RESULT_TYPE [file FILE_INDEX] [test]
local NAME TYPE let|mut
...
endfn
```

A `test` function takes no parameters and returns unit. Control cannot fall
through endfn. Slice and str cannot be direct local/result types.
There are no source overloads, trait lookup, inference or indirect source calls.

A place is `LOCAL` followed by zero or more projections:

| Projection | Meaning |
| --- | --- |
| `deref` | Referent of a reference or owned box. |
| `field INDEX` | Struct field. |
| `variant TAG FIELD` | Enum payload field, requiring a checked active-tag fact. |
| `index LOCAL` | Sequence element at the initialized index local. |

Projection types, mutability and index types are checked. Indexed access retains
a runtime bounds check. Moving or dropping through a borrow or indexed
collection rejects; transfer a value through `replace` instead.

## Operations

DST, SRC, LEFT, RIGHT and operands in argument lists are local indices.
PLACE follows the grammar above. Constructors, calculations, assignments,
calls and intrinsics consume operand locals even when their types are Copy;
use `read ... copy` to retain a value. Destinations must be uninitialized.

| Text after the source location | Effect |
| --- | --- |
| `unit DST` | Initialize unit. |
| `int DST VALUE` | Initialize an integer, bool (0/1), or valid Unicode scalar. The textual value is a signed 64-bit bit pattern. |
| `float DST BITS` | Initialize f64 from an unsigned 64-bit pattern; NaNs use the canonical pattern. |
| `text DST HEX_BYTES` | Immutable module-lifetime str or exact-length u8-array reference; str bytes must be UTF-8. |
| `read DST copy\|move PLACE` | Copy a Copy value or transfer ownership from PLACE. |
| `borrow DST shared\|mut PLACE` | Create a checked reference and its loan. |
| `slice-borrow DST shared\|mut PLACE` | Borrow array/vector storage as a slice. |
| `view-range DST shared\|mut exclusive\|inclusive START END PLACE` | Consume usize bounds and borrow a range; preserve source/parent loans. |
| `array DST ELEMENT ...` | Build an array of exactly the declared length. |
| `struct DST FIELD ...` | Build fields in declaration order. |
| `variant DST TAG FIELD ...` | Build the selected alternative and establish its tag. |
| `assign SRC PLACE` | Transfer SRC into writable storage. An existing owner needs explicit destruction first. |
| `drop PLACE` | Destroy initialized storage; structural cleanup skips moved children. |
| `drop-if PLACE` | Destroy only if this exact owned storage is initialized; projected ancestors must exist. |
| `unary DST OP SRC` | Typed scalar negation or complement. |
| `binary DST OP LEFT RIGHT` | Typed scalar arithmetic, bit operations, comparison or wrapping arithmetic. |
| `call DST FUNCTION ARG ...` | Direct call with exact parameter/result types. |
| `tag DST SRC` | Read an enum discriminant without consuming the enum. |
| `tag-test DST SRC TAG` | Produce a boolean tag witness for a local enum. |
| `tag-test-place DST TAG PLACE` | Produce a witness for a projected enum place. |
| `intrinsic DST NAME ARG ...` | Apply one of the checked signatures below. |
| `if CONDITION`, `else`, `end` | Structured boolean branch. |
| `loop`, `end` | Structured repetition. |
| `break`, `continue` | Transfer to the nearest loop's exit or backedge. |
| `break-to DEPTH`, `continue-to DEPTH` | Select an enclosing loop; zero is nearest, and only loops count. |
| `return SRC` | Transfer the result after checking remaining owners and loans. |
| `trap MESSAGE` | Terminate with a located trap; there is no unwinding. |

Unary `-` supports signed integers and f64; `!` supports integers and bool.
Integer binary operations are `+ - * / % & | ^ << >>`, comparisons, and
`wrapping_add wrapping_sub wrapping_mul`. F64 supports arithmetic and
comparisons; char supports comparisons; bool supports equality and logical/bit
operators. Operands have the same IR type. Comparisons produce bool.
Ordinary integer arithmetic, division and shifts trap on invalid operations.
Wrapping operations compute modulo the declared width.

The intrinsic signatures are checked in `src/check.rs` and `src/check/io.rs`:

| Intrinsics | Contract |
| --- | --- |
| `array_repeat` | Copy one element into a declared array of length at least two. |
| `vec_new vec_len vec_push vec_pop` | Typed array/slice/vector length and vector ownership operations; mutation requires an exclusive vector reference. |
| `box_new` | Consume one value into Box of that exact type. |
| `replace` | Exchange through an exclusive reference and return the old owned value. |
| `clone` | Copy a Copy referent; general source Clone dispatch is an ordinary call. |
| `str_len str_as_bytes str_from_utf8` | Byte length, immutable byte view, or UTF-8 validation preserving the source lifetime. |
| `char_from_u32` | Return None or Some(valid Unicode scalar). |
| `cast` | Checked scalar type combinations; references and capabilities cannot be cast. |
| `assert` | Consume bool, returning unit or trapping. |
| `world_stdout world_stdin world_stderr` | Derive the named capability through an exclusive World reference. |
| `world_arg_count world_arg_len world_arg_copy` | Read/copy byte arguments through World. |
| `world_exit_status` | Set the requested u8 process status through World. |
| `world_directory directory_open` | Acquire a Directory from World or a File beneath a Directory. |
| `stdin_read file_read file_write` | Use an exclusive authority reference and a byte slice; return i64 count or negative errno. |
| `stdout_write stderr_write` | Write an immutable byte slice through the corresponding exclusive authority reference. |

Unary, binary and intrinsic names form closed vocabularies. Unknown spellings
reject with E_IR_TEXT; wrong typed signatures reject at the checker. A type name
or integer constant cannot forge authority. Raw memory operations are available only through
the separately checked and sealed runtime interface.

## Ownership, loans and control

Parameters start initialized; other locals start absent. Reads require definite
initialization. Moves consume storage. Reinitialization cannot overwrite a live
owner. A function cannot return while leaking an owned value. Custom destruction
requires all fields it may inspect to remain initialized.

Shared loans prohibit conflicting writes/moves; exclusive loans also prohibit
conflicting reads and loans. Reborrows retain parent dependencies.
Provenance tracks each reference leaf, its possible source bounds and its storage
identity. A copied descriptor may outlive the carrier from which it was copied
only while its actual referent and required parents remain valid.

Branches intersect definite initialization and union possible initialization,
moved fields and reference origins. A later operation must be valid for every
possible incoming state. Conditional cleanup occurs where `drop-if` appears.
A loop backedge must restore its entry initialization and loan state.
No fixed-point iteration is required.

Enum payload access requires a tag witness that still identifies the same
storage. Replacing an enum or its witness invalidates that fact. Moves or writes
to an index used by a witness also invalidate it. Joins retain only common facts.
Instruction shapes and type indices are validated even on unreachable paths.

## Returned views

A view result declares its allowed origins, and the checker verifies the body:

```text
fn NAME PARAMETER_COUNT RESULT_TYPE from PARAMETER [PROJECTION ...]
fn NAME PARAMETER_COUNT RESULT_TYPE from static
return-leaf 0 [RESULT_PROJECTION ...] exact INPUT [or INPUT ...]
return-leaf 0 [RESULT_PROJECTION ...] bound INPUT [or INPUT ...]
return-leaf 0 [RESULT_PROJECTION ...] static
```

An INPUT is a parameter index followed by projections selecting its stored
reference leaf. Per-leaf contracts cover every result reference leaf exactly
once. `exact` preserves the selected referent; `bound` permits a descendant.
`static` permits immutable module-lifetime storage. A call transports the
actual arguments' provenance through the declared contract. A local owner or
temporary reference carrier cannot become an escaping view's origin.

## Backend contract and trust

A backend may assume valid indices and operation shapes, matched types,
initialized reads, legal moves/destruction, checked loans, valid return origins,
and authority values supplied through verified capability operations.
It must preserve value, destruction, trap and effect behavior. Admission does
not remove language-defined runtime checks: bounds, arithmetic/division, UTF-8
boundaries, required tag tests and allocation/stack failure remain.

The trusted implementation comprises the IR checker, native selection/encoding,
sealed allocator/runtime and capability bindings, plus the operating system and
hardware they use. The frontend and future IR optimizers are untrusted producers.
The independent interpreter checks behavior without sharing checker or lowering
logic; it is a differential oracle. Optimization output is rechecked and rejected
output discarded, then native behavior is compared with interpretation of the
original IR.
