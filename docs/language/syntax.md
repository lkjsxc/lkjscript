# Syntax And Semantics

## Purpose

Define expression, type, and import behavior above the physical line format.

## Status

**Current** where explicitly described below. Extension and numeric-surface
changes are **Accepted Target** and are not current until their gates pass.

## Expressions

- Atoms are numbers, `true`, `false`, `nil`, or symbols.
- `str/` blocks produce `Str` values.
- Calls use matching open and close markers around child expressions.
- Division is named `div` because slash is structural.
- Evaluation is eager except for special forms that explicitly control it.
- Program definitions and mutable global values currently share one global
  namespace across imported files.

## Current Special Forms

Implemented control and binding forms include `def`, `fn`, `if`, `while`,
`let`, `bind`, `set`, `do`, and `quote`. `sig`, `params`, `forall`, `type`, and
`import` are contextual declaration/loading forms rather than freely evaluable
runtime calls.

Every function definition has a mandatory signature and typed parameters.
`forall/` declares annotation-driven type variables. There is no `Any`, trait,
typeclass, Hindley-Milner inference, or implemented user-defined type alias.

`set` mutates program-global state. The baseline checker does not yet prove
that its target exists or matches the declared global type; repairing that
contract is part of the foundation cycle.

## Current Numeric Reality

The type vocabulary advertises `I32`, `I64`, `U32`, `U64`, `F32`, and `F64`,
but the baseline runtime does not faithfully implement every width, cast, or
operator. Integer execution can pass through `f64`, wide values can lose
precision, and some prelude names typecheck without executable lowering.

## Accepted Numeric Target

The first truthful numeric surface contains exact `I64` and `F64` behavior
only. Unsupported widths, aliases, casts, and float-prefixed operator names are
removed until implemented completely. Integer overflow and invalid literals
must fail clearly rather than silently changing representation.

## Files And Imports

Top-level forms are `def`, `do`, and `import`, bounded by
`MAX_TOPLEVEL_FORMS`.

Current path resolution supports:

- `std/...`, `lib/...`, and `examples/...` from package `src` trees;
- `./...` relative to the importing file;
- installed fallback through `LKJSCRIPT_ROOT` when a local category directory
  is absent.

Paths containing `..` are rejected. Cycles are rejected and repeated canonical
files are deduplicated. Definitions are not namespaced modules yet.

The **Accepted Target** requires every entry and import to end in `.lkjscript`.
`.lkjml`, extensionless source, and unrelated extensions are rejected before
parsing. Absolute paths and canonicalized symlink escapes must not bypass the
package-root contract.

## Strings And Bytes

Strings are UTF-8 host strings but many operations index bytes. Arbitrary file
and network bytes do not currently have a complete round-trip-safe contract.
A distinct bulk byte path is deferred; current APIs must document byte indexing
and reject invalid UTF-8 boundaries rather than imply character indexing.
