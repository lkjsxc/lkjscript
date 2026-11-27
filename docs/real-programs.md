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

The granted lkjscript restorer is built by the compiler under test. The offline
gate uses Bash and base utilities to authenticate the exact reviewed projection,
manifest and license, reconstruct the two original Rust files, and verify their
upstream digests. It compares the complete provenance report, checksum list and
both importer requests with independently constructed expectations. It checks
the restored bytes before either lkjscript engine reads them and exercises the
restorer's refusal before publication. The optional Rust route uses its own Node
provenance verifier and checks the restored files before rustc reads them.

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
The required real-program gate runs 19 census, 81 projection and 68 probe
controls, including exact diagnostic bytes and non-ASCII roots. Sources must be
quiescent during projection; this is not a hostile concurrent-namespace snapshot.

### Ranked observed barriers

`tools/real-programs/rerank.mjs` repeats the accounting from the complete probe
receipts and the reviewed site tables below. Run it once with identical `--before`
and `--after` inputs to reproduce the baseline, then against each new producer:

```sh
node tools/real-programs/rerank.mjs --baseline docs/real-programs-nom.json \
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
decision. The required offline gate runs 38 controls for this accounting,
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
`node tools/real-programs/project.mjs --verify`; use `node tools/real-programs/probe.mjs --help` for its exact entry and package options.

#### Match-arm configuration

The replay from main `f6c1e77fe` gives nom 44 distinct primary sites and
41 blocked rows. The former statement-list macro site is already absent on
this producer. Its leading causes are match-arm `cfg` (16), higher-ranked
lifetime trait bounds (13), and the namespaced rustfmt attribute (6).

The match-arm change removes all 16 nom sites and their 16 blocked rows. The
same unmodified root now reports 28 primary sites and 25 blocked rows, with
no newly observed primary diagnostic. Higher-ranked lifetime trait bounds
(13), the rustfmt attribute (6), and function-pointer result types (4) now
lead nom's frontier. These are counts from observed diagnostics on one root.

For the DOM stack, the existing `dom_query` root gives 39 primary sites.
Separate probes on the pinned registry archives add the `html5ever` (90),
`markup5ever` (6), `selectors` (37), and `cssparser` (38) library roots.
The four archive counts are located nonblocked rows from isolated checks.
Their commands supply each root's own source bytes and a suffix projection.
All five DOM-stack roots and all four Sōzu roots retain byte-identical
diagnostic streams. Both producers checked ten roots with the original suffix
and the same ten through the byte-preserving projection.

Sōzu gives 960 classified primary sites across its four roots, with 34 prior
unclassified sites, 18 reviewed recovery diagnostics and 1,018 blocked rows.
The main replay newly reaches a literal dollar passed by `StateMachineBuilder!`
at `lib/src/lib.rs:520`. That macro-transcriber refusal remains in the candidate.
The top three causes in the six established roots are the same before and after
match-arm configuration.

| Cause | Sites | Projected roots |
| --- | ---: | --- |
| Macro calls without a visible declarative definition | 478 | Sōzu bin, lib, command and e2e |
| External prettytable declarative macro provider | 121 | Sōzu command |
| Ambient effect calls | 96 | Sōzu bin, lib, command and e2e |

The larger macro cohorts need package binding and authenticated providers.
Ambient calls require capability-aware ports. Borrowed formatting arguments
need a view and lifetime contract. Match-arm configuration removed the most
primary sites among the changes that fit one parser slice. The next narrow
candidate is trailing-point floating literals, with 13 located sites in cssparser.

#### Declarative macro binding review

Of the 478 reviewed sites, 472 request the Sōzu command crate's logging macros
and four request the Sōzu library crate's metrics macros. Both imports use
`#[macro_use] extern crate`. The baseline projected checks selected one source
root. The other two sites are `info!` in the command crate, whose local
provider file is blocked by lexing, and Rust's standard `unimplemented!` in the
e2e crate.

Admitting `@` as a token lets the command root reach the logging definitions.
All 19 affected logging calls still refuse at a repetition that starts with a
comma and precedes an optional trailing comma. The provider also reaches
unsupported `block` fragments and logger dependencies. The experiment exposed
additional diagnostic sites and yielded no admitted calls from the 478-site group.

#### Declared Sōzu metrics provider

Checking the pinned bin projection with
`--macro-provider sozu_lib:sozu=lib/src/lib.lkj` under its source directory
loads the unchanged Sōzu library root. With the selector omitted, the baseline
and candidate have byte-identical stdout and diagnostics and both refuse.

| Bin invocation | Selected export in `lib/src/metrics/mod.lkj` |
| --- | --- |
| `bin/src/command/server.lkj:613:9` | `gauge` at 332:14 |
| `bin/src/command/server.lkj:614:9` | `gauge` at 332:14 |
| `bin/src/command/server.lkj:615:9` | `gauge` at 332:14 |
| `bin/src/main.lkj:198:9` | `incr` at 313:14, then unqualified `count` at 302:14 |

All four calls expand to a reference to the defining crate's `metrics::METRICS`.
The selected replay removes just ten bin rows. These are four missing-binding
diagnostics, their four blocked statements and the import diagnostic with its
blocked item. The loaded provider contributes 530 diagnostic lines and 532
blocked lines, including `RefCell` at 6:18 and `thread_local!` at 22:1 in the
metrics module. The bin still refuses, with zero checked or executed metrics
calls. The logging calls retain their baseline diagnostic rows.

#### Declared Sōzu logging provider

Use `--macro-provider sozu_command_lib:sozu_command_lib=command/src/lib.lkj`
for the bin and `--macro-provider sozu_command_lib:sozu_command=command/src/lib.lkj`
for the lib. The pinned source is Sōzu 1.0.6 at
`c5ff06a12bd514e06ab28965052d0ad1e19c45b3`; the byte-preserving projection
verifies all 196 tracked files and 92 suffix aliases.

The token-tree/block slice matched the first provider rule at 470 of the
original 472 missing bindings. Its isolated nested replay stopped at
`module_path!` for 280 sites, including all three access-log calls; 190
stopped at `cfg(debug_assertions)` and two at unsafe argument syntax.

