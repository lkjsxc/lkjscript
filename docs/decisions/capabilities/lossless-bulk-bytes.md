# Lossless Bulk Bytes

## Purpose

Provide a bounded, exact byte boundary for generic file and socket consumers.
The successor article product needs it, but routes, HTTP framing, storage,
authorization, rendering, and data policy remain `.lkjscript` work.

## Status

**Current.** The primitive surface, real `.lkjscript` file-buffer consumer,
canonical local gate, and Docker verification are implemented and verified.

## Decision

`buf` is the lossless file/wire representation; `string` remains valid UTF-8.
Add these `result`-valued primitives:

```text
read-into:
  forall resource; resource one-of input-stream,file-reader,tcp-stream;
  fn inputs resource buf i64 i64 output result i64 system-error
write-from:
  forall resource;
  resource one-of output-stream,file-writer,file-appender,tcp-stream;
  fn inputs resource buf i64 i64 output result i64 system-error
convert-string-to-buf: fn inputs string output buf
convert-buf-to-string: fn inputs buf output result string utf8-error
copy-buf-slice: fn inputs buf i64 i64 output result buf system-error
```

For read/write, offset and requested length are non-negative, fit the buffer,
and do not exceed a fixed bulk-I/O limit. An `ok 0` branch means EOF or no progress;
writes report actual progress and never hide a partial write. Invalid ranges,
wrong/stale typed resources, ordinary OS errors, and invalid UTF-8 are
`result` errors.
UTF-8 conversion never uses replacement characters. `convert-string-to-buf` encodes
exact UTF-8 and is bounded by the existing buffer limit. `copy-buf-slice` copies an
exact validated range into a bounded `buf`; it supplies protocol consumers with
an exact received prefix without exposing host slices.

The current string-oriented socket operations remain distinct while canonical
consumers migrate to byte storage. Blocking bulk calls remain rejected by
hard-deadline execution before effects.

## Safety And Ownership

Unsafe syscall FFI stays in `lkjscript-sys`. Safe wrappers validate all ranges
before slicing, preserve exact resource-kind and closed-state checks, and use
retry only for interrupted syscalls. They do not allocate from untrusted
requested lengths. The VM owns buffers and typed resources; no raw pointer,
descriptor, or borrowed byte
slice crosses the language boundary.

## Verification

Compiler signatures/effects, opcode type-stack validation, malformed chunks,
file/socket progress, EOF, closed/wrong resource kinds, invalid ranges, invalid UTF-8,
NUL/non-ASCII exact round trips, limits, and hard-deadline rejection pass in
`cargo run --locked -p lkjscript-xtask -- quiet verify`. The `bulk-bytes` source
consumer and Docker verification also pass.

## Rejected

Lossy UTF-8, implicit string conversion, a script-selected buffer size,
unbounded allocation, and app-specific HTTP/database helpers in Rust.
