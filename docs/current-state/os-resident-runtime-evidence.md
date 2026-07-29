# OS-Resident Runtime Foundation Evidence

## Status

**Current on Linux x86-64** for the exact foreground coordinator, exclusive
state-directory lease, database-independent control store, authenticated Unix
control, describe/status/stop client, and deterministic service-definition
slice named here. App-private host environments, portable application paths,
provider-backed VM stdio/clock, and trusted arguments/stdio applications are
also Current for their exact tests. Supervised isolated process cells are Current
for the exact Linux evidence below. Application persistence, database attachment,
session brokers, and GUI remain non-Current.

## Implemented Boundary

`lkjscriptd --foreground` is a persistent process and the sole coordinator for
one state directory. It requires an explicit state directory, configured
principal, and nonzero coordinator identity. Atomic create-new lease acquisition
rejects a second live coordinator. Linux stale recovery checks the recorded
process under `/proc` once before replacement. Other hosts conservatively treat
an existing lease as live.

The coordinator opens `lkjscript.control-store` before any application database.
Its journal and snapshot carry platform revision 4, the exact
`lkjscript.runtime-control` digest, explicit little-endian widths, monotonic
sequence, bounded key/value payloads, and full SHA-256 checksums. Commit syncs
before fact publication. Recovery replays idempotently, rejects corruption and
sequence gaps, and repairs an incomplete final frame before future append.
Checkpoint atomically publishes a canonical snapshot before journal reset.

First boot persists coordinator identity and clean-shutdown state. Reopen
rejects identity, platform-revision, or control-digest mismatch. Clean shutdown
writes and checkpoints the marker. A dropped coordinator leaves the marker
unclean while control recovery remains independent of database availability.

## Local Control

Linux uses a mode-0600 Unix-domain socket. The registered host unsafe boundary
calls `SO_PEERCRED` behind a safe typed API and admits only the configured kernel
user identity. A request field cannot supply principal authority.

Frames have one exact bounded length, platform revision, contract digest,
nonzero request identity, idempotency digest, and closed operation. Current
operations are `describe`, `status`, and `shutdown`. Unknown, partial, oversized,
stale, wrong-digest, malformed, and replay-conflicting frames fail closed.
A bounded cache returns the prior result for an exact modifying replay.

`lkjscript system describe|status|stop --endpoint PATH` is a real client of the
daemon. It never starts a second authority.

## Service Definitions

`lkjscript system install --output DIR --principal UID --coordinator ID`
generates and synchronizes six deterministic definitions:

- Linux system `lkjscriptd.service`;
- Linux graphical-session `lkjscript-session.service`;
- Windows noninteractive service command contract;
- macOS LaunchDaemon property list;
- macOS per-user LaunchAgent property list; and
- container foreground command.

`system uninstall --output DIR` removes only those exact generated files and
synchronizes the directory. These commands do not claim privileged native
installation, signing, SCM, launchd, or systemd execution. Windows Session 0
and macOS LaunchDaemon definitions never present UI; interactive execution is
assigned to the subordinate session broker contract.

## Focused Evidence

Executed on Linux x86-64 at the implementing worktree:

```text
cargo test --locked -p lkjscript-host
cargo test --locked -p lkjscript-core outcome::codec
cargo test --locked -p lkjscript-runtime
cargo test --locked -p lkjscript-app --test cli_contract process_cells
cargo run --locked -p lkjscript-xtask -- check-unsafe
cargo clippy --locked -p lkjscript-host -p lkjscript-runtime -p lkjscript-app --all-targets -- -D warnings
```

The runtime suite covered first boot, clean and unclean reopen, commit,
checkpoint, truncated-tail repair, complete-record corruption, sync failure,
stale platform revision, lease exclusion, exact frames, malformed prefixes,
stale/digest rejection, Unix peer credentials, idempotent replay, service
bundle generation/removal, private trusted VMs, bounded process framing, and
manifest/cell-class mismatch rejection.

A real process smoke built `lkjscriptd` and `lkjscript`, started the daemon,
waited for its Unix endpoint, ran describe and status through the CLI, requested
shutdown, joined the daemon, and observed a zero-byte journal plus a 193-byte
durable snapshot. It reported coordinator 904, platform revision 4, exact control
digest `754118950895d44b32a89a23ac895188b41baf663f44b23e81b2764758e07b8d`,
clean previous shutdown, control sequence 2, and zero applications.

A separate CLI smoke generated all six service files, required each to be
nonempty, removed them, and observed no remaining generated file.

## Host Provider Cut

`lkjscript-host` now composes narrow stdio, clock, logging, cancellation,
directory, and database-provider families in one cloneable app-private
`HostEnvironment`. `ApplicationPath` accepts only bounded normalized relative
segments. Portable directory conformance covers contained read, write, list,
and remove. The database family is a typed interface only; its runtime binding
is not Current in this slice.

The validated VM routes direct print, flush, read-byte, write-byte, write-string,
monotonic time, and wait operations through granted stdio or clock providers.
A matching capability without a provider fails before ambient effect. Existing
file, terminal, network, SQLite, and stream-resource operations still retain
inherited sys paths and are explicitly outside this cut.

The runtime admits sorted exact `arguments`, `stdio`, and `clock` requirements
only when package grants and provider availability match. Focused concurrent
execution used a private buffered stdio provider and separate VM state; an
unsupported filesystem grant failed before application publication. The
argument and stdio applications both executed validated bytecode through real
VM entry.

## Isolated Process Cells

A manifest now declares `trusted-in-process` or `isolated-process` with one
normalized relative package entry. Installation rejects a code/class mismatch,
escaping or non-file entries, unavailable providers, and an unbound worker
before application publication. The application cannot select its executable.

The fixed `lkjscript-cell` worker receives a cleared environment and private
pipes, validates platform revision and runtime-control digest, verifies the
package lock, compiles the entry once, and runs a fresh validated VM for each
invocation. Four-byte length frames and all fields are bounded. Application
stdio uses a worker-private buffer and is relayed to the app-private parent
provider; it cannot corrupt protocol output. A lossless core codec transports
all closed execution outcomes and rejects unknown tags, trailing bytes,
inconsistent cleanup accounting, and oversized data.

Focused Linux x86-64 integration executed the factorial hello application twice
in one worker, observed exact output and flush counts, stopped it, restarted it
with a new incarnation and process, and rejected the stale incarnation. A second
test started two applications, sent `SIGKILL` to one worker, observed only that
application enter `failed`, then invoked and stopped the surviving application.
Existing ticket admission, per-app concurrent/total quotas, metrics, and bounded
logs run before both cell classes.

## Exact Limits

- The coordinator currently admits one configured Linux principal only.
- No Windows named pipe or macOS native control execution exists.
- Generated service definitions were not installed or started with privilege.
- Shutdown is control-driven; OS signal mapping is not yet Current.
- Application registry facts are not yet reconstructed into runnable code.
- Only arguments, direct stdio, and clock operations use the composed VM host;
  tenant attachment, session brokers, native windows, graphics, accessibility,
  and GUI execution remain absent.
- Process apps are not yet persisted or reconstructed by `lkjscriptd`, and the
  daemon service definitions do not yet bind a worker executable path.
- No Miri, sanitizer, fuzz campaign, non-Linux build, or non-x86 execution was
  run for this slice.
