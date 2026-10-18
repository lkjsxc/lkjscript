# Current status

Updated 2026-10-18. Accepted typed meaning remains the sole editable authority.
Production bytecode is checked against an independent canonical interpreter.
Verification has no three-minute requirement; elapsed time is an observation.

Phase 1 implements one checkout and one shared Cargo target, bounded copy-on-write
memory graph/data projects, shared admitted standard packages/programs and one
current schema epoch. Default checks retain a bounded receipt and remove joined
scratch. Release handoffs explicitly retain originals; previous formats have no
fallback reader.

Phase 2 adds `tx` for symbolic upserts with identity preservation, impact-scoped
differential tests before atomic publication, and one bounded JSON result. `gc`
retains accepted history and selected package/compiler roots, then compacts disk
objects into one deflate pack with raw fallback. Memory collection filters a
copy-on-write index; existing views remain valid. Failed preparation cannot publish.
Its changed and fresh full runs each passed all 29 gates.

Phase 3 connects lexical region allocation, reads, writes, freeze, relative offsets
and digest-bound slab persistence to both evaluators. A 64-byte-aligned buffer owns
the complete slab; nodes have relative pointers and no reference counts. Frozen
reads share that arena through joined parallel tasks; unique custody can move into
a child. Runtime generations reject stale custody, and compiler control-flow
admission prevents bypassing lexical cleanup. DataStore transactions persist exact
image bytes and load them into an aligned buffer after complete digest/layout
admission, without node deserialization or external mapping crates.

Additive planner fixtures cover cyclic stage storage, frozen parallel traversal
and DataStore persistence. The existing planner entrypoints and all 176 exact
IndexedData/BlockRead expectations remain unchanged. The
[region specification](spec/regions.md) states supported cell layouts and boundaries.

## Verification and next work

The bounded `.artifacts/lkjscript-dev/latest-receipt.json` records the latest local
acceptance result, exact gate inventory and input stability. Required verification
includes changed, fresh full, complete workspace harnesses, maintained projects,
independent expectations, real process/network lifecycles and release target policy.
Generated discovery comes from the supported capabilities operation.

Phase 4 adds bounded trap-preserving native algebraic optimization and standalone
Linux x86_64 ELF emission with direct three-way execution checks. Native lowering
leaves metered bytecode unchanged. The [native backend contract](spec/native-elf.md)
states its supported instructions, I64 argument boundary and quota distinction.
The self-hosting contract now targets Tier 2 semantic compiler closure; the first
Tier 1 backend component validates LKJCI1 normalized I64 input and emits ELF on
both evaluators. Rust source admission/export remain required. The maintained
meaning graph and [schema/dependency inventory](spec/selfhosting.md) identify the
current boundary. Tier 1 native compiler execution now agrees with both evaluators;
backend closure, semantic admission, both formats and C2/C3 proof remain next.

Latest Attempt: running — final Milestone C acceptance requires a clean candidate
verifier bound to its changed production hash. Five compiler tests, eight native
runtime tests and two public CLI tests passed, including malformed input and
source mutation. Final checker receipts establish acceptance before integration.
Last Accepted Commit: `892b46da` (LKJCI1 backend slice, integrated on origin/main).
Its five focused tests and both 29-gate changed/fresh-full runs passed; all gates
executed freshly and their input snapshots stayed stable. The bounded receipt
`.artifacts/selfhost/acceptance.json` binds its exact source/tree and limitations.
Known Limitations: no self-produced `.lkja`, complete backend compiler closure,
semantic checker or C2/C3 proof. Required broader types,
ownership, witnesses and regions remain in the Tier 2 acceptance profile.
Git records integration; local acceptance does not establish hosted release or
public promotion. [Verification](spec/verification.md) owns proof and gates;
[release](release.md) owns candidate and publication authority.
