# Terminal event, frame, and lifecycle contract

This specification owns the native trusted-terminal adapter used by interactive applications. It
does not own application state, key policy, semantic commands, project/file authority, or terminal
bytes supplied by application meaning.

## Contract and trust boundary

Terminal contract version 3 runs only when standard input and output are terminals under one
trusted local operator and OS account. The terminal byte stream, dimensions, paste, escape
sequences, EOF, signals, and output errors are hostile observations. A terminal process boundary is
not a sandbox.

The application supplies a validated interactive-profile-2 artifact. The adapter supplies closed
events and converts closed full frames to terminal commands. Application values never contain ANSI,
file descriptors, terminal handles, capabilities, or OS objects. Terminal output is presentation;
it cannot acknowledge, undo, or authorize a semantic-project or filesystem publication.

## Event decoding

The retained input vocabulary is key press/repeat, bracketed paste, resize, and close/EOF. Key
codes are character, enter, backspace, delete, left, right, up, down, home, end, and escape. Tab and
back-tab normalize to the tab character with modifiers. A key separately reports control, alt,
shift, and repeat. Releases are ignored. Super, hyper, and meta reject; unsupported codes, mouse,
and focus events are ignored because the product has no consumer.

Paste is one event and is limited to 65,536 Unicode scalars before application execution. Resize
reports explicit rows and columns; host dimensions clamp to 1 through 1,000. Polling is sequential
at 25 ms. A Unix hangup, invalid descriptor, zero readable bytes, `UnexpectedEof`, or terminal EIO
is EOF rather than an endlessly readable event. Other decode failures are `terminal_decode`.

The vendored crossterm 0.29 source differs from the published crate only at its Unix mio reader:
a zero-byte read becomes `UnexpectedEof`, and nonblocking errors other than `WouldBlock` or
`Interrupted` are returned. The patch prevents a disconnected pseudo-terminal from spinning in
the dependency event loop; `vendor/crossterm/LKJSCRIPT-PATCH.md` records the exact reason.

## Full-frame projection

Full-frame output is both the retained route and correctness oracle. Each semantic frame contains
rows, columns, Unicode scalars, status text, and cursor state. Projection clears the screen, writes
body rows, writes status on the final row, positions or hides the cursor, and flushes. Encoded output
is limited to 8 MiB before it reaches the live writer; the application frame itself is limited to
131,072 Unicode scalars before projection.

LF advances to the next body row. Tab expands to the next four-cell stop. Control characters render
as U+FFFD. Combining characters render after their base; at column zero the adapter first renders a
dotted circle. `unicode-width` determines scalar cell width. A wide scalar that cannot fit at the
right edge renders U+FFFD in one cell. Content clips to declared rows/columns. Cursor columns are
application scalar columns converted to bounded terminal-cell columns under the same rules.

There is no style identity, color, cell-patch, row-span, diff cache, or raw-ANSI application value.
These lost to the full frame because the complete product meets its current bounds without another
render protocol. Missing or corrupt derived rendering state is therefore impossible.

## Lifecycle and cleanup

The exact acquisition order is raw mode, alternate screen, bracketed paste, cursor hidden. The
adapter then renders the initial semantic frame and processes events and at most 10,000 host
actions. Normal application exit, a registered `SIGINT`, `SIGTERM`, `SIGQUIT`, or `SIGHUP`, and EOF
all leave the loop through one cleanup owner.

Cleanup attempts every acquired stage in reverse: show cursor, disable bracketed paste, leave the
alternate screen, disable raw mode. It is idempotent, records the first cleanup failure only after
attempting the rest, and also runs from `Drop` during unwinding. Failure at any acquisition stage
cleans every earlier stage. Output failure is `terminal_output`; cleanup failure is
`terminal_cleanup` and takes precedence in the returned result because the live resource is not
known restored.

Tests use a deterministic fake backend to fail every acquisition/output/cleanup stage, exercise
drop/unwind, and inspect bounded projection. Pseudo-terminal acceptance covers normal exit, project
action/resume, signal, disconnected input, cleanup sequences, and exact restoration of terminal
attributes. No claim is made for unhandled fatal signals, hostile kernel/native code, Windows, or a
terminal other than the verified Linux x86-64 environment.
