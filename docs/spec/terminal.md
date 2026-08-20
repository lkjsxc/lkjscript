# Terminal event, frame, worker, and lifecycle contract

This specification owns terminal contract 4, the trusted native adapter for interactive-profile-3
applications. It does not own application state, Vim/key/mouse policy, semantic geometry, project or
file authority, or terminal bytes supplied by application meaning.

## Trust and topology

Live execution requires terminal stdin/stdout under one trusted local operator and OS account. The
terminal byte stream, dimensions, paste, mouse coordinates, escape sequences, EOF, signals, and
output errors are hostile observations. The terminal process is not a sandbox.

One foreground thread owns terminal acquisition, event ordering, application steps, logical-frame
validation, projection, output, and cleanup. One worker thread receives at most one request through
a capacity-one synchronous channel and returns at most one bounded result through another. It never
borrows or mutates application state. No executor, unbounded queue, scheduler, hidden retry, or
daemon exists.

An admitted request carries one exact nonzero session-local job identity. While it runs, the
foreground continues local events and polling at 25 ms. A second request reports `authority_busy`.
The result is returned only through application `resume`; duplicate/foreign/stale identity rejects.
A worker panic is contained as typed infrastructure/possible-visibility failure. Shutdown drops the
request channel and does not let an abandoned read prevent terminal cleanup. A possibly visible
write remains subject to its filesystem/project reconciliation contract.

## Event decoding

The closed vocabulary is key press/repeat, bracketed paste, resize, SGR mouse, focus gained/lost,
deployment open, and close/EOF. Key codes are character, enter, backspace, delete, left, right, up,
down, home, end, and escape. Tab/back-tab normalize to the tab character with modifiers. A key
separately reports control, alt, shift, and repeat. Key releases are ignored; super/hyper/meta reject.

Mouse button is none, primary, middle, or secondary. Kind is press, release, drag, scroll up/down,
or horizontal scroll. Rows and columns are terminal-cell coordinates and modifiers remain explicit.
The adapter enables basic press, button-motion, and SGR extended modes (`1000`, `1002`, `1006`). It
does no tab/split hit testing and never turns malformed mouse bytes into text.

Paste is one event and is admitted only through 65,536 Unicode scalars. Resize reports explicit
dimensions clamped to 1 through 1,000. Unix hangup, invalid descriptor, zero readable bytes,
`UnexpectedEof`, and terminal EIO are EOF rather than endlessly readable input. Other decode failures
are `terminal_decode`.

The vendored crossterm 0.29 source differs from the published crate only in its Unix mio reader: a
zero-byte read becomes `UnexpectedEof`, and nonblocking errors other than `WouldBlock`/`Interrupted`
return. `vendor/crossterm/LKJSCRIPT-PATCH.md` owns that exact patch.

## Logical frame and safe projection

The full logical frame is the semantic oracle. It contains rows, columns, Unicode scalars, one
abstract style 0–15 per scalar, status plus status style, cursor row/column/visibility, and cursor
shape (block, bar, underline). It contains no ANSI or terminal color numbers. Frame scalars are
bounded by 131,072 and encoded output by 8 MiB before a live write.

LF advances a row. Tab expands to the next four-cell stop. C0 controls render as U+FFFD. Combining
characters remain after a base; at column zero projection inserts a dotted circle. `unicode-width`
determines display cells. A wide scalar that cannot fit at the right edge becomes one U+FFFD cell.
Content clips to declared dimensions. Application text indexes and terminal cells remain separate.
The adapter maps the closed style palette and cursor shapes to trusted crossterm commands.

The first frame, cache miss, and dimension change emit a full clear/projection. Otherwise the adapter
compares the next projected frame with the last successfully flushed frame and emits only changed
rows/status/cursor. Any encode/write/flush failure discards the acknowledged cache; retry/restart
therefore begins with the full oracle. The cache advances only after `write_all` and `flush` both
succeed. It is disposable presentation state and cannot change logical content or authority.

## Acquisition and cleanup

Acquisition order is raw mode, alternate screen, bracketed paste, SGR mouse modes, focus capture,
then cursor hide. The initial full frame follows successful acquisition. Normal application exit,
registered `SIGINT`/`SIGTERM`/`SIGQUIT`/`SIGHUP`, and EOF all leave through one cleanup owner.

Cleanup attempts every acquired stage in reverse: show cursor, disable focus, disable SGR mouse,
disable bracketed paste, leave alternate screen, disable raw mode. It is idempotent, retains the
first cleanup failure after attempting the rest, and runs from `Drop` during unwind. Acquisition
failure cleans every prior stage. Terminal output never rolls back completed project or filesystem
publication.

Deterministic fake tests cover every lifecycle/output stage, worker admission/result/panic/channel
behavior, projection and cache loss. Pseudo-terminal acceptance covers keyboard editing/save,
SGR mouse selection/drag, signal, disconnected input, exact cleanup sequences, and terminal-attribute
restoration on verified Linux x86-64. No claim is made for unhandled fatal signals, hostile kernel or
native code, Windows, or untested terminal emulators.
