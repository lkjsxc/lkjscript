# Owned Unix streams and a real Wayland frame

The runtime adds an explicitly granted Unix stream as an ordinary affine `File`.
An entry uses `socket=unix:/absolute/path` or `socket=unix:env:VARIABLE`.
Only the reviewed environment key is consulted during startup. `World.unix_connect`
accepts an absolute path; `Directory.unix_connect` resolves beneath its held root,
pins the socket inode, and connects through that pinned kernel handle. Both return
nonblocking, close-on-exec Files usable by ordinary File operations and Readiness.

## Wire and ownership contract

`send_message(bytes, files, absolute_deadline)` sends the caller's bytes and an
owned `Vec<File>`, without extra framing. Every supplied local owner is consumed
on every result, including error. A positive short send transfers its rights once;
the caller sends the remaining bytes with an empty File vector. Empty payloads
refuse instead of inventing a hidden marker byte.

`receive_message(&mut bytes, descriptor_limit, absolute_deadline)` returns a byte
count and fresh owned Files. Received rights have close-on-exec set and are
promoted away from closed standard-stream sentinels. A truncated or invalid
control message closes every received right and leaves the caller's byte buffer
unchanged. The returned File does not assert a particular kernel object kind.

The operation first attempts one nonblocking transfer; the absolute monotonic
deadline bounds subsequent readiness waits. A ready transfer can therefore
complete even when that deadline has passed. Interrupted waits recompute the
remaining time. Non-stream Unix sockets, TCP sockets and ordinary files refuse
message transport; the input owners still close. Broken peers do not deliver
SIGPIPE to the caller.

Four checked intrinsics carry the new operations. Entry validation, authority
reporting, native lowering, the independent interpreter broker and unit runtime
feature 18 all participate. No raw descriptor or new source-level File
representation is exposed.

## Independent protocol and ownership proof

`bash tools/unix-stream-test.sh --compiler target/lkjscript` runs the same
program against an independent C peer on native and interpreter routes. It
checks the actual Wayland display/registry/bind byte sequence, received keymap
File contents and flags, and a sent shm pool File. The peer observes the
descriptors and bytes directly.

The additional cases establish a positive short send with exactly one rights
transfer, timeout cleanup, empty payload refusal, wrong socket-kind refusal,
broken-peer cleanup, EOF, ordinary File use and Readiness. Repeated truncation
runs include a one-descriptor limit with two delivered descriptors even when
control-message padding fits both, zero permitted descriptors, and a live
descriptor census under a low descriptor limit. A child-owned tracer pauses the
runtime after confined path resolution, replaces the path with an outside
symlink, and proves that only the original pinned listener receives the bytes.

Source moves and direct-IR signature mutations refuse; unchecked interpreter
execution also refuses a forged scalar File owner. Missing environment grants
fail without output authority, and produce the precise located diagnostic when
Stderr is explicitly granted. Cold and warm separately admitted unit images
execute the protocol and are byte-identical.

## Actual compositor result

The retained [client and runner](../docs/proofs/unix-wayland-20261118/README.md)
reuse already available Weston binaries. They launch only a private headless
compositor and its client, then reap both. The source protocol client is copied
from browser commit `2d37da55ffd5b881b2d441b049c72642f8ef4b52`; its transport
alone is adapted to explicit capabilities and owned File messages. The browser
checkout and compiler pin are unchanged.

The client creates the frame backing through a granted Directory, writes its
bytes using ordinary File methods, and sends that owner in `wl_shm.create_pool`.
It exchanges 25 outgoing and 34 incoming messages and commits one surface frame.
Weston's own screenshooter captures the resulting 320 by 240 output. An
independent PNG verifier compares every one of the 76,800 pixels with the expected
gradient; all 230,400 RGB bytes match. Screenshot SHA-256:
`d2980c8251ceec85bbec69840807d0b375b97cb67f0a9b64fb03293aacf71e4b`.

![Actual Weston frame](unix-stream-data/wayland/frame.png)

This proves presentation through the compositor, separately from the existing
[1920 by 1080 seek/write measurement](shm-frame.md). Writable mapping remains
deferred. Protocol framing and compositor buffer-release tracking remain source
responsibilities.

## Producer and publication validation

Native SHA-256:
`9f4660c14d9d0a8082d84affcbe4180416cfab873da048fd7c711a10d02c440b`.

Seed gzip SHA-256:
`7c25f3f73dcfc94c3855e3939637905dfde7a93d15c4c622e12714ca755076de`.

The integrated producer reproduces itself exactly. Its complete focused
protocol script, real compositor proof and both original browser probes pass.
All twelve unchanged-source census observations repeat the previous frontier
byte for byte: nom 17, DOM 63, and Sōzu 448 classified plus 34 prior-unclassified
rows. This runtime addition does not admit a large source root or a nom own test.

The repaired integrated producer passes the complete four-worker
[gate](unix-stream-data/full-gate.log) in 989.660 seconds, with all 6,188 case
requests rerun and no reused requests. The separate
[real-program test](unix-stream-data/standalone-real-programs.log) passes the
original Levenshtein algorithm, eight tests on each engine, eighteen provenance
controls and all 7,239 native corpus rows. Producer and seed identities still
match after the gate. Exact-head publication gate/witness CI is recorded with the
session report.

### Failure retained and repaired

The [first full gate](unix-stream-data/first-failed-gate.log.gz) exposed a real
interpreter cleanup omission: when a later entry grant failed, a preceding
successfully acquired Unix File was not closed. The new semantic regression
caught it. The repair includes File in that cleanup path, with the regression
unchanged. It also promotes the former blanket File-entry rejection fixture to
an explicitly granted Unix File control, retaining no-grant rejection and adding
a canonical direct-IR ReadFile rejection. The closed core now checks 199
contracts, and the combined closed contract cohort checks 735.

The first focused toolchain retry also caught an incomplete primitive prefix in
the new negative IR fixture. The canonical fixture was corrected before the
successful full toolchain and gate. These failures remain recorded; they are not
credited as passing validation.
