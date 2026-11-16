# lkjscript IR reference

The lkjscript IR is an explicit, monomorphic program of types, functions, local
slots, places and structured instructions. Producers may construct arbitrary IR.
Only the checker can produce the `CheckedProgram` accepted by a backend.
Parsing text, producing transformed IR and loading store bytes grant no authority.

## Text and identity

Files use `.lkjir` and begin with the exact first line `lkjscript-ir 1`.
The revision describes the text format. Unsupported or absent headers are errors.
`lkjscript ir SOURCE` writes it; `check`, `build`, `run`, `test` and `authority` accept it.

`ir` serializes the admitted original program. Native emission may instead use
a rechecked optimization of that program. `build --profile --profile-ir PATH`
writes the exact checked IR selected for native emission, including the
original fallback when an optimization candidate is rejected. See
[native CPU profiles and selected IR](#native-cpu-profiles-and-selected-ir).

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
| `directory_rename_new` | `(&mut directory, &[u8] old, &[u8] new) -> i64`: same held parent, validated NUL-terminated basenames, atomic absent-only rename. An occupied destination is never replaced; unsupported syscall/filesystem errors have no fallback. |
| `directory_open_regular` | `(&mut directory, &[u8] name) -> Result<file,i64>`: one validated basename; retain its inode without following a final symlink, require a regular file, then reopen that inode read-only. |
| `directory_identity` | `(&mut directory, &mut [u8]) -> i64`: require exactly 16 output bytes; write little-endian device/inode from the held directory. Invalid length, non-directory or any metadata error leaves every output byte unchanged. |
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

Absent-only rename uses Linux x86-64 renameat2 with RENAME_NOREPLACE in the
same acquired directory. Both names reject slash, empty name, dot, dot-dot,
interior NUL and a missing trailing NUL. It has no check-then-replacing-rename
path. Directory regular-open uses O_PATH|O_NOFOLLOW before fstat and data
access; symlinks, directories, FIFOs and devices refuse without a data open.
The retained identity descriptor stays alive through the read-only reopen.

Directory identity observes st_dev and st_ino from fstat of the same held
descriptor, requires S_IFDIR, and copies only those two u64 values after
successful validation. Kernel metadata is read into private scratch, never
directly into the caller's 16-byte result. The ordinary library helper
`std::io::directory_identity` returns `Result<[u8;16],i64>` and needs an
exclusive Directory borrow. These values describe a held kernel directory;
they may be recycled after release and do not enter semantic/image identities.

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

## Native CPU profiles and selected IR

CPU profiling is an explicit build option:

```text
lkjscript build SOURCE --profile --profile-ir PROGRAM.native.lkjir -o PROGRAM
lkjscript profile PROGRAM.samples.PID
lkjscript profile PROGRAM.samples.PID --copy-sites \
  --profile-ir PROGRAM.native.lkjir --limit 100 --bytes 65536
```

`--profile` adds the existing CPU sampler and its output path to the executable.
Each process writes its own `OUTPUT.samples.PID` file. The default `profile`
report shows function self/inclusive counts and instruction-kind counts.

`--profile-ir PATH` on a profiled build writes the exact IR selected for native
emission. Emission and serialization use the same checked selection. Rejection
of an optimization candidate selects the original program for both outputs.
Ordinary `ir` output continues to describe the original admitted program.

A companion request bypasses the early whole-image memo return. Frontend,
checker and native-fragment reuse may still operate. An image-only cache hit
does not supply the requested companion. Ordinary builds do not collect
copy-site metadata or serialize/hash selected IR for this feature.

### Output publication

The driver retains both output directories and rejects destinations with the
same held parent identity and basename, including aliases for the parent.
It stages both contents, publishes the companion, then publishes the image.

These two publications are not an atomic transaction. A failure can leave one
destination replaced. The command returns nonzero and publishes no new image
memo after a partial publication; it does not promise rollback of both paths.
Successful publication requires both renames.

The profile binds the exact companion bytes by SHA-256. A missing or different
companion is refused by the detail reader. Identical companion bytes retained
from an earlier build still satisfy this correspondence. The digest establishes
neither freshness nor a common publication event.

### Copy-site detail and bounds

`--copy-sites` is accepted only by `profile`. It requires `--profile-ir` and
explicit positive `--limit` and `--bytes` values. On this route,
`--profile-ir` names the companion to read.

Each detail row identifies a native copy interval, sample count, selected-IR
function, instruction, subsite and type, fixed byte width, and source/destination
roles. These numeric coordinates belong to the exact companion bytes. They
are not stable source identities. Source locations survive inlining, but the
profile does not reconstruct an inline-call chain.

Rows sort by descending samples, then the complete
`(function, instruction, subsite, start, end)` tuple. Names are quoted
deterministically. The byte bound covers the complete encoded reply, including
both header lines and its continuation. A bound too small for a complete next
row and continuation is refused; rows are never cut in the middle.

A continuation has the form
`4:PROFILE_SHA256:COMPANION_SHA256:NEXT`. Supply it with `--cursor` to resume the
same ordering. Both exact byte digests are part of the selector. Bounds may
change between pages; changed profile or companion bytes invalidate the
continuation. `cursor=none` marks the end. A continuation is a content selector,
not an authenticated store token.

The reader validates the complete profile and companion before returning any
detail output. Parsing, independent IR checking and sorting follow artifact
size; the output bounds do not make this an answer-size-bounded semantic-store
query.

### LKJPROF4 representation

The reader requires `LKJPROF4`; older headers are rejected. Numeric fields are
little-endian u64. Encoded sections are contiguous and appear in this order:

1. The 160-byte header.
2. The 64-byte symbol rows and their UTF-8 name bytes.
3. The 16-byte instruction-kind rows.
4. Sixteen u64 instruction-kind counters.
5. The 72-byte copy-site rows.
6. The 16-byte selected-function map rows, ending exactly at EOF.

The header contains:

| Offset | Bytes | Field |
| ---: | ---: | --- |
| 0 | 8 | `LKJPROF4` |
| 8 | 8 | Symbol-row count |
| 16 | 8 | Requested sample period, 1,000,000 ns |
| 24 | 8 | Handler serial |
| 32 / 40 | 8 each | Kind-table offset / row count |
| 48 | 8 | Kind-counter offset |
| 56 / 64 | 8 each | Code interval start / end |
| 72 | 8 | Reserved zero |
| 80 | 32 | Exact selected-IR companion SHA-256 |
| 112 / 120 | 8 each | Copy-table offset / row count |
| 128 / 136 | 8 each | Function-map offset / row count |
| 144 | 8 | Unmapped-copy sample count |
| 152 | 8 | Reserved zero |

A symbol row contains eight words: native start/end, self count, inclusive
count, last included serial, name offset/length, and frame position.
A kind row contains its native interval start and kind; its end is the next
row's start or the code end.

A copy row contains nine words:

| Offset | Field |
| ---: | --- |
| 0 / 8 | Nonempty half-open native interval start / end |
| 16 | Sample count |
| 24 | Selected-program function index |
| 32 | Selected-IR instruction index |
| 40 | Subsite index |
| 48 | Copied-payload type index |
| 56 | Positive fixed copied byte width |
| 64 | Origin, roles and place flags |

An incoming parameter copy uses the all-ones instruction sentinel and its
parameter index as the subsite. Body copies use the actual selected-IR
instruction index. Constructor subsites retain the original field or element
index, including gaps for elements that emit no copy. Other transfers use
subsite zero.

Each function-map row contains a selected-program function index and the final
symbol-row index. Every selected function has one map row. Unemitted functions
use the all-ones symbol sentinel; a copy site's owner cannot use that sentinel.
The symbol table also includes runtime and profiler helpers, so selected
function indices and symbol-row indices are distinct.

Tag bits are:

| Bits | Meaning |
| --- | --- |
| 0–7 | Origin: 1 body instruction, 2 incoming parameter |
| 8–15 | Source role |
| 16–23 | Destination role |
| 24–26 | Source-place flags |
| 32–34 | Destination-place flags |
| All others | Reserved zero |

Role values are 1 Local, 2 TypedPlace, 3 ConstructorField, 4 ArrayElement,
5 ABIIncoming, 6 ParamLocal, 7 ABIResult, 8 VecElement, 9 ResultVariantField,
10 RepeatedArrayElement, 11 BoxPayload, 12 Referent and 13 ViewHeader.
Place flags may combine: 1 local projection, 2 dereference and 4 collection
projection. Nonzero place flags apply only to TypedPlace.

The parser checks section sizes and arithmetic before indexed access. It
requires valid symbol/function mappings, reserved fields and tags, and ordered
nonoverlapping copy intervals contained within their owner and copy-kind
interval. The detail reader additionally checks instruction, subsite, type,
roles and width against the independently checked companion and native layout.

### Sampling scope and trust

Only intervals classified as the existing copy instruction kind receive
copy-site attribution. Runtime/allocator overrides, variable-width runtime
copy, Replace's byte-swap, and scalar/frame/address work outside those intervals
remain excluded. Zero-size and storage-coalesced transfers emit no copy site.
Unknown copy attribution increments the explicit unmapped counter.

The signal handler increments a site or the unmapped counter before incrementing
the copy-kind counter. Let D be all site counts plus unmapped, K the copy-kind
count, T all completed kind counts and S the handler serial. Valid captures
require `S - T` to be zero or one. Normally `D = K`; `D = K + 1` is accepted
only when `S - T = 1` and is reported as one possibly in-flight detail sample.
Larger differences, reverse differences and sum overflow reject. The reader
does not decrement an arbitrary row to make the totals equal.

The companion is diagnostic IR text. Neither it nor the sample file is an
admission certificate, authenticated store snapshot or proof that a particular
execution occurred. The detail reader independently checks the companion's IR
before validating coordinates and layout. Profile counts remain observations
supplied by the sample file.

A machine copy interval does not prove a Copy type, freshness, nonescape or
removable work. Multiplying samples by width does not measure dynamic byte
traffic or recoverable execution time.

## Structural questions and snapshot identity

`build SOURCE --index --store DIRECTORY --store-key KEY` publishes an immutable
source/checked-IR snapshot and prints `LKJSCRIPT_SNAPSHOT ID`. `query SOURCE`
uses the same source/library selection and store, with `--kind`, `--id`,
`--snapshot ID`, `--depth`, `--limit`, `--bytes` and `--cursor`.
Kinds are outline, item/text, slice, uses, callers, callees, impls, instances,
type, layout, authority and dependents; meta/signature/references expose detail.
Every fact starts with its quoted identity and declares source-claim, checked-ir
or protocol. Layout uses the native target's actual checked-IR layout; reachable
effects include destruction and work for ordinary functions with scalar inputs.

A captured source signature exceeding 1,024 bytes is split at UTF-8 boundaries.
Its facts carry zero-based fixed-width hexadecimal `chunk`, decimal `chunks`
and total `bytes`; only the last fragment has `complete=true`. Concatenating
`text` fragments in order reconstructs the trimmed declaration header without
its body. Bounded continuation carries all remaining fragments, including across
stored pages. Short signatures retain their single-fact form. Historical pins
retain their original producer's claims; rebuild to publish newly complete facts.

Source identities use semantic paths, independent of physical files and whitespace.
Fields, variants, named parameters, generic parameters and implementations have
named identities. An anonymous `_` parameter uses `OWNER::param:POSITION`, its
zero-based position in the complete signature, including any receiver; named
parameters retain `OWNER::param:NAME`. Anonymous IDs remain stable until signature
positions change, then clients query a fresh outline. Numeric positions cannot
collide with legal names, and there is no `param:_` alias. A semantic rename/move
changes its path; physical reorganization does not. Bindings and calls are scoped
by the token-normalized body digest and a local
ordinal; only an unchanged token body preserves that epoch. Comments/formatting do
not change it. Source byte ranges and file/item hashes are exact stale-edit inputs.
These producer identities never replace the checker's exact proof identities.

Source declaration/signature facts are resolved frontend claims. Solved generic
definition constraints supply binding and expression types, exact lexical binding
targets and nominal field targets. Method selections come from elaboration.
Associated-call member facts use a declared trait-selection contract or a source
choice whose calling function and enclosing implementation have no type or const
parameters and whose owner is not a trait; lifetime parameters do not vary it.
The trait contract can cover an undemanded generic definition. Every accepted
member has an exact terminal-token span, separate from owner-alias qualifications.
Agreeing generic inherent instances, builtin or conflicting choices and missing
terminal-token evidence remain explicit gaps. These source references do not
replace checked monomorphic calls or their authority facts.
Constructor/pattern paths and member labels carry resolved identities and exact
spans; shorthand labels are distinct from the coincident lexical binding use.
Resolved module/type qualifications are separate sites, including grouped/glob
imports and constructor patterns. Alias provenance is retained through lexical
imports, reexports and globs, even when an explicit `as` repeats the original
spelling; alias-value/type/constructor/pattern/qualification/import rows do not
rename the underlying declaration's spelling. Associated suffixes are not guessed
to be references to their owner. Generic/intrinsic qualifications, ancestor-only
visibility paths, unresolved projected/closure types, or-pattern binding joins
and unsampled member choices remain explicit gaps.

The full in-memory catalog also retains solved expression types keyed by exact
file/start/end; an absent value or conflicting same-span expansion is unavailable,
never an inferred default. Refactoring uses this source claim to retain original
numeric/type context before independently checking the candidate. Nominal type
labels use the resolver's already qualified declaration name exactly once.

Source-reference recording is selected before inference and retained by both
the concrete and generic-verification compilers. Ordinary and declaration-only
requests omit only the local-path and binding-export vectors; lexical origins,
constraints, closure capture, method selection and checked IR remain unchanged.
Initial, demanded, fallback and resumed inference obey the retained mode. Asking
for full export after capture-off inference, or mixing compiler/verifier modes,
returns `E_SEMANTIC_CAPTURE` before taking or publishing either catalog.

The current frontend checkpoint omits this evidence, so full reference/type
capture re-infers definitions with recording enabled. An ordinary image or
checkpoint alone cannot satisfy a full index; unchanged indexed image/snapshot
reuse still requires the separately authenticated full semantic snapshot. Declaration-only capture retains item/body/file byte identities and
digests, omits body binding/call-site/reference discovery, and explicitly marks
`references_recorded=false` and every item's reference completeness false.
It may resume the frontend checkpoint and serves whole-item/body operations;
it cannot authorize rename, refactoring, absence-of-uses claims or a full index.
Indexing remains explicit while its measured construction cost is high.

Checked instances name their source mapping and expanded type arguments; source
mappings remain claims. Native sizes are producer measurements of emitted function
instructions, excluding shared runtime, cleanup and literal data; zero means that
instance is absent from this image. IR operation counts refer to the original
checked program; native counts include the selected rechecked optimization.
Generic queries cover current instances, not all future substitutions. Dependents
are conservative recorded signature dependencies; remaining evidence gaps prevent
exhaustive rename proofs.

Version-three snapshots use one immutable-by-publication container. Its fixed
400-byte manifest encodes word 3, five length-framed 64-digit hexadecimal fields
(producing executable digest, source digest, exact checked IR, root-node digest,
selector), fact count, root offset, root envelope length and total container
length. Numeric words are little-endian u64 with checked host conversion.
The snapshot identity is SHA-256 of those exact manifest bytes. A 90-byte reuse
envelope authenticates them under the raw producing compiler, namespace
`semantic-container-v3` and snapshot identity; the fixed front is 490 bytes.

Nodes follow in deterministic postorder. Each is separately authenticated in
`semantic-node-v3` under its payload digest and producing compiler. A reference
contains that digest, physical offset and complete envelope length. Node payloads
are 32–32,768 bytes; offsets begin at 490 and checked ends fit both the signed
total and nonnegative i64. The root ends exactly at the authenticated total.
Every branch descriptor, including an unselected child, must end before or at
its parent's offset. Ordered radix digits, depth, leaf key hashes/prefixes,
framing and fact continuations are checked on the requested path. No whole-file
payload scan or validation of every unselected node is required for a query.

Open an existing regular container and retain its descriptor. Read/authenticate
the fixed manifest and require exact physical EOF before seeking to requested
node extents. Authenticate each fixed node header before allocating its bounded
owned payload, require exact reference-length agreement and verify its digest.
The reader's per-read capacity bounds manifest/node buffers, not the signed
container total; a larger valid container remains queryable with smaller read
settings. Already verified owned bytes may be cached even if another writer
later changes the inode; no filesystem freshness is inferred from that cache.

An explicit pin uses a separately authenticated version-three locator and a
narrow query reader; it does not read or hash the current executable. The locator
binds raw producer, selector and root digest in 104 bytes, under the snapshot key
and protocol producer SHA-256(`lkjscript-semantic-locator-3`). The answer reports
the producing digest and `validation=authenticated-producing-compiler`.
These are historical facts admitted by that producer, not an assertion that
the current checker would admit them. This reader cannot create general store
records, return `CheckedProgram`, or supply admission, image authority or gate
reuse. Ordinary builds, current authority and unpinned lookup retain exact
current-producer checks; latest selection must match the actual container front.

The writer exclusively borrows its Store while streaming a newly owned temporary
file. Before each write it reserves the full prospective staged extent, entry/
generation metadata and the larger current/prospective signed index. It seals
nodes, backpatches the fixed front, checks EOF and renames through the held parent
directories. Budget accounting charges the whole physical file, including gaps,
not just its front. Container reconciliation authenticates that bounded front
and requires its total to agree with both physical EOF and signed accounting;
an unindexed container remains an orphan, not an adoptable proof.

Publication orders the container, locator and latest before semantic/input memo
writes. After the final memo writes, a healthy writer, retained full-container
accounting, newly authenticated front/EOF and authenticated locator are required.
Eviction there refuses `E_QUERY_STORE` before printing a snapshot identity.
Later eviction/corruption remains a disposable miss. Each distinct snapshot owns
a complete stream, so physical page deduplication across snapshots is lost.

Under the writer lock, migration retires only canonical old semantic-page,
semantic-root, semantic-latest and old locator names. Old locator recognition
remains for signed budget decoding/retirement; it does not permit old queries.
Unrelated signed entries survive a valid mixed-store migration. The first
migration still visits the old population; subsequent discovery has no trusted
clean marker. Version-two pins/cursors and concurrent old-format writers are
unsupported. Historical producer queries remain supported within version three.

A query reader fixes one snapshot for its lifetime. Raw node cache keys include
digest, offset and length, with extent bounds checked before cache lookup.
Fully validated decoded fact pages and authenticated absence may be cached by
exact kind, identity and page number. Every cache miss traverses authenticated
bytes and performs range, framing and continuation checks; errors invalidate the
snapshot and stay uncached. Unlocked pinned invalidation does not delete latest;
locked invalidation preserves a newer selection.

Cursor state uses query protocol version three and authenticates the exact
snapshot/question/depth under the selected producer. It carries a bounded
depth-first traversal stack; cycles on that path are skipped. A malformed or
missing locator/manifest/node refuses before any answer is written. There is no
fallback from an explicit pin to latest, another producer or an older format.
Invalidation makes the next indexed build repair a damaged selection.

Every answer states `freshness=not-asserted`. A mutable latest pointer cannot prove
freshness against valid old-record replay, and an explicit historical pin makes
no claim about current source bytes. Editing source files does not retarget an
existing snapshot. Text remains the stored repository projection.

## Rechecked optimization and prior proofs

The inliner consumes admitted original IR and produces an untrusted candidate.
The checker validates that candidate before native emission; rejection discards
the whole candidate. Interpretation, authority reports and IR output retain the
original. Differential cases compare optimized native behavior with that original.
Copied instructions retain their original file, line and column, including traps.

A recheck may retain an original body's admission only when its exact body and
shared type/destructor/signature/ownership-contract graph retain their proof
identity. Changed bodies are checked. Stored prior proofs additionally bind
the exact producer and authenticated input/output domain; a decoded payload
or a structural-review digest is not admission authority.

Identity-addressed source transactions, bounded checked comparisons, type
representation identities and diagnostic-preserving gate closures are specified
in [structural operations](structure.md). They preserve the single checked-IR
boundary. Whole-program review equality erases diagnostic labels/positions;
execution reuse still binds exact image, runner and closed input/output contract.
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
