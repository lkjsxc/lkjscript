# Durable File and Entropy Capabilities

## Purpose

Expose generic Linux filesystem durability and OS entropy needed by real
single-host stores and opaque credentials. Application log framing, recovery,
projection ordering, storage policy, and token formats remain `.lkjscript`.

## Status

**Current.** The primitive surface, append/replay consumer, canonical local
and Docker verification are implemented and verified.

## Decision

Add `result`-valued primitives:

```text
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
```

Append uses `O_APPEND`; it is not a multi-process transaction. Create-new is
exclusive. Rename is atomic only within a filesystem; durable replacement needs
file sync, rename, then parent-directory sync. Random fill invokes Linux
`getrandom` only, retries interruption, and has no PRNG or time-based fallback.
Offset/length range validation follows the lossless bulk-byte contract.

## Safety

The sys crate owns FFI and validates exact absolute `path` bytes before C calls.
`directory` is a distinct owned resource kind. All errors, wrong/stale resource
kinds, overflow, short random fill, and ordinary errno become qualified
language `result` errors. No
script controls flags, permissions, or random source selection.

## Verification

Append/create exclusivity, binary random fill/range errors, file/directory
sync, truncation, rename, stale/wrong resource kinds, and opcode validation pass. The
`.lkjscript` append/replay/restart consumer passes locally; canonical and Docker
gates pass.

## Rejected

A giant JSON rewrite, pseudo-random fallback, unconditional overwrite, raw file
descriptors, arbitrary syscall flags, and app-specific record/projection helpers.
