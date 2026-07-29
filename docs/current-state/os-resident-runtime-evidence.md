# OS-Resident Runtime Foundation Evidence

## Status

**Current on Linux x86-64** for the exact foreground coordinator, exclusive
state-directory lease, database-independent control store, authenticated Unix
control, describe/status/stop client, and deterministic service-definition
slice named here. App-private host environments, portable application paths,
provider-backed VM stdio/clock, and trusted arguments/stdio applications are
also Current for their exact tests. Supervised process cells and durable process
application control are Current for the exact Linux evidence below. One daemon-
owned ordered database and incarnation-bound tenant attachment are Current.
[Session presence](session-broker-evidence.md) is Current; database VM operations, interactive cells, and GUI are not.

## Implemented Boundary

`lkjscriptd --foreground` is a persistent process and the sole coordinator for
one state directory. It requires an explicit state directory, configured
principal, and nonzero coordinator identity. Atomic create-new lease acquisition
rejects a second live coordinator. Linux stale recovery checks the recorded
process under `/proc` once before replacement. Other hosts conservatively treat
an existing lease as live.

The coordinator opens `lkjscript.control-store` before any application database.
Its journal and snapshot carry platform revision 8, the exact
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
operations include describe/status/shutdown plus application install, list,
start, stop, restart, remove, and invoke. Unknown, partial, oversized, stale,
wrong-digest, malformed, and replay-conflicting frames fail closed.
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
cargo test --locked -p lkjscript-database
cargo test --locked -p lkjscript-runtime
cargo test --locked -p lkjscript-app --test cli_contract application_control
cargo test --locked -p lkjscript-app --test cli_contract process_cells
cargo run --locked -p lkjscript-xtask -- check-unsafe
cargo clippy --locked -p lkjscript-host -p lkjscript-database \
  -p lkjscript-runtime -p lkjscript-app --all-targets -- -D warnings
```
The runtime suite covered first boot, clean and unclean reopen, commit,
checkpoint, truncated-tail repair, complete-record corruption, sync failure,
stale platform revision, lease exclusion, exact frames, malformed prefixes,
stale/digest rejection, Unix peer credentials, idempotent replay, service
bundle generation/removal, private trusted VMs, bounded process framing, and
manifest/cell-class mismatch rejection.

The application-control integration built `lkjscriptd`, `lkjscript-cell`, and
`lkjscript`; started the daemon; installed, started, invoked, listed, stopped,
and removed a real process application; killed and restarted the daemon; and
observed unclean desired-state recovery plus an attached database tenant on the
replacement incarnation. Exact describe output now reports platform revision 8
and runtime-control digest
`5ca07048677f164ef9d25b8fc65a455b670e7e385b94d850692204fafe1a3305`.

A separate CLI smoke generated all six service files, required each to be
nonempty, removed them, and observed no remaining generated file.

## Host Provider Cut

`lkjscript-host` now composes narrow stdio, clock, logging, cancellation,
directory, and database-provider families in one cloneable app-private
`HostEnvironment`. `ApplicationPath` accepts only bounded normalized relative
segments. Portable directory conformance covers contained read, write, list,
and remove. The ordered database now implements the typed family with tenant-
bound read/write handles, immutable snapshots, exact ranges, single-writer
commit, abort, and abort-all. Provider identity binds tenant and incarnation;
foreign, stale, missing, and read-only handles fail before database effects.

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
One FIFO coordinator ticket plus coordinator/app concurrent and total ceilings
run before both cell classes. Focused two-app evidence observed global peak one,
no starvation, exact total rejection, per-app metrics, and bounded logs.

## Durable Application Control

The bootstrap store now retains stable monotonic registry identities, full
package digests, canonical package roots, normalized entries, grants, quotas,
and desired stopped/running state. Recovery validates every record in identity
order, reconstructs each process application with the daemon's fixed sibling
worker, and starts desired-running records. It fails coordinator startup rather
than skipping malformed or unstartable records.

Authenticated local control and `lkjscript system app` implement install, list,
start, stop, restart, remove, and invoke. Invoke returns the exact closed outcome
and only that invocation's bounded buffered stdout. Focused Linux evidence
installed registry identity 1, invoked real validated source, killed and
restarted the daemon from the same state directory, observed unclean recovery,
and recovered the app as running with a
new worker and attached database tenant, invoked it again, then stopped,
observed tenant detachment, and removed it.

## Database Tenant Lifecycle

After bootstrap control recovery, `lkjscriptd` opens or creates one ordered
database under the state directory and attaches its factory to the coordinator.
Stable registry identity derives the tenant; each running incarnation receives
a fresh provider identity. Start and restart attach before control success.
Stop, restart, remove, process failure, shutdown, and provider drop abort active
transactions. Clean shutdown then checkpoints; abrupt death recovers the WAL.

Focused provider evidence proved immutable pre-commit snapshots, post-commit
visibility, cross-tenant isolation, stale/foreign handle rejection, abort-all
rollback, and writer release. The complete database suite also proved exact
model agreement, synchronized WAL recovery, torn-tail handling, atomic
checkpoint close, and failed-I/O nonpublication. Language database operations
and process-cell provider proxy framing remain outside this Current slice.

## Exact Limits

- The coordinator currently admits one configured Linux principal only.
- No Windows named pipe or macOS native control execution exists.
- Generated service definitions were not installed or started with privilege.
- Shutdown is control-driven; OS signal mapping is not yet Current.
- Only arguments, direct stdio, and clock operations use the composed VM host;
  database tenant attachment exists at the coordinator boundary, but database
  process proxy and VM operations, interactive cells, native windows, graphics,
  accessibility, and GUI execution remain absent.
- Persistent reconstruction is isolated-process only; trusted in-process code
  artifacts are not serialized by the registry.
- No Miri, sanitizer, fuzz campaign, non-Linux build, or non-x86 execution was
  run for this slice.
