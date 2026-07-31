# Lossless Bulk Bytes

## Purpose

Provide a bounded, exact byte boundary for generic file and socket consumers.
The successor article product needs it, but routes, HTTP framing, storage,
authorization, rendering, and data policy remain `.lkjscript` work.

## Status

**Current.** The primitive surface, real `.lkjscript` file-buffer consumer,
canonical local gate, and Docker verification are implemented and verified.

## Decision

Checked byte slices are the lossless file/wire boundary; `string` remains valid
UTF-8. The primitives are:

```text
read-into:
  forall resource; resource one-of input-stream,file-reader,tcp-stream;
  fn inputs resource byte-slice-mut output result i64 system-error
write-from:
  forall resource;
  resource one-of output-stream,file-writer,file-appender,tcp-stream;
  fn inputs resource byte-slice output result i64 system-error
convert-string-to-bytes: fn inputs string output bytes
convert-bytes-to-string: fn inputs bytes output result string utf8-error
copy-bytes-slice: fn inputs bytes i64 i64 output bytes
```

For read/write, the validated view length does not exceed the fixed bulk-I/O
limit. An `ok 0` branch means EOF or no progress;
writes report actual progress and never hide a partial write. Invalid ranges,
wrong/stale typed resources, ordinary OS errors, and invalid UTF-8 are
`result` errors.
UTF-8 conversion never uses replacement characters.
`convert-string-to-bytes` encodes exact UTF-8 and is bounded by the byte-value
limit. `copy-bytes-slice` copies an exact validated range into immutable
`bytes`; it supplies protocol consumers with an exact received prefix without
exposing host slices.

The current string-oriented socket operations remain distinct while canonical
consumers migrate to byte storage. Blocking bulk calls remain rejected by
hard-deadline execution before effects.

## Safety And Ownership

Unsafe syscall FFI stays in `lkjscript-sys`. Safe wrappers validate all ranges
before slicing, preserve exact resource-kind and closed-state checks, and use
retry only for interrupted syscalls. They do not allocate from untrusted
requested lengths. The VM owns unique byte storage and typed resources; no raw pointer,
descriptor, or unchecked borrowed byte slice crosses the language boundary.

## Verification

Compiler signatures/effects, opcode type-stack validation, malformed chunks,
file/socket progress, EOF, closed/wrong resource kinds, invalid ranges, invalid UTF-8,
NUL/non-ASCII exact round trips, limits, and hard-deadline rejection pass in
`cargo run --locked -p lkjscript-xtask -- quiet verify`. The `bulk-bytes` source
consumer and Docker verification also pass.

## Rejected

Lossy UTF-8, implicit string conversion, a script-selected buffer size,
unbounded allocation, and app-specific HTTP/database helpers in Rust.
