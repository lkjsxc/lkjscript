# Lossless Bulk Bytes

## Purpose

Provide a bounded, exact byte boundary for generic file and socket consumers.
The successor article product needs it, but routes, HTTP framing, storage,
authorization, rendering, and data policy remain `.lkjscript` work.

## Status

**Current.** The primitive surface, real `.lkjscript` file-buffer consumer,
canonical local gate, and Docker verification are implemented and verified.

## Decision

`Buf` is the lossless file/wire representation; `Str` remains valid UTF-8.
Add these Result-valued primitives:

```text
sys-read-into Handle Buf I64 I64 -> Result I64 Str
sys-write-from Handle Buf I64 I64 -> Result I64 Str
buf-from-str Str -> Buf
buf-to-str Buf -> Result Str Str
buf-slice Buf I64 I64 -> Result Buf Str
```

For read/write, offset and requested length are non-negative, fit the buffer,
and do not exceed a fixed bulk-I/O limit. `Ok(0)` means EOF or no progress;
writes report actual progress and never hide a partial write. Invalid ranges,
wrong/stale handles, ordinary OS errors, and invalid UTF-8 are Result errors.
UTF-8 conversion never uses replacement characters. `buf-from-str` encodes
exact UTF-8 and is bounded by the existing buffer limit. `buf-slice` copies an
exact validated range into a bounded `Buf`; it supplies protocol consumers with
an exact received prefix without exposing host slices.

The existing Str-only socket operations are retained only while legacy examples
need them; new consumers use this surface. Blocking bulk calls remain rejected
by hard-deadline execution before effects.

## Safety And Ownership

Unsafe syscall FFI stays in `lkjscript-sys`. Safe wrappers validate all ranges
before slicing, preserve handle kind/closed-state checks, and use retry only for
interrupted syscalls. They do not allocate from untrusted requested lengths.
The VM owns buffers and handles; no raw pointer, descriptor, or borrowed byte
slice crosses the language boundary.

## Verification

Compiler signatures/effects, opcode type-stack validation, malformed chunks,
file/socket progress, EOF, closed/wrong handles, invalid ranges, invalid UTF-8,
NUL/non-ASCII exact round trips, limits, and hard-deadline rejection pass in
`cargo run --locked -p lkjscript-xtask -- quiet verify`. The `bulk-bytes` source
consumer and Docker verification also pass.

## Rejected

Lossy UTF-8, implicit string conversion, a script-selected buffer size,
unbounded allocation, and app-specific HTTP/database helpers in Rust.
