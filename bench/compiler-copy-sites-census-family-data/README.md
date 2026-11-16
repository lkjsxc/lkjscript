# Complete-family census of the frozen 20 selected bodies

No Struct root in this bounded population qualifies for the frozen initial
subset. This conclusion comes from every constructor and every flat-shaped local
in the complete bodies, including unsampled values and constructors with scalar
field-copy payloads. It is not extrapolated from the earlier payload screen.

The one separately authorized offline inventory ran once and succeeded. It read
the accepted extraction and immutable all-row join, without running a compiler,
profiler, official reader, accounting helper, extractor or old join, and without
requesting another detail page. No tracked source changed.

## Population and disposition

All 20 original function indices and their first-appearance order are unchanged.
The bodies contain 6,202 instructions and 64 Struct constructors. There are
13 locals with the initial flat Copy IR-Struct shape, all t41
(diagnostic::Span), in three functions. None is a parameter.

| Complete-body group | Flat-shape locals | Struct roots | Flat-shape Struct roots |
| --- | ---: | ---: | ---: |
| fn7710 expr_with_inference | 6 | 0 | 0 |
| fn5719 definition_types::instantiate | 6 | 6 | 0 |
| fn2652 scoped::check | 1 | 12 | 1 |
| Other 17 selected bodies | 0 | 46 | 0 |
| Total | 13 | 64 | 1 |

Every constructor is retained in phase-1/output/struct-roots.tsv and its JSON
record. The mutually exclusive dispositions are:

| Roots | Independent blocking rule |
| ---: | --- |
| 60 | Destination is a Move Struct, outside the Copy subset. |
| 1 | Copy closure t3277 has a reference field t1286. |
| 2 | Copy IR tuple t3284 has enum field t1325. |
| 1 | Fresh flat-shaped Span is passed whole as a call argument. |

The type block for t3277 is at fn2652 i646; the t3284 constructors are fn690
i232 and i267. Those nonflat type rules suffice independently of nominal origin.
Some blocked types also have custom destructors or other nonflat fields; the
complete records retain those reasons. On already blocked nonflat shapes, the
reader label `no-nonzero-field` means no recognized nonzero scalar field, not
that the aggregate occupies zero bytes. Their layout is explicitly not derived.

Six constructor records have no matching recorded copy interval; another 32
have matching intervals with zero samples. All remain in the inventory. The
instruction-level interval association is evidence of relevant coordinates,
not an exclusive attribution of samples to a proven family. Overlaps must not
be summed as removable coverage.

## The fresh root that the payload-only screen did not reveal

The complete fn2652 body is 1,339 instructions and 701 locals, with five
parameters. At i1262 it constructs l698 of t41:

```text
@108/12:9 struct 698 693 696 697 694 695
@108/19:9 read 699 copy 692
@108/20:9 read 700 copy 207
@108/11:5 call 208 1295 698 699 700
```

The field operands, in declaration order, are l693/l696/l697/l694/l695, all
t4/usize. They initialize file/start/end/line/column. Span has five scalar fields,
width 40, alignment 8 and offsets 0/8/16/24/32 under the pinned native layout.
The complete destructor table has no entry for t41 or t4; these scalar fields
and their Copy aggregate require no native cleanup. IR shape and name do not
independently establish nominal source origin.

The complete body gives l698 exactly one whole definition and one use. It has
no earlier use, whole-local Read descendants, borrow/view, field read, partial
write, aggregate return or Drop. The one use is the first argument of f1295 at
i1265. The retained signature is diagnostic::new with parameters t41, t985 and
t985 and result t43. The call consumes l698; its Copy shape does not remove
that initialization effect. The callee's body and behavior are not inferred.

There is no branch or join between the constructor and its sole consuming use.
The surrounding loop is not treated as proof of a live backedge crossing.
The aggregate-argument exclusion alone blocks the frozen subset, independently
of the unresolved nominal-source point. There is no recorded raw copy interval
at the root or its use. That absence proves neither no execution nor no cost.

