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
The self-hosting contract targets Tier 2 semantic compiler closure. The Tier 1
backend validates LKJCI1 profile 4 byte/aggregate and canonical-unit projections,
emitting ELF or `.lkja`. Rust source admission/export remain required.
The maintained meaning graph and [schema/dependency inventory](spec/selfhosting.md)
identify the current boundary. Focused native backend reproduction has identical
C2/C3 ELF and A1/A2/A3 artifact bytes, including changed-source behavior.

Latest Attempt: running — source-record decoder acceptance and GC-test retry.
Six decoder tests passed across VM, reference and native execution. Maintained
proposal replay matched all 289 canonical units and the native backend closure.
The readers cover every current compiler owner/type record; graph and semantic
admission remain ahead. Fresh full acceptance is pending for this candidate.
The GC test checks duplicate reader descriptors and bounded retry after local
readers drop; production compaction still rejects active readers without waiting.
Last Accepted Commit: `108906b5` (Tier 1 both-output closure, on origin/main).
All 34 compiler and ten native runtime tests passed inside official full;
the library passed 2589 tests, with five explicitly ignored.
The clean candidate changed check selected two gates; fresh full passed all 29
gates, all freshly executed with stable input, and release-target policy passed.
`.artifacts/selfhost/accepted-d-artifact/receipt.json` binds source and limitations.
Known Limitations: the source decoder has no graph or semantic checker yet.
Native allocation is process-owned, with no lexical region reclamation or RSS
claim. Broader types, witnesses, regions and source-graph C2/C3 proof remain required
for Tier 2; these byte fixed points still consume Rust-admitted input.
Git records integration; local acceptance does not establish hosted release or
public promotion. [Verification](spec/verification.md) owns proof and gates;
[release](release.md) owns candidate and publication authority.
