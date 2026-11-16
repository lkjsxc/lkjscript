# A foreign command held to a kernel profile

[foreign-status](foreign-status/program.lkj) is a program directory whose
consumer runs this host's `/usr/bin/git status`. The language now has affine
`ForeignCommand` and `NullDevice` grants, independent IR checking, native
leaves, interpreter handles, an authority report and executable cases.
It does not grant an unrestricted process launcher.

`bash tools/foreign-child-test.sh` is the default, C-free gate. It creates
fresh repositories under `target`, stages the dynamic runtime, checks the
consumer with standalone, native and interpreter execution, then removes the
fixture. `bash tools/foreign-child-adversarial.sh` adds optional C adversaries
and an owned test supervisor. C is never used by the language runtime or
required by the default bootstrap/gate.

## Entry and source contract

The consumer's reviewed grants are:

```text
tool=/usr/bin/git                    ForeignCommand
root=env:LKJSCRIPT_FOREIGN_ROOT       ReadDirectory
sink                                NullDevice
output                              Stdout
errors                              Stderr
status                              ExitStatus
```

A foreign image grant takes one absolute, NUL-free pathname, with no environment
alias. Entry acquisition pins that pathname with `O_PATH`, checks that its
inode is a regular file, reopens the pinned inode read-only, copies its bytes,
rebases its ELF interpreter path and seals the resulting memfd against write,
growth, shrink and further seal changes. A FIFO or device cannot be data-opened
as an image. The pathname can subsequently change without changing the admitted
snapshot. This is an immutable foreign ELF snapshot, not an authenticated
lkjscript image or a checked authority manifest.

`NullDevice` has no value argument. It pins `/dev/null`, verifies a character
device with major 1, minor 3, then reopens that same inode for read/write.
The profile needs this separate grant: the permanent strict-root-only C case
shows that this Git opens `/dev/null` even when stdin is an empty pipe.
Without the device exception it refuses to run. The exception is explicit in
the authority report.

The checked library offers:

```rust
std::foreign::run(
    &mut tool, &mut root, &mut sink, &arguments, duration_ns
) -> Result<Outcome, i64>
```

Arguments are owned byte vectors, exclude argv[0], and cannot contain NUL.
The wrapper supplies the fixed argv[0] `foreign`. There is no shell expansion.
The nonzero `usize` duration is nanoseconds. The three affine capabilities
remain exclusively borrowed by the synchronous operation until the child has
been reaped, and are usable again afterward.

`Outcome` contains `exit_code: Option<u8>`, `signal: Option<u8>`,
`timed_out: bool`, `output: Vec<u8>` and `errors: Vec<u8>`.
A launch/setup failure returns its positive errno after cleanup and reap.
A program that successfully starts and exits 125 is an ordinary exit value;
it cannot forge a launch error by writing its stdout.

The lower-level `tool.run(...)` returns an immutable `ReadFile` capture.
Its record contains four little-endian u64 words: raw wait status, timeout
flag, stdout length and stderr length, followed by the two byte sequences.
`std::foreign::decode` checks that record and constructs `Outcome`.
The result is never a writable `File`, and neither new capability can be
copied, forged from bits, minted through `World`, or substituted for `Process`.

## Runtime files are part of the read root

The consumer root includes the repository and these staged assets:

```text
R/.runtime/ld.so
R/.runtime/libc.so.6
R/.runtime/libpcre2-8.so.0
R/.runtime/libz.so.1
```

The current host's Git needs those libraries. The test scripts copy them from
the host into the fresh fixture. Other foreign programs must stage whatever
their own runtime needs; no ambient `/lib` or `/usr/lib` read exception is
silently supplied.

For a dynamic ELF, the admitted snapshot's PT_INTERP refers to
`/proc/self/fd/5/.runtime/ld.so`. Descriptor 5 retains the borrowed read root
across exec and reexec, so a host pathname replacement cannot redirect that
anchor. The original image's PT_INTERP bytes are not the executed loader path.
The snapshot identity therefore differs from the original installed file.

Only these seven environment entries reach the child, in this order:

```text
LD_LIBRARY_PATH=/proc/self/fd/5/.runtime
HOME=/proc/self/fd/5
GIT_CONFIG_GLOBAL=/proc/self/fd/5/.no-global-config
GIT_CONFIG_NOSYSTEM=1
GIT_OPTIONAL_LOCKS=0
GIT_TERMINAL_PROMPT=0
LC_ALL=C
```