A fresh before/after isolated replay against `d138ee770` retains exactly the
same 115 bin and 357 lib identities. Both producers use the same instrumentation
around the ordinary expansion walker, with a separate source clone per authored
call. Call-site module paths now include the expansion destination through nested
provider calls and forwarded captures, using ordinary static-string lowering.

| Root | Replayed sites | module_path before → after | format_args after | file after | cfg(debug_assertions) | unsafe argument | Complete syntax after |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Bin | 115 | 68 → 0 | 68 | 0 | 45 | 2 | 0 |
| Lib | 357 | 212 → 0 | 204 | 5 | 145 | 0 | 3 |
| Total | 472 | 280 → 0 | 272 | 5 | 190 | 2 | 3 |

All three newly complete expansions are the original access-log calls:
`log_access!` at `lib/src/protocol/kawa_h1/mod.lkj:944:9` and
`lib/src/protocol/pipe.lkj:239:9`, and `info_access!` at
`lib/src/tcp.lkj:219:9`. The 190 configuration and two unsafe-argument
frontiers remain unchanged. All 75 existing consumer capability rows (20 bin,
55 lib) are byte-identical across both fresh audits and the frozen ordinary
baseline. Provider interior mutation, ambient streams and socket types retain
their refusals. Syntax completion grants no checking or execution authority;
this replay checks and executes zero original logging calls.

The candidate isolated replay takes 45.78 s for bin and 221.11 s for lib while
reaching the later syntax above. The supplementary ordinary unselected lib
check was stopped during diagnostic recovery after more than 267 s; it gives
no terminal verdict or fresh whole-root comparison. The exact-site audit and
capability-row comparison are complete. At that checkpoint, borrowed
`format_args!`, call-site `file!` and explicit debug-configuration inputs remained
distinct next slices.

#### Borrowed formatting in the pinned logging provider

A fresh replay from `c9608959b6cf2e2a60da0032dca539ffed8322a6` keeps the same
472 authored path/line/column/macro identities and unchanged provider selection.
Borrowed `format_args!` now uses checked stack environments and source formatting
traits. The provider additionally forwards `concat!` expressions through opaque
expression captures. An initial replay exposed that dependency as 272 literal
validation refusals; the completed slice folds the string/character literal
concatenations into ordinary static text before formatting normalization.

| Root | Sites | format_args before → after | file before → after | cfg(debug_assertions) | Unsafe argument | Complete syntax before → after |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Bin | 115 | 68 → 0 | 0 → 0 | 45 → 45 | 2 → 2 | 0 → 68 |
| Lib | 357 | 204 → 0 | 5 → 5 | 145 → 145 | 0 → 0 | 3 → 207 |
| Total | 472 | 272 → 0 | 5 → 5 | 190 → 190 | 2 → 2 | 3 → 275 |

Every former format_args first refusal now completes isolated recursive syntax
expansion. There are no new other first-refusal categories and no changed
non-target outcomes. All 75 original primary consumer capability rows (20 bin,
55 lib) remain byte-identical to the frozen ordinary baseline. Both runs verify
the same Sōzu pin, 196 tracked files and 92 suffix aliases. This is syntax evidence:
zero original logging calls are semantically checked or executed by this replay.

The before runs take 51.00 s for bin and 225.83 s for lib; the after runs take
66.64 s and 327.21 s while traversing farther syntax. These are host observations,
not a performance bound. The reproducible scripts, exact site transitions and
comparison are retained in the assigned format-args handoff report. At that checkpoint the remaining
frontiers were explicit debug configuration at 190 sites, file! at five, and two
unsafe argument forms; ordinary checking of the 275 syntax-complete calls remains
separate from this expansion count.

#### Explicit configuration and call-site source paths

A fresh baseline from `74a08b76f5fd44b59a8015f6d2cb4e398772f92c`
and the configured candidate retain the same Sōzu 1.0.6 pin,
`c5ff06a12bd514e06ab28965052d0ad1e19c45b3`, all 196 tracked files,
92 byte-preserving suffix aliases and 472 authored path/line/column/macro identities.
The candidate receives explicit `--debug-assertions off` and `on` inputs; no host
build flag or environment variable selects the mode. Call-site `file!` preserves
the expansion destination and captured source location. All five file sites also
use `line!`, so that ordinary source intrinsic is included in this slice.

| Root | Sites | file before / off / on | cfg before / off / on | Unsafe before / off / on | Complete syntax before / off / on |
| --- | ---: | ---: | ---: | ---: | ---: |
| Bin | 115 | 0 / 0 / 0 | 45 / 0 / 0 | 2 / 2 / 2 | 68 / 113 / 113 |
| Lib | 357 | 5 / 0 / 0 | 145 / 0 / 0 | 0 / 0 / 0 | 207 / 357 / 357 |
| Total | 472 | 5 / 0 / 0 | 190 / 0 / 0 | 2 / 2 / 2 | 275 / 470 / 470 |

All 195 targeted sites now complete isolated recursive syntax expansion in both
modes. There are no other first-refusal categories or non-target frontier changes.
All 75 original primary consumer capability rows (20 bin, 55 lib) remain
byte-identical. The two unsafe argument forms remain unchanged. This is syntax
evidence: zero original logging calls are semantically checked or executed.
Ordinary checking of the 470 complete expansions is the next separate step.

The logging provider requires cfg attributes on ordinary statements as well as
items. Disabled statements retain syntax validation without contributing bindings,
source loads or macro expansions. Removing them, including through empty macro
splices, preserves Rust's block-tail and semicolon rules. Source configuration,
file paths and line numbers lower to ordinary boolean, text and integer IR values;
no new trusted IR operation or capability exemption is introduced.

The before bin/lib runs take 67.73 / 360.56 s; off takes 62.57 / 333.15 s;
on takes 97.19 / 588.82 s. These are host observations while traversing different
selected bodies, not a performance bound. The exact comparison, source snapshots,
per-site transition table and reproducible scripts are retained in the assigned
cfg/file handoff report. Its private audit adapter follows the changed internal
statement-expansion signature; the authored-site protocol is unchanged. The
incomplete first candidate exposed the statement requirement and is retained as
nonacceptance evidence alongside the completed replay.

