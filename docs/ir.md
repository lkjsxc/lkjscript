# lkjscript IR reference

The lkjscript IR is an explicit, monomorphic program of types, functions, local
slots, places and structured instructions. Producers may construct arbitrary IR.
Only the checker can produce the `CheckedProgram` accepted by a backend.
Parsing text, producing transformed IR and loading store bytes grant no authority.

## Text and identity

Files use `.lkjir` and begin with the exact first line `lkjscript-ir 1`.
The revision describes the text format. Unsupported or absent headers are errors.
`lkjscript ir SOURCE` writes it; `check`, `build`, `run`, `test` and `authority` accept it.

Optional `source "path"` records assign diagnostic file indices in order. They
carry no read authority. Nonempty maps require every function/instruction file
index to exist; empty paths reject. Source emission binds actual source/library
roots before authentication, so program directories and explicit library paths
remain openable. Paths and metadata survive text/binary round trips and inlining.
CLI input without source records uses the `.lkjir` path and actual instruction
line in that file; raw in-memory IR interpretation retains producer metadata.

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
| `enum copy\|move` ... `endtype` | Ordered alternatives written `variant NAME FIELD TYPE ...`; Copy requires Copy payloads. Non-Copy affine enums use move; affine is rejected with a token edit to move. |
| `ref shared\|mut TYPE` | Shared or exclusive reference to a sized value, slice or text. |
| `slice shared\|mut TYPE` | Unsized sequence shape; values require a reference type. |
| `array TYPE LENGTH` | Fixed-length owned elements. |
| `vec TYPE`, `box TYPE` | Owned growable sequence or owned indirection. |
| `str` | Unsized immutable UTF-8 text; values require a shared reference. |
| `world`, `arguments`, `environment`, `exit_status`, `stdout`, `stdin`, `stderr`, `directory`, `read_directory`, `file`, `read_file`, `foreign_command`, `foreign_session`, `image`, `channel`, `worker`, `read_region`, `null_device`, `process`, `clock`, `child`, `listener`, `connector`, `readiness`, `entropy`, `stop` | Affine authority-bearing values. |

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
| `trap MESSAGE` | Terminate with a located trap; there is no unwinding. Codes from R0016 use the zero-based lexical order of distinct message strings, so duplicating/reordering instructions preserves them. |

Unary `-` supports signed integers and f64; `!` supports integers and bool.
Integer binary operations are `+ - * / % & | ^ << >>`, comparisons, and
`wrapping_add wrapping_sub wrapping_mul`. F64 supports arithmetic and
comparisons; char supports comparisons; bool supports equality and logical/bit
operators. Operands have the same IR type. Comparisons produce bool.
Binary64 negation flips only the sign bit; binary arithmetic canonicalizes NaNs.
Bit conversions preserve every bit.

Ordinary integer arithmetic, division and shifts trap on invalid operations.
Wrapping operations compute modulo the declared width.

