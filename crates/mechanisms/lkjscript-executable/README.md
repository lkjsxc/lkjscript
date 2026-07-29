# lkjscript-executable

Private Linux x86-64 executable-memory mechanism crate.

It owns bounded RW-to-RX installation, synchronous generated entry, native
reference frames, and native runtime services. Its public Rust facade is safe;
every unsafe-containing file is registered under the existing executable or
native-runtime boundary ID. Non-Linux code remains fail-closed and unexecuted.
