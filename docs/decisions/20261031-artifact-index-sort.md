# Stable artifact index ordering

Date: 2026-10-31 (Asia/Tokyo).

State: publicly adopted implementation with focused verification and a measured
complete-compiler comparison. Fresh full staged-tree acceptance and mainline
integration are pending. The acceptance receipts, when complete, must bind the
exact staged tree; no result below substitutes for that seal.

## Problem and chosen dependency path

Mode-1 artifact construction uses `artifact-order::sorted` to establish canonical
ObjectKey order and select one representative for each key. The previous selection
sort repeatedly searched every 56-byte input row for the next minimum. Its
quadratic comparisons and repeated byte-level work made the interpreted compiler
expensive even when ordinary proof admission selected `None`.

This batch changes the complete ordering path: the maintained proposal, accepted
GraphRepository body, row-order probes, canonical artifact construction, strict
artifact loading and VM/reference/Rust-native/selfhost-native agreement. It
changes no canonical schema, carrier profile, explicit proof-policy metric or
semantic feature family. It retires no independent unit, storage or source
carrier limit. Existing source-index window comparisons are unchanged.

## Algorithm and exact behavior

Build an 8-byte index containing each original 56-byte row offset. Iteratively
merge adjacent index runs of widths 8, 16, 32 and so on, comparing the original
row windows through the existing domain-rank and complete digest comparator.
Choose the left index on equal keys. Finally scan the ordered indices once,
emit the first representative of each strictly increasing key, and retain any
existing output prefix.

The `last` argument remains a row offset in the original unsorted input, or the
negative initial sentinel. It is a strict lower key threshold; it is never
reinterpreted as an index in the sorted scratch buffer. Equal ObjectKeys retain
the earliest input row, including that row's original payload location/length.
The public sorted-run merge operation is unchanged.

Each merge cursor advances over a finite index interval. `run-end` adds a width
only when it fits in the remaining interval. The run width doubles only when
`width <= length - width`; otherwise it reaches the final interval. These
operations terminate without an arbitrary work/depth allowance. All global row
offsets retain the existing checked signed byte-index domain. Producers still
validate the exact 56-byte row framing.

For N input rows and U distinct selected rows, sorting performs O(N log N)
comparisons and writes 8N initial index payload bytes, 8N per merge pass and 56U
final row payload bytes. This is an algorithmic payload-write description, not a
measurement of allocator traffic, total copying, peak retained storage, mapped
capacity or RSS. ByteBuffer growth and the native process-lifetime allocator
remain separate costs.

## Accepted meaning and complete inventory

Base main and candidate precommit HEAD:
`bcf455816c11f99ebb60e76f786a1c2baf1e41e7`.
The focused implementation index tree was
`a9e9819ac81d509b8a180780e1f4a024f506fa3e`; documentation finalization produces a
different candidate tree that still requires the full seal.

Public transaction:
`transaction_53bde7febf56aceec3b48707947b5571c41c6950d5f885fad0d544dc33fd1d33`.

Accepted revision:
`rev_71fade8bd535b16925f51aa8c0e00260bf375288febad1654a602f7308f01d83`.

Public HEAD SHA-256:
`34b5ed697976321ad1c26b5554ee9fb82911a2389c5398c591c1ae7fb12048bc`.

Semantic state:
`semantic_state_4afafbe96907620b4e04d62319faa3c1253ce8c8ee56a012fd834a084883d7fc`.

The transaction created 291 owners, updated the sorting body and deleted the
76-owner obsolete minimum closure. Public readback covered 36,295 owners in nine
pages; differential admission agreed. Complete maintained-proposal replay
matched 655 canonical units and the native backend closure. Public GC retained
153,665 objects and reduced 38 packs / 90,696,823 bytes to four packs /
87,934,112 bytes. GC receipt SHA-256:
`a9d1ba772d27e62a776f2ae5bc3d7e7834a51418826dc0b37bb569cc68e4cf78`.

The complete owner/type inventory passed after adoption: 73 modules, 651
callables (627 pure, 24 external), 36,295 owners, 41 types, 30,873 expressions,
7,768 call expressions, 2,527 bindings and 16 closed generic application sites.
Parameter modes are 1,980 unrestricted, 182 consume and one borrow. No new
intrinsic or source feature family entered F_active. Source carrier:
12,312,663 bytes, SHA-256
`0f8b486925e1482fbb482204c3952313448635d868dca42156d83682707cbc60`.
The complete profile report is 14,106,680 bytes, SHA-256
`40dd7b77452f3054020567a8600b30bfc5c1738f2021f7f4e1f2a0160733b8f2`.

## Focused behavioral and complexity controls

All commands used the shared target lease, release mode and locked dependencies.

| Selection | Result | Test execution |
| --- | --- | ---: |
| `selfhost_artifact_order_` | 3 passed, 0 failed, 0 ignored, 2,857 filtered | 42.53s |
| `selfhost_maintained_proposals_match_authoritative_function_bodies` | 1 passed, 0 failed, 0 ignored, 2,859 filtered | 47.55s |
| `selfhost_source_profile_inventories_every_owner_and_type` | 1 passed, 0 failed, 0 ignored, 2,859 filtered | 13.03s |