The intrinsic signatures are checked in `src/check.lkj` and `src/check/io.lkj`:

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
| `world_arg_count world_arg_len world_arg_copy` | Read/copy byte arguments, excluding argv0, through an exclusive World or Arguments reference. Empty and non-UTF-8 arguments are preserved. |
| `environment_len` | `(&mut environment) -> i64`: length of the granted variable's value, or -1 when absent. Empty values have length zero. |
| `environment_copy` | `(&mut environment, &mut [u8]) -> usize`: copy at most the destination length from that same value; return bytes copied, zero when absent. No supplied lookup name or enumeration. |
| `world_exit_status` | Set the requested u8 process status through an exclusive World or ExitStatus reference; execution and normal cleanup continue, and the last value wins. |
| `world_directory directory_open directory_open_directory` | Acquire a root Directory from World, or a File/Directory beneath an existing Directory. |
| `read_directory_open read_directory_open_directory` | `(&mut read_directory, &[u8] path) -> Result<read_file/read_directory, i64>`: exact read-only kinds; only regular files enter ReadFile. No writable File/Directory result substitution is admitted. |
| `file_seek` | `(&mut file, i64 offset, u8 origin) -> Result<usize,i64>`: origins 0/1/2 are SET/CUR/END; others fail EINVAL. O_APPEND controls writes independently of the seek offset. |
| `file_modified_nanoseconds` | `(&mut file/read_file) -> Result<usize,i64>`: fstat of that held regular inode; Unix mtime nanoseconds, EINVAL for nonregular, EOVERFLOW for pre-epoch or unrepresentable time. |
| `foreign_run` | `(&mut foreign_command, &mut read_directory, &mut null_device, &[u8] argv, usize duration_ns) -> Result<read_file,i64>`: exact typed loans, immutable initial image, fixed kernel profile below, synchronous deadline plus kill/reap. |
| `foreign_image` | `(&mut foreign_command) -> Result<read_file,i64>`: fresh read view of immutable transformed initial ELF; checked code can hash these exact bytes. |
| `foreign_start` | `(&mut foreign_command, &mut read_directory, &mut null_device, &[u8], usize duration_ns) -> Result<foreign_session,i64>`: attached confined session; explicit argv encoding and retained read-only root. |
| `foreign_exchange foreign_finish` | `(&mut foreign_session, &[u8], &mut [u8], usize duration_ns) -> i64` exchanges exact framed bytes transactionally; `(&mut foreign_session, usize duration_ns) -> i64` sends input EOF and returns cached raw wait status. Negative results are errno; [full lifecycle](programs-foreign-session-contract.md). |
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
| `file_lock_wait` | `(&mut file) -> Result<(), i64>`: blocking advisory exclusive flock, retrying EINTR; held until the last duplicate closes. |
| `file_set_private` | `(&mut file) -> Result<(), i64>`: fchmod to exactly 0600. |
| `file_duplicate` | Duplicate an exclusively borrowed File, sharing its offset and preserving closed state. Return Result<File, i64>. |
| `file_readonly` | Reopen an exclusively borrowed File read-only with a fresh offset at zero, preserving closed state. Return Result<File, i64>; no path authority is granted. |
| `file_seal` | `(&mut file) -> i64`: F_ADD_SEALS with SEAL/SHRINK/GROW/WRITE, zero or negative errno; never accepts ReadFile or manufactures image authority. |
| `file_rewind file_executable directory_rewind` | Seek to byte zero or make a File executable; return zero or negative errno. |
| `directory_read` | Exclusively borrow Directory/ReadDirectory and read Linux directory records into mutable bytes; return count or negative errno. |
| `directory_create directory_remove directory_rename` | Mutate NUL-terminated basenames through exclusive Directory references. Reject slash, empty name, dot and dot-dot. Return zero or negative errno. |
| `clock_read` | Clock plus mutable bytes of length at least 16. Write little-endian i64 seconds/nanoseconds from CLOCK_MONOTONIC; return zero or negative errno. |
| `clock_unix_seconds` | `(&mut clock) -> Result<i64, i64>`: signed CLOCK_REALTIME seconds since Unix epoch; negative successful timestamps remain distinct from positive errno. |
| `clock_unix_nanoseconds` | `(&mut clock) -> Result<usize, i64>`: CLOCK_REALTIME nanoseconds since Unix epoch, which may jump; negative or unrepresentable timestamps return EOVERFLOW (75). |
| `process_spawn` | Exclusive Process, shared executable File, NUL-separated argv, and four shared Files for stdin/stdout/stderr/fd3. Execute with empty environment; return Result<Child, i64>. |
| `child_wait child_terminate` | Exclusive Child. Wait takes a nonblocking bool and returns zero while running, Linux wait status plus one when reaped, or negative errno. Terminate requests SIGKILL; drop terminates and reaps a live direct child. |
| `child_request_stop` | `(&mut child) -> Result<(), i64>`: send SIGTERM only to the owned child without waiting; a reaped child returns ECHILD. |
| `stdin_read file_read file_write` | Use an exclusive authority reference and a byte slice; return i64 count or negative errno. FileRead accepts File/ReadFile; FileWrite requires File. |
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

