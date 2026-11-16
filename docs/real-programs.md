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
| 1 | Open-ended range as a value: located E_UNSUPPORTED at library line 32 after iterator support | 1 | 65 | Language; pending ordinary lowering, one explicitly counted producer normalization |
| 2 | Iterator count: located E_UNSUPPORTED at line 17 in the initial byte-identical check | 2 | 65 | Library; ordinary-source count now implemented |
| 3 | Vec collection: absent IteratorExt method in the baseline library, inventoried rather than independently exposed as the first refusal | 1 | 65 | Library; Vec-only collect now implemented; general FromIterator remains absent |
| 4 | Rust-2015 integration-crate header and crate binding | 2 header lines | 60 | Package/test adapter; both original test bodies and attributes stay exact |
| 5 | Rust source suffix | 2 source paths | 125 | Intentional .lkj ecosystem naming; path projection only |

The normalization is exactly `(1..)` to `std::iter::range_from(1)`.
It is an untrusted producer workaround for a language gap, not unmodified Rust
acceptance. The usize-only iterator, count and Vec collector use ordinary source,
existing ownership checking and existing checked arithmetic; no IR, interpreter,
backend or runtime primitive was added.

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

## Three-class cohort

The first route is complete before this cohort begins. Selection now requires
10,000–100,000 Rust lines, an authentic upstream test suite and a copying/modifying
licence. Direct code, generated code and dependency-closure sizes will be reported
separately. Initial extension-only projections precede any port. Actual checker
refusals and source inventories will remain distinguishable in the ranked table;
blocked line estimates are overlapping upper bounds, not additive totals.

## Dependency and runtime handoffs

Package/version identity, authenticated package storage and per-package authority
are the next dependency work; no manifest-only capability promise is made.
Macros must remain untrusted producers whose complete output is checked.
The fourth-note narrow Admission value and semantic message identity/generated
channel codecs remain pending on the compiler side; their current broad-World
producer and byte-channel implementations are unchanged.