manual-review.json retains these separate shape, freshness, consumption,
control and use decisions. manual/fn2652-body.lkjir is the complete original
body; its type closure, full shared destructor table, exact callee signature
and coordinate excerpt are copied with unchanged hashes. They are review
fragments, not a newly admitted standalone program.

## The other flat-shaped locals

The six fn7710 Span locals arise from projected reads, a consuming Assign,
a whole-local Read and Replace results. Its one same-type whole-local Read
edge is i16, l5 to l17. There is no Struct root in the body. The projected
origins fail the fresh-root rule, and the Replace uses are outside the initial
use subset. This statement does not assume the intervening callee is irrelevant.

The six fn5719 Span locals form the two independently reviewed chains:
i105 read to l30, i106 Assign to l31, i113 Read to l37, i114 consuming call;
and i310 read to l97, i311 Assign to l98, i318 Read to l104, i319 consuming
call. Neither begins with Struct, and both pass the Span as an aggregate
argument. The retained values cross branches and joins but are dropped before
their respective loop backedges. No live-backedge claim is made.

The separate fn5719 proof is frozen at:
`/home/coder/workspace/lkjscript-compiler-scale-refusal-20261116/target/compiler-copy-sites-fn5719-review.20261116-1.bcni8c`,
manifest c7438357fcb0247d2ef379722303a33b3fb4e49f331bd620ad59a9881090a0ba.
It is not replaced or rerun here.

Across all 13 flat-shaped locals, the inventory records 13 whole definitions,
21 use events (including three assignment destination writes), and all three
same-type whole-local Read edges. All definitions and uses are available in
phase-1/output/flat-shape-definitions-uses.tsv. Only the one fresh Struct has
a root-family JSON; the complete local tables also account for incoming-storage,
assignment and intrinsic-result origins.

## Scope, counters and uncertainty

All 64 Struct constructor roots have a known independent blocker. The other 12
flat-shaped locals do not originate in Struct within these bodies. Therefore
there is no qualifying fresh-Struct family for this exact initial subset in
these 20 bodies. No positive eligibility claim relies on an unestablished
nominal origin, missing callee behavior or invented per-instruction loan state.

The earlier accepted join remains unchanged: 258 raw rows and 469 samples
belong to these functions, including 188 unshown rows and 39 unshown samples.
The four Span payload rows have 21 samples; they are the projected-read cases,
not this fresh constructor. Outside the population are 67,834 rows and 1,034
samples. Global unmapped and in-flight detail counts are zero in that capture.
These are observed attribution counts, not byte traffic, removed time or a
speedup forecast. The bounded review does not establish a whole-program zero
and implements no optimization pass.

## Execution and proof

| Field | Retained value |
| --- | --- |
| Gateway request | compiler-copy-tests-family-once-1261 |
| Invocations | 1 |
| Exit | 0 |
| GNU time elapsed / user / system | 0.38 / 0.36 / 0.01 s |
| Peak RSS | 89,660 KiB |
| Load before and after | 1.32 / 1.35 / 1.70, unchanged |
| Limits | 4 GiB address space; 300 s watchdog; 10 s grace |
| stderr | 0 bytes |
| Inventory script SHA-256 | b4f3df6b6ffc443d7b3baf839b4e1dbdbb664a21f8ff366a437f05c5114113a6 |
| Output manifest SHA-256 | 7c14fc93ff46c4ad19ba9d20625d9572e25a599e73066d0b826df0a925c74e9a |
| Result SHA-256 | 1390bc573b5483ab08ae85c19383983132712b9e9bf57d3b9754cee4e023042e |

The manifest-verified inputs are extraction63d59716..., joinb69c1d21... and
companion32c0f43d.... Full pins, script, argv/cwd, limits, captures and loads
are retained. Peer review of the exact inventory script preceded execution and
covered operand positions, Read closure and scalar-constructor completeness.
The original join warning bytes and its separate archive remain intact.