Only openat2 ENOSYS selects a stricter openat fallback. It acquires every component
with O_NOFOLLOW, keeps each directory inode owned while opening its child, and
rejects absolute paths, parent/empty components and every symlink, including an
O_PATH final symlink. Native execution copies the actual validated path length;
the independent broker splits/restores its private request payload. Both return
host allocation/open errors without a guessed path or component limit.
ReadDirectory and ReadFile are distinct affine, non-Data kinds. Reading and
directory enumeration accept their exact kinds; mutating, lock, executable,
duplicate and writable conversion operations do not. Read opens acquire O_PATH,
check regular-file mode on that inode, and only then reopen the same inode for
reading, so a named FIFO/device is never data-opened. mtime and contents refer to
that same acquired inode even after atomic replacement.

Storage creation preserves the Directory's acquired identity and constrains paths
with openat2 RESOLVE_BENEATH|RESOLVE_NO_MAGICLINKS.
create_new and create_private use O_CREAT|O_EXCL|O_RDWR; open_append uses
O_CREAT|O_APPEND|O_RDWR|O_NOFOLLOW, never truncates, starts at offset zero, and places each
write at the current end. A final symlink returns ELOOP (40), even if its target
would remain beneath the granted directory. Ordinary creation requests mode 0666; private creation
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
or owned Arguments, Environment, ExitStatus, Stdout/Stdin/Stderr, Directory,
ReadDirectory, ForeignCommand, NullDevice, Listener, Connector, Readiness, Process,
Clock, Entropy and Stop parameters.
The sole World entry declares implicit full authority. Every other parameter
requires an explicit invocation grant selected by its distinct, nonempty local
name. ASCII whitespace, control bytes and equals signs cannot occur in grant names.
File, ReadFile, Child, reference and aggregate parameters cannot be provisioned as entries;
use a Directory entry and acquire files through ordinary checked operations.

Grants are invocation data outside the IR. Directory/ReadDirectory descriptors contain the
NUL-terminated root path; Listener and Connector descriptors contain the sixteen-byte address
above. Environment descriptors contain one exact NUL-terminated variable name; names are nonempty and exclude ASCII whitespace/control bytes and equals signs. ForeignCommand supplies an absolute executable path for immutable snapshot acquisition. Other capabilities, including NullDevice, Arguments and ExitStatus, need no payload.
CLI Environment grants use NAME=VARIABLE. The bound value preserves arbitrary non-NUL bytes; absence is valid and differs from an empty value.
Directory/ReadDirectory also accept NAME=env:VARIABLE, resolving only that variable to its root at invocation/startup. Absence or an empty value fails with errno 2; there is no default or implicit creation. This selector supplies the declared directory kind, not an Environment value. Program-directory relative paths resolve against that program directory for both kinds.
Build retains names for startup resolution. Native run binds only the named environment values and directory selectors before its empty-environment launch; authority queries retain the named grant specification.
Both executors independently validate the complete parameter/grant correspondence
before acquiring handles.
Raw scalar arguments cannot manufacture an address or derived capability.
Missing, duplicate, unknown or malformed CLI grants reject before publication or
execution. Build embeds validated descriptors; acquisition occurs at startup.
Acquisition failure reports R0007 at entry with the grant kind/value, actual failed operation and symbolic/numeric errno, then releases acquired resources. Listener enables SO_REUSEADDR for restart after TIME_WAIT; another active listener still refuses the grant.


Runtime diagnostics use source paths and lines in every execution route. Native
standalone output is bounded to one 2048-byte line built without allocation after
failure, through an already granted output. An entry without World/Stdout/Stderr
exits unsuccessfully without printing. If both output streams are granted and
stderr refuses the diagnostic, stdout receives it. Path/message controls are
escaped; long fields end in `...`. Missing roots, occupied listeners, denied
readiness/signal descriptors and stream failures preserve the original syscall
and errno even when cleanup issues more syscalls. Token-only grants have no
fallible kernel acquisition; ordinary Result-returning calls keep their errors.
Malformed interpreter-host replies are M_INVALID_HOST, not a program R0007.

A private supervisor trap record begins with 32 bytes: `LKJTRAP2`, then little-
endian u64 code, file index and line, followed by the complete bounded line.
The decoder validates the same selected main/runtime source and message table,
rejects incomplete/invalid records, and reconstructs the full static identity.
Batch, mutation and authenticated test records preserve that full identity;
only user delivery truncates. Equal visible prefixes cannot merge different
long static messages. The compiler's tooling-only unchecked `--observe` route
emits the existing JSON observation record for independent IR subprocesses.

