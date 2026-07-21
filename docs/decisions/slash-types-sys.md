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
4. **Core types** — `Nil`, `Bool`, `Int`, `Float`, `Str`, `Buf`, `Symbol`,
   `(List T)`, `(Fn …)`, `Handle`, `(Option T)`, `(Result T E)`, plus
   user `type/` aliases as needed. Errors use `Result` (no exceptions).
5. **Safe sys** — Scripts see only opaque `Handle` + `Result`; raw fds stay
   inside VM/`lkjscript2026-sys`. Portable façade module; Linux impl first.
6. **GC** — Precise mark-sweep replaces bump-only; value tags include Handle.
7. **JIT** — Stub + typed call hooks only in this cut.

## Example
```text
def/
  name/ wait-ms /name
  fn/
    sig/ Int -> Nil /sig
    params/ ms Int /params
    sys-wait-ms/ ms /sys-wait-ms
  /fn
/def
```

## Consequences
- Supersedes [xml-surface.md](xml-surface.md) for new sources.
- Full corpus migration required; honesty gates must stay green.
- Token/fan-out constants may be retuned with docs if signatures demand it.

## Rejected
- Gradual/optional typing
- Raw integer fds in the script API
- Shipping non-Linux backends in this cut
- Full baseline JIT in this cut
