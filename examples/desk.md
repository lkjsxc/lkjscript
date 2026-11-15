# Desk command and host composition

Desk owns its notebook, reads two named roots, and listens on one loopback port.
The host publishes the board and starts the relay. The installed Python desk
remains live until Claude performs the separately reviewed switch.

## Host file contracts (third note, 2.1)

These contracts are fixed for the cxloop writer and host supervisor.

**Board.** The named read-only root `board` is
`/home/coder/.local/state/cxloop` (the default `CXLOOP_STATE`).
Read `board.txt` once per state request, through one opened descriptor:
UTF-8, first line `desk-board 1 <writer-unix-seconds>`, remaining lines are the
current `cxloop board` text. Empty lines and lines starting `handoff:` are
omitted from `jobs`, matching the page contract. The writer updates this file
on every state change by creating a temporary file in the same directory,
writing the entire snapshot, closing it, and atomically renaming it to
`board.txt`. Durable writers sync the file before rename and the directory
after. A read concurrent with replace sees one complete old or new snapshot.
Desk has no board cache, child process, or board subprocess deadline.
The age bound is **30 seconds**; stale content remains visible with its age.
Missing, invalid UTF-8, or malformed-header snapshots are explicitly unavailable.

**Handoff.** The separate named read-only root `handoff` is
`/home/coder/.claude/projects/-home-coder-workspace/memory`.
Select the greatest descriptor modification time among regular
`handoff-*.md` files; filename breaks equal-time ties. Inspect metadata and
read from that same opened file. An entry disappearing during enumeration is
skipped. Empty directories return an empty detail. No writable directory or
file capability can be obtained from either read-only root.

**Listener.** The delivered interface is the two marker subcommands
`desk listen-begin` and `desk listen-end`. They refresh only `alive` and never
start or supervise a command. The external caller owns CMD and holds
`flock "$DESK_STATE/listening"` continuously: acquire before `listen-begin`,
run CMD with the lock descriptor closed in CMD, then invoke `listen-end` before
releasing the lock. The caller owns child termination, reaping and cleanup under
its host policy. A stale alive timestamp is not a substitute for the listener lock.

The delivered `desk-command` binds literal state/TZ defaults and execs the native
CLI, preserving arguments, standard streams and exit status. Defaults apply only
to unset selectors; explicit empty values remain empty, so the native entry
rejects an empty state selector. Defaults do not inspect HOME. Callers using
`desk listen CMD...` must adopt the marker contract before the switch. Claude
owns that cxloop/caller adaptation.

**Ping and wake.** Accepting an owner message always updates `ping` under
the shared `.lock`, as it does today. A nonblocking exclusive probe of
`listening` distinguishes a live listener from nobody listening. Only when
nobody holds it does desk publish `wake`, also under `.lock`.
Its UTF-8 contract is `desk-wake 1 <inbox-row-count> <row-id>\n`.
It names a durable inbox high-water mark, not a command or executable.
Retries repair a missing publication without changing the accepted row ID;
an unchanged pending request is not rewritten. Multiple pending messages
coalesce to the newest high-water mark.

The host's `desk-wake.path` uses `PathExists` on `wake`; its service starts
the reviewed relay. The relay must be idempotent by the row ID/high-water mark.
After accepting that work, under the same `.lock`, it atomically publishes
`wake-ack` containing the acknowledged inbox count plus newline, syncs it,
and removes `wake` only if its count is no newer than that acknowledgement.
A later request must survive an older relay's cleanup. Desk never republishes
a count at or below the acknowledgement. Relay start and durable acknowledgement
cannot be one atomic transaction: after a kill, a relay invocation may repeat.
The promise is a durable, retryable notification with idempotent consumption,
not exactly-once process execution. Claude owns the cxloop side and installation.


## Program directory and entries

