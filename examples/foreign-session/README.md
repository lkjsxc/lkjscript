# Persistent confined foreign workers

This specimen is a Git object worker, not an application or deployment. It is the
small proof for foreign libraries that must stay alive: foreign machine code runs
in an attached kernel-confined process; no FFI, executable pointer or foreign
unwinding enters a checked lkjscript image.

`ForeignCommand` snapshots and seals the initial ELF as before. A mutable command,
read-only root and explicit `NullDevice` start an affine `ForeignSession` through
`std::foreign_session::start`. `ForeignSession` is not Copy, Clone or Data and has
no entry grant. Its only construction is checked `foreign_start`; mutable borrows
serialize operations on one session. The read-only root descriptor is duplicated
into the child; its language borrow ends after start. No writable root is accepted.

The child retains one duplex stdin/stdout channel, the read root and `/dev/null`
for stderr. All other inherited descriptors close before foreign execution. The
existing fail-closed Landlock/seccomp profile applies: read/execute within the root,
no new sockets, descendants, metadata mutations or privilege regain. Reexecution
retains confinement. Runtime code under the root may change; path metadata,
clocks/entropy, existing special files and this explicit channel are observable.
This is neither an information-flow claim nor a filesystem snapshot. The initial
ELF's transformed interpreter bytes remain available through `foreign::identity`.

## Protocol and lifecycle

`exchange(session, request, expected_bytes, duration_ns)` is byte transport with a
caller-defined exact response size. The proof uses Git's existing batch protocol
and validates the complete response. It does not claim a semantic type identity
for Git or descriptor/shared-memory transfer to a foreign library. A future such
protocol must explicitly define and validate that boundary.

One absolute monotonic deadline covers both the request write and full response
read. Partial progress never refreshes it. Response bytes are held in a private
runtime mapping and copied to the caller only after the whole exchange completes
before the deadline. Timeout returns errno 110, kills and reaps the worker, and
poisons the session; EOF/reset returns errno 32 and also reaps it. Further exchanges
return 32, so late responses cannot become another request's result. Allocation
or invalid-duration failure before any I/O leaves the session usable. Kernel
resource refusal is a value; there is no payload or lifetime policy cap.

`finish` sends input EOF, waits under its own absolute deadline, and returns the
actual raw wait status. The result is cached; drop always kills/reaps any remaining
worker and closes every session descriptor. `PR_SET_PDEATHSIG` plus a post-install
parent check prevents an attached worker from surviving its owner. The interpreter
broker validates session identities in its own registry and reaps every session
on shutdown.

`idempotent_request` owns the restart count in the library. It retries only where
the caller explicitly names the request idempotent: a missing response does not
prove that the foreign process performed no work. A zero restart count is one
attempt. Each restart owns a fresh session and cannot receive an earlier reply.

## Evidence

`tools/programs-foreign-session-test.sh` runs the real Git worker in standalone,
native-run and independent-interpreter modes, checks authority-report terms,
complete replies, finish, EOF, timeout poisoning, transactional output and bounded
restart. Its direct IR program checks two admitted controls and 517 rejected
trials: five construction/duplication attacks plus 512 seeded applications of
eight signature/type mutations (seed 901171). It does not count those trials as
517 distinct mutation operators.

`tools/programs-foreign-session-adversarial.sh` is optional. Its C files are test
adversaries and an OS subreaper only, never runtime helpers. All three engines deny
outside reads, escaping symlinks, writes, socket creation, fork, ungranted FD/env
inheritance and privilege recovery by reexec. A partial response misses its fixed
deadline without changing the caller's buffer. Twelve owner kills observe child
pidfd death and reap every owned/adopted descendant. A responsive broker may reap
its worker first; the subreaper then records ECHILD after observing the owned
pidfd readable, rather than claiming it personally reaped that worker. Unsupported
Landlock fails before any foreign code executes.

On the shared x86-64 host, the focused suite took 9.205 s (load 5.67→5.94).
In the native-run sample, starting Git took 0.324 ms and 64 persistent requests
averaged 28.775 µs; 16 requests with a fresh spawn averaged 771.479 µs, or 26.81×
more per request (both measured within load 5.67→5.67). These are a small shared-host
sample, not a guaranteed latency or an interpreter speed claim. The scripts print
current timing and load windows on every run.