A later semantic-test fixture correction reproduced both the compiler and its
private audit byte-for-byte. The retained execution artifacts remain bound to
those exact binaries; the corrected source snapshot and unchanged log hashes
are recorded explicitly in the handoff report.


#### Item diagnostics in refused real-program roots

Ordinary whole-root checks on the pinned Sōzu inputs stop during source loading,
before resolving or checking the 470 syntax-complete logging expansions. The
earlier private syntax audit does not preserve all consumer expression/statement
edges and cannot supply a semantic-check substitute. The agreed next slice is
therefore the first conservative stage of item diagnostics after source refusal,
with exact recovery ownership and dependency accounting.

The initial implementation made `check` emit a non-authorizing `LKJSCRIPT_ITEMS`
report. Refused regions retain tombstones, and uncertain names, imports, macros
and coherence inputs poison affected lookups. Eligible nongeneric free functions
and their complete direct-call closures are reparsed from the exact loaded source
bytes. Their recorded bindings must agree with the original declaration context
before ordinary resolution, elaboration and the independent IR checker run.
Methods, nominal/generic contexts, macro-generated functions and incomplete
declaration environments remain blocked. Structural failures cannot become local
type/borrow diagnoses merely because the smaller replay omitted library inputs.
The [language contract](language.md#item-diagnostics-after-a-source-refusal)
defines the bounded population and the checked/blocked/rejected statuses.
No partial fact publishes checked IR or an executable, changes capability
requirements, or admits a checked-edit transaction.

The ranked whole-root barriers below account for the same 470 syntax-complete
logging sites in either debug mode. They are the populations behind the first
source refusal in each root, **not independently established local log-call
diagnoses**. Type, borrow, trait/method and missing-library classifications for
these calls remain unproved.

| Rank | Root-first source barrier | Complete logging sites behind it |
| --- | --- | ---: |
| 1 | Lib: `E_CAPABILITY`, unavailable `std::cell::RefCell`, `lib/src/lib.lkj:340:18` | 357 |
| 2 | Bin: `E_UNSUPPORTED`, `extern`, `bin/src/main.lkj:27:1` | 113 |

The completed paired census uses ordinary production CLI executables built from
the accepted and candidate compiler sources by the Rust witness. These are the
normal source loader and checking paths, without a private audit adapter or an
erased refusal gate. Both debug configurations give the following result:

| Root | Before checked | After checked | After blocked | After rejected |
| --- | ---: | ---: | ---: | ---: |
| Sōzu bin | 0 | 0 | 607 | 0 |
| Sōzu lib | 0 | 0 | 1,234 | 0 |
| nom 8.0.0 | 0 | 0 | 659 | 0 |
| selectors 0.38.0 | 0 | 0 | 348 | 0 |

The baseline emits no per-item report, so its blocked/rejected totals are
unavailable; zero before checked means no function IR reached ordinary checking.
After totals inventory retained non-library functions with bodies. Sōzu bin
contains 176 consumer and 431 declared command-provider functions; lib contains
803 consumer and the same 431 provider functions. These overlapping provider
populations must not be summed as distinct functions.

Every Sōzu item is blocked by an incomplete recovered declaration environment:
the ordinary declaration catalog refuses `./command/src/proto/mod.lkj:10:5`
with `E_UNSUPPORTED: external modules require a source bundle or the normal CLI`.
This guard records an upstream catalog limitation, not a local logging failure.
nom has 657 poisoned lookups and two functions overlapping owned source refusals;
its root-first source refusal is unsupported `cfg(doctest)` at
`src/lib.lkj:385:1`. Selectors has 177 poisoned lookups and 171 functions
overlapping refused regions. Its source stream first refuses unsafe code at
`tree.lkj:19:1`; item poison also retains the higher-ranked trait application
refusal at `parser.lkj:275:6`. Every blocked row records an exact printed
upstream cause.

All 472 frozen logging identities map to retained enclosing-function records.
All 470 previously syntax-complete contexts remain unchecked in both modes;
there is consequently no admitted original call to execute. The two unsafe
arguments at `bin/src/worker.lkj:194:26` and
`bin/src/upgrade.lkj:86:26` remain separate and exact. All 75 original capability
rows (20 bin, 55 lib) retain their bytes, with no additions, omissions or duplicate
rows in either mode.

The Sōzu pin remains `c5ff06a12bd514e06ab28965052d0ad1e19c45b3`, with
196 original tracked files and 92 byte-preserving suffix aliases. nom is pinned
to `2cec1b3e4c9ccac62c902d60c00de6d1549ccbe1`. The selectors archive digest is
`8adfa1c298912827b8a28b223b3b874357397ae706e6190acd9bf28cee99114d`;
its projection retains 22 archive files and adds 16 suffix aliases. Input and
producer hashes are checked before and after each completed census batch.

The native Sōzu and nom baseline attempts were stopped unfinished after slow
macro-error recovery and are retained as observations, not completed census
results. Native selectors baseline diagnostics agree with the Rust-built route
in both modes, and the candidate native selectors item JSON is byte-identical to
the Rust-built route in both modes. No cross-producer timing comparison is
claimed. Full native acceptance remains separate from this paired census.

The focused suite checks 112 authored item rows across both debug modes:
34 checked, 76 blocked and two rejected. It preserves valid IR byte identity,
passes eight native/interpreter source-and-IR value routes and 60 execution or
publication refusals, and runs Rust opt0/opt3 controls covering eight value
observations and 56 context/boundary verdicts. Twelve accepted Rust context
executions and four unsafe-shadowing value observations exercise intentional
lkjscript boundaries; these are not claims of identical admission decisions. Regressions include mutable/qualified headers,
intermediate and normalized-`self` imports, transitive import poison, structural
library inputs and exact dependency causes. Standalone bootstrap passes in
186.965 s and reproduces native SHA-256
`1deb2da75ea8280f9f15b529a156ff4db8bfcdb874353c701c9878236c054e98`.
Fresh focused acceptance passes in 23.755 s. Standalone origin controls pass all
87 malformed macro origins on both engines and 18 configuration origins per
engine. The complete fresh-store four-worker gate passes in 2,197.217 s
(2,197.219 s shell wall time), rerunning all 7,535 case requests with none skipped.
All 128 IR mutants produce zero findings. Bootstrap inside that gate passes in
184.865 s and reproduces the same native image; its partial-check suite passes in
18.178 s. Each ordinary/inline64 conformance mode passes 1,832 cases / 3,662
route checks, with the same two pending routes for the borrowed-closure-escape
fixture. Each generated mode passes 805 cases / 2,430 routes with no pending.

Integration repairs updated four reduced diagnostic harnesses, made the store
fixture compare exact fresh/cached build refusals while preserving destinations,
and registered the new module in the independent runtime-admission entry point.
That one-line entry-point repair is the only compiler-source change after the
qualified census; rebuilding the primary compiler from the final source
reproduces the same native bytes in 9.371 s. The two stopped full-gate attempts
and their failures remain recorded separately; the final complete run used a
fresh store and a new final-source input snapshot.

#### Expanded independent item closures

The partial declaration catalog keeps unloaded module names and their exact
loader causes. A refused attribute or import carries the written declaration or
alias when the parser can establish it. The replay requires a unique source
owner for each selected module and gives a named lexical refusal priority over
conservative poison from other scopes.

The fixture cohort adds literal constants, rigid generic definitions, explicit
library calls, an empty source trait bound, a complete inherent implementation
and a local expression macro whose producer, rule and invocation tokens are
replayed. Negative controls change a library body, a macro template and the
inherent implementation inventory, and introduce ambiguous module ownership.
The original capability and module-loading refusals remain authoritative.

The ordinary Rust-built CLI checked the same pinned roots in both debug modes
against `80278ca3f`. Both modes produced the same counts.

| Root | Before checked / blocked / rejected | After checked / blocked / rejected |
| --- | ---: | ---: |
| Sōzu bin | 0 / 607 / 0 | 8 / 599 / 0 |
| Sōzu lib | 0 / 1,234 / 0 | 9 / 1,225 / 0 |
| nom 8.0.0 | 0 / 659 / 0 | 8 / 651 / 0 |
| selectors 0.38.0 | 0 / 348 / 0 | 0 / 348 / 0 |

Eight Sōzu checks cover the same scalar configuration defaults in the command
provider loaded by both roots. The library root also checks
`protocol::proxy_protocol::header::u16_to_array_of_u8`. The source checks all
return status 1. The census maps all 470 syntax-complete logging sites to
blocked items and matches the original capability, missing-module and unsafe
rows byte for byte. Native and Rust-built selectors item reports agree in both
debug modes.

#### Macro recovery on the full Sōzu roots

Recovery walks each source closure once. Matcher and parser rollback undo a
failed invocation's writes and discard appended syntax from the end of private
arenas. Completed parents and siblings keep their origin records.

The paired Rust-built and candidate native wall times cover the pinned full
roots. The ordered diagnostics, blocked extents, recovery owners and complete
recovered programs agree, including all retained macro origins.

| Root and debug mode | Invocation attempts before → after | Retained expansions | Rust-built wall before → after | Candidate native wall |
| --- | ---: | ---: | ---: | ---: |
| Sōzu bin off | 93,925 → 893 | 762 | 37.96 → 0.35 s | 1.88 s |
| Sōzu bin on | 161,175 → 1,447 | 1,316 | 100.745 → 0.39 s | 2.24 s |
| Sōzu lib off | 346,763 → 2,796 | 2,651 | 309.853 → 0.77 s | 3.77 s |
| Sōzu lib on | 566,380 → 4,483 | 4,338 | 975.513 → 1.11 s | 5.05 s |

All four source checks return status 1. Their native diagnostic streams,
including item reports, match the published baseline bytes after the optional
timing and work counters are excluded.
The native bin-off baseline was still running when the measurement stopped
after 2,435 s. The completed paired timings in the table use the Rust-built
producers.

#### Borrowed receiver and trait evidence

The receiver fixture retains Sōzu's `Ready` tuple declaration and
`is_empty` method, a named-field buffer capacity reduction, two applications
of a source trait, a second trait with the same short name, and the buffer
destructor. Checked item records advance from one to seven in each debug mode.
The corresponding valid source evaluates to 42 on both lkjscript engines and at Rust opt0 and
opt3. An unresolved import and an ambient-output refusal keep the reduced root
in item-reporting mode.

A duplicate or blanket receiver for the `u16` application blocks that
application. The `u8` application and the separately declared trait can still
check. A malformed trait qualifier leaves the trait identity unproved for
that receiver. A competing receiver through a refused alias keeps the exact
import cause. A refused destructor blocks the buffer method through its
recorded dependency. Whole-source execution and publication still refuse.

The pinned bin and lib roots retain eight and nine checked items respectively in
both modes. Their original diagnostic text and checked item records match the
preceding producer byte for byte. Complete imports remain namespace
dependencies. In the library root, 346 blocked method records now cite the
refused `dyn` region that leaves coherence open.

#### Retained inherent implementation headers

The Sōzu readiness reduction retains the exact `Ready` declaration and
inherent implementation, including its associated constants and the eight
method bodies, then appends an ambient-output refusal. In both modes the
reduction advances from zero checked items to one, `Ready::is_empty` at the
original line 17. The other seven methods keep their unresolved body inputs.
The full Sōzu and nom roots keep their existing checked sets while unrelated
opaque implementation producers remain in their source closures.

The independent fixture checks one borrowed scalar method across a refused
sibling. Its valid twin agrees on 42 across both lkjscript engines and Rust.
The semantic-only control enters the report through ordinary checking,
retains the unresolved-name diagnostic, and still checks the sibling method.
A repeated member name or refused member header blocks the affected owner.

#### Earlier ordinary numeric traits frontier

<!-- numeric-evidence-v1 sha256:53c52136581be3be25a96ff0bfa4e7077207da1e8f75548cc3aaa456e746a484 -->

The actual source checkpoint, independent source review, completed unchanged-source census and review, and current full gate are bound by the canonical evidence digest above. Bootstrap starts from published core128 native fd83ea00de8b306e44534b50bdee0da47464e2c096a36c2567e2699a2cb1ec86 and seed 4bbd931edf8422044cb61acfc6c3c4fba580a0d61e7283474ba624b9696f90a3. The initial and refreshed logs record no automatic seed bridge. First and reproduced numeric generations are byte-identical; final acceptance accounting is bits: reused exact first-producer evidence, rhs: reused exact first-producer evidence, range: reused exact first-producer evidence. Core34/strict76 and parent CI are retained history, not fresh numeric proof.

The numeric form is limited to the exact ordinary source operator and conversion contracts proved by its authenticated numeric source checkpoint. Its declared RHS applications, selected implementation identities, provider behavior, focused cases, retained boundaries and query/unit evidence remain bound to that final producer and source library.

The focused numeric population passes 37 cases / 74 engine routes. Rust covers 13 values/output cases and 24 verdicts: 20 matching refusals and four Rust-accepted LKJ boundaries. The unchanged prior population passes 200 cases / 400 routes. First bits and RHS checks are one case / two routes each; the four range-repair cases are eight routes. The closed source proof contains 200 core and 536 source cases (736 total). Numeric query/edit evidence and retained core/defaults/origins proofs are separate; only the explicit core-unit artifacts establish cold/warm unit behavior.

Twelve unchanged-source observations across six projected roots give nom 45 -> 45 classified, 0 prior-unclassified, 0 recovery; dom 39 -> 39 classified, 0 prior-unclassified, 0 recovery; sozu 968 -> 968 classified, 34 prior-unclassified, 18 recovery. The actual changed diagnostic-stream count is 0; new unclassified identities are zero. Original source/projection bytes are unchanged. These refusals do not admit a whole foreign root or execute nom's own tests.

| Specimen | Rank | Observed cause | Sites |
| --- | ---: | --- | ---: |
| nom | 1 | Configuration attributes on match arms | 16 |
| nom | 2 | Higher-ranked lifetime trait bounds | 13 |
| nom | 3 | Namespaced rustfmt attribute | 6 |
| nom | 4 | Function-pointer result types | 4 |
| nom | 5 | Documentation cfg predicates | 3 |
| nom | 6 | Unsafe block surface | 2 |
| nom | 7 | Statement-list macro expansion | 1 |
| dom | 1 | Dependency-provided atom macros in expressions and patterns | 24 |
| dom | 2 | Shared interior mutation and borrow guards | 9 |
| dom | 3 | Macro-valued documentation attribute | 1 |
| dom | 4 | Boxed dyn Iterator | 1 |
| dom | 5 | Namespaced rustfmt attribute | 1 |
| dom | 6 | Attribute on an expression statement | 1 |
| dom | 7 | Immutable static test data | 1 |
| dom | 8 | std::io data/type facade | 1 |
| sozu | 1 | Macro calls without a visible declarative definition | 478 |
| sozu | 2 | External prettytable declarative macro provider | 121 |
| sozu | 3 | ambient_effect_calls | 96 |
| sozu | 4 | unsafe_or_raw_pointer | 65 |
| sozu | 5 | trait_object_dispatch | 44 |
| sozu | 6 | pure_standard_data_types | 38 |
| sozu | 7 | Borrowed formatting-arguments macro surface | 20 |
| sozu | 8 | RefCell-based interior mutation | 18 |
| sozu | 9 | Error-derive helper attributes on enum variants | 12 |
| sozu | 10 | Standard unimplemented macro surface | 10 |
| sozu | 11 | standard_io_traits | 9 |
| sozu | 12 | Source-information and token-string macros | 8 |
| sozu | 13 | standard_os_imports | 8 |
| sozu | 14 | associated_constants | 6 |
| sozu | 15 | external_crate_binding | 5 |
| sozu | 16 | Thread-local declaration macro surface | 4 |
| sozu | 17 | Namespaced source attributes | 3 |
| sozu | 18 | Target-environment cfg predicates | 3 |
| sozu | 19 | Compile-time environment macro input | 2 |
| sozu | 20 | Global static state | 2 |
| sozu | 21 | same_process_thread_api | 2 |
| sozu | 22 | Standard todo macro surface | 2 |
| sozu | 23 | async_block | 1 |
| sozu | 24 | at_pattern_lexing | 1 |
| sozu | 25 | build_generated_source | 1 |
| sozu | 26 | Configuration expression macro surface | 1 |
| sozu | 27 | Unsupported declarative fragment kind | 1 |
| sozu | 28 | Declarative macro statement sequences | 1 |
| sozu | 29 | Type fragments in declarative macro matchers | 1 |
| sozu | 30 | External nix ancillary-data macro provider | 1 |
| sozu | 31 | macro_token_lexing | 1 |
| sozu | 32 | Reserved gen spelling used as a callable identifier | 1 |
| sozu | 33 | shared_heap_ownership | 1 |
| sozu | 34 | Tuple destructuring in a function parameter | 1 |

The [numeric proof](../bench/numeric-traits.md) records actual gate and standalone outcomes. Exact reviewed identities are retained for [nom](https://github.com/lkjsxc/lkjscript/blob/evidence-archive-20261120/docs/real-programs-numeric-traits-nom-review.json), [DOM](https://github.com/lkjsxc/lkjscript/blob/evidence-archive-20261120/docs/real-programs-numeric-traits-dom-review.json), and [Sozu](https://github.com/lkjsxc/lkjscript/blob/evidence-archive-20261120/docs/real-programs-numeric-traits-sozu-review.json). The [previous frontier](https://github.com/lkjsxc/lkjscript/blob/evidence-archive-20261120/bench/numeric-traits-data/previous-frontier.json) and its published core proof remain immutable history. Disappearing diagnostics alone do not establish source admission.

#### Earlier lifetime-where frontier

The [preceding lifetime-where implementation](https://github.com/lkjsxc/lkjscript/blob/evidence-archive-20261120/bench/positional-traits-data/previous-frontier.json), producer
`59c7df4c…749a0836a`, adds named lifetime-outlives where predicates on free
functions and inherent methods to the earlier forms, within the
[language limits](language.md). A predicate such as `where 'long: 'short`
feeds the existing directed region obligations. Both sides must resolve to
declared names in the proper scope, including an enclosing inherent impl.
The relationship applies during unused-body checking and at calls. Queries
preserve the written edge and exact declaration references; checked body edits
and function renames retain the contract and refused edits roll back.

Ordinary results retain mutable-storage invariance. An opaque result lifetime
requires its hidden references to outlive that lifetime without equating their
stored lifetimes to it. The controls combine these rules with const-qualified
functions, const usize parameters and borrowed `T: ?Sized` referents.
Trait return origins still require exact lifetime-label agreement. Paired
trait methods must have identical directed inline edges after method-local
binders are paired by position; adding or dropping an edge refuses. Edges
involving an enclosing lifetime remain refused pending owner substitution.
Trait-method and implementation-header where predicates, anonymous/static
lifetime predicates, higher-ranked predicates and type-outlives where predicates
such as `T: 'a` remain located boundaries. No checked-IR, independent-checker,
library or runtime rule changes.

All twelve observations repeat on unchanged upstream bytes. **All 146 original
impl-Trait/qualified/equality target diagnostic identities remain absent**: 129
nom sites and 17 DOM sites. The primary frontier is nom **149→17** and DOM
**95→63**, for a combined **244→80**. Relative to published Sized, nom changes
**15→17** and DOM **64→63**. The net increase from 79 to 80 records two newly
explicit type-outlives where refusals and one removed named-lifetime predicate.
Sōzu remains at **448 classified sites and 34 prior-unclassified rows**.
No large root or real nom module tree has been admitted, and no nom own test
has run under lkjscript.

| Stage | nom primary sites | DOM primary sites | Original target diagnostics absent | Semantic scope established |
| --- | ---: | ---: | ---: | --- |
| Original baseline | 149 | 95 | 0 | Original compiler |
| Associated equality | 143 | 92 | 13 | Equality bindings |
| Qualified paths | 100 | 83 | 63 | Equality and qualified type/value paths |
| Argument impl Trait | 24 | 78 | 146 | Arguments; only 65 original target source forms supported, results still refused |
| Result impl Trait | 24 | 78 | 146 | All three form categories, in their documented subset |
| Const functions | 24 | 64 | 146 | Bounded const-qualified declarations and runtime calls; CTFE remains separate |
| Const generics | 18 | 64 | 146 | Function/receiver-impl const usize parameters; nominal const parameters and global constant array lengths remain separate |
| Sized bounds | 15 | 64 | 146 | Named relaxed parameters and existing borrowed representations; owned unsized storage remains separate |
| Lifetime where predicates | 17 | 63 | 146 | Named free/inherent lifetime edges; two type-outlives where boundaries newly explicit |
| Positional trait arguments | 17 | 63 | 146 | Exact applied type contracts and input/result inference for concrete tuple methods; later barriers still mask the original roots |
| Type-parameter associated families | 17 | 61 | 146 | Exact family contracts and delegated parser inference; two lifetime-family parse refusals vanish behind later semantic boundaries |

The APIT/RPIT history remains explicit: APIT consumed opaque result syntax
while other parse errors masked its unsupported-result resolver. Dedicated
semantic and query/edit controls later established RPIT support. Const-header
and relaxed-bound diagnostic removals likewise did not admit their original
positional `Parser<I>`, Fn/Borrow or GAT contracts. The six nom const-header
removals retain positional-trait and receiver-occurrence limits. In this
historical cohort, parser and macro errors masked the later `MAX_FDS_OUT`
array length refusal. Sized, const-function and opaque-result restrictions remain
as documented.

The summary pins durable raw receipts, exact re-ranking payloads and reviewed
original-source hashes. Reproduce the results with the
[nom review](https://github.com/lkjsxc/lkjscript/blob/evidence-archive-20261120/docs/real-programs-lifetime-where-nom-review.json),
[DOM review](https://github.com/lkjsxc/lkjscript/blob/evidence-archive-20261120/docs/real-programs-lifetime-where-dom-review.json), and
[Sōzu review](https://github.com/lkjsxc/lkjscript/blob/evidence-archive-20261120/docs/real-programs-lifetime-where-sozu-review.json) passed to
`node tools/real-programs/rerank.mjs --review`. Ten complete exit/status/stdout/stderr identities are
byte-identical to published Sized; only the nom and DOM projections change.
Original-suffix observations, source/projection hashes and the literal library
inventory remain unchanged. There are no new unclassified diagnostics or
current recovery exclusions after exact source review.

The DOM `Selection::select` predicate at `selection.rs:569:9` relates its
enclosing impl lifetime to a method lifetime and now parses. Its disappearance
establishes no surrounding-method or root admission. The nom predicates at
`bytes/complete.rs:424:46` and `multi/mod.rs:964:29` instead constrain types
`I` and `F` to outlive a lifetime. The parser now explicitly refuses those
contracts before folding type-parameter bounds. Previously those generic
lifetime entries were skipped; concrete type-target predicates already had a
later refusal. These are independent type-outlives where primaries, not recovery
from an earlier opaque return and not evidence of prior root admission.

The focused suite passes **90 cases / 270 route checks**: 180 final-native
engine checks and 90 separate Rust checks, comprising 37 values/output cases
and 53 verdicts. The population is 46 lifetime-family cases plus 12 APIT and
32 RPIT regressions. Source checking and all four query/edit/cache suites pass.
The unchanged associated-equality refusal and option/result-view control each
pass separately on two engine routes and their Rust oracle; both are outside
the 90 cases. The owner-substitution refusal also has its exact located
diagnostic and a direct Rust library-metadata acceptance oracle. This repeated
boundary check is not an additional focused case. Prior and overlapping
hosted populations are retained as history, without aggregation.

The native self-build reaches a byte-identical fixed point and passes library
checking with the refreshed seed. The first unpublished lifetime bootstrap
failed on `prelude.lkj:148:9`: duplicate result-scope proof retained both
exclusive enum alternatives despite the existing returned-view contract.
The frontend repair skips only surface-result obligations already covered at
the exact projection and region. Nested and invariant regions, and unrelated
named/elided result leaves, still require their scoped proof. The library and
independent checker are unchanged. The failed producer `ff6669ff…`, original
logs, source patch and Rust-hosted repair proof remain in the validation record;
the canonical proof above is fresh on final native `59c7df4c…`.
The full four-worker gate passes in 721.195 seconds with 6175 fresh case
requests and 128 IR mutants, zero findings. The standalone real-programs
validation also passes. Publication CI is recorded with the landing report. [Historical proof records](https://github.com/lkjsxc/lkjscript/blob/evidence-archive-20261120/bench/positional-traits-data/previous-frontier.json)
pin completed checks and root-owned validation paths.

The original fifteen foreign reductions retain **six checking and nine
refusing**, with no fixture changed or newly promoted in this stage. The
handwritten Holder/opaque/qualified/equality reductions retain their recorded
origin and do not count as upstream module admissions. The independently
admitted session literal-repeat and Poll default-method diagnostics remain
outside those fifteen; their earlier unchanged-body proofs are preserved.

Published Sized passed its full gate in 750.411 seconds with 6083 fresh
requests, zero reuse and 128 mutants, zero findings, plus standalone validation.
Its first failed gate and diagnostic-precedence/vector-advice repair remain
recorded, along with the separate Poll admission. The earlier const-generics
expectation/argument-placement failure and const-function fingerprint failure
also retain their repairs and later successful gates. These historical results
are separate from that lifetime gate.

The tables give **observed unique sites** and physical lines in files containing
those sites. File coverage overlaps between rows and includes inactive code;
it must not be summed as independent blocked lines. Exact causal lines blocked
behind each refusal are unknown while earlier parsing stops admission. Selected
source upper bounds are nom 21,397, DOM root 7,773 and Sōzu 38,575. No fabricated
transitive blocked-line count is used.

#### nom: generic code

[Original site table](real-programs-nom.json) and [current frontier](real-programs-current.json):
the current producer emits 35 raw rows: 17 unique primary sites and 18 E_BLOCKED
rows, with no current recovery exclusions. There are 13 upstream default-path
candidates and four inactive sites. The original baseline had 149 primary sites;
published Sized had 15. Two real type-outlives where bounds are now refused explicitly:
`I: Clone + crate::traits::Offset + Input + 'a` in escaped at
`src/bytes/complete.rs:424:46`, and `F: Parser<I, Error = E> + 'a` in fill
at `src/multi/mod.rs:964:29`. These are type-to-lifetime requirements, distinct
from the supported named lifetime-to-lifetime predicates. Both original functions
also retain positional generic-trait and core parser dependencies.

The three previous relaxed-bound diagnostics at `combinator/mod.rs:503:7`,
`513:29` and `524:7` remain absent. The six const-header diagnostics remain
absent too. Two AsBytes array receivers fit that declaration subset; two Compare
implementations place N only in a positional trait argument, and two FindToken
implementations still need positional generic-trait support. No surrounding
implementation or module admission follows from those header removals.

| Rank | Cause | Sites | Affected file lines | Owner / next route |
| --- | --- | ---: | ---: | --- |
| 1 | Macro $ tokenization | 8 | 6,418 | Compiler / checked expansion producer |
| 2 | error_position! expansion | 2 | 365 | Compiler / checked source producer |
| 3 | Type-outlives where predicates | 2 | 2,436 | Programs; explicit boundary for generic type lifetime bounds |
| 4 | Documentation/doctest cfg predicates | 3 | 460 | Language/configuration; inactive candidates |
| 5 | Unsafe blocks | 2 | 1,681 | Refused by design; checked indexing/ownership port |

All eight lexical errors are macro $ tokens. The two reviewed macro calls remain
the original `error_position!` branches in `bits::complete::tag` at 100:24 and
`bits::streaming::tag` at 76:24. The two new lifetime-bound diagnostics are also
independent primaries: the parser now rejects the actual type-target bound at
its exact span before generic-bound handling. No current recovery exclusion
is inferred from an earlier failure elsewhere in the root. Historical recovery
classifications and the original baseline are preserved unchanged.

Generic associated types, heap-held views and raw offsets remain in source
inventory behind this frontier; inventory is not an independently observed
semantic-refusal count. Unsafe get_unchecked paths need an explicit checked
implementation or an identified foreign boundary, with the port cost retained.

The [read-only module feasibility review](real-programs-current.json) remains
a source-dependency analysis, with no separate module check or port/harness.
Its smallest useful candidate is the bits tree: 539 physical local lines and
13 literal upstream test attributes. Actual error_position! calls remain
observed; positional Parser contracts, macro-generated tuple-parser tests,
Mode::Output<T>/process GAT dependencies and unsafe input-trait code remain
in the unchanged graph. Consuming const-generic/relaxed-bound headers and the
limited named-lifetime predicates does not establish module or own-test admission.

#### DOM: parser, owned tree and selector views

[Original site table](real-programs-dom.json) and [current frontier](real-programs-current.json):
127 raw messages contain 63 primary sites and 64 E_BLOCKED rows, with no current
recovery exclusions. There are 56 upstream default-path candidates and seven
test-only inventory sites. The original baseline had 95 primary sites; equality
had 92, qualified paths 83, APIT/RPIT 78 and Sized 64. The sole current named
lifetime predicate disappears at `src/selection.rs:569:9`: `select<'b>` inside
`impl<'a> Selection<'a>` declares `where 'a: 'b`. No new diagnostic replaces it
in this observation, but surviving parser barriers still prevent root admission.
The surrounding Selection operation is not claimed checked or executable.

All fourteen original const-function header identities remain absent. The
existing atom-macro pattern refusal inside `elem_require_linebreak` survives
at 127:19 after its const header at 124:7 disappears. The 22 macro sites retain
exact source accounting; only the atom-pattern site needs a current explicit
review now that the lifetime predicate is absent. The original nine shared-
mutation sites remain an ownership/representation port cost. The following table
preserves the original inventory; its Debug row is historical, not a current refusal count.

| Rank | Cause | Sites | Affected file lines | Owner / next route |
| --- | --- | ---: | ---: | --- |
| 1 | ns! / local_name! expansion | 22 | 1,843 | Compiler / producer; 17 default candidates and five test-only |
| 2 | Debug ? formatting | 24 | 207 | Historical producer refusal; ordinary Debug now supported |
| 3 | Generic associated types | 2 | 844 | Language |
| 4 | Box with dyn Iterator | 1 | 917 | Language |
| 5 | doc include_str! attribute | 1 | 45 | Language / producer |
| 6 | Expression-statement cfg | 1 | 510 | Language/configuration |
| 7 | rustfmt::skip; static CONTENTS | 1 + 1 | 692 each | Language; test-only |
| 8 | std::io imported as io::Result | 1 | 115 | Library data facade; not an ambient-I/O proof |
| 9 | Ref / RefCell / Cell | 9 | 4,073 | Refused by design; explicit ownership/tree port |

The original 24 Debug-format sites were direct producer refusals. Ordinary `?`,
`#?`, derive(Debug) and source builders now work; map/set builders and ordered
collection Debug also have focused Rust witnesses. This does not establish DOM
root admission or a new whole-root refusal count. At that earlier checkpoint, borrowed `format_args!` remained
a separate representation boundary. RefCell supports upstream's shared mutable
tree, and `Vec<NodeRef>` forms support heap-held views. The latter have
20 lexical occurrences on 19 lines, but no additional heap-view semantic-refusal
count is claimed before parsing succeeds. An eventual handle-based rewrite must
count both representation and API/test adaptation.

#### Sōzu: concurrent network service

[Original site table](real-programs-sozu.json) and [current frontier](real-programs-current.json):
four roots still produce 482 unique located nonblocked rows: 448 now have a
classified cause and 34 prior parse/recovery rows remain unclassified. Another
486 E_BLOCKED rows are excluded, giving 968 raw projected rows. Published const
generics had 447 classified + 35 unknown; the raw and nonblocked totals are
unchanged. The original baseline had 486 nonblocked rows, including 37 unknown,
and 490 E_BLOCKED rows.

The old unclassified `T: ?Sized` diagnostic at
`command/src/logging/access_logs.rs:35:8` disappears. Parsing reaches the real
`unsafe fn duplicate` at 39:5 in the Option<&T> DuplicateOwnership implementation,
and emits E_UNSUPPORTED `unsupported construct: impl item`. Source review
classifies this as an independent unsafe primary, matching the neighboring
unsafe methods at 29, 45 and 51. It is neither recovery nor unsafe admission.
The frozen original unknown row is not retroactively added to a Sized baseline
category. The two earlier removed unknown rows remain explicitly recorded at
`command/src/state.rs:1409:23` and `1435:27`, on Iterator equality bindings.

The earlier `MAX_FDS_OUT` parser diagnostic at `scm_socket.rs:111:39` was absent
from this cohort. The later literal module constant change admits the
`[RawFd; MAX_FDS_OUT]` type in a [reduction](../tests/conformance/pass/sozu_module_constant_array_length.lkj)
from the pinned Sōzu 1.0.6 file. The reduction preserves the count of 200
and uses `i32` for `RawFd`. It uses a literal repeat count. The full receive
function still includes the named repeat count and separate socket and macro
dependencies. The earlier delimiter_size observation covered its header only.
The historical totals include all six associated-constant sites.
[Pinned Sōzu source](https://github.com/sozu-proxy/sozu/blob/c5ff06a12bd514e06ab28965052d0ad1e19c45b3/command/src/scm_socket.rs#L99-L116)

| Rank | Cause | Sites | Affected file lines | Owner / next route |
| --- | --- | ---: | ---: | --- |
| 1 | Macro expansion | 235 | 11,224 | Language / checked producer |
| 2 | Macro $ / @ DSL tokenization | 2 | 2,306 | Language |
| 3 | Debug formatting | 21 | 7,344 | Language formatting producer |
| 4 | dyn traits | 15 | 2,622 | Language |
| 5 | Associated constants | 6 | 148 | Language |
| 6 | @ pattern | 1 | 167 | Language |
| 7 | async block | 1 | 68 | Language producer/runtime-future route |
| 8 | extern crate bindings | 4 | 320 | Package/language binding; not FFI |
| 9 | Generated source absent from original checkout | 1 | 257 | Package/build producer |
| 10 | Pure standard types/namespaces | 14 | 5,106 | Library and namespace routing |
| 11 | std::io Read/Write/self imports | 6 | 1,802 | Library facade; imports alone do not prove ambient calls |
| 12 | OS imports | 8 | 6,175 | Library adapters and explicit capabilities |
| 13 | Actual ambient calls | 86 | 7,562 | Refused by design; explicit authority |
| 14 | Unsafe/raw-pointer forms | 45 | 4,399 | Refused by design; ownership or foreign-process port |
| 15 | Arc | 1 | 68 | Refused by design |
| 16 | Same-process thread API | 2 | 209 | Refused by design / missing concurrency IR |
| — | Parse/recovery, not independently classified | 34 | 9,163 | No new gap assigned |

The 86 ambient calls are 82 print-family uses and one each of exit, read,
read_to_string and Instant::now. The unsafe group includes five unsafe function
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
The associated-equality, three qualified-path, opaque-return and relaxed-Sized
view reductions now check: six checking reductions and nine located refusals.
The opaque-return and Holder<T: ?Sized> reductions are accepted on both engines
and by direct rustc library-metadata compilation of each exact file; no
executable wrapper was invented for these check-only sources. The optional Rust diagnostic oracle and required diagnostic
gate retain each current verdict. These are reduced compiler work items, not
extra root-census sites or a real nom module's own tests.

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