`target/lkjscript build examples/desk` builds the reviewed service in
`program.lkj`; `authority examples/desk` reports its exact grants, and
`test examples/desk` checks its declared cases. The manifest uses ephemeral
loopback port zero for isolation. The production build in
[desk-switch.md](desk-switch.md) changes that one reviewed binding to 8911.
The source is `main.lkj`, with arguments, store, CLI, web and composition modules
beside it. The directory's focused cases use the production argument/durability
parser; the complete module tests remain in the explicit crash campaign.
Shared HTTP code is source library code, also used by site and journal; all three
consumers now have program directories.

The command has a separate entry, so a note never binds a listener:

~~~sh
target/lkjscript build examples/desk/main.lkj --library-dir library -o target/desk \
  --grant args --grant state=env:DESK_STATE --grant state_env=DESK_STATE \
  --grant timezone=TZ --grant output --grant errors --grant input \
  --grant clock --grant entropy --grant exit
~~~

The existing state root must be named explicitly by `DESK_STATE`. The command
supports note, ask, close, inbox, inbox --peek, path, listen-begin and listen-end.
Arguments retain exact bytes until a command requires UTF-8. Only the granted
`DESK_STATE` and `TZ` environment values are available; there is no HOME lookup
or environment enumeration. `--stdin` appends UTF-8 input to note/ask text.
Normal status is an affine capability and preserves cleanup. Fixed-offset time
is UTC by default, or explicitly JST-9 / +/-HH:MM.

## Append, acknowledgement and durability

Both implementations share `.lock`, `notes.jsonl`, `inbox.jsonl`, `seen`,
`ping` and `alive`. Writers hold the same blocking flock. IDs combine realtime
nanoseconds with kernel entropy. The old JSONL prefix is never copied, reparsed
or rewritten by an append. A healthy log costs one final-byte inspection, one
O_APPEND write, and the chosen durability step. First creation additionally
syncs the directory in sync mode.

`--durability=sync` is the default. A successful ID acknowledges the appended
file's fsync. `--durability=buffered` explicitly requests Python-equivalent
write/close durability and makes no power-loss promise. `--timings` prints the
selected policy and arguments, lock, row construction, open/recovery, append,
sync, alive and total nanoseconds to stderr. Liveness publication is deliberately
unsynced: it is neither a durable acknowledgement nor evidence of a listening
process. Both policies use the same declared file authority. The compiler's
authority report lists reachable effects, not the caller's runtime policy;
the selected `sync` or `buffered` value is recorded by `--timings`.

A short write is rolled back while the lock is held. After a kill during a
write, the next native operation removes only an incomplete suffix, scanning
backward to the last newline before UTF-8 decoding. Previously acknowledged
rows and the raw prefix remain unchanged. Recovery can cost the length of the
interrupted row; ordinary appends are flat in historical log size. A Python
reader must run after that recovery if a kill left a torn final row. This is a
different crash boundary from the former whole-file replacement protocol and
is tested explicitly.

Other small state files still use a fresh private temporary inode, fsync,
atomic rename and directory fsync. Final symlinks are refused on append, and
alive is atomically replaced without following a preexisting symlink.
Owner request IDs and first-answer references deduplicate retries. Retrying a
durable inbox row repairs ping and the wake marker; a read/acknowledged message
does not recreate either. As before, inbox without --peek records seen before
printing; agents use --peek.

## Live page and reconnect contract

The page retains the original routes and data: GET state (including proxy
prefixes), GET other paths for the page, and POST say with X-Desk:1. Board and
handoff are populated by the two read roots. Every ordinary state request
opens the board afresh. The page displays missing/invalid/unavailable status
and the age of stale board content.

The page receives server-sent events on GET events. Desk's commands already use
POST, so this consumer only needs server-to-page delivery. The reviewed older
lkjournal application has bidirectional WebSocket subscriptions; migrating that
separate protocol is not claimed here.

