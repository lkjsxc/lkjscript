# Truthful System Results

## Purpose

Make ordinary host and resource failures explicit language values rather than
unexpected VM termination or false success.

## Status

**Current.** The compiler prelude, opcode mapping, VM dispatch, and standard
library wrappers use the Result-explicit system surface.

## Decision

Every canonical `sys-*` OS/resource primitive returns
`Result Success SystemError`. Core console operations remain VM-failure
surfaces. Static types, bytecode dispatch, and generic enum allocation must
agree on the exact success and error identities. Ordinary errno, invalid/stale
handle, range, and no-progress outcomes become the capability-domain
`SystemError` variant. Hosts translate once at the VM/native capability
boundary; display text is never inspected to select a variant.

VM errors remain appropriate for malformed bytecode, impossible compiler/VM
ABI states, and violations of a non-Result core language operation. A standard
library wrapper may deliberately call `unwrap-ok`, but that explicit policy is
different from the VM silently converting an OS error into process failure.

## Canonical Primitive Names

The descriptor-facing surface becomes:

```text
stdin-handle                         -> Handle
sys-isatty Handle                    -> Result Bool SystemError
sys-close Handle                     -> Result Unit SystemError
sys-read-byte Handle                 -> Result I64 SystemError
sys-write-byte Handle I64            -> Result Unit SystemError
sys-read-into Handle Buf I64 I64     -> Result I64 SystemError
sys-write-from Handle Buf I64 I64    -> Result I64 SystemError
buf-from-str Str                      -> Buf
buf-to-str Buf                        -> Result Str Utf8Error
sys-open-append Str                  -> Result Handle SystemError
sys-open-create-new Str              -> Result Handle SystemError
sys-open-dir Str                     -> Result Handle SystemError
sys-fsync Handle                     -> Result Unit SystemError
sys-truncate Handle I64              -> Result Unit SystemError
sys-rename Str Str                   -> Result Unit SystemError
sys-random-fill Buf I64 I64          -> Result Unit SystemError
sys-tty-guard-save Buf               -> Result Unit SystemError
sys-tty-guard-clear                  -> Result Unit SystemError
```

The old `stdin-fd`, `isatty`, `close`, `read-byte-fd`, `write-byte-fd`,
`tty-guard-save`, and `tty-guard-clear` names are removed without aliases.

Existing `sys-open-*`, `sys-path-exists`, time, socket, poll, and terminal
operations all return Results. `sys-path-exists` returns `Ok(false)` only for
absence-class errors; permission, malformed path, and other failures return
`Err`. Negative waits, ports, backlogs, and poll timeouts return errors rather
than being clamped or cast.

## Standard Library Policy

User-facing convenience wrappers may preserve direct return types by calling
`unwrap-ok` explicitly. Low-level applications may call `sys-*` primitives and
handle errors without terminating the VM. `unwrap-ok` traps on `Err`; human
rendering of the structured error is diagnostic projection only.

## Verification

- A missing file open returns generic `Result.Err(SystemError.Io)`, subsequent expressions run, and the
  process exits successfully.
- A repeated close returns generic `Result.Err(SystemError.Io)` without reviving or terminating the VM.
- Integer, borrowed, stale, and wrong-kind handles return Result errors at the
  language boundary.
- Missing path returns `Ok(false)`; malformed path returns `Err`.
- Negative wait, timeout, port, and backlog values return `Err`.
- Successful send and bulk write report actual byte counts.
- Bulk reads/writes preserve bytes exactly, validate offset/length before
  slicing, and reject invalid UTF-8 rather than applying replacement text.
- Append, sync, truncate, rename, and OS random-fill expose no raw descriptor,
  caller-selected flags, or pseudo-random fallback.
- Prelude names, codegen mappings, and opcode dispatch have conformance tests.
- Hello, lkjedit, and one-shot HTTP remain green.

## Rejected

- Declaring `Result` while propagating only the Rust `Err` branch.
- Returning zero or absence as a placeholder success payload.
- Keeping descriptor terminology after the language value became an opaque
  handle.
- Treating all `access(2)` failures as a nonexistent path.