The row controls independently prescribe canonical domain order, all digest
bytes, stable duplicate representatives, original-input thresholds, output
prefixes, empty/singleton inputs, odd run lengths, sorted/reversed rows and
separated duplicates. Behavioral probes execute across all four evaluators.
The scaling control counts unsaturated VM instructions on 64/128/256 reverse
rows: 430,207 / 954,599 / 2,096,719, with maximum call depth seven. Its generous
less-than-threefold doubling envelope detects renewed full rescans without
turning elapsed time or passive allocation observations into admission policy.

Logs under `.artifacts/capacity-unbounding/artifact-sort-20261031/focused-round2/`:
order selection SHA-256
`51f83d32bf01a102e3fc3ea4421cbed3303aaac63086db597d6b230d4ac4f7e3`;
proposal replay
`eae827349c0ad8b7adc0f9c42d271982d6f77631e8a25220463bc6fe8f9a0da4`;
complete profile
`1284b3f26db3e659660b33a2e4197a7b18466622f58d67c75d3e5b3ed2a9eac2`.
The Rust formatting check also passed. These are focused results, not full gates.

## Pinned whole-compiler comparison

The ignored `selfhost_artifact_measure_pinned_compiler_bundle` test is invoked
explicitly with `LKJSCRIPT_SELFHOST_PINNED_ARTIFACT`. It strictly loads the pinned
host artifact, constructs the same Mode-1 request, executes all four evaluators,
strictly loads their equal output and independently compares the exact keys and
canonical bytes of CompilerUnit, Owner, Type, Blob, PackageRevision and
PackageInterface objects. It records phase times, unsaturated work counters and
explicitly unmeasured metrics. Optional expected-output SHA-256 binds comparisons.

Pinned host artifact: 650 units / 10,403,995 bytes, SHA-256
`977bec24d297c2420d5fb1d14b4bd22c765597c676e3689b6cb5db77d7573574`.

Identical Mode-1 input: 10,806,982 bytes, SHA-256
`f15e861ab40e81e9d1faadacbd407677831794ee0e371e7a3331b5a2b375f18c`.

Identical output before/after and across all four evaluators: 10,338,849 bytes,
SHA-256 `cd1906c2413209d941f45004f2bd02120e494e941f135d145a8246d0381fc322`.
Strict loading and all 37,712 independent object comparisons passed.

| Observation | Before | After | Reduction |
| --- | ---: | ---: | ---: |
| VM instructions | 826,835,225 | 739,945,099 | 10.51% |
| Reference expressions | 438,158,052 | 387,022,989 | 11.67% |
| VM modeled cumulative allocated bytes | 86,155,580,476 | 79,041,132,506 | 8.26% |
| Reference modeled cumulative allocated bytes | 97,782,657,028 | 89,669,132,698 | 8.30% |
| VM elapsed | 103.671s | 93.557s | 9.76% |
| Reference elapsed | 526.016s | 482.161s | 8.34% |
| Rust-emitted native elapsed | 1.607s | 1.425s | 11.32% |
| Selfhost-emitted native elapsed | 1.025s | 0.942s | 8.16% |
| Profiled total elapsed | 665.200s | 612.740s | 7.89% |

Each timing is one observation on this pinned workload, not a distribution or
universal speedup. Native timing includes executable publication, process
creation, transport, execution, waiting and cleanup. Initialization increased
from 32.195s to 33.872s. VM/reference maximum call depths remained 21/22 and all
reported work/allocation saturation flags were false. Actual allocator traffic,
copied bytes, peak live bytes, mapped arena capacity and RSS were not measured.
The comparison's elapsed duration is separate from full verification duration.

Baseline and after runs each passed one test and joined with exit zero. The
runner bound pre/post raw source inventory, index tree, project HEAD, test-file
hashes and executable digests, and required stability. The baseline report binds
the baseline compiler semantic state; the after report also records the authored
probe semantic state. The added probe-state field is observational metadata.

Evidence directory:
`.artifacts/capacity-unbounding/artifact-sort-20261031/`.

| Evidence | SHA-256 |
| --- | --- |
| `baseline-round2.json` | `641ef390cc5327763aeebdda9e764cadd07e62ffbe9fa68b4088e8edbce98b3d` |
| `baseline-round2-run.json` | `642fc19c02aac1483d414e7ccbc8fffbe2292a88a035a501db639fbc2062e19e` |
| `after-sort.json` | `4058a12a042f6df76901b1a4e71b0a77f8ede06d526bbec36f086f7b252c677f` |
| `after-sort-run.json` | `7b3ec47fa35a73c8671259df4ef62f37447da3554ec310949f4bdb792e164af9` |

The host artifact has a different physical map/manifest layout from this emitted
artifact. Preserved canonical objects and before/after equality do not establish
`A_host = A1 = A2 = A3`. Raw-source semantic admission, ByteBuffer custody,
source-derived canonical summaries/artifacts, full P_self families and native
phase reclamation remain independent obligations.