There is no inherited PATH or inherited environment. The root's runtime files
and configuration are separate, mutable, untrusted inputs. They are confined
by the same kernel policy; sealing the initial ELF does not authenticate them.

`tool.image()` returns a read-only view of the sealed transformed image.
`std::foreign::identity(&mut tool)` hashes those exact bytes with the checked
SHA256 library. A native proof independently streamed `image()` to a fresh
file and compared the helper to host `sha256sum`:

| Host observation, 2026-11-15 | SHA256 |
| --- | --- |
| Original `/usr/bin/git` | `2a8c18fbf43da9f692d75474c72bea9dfd796c260b0f3dfe456376abc3bbd668` |
| Sealed transformed image, 4,066,263 bytes | `f2e6815387a2e9cc35e4de6e331dc954f933a43be3bced41dd46682a977969a5` |

The helper and host digest matched. Build, acquisition, hash and streaming took
338 ms, load 11.76 / 5.97 / 5.80 before and after. These identify that measured
host image; they are not universal Git identities or hashes of the runtime
directory. The authority command reports the grant template and fixed policy.
It does not pretend to bind an invocation's concrete path, runtime assets or
image hash.

## What the kernel holds

The measured host is Linux 7.3.4 with Landlock ABI 11. The implementation
requires ABI 9 or later and refuses unsupported setup. It installs
`no_new_privs`, Landlock and an x86-64 seccomp allowlist in the owned child
between clone and exec. The native emitter and the descriptor broker have
separately owned implementations; the interpreter does not call the native
compiler's checker or lowering to grant the operation.

The child imports exactly:

| Descriptor | Authority |
| --- | --- |
| 0 | Verified null device, read/write; reads return EOF |
| 1 | Private stdout capture memfd |
| 2 | Private stderr capture memfd |
| 3 | Sealed initial image; closes on successful exec |
| 4 | Private setup-error channel; closes on successful exec |
| 5 | Borrowed read-root descriptor, retained across exec |

The import closes every other inherited descriptor. The child cannot reopen
its parent's descriptors through proc: the optional campaign checks owned
parent descriptor aliases as well as ordinary inherited-fd closure.

Landlock permits pathname file-content reads and execution beneath R, plus
read/write of the verified null device. It denies other handled file-content
access, handled filesystem mutation and handled network access. The seccomp
allowlist additionally refuses every socket, connect, accept, send/receive and
socketpair syscall; fork/vfork/clone/clone3; ioctl and mknod; ptrace and
io_uring; namespace changes; credential changes; and later prctl changes.
Compatibility syscall architectures and x32 calls are refused. There are no
passed socket descriptors from which network access could be recovered.

The root is a kernel subtree grant. Unlike `ReadDirectory.open`, it includes
preexisting special files under R: reading an in-root FIFO or device can have
effects that regular-file reads do not. This profile is not a claim of no IPC,
no side channels, or regular-file-only access. Outside stat/readlink metadata,
ordinary process metadata, clock and entropy remain observable.

Private anonymous captures are also explicit authority. The child can read,
rewrite, seek and truncate its own stdout/stderr memfds. Landlock does not
treat these anonymous files as pathname content under R. The optional campaign
also executes bytes copied into a capture through the permitted loader: the
same file, socket, fork and privilege denials still hold afterward.
Consequently the report says that reexec remains confined; it does not claim
that every later instruction or image must come from a named file under R.

The syscall/file rules do not describe host-side cgroup accounting, existing
namespace placement, resource charging, core-dump services or other crash
handling. Those remain host policy. There is no new cgroup, namespace or
machine-wide denial-of-service boundary here, and no claim that a crash has
zero host-side effects. Test signal cases disable core files in their owned
test supervisor.

## Deadline and parent lifetime

The absolute monotonic deadline is computed at ForeignRun entry, before
argument preparation, capture allocation, confinement setup or clone. It is
not reset on exec or reexec. Image grant acquisition occurs earlier during
entry startup and is outside this child-run deadline.

