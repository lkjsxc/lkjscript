# Unsafe Boundary Registry

## Status

**Current.** `LKJ-UNSAFE-BOUNDARY` checks
[registry.json](registry.json) against authored Rust sources. Its structural
shape is documented by [registry.schema.json](registry.schema.json).

## Contract

Each registry entry is one stable reviewed boundary with a responsibility, safe
caller contract, and at most 16 files. There are at most 16 boundaries. Boundary
IDs and file lists are sorted and unique.

The scanner walks authored `.rs` files while excluding repository metadata and
generated artifact trees. It recognizes an exact Rust `unsafe` code token while
ignoring line comments, nested block comments, character literals, ordinary
string literals, and raw string literals. Identifiers merely containing the
word do not match.

The relation is bidirectionally exact:

- every file with an `unsafe` code token is registered exactly once; and
- every registered path is a regular Rust file containing an `unsafe` token.

A boundary may be located outside `lkjscript-sys` after architecture and safe
caller-contract review. The Current registry contains only inherited
`lkjscript-sys` files; this slice moves no unsafe code.

## Command

```sh
cargo run --locked -p lkjscript-xtask -- check-unsafe
```

The command is part of `quiet verify`. Registry or scanner failure is closed and
reports `LKJ-UNSAFE-BOUNDARY`.
