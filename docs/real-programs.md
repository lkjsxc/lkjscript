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
The checked-IR query for `upstream::levenshtein` reports no reachable capability operations;
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
The initial failed gate is not counted as a complete pass. Publication
[CI 43990653851](https://github.com/lkjsxc/lkjscript/actions/runs/43990653851)
passes both the required gate and optional Rust witness. The completed first-route
build outputs and gate logs have been removed; the exact cleanup and CI result are
retained in the [operation receipt](../tests/fixtures/real_programs_levenshtein.operation.json).

Independent adoption audit covers every changed `.lkj` line in the two commits:
the first route credits 382/408, and the fixture repair 31/33, for **413/441
(93.65%) through 21 successful operations**. Four scaffold lines, 22 handwritten
diagnostic lines and two comment-change lines are uncredited. Counts are added
plus deleted lines per commit, not net tree size; source normalization is still
a port cost even when the checked operation publishes it.

## Three-class cohort

The first route and its own-test oracle completed before this cohort began.
All three roots below were checked without source edits. None yet admits as a
large lkjscript program, and none has run its own suite under lkjscript. Their
Rust baselines establish the upstream oracle; they do not establish a completed
port or a native speed ratio.

| Class and pinned origin | Revision | Selected physical Rust lines | Licence |
| --- | --- | --- | --- |
| Generic computation: [nom 8.0.0](https://github.com/rust-bakery/nom) | 2cec1b3e4c9ccac62c902d60c00de6d1549ccbe1 | 21,397 in 28 library files; 2,178 in 16 external test files | MIT |
| Browser-shaped tree: [dom_query 0.28.0](https://github.com/niklak/dom_query), with its actual HTML/selector core below | 186c02d9359762404054fbc8af3a5b07cdf795a9 | Root: 7,773 in 31 library files, 4,090 in 12 external test files. Named core: 32,180 library lines in 80 files | Root MIT; dependency licences below |
| Concurrent network service: [Sōzu 1.0.6](https://github.com/sozu-proxy/sozu) | c5ff06a12bd514e06ab28965052d0ad1e19c45b3 | 38,575 in 74 selected production/build-script files; 2,976 in 13 end-to-end files. All 92 tracked Rust files: 42,038 | bin/lib AGPL-3.0; command LGPL-3.0; end-to-end root AGPL-3.0 |

The DOM root alone is below 10,000 implementation lines. Its measured class is
the actual parser/tree/selector stack. This named core is deliberately smaller
than its complete dependency closure:

| Package | Library files | Physical library lines | Recognized test-only lines | Remainder | Licence |
| --- | ---: | ---: | ---: | ---: | --- |
| dom_query 0.28.0 | 31 | 7,773 | 1,183 | 6,590 | MIT |
| html5ever 0.39.0 | 15 | 7,712 | 306 | 7,406 | MIT OR Apache-2.0 |
| markup5ever 0.39.0 | 6 | 1,290 | 106 | 1,184 | MIT OR Apache-2.0 |
| selectors 0.38.0 | 15 | 8,918 | 1,176 | 7,742 | MPL-2.0 |
| cssparser 0.37.0 | 13 | 6,487 | 1,414 | 5,073 | MPL-2.0 |
| **Named core** | **80** | **32,180** | **4,185** | **27,995** | |

These are physical lines including comments, blanks and optional branches, not
SLOC or exact active-code counts. Separate core external tests total 4,201 lines,
examples 1,089 and benchmarks 506; selectors' build script adds 77. Its generated
selector file is excluded and unmeasured. web_atoms, tendril, servo_arc and other
dependencies are outside this named core. All four registry archives were checked
against their pinned checksums, and the reported cached Rust/notice/manifest bytes
were independently compared with those archives. selectors' MPL notices are
source headers rather than a standalone LICENSE. The exact package/file inventory
and notice paths are in [DOM evidence](real-programs-dom.json).

### What unmodified measured

The source store retains each pristine checkout, manifest, notice and asset once.
The projection tool verifies Git index against HEAD and raw Git blobs, hard-links
tracked regular files, and adds byte-identical .lkj aliases. nom has 106 tracked
files/65 aliases, DOM 86 tracked nodes/59 aliases including one internal symlink,
and Sōzu 196 tracked files/92 aliases. These aliases change paths, not source
expressions, module contents or test bodies.

Original .rs entries all refuse E_SOURCE_EXTENSION. Their .lkj entries then
expose syntax/library/design barriers. Twelve observations cover original and
projected paths for nom, DOM and four Sōzu workspace roots. The initial producer
was 60149069984ecfe491e322eba406238555c13acc90375c5468e5d5d4954a83ca.
After source units and open-ended ranges landed, the twelve were repeated with
9db99295e4810a4afc3b622b49cf7f38239915cf0d55d0928c34da3ffa44d7f0
on source d095ba7cd32a44bea7e7964767f02fa5195c3462.
The integrated cohort repeated them with fixed-point producer
2a76113394bb40ea5612965ed4567ca69f3f43555334bb6874b88588975cee3b
on source 8ddb023f804d511799cdf9b9f007e14f53f61639, then with
eb08fbaa3532b3255f2f8d5ff12a5052db1ec449b55f7c99c4251ce231dc938f
on source 20412b9a49a726e48273b0a07943bb77d1117058 after the bounded-unit integration.

Across those four historical baseline producers, corresponding commands, source-entry bytes, stdout, stderr, exit statuses
and signals agree across all four producers. Library inventories, projection
identities and before/after source verification also agree. The compact evidence
files identify every raw receipt and comparison; wall time and RSS are measured
but are not treated as identity fields. Sōzu's observer label changed from
sozu-workspace to sozu after the initial receipt. That is the only package-metadata
difference, not a source or diagnostic change; complete metadata objects are not
claimed identical.

This check is not a Cargo configuration loader. Test mode and feature predicates
differ from the upstream default-feature build; parsing can reject inactive
payloads before conditional elimination. Default-path candidates, test-only sites,
lexical inventory and error recovery are distinguished below. A stopped parser
or an empty admitted test set never counts as a passing program.

Initial projection refusal of DOM's tracked
dom-query-bench/test-pages -> ../test-pages was a tooling gap. The projector
now preserves safe internal relative tracked symlinks and refuses escaping,
dangling, cyclic, untracked or alias-only targets and gitlinks. Review also
corrected Git owner-execute interpretation and UTF-8 entry/path handling.
The required real-program gate runs 19 census, 69 projection and 68 probe
controls, including exact diagnostic bytes and non-ASCII roots. Sources must be
quiescent during projection; this is not a hostile concurrent-namespace snapshot.

### Ranked observed barriers

`tools/real-programs/rerank.pl` repeats the accounting from the complete probe
receipts and the reviewed site tables below. Run it once with identical `--before`
and `--after` inputs to reproduce the baseline, then against each new producer:

```sh
perl tools/real-programs/rerank.pl --baseline docs/real-programs-nom.json \
  --before OLD_PROBE.json --after NEW_PROBE.json \
  --projection PROJECTION/projection.json --source ORIGINAL_SOURCE \
  --output NEW_COMPARISON.json
```

Use the corresponding DOM and Sōzu tables and receipts for those packages. The
baseline table's `current_verification.receipt_sha256` pins the before receipt.
The tool verifies captured stream hashes, unchanged probe scope and commands,
projection identity, and complete original-file hashes for every diagnostic
anchor. Its columns follow the compiler's UTF-8 byte convention, distinct from
the census tool's Unicode-scalar columns.

Old causes carry forward only at the same unchanged source coordinate with the
same complete diagnostic. Recovery remains excluded only while its reviewed
parent diagnostic also survives. New or changed diagnostics remain unclassified;
an optional `--review` file assigns exact code/message/file-hash sites after source
review. A reviewed recovery row also supplies its complete parent diagnostic,
which must occur earlier in every root reporting the child. `--help` gives that
schema. The comparison separates an exact surviving
diagnostic, a different diagnostic at the old coordinate, and an old site no longer
observed. The last is not proof that the surrounding program admits or runs.
Counts re-rank the observed frontier; implementation priority is a separate
decision. The required offline gate runs 30 controls for this accounting,
including all three table schemas, repeated-root de-duplication, changed recovery
parents, source substitution and receipt corruption.

An offline reviewer needs the frozen baseline table, the complete before and
after probe JSON files, the frozen `projection.json`, the original source bytes
for their diagnostic anchors, and any explicit review JSON. The original source
can be restored elsewhere with `--source`; leave the captured paths and identities
inside receipts and the projection manifest unchanged. This comparison executes
no compiler and requires no build output. It recomputes the captured stream and
diagnosed source-file hashes and retains every input-file identity in its result.

Before removing a completed branch's build output, preserve raw probe receipts
byte-for-byte in the shared specimen evidence directory and record their retained
paths and hashes in the current summary. Keep the baseline classification and
explicit review inputs with the code; retain upstream originals and notices once.
The larger comparison JSON can then be regenerated from these inputs. A receipt
still under `target/` is a temporary candidate observation, not a durable evidence
link. Fresh observations additionally require the original/projected trees to pass
`project.pl --verify`; use `probe.pl --help` for its exact entry and package options.

The sixth note assigns ordinary `impl Trait`, qualified associated paths,
associated-type equality and the following ranked ordinary forms to Programs.
Macro tokenization/expansion and Debug formatting remain with the compiler stream.
Intentional ownership and authority refusals retain their distinct port cost.
These are observed frontier lower bounds, not all gaps that the complete dependency
graph will expose.

The [current equality implementation](real-programs-current.json), producer
`809c5052…0a32a9f`, repeats all twelve observations on unchanged upstream bytes.
Its ten nom and three DOM equality-binding sites are no longer observed. Four
qualified-path sites become independently visible in nom, so its primary frontier
changes from 149 to 143; DOM changes from 95 to 92. Sōzu keeps its 449 classified
sites while two previously unclassified parse rows disappear. No large root admits
and no nom own test has run under lkjscript. All three raw receipts are retained
in the shared specimen evidence directory at the paths and hashes in the current
summary. Their complete diagnostic streams and exact re-ranking payloads match
the source-reviewed candidate; the new receipt identities and measurements are recorded.
To reproduce this frontier, add `--review docs/real-programs-equality-review.json`
to the nom comparison. DOM and Sōzu need only their frozen baseline classifications.

The producer passes 29 focused cases across 87 route checks. Its refreshed seed
reproduces the fixed point; the full four-worker gate passes in 1,165.546 seconds
with 5,543 fresh case requests and 128 IR mutants, zero findings. The standalone
real-programs test also passes. [Retained proof records](real-programs-current.json)
identify the logs; publication CI is recorded with the landing report.

The tables give **observed unique sites** and physical lines in files containing
those sites. File coverage overlaps between rows and includes inactive code;
it must not be summed as independent blocked lines. Exact causal lines blocked
behind each refusal are unknown while earlier parsing stops admission. Selected
source upper bounds are nom 21,397, DOM root 7,773 and Sōzu 38,575. No fabricated
transitive blocked-line count is used.

#### nom: generic code

[Original site table](real-programs-nom.json) and [current frontier](real-programs-current.json):
the equality candidate has 445 raw rows, 143 unique primary sites, 79 recovery
artifacts and 223 E_BLOCKED rows. Of its primary sites, 139 are upstream default-path
candidates and four remain inactive (three documentation predicates and one test
macro). The baseline was 457 rows, 149 primary sites, 79 recovery rows and 229
E_BLOCKED rows. The ten original equality-binding sites disappear; four newly
exposed qualified paths explain why the net primary reduction is six.

| Rank | Cause | Sites | Affected file lines | Owner / next route |
| --- | --- | ---: | ---: | --- |
| 1 | Macro $ tokenization | 8 | 6,418 | Compiler / checked expansion producer |
| 2 | Returned impl Trait | 78 | 5,964 | Programs |
| 3 | Fully qualified associated paths | 45 | 4,360 | Programs; 38 type/where/nested paths and seven value calls |
| 4 | Const generics | 6 | 1,681 | Programs after the three leading ordinary forms |
| 5 | ?Sized bound syntax | 1 | 1,044 | Programs |
| 6 | Documentation/doctest cfg predicates | 3 | 460 | Language/configuration; inactive candidates |
| 7 | Unsafe blocks | 2 | 1,681 | Refused by design; checked indexing/ownership port |

All eight lexical errors are macro $ tokens. The [explicit source review](real-programs-equality-review.json)
classifies 51 changed recovery rows: forty where-clause messages after a rejected
`impl Trait`, eight reparsed qualified return types, two function-body constructors
mistaken for impl members, and one return reparsed after a qualified parameter
failure. Another 28 recovery rows survive unchanged. These do not become new
language causes. The four independent new sites are `src/bytes/mod.rs:941:3` and
`src/multi/mod.rs:982:16`, `996:13`, `1440:3`.
Generic associated types, heap-held views and raw offsets
occur in source inventory behind this frontier; inventory does not establish an
independently observed semantic refusal. The unsafe get_unchecked paths need
an explicit checked implementation or an identified foreign boundary; a hand
rewrite still costs the original gap.

#### DOM: parser, owned tree and selector views

[Original site table](real-programs-dom.json) and [current frontier](real-programs-current.json):
95 candidate primary messages reduce to 92 unique sites after three same-coordinate
impl-item recovery duplicates; 96 E_BLOCKED rows are excluded. There are 85
default-path candidates and seven test-only sites; diagnosed files still cover
5,375 physical lines in 16 files. The baseline was 95 unique sites and 99
E_BLOCKED rows. All three equality sites in `src/matcher.rs` disappear without
new independent or unclassified diagnostics.

| Rank | Cause | Sites | Affected file lines | Owner / next route |
| --- | --- | ---: | ---: | --- |
| 1 | ns! / local_name! expansion | 21 | 1,843 | Compiler / producer; 16 default candidates and five test-only |
| 2 | Debug ? formatting | 24 | 207 | Compiler formatting producer |
| 3 | const fn | 14 | 2,839 | Programs after the three leading ordinary forms |
| 4 | Fully qualified associated types | 9 | 556 | Programs |
| 5 | impl Trait arguments/results | 5 | 1,784 | Programs |
| 6 | Generic associated types | 2 | 844 | Language |
| 7 | Lifetime-outlives where predicate | 1 | 917 | Programs |
| 8 | Concrete-type where left-hand side | 1 | 867 | Programs |
| 9 | Box with dyn Iterator | 1 | 917 | Language |
| 10 | doc include_str! attribute | 1 | 45 | Language / producer |
| 11 | Expression-statement cfg | 1 | 510 | Language/configuration |
| 12 | rustfmt::skip; static CONTENTS | 1 + 1 | 692 each | Language; test-only |
| 13 | std::io imported as io::Result | 1 | 115 | Library data facade; not an ambient-I/O proof |
| 14 | Ref / RefCell / Cell | 9 | 4,073 | Refused by design; explicit ownership/tree port |

All 24 Debug-format refusals are direct producer refusals; the compiler formatting
parser unconditionally rejects that syntax. RefCell supports upstream's shared
mutable tree, and `Vec<NodeRef>` forms support heap-held views. The latter have
20 lexical occurrences on 19 lines, but no additional heap-view semantic-refusal
count is claimed before parsing succeeds. An eventual handle-based rewrite must
count both representation and API/test adaptation.

#### Sōzu: concurrent network service

[Original site table](real-programs-sozu.json) and [current frontier](real-programs-current.json):
four roots produce 484 unique located nonblocked rows in the equality candidate.
Of these, the same 449 have a classified cause; 35 parse/recovery rows stay
unclassified. Another 488 E_BLOCKED rows are excluded. The baseline had 486
nonblocked rows, including 37 unclassified, and 490 E_BLOCKED rows. The two removed
unclassified rows point to `Iterator<Item = (K, &'a V)>` at
`command/src/state.rs:1409:23` and `1435:27`; they are not retroactively added to
the original ranked equality-site count.

| Rank | Cause | Sites | Affected file lines | Owner / next route |
| --- | --- | ---: | ---: | --- |
| 1 | Macro expansion | 235 | 11,224 | Language / checked producer |
| 2 | Macro $ / @ DSL tokenization | 2 | 2,306 | Language |
| 3 | Debug formatting | 21 | 7,344 | Language formatting producer |
| 4 | dyn traits | 15 | 2,622 | Language |
| 5 | Associated constants | 6 | 148 | Language |
| 6 | Compile-time evaluation | 2 | 1,092 | Language |
| 7 | @ pattern | 1 | 167 | Language |
| 8 | async block | 1 | 68 | Language producer/runtime-future route |
| 9 | extern crate bindings | 4 | 320 | Package/language binding; not FFI |
| 10 | Generated source absent from original checkout | 1 | 257 | Package/build producer |
| 11 | Pure standard types/namespaces | 14 | 5,106 | Library and namespace routing |
| 12 | std::io Read/Write/self imports | 6 | 1,802 | Library facade; imports alone do not prove ambient calls |
| 13 | OS imports | 8 | 6,175 | Library adapters and explicit capabilities |
| 14 | Actual ambient calls | 86 | 7,562 | Refused by design; explicit authority |
| 15 | Unsafe/raw-pointer forms | 44 | 4,399 | Refused by design; ownership or foreign-process port |
| 16 | Arc | 1 | 68 | Refused by design |
| 17 | Same-process thread API | 2 | 209 | Refused by design / missing concurrency IR |
| — | Parse/recovery, not independently classified | 35 | 9,351 | No new gap assigned |

The 86 ambient calls are 82 print-family uses and one each of exit, read,
read_to_string and Instant::now. The unsafe group includes four unsafe function
trait/impl forms. The six associated constants are EMPTY, READABLE, WRITABLE,
ERROR, HUP and ALL in command/src/ready.rs. The original build script generates
command/src/proto/command.rs: isolated Rust output is 1,678 lines/68,847 bytes,
SHA-256 145d713a68f1ffb6b05de24ddfcae546a0e67a003fad306648022ba4c8accfdc.
That file is absent from the pristine original and is counted separately,
not silently supplied to the unmodified lkjscript check.

### Port cost and command adoption

| Work | Upstream source changes retained | Operation route / present result |
| --- | --- | --- |
| Levenshtein historical port | One range expression; two Rust-2015 integration-header lines | Checked add-item/module projection and harness operations; current gate restores original algorithm bytes after compiler range support |
| nom | Zero | Byte-preserving path projection only; no accepted large port |
| DOM core | Zero | Byte-preserving path projection only; no accepted large port |
| Sōzu | Zero | Byte-preserving path projection only; generated Rust oracle output kept separately; no accepted large port |

The cohort adds 216 path aliases and no mechanical semantic translation.
Existing edit requires its original root and candidate to check, so it cannot
publish a repair directly into these invalid roots. The first crate found a
route through a checked bootstrap root, an untrusted projection producer and an
actual checked edit transaction. Future translators must preserve source mapping,
count every omitted/rewritten form, and run the upstream tests. A hand rewrite
bypassing a refusal remains charged to it.

Fifteen independently reduced Rust-valid diagnostic fixtures retain specific
exposed forms. Their original 105 handwritten .lkj lines earn no operation credit.
The associated-equality reduction now checks; fourteen other reductions retain
located refusals. The optional Rust witness compiles those exact final files as
Rust metadata; the required diagnostic gate checks each current verdict. These
are compiler work items, not extra sites added to the root census.

### Own-test Rust oracle and measurements

[Baseline evidence](real-programs-rust.json) pins toolchains, locks, profiles,
available exact command lines, phase statuses, measurements and raw-output hashes.
nom uses cargo test -p nom; DOM uses -p dom_query; Sōzu uses --workspace.
No upstream source, tests or default features were edited. nom's workspace
benchmark packages are outside the selected-package oracle. nom had no tracked
lock: its once-resolved [lock file](../tests/fixtures/real_programs_nom.Cargo.lock)
is retained for replay. DOM's Wasm-only .cargo/config.toml is preserved and
hash-reviewed; its runner is inactive for these native x86_64 tests.

Each profile uses two Cargo jobs, one codegen unit and one test thread, with
incremental and LTO disabled. These are test-profile opt-level 0 and 3 builds,
with debuginfo=2, debug assertions and overflow checks enabled in both; they are
not the default release profile. Doctest snippets retain Cargo rustdoc defaults
with embed-bitcode=no/codegen-units=1, without opt-level or debuginfo flags; their
dependency libraries use the selected profile. nom and DOM use rustc 1.98.1;
Sōzu retains its original rustc 1.80.0 pin. Protoc 27.3 generated Sōzu's source;
its archive hash is a locally recorded digest, not a publisher signature.

| Specimen / opt level | Initial cargo test --no-run: s / max RSS KiB | Complete test driver: s / max RSS KiB | Own-test outcome |
| --- | ---: | ---: | --- |
| nom / 0 | 9.77 / 559,496 | 15.51 / 167,680 | 582 pass, zero fail/ignore |
| nom / 3 | 22.01 / 674,188 | 15.02 / 170,728 | 582 pass, zero fail/ignore |
| DOM / 0 | 14.39 / 592,084 | 5.26 / 206,948 | 314 pass, zero fail/ignore |
| DOM / 3 | 37.81 / 628,628 | 4.82 / 208,788 | 314 pass, zero fail/ignore |
| Sōzu / 0 | 96.59 / 1,532,260 | 47.66 / 1,623,832 | Initial broad run: 118 pass, one external-network failure |
| Sōzu / 3 | 215.61 / 2,012,400 | 117.71 / 2,205,724 | Initial broad run: 118 pass, one external-network failure |

nom's total is 208 library, 61 integration and 313 documentation tests; DOM's is
40 library, 254 integration and 20 documentation tests. Sōzu passes all 20
end-to-end cases in each initial broad run. Its socket::stats::test_rtt connects
to google.com:80 and fails DNS in the loopback-only namespace, so both broad
runs exit 101. The documentation continuation passes five tests with nine upstream
ignores (opt0: 15.47 s/1,374,888 KiB; opt3: 3.86 s/541,444 KiB).
The exact already-built RTT case then passes with an explicit host-network grant
(opt0: 0.58 s/7,320 KiB; opt3: 0.43 s/5,160 KiB). No Cargo or build script receives
that grant, and source/target views stay read-only there.
Thus **124 active Sōzu tests are successfully covered across separate runs in each
profile**, not in one all-pass suite. Original DNS and resolver-bind setup failures
remain in the evidence.

All build scripts and broad tests use fresh user/mount/PID/network/IPC/UTS
namespaces with loopback, no host home/credentials, read-only toolchains,
registry, system and protoc, private writable source overlays/targets/tmp,
dropped capabilities and no-new-privileges. This contains Rust oracle tooling;
it does not establish a lkjscript package authority guarantee.

These are one-shot shared-host measurements. One-minute load before/after
build then test is nom0 7.35→7.15 / 7.15→6.69; nom3 6.69→7.34 / 7.34→6.46;
DOM0 3.69→3.69 / 3.69→3.63; DOM3 3.63→3.62 / 3.62→3.81;
Sōzu0 4.10→7.25 / 6.39→3.64; Sōzu3 4.30→6.53 / 8.22→6.39.
Test-driver time includes Cargo verification, additional compilation and rustdoc
compilation. GNU time reports the maximum RSS of an individual measured process,
not the simultaneous aggregate. No large lkjscript execution exists yet
against which to calculate a ratio.

Disposable Rust build outputs (about 4.4 GiB Sōzu, 677 MiB nom and 1.2 GiB DOM)
were removed after replay evidence was retained. Shared sources, notices,
locked archives, toolchains and raw evidence used by this comparison remain once.

## Routes for intentional refusals

These are porting rules for the observed cohort. Actual occurrences appear
above with source sites and overlapping file coverage. They add no new
refusal sites to Levenshtein.

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

## Exact dependencies and authenticated source

The first dependency-bearing specimen is nom. Its normal memchr requirement
(^2.3) resolved to **memchr 2.8.3**, archive SHA-256
cf8baf1c55e62ffcace7a9f06f4bd9cd3f0c4beb022d3b367256b91b87513d98.
The ordinary-source [package module](../src/programs_package.lkj) implements
the least source contract for this edge: exact name, version, origin, revision,
content-tree digest and sorted exact dependency identities. This is not a Cargo
resolver, feature/target selector, transitive package graph or test-dependency
loader. nom's dev dependencies remain part of the Rust oracle's lock.

A versioned canonical manifest lists sorted regular-file paths, lengths and
SHA-256 digests, plus a nonempty subset of licence/notice paths. The tree digest
has its own domain. A separate trusted Expected value pins identity and dependency
identities. The local issuer authenticates the complete canonical manifest with
the existing sealing boundary and a distinct source-package protocol domain.
This proves local issuer integrity, not an upstream publisher's signature or
checked code admission.

The issuer key and trusted pin/issuance catalog are outside source grants and
the untrusted envelope store. The command accepts separate ReadDirectory
capabilities for that catalog and envelope store. Replacing a valid signed
envelope and placing a matching decoy pin beside it must not replace the caller's
trusted expectation. Package names do not grant authority. This deployment
separation is part of the consumer contract.

The trusted inventory producer verifies Git HEAD/index/blob bytes or exact
registry archive bytes. It is responsible for complete source inventory and
notice provenance; signing does not prove omitted files were inventoried.
The initial store profile accepts regular files only, rejecting symlinks and
gitlinks even though the observation projector preserves safe internal symlinks.
Direct inventory inputs open without blocking and must be regular files.
A source read opens once, checks type/length/hash against the authenticated
manifest and returns the **same owned bytes** it verified, not a pathname
certificate to be reopened. Reading at most pinned-length+1 bounds logical
input consumption, not the allocator's internal growth factor.

The envelope reader authenticates its fixed header before trusting the payload
length, then reads that authenticated length and one EOF probe. Truncated or
trailing payload data refuses before manifest construction. The final 24 native
controls require normal exit status 1 and exact refusal text; a watchdog or signal
never counts as a refusal. The inventory helper has 14 controls, including a FIFO
case whose deliberately blocking mutant fails the watchdog rather than earning
refusal credit.

The real round trip covers Levenshtein's ten original files, memchr's 59 archive
files and nom's 106 tracked files. Reading nom's pinned memchr edge returns
original memchr source bytes. All 175 listed original file lengths/hashes,
tree digests, notices, envelope digests and returned-buffer digests are
independently audited. The [operation and execution receipt](../tests/fixtures/programs_package.operation.json)
records each tested configuration and final source identities. The required gate
runs the 14 inventory controls, six pure contract tests on each execution engine,
and 24 native read/issue/substitution controls including catalog isolation.
This is source authentication; none of these large crates thereby becomes an
admitted lkjscript program.

The final source-store native phase passed in 1.20 s with peak process RSS
65,388 KiB. The subsequent private-catalog migration, three envelope reissues
and four real-source reads passed in 0.16 s/9,460 KiB. The six trusted metadata
files are fresh 0600, single-link inodes in a 0700 catalog outside source/store
grants; their old store names are removed. All envelopes and returned source
buffers remain byte-identical. These focused measurements use the d095 source
snapshot and 9db producer; they are not timings of the later integrated compiler.
The receipt preserves the original combined-layout limitation and exact inverse
source history, so earlier execution is never relabeled as testing final bytes.

## Concrete authority and compiler handoffs

[Concrete unit policy](../src/programs_unit_policy.lkj) consumes the landed
checked/authenticated unit contracts. Its private affine Admission owns the
exact CheckedUnit or an authenticated Interface. Its binding includes the trusted
caller's 32-byte producer identity, unit name, input, exact body, interface and
dependency **interface** identities. Consuming emit/open rechecks its owned
proof and current policy. An interface alone cannot emit a body; warm/native
reopening cannot substitute another interface, body, source base or source table.

The closed computation policy allows sixteen ordinary operations: array_repeat,
assert, box_new, cast, char_from_u32, clone, f64_from_bits, f64_to_bits, replace,
str_as_bytes, str_from_utf8, str_len, vec_len, vec_new, vec_pop and vec_push.
Its only public modification narrows that list; unknown operations refuse.
Concrete compiler effect closures include direct calls, specialized callbacks,
custom/structural Drop and unreachable IR. This consumer refuses file-open,
socket-connect, effectful callback and capability-destructor closures, while
a pure captured callback executes to 42.
The [27 controls](../tools/programs-unit-policy-test.sh) also exercise
cold/warm execution, changed policy, an exact body change with unchanged interface,
dependency/producer changes, correctly re-signed malformed native payloads,
source-base substitution and a synthetic unknown operation. This wrapper is
a required gate step.

This is an **opt-in policy for concrete projected interface export computations
after specialization**. It is not yet authority per Cargo package, all private
bodies, future generic instantiations or later invocation of returned callables.
Dependency bindings are interface identities, not recursive inventories of all
dependency bodies. The caller supplies the cold producer identity; the wrapper
hashes the actual compiler, but the API does not prove executable provenance.
Ordinary compiler CLI routes are unchanged. Enforcement assumes the checked
compiler/signing domain and sealed runtime; it is an operation policy rather
than operating-system process confinement.

Source authentication and this concrete policy are independent foundations.
Connecting them needs compiler-owned package principals: exact package/tree,
configuration and ceiling identity must follow declarations and imported generic
templates into every instance, including consumer-elaborated instances.
Current CheckedUnit bindings label such instances by consumer unit.
Source-unit roots mount as modules, not Cargo crates; crate:: paths can need
counted adaptations. Public interface projection omits unrelated private bodies,
so a package-wide promise needs a complete concrete-body aggregate plus generic
obligations. A pure dependency accepting an effectful host callback must refuse
that instantiation regardless of file/function spelling. Policy must be applied
on cold admission and authenticated reuse/native paths, not merely in a manifest
or historical query.

General declarative, procedural and derive macros and build-time generators remain
untrusted producers with explicit input/effect grants and complete output checked
through normal admission. Only existing fixed built-in syntax is implemented.
A checked expansion establishes admitted behavior of its output, not equivalence
to upstream expansion, hygiene or a passing upstream oracle. Sōzu's generated
protobuf source and selectors' build output make this a measured requirement.

The fourth-note narrow **runtime** Admission and semantic message identities with
generated channel codecs still await compiler contracts. The ordinary concrete
unit-policy Admission here does not implement that runtime contract.
Existing broad-World admission and byte channels are unchanged. A channel can
transfer a File or connected socket, so channel-only cannot mean unable to
perform I/O. Same-address-space threads and unchecked foreign code are not added.

## Integrated proof for this cohort

The required gate includes the three offline source-observation tools, the
original Levenshtein algorithm/oracles, all fifteen reduced diagnostic cases,
authenticated package-source controls and the concrete unit-policy consumer.
The optional Rust witness covers the small original crate and the fifteen exact
Rust-valid reductions; large upstream Rust baselines remain explicit contained
comparison runs.

The independent source-package audit credits 536/538 retained language lines
through 17 operations in five successful transactions. The concrete policy
credits 458/459 lines through 24 Programs operations. Its native Context
adaptation was one coherent checked transaction: four consumer-body operations
covered five API calls, alongside fourteen temporary operations importing exact
published SDK definitions. Both roots checked, with no authority delta. The SDK
was normalized to its pinned upstream bytes, rechecked, and its 27 controls passed
in 7.996 s/251,028 KiB. SDK integration earns zero retained Programs credit.

| Cohort language files | Retained changed lines | Operation-published lines | Successful Programs operations |
| --- | ---: | ---: | ---: |
| Authenticated source packages | 538 | 536 | 17 |
| Concrete unit policy | 459 | 458 | 24 |
| Fifteen reduced foreign diagnostics | 105 | 0 | 0 |
| **Cohort total** | **1,102** | **994 (90.20%)** | **41** |

The three retained scaffold lines and all 105 diagnostic lines are uncredited.
Another 19 embedded language-fixture lines in the policy shell wrapper are
separately uncredited. Failed transactions receive no credit. The policy API
adaptation changed nine added/five removed lines before this first cohort commit;
those intermediate changes are not counted twice as retained commit lines.
Including the published first route gives 1,407/1,543 changed language lines
(91.19%) and 62 successful Programs operations. The fourteen temporary SDK
operations remain separate, with no retained SDK delta after upstream integration.

The final four-worker integrated gate passes on source 20412b9a49a726e48273b0a07943bb77d1117058
with fixed-point producer eb08fbaa3532b3255f2f8d5ff12a5052db1ec449b55f7c99c4251ce231dc938f
(34,576,115 bytes). It takes 754.213 s at load
7.72→8.50 and peak individual-process RSS 1,296,976 KiB.
The gate reruns 5,485 case requests and reuses 0
authenticated exact cases; all host suites pass, and 128 IR mutants report zero
findings. The required package suite executes its
six pure tests on each engine, fourteen inventory controls and twenty-four native
I/O/substitution controls; all twenty-seven concrete policy controls pass.
The native-v2 object format needs no Programs source adaptation: the consumer
uses the published namespace/key/encoding APIs and checks exact EOF.

The [compact integrated proof](../tests/fixtures/real_programs_cohort.gate.json)
retains actual source/seed/producer identities, suite results, timing scope and
the twelve-observation repeat. It separately retains the successful 824.679 s
gate on 8ddb/2a761133 before the compiler integration, with 5,485 fresh requests,
128 mutants and peak process RSS 1,305,308 KiB. Earlier focused measurements stay
attached to their actual source and producer; they are not relabeled as this run.
These shared-host measurements do not establish a performance comparison between
the two compiler checkpoints.

Completed cohort build/store/query outputs and the private temporary query key
are removed after this proof and reconstructable operation history are retained.
Shared original sources, notices, locked archives, the authenticated source store,
private issuer and trusted catalog remain once. The published commit is also
checked by the independent gate and Rust-witness jobs of the
[check workflow](../.github/workflows/check.yml); GitHub retains those run results.
