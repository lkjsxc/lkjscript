# Canonical Lowercase Vocabulary Migration Evidence

## Status

**Historical migration evidence.** This records the one-time source publication
from `c17008ffa8dedfe0e010d58dddb05071836103c1`. It is not a parser fallback,
edition, alias table, or Current source authority.

## Corpus Identity

The tracked corpus contained 133 `.lkjscript` files; every file changed. The
corpus digest framed each sorted entry as decimal path length, path bytes,
decimal content length, then content bytes before SHA-256.

- before: `4c9712d62ed4fccfa7ef18ab8b6ce20fea3d86f403d0b6e63d7c35ed33769f60`
- published tree: `4a9b9670105742cc93dcc6f6df4f30dffd33149e11156adbd095826d52b9f623`
- lowercase declaration collisions: zero across 238 declaration-name forms

The package manifest gained the previously omitted
`src/std/fs/absolute-path.lkjscript`; the canonical lock was regenerated from
the complete 133-module graph.

## Raw Text Accounting

The old corpus had 42 `str/` payloads. The final corpus has 41
`string-literal/` payloads:

- 40 payload byte sequences are unchanged;
- `sys-read-byte` was intentionally changed to the public canonical diagnostic
  text `read-resource-byte`;
- the `sys-open-read` payload was removed when the Brainfuck open helper stopped
  duplicating a resource-bearing result solely to customize that error.

No path payload was normalized. Structured imports carry module path bytes in
`module/` rather than punctuation-separated semantic atoms.

## Identity And Rejection

The operation registry retains all 123 original positional stable identities.
An exhaustive compiler agreement test binds each enum identity to its stable
record and checks canonical type schemes, generic variables/constraints,
effects, capabilities, ownership, lowering, Semantic Source relationship, and
legal-action availability.

The final removed-spelling registry contains the old operation/type/capability/
trait/prelude spellings plus removed arrows, literal markers, ownership forms,
and universal `handle`. Compiler tests generate a rejection case from every
record. Ordinary parsing performs no migration.

Temporary migration and fixture-rewrite programs lived only under
`target/lkjscript/` and were deleted after this compact evidence was retained.
