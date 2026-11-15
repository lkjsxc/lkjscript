# lkjscript IR reference

The lkjscript IR is an explicit, monomorphic program of types, functions, local
slots, places and structured instructions. Producers may construct arbitrary IR.
Only the checker can produce the `CheckedProgram` accepted by a backend.
Parsing text, producing transformed IR and loading store bytes grant no authority.

## Text and identity

Files use `.lkjir` and begin with the exact first line `lkjscript-ir 1`.
The revision describes the text format. Unsupported or absent headers are errors.
`lkjscript ir SOURCE` writes it; `check`, `build`, `run`, `test` and `authority` accept it.

Types and functions are indexed from zero in declaration order. A function's
locals are indexed from zero; its first `params` locals are parameters.
Names identify diagnostics and selection of entry functions and grant parameters.
Type names do not grant authority or change a type's shape. Function names must be unique.

Outside quoted words, whitespace separates tokens and `#` starts a comment.
Quoted words use JSON-style string escapes. Instructions begin with
`@line:column`; `@line` means column 1. A function may specify its file index.
An instruction may override that file with `@file/line:column`, preserving the
original location when an IR transformation copies code between functions.
Text-format errors use E_IR_TEXT; checker errors retain instruction locations.

The complete [scalar program](../tests/reference/scalar.lkjir) is an executable
reference case: its expectation is 42 and the native gate runs both routes.

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
| `world`, `stdout`, `stdin`, `stderr`, `directory`, `file`, `process`, `clock`, `child`, `listener`, `connector`, `readiness`, `entropy`, `stop` | Affine authority-bearing values. |

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
Binary64 negation flips only the sign bit; binary arithmetic canonicalizes NaNs.
Bit conversions preserve every bit.

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
| `f64_from_bits f64_to_bits` | Bit-preserving conversion between unsigned 64-bit integers and binary64, including NaN payloads. |
| `assert` | Consume bool, returning unit or trapping. |
| `world_stdout world_stdin world_stderr world_process world_clock world_entropy` | Derive the named capability through an exclusive World reference; world_entropy returns owned entropy directly. |
| `entropy_fill` | `(&mut entropy, &mut [u8]) -> Result<(), i64>`: fill the entire slice from the kernel random source. |
| `world_arg_count world_arg_len world_arg_copy` | Read/copy byte arguments through World. |
| `world_exit_status` | Set the requested u8 process status through World. |
| `world_directory directory_open directory_open_directory` | Acquire a root Directory from World, or a File/Directory beneath an existing Directory. |
| `world_file` | World plus u8 selector: 0 anonymous file, 1/2/3 owned stdin/stdout/stderr, 4 running executable. Return Result<File, i64>; closed standard streams produce closed Files. |
| `world_listen` | `(&mut world, &[u8]) -> Result<listener, i64>`: bind and listen on an exact IPv4 socket address. |
| `listener_accept listener_accept_nodelay` | `(&mut listener) -> Result<file, i64>`: accept an owned TCP byte stream; the second disables Nagle before returning it, closing on configuration failure. |
| `world_connector` | `(&mut world, &[u8]) -> Result<connector, i64>`: authorize one exact numeric IPv4 TCP destination. |
| `connector_connect` | `(&mut connector) -> Result<file, i64>`: connect only to its authorized destination. |
| `listener_address` | `(&mut listener, &mut [u8]) -> i64`: write its local socket address; zero or negative errno. |
| `world_readiness` | `(&mut world) -> Result<readiness, i64>`: own a kernel readiness set. |
| `world_stop` | `(&mut world) -> Result<stop, i64>`: claim SIGTERM and own a nonblocking signal descriptor. |
| `stop_requested` | `(&mut stop) -> Result<bool, i64>`: consume a pending SIGTERM or return false; true remains sticky for this Stop. |
| `stop_target` | `(&mut stop) -> Result<u32, i64>`: identify the execution's explicit SIGTERM endpoint. |
| `file_set_nonblocking listener_set_nonblocking` | Exclusive handle and bool; set O_NONBLOCK on its open file description, returning Result<(), i64>. |
| `readiness_watch_file` | `(&mut readiness, &mut file, usize token, u8 interest) -> Result<(), i64>`: add or update a watch; interest is 0..3. |
| `readiness_watch_listener` | `(&mut readiness, &mut listener, usize token) -> Result<(), i64>`: add or update an acceptance watch. |
| `readiness_watch_stop` | `(&mut readiness, &mut stop, usize token) -> Result<(), i64>`: add or update a watch for unread SIGTERM. |
| `readiness_unwatch_file readiness_unwatch_listener readiness_unwatch_stop` | Exclusive readiness and corresponding exclusive handle; remove its watch, returning Result<(), i64>. |
| `readiness_wait` | `(&mut readiness, &mut clock, usize deadline) -> Result<Vec<(usize,u8)>, i64>`: owned token/event tuples by an absolute monotonic nanosecond deadline. |
| `directory_create_new directory_open_append directory_create_private` | `(&mut directory, &[u8] path) -> Result<file, i64>`: constrained exclusive create, create/append, or exclusive private create. The path has exactly one trailing NUL. |
| `directory_sync file_sync` | Exclusive Directory or File respectively; fsync its storage, returning Result<(), i64>. |
| `file_truncate` | `(&mut file, usize length) -> Result<(), i64>`: set the file length without changing its offset. |
| `file_lock_exclusive` | `(&mut file) -> Result<(), i64>`: nonblocking advisory exclusive flock, held until the last duplicate closes. |
| `file_set_private` | `(&mut file) -> Result<(), i64>`: fchmod to exactly 0600. |
| `file_duplicate` | Duplicate an exclusively borrowed File, sharing its offset and preserving closed state. Return Result<File, i64>. |
| `file_readonly` | Reopen an exclusively borrowed File read-only with a fresh offset at zero, preserving closed state. Return Result<File, i64>; no path authority is granted. |
| `file_rewind file_executable directory_rewind` | Seek to byte zero or make a File executable; return zero or negative errno. |
| `directory_read` | Read Linux directory records into mutable bytes; return count or negative errno. |
| `directory_create directory_remove directory_rename` | Mutate NUL-terminated basenames through exclusive Directory references. Reject slash, empty name, dot and dot-dot. Return zero or negative errno. |
| `clock_read` | Clock plus mutable bytes of length at least 16. Write little-endian i64 seconds/nanoseconds from CLOCK_MONOTONIC; return zero or negative errno. |
| `clock_unix_seconds` | `(&mut clock) -> Result<i64, i64>`: signed CLOCK_REALTIME seconds since Unix epoch; negative successful timestamps remain distinct from positive errno. |
| `process_spawn` | Exclusive Process, shared executable File, NUL-separated argv, and four shared Files for stdin/stdout/stderr/fd3. Execute with empty environment; return Result<Child, i64>. |
| `child_wait child_terminate` | Exclusive Child. Wait takes a nonblocking bool and returns zero while running, Linux wait status plus one when reaped, or negative errno. Terminate requests SIGKILL; drop terminates and reaps a live direct child. |
| `child_request_stop` | `(&mut child) -> Result<(), i64>`: send SIGTERM only to the owned child without waiting; a reaped child returns ECHILD. |
| `stdin_read file_read file_write` | Use an exclusive authority reference and a byte slice; return i64 count or negative errno. |
| `stdout_write stderr_write` | Write an immutable byte slice through the corresponding exclusive authority reference. |

