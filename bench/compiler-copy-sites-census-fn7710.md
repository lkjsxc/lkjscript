# Complete Span review in selected function 7710

The complete [retained body](compiler-copy-sites-census-extraction-data/fn7710.lkjir)
is `elaborate::expr_with_inference$f1147<Self=t122;>` in the exact accepted
selected companion. The name is a producer label; numeric coordinates and
body SHA-256 `2dd35e998d34896cf8e255f30a365da643f65abe71d6d50c897c366d07f1a435` bind this instance.
The body has 29 instructions, 22 locals and four parameters. All bytes and
coordinates were independently verified by the extraction audit.

There are six type-41 locals: 4, 5, 7, 9, 17 and 19. None is a parameter.
**There are no Struct instructions of any type in the complete body.**
Both sampled Span chains therefore fail the frozen fresh-Struct condition,
independently of the unresolved source-nominal qualification.

| Chain | Complete Span value events | Original sampled row |
| --- | --- | --- |
| Previous Span | i0 reads existing self storage into l4; i1 consumes l4 into l5; i16 copies l5 into l17; i18 consumes l17 in replace and produces l19; i19 drops l19; i23 drops l5 | 40165, i0, 1 sample |
| Expression Span | i3 reads indexed compiler storage into l7; i5 consumes l7 in replace and produces l9; i6 drops l9 | 40166, i3, 10 samples |

The source places are exactly `0 deref field 14` at i0 and
`0 deref field 3 deref field 0 deref field 6 index 6 field 2` at i3.
The joined contracts establish type 41, width 40 and the corresponding
projected/dereferenced/indexed roles. The type has five usize fields and no
custom destructor or native cleanup; this does not establish a fresh origin.

The retained l5 snapshot spans the replacement at i5, the mutable-self call
to f8359 at i14, and the replacement at i18. Mutable borrows at i4/i10/i17
target compiler storage, not the six Span locals. Redirecting the later read
to the original self field would require preserving the old value across
those operations; the body supplies no irrelevance proof. Assign and replace
consume their operand locals even though Span has the Copy flag.
The frozen initial subset also excludes replacement and these call barriers.

The body contains no branch or loop. No loop-lifetime blocker is invented.
There is no direct scalar-field read or direct borrow of a Span local.
The two intervals have 11 samples in total. Replace and any other work outside
the existing KIND_COPY classification are not added to that count. Nothing
here predicts removed time or classifies another function.

[Structured review](compiler-copy-sites-census-extraction-data/fn7710-review.json)
records all six locals, both complete chains and the separate interpretation
limits. No compiler, extractor, join, profile or transformation was rerun.
