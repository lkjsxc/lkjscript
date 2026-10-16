# Current status

Updated 2026-10-16. Integrated source `507efb03` completed hosted
[Verify](https://github.com/lkjsxc/lkjscript/actions/runs/38762806893) and
[Release candidate](https://github.com/lkjsxc/lkjscript/actions/runs/38767128052).
Verify took 128m54s. Candidate success does not establish public promotion.

Phase 1 implements one checkout and one shared Cargo target, bounded copy-on-write
memory graph/data projects, shared admitted standard packages/programs and one current
schema epoch. Default checks retain a bounded receipt and remove joined scratch;
release handoffs explicitly retain originals. Previous formats have no fallback reader.

Phase 2 adds `tx` for symbolic upserts with identity preservation, impact-scoped
differential tests before atomic publication, and one bounded JSON result. `gc`
retains accepted history and selected package/compiler roots, then compacts disk
objects into one deflate pack with raw fallback. Memory collection filters a
copy-on-write index; existing views remain valid. Failed preparation does not publish.

## Established capability

Accepted typed meaning remains the sole editable authority, with atomic publication,
exact offline packages, affine ownership, explicit generic witnesses, nominal
records/sequences, associated owned types, source-tied views, scoped parallel reads
and shared multi-instance runtime. Production bytecode is checked against an
independent canonical interpreter. Native dependency validation, component analysis
and stage planning are maintained consumers.

All five maintained projects passed the same 477 differential tests before and after
the epoch reset: standard 116, lkjournal 44, planner 176, policy 62 and guides 79.
Complete closures were re-admitted through ordinary publication without changing
pinned dependencies, and their transports and artifacts match independent derivation.

## Local verification and next work

The Phase 2 implementation passed all 29 changed gates in 272.29 seconds and all
29 fresh full gates in 275.83 seconds, with stable inputs and no reused gate results.
The full compact receipt was 24,655 bytes. Complete workspace harnesses, independent
semantic expectations and actual process/network lifecycles remain covered. The
full receipt digest is
`verification_101b64c62d520c2fb2f691b9471a800f6ef33bb6ca3a7d72272e2f0da9466655`.

A command project with parallel functions compacted eight packs from 1,339,418 to
1,318,600 bytes. HEAD and generated artifact bytes were unchanged; all 117
differential tests passed afterward. Reader exclusion, retained history, corrupt
compression, read bounds and 21 compaction interruption points are covered.
These are local results; remote main remains `507efb03`, verified independently,
and no new publication is claimed. Only this status summary changed after acceptance.
Wall time is measured and reported; acceptance has no three-minute requirement.

Next implement regions, effect/resource analysis, equality optimization, native ELF
and self-hosting. Region language/runtime integration has not begun.
Each increment requires independent semantic and boundary checks before normal main
integration. [Verification](spec/verification.md) owns gates and evidence;
[release](release.md) owns exact candidate and publication authority.