Listener and Connector addresses contain exactly 16 bytes: bytes 0..2 hold AF_INET (2) as a
little-endian u16, bytes 2..4 hold the port as a big-endian u16, bytes 4..8 hold
the IPv4 octets, and the remaining bytes are padding. A zero port requests a
kernel-selected listener port. Construction rejects another length or family with errno
22. Connector additionally requires zero padding and snapshots the destination. Address queries require an exact 16-byte writable slice and reject another
length without writing it. No DNS lookup or outbound connection is implicit.

Listener is non-Copy and non-Data. Structural destruction closes its owned
handle; accepted Files follow ordinary File read/write/drop rules. Connector is
non-Copy and non-Data, but owns only an immutable endpoint: constructing or dropping
it acquires or releases no socket. Each connect returns a separate owned stream.
Capability acquisition results require a non-Copy enum with two alternatives: one success
field of the declared capability type, then one i64 errno field. Native capability
values occupy eight bytes except Stop, whose descriptor and sticky observation
occupy sixteen bytes. Owned descriptors are close-on-exec and promoted above
standard streams before exposure, preserving any originally closed stdio.
Interrupted operations retry; closing consumes the descriptor without retry.
The kernel determines backlog and resource limits.

Readiness is affine and non-Data. It owns only the wait set; dropping it does not
close watched handles. Registration borrows are temporary and retain no language
views. Tokens are caller data, never descriptors or authority. Interest bits are
1 readable and 2 writable; zero watches full close/error only. Other bits fail
EINVAL. Re-registering the same descriptor updates its token and interests.
Event bits are 1 readable, 2 writable, 4 closed and 8 error. Read interest also
reports peer write-half closure; write-only interest does not repeatedly report
that half-close while a response remains backpressured. Readable data may accompany
closure and must be drained before treating the stream as empty.
A nonblocking operation returns errno 11 when it would block; readiness is a hint,
so another operation must still handle that result. Aliases share O_NONBLOCK.
The kernel removes registrations on final open-file-description closure; when
aliases remain, explicitly unwatch before closing the registered descriptor.
Each wait returns at most 64 events, with no bound on registered handles. A past
deadline polls once without blocking. EINTR recomputes the remaining absolute
time. An empty result means the deadline elapsed without a ready event.

