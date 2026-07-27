# Canonical Lowercase Word Vocabulary

## Status

**Accepted contract with the vocabulary cutover implemented.** Compiler,
tracked source corpus, packages, Semantic Source discriminants, diagnostics,
structured signatures/imports, string literal markers, and removed-spelling
rejection use the typed lowercase registry. Promotion of this entire capsule to
Current remains blocked by the atomic removal of transitional `buf`; no edition
or compatibility mode exists.

## Identifier identity

Every language-owned or user-defined semantic name obeys:

```text
[a-z][a-z0-9]*(?:-[a-z0-9]+)*
```

Identity is exact ASCII bytes. A name starts with a lowercase letter; later
components contain lowercase letters or digits and are separated by one
hyphen. Uppercase, underscore, Unicode, leading/trailing/consecutive hyphens,
and punctuation are rejected rather than normalized.

The rule covers declarations, references, fields, bindings, functions, types,
constructors, variants, traits, type parameters, capabilities, operations,
package names, target names, and Semantic Source projections. It does not cover
string contents, documentation prose, exact OS path bytes, JSON punctuation,
third-party names, Git hashes, or immutable historical evidence.

## Source operation words

Arithmetic has exactly these source spellings:

| Removed | Canonical |
| --- | --- |
| `+` | `add` |
| `-` | `subtract` |
| `*` | `multiply` |
| `div` | `divide` |

A minus prefix remains signed literal notation. Decimal points remain floating
literal notation. No semantic operation identifier contains `+`, `*`, `=`,
`!`, `?`, `<`, `>`, `:`, `.`, `_`, or Unicode.

Ordering is `less-than`, `less-than-or-equal`, `greater-than`, and
`greater-than-or-equal`. Equality is `equal-value`, `equal-list`,
`equal-f64-bits`, and `is-same-object`. Lists use `list-prepend`, `list-first`,
`list-rest`, and `is-empty-list`.

Numeric conversions are `convert-i64-to-f64-exact`,
`convert-i64-to-f64-rounded`, `convert-f64-to-i64-exact`, and
`convert-f64-to-i64-truncating`. Arguments use `argument-count` and
`argument-at`. Length, byte access, formatting, conversion, parsing, copying,
borrowing, encoding, and decoding names state the represented unit and action.

Public resource calls use typed domain words such as `open-file-reader`,
`read-into`, `write-from`, `sync-file`, `truncate-file`, `rename-path`,
`listen-tcp`, `accept-tcp`, `receive-into`, `send-from`, `prepare-sqlite`, and
`step-sqlite`. `sys-` is not an ordinary public namespace. A documented
`host-` prefix may identify an internal provider boundary.

## Type words

| Removed | Canonical |
| --- | --- |
| `Never` | `never` |
| `Unit` | `unit` |
| `Bool` | `bool` |
| `I64` | `i64` |
| `F64` | `f64` |
| `Str` | `string` |
| `Buf` | `buf` during atomic data migration only |
| `Path` | `path` |
| `Symbol` | `symbol` |
| `Handle` / `handle` | exact typed resource kind |
| `Capability` | `capability` |
| `Owned` | `owned` |
| `Ref` | `ref` |
| `RefMut` | `ref-mut` |
| `Product` | `product` |
| `List` | `list` |
| `Option` | `option` |
| `Result` | `result` |

The implementation must not publish this capsule as Current while transitional
`buf` remains. Universal `handle` is already removed. Current owned text becomes `string`; `str` is reserved for a
borrowed valid-UTF-8 view. Type parameters are lowercase names whose binder
identity, not capitalization, distinguishes them from nominal types.

Built-in errors are `numeric-error`, `utf8-error`, and `system-error`.
Capabilities are `arguments`, `clock`, `entropy`, `file-system`, `network`,
`sqlite`, `stdio`, and `terminal`. Compiler traits are `copy`, `clone`, `drop`,
`send`, and `sync`. Prelude types and variants are lowercase, including
`option`, `result`, `some`, `none`, `ok`, `err`, `non-finite`, `out-of-range`,
`fractional`, and `inexact`.

## Structured signatures

The atom `->` is removed. A signature has two typed child fields:

```text
sig/
inputs/
i64
i64
/inputs
output/
i64
/output
/sig
```

`inputs/` may be empty. `output/` contains exactly one type. Nested type forms
remain structural. No parser, formatter, diagnostic, typed-hole candidate, or
legal action emits or accepts an arrow.

## Structured imports

An import contains raw module path data and sorted declaration identities:

```text
import/
module/
src/examples/hello/fact.lkjscript
/module
declarations/
fact
/declarations
/import
```

The path may contain filesystem punctuation. Declaration children obey the
identifier grammar. The composite `module#name-list` representation is removed
without another punctuation separator.

## Typed vocabulary authority

A dependency-free typed registry owns canonical records for contextual forms,
simple types, constructors, capabilities, compiler traits, built-in errors,
prelude types and variants, reserved words, operations, and exact removed
spellings used only by rejection diagnostics.

Each operation has one stable internal identity and one source spelling plus
category, arity, type scheme, generic variables, effects, capability needs,
ownership, trap/divergence facts, lowering facts, documentation, Semantic
Source relationship, and legal-action availability. Alias arrays are forbidden.
Compiler consumers either use these records or mechanically prove agreement.
Resolved HIR, SSA, bytecode, VM, native lowering, and proof certificates carry
stable IDs, never hot-path source string dispatch.

## Migration and identities

The compiler-owned migration resolves declarations and references, detects
lowercase collisions, rewrites structured source nodes, preserves raw strings
and paths, formats deterministically, checks the complete package graph, and
publishes atomically. It is removed after publication. Ordinary parsing never
uses it.

The language, source, diagnostics, Semantic Source, HIR, package, lock, and
artifact contract identities change as their descriptors require. Source and
package content hashes necessarily change. Resolved arithmetic identity and
mathematical semantics do not change merely because `Operation::Add` is
projected as `add`.

## Performance and deletion policy

Names resolve once before HIR. Structured signatures and imports have no
runtime representation. Word names add no runtime lookup or dispatch cost.
Old spellings, parser branches, fixtures, package values, and generated source
are deleted, not deprecated. Historical immutable evidence remains unchanged.