The authority query starts from a checked entry. It lists all declared grants,
then conservatively follows reachable structured branches, calls and owning
cleanup, including custom destructors and capabilities nested in owned values.
Unreachable instructions after return, trap or a loop transfer do not contribute.
Borrowed values do not cause destruction of their referents. Recursive functions
are listed once. This query reads checked IR alone and does not infer a smaller
grant set from source spelling or a particular runtime branch. Stop ownership
cleanup contributes stop_close; merely borrowing Stop does not close it.

### Foreign execution profile

ForeignCommand provisioning requires one absolute NUL-terminated executable path,
with no environment selector; NullDevice takes no payload and verifies the opened
/dev/null device identity. Neither kind derives from World or a raw scalar.
The initial image is snapshotted immutably; its ELF interpreter path is transformed
to the fixed retained-root descriptor before sealing. Its readable identity is not
an authenticated lkjscript entry-authority manifest.
ForeignRun retains the exclusive root and null-device loans until reap. Its kernel
profile permits read/execute access beneath that root, including preexisting
special files and mutable runtime code, and verified null-device read/write without
ioctl. Private readable/writable/seekable stdout/stderr capture memfds are also
explicit handles; their anonymous contents can be reexecuted within the same
confinement. It denies socket syscalls, descendants, filesystem metadata mutation
syscalls and credential changes; reexec retains confinement. Zero duration returns EINVAL before a child starts. An absolute monotonic deadline begins before
setup, and parent death kills the attached child. Output/exit/deadline arrive in an
owned readable result only after kill/reap. Outside metadata, clocks and entropy
remain observable; this profile does not claim isolation of all IPC or information
channels or immutability of runtime dependencies.
The environment is exactly LD_LIBRARY_PATH=/proc/self/fd/5/.runtime,
HOME=/proc/self/fd/5, GIT_CONFIG_GLOBAL=/proc/self/fd/5/.no-global-config,
GIT_CONFIG_NOSYSTEM=1, GIT_OPTIONAL_LOCKS=0, GIT_TERMINAL_PROMPT=0 and LC_ALL=C.
There is no PATH lookup or inherited environment; fd5 retains the granted root.
The authority report names each capability and this fixed profile. It is an
invocation template; the checked consumer hashes ForeignImage when it needs an
exact image identity. Optional Rust witness methods return ENOSYS rather than
running an unconfined substitute.

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

A recheck ma
Authenticated transfer-only child entry, stable capability slots and the sealed executable identity contract are specified in [child images](child-image.md). Their manifest is derived from CheckedProgram, never from producer annotations.

## Isolated workers and immutable regions

`image`, `channel`, `worker` and `read_region` are opaque affine IR kinds,
not integer/aggregate aliases, Copy or Data. Their owners can inhabit owned
containers; views of them obey the ordinary checker loan rules. There is no
cross-thread Send/Sync rule or shared mutable address space. Parallel execution
uses separate processes and heaps; the only shared mapping is immutable.

An ordinary Image grant is `--grant name=CHILD_ELF --image-key KEY`. The driver
authenticates the exact compiler producer, ELF bytes and canonical child
manifest, snapshots/seals the admitted descriptor, then embeds those exact bytes
and identities in the parent. Only a manifest with one owned Channel slot is
accepted by this runtime form. No World, root, listener, clock or ambient stream
is silently granted to the worker. Normal entry acquisition cannot construct
Channel, Worker or ReadRegion, and arbitrary File bytes cannot become Image.
The producer key must remain outside every read grant, including hardlink aliases;
runtime-grant overlap exclusion is a documented pending provisioning check.

The parent's report names each image digest and manifest identity, its exact
entry slot and attached lifetime. Entry authority is distinct from later moved
File messages. A Channel permits byte messages and explicitly received File
values; it does not prove that the worker forever holds only its initial slot.
Both IR bodies and implicit drops contribute reachable operations to the report.

