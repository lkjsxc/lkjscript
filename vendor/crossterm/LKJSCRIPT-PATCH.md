# lkjscript crossterm patch

This directory is the published `crossterm` 0.29.0 crate source retained under its MIT license.
It is the exact terminal input/output dependency consumed by the native `lkjstudio` runner.

`src/event/source/unix/mio.rs` has one local correctness patch. A zero-byte terminal read returns
`UnexpectedEof`, and an error other than `WouldBlock` or `Interrupted` is returned to the caller.
The published implementation otherwise repeats those reads forever after a pseudoterminal endpoint
closes. The runner maps EOF and the Linux pseudoterminal `EIO` outcome to its typed EOF lifecycle,
attempts every acquired cleanup stage, and never leaves the dependency's reader spinning.

`src/terminal/sys/unix.rs` has one warning-only parenthesis cleanup so the repository's
warning-denial gate also covers the retained source.
