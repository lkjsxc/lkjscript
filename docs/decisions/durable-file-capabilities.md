# Durable File and Entropy Capabilities

## Purpose

Expose generic Linux filesystem durability and OS entropy needed by real
single-host stores and opaque credentials. Application log framing, recovery,
projection ordering, storage policy, and token formats remain `.lkjscript`.

## Status

**Current.** The primitive surface, append/replay consumer, canonical local
and Docker verification are implemented and verified.

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

Append/create exclusivity, binary random fill/range errors, file/directory
sync, truncation, rename, stale/wrong handles, and opcode validation pass. The
`.lkjscript` append/replay/restart consumer passes locally; canonical and Docker
gates pass.

## Rejected

A giant JSON rewrite, pseudo-random fallback, unconditional overwrite, raw file
descriptors, arbitrary syscall flags, and app-specific record/projection helpers.
