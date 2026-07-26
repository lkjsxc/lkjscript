# Edition 2: Typed Errors

[Authority](../edition-2.md)

## Purpose

Define disjoint typed failure domains and prevent textual error matching from
becoming control flow.

## Status

**Current only for the stable compiler-recognized `NumericError` enum used by
the four Edition 2 numeric conversions.** The wider typed-error hierarchy in
this record remains an Accepted Target. Existing Current Result and structured
outcome surfaces retain their documented boundaries.

## Failure Domains

The following are distinct and are never collapsed into one stringly error:

- `Option T`: ordinary absence;
- `Result T E`: recoverable typed domain failure;
- `Trap TrapValue`: execution cannot continue in this execution;
- `ResourceLimit`: a named checked budget was exhausted;
- `Deadline`: a configured execution deadline was reached;
- `HostFailure`: the host boundary failed; and
- compiler and protocol diagnostics: invalid program/request/edit authority.

Catch-all exception unwinding and string matching over messages or platform
text are not control mechanisms.

## UTF-8

`Utf8Error` is a compiler-recognized prelude enum identity. It records the exact
zero-based byte offset of the first invalid sequence and a typed category:
`UnexpectedContinuation`, `InvalidLeadingByte`, `MissingContinuation`,
`OverlongEncoding`, `Surrogate`, or `OutOfRange`. Human detail is diagnostic
projection, not semantic discrimination.

## System Error Target

`SystemError` is a typed hierarchy whose top-level cases are exactly `Io`,
`Network`, `Terminal`, `Time`, `Random`, `Sqlite`, `Utf8`, and `Unsupported`.
Each case has operation-specific typed subcases where known and may include an
optional platform code and optional bounded detail. Platform code/detail is
for reporting and roundtrip, never name-based branching. `Utf8` carries
`Utf8Error`.

Hosts translate platform failures once at the capability boundary. VM,
evaluator, native runtime, and protocol compare stable enum/variant identities,
not display strings. Unknown platform failures use a typed residual case within
the owning hierarchy rather than pretending to be a known category.
