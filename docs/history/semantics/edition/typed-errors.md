# Edition 2: Typed Errors

[Authority](../edition.md)

## Purpose

Define disjoint typed failure domains and prevent textual error matching from
becoming control flow.

## Status

**Current for the compiler-recognized `NumericError`, `Utf8Error`, and
`SystemError` enums and the generic prelude `Option T` and `Result T E`
boundary.** Structured execution outcomes retain their distinct boundary.

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

`Utf8Error` is a compiler-recognized prelude enum identity. Its closed variants
are `UnexpectedContinuation`, `InvalidLeadingByte`, `MissingContinuation`,
`OverlongEncoding`, `Surrogate`, and `OutOfRange`; each carries one `offset I64`
field containing the exact zero-based byte offset of the first invalid sequence.
Human detail is diagnostic projection, not semantic discrimination.

## System Error Target

`SystemError` is a closed compiler-recognized prelude enum whose top-level
cases are exactly `Io`, `Network`, `Terminal`, `Time`, `Random`, `Sqlite`,
`Utf8`, and `Unsupported`. `Utf8` carries one `error Utf8Error` field. Every
other case carries `code Option I64` and `detail Option Str`; either may be
absent. The selected top-level identity records the capability domain, while
code/detail preserve bounded reporting evidence and are never name-based
branching inputs. More detailed operation-specific sub-enums require a later
contract change rather than an open residual value.

Hosts translate platform failures once at the capability boundary. VM,
evaluator, native runtime, and protocol compare stable enum/variant identities,
not display strings. Unknown platform failures use a typed residual case within
the owning hierarchy rather than pretending to be a known category.
