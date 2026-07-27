# Syntax And Semantics: Expressions

[Authority](../syntax.md)

## Status

**Mixed.** Current and accepted-contract features are distinguished below.

## Values And Calls

- Atoms are numeric literals, `true`, `false`, `unit`, or canonical names.
- `unit` is the sole `unit` value. `nil` is removed.
- `empty-list/ t /empty-list` creates `list t`; `is-empty-list` tests it.
- `some`, `none`, `is-some`, and `unwrap-some` operate on `option t`.
- `ok`, `err`, `is-ok`, `unwrap-ok`, and `unwrap-err` operate on results.
- `string-literal/ ... /string-literal` creates owned `string` text.
- Calls use matching open and close markers around eager child expressions.

Every source-visible name follows lowercase ASCII kebab-case. Arithmetic uses
`add`, `subtract`, `multiply`, and `divide`. Ordering uses `less-than`,
`less-than-or-equal`, `greater-than`, and `greater-than-or-equal`.

## Declarations And Signatures

Current declaration forms include `main`, `def`, `fn`, `product`, `enum`,
`trait`, and `impl`. `sig/` contains exactly `inputs/` and `output/` children;
`->` is removed. `forall/` declares lowercase type variables. `params/` names
parameters and repeats their exact structural types.

There is no `any`, implicit global, type alias, ambient module lookup, or
source-spelling alias.

## Control And Local State

Current forms include `if`, `while`, typed `loop`, `return`, `break`,
`continue`, `trap`, `exit`, `let`, `bind`, `var`, `set`, and `do`.

`if` requires a `bool` condition and equal reachable branch types. `never`
joins only with the surviving type. `loop` carries an explicit result type;
`break` must produce it. `return` must match the containing result. `trap`
accepts `string`; `exit` accepts `i64`. All transfers have type `never`.

A `var` has one declared type and exact initializer. `set` targets only the
nearest function-local `var` and returns `unit`.

## Products, Enums, And Traits

Product, enum, variant, field, trait, and type-parameter names are canonical
lowercase identifiers. Products are nominal and immutable. Generic enums and
exhaustive `match` use exact declared identities. Compiler traits are `copy`,
`clone`, `drop`, `send`, and `sync`; source cannot implement compiler-owned
roles.

## Capabilities

A capability type is structural:

```text
capability/
file-system
/capability
```

The closed kinds are `arguments`, `clock`, `entropy`, `file-system`, `network`,
`sqlite`, `stdio`, and `terminal`. Capability-bearing `main` and ordinary
functions name exact parameters. Capabilities are unforgeable and have no
ambient lookup.

## Typed Resource Foundation

The accepted typed-resource vocabulary is `input-stream`, `output-stream`,
`file-reader`, `file-writer`, `file-appender`, `directory`, `tcp-listener`,
`tcp-stream`, `sqlite-connection`, `sqlite-statement`, and
`terminal-session`.

The universal source type `handle` is removed. Current implementation carries
exact kinds through source typing, HIR, verified SSA, bytecode validation, and
the VM resource table. Resources are affine, cannot use value or object
equality, cannot escape from `main`, and cannot be stored inside unsupported
aggregates. Complete lexical cleanup, provider/state proofs, and forced native
host execution remain accepted-contract work; typed resources are therefore
not promoted to Current as a whole.

## Byte And Text Ownership Foundation

Owned UTF-8 remains `string`. The old literal marker `str/` is removed and
`str` is reserved for future borrowed text.

The existing whole-place ownership slice now has direct source types:

- `byte-vector`: affine mutable storage;
- `byte-slice`: shared lexical view;
- `byte-slice-mut`: exclusive lexical view.

`new-byte-vector`, `byte-slice-length`, `byte-slice-byte-at`, and
`byte-slice-mut-set-byte` exercise that slice. `move`, `borrow`, and
`borrow-mut` require whole local places. Views cannot be returned, stored in
aggregates, captured, or carried across unsupported control flow.

Transitional `buf` remains Current for the unmigrated corpus. Immutable
`bytes`, arbitrary ranged byte views, and borrowed `str` remain accepted but
non-Current. No alias equates `buf` with those destination semantics.

## Numeric Contract

Only `i64` and `f64` are numeric source types. Arithmetic is checked `i64`
unless explicit conversion produces `f64`; implicit mixed arithmetic and
ordering are rejected. Conversions are:

- `convert-i64-to-f64-exact`;
- `convert-i64-to-f64-rounded`;
- `convert-f64-to-i64-exact`;
- `convert-f64-to-i64-truncating`.

Equality families are `equal-value`, `equal-list`, `equal-f64-bits`, and
`is-same-object`. No resource has object identity.
