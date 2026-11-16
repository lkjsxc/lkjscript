# Complete fn5719 Span review — 2026-11-16

The two sampled Span chains are **blocked for the frozen initial subset**. Both start with an indexed, dereferenced read from incoming compiler storage, and both pass the whole Span to a callee. Neither starts with a Struct instruction. The retained value also spans branches and joins. This is a verdict on these two chains only, not on the twenty-function population or all scalar constructor payloads.

## Exact population

The complete retained body has 435 instructions, 211 locals and 3 parameters. Its SHA-256 is 22be5dcdac99e1c8d4a10492d438482363cb65c1b7c214579906261b6dc3eec9. Coordinates below are zero-based selected-IR instruction/local indices; source file is 82 unless a record explicitly overrides it. The checked companion and frozen plan pins are in review.json and input-pins.json.

There are exactly six locals of t41: l30, l31, l37, l97, l98 and l104. None is a parameter. All six Struct definitions in the body have other destination types. span-locals.tsv and span-uses.tsv retain every declaration and every instruction mentioning these six local coordinates; relevant-instructions.tsv supplies the original byte ranges, source coordinates and exact record hashes from the accepted extraction.

| Chain | Storage read | Transfer and retained copy | Call consumption | Destruction | Joint row |
| --- | --- | --- | --- | --- | --- |
| Expression | i105 defines l30 from compiler.source.exprs[l29].span | i106 consumes l30 into l31; i113 copies l31 to l37 and retains l31 | i114 consumes l37 as f6852 argument 5 | i126 on error, i179 on normal continuation | raw32811, 9 samples, displayed rank 13 |
| Pattern | i310 defines l97 from compiler.source.patterns[l96].span | i311 consumes l97 into l98; i318 copies l98 to l104 and retains l98 | i319 consumes l104 as f6852 argument 5 | i331 on error, i384 on normal continuation | raw32818, 1 sample, unshown |

Both joint intervals are disjoint, width 40, typed-place to local, source flags 7 (field+deref+index). These two rows account for 10 observations. Other value-transfer instructions are not extra profile rows or extra observations; their absence from the joint table does not prove they execute no native code.

## Types, uses and control

The supplied t41 declaration is a Copy IR Struct with five t4/usize fields. Its native field offsets are 0, 8, 16, 24 and 32, alignment 8 and extent 40. The complete destructor table has no declaration for t41 or t4; the scalar fields require no native cleanup. This establishes shape and layout only. The supplied IR name diagnostic::Span does not establish source nominal origin; that remains explicitly unestablished.

No instruction borrows, returns, partially writes, replaces or directly reads a scalar field of any of the six Span locals. Their only whole-local Read edges are i113 and i318. Assign is a consuming transfer, not an alias: l30/l97 become absent, l31/l98 become initialized. The call consumes l37/l104 even though t41 is Copy. Those temporaries need no later Drop. The retained l31/l98 remain initialized until the branch-specific Drops. Cleanup-free native storage does not erase these IR state changes.

The first chain lies in loop i72..i212 and the second in loop i277..i417. The retained values cross the call-result conditions and the normal-path joins i145..i150 plus i151..i178, or i350..i355 plus i356..i383. Error paths drop the Span before Return i137/i342. Normal paths drop it before Continue i211/i416. No Span member is claimed to survive a loop backedge; the failure of the straight-line rule is supported by the live branching region, not merely by textual placement inside a loop.

The surrounding borrows at i108/i110/i112 and i313/i315/i317 target self, compiler and known; later i159/i364 borrows target result-map fields. None borrows a Span local. f6852's retained signature takes Span by value as its sixth parameter. Conditional calls to f1814 at i162/i367 occur while the retained Span remains live. Only callee signatures were supplied; no claim of callee irrelevance or behavior is made. The family's aggregate argument use already blocks the initial subset.

## Evidence and limits

body.lkjir and the two callee signatures are exact retained input bytes. joined-span-rows.tsv is the exact header plus the two matching rows from the accepted all-row join. type-context.lkjir is an exact-record excerpt for the storage path and shape, **not** a closed standalone program; the complete 449-type context remains bound by its accepted input hash. control.json records structured region boundaries, and review.json gives the scoped decision and accounting.

The accepted join emitted Perl portability warnings for its hexadecimal mask; those warnings remain with its owner. This review neither repairs nor reruns that result. No compiler, extractor, official reader, join, accounting helper, profile or transformation ran. The analysis does not recover serialized loan state, predict removed time, multiply samples by width, or claim a global zero of eligible families.
