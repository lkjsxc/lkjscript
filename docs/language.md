# Current language semantics

This document summarizes meaning implemented by the compiler and tests. The physical text notation
is provisional and may be replaced without compatibility support.

## Programs

A package contains modules and declarations. Imports name exact package-relative module paths and
explicit declarations. Resolution is static and deterministic; duplicate declarations, unresolved
names, invalid imports, and import cycles are errors.

Functions have typed inputs, results, and parameters. Calls are statically resolved, including the
implemented generic and trait-dispatch subsets. Local bindings may be immutable or explicitly
mutable. Control expressions include `if`, `do`, loops, `while`, `break`, `continue`, `return`, and
exhaustive enum matching. `never` represents non-returning control.

## Values and types

Implemented foundational values include `unit`, `bool`, `i64`, `f64`, symbols, text/bytes,
byte vectors and borrows, lists, nominal products, nominal enums, capabilities, and typed host
resources. `Option` and `Result` are ordinary generic enums. Products and enums are nominal rather
than structurally interchangeable.

Integer arithmetic and conversions are checked where specified by their operations. Floating-point
values retain IEEE-754 bit behavior tested by the differential suites. Equality is type-directed.
Pattern matching is checked for type correctness and exhaustiveness.

## Effects, capabilities, and ownership

Host effects require capabilities supplied by the package and runtime provider. Source code does
not receive ambient filesystem, network, database, terminal, or process authority.

Ordinary source does not expose raw pointers, retain/release, a general `free`, tracing controls,
or named implementation lifetimes. Copy values may be duplicated. Affine resources and unique
storage move or borrow under compiler checks. Supported aggregate representations carry a verified
memory plan and deterministic cleanup behavior.

## Incomplete programs

Typed holes are supported by the current Semantic Source editing service. A snapshot containing a
reachable hole is a valid editing state but is rejected for executable compilation. The future
semantic model will also represent unresolved references, ambiguity, conflicts, and parse-import
errors without requiring a valid full text file.

## Text projection

The current notation uses one atom or open/close marker per physical line, with forms such as
`name/` and `/name`. Only `.lkjscript` is accepted. Exact syntax examples live under
`src/examples/`; parser and compiler tests own accepted and rejected forms.

Formatting, comments, spans, and file locations are not intended to become semantic identity.
A future concise renderer may replace this projection.

## Bounds and failure

Type compatibility, ownership legality, capability authority, exhaustive matching, valid control
flow, and artifact well-formedness are semantic laws. A declaration or expression count is not.

The lexer-token, children-per-form, top-level-form, 16 MiB per-source, 256 MiB aggregate-source,
and 65,536 source-unit ceilings have been removed. Trusted source validation, loading, package
analysis, and compilation are unrestricted by source-byte or source-unit policy. An untrusted
Semantic Source request may apply an explicit aggregate source-byte policy; exhausting that policy
is a typed host resource failure and does not make the unchanged program invalid.

The current implementation still retains a nesting safety ceiling until recursive source
processing is made stack-safe. Source positions and spans remain `u32`, creating a separate
addressable representation boundary. Later HIR, ownership, memory-plan, SSA, structural-value, and
executable-width ceilings also remain. These inherited ceilings are known defects, not permanent
language rules. New work must remove the checks and repair the algorithms or representations
rather than publish larger numbers. Real host exhaustion, cancellation, checked representation
overflow, and explicit untrusted-request policy must report typed failures without partial
publication.
