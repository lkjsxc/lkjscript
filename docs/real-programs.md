# Independently authored programs

These specimens measure distance from real Rust, rather than applications written
to fit lkjscript. Upstream bytes and licences are retained once in the shared
`/home/coder/workspace/lkjscript-specimens` source store. Git records the derived
lkjscript projections, exact origins and hashes, checked edit receipts, and the
fixtures that the gate actually reads. No deployed service is changed.

## First route: levenshtein 1.0.5

Origin: [wooorm/levenshtein-rs](https://github.com/wooorm/levenshtein-rs/tree/9c4730b1973d4e61e187f8fe6d5f299ad5c991fc),
commit `9c4730b1973d4e61e187f8fe6d5f299ad5c991fc`, MIT, copyright
2016 Titus Wormer. Its manifest has no normal or development dependencies.
The fetched source has **65 library lines and 60 test lines**, not the 67 lines
shown by the published-source web view. There are eight upstream tests and
26 assertions. [Origin manifest](../examples/real_programs/levenshtein/upstream.json)
and the adjacent LICENSE preserve attribution and exact source/test/licence hashes.

### Unmodified admission and ranked barriers

The exact .rs input first refuses E_SOURCE_EXTENSION. A hard-linked .lkj path
projection preserves every byte, then exposes the first semantic refusal.
Blocked line counts below overlap: each algorithm barrier blocks the same
65-line function, rather than independent regions that may be summed.

| Priority | Cause and evidence | Sites | Lines behind it | Owner / disposition |
| --- | --- | ---: | ---: | --- |
| 1 | Open-ended range as a value: original E_UNSUPPORTED at library line 32 after iterator support | 1 | 65 | Language; usize value ranges now lower through the ordinary library, and the gate executes the exact original algorithm |
| 2 | Iterator count: located E_UNSUPPORTED at line 17 in the initial byte-identical check | 2 | 65 | Library; ordinary-source count now implemented |
| 3 | Vec collection: absent IteratorExt method in the baseline library, inventoried rather than independently exposed as the first refusal | 1 | 65 | Library; Vec-only collect now implemented; general FromIterator remains absent |
| 4 | Rust-2015 integration-crate header and crate binding | 2 header lines | 60 | Package/test adapter; both original test bodies and attributes stay exact |
| 5 | Rust source suffix | 2 source paths | 125 | Intentional .lkj ecosystem naming; path projection only |

The retained historical normalization is exactly `(1..)` to
`std::iter::range_from(1)`. It records the original port cost. Source `83f793cb1`
adds ordinary lowering for usize open-ended range values, evaluating the start
once and binding the actual library declaration. The offline gate now reverses
that normalization in fresh state, verifies the complete original 65-line library
and original test-section hashes, then executes the original algorithm. The
retained import and operation receipt are unchanged. The usize iterator, count
and Vec collector still use ordinary source, ownership checking and checked
arithmetic; no IR, interpreter, backend or runtime primitive was added.

The importer verifies the original hashes, proposes one `add-item` external-module
projection, and the real `edit` command checks and publishes it. A separate
transaction adds the common output corpus and gives only its output wrapper
Stdout. Indexed `query` precedes publication. Invalid originals cannot currently
be repaired through edit: both the original edit root and candidate must check.
The initial two-line root is explicitly uncredited bootstrap work. Import through
a checked operation does not erase the source-normalization cost.

### Oracle and measured result

All eight original tests pass separately under lkjscript native and independent
interpretation, and under the **unmodified** Rust source/tests at opt-level 0 and 3.
The independent provenance verifier reverses only the declared normalization and
restores only the exact integration header. Whole-file byte equality preserves
every test body, literal and assertion; all 18 deliberate provenance alterations
are refused.

A fixed shared corpus enumerates 85 strings of zero through three Unicode scalars
over a/b/é/🦀 and all 7,225 ordered pairs. Fourteen further ordered cases cover
combining characters, NUL, CRLF, zero-width joiners and 65-character inputs.
All **7,239 complete output rows are byte-identical on all four routes**.
The compressed Rust output is a gate input, not an unused observation archive.
The checked-IR query for `upstream::levenshtein` reports no reachable effects;
the Rust oracle is never called from the lkjscript specimen.

The three direct corpus executions were each below GNU time's 0.01 s resolution,
with load 8.35→8.35; observed peak RSS was native/Rust0/Rust3
264/1,980/2,040 KiB. These short runs establish no speed ratio. The full
interpreted corpus also completed once; its enclosing command is not an isolated
algorithm timing. Routine gates retain
the eight upstream tests on both engines and the full native output comparison;
the already-executed full interpreted corpus is an explicit slow option.

### Reproduction and boundaries

`bash tools/real-programs-test.sh target/lkjscript` is offline and needs no Rust.
`bash tools/real-programs-rust.sh` reconstructs and hash-checks the original
upstream files, then reruns both Rust profiles. The optional witness gate calls it.
`bash tools/real-programs-test.sh target/lkjscript --full-interpreter` additionally
reruns the complete interpreted corpus. Every wrapper removes its fresh state.

The library regression separately checks ownership/destructor order, terminal
None (including non-fused iterators), Unicode counts, and a maximum-valued range
under take(0). `usize::MAX` and primitive `to_string` exposed additional
adapter/test surface gaps; permanent diagnostics retain their accepted rewrites.
The corpus uses supported format syntax. No full Rust compatibility, generic
collection destination, Cargo loader, macro expansion or package authority system
is claimed by this first route.

### Integrated gate and operation adoption

The first integrated attempt exposed one pre-existing custom-count fixture:
ordinary IteratorExt count made its repaired call genuinely ambiguous. The
[checked fixture repair](../tests/fixtures/iterator_count_integration.operation.json)
preserves that call, requires the exact two-implementation refusal on both engines,
and executes an explicit Count-bound version. Method resolution is unchanged.

Source `12f11d8ae` passes the four-worker gate in **341.496 s**, load
5.09→5.13; GNU-time peak RSS is **1,396,456 KiB** for the largest process.
It reuses 4,551 authenticated case results, runs 912 fresh requests and all host
suites, and finishes 128 IR mutants with zero findings. Bootstrap takes 80.303 s,
load 5.09→4.66, and preserves the 32,594,327-byte fixed point. The seed is unchanged.
The initial failed gate is not counted as a complete pass; publication CI is next.

Independent adoption audit covers every changed `.lkj` line in the two commits:
the first route credits 382/408, and the fixture repair 31/33, for **413/441
(93.65%) through 21 successful operations**. Four scaffold lines, 22 handwritten
diagnostic lines and two comment-change lines are uncredited. Counts are added
plus deleted lines per commit, not net tree size; source normalization is still
a port cost even when the checked operation publishes it.

## Three-class cohort

The first route is complete before this cohort begins. Selection now requires
10,000–100,000 Rust lines, an authentic upstream test suite and a copying/modifying
licence. Direct code, generated code and dependency-closure sizes will be reported
separately. Initial extension-only projections precede any port. Actual checker
refusals and source inventories will remain distinguishable in the ranked table;
blocked line estimates are overlapping upper bounds, not additive totals.

## Routes for intentional refusals

These are porting rules for the forthcoming cohort, not additional observed
refusals in Levenshtein. Each actual occurrence belongs in the ranked table with
its source sites and blocked scope.

**Unsafe code.** An unsafe block cannot waive admission. Express ownership with
Box, Vec and ordinary records, use checked arithmetic and checked views, and
perform effects through explicitly granted capabilities. Existing OS operations
belong to the sealed runtime contract; ordinary code cannot add an unchecked
intrinsic or a same-address-space FFI call. When an implementation must remain
foreign, place it behind the existing explicit ForeignCommand/process boundary
and validate its returned data. This preserves a port route without treating the
foreign implementation as checked lkjscript.

**Raw pointers.** Replace address arithmetic with bounds-checked slices and
temporary shared or exclusive views. Represent persistent graphs with owned
nodes and integer handles into a Vec or map; an ordinary slot-generation field
can reject a stale handle before borrowing its node. Owners remain explicit and
views remain in locals, parameters, results or view aggregates rather than heap
containers. A design that requires stable externally visible machine addresses
must change its data interface or retain that part behind the foreign-process
boundary; integer-to-pointer casts do not provide an escape.

**Macros that execute arbitrary code.** General procedural macro expansion is
not implemented, and a generator is never admitted as part of the checker.
Run generation as an untrusted producer with explicit input and effect grants,
then check its complete ordinary source or IR output through the same admission
path as handwritten code. Existing fixed syntax such as vec!, formatting and
assertions remains available; reusable logic can also be ordinary generic code.
Generation may use semantic queries and checked edit operations, but neither a
macro name nor a generation receipt substitutes for checking the result.

## Dependency and runtime handoffs

Package/version identity, authenticated package storage and per-package authority
are the next dependency work; no manifest-only capability promise is made.
Macros must remain untrusted producers whose complete output is checked.
The fourth-note narrow Admission value and semantic message identity/generated
channel codecs remain pending on the compiler side; their current broad-World
producer and byte-channel implementations are unchanged.