Each SSE ID contains the two committed JSONL byte offsets. A frame contains every
new note/inbox row since those boundaries and a coherent complete state snapshot.
The snapshot and both offsets are read under one writer lock. Invalid, mid-line
or beyond-end cursors explicitly reset to full replay. Events are cached only
within one observed revision; read-only state routes do not use that cache.
The page commits a cursor only after applying the frame and deduplicates row IDs.
DOM application must finish before its render cache, snapshot cursor or initial
polling cursor advances; a failed render can therefore retry the same frame.
Reconnect can replay bytes, while each row is applied once and no committed row
between acknowledged cursors is skipped.

One 250 ms service tick checks log sizes/metadata; a changed revision schedules
only subscribers. Idle ticks never walk all peers. External board/handoff
metadata is checked once per second, and the page advances displayed age locally.
Each pending frame has a fixed two-second write deadline; tiny writes do not
renew it. Slow readers are disconnected, then replay from their last applied
cursor. Unsupported/failed event connections fall back to 800 ms polling while the page is visible.
Closed-stream callbacks cannot replace newer connections, and BFcache resumes
from the retained cursor.

## Units and handoff

[desk.service](desk.service), [desk-wake.path](desk-wake.path) and
[desk-wake.service](desk-wake.service) are delivered for review only.
The service has a read-only system/home view with one writable state exception,
no capability set, private devices/tmp, namespace/SUID restrictions and W^X.
The host's global service drop-in resets some requested settings; the transient
proof prints the effective properties instead of asserting the text is enforced.

The exact-unit proof passed in **7.953 s**, load **13.28 -> 13.74**: both live
GET-only parity rounds, same-port restart and a relay crash followed by an
idempotent path-unit retry. The effective unit retains ProtectSystem=strict,
ProtectHome=read-only, private devices/tmp, an empty capability set and W^X;
the host-wide overrides disable NoNewPrivileges, ProtectKernelTunables and
ProtectControlGroups. An openat2 ENOSYS refusal on this host uses the anchored,
symlink-rejecting fallback, with the same read-root containment.

The wake unit deliberately owns broader host authority for the reviewed cxloop
relay. Its [wrapper](desk-wake) passes `CXLOOP_WAKE_ID` and
`CXLOOP_WAKE_COUNT`, keeps host launches serialized and closes its lock
descriptors in the relay. The relay must durably honor that idempotence key
before installation. Desk itself cannot launch the wrapper, cxloop or any child.

[desk-switch.md](desk-switch.md) contains 24 switch/rollback commands and keeps
the live state root in place. Claude performs the switch and removes Python
after verification. No installed service or agent-toolkit file is changed by
these proofs.

## Verification

~~~sh
bash tools/http-desk-test.sh --compiler target/lkjscript
bash tools/http-desk-events.sh --compiler target/lkjscript
bash tools/http-desk-test.sh --compiler target/lkjscript --oracle /home/coder/workspace/agent-toolkit/bin/desk --measure
bash tools/http-desk-crash.sh --compiler target/lkjscript --oracle /home/coder/workspace/agent-toolkit/bin/desk
bash tools/http-desk-events.sh --compiler target/lkjscript --campaign
bash tools/http-desk-systemd.sh target/lkjscript
~~~

The first two are short gate suites. The desk suite includes typed read-root
cases and the host-wrapper proof; `bash tools/http-desk-wrapper-test.sh` runs
that shell-only subset independently. It uses a copied wrapper with a foreground
stand-in image and fresh state to check defaults, explicit overrides, exact
arguments, standard streams, exit status and marker forwarding. Long kill/load campaigns and transient
unit/live-GET proofs remain outside the gate. Every test creates fresh roots and
loopback ports, starts only its own children, and removes/reaps them. The live
parity tool uses GET only on 8911 and reconstructs the complete visible
feed/asks/board/handoff in a private fixture before comparing the candidate.
Page bytes match the reviewed candidate source; the intentional SSE and board-age
additions explain the difference from Python's page.

