# Durable File and Entropy Capabilities

## Purpose

Expose generic Linux filesystem durability and OS entropy needed by real
single-host stores and opaque credentials. Application log framing, recovery,
projection ordering, storage policy, and token formats remain `.lkjscript`.

## Status

**Accepted Target.** No primitive in this record is implemented yet.

## Decision

Add Result-valued primitives:

```text
sys-open-append Str -> Result Handle Str
sys-open-create-new Str -> Result Handle Str
sys-open-dir Str -> Result Handle Str
sys-fsync Handle -> Result Unit Str
sys-truncate Handle I64 -> Result Unit Str
sys-rename Str Str -> Result Unit Str
sys-random-fill Buf I64 I64 -> Result Unit Str
```

Append uses `O_APPEND`; it is not a multi-process transaction. Create-new is
exclusive. Rename is atomic only within a filesystem; durable replacement needs
file sync, rename, then parent-directory sync. Random fill invokes Linux
`getrandom` only, retries interruption, and has no PRNG or time-based fallback.
Offset/length range validation follows the lossless bulk-byte contract.

## Safety

The sys crate owns FFI and validates strings before C calls. Directory is a
distinct owned handle kind. All errors, wrong/stale handles, overflow, short
random fill, and ordinary errno become qualified language Result errors. No
script controls flags, permissions, or random source selection.

## Verification

Test append/create exclusivity, binary random fill and range errors, file and
directory sync, truncation, same-filesystem rename, stale/wrong handles, and
interrupted/error propagation. A `.lkjscript` append/replay/restart consumer
must exercise the capability before a product claim. Docker and canonical gates
must pass.

## Rejected

A giant JSON rewrite, pseudo-random fallback, unconditional overwrite, raw file
descriptors, arbitrary syscall flags, and app-specific record/projection helpers.
