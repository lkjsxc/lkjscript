# Truthful System Results

## Purpose

Make ordinary host and resource failures explicit language values rather than
unexpected VM termination or false success.

## Status

**Current.** The compiler prelude, opcode mapping, VM dispatch, and standard
library wrappers use the `result`-explicit system surface.

## Decision

Every canonical host/resource operation described below returns
`result success system-error`. Core console operations remain VM-failure
surfaces. Static types, bytecode dispatch, and generic enum allocation must
agree on the exact success and error identities. Ordinary errno, invalid/stale
resource kind, range, and no-progress outcomes become the capability-domain
`system-error` variant. Hosts translate once at the VM/native capability
boundary; display text is never inspected to select a variant.

VM errors remain appropriate for malformed bytecode, impossible compiler/VM
ABI states, and violations of a non-`result` core language operation. A standard
library wrapper may deliberately call `unwrap-ok`, but that explicit policy is
different from the VM silently converting an OS error into process failure.

## Canonical Primitive Names

The descriptor-facing surface becomes:

```text
standard-input: fn inputs capability stdio output input-stream
is-terminal: fn inputs input-stream output result bool system-error
drop:
  forall resource;
  resource one-of output-stream,file-reader,file-writer,file-appender,directory,
    tcp-listener,tcp-stream,sqlite-connection,sqlite-statement,terminal-session;
  fn inputs resource output result unit system-error
read-resource-byte:
  forall resource; resource one-of input-stream,file-reader,tcp-stream;
  fn inputs resource output result i64 system-error
write-resource-byte:
  forall resource;
  resource one-of output-stream,file-writer,file-appender,tcp-stream;
  fn inputs resource i64 output result unit system-error
read-into:
  forall resource; resource one-of input-stream,file-reader,tcp-stream;
  fn inputs resource buf i64 i64 output result i64 system-error
write-from:
  forall resource;
  resource one-of output-stream,file-writer,file-appender,tcp-stream;
  fn inputs resource buf i64 i64 output result i64 system-error
convert-string-to-buf: fn inputs string output buf
convert-buf-to-string: fn inputs buf output result string utf8-error
open-file-appender: fn inputs capability file-system path output result file-appender system-error
create-file: fn inputs capability file-system path output result file-writer system-error
open-directory: fn inputs capability file-system path output result directory system-error
sync-file:
  forall resource; resource one-of file-writer,file-appender,directory;
  fn inputs resource output result unit system-error
truncate-file:
  forall resource; resource one-of file-writer,file-appender;
  fn inputs resource i64 output result unit system-error
rename-path: fn inputs capability file-system path path output result unit system-error
fill-random: fn inputs capability entropy buf i64 i64 output result unit system-error
save-terminal-guard: fn inputs capability terminal buf output result unit system-error
clear-terminal-guard: fn inputs capability terminal output result unit system-error
```

The old `stdin-fd`, `isatty`, `close`, `read-byte-fd`, `write-byte-fd`,
`tty-guard-save`, and `tty-guard-clear` names are removed without aliases.

The canonical file-open, `does-path-exist`, time, socket, poll, and terminal
operations all return `result` values. `does-path-exist` returns `ok false` only
for absence-class errors; permission, malformed path, and other failures return
`err`. Negative waits, ports, backlogs, and poll timeouts return errors rather
than being clamped or cast.

## Standard Library Policy

User-facing convenience wrappers may preserve direct return types by calling
`unwrap-ok` explicitly. Low-level applications may call canonical host
operations and handle errors without terminating the VM. `unwrap-ok` traps on
`err`; human
rendering of the structured error is diagnostic projection only.

## Verification

- A missing file open returns the generic `err system-error` branch,
  subsequent expressions run, and the process exits successfully.
- A repeated drop returns an `err system-error` branch without reviving or
  terminating the VM.
- Scalar, borrowed, stale, and wrong-kind resources return `result` errors at
  the language boundary.
- Missing path returns `ok false`; malformed path returns `err`.
- Negative wait, timeout, port, and backlog values return `err`.
- Successful send and bulk write report actual byte counts.
- Bulk reads/writes preserve bytes exactly, validate offset/length before
  slicing, and reject invalid UTF-8 rather than applying replacement text.
- Append, sync, truncate, rename, and OS random-fill expose no raw descriptor,
  caller-selected flags, or pseudo-random fallback.
- Prelude names, codegen mappings, and opcode dispatch have conformance tests.
- Hello, lkjedit, and one-shot HTTP remain green.

## Rejected

- Declaring `result` while propagating only the Rust `err` branch.
- Returning zero or absence as a placeholder success payload.
- Keeping universal descriptor terminology after language values became exact
  typed resources.
- Treating all `access(2)` failures as a nonexistent path.