Typed read-root cases run standalone, native-run and independent interpretation.
They prove containment, regular-only data opens, FIFO refusal, subdirectory
escape refusal, same-inode timestamp/content across replace, pre-epoch timestamp
overflow, append seek, and symlink refusal. Forged IR cannot write/chmod a
ReadFile, open a writable result from ReadDirectory, or copy either affine value.

## Measured rewrite cost and gain

The following 100-command samples include process launch, output redirection and
all storage work. Each starts from the same fixture size; Python is the unchanged
installed executable with fresh test state. Times are arithmetic means, not
latency gates. The delivered rows invoke a copied desk-command with only its
native image path substituted; shell startup, binding setup and exec are included.
Direct-image rows show the cost without that wrapper. Both native paths request
the phase report, whose stderr write is included in the end-to-end sample.

Each cell gives **mean milliseconds (load before -> after)**.

| Invocation | 0 initial rows | 10,000 initial rows | 100,000 initial rows |
| --- | ---: | ---: | ---: |
| Direct image, sync | 4.119 (10.00 -> 10.00) | 4.238 (9.52 -> 9.52) | 4.060 (9.52 -> 9.52) |
| Direct image, buffered | 0.540 (10.00 -> 10.00) | 0.559 (9.52 -> 9.52) | 0.588 (9.52 -> 9.52) |
| Delivered wrapper, sync | 4.912 (10.00 -> 10.00) | 4.748 (9.52 -> 9.52) | 10.252 (9.52 -> 9.88) |
| Delivered wrapper, buffered | 0.723 (10.00 -> 10.00) | 1.101 (9.52 -> 9.52) | 0.741 (9.88 -> 9.88) |
| Python, buffered | 19.608 (10.00 -> 9.52) | 22.112 (9.52 -> 9.52) | 28.659 (9.88 -> 9.88) |

Path-only launch/control was **0.360 ms direct image / 0.571 ms delivered
wrapper / 19.887 ms Python**, all at load **10.00 -> 10.00**. Delivered buffered
note internal work was **0.101-0.156 ms**, while the remainder was
**0.622-0.945 ms**, at the corresponding delivered-row load windows above.
The native phase clock starts inside the image. The remainder includes process
launch, pre-entry setup, the shell wrapper, final output and waiting; it is not
a separate executable-loader or shell-only measurement.

The following phases come from the **delivered wrapper** invocations. Buffered
mode's sync field records timer/branch overhead; it issues no durability syscall.

| Mean phase, microseconds | Empty sync | 100,000 sync | Empty buffered | 100,000 buffered |
| --- | ---: | ---: | ---: | ---: |
| Observed load before -> after | 10.00 -> 10.00 | 9.52 -> 9.88 | 10.00 -> 10.00 | 9.88 -> 9.88 |
| Arguments | 16.582 | 21.921 | 14.568 | 15.320 |
| Writer lock | 4.829 | 6.699 | 4.109 | 4.230 |
| Row/ID construction | 19.698 | 41.686 | 15.353 | 17.004 |
| Open/tail inspection | 6.295 | 10.395 | 5.706 | 6.784 |
| Append syscall | 9.353 | 15.134 | 15.013 | 7.359 |
| Sync/timing | 3,994.396 | 8,880.108 | 0.247 | 0.266 |
| Alive publication | 83.292 | 269.500 | 42.393 | 54.848 |
| Internal total | 4,141.755 | 9,256.961 | 101.000 | 109.658 |
| Outside native phase clock | 769.899 | 994.829 | 622.216 | 631.314 |

The earlier **29.914 ms** command, load **2.40 -> 2.37**, copied and durably
republished the log, then did another durable replacement for alive. The new
command removes both structural costs; sync now dominates the explicit sync
choice. At matched buffered durability, the delivered wrapper beats the earlier
**18.767 ms** Python baseline, load **2.37 -> 2.37**, and the current oracle at
every measured size. The final 100,000-row delivered sync sample spent
**8.880 ms** in its durability phase, load **9.52 -> 9.88**; this accounts for
most of its slower end-to-end result. The delivered buffered samples show shared-host
variation without a monotonic increase with historical row count.