All deadlines below are unsigned 64-bit absolute monotonic nanoseconds. The
caller supplies them; progress never extends them. These are separate from
ForeignSession's duration arguments. In this table `R<T>` means structural
`Result<T,i64>`; errors are positive Linux errno.

| Intrinsic | Checked signature |
| --- | --- |
| `image_spawn` | `(&mut image, usize deadline) -> R<worker>` |
| `image_buffer` | `(&mut image, &[u8]) -> R<file>` |
| `channel_send worker_send` | `(&mut channel/worker, &[u8], usize deadline) -> R<usize>` |
| `channel_receive worker_receive` | `(&mut channel/worker, &mut [u8], usize deadline) -> R<usize>` |
| `channel_send_file worker_send_file` | `(&mut channel/worker, file, usize deadline) -> R<()>` |
| `channel_receive_file worker_receive_file` | `(&mut channel/worker, usize deadline) -> R<file>` |
| `worker_wait` | `(&mut worker, usize deadline) -> R<i64>` |
| `worker_terminate` | `(&mut worker) -> R<()>` |
| `worker_result` | `(&mut worker) -> R<read_file>` |
| `file_map_readonly` | `(&mut file) -> R<read_region>` |
| `read_region_bytes read_region_len` | `(&read_region) -> &[u8]` or `usize` |

Spawn creates a private message endpoint and result channel and executes the
same sealed image descriptor. Its deadline covers the pre-exec handshake.
Parent-death SIGKILL plus a post-install parent identity check close the startup
race; unrelated descriptors are closed at exec. Worker drop kills, reaps and
closes its resources. There is no detached operation. Wait retains the actual
exit code or 128+signal for repeat queries; a wait timeout kills/reaps and returns
ETIMEDOUT while retaining that status. Terminate kills/reaps and retains the
actual status. Result returns a fresh readonly result-file view after wait or
terminate has reaped the child (EBUSY until then, even if the OS child exited). A trap ends that worker; interpretation/location of its
private result record is a caller/library decision bound to its admitted image.

Messages are nonempty kernel packets. Send preserves a packet boundary; receive
returns the packet length and changes only that prefix on success. An empty
buffer is EINVAL, oversized packets or unexpected ancillary data reject, and any error leaves
the caller's buffer unchanged. EOF is zero. Descriptor messages accept exactly
one File and reject malformed, extra or truncated ancillary data while closing
received descriptors. A sent File is consumed even when the operation fails;
the sender cannot reuse that value. Explicit duplicates made beforehand remain
separate owners of the same kernel object. No Directory, Listener, Image or
other capability kind can currently travel in a message.

Image.buffer copies bytes once into anonymous storage and permanently seals
write/grow/shrink/seal changes. File.map_readonly requires all four seals, retains
its own descriptor and maps PROT_READ; an unsealed file returns EINVAL without
consuming it. Empty regions are valid. Mapping lifetime does not depend on the
original File or another map. ReadRegion.bytes derives a shared view tied to
that region owner, including the exact interpreter extent; mutable conversion,
owner drop/move while viewed and false static-return metadata reject from IR
alone. Region drop unmaps/closes once. Existing writable File aliases cannot
write or resize sealed storage. The typed view here is a byte slice, not an
unchecked cast to a foreign object layout.

Byte framing is still a library convention. A common std::workers codec avoids
duplicated encoding in its specimens, but does not supply semantic message
identity or generated arbitrary typed codecs. The pending compiler handoff is
`tests/diagnostics/programs_semantic_message_identity.lkj`. General capability
messages, structured typed shared views and in-process mutable threads remain
unsupported. The process pool handoff is
`tests/diagnostics/programs_compiler_pool_adoption.lkj`; checked-unit serialization
and separate heaps are real adoption costs.

std::workers::ordered_map uses one in-flight job per owned worker, returns input
order only after all replies and successful joins, and drops workers on error.
Its explicit event history is checked by std::process_model; deterministic
seeded histories explore protocol interleavings, not native scheduling or
thread races. Durable publication belongs to the sole parent storage owner:
write a private file, sync it, rename, then sync the directory. The retained
pool publication specimen checks complete old/new results under parent kills;
that is process-kill atomicity, not a sudden-power-loss experiment.
