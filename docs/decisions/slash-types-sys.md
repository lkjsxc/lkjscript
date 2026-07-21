# Slash grammar, mandatory types, safe sys

## Context
XML `<>` surface and untyped defs block the performance roadmap’s types arc.
User authorized a full rewrite: mandatory signatures, Result-aware checking,
opaque `sys-*` handles, and bundling precise GC. Multi-OS is a future goal;
Linux backend is fine now behind a portable façade.

## Decision
1. **Slash surface** — Open forms end with `/` (`if/`), close forms start with
   `/` (`/if`), atoms have no slash (`xs`, `0`, `"str"`). Whitespace separates
   tokens. Strings use `"` with `\` escapes. Comments: `;;` to EOL.
2. **Structural `/` only** — Arithmetic division is the atom/call name `div`
   (not `/`), so close/open markers stay unambiguous.
3. **Mandatory types** — Every `def` of an `fn` carries `sig/` … `/sig` and
   typed `params/` pairs `name Type …`. Untyped defs are hard errors. Builtins
   and `sys-*` live in a typed prelude.
4. **Core types (sized; no `Any`)** — `Nil`, `Bool`, sized numerics
   `I32`/`I64`/`U32`/`U64`/`F32`/`F64` (aliases `Int`→`I64`, `Float`→`F64`),
   `Str`, `Buf`, `Symbol`, `List T`, `Handle`, `Option T`, `Result T E`,
   plus user `type/` aliases. `Any` is rejected. Errors use `Result`
   (no exceptions). Parametric polymorphism is **annotation-driven**:
   `forall/ T /forall` on fns; call sites instantiate from argument types
   (no Hindley–Milner inference). No traits/typeclasses.
5. **`print` is `Str`-only** — Convert with `str-from-i64` / `str-from-byte`.
6. **Literals** — `2` is `I64`; `2.0` (source contains `.`) is `F64`.
7. **Safe sys** — Scripts see only opaque `Handle` + `Result`; raw fds stay
   inside VM/`lkjscript2026-sys`. Portable façade module; Linux impl first.
8. **GC** — Precise mark-sweep replaces bump-only; value tags include Handle.
9. **JIT** — Stub + typed call hooks only in this cut.

## Example
```text
def/
  name/ wait-ms /name
  fn/
    sig/ I64 -> Nil /sig
    params/ ms I64 /params
    unwrap-ok/ sys-wait-ms/ ms /sys-wait-ms /unwrap-ok
  /fn
/def

def/
  name/ list-len /name
  fn/
    forall/ T /forall
    sig/ List T -> I64 /sig
    params/ xs List/ T /List /params
    if/
      null?/ xs /null?
      0
      +/ 1 list-len/ cdr/ xs /cdr /list-len /+
    /if
  /fn
/def
```

## Consequences
- Supersedes [xml-surface.md](xml-surface.md) for new sources.
- Full corpus migration required; honesty gates must stay green.
- Token/fan-out constants may be retuned with docs if signatures demand it.
- Numeric widths are type-level today; VM still stores tagged i64 + heap float
  (narrower widths checked at the type layer; casts are explicit).

## Rejected
- Gradual/optional typing
- The type `Any`
- Traits / typeclasses for ad-hoc polymorphism
- Full Hindley–Milner inference
- Raw integer fds in the script API
- Shipping non-Linux backends in this cut
- Full baseline JIT in this cut
