# Copy-site profiles bound to selected checked IR

Source `d461c2d98e93aeed6061508ee80ab36b9eeaa04f` implements the
[prospective copy-site contract](compiler-profile-copy-sites-plan.md).
The [publication clarification](compiler-profile-copy-sites-clarification-20261116.md)
keeps its original plan bytes and corrects the earlier overstatement about
partial publication. The normative format and command contract are in
[docs/ir.md](../docs/ir.md#native-cpu-profiles-and-selected-ir).

This is attribution infrastructure, not an optimization or a new attribution
measurement. The historical 10.369% marginal remains unchanged. Neither a
removable aggregate subset, a Rust-O milestone nor a backend floor follows from
these acceptance cases.

## Implemented correspondence

```text
lkjscript build SOURCE --profile --profile-ir PROGRAM.native.lkjir -o PROGRAM
lkjscript profile PROGRAM.samples.PID --copy-sites \
  --profile-ir PROGRAM.native.lkjir --limit 100 --bytes 65536
```

The emitter and companion writer use the same `checked.native()` selection.
Ordinary `ir` still writes the original admitted program. A companion request
bypasses the early image-only memo return; frontend/check/fragment reuse remains
available. The detail reader binds exact companion bytes, independently checks
that IR, then validates every copy coordinate, role and width before returning
a bounded reply. This whole-artifact work is separate from authenticated-store
semantic-query latency.

LKJPROF4 adds a selected-IR digest, 72-byte sampled copy rows and a 16-byte row
per selected function. Recorded coordinates use the actual post-inline body
index, incoming-parameter sentinel and original constructor indices. Only
existing KIND_COPY intervals participate. Runtime/allocator overrides,
Replace, variable-width runtime copy and work outside those intervals remain
excluded. Unknown attribution has an explicit unmapped counter.

LKJCODE3/native-functions version 3 preserve relative metadata through cold
emission and warmed fragments, with complete preflight before assembler
mutation. Ordinary fragments have no copy-count field or rows. Ordinary
emission does not allocate copy-site rows, construct semantic role/path metadata,
serialize/hash selected IR for this feature, or add per-copy application
instructions. Empty vector state, two Body coordinates and conditional
bookkeeping remain; their compiler execution cost is not claimed to be zero.

Held output-parent identities and basenames reject aliases. Both outputs are
staged, then companion and image are renamed in that order. A partial failure
returns nonzero and publishes no new image memo, but can leave the companion
replaced. The pair is not atomic. A digest-identical old companion remains
corresponding without a freshness or common-publication claim.

## Checked source transactions and retained refusals

Baseline Git revision is `27c94440646d787efc4e7b43a712405288c22c61`.
All applications use the prior fixed-point producer
`34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377`.
The first three transactions refused before publication; the tree and generation
were checked unchanged before proceeding. These were repaired candidates or
arguments, not repeated benchmark trials.

| Attempt | Result | Wall s | User / system s | Peak RSS KiB | Load before → after, 1 min |
| --- | --- | ---: | --- | ---: | --- |
| transaction-1 | Explicit root plus absolute desired entry refused E_IO, errno 18 | 20.98 | 20.30 / 0.23 | 772,056 | 3.07 → 4.05 |
| transaction-2 | Constant patterns, owned-array iteration and usize.to_string refused | 40.48 | 39.44 / 0.38 | 1,059,124 | 3.40 → 3.00 |
| transaction-3 | Added imports refused E_REPLAY_FRAME after candidate elaboration | 41.66 | 40.48 / 0.39 | 1,128,844 | 1.79 → 1.84 |
| transaction-4 | 71 operations; six production files published | 64.86 | 62.81 / 0.58 | 1,277,548 | 1.38 → 1.83 |
| fixture-transaction-1 | 14 add operations; one existing fixture extended | 0.84 | 0.77 / 0.01 | 67,240 | 2.34 → 2.34 |

Transactions used a 4 GiB address-space limit and a 900 s watchdog with 10 s
grace. The successful production replay records 12,877 checked instances and
no entry-authority change. Its selected-root checked meaning is identical on
the desired and published candidates:

```text
241e20431bf668f7542324342181d868fc21dae82cf55bf8acc9c3446f227576
```

The fixture replay checks 39 instances and records the new pure `copy_once`
and `copy_roles` entries plus the explicit World/stdout `copy_samples` entry.
Its desired and published checked meaning is:

```text
48792404fbeb35c3ae9c28bc89d0d00d3bcc552ece5847b4480095f6515915b2
```

The fixture bytes exactly match the desired proposal,
`184adbc8dd758c72474f268122602285468e99d2b21362216b9c5918b6e79fd4`.
Production projection differs from the final proposal only by 41 added/four
removed blank lines and three omitted standalone explanatory comments.
No code, attribute, signature or literal was lost. The proof is selected-root
canonical checked-meaning equality, not raw serialized-IR byte equality.

### Adoption audit

The independent audit checks 1,937 baseline input blobs and the exact path set
against Git, all six preimages and transaction-reported after hashes, all 216
current production files and all 210 untouched production files. It reproduces
all six candidate/projection diffs byte-for-byte. The separate fixture proof
binds its complete before/after bytes.

| Retained change | Added | Deleted | Operation-written lines |
| --- | ---: | ---: | ---: |
| Six src files | 1,108 | 98 | 1,206 |
| tests/fixtures/profile.lkj | 102 | 0 | 102 |
| Total | 1,210 | 98 | 1,308 |

All 1,308 retained language lines in this source commit went through 85
operations. Preparatory untrusted candidates are archived as evidence, not
counted as adopted source. Relative to the previously audited cumulative
8,074/10,690 and production 8,072/9,930 counts, totals become
**≥9,382/11,998 (78.2%)** overall and **9,278/11,136 (83.3%)** in src/library.
The denominator remains the sum of per-commit added/deleted language lines,
with the earlier exclusions and uncredited work unchanged.

## Focused validation

The first current producer is 32,584,542 bytes, SHA-256
`6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc`.
It was built from the applied source at producer inline limit 64, with no store.
Ordinary output continues to default to inline limit zero.

| Check | Result | Wall s | User / system s | Peak RSS KiB | Load before → after, 1 min |
| --- | --- | ---: | --- | ---: | --- |
| Build applied compiler | Passed; 46,716 inlined calls, optimized output rechecked | 34.04 | 33.07 / 0.32 | 1,076,176 | 1.88 → 1.72 |
| Closed core | Passed, including native fragment contracts | 33.20 | 32.13 / 0.42 | 571,184 | 1.63 → 1.93 |
| Old producer, ordinary original fixture | Build passed | 0.17 | 0.15 / 0.00 | 43,340 | 1.57 → 1.57 |
| New producer, same ordinary original fixture | Exact image-byte equality with old producer | 0.20 | 0.19 / 0.00 | 43,784 | 1.57 → 1.57 |
| Short role fixture, profiled build | Exact selected companion; build passed | 0.18 | 0.16 / 0.00 | 45,320 | 1.22 → 1.22 |
| Short role fixture, execution | Assertions passed; scalar standalone image has empty stdout | 0.00 | 0.00 / 0.00 | 2,132 | 1.22 → 1.22 |
| Short role fixture, detail | All 58 rows validated against independently checked companion | 0.01 | 0.01 / 0.00 | 14,928 | 1.22 → 1.22 |
| Extended profiler regression | All positive controls and 32 named new refusals passed | 6.27 | 5.97 / 0.20 | 47,264 | 0.89 → 1.59 |

GNU time has two-decimal precision here; 0.00 retains that precision rather
than asserting zero execution cost. Full 1/5/15-minute load records, commands,
UTC endpoints, stdout/stderr, statuses and resource captures are retained in
[the raw data](compiler-copy-sites-data/). Build/core phases used 4 GiB/900 s;
the regression used 4 GiB/600 s; short role execution/detail used 4 GiB/60 s,
each with 10 s watchdog grace. These are acceptance timings, not a speed trial.

The ordinary comparison uses the original 411-byte profile fixture
(`8899630364d9922f2de4c5ccfd667e1c582551dcad6800060c047a6a0534b7a5`),
before adding the new entries. Both images equal
`abcba417f3d240c9d89e885edf3a3d0e3ffb8d9a764b706c59e0ab963ca0cffd`.
This is a fixed specimen byte-equivalence check, not a general performance
comparison.

The short role artifact has zero samples as expected for this small execution;
its 58 rows remain visible. Its exact 59,965-byte companion is retained as
[roles.lkjir](compiler-copy-sites-data/roles.lkjir), SHA-256
`b323c6407a905ef3f7097406bd4659f728b9c252b27a70452789da7233b0154b`.
The compressed raw sample decompresses to
`1f0912222954ccfc998e2ee27f8164c0e631381701b6ae1b04d1ae43907a27a1`.
This supplies inspectable coordinate/type/layout evidence without pretending
that zero-count rows are dynamic cost measurements.

### Regression coverage and limits

The frozen test tool is
`bd2bdedbbfeebaf64eb9369dd6b077ad0176a42e64a914a17b97b2d0f0b837a9`.
The complete successful shell trace is
`45e4407fd3510edfa7e943c1f4c26bce670b2ca181d86435f836fb599dce8ee6`.
An independent read-only audit verifies exact tool/fixture/producer bindings,
the ordered 32 new refusals and the completed positive controls.

The controls cover original-IR interpreter/native value parity, ordinary versus
profiled hot-loop output, incoming/result 1/16/256-byte aggregates, constructor
subsites 1/3 across scalar gaps, local and projected copies, arrays/repetition,
vector push/pop, boxes, derived clone and fat views. Recorded read flags are
1/2/3/4; recorded write flags are 1/2. The compound/indexed writes in this
fixture lower to Replace and are correctly absent from copy rows. Source-derived
clone lowers to a dereferenced Read. The separate raw Referent intrinsic role
is not dynamically covered; the existing check-only IR provenance specimen
does not provide a runnable zero-argument entry.

Other controls establish exact selected-IR correspondence, independent refusal
of a matching-digest invalid IR, cold/warm fragment equality, regeneration of a
deleted companion despite an image memo, malformed ranges/indices/tags/widths,
counter overflow and accepted/rejected one-handler states. Forced
optimizer-rejection fallback was not dynamically induced; emitter and writer
share the same Selection in that branch by construction.

The hot regression observes mapped copy samples and 14 sites. Its first page
has one row and its continuation the remaining 13; 896 bytes admits the exact
first page and 895 refuses it. These counts describe the test, not compiler
attribution. Raw hot samples, companions, cache stats and individual error files
were removed by the test's ordinary cleanup. The retained tool and completed
trace prove their assertions ran; they do not support a fresh independent
recomputation of those transient artifacts.

Same output names and parent aliases refuse before staging. An owned
nonwritable companion parent reaches its exact creation failure. An occupied
image destination forces the second rename to fail after the exact companion
is published. All controls preserve their specified prior outputs, remove
temporary stages and leave no new image or inputs-image memo. The test's exact
fixture and private-key directory were removed. No new kill/race campaign or
operating-system observer was introduced.

## Full gate, seed and CI

Source `d461c2d9` passes the complete four-worker gate from a fresh empty
authenticated store: **616.092 s, load 2.68 → 4.26**. All **5,448 case requests
are fresh and zero are reused/skipped**; host suites run. The mutation phase
passes 128 mutants: 104 rejected, 22 values, one trap, one nontermination,
zero resource refusals and zero findings.

GNU time records wall 616.09 s, user/system 1,536.38/34.01 s and peak RSS
1,229,496 KiB over that same load window. RSS is the largest recorded process,
not the aggregate memory of four simultaneous workers. The wrapper is 4 GiB
per process with a 2,400 s watchdog and 10 s grace. All 1,937 pinned input files
verify unchanged afterward. The fresh store contains 321,266,634 apparent
bytes; all 114 gate logs are retained under [gate](compiler-copy-sites-data/gate/).

Bootstrap passes in **78.236 s (load 2.68 → 2.54)**, requiring consecutive
current-emitter compiler bytes to agree. The resulting canonical image is the
same 32,584,542-byte `6cd52308…e7bdacc` produced by the focused build.
The extended profile suite passes inside the gate in
**6.701 s (load 3.94 → 4.03)**. The mutation phase takes
**2.463 s (load 4.37 → 4.26)**.

Seed commit `7d0731e99aeaf241c4334fc5ff065a341fb388c8` stores 6,547,345 bytes,
SHA-256 `505be947525d53b4db1bff6c85cb77622cb2f5dc7e7d7fe92b772ee3b8ffc3e2`.
Two independent gzip -n -9 encodings agree, and decompression compares directly
with the fixed-point canonical image. Production source tree is
`c710586b4d1b7ab476fd179a7d2c699477a2054a`; library tree
`25bdb8ff8567b941790d3951c78e896b23e26e8e` and runtime blob
`2fed59cd33d8a2ab80d562f160648b05f24b8c4c` are unchanged.

Current-source CI remains pending publication and inspection. The preceding
bench/docs-only head `dc0a325e` passed both CI43864548875 jobs; that result is
not a CI claim about this source change.

## Evidence and next measurement

[The focused summary](compiler-copy-sites-data/focused-summary.json) derives
the phase table from raw records. Source/proposal archives retain exact bytes,
including refused candidates and the prevalidation role README. Its proposed
projected-write coverage and standalone scalar output were corrected by the
actual selected IR and execution described above; those proposals were not
retroactively rewritten.

Before cleanup, verify the retained bundle and exact original source/proposal
contents. The global compiler store key and unrelated query-container operation
credentials remain outside this evidence and cleanup scope.

A separately committed and released fixed-source protocol must bind the gated
producer, runtime/library inputs, selected-IR bytes, ordinary control, limits,
command order and output oracle before a compiler attribution run. One bounded
detail page is not a full census, and samples multiplied by width do not measure
dynamic copied bytes or removable time.