| Item | Python oracle | Native candidate measurement |
| --- | --- | --- |
| Direct source | 249 command/service + 186 HTML lines | 1,051 .lkj lines (including 6-line manifest and tests) + 254 HTML |
| Command host wrapper | listen handling included above | 8 reviewed shell lines, outside the language image |
| Wake host composition | relay launch included above | 49 shell lines and 87 lines across the three delivered units |
| Cold command/service build | none | 916.833 / 897.239 ms; separate build samples, both load 7.73 -> 7.73 |
| Command/service image | script + interpreter | 345,791 / 703,957 B |
| Service resident memory | not measured in this run | 1,088 KiB after CLI/HTTP/composition parity; 852 KiB in the short SSE proof |
| Note, 100,000 rows | 28.659 ms buffered; load 9.88 -> 9.88 | delivered wrapper: 0.741 ms buffered (load 9.88 -> 9.88) / 10.252 ms sync (load 9.52 -> 9.88) |
| Authority | ambient host identity, board/relay subprocesses | state root, two typed read roots, one loopback listener, declared clocks/entropy/readiness/stop/output; no Process/World |

Direct-source counts cover the consumer and its tests; the shared HTTP engine,
source library and new compiler/runtime capability implementation are additional
infrastructure, not included in those line counts.

The full authority report is [authority.txt](desk/authority.txt). Command
arguments include the explicit durability choice; selecting that argument does
not change the command's static authority report. Its phase report records the
runtime choice. Clock, entropy and output are explicit capabilities in addition
to the filesystem/network roots.

The final CLI/capability/HTTP/composition checks, including the unchanged Python
oracle session, took **8.702 s**, load **10.84 -> 10.00**, before optional timing
samples. This includes **9 ms** of binding-wrapper proofs, load **10.84 -> 10.84**,
and **5.152 s** of capability proofs, load **10.84 -> 10.61**. The latter includes
**2.362 s** of read-root cases, load **10.84 -> 10.84**. Concurrency, board/handoff,
listener/wake and atomic replacement cases all passed. The complete optional
oracle/measurement run took **21.602 s**, load **10.84 -> 9.88**.

The final short SSE suite used **20 peers, 10 idle**, and took **4.125 s**, load
**9.88 -> 13.25**. Within its protocol proof, load **13.25 -> 13.25**, a note reached
all peers in **172.992 ms**; 100 active requests took **12.916 ms alone / 9.803 ms
with idle peers**. Observed service RSS was **852 KiB**. These are short-suite
observations, separate from the unchanged last 1,000-peer campaign below.

The last 1,000-peer campaign held **990 idle SSE connections and 10 active peers**.
One hundred active requests took **7.909 ms alone / 7.589 ms with idle peers**;
a new note reached all SSE clients in **252.958 ms**, within the campaign's observed
load interval **7.63 -> 7.98**. A slow reader was dropped at the
fixed write deadline; **2,621,644** already-buffered bytes were drained to EOF
afterward. Observed service RSS after the large-frame proof was **5,620 KiB**.
These latency samples end at complete event receipt by the protocol clients;
browser painting was not timed separately. The page applies each accepted
snapshot synchronously before advancing its replay cursor.
The campaign took **7.528 s**, load **7.63 -> 7.98**. The lower loaded sample is
normal measurement variation, not a speedup caused by idle connections.

The append kill campaign delivered **96 SIGKILLs in 191 attempts**, preserving
all **286 acknowledged IDs** and the original **910,000-byte** prefix; the Python
oracle read the recovered state. It took **9.126 s**, load **5.94 -> 5.79**.
Buffered mode does not claim crash/power-loss durability. The long campaigns
preceded the final hardening-specific lookup proof and module extraction. Their
results above are retained unchanged; the final image measurements and short
suite runs above cover the integrated consumer without repeating those campaigns.