Storage creation preserves the Directory's acquired identity and constrains paths
with openat2 RESOLVE_BENEATH|RESOLVE_NO_MAGICLINKS, without a weaker fallback.
create_new and create_private use O_CREAT|O_EXCL|O_RDWR; open_append uses
O_CREAT|O_APPEND|O_RDWR, never truncates, starts at offset zero, and places each
write at the current end. Ordinary creation requests mode 0666; private creation
requests mode 0600 atomically, both subject to umask. Existing paths cause an
exclusive create to fail. set_private sets exactly 0600 regardless of umask,
restricting future opens but never revoking already-open descriptors. File sync
and directory sync are separate durability effects; directory sync opens a
readable descriptor for the acquired directory and may fail if that access is
unavailable. Advisory locks do not exclude noncooperating writers. Closed File
sentinels return EBADF for these effects without operating on inherited stdin.

Entropy is affine and non-Data. Derivation has no fallible acquisition; fill uses
kernel getrandom with flags zero, blocking until randomness is ready, and retries
EINTR and short reads. Empty slices succeed. On a later error, the filled prefix
remains changed and the rest is untouched; the interpreter preserves this result
rather than rolling back bytes. No userspace generator or entropy fallback is used.
Wall-clock seconds may jump; monotonic clock_read remains the deadline source.

Stop is affine and non-Data. Successful acquisition blocks SIGTERM and owns a
nonblocking, close-on-exec signalfd. It claims the signal for the execution
lifetime: dropping any or the last Stop closes only that descriptor and does not
restore the mask. Failure restores the previous mask and does not claim the
signal. Multiple Stops share the process pending standard-signal queue; delivery
is not broadcast and repeated signals may coalesce. Before its first true,
requested consumes one pending SIGTERM, returning false without blocking if none
is pending; later calls return cached true without consuming another signal.
Readiness reports unread SIGTERM as read bit 1;
after consumption the descriptor need not remain ready despite the cached result.

Native stop_target returns the actual program PID. The interpreter returns the
PID of its owned syscall broker, whose signal mask and queue implement the
interpreted execution's Stop contract. The CLI driver or wrapper PID is not that
endpoint: graceful interpreted stop must address the capability's published
target. The returned integer identifies an endpoint but grants no signal-sending
authority; child_request_stop takes an owned Child, never an arbitrary PID.

On the first successful Stop acquisition, execution records the previous SIGTERM
mask bit. Children spawned after that claim restore exactly this bit before exec,
preserving every other mask bit; executions without a successful claim do not
change it. Thus an originally blocked SIGTERM remains blocked in spawned children.
This rule still applies after all Stops drop. Child stop requests do not wait or
change reaping ownership; live Child destruction still kills and reaps.

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

## Host entry and authority reports

A host entry returns unit or a scalar. It may take no parameters, a sole World,
or owned Stdout/Stdin/Stderr, Directory, Listener, Connector, Readiness, Process,
Clock, Entropy and Stop parameters.
The sole World entry declares implicit full authority. Every other parameter
requires an explicit invocation grant selected by its distinct, nonempty local
name. ASCII whitespace, control bytes and equals signs cannot occur in grant names.
File, Child, reference and aggregate parameters cannot be provisioned as entries;
use a Directory entry and acquire files through ordinary checked operations.

Grants are invocation data outside the IR. Directory descriptors contain the
NUL-terminated root path; Listener and Connector descriptors contain the sixteen-byte address
above. Other allowed capabilities, including Entropy and Stop, need no payload.
Both executors independently validate the complete parameter/grant correspondence
before acquiring handles.
Raw scalar arguments cannot manufacture an address or derived capability.
Missing, duplicate, unknown or malformed CLI grants reject before publication or
execution. Build embeds validated descriptors; acquisition occurs at startup.
Acquisition failure reports R0007 at entry and releases acquired resources.

The authority query starts from a checked entry. It lists all declared grants,
then conservatively follows reachable structured branches, calls and owning
cleanup, including custom destructors and capabilities nested in owned values.
Unreachable instructions after return, trap or a loop transfer do not contribute.
Borrowed values do not cause destruction of their referents. Recursive functions
are listed once. This query reads checked IR alone and does not infer a smaller
grant set from source spelling or a particular runtime branch. Stop ownership
cleanup contributes stop_close; merely borrowing Stop does not close it.

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

## Rechecked optimization and prior proofs

The inliner consumes admitted original IR and produces an untrusted candidate.
The checker validates that candidate before native emission; rejection discards
the whole candidate. Interpretation, authority reports and IR output retain the
original. Differential cases compare optimized native behavior with that original.
Copied instructions retain their original file, line and column, including traps.

A recheck may reuse an immutable original function proof only after validating
shared structure and independently comparing the exact type/destructor graph,
ordered signatures, parameter locals and return contracts. It then compares the
whole function, including locals, body, literal bits and source provenance.
A changed graph disables this local reuse; changed bodies receive ordinary checks.
Authenticated persistent proofs use the same dependencies and remain separately
counted. Native fragments never supply checker authority. Observation-only commands
skip optimization unless the corpus roundtrip explicitly exercises both forms.
