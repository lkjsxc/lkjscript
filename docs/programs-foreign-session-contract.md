# ForeignSession checked contract

This extends the existing [foreign-process contract](../examples/foreign-child.md).
Foreign machine code stays in an attached confined process. There is no FFI,
foreign function pointer, or foreign unwinding in a checked image. The specimen
is [persistent Git object requests](../examples/foreign-session/README.md).

## Types and exact operations

IR text `type NAME foreign_session` denotes an opaque affine `ForeignSession`.
It is neither Copy nor Data; Clone, integer/aggregate construction and a copied
wrapper containing it reject independently of frontend metadata. It has no entry
grant, World constructor or public handle conversion. `foreign_start` is its only
constructor. Ordinary move, exclusive borrow and structural drop rules apply.

Here `usize` is the IR unsigned 64-bit integer. Result types are structural
`Result<T, i64>` with the usual checked Ok/Err variants.

| Intrinsic | Checked signature |
| --- | --- |
| `foreign_start` | `(&mut ForeignCommand, &mut ReadDirectory, &mut NullDevice, &[u8], usize) -> Result<ForeignSession, i64>` |
| `foreign_exchange` | `(&mut ForeignSession, &[u8], &mut [u8], usize) -> i64` |
| `foreign_finish` | `(&mut ForeignSession, usize) -> i64` |

The source receiver methods are respectively `ForeignCommand.__lkjscript_start`,
`ForeignSession.__lkjscript_exchange` and `ForeignSession.__lkjscript_finish`.
Argument/result types, reference mutability and argument counts are checked from
IR alone. Mutable borrowing serializes operations on one session; no thread or
shared-mutable handle rule is implied. The source library offers
`std::foreign_session::{start, exchange, finish, idempotent_request}`.

`start` receives argument byte vectors excluding argv[0], rejects embedded NUL,
and supplies the fixed argv[0] `foreign`. The lower intrinsic receives that
NUL-terminated argument encoding. A nonzero duration is in nanoseconds. Success
means confinement and exec completed; setup errors return positive errno in Err
only after resource cleanup and child reap. Command, root and sink remain parent
values; their exclusive borrows end when start returns. The child retains a
read-only root descriptor for the whole session, independently of that borrow.

## Exchange, failure and publication

`foreign_exchange` writes the whole request and reads exactly the destination
slice length. It returns that length or negative errno. The library allocates the
chosen response size and converts the result to `Result<Vec<u8>, i64>`. Framing is
bytes plus a caller-selected exact response size; semantic message-type identity,
capability transfer and shared-memory transfer to foreign code are not provided.

The duration becomes one absolute monotonic deadline after staging allocation
and before I/O. The same timer covers all request and response chunks; readiness
polling checks it between chunks even when I/O keeps making progress, and expiry
wins a simultaneous readiness event. Progress never renews the deadline. A final
expiry check precedes publication. Runtime-owned staging bytes are copied into
the caller's destination only after a complete timely response.

Timeout returns errno 110; EOF/reset returns errno 32. An exchange I/O failure
poisons, kills and reaps the worker. Later exchanges return 32, so a late reply
cannot become the next request's result. Allocation or invalid-duration refusal
before I/O leaves the session usable and its output unchanged. Kernel allocation
refusal remains a value; there is no payload-size or idle-lifetime policy cap.

`finish` sends input EOF and waits under its own absolute deadline. It returns the
actual raw Linux wait status or negative errno; timeout kills and reaps. A reaped
status is cached and repeated finish is idempotent. It does not consume the
session value. Drop kills/reaps any remaining child, closes the channel, pidfd
and timer, and releases runtime state. Parent-death SIGKILL plus the post-install
parent check cover owner disappearance. The independent interpreter broker owns
and validates session identities and drops its registered sessions on shutdown.

`idempotent_request` owns restart policy in the library. Its explicit count is
the maximum restarts after the initial attempt; zero means one attempt. It retries
exchange failure only after dropping the failed session and starting a fresh one.
A start failure returns directly. The name requires the caller to declare the
request idempotent: missing output does not prove no work was performed. A new
session cannot receive an old session's response.

## Authority and evidence

Authority reports name `ForeignCommand`, `ReadDirectory`, `NullDevice`, the three
intrinsics and implicit `foreign_session_kill_reap_close`. Their foreign section
is an invocation template, not an authenticated identity of each foreign worker.
The immutable transformed initial ELF can be hashed through `foreign_image`;
mutable loader/runtime files under the root are separate untrusted inputs.

Sessions explicitly retain a duplex stdin/stdout channel, the read root and the
verified null device for stderr. The initial image and setup channel close on
exec; all other inherited descriptors close. Reports distinguish this channel
from one-shot writable capture memfds, and disclose root special files, outside
metadata, clocks/entropy, fixed environment and confined reexecution. The profile
denies new sockets, descendants, filesystem metadata mutation and privilege
regain; it does not promise complete IPC/information isolation or host resource
accounting. Unsupported confinement refuses before foreign code executes.

The C-free `tools/programs-foreign-session-test.sh` covers real Git requests in
standalone, native-run and independent-interpreter modes, report terms, lifecycle,
transactional failure and bounded restart. Its direct-IR campaign admits two
controls and rejects five forging/duplication trials plus 512 seeded applications
of eight signature/type mutations. The optional adversarial script adds owned
OS witnesses, partial deadlines, confinement/reexec and owner-death cleanup.

## Foreign code inside the image: boundary cost

Foreign machine code could read or overwrite every address in the host process.
Unchecked pointers could forge capability representations and corrupt checked owners.
A source signature would not prove its actual reads, writes, aliases or lifetime.
The IR checker could not establish affine transfer or race freedom across that call.
Foreign callbacks would require a verified call boundary and ownership protocol.
Foreign unwinding and signal handlers could bypass drops and publication rules.
Arbitrary syscalls could exercise the host process's ambient kernel authority.
An honest report would therefore admit arbitrary process-wide effects and memory access.
Containing those effects would require trusting that code or isolating the whole host.
The adopted form keeps foreign code in an attached kernel-confined process.