The parent obtains a pidfd atomically with the clone, arms an absolute timerfd,
and waits for exit or deadline. At the deadline it sends SIGKILL through the
pidfd and reaps the child before returning. Every setup failure follows the
same cleanup discipline. A deadline is a kill deadline followed by reap;
scheduling delays or an uninterruptible kernel wait can delay completion.
This is not a hard real-time return bound.
Capture materialization and source decoding can continue after the child exits.

Before exec the child installs parent-death SIGKILL and checks that its parent
did not disappear during setup. Later changes to that setting, credentials
and process creation are denied. There is no detached mode. On owner death
the kernel kills the child; a live owner never returns a completed run before
reaping it. The optional supervisor adopts and reaps children when testing a
deliberately killed owner.

The campaign repeats that owner-death check after reexec of an owned adversary
whose on-disk mode is 6755. Under no_new_privs on this host, the marked reexec
preserved the parent-death behavior. Ordinary reexec and anonymous-capture
reexec also retain the confinement profile.

## Verification and loop cost

| Suite | Coverage | Result and load |
| --- | --- | --- |
| `tools/foreign-child-test.sh` | No C/Rust compilation; standalone/native/interpreter Git consumer, outside-root repository refusal, read-only image view, arguments, authority, FIFO image refusal before data-open | PASS, 3.379 s; 8.24 / 6.60 / 6.24 -> 8.30 / 6.64 / 6.25 |
| `tools/foreign-child-adversarial.sh` | All three engines; kernel denial attacks, parent proc-fd aliases, confined and anonymous reexec, capture mutation, binary stdout/stderr, exit/signal, 100 ms deadlines, loader errors, unsupported host, immutable image after source replacement, ordinary and 6755-marked owner death/reap | PASS, 15.311 s; 5.47 / 6.01 / 6.05 -> 6.40 / 6.21 / 6.11 |

The optional campaign includes the C feasibility proof and its negative
strict-root-only Git case. All fixtures and children belong to the campaign;
it leaves no fixture directory or running process.

On a host where Landlock setup is unavailable or prohibited, the default
suite requires an explicit refused-execution outcome and prints that the
kernel was unavailable. It does not substitute an unrestricted child or
claim positive confinement coverage there. The optional campaign requires
the supported profile and tests forced ENOSYS refusal as an additional case.

The IR and source cases also reject copied foreign authority, a writable
root in place of ReadDirectory, a generic File in place of NullDevice,
writable capture results, signed durations, and malformed image grants before
host acquisition. The optional Rust witness has private typed stubs returning
an explicit unsupported error; it never uses unconfined spawning as a substitute.

Implementation: [checked wrapper](../library/foreign.lkj),
[native leaves](../src/native/foreign.lkj),
[independent broker leaves](../src/host_network/foreign.lkj),
[consumer](foreign-status/main.lkj), and
[adversarial source fixture](../tests/fixtures/authority/foreign_command.lkj).
Kernel references: [Landlock](https://docs.kernel.org/userspace-api/landlock.html),
[seccomp filtering](https://docs.kernel.org/userspace-api/seccomp_filter.html),
[no_new_privs](https://docs.kernel.org/userspace-api/no_new_privs.html), and
[parent-death signal](https://man7.org/linux/man-pages/man2/PR_SET_PDEATHSIG.2const.html).

## What cxloop still needs

This profile cannot run cxloop unchanged: it needs writes and descendants.
Codex and Claude also need a home directory and network access.
Those are different, broader authorities, each of which must appear in a report.
Landlock port rules alone cannot honestly promise a particular remote hostname.
A future profile must name writable roots, secret inputs, network scope and child lifetime.
Until those policies are implemented and proved, those tools remain host compositions.
Authenticated lkjscript children separately await compiler image identity, a comparable entry-authority manifest and descriptor-only startup.
This foreign ELF profile supplies none of those proofs, and does not start cxloop.

## Long-lived foreign code

The same outside-image boundary applies to a worker retained for the parent's
lifetime. [ForeignSession](../docs/programs-foreign-session-contract.md) specifies
the exact affine operations, per-request deadline, transactional response and
library restart policy. The [Git batch specimen](foreign-session/README.md)
compares persistent requests with one spawn per request. Its channel currently
carries bytes; passed descriptors and shared regions for foreign workers remain
an explicit gap. The [ten-line in-image boundary cost](../docs/programs-foreign-session-contract.md#foreign-code-inside-the-image-boundary-cost)
explains why native FFI is not admitted to the checked image.
