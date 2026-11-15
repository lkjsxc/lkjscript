# Compiler structural-query baseline

These twenty questions were selected from the compiler stream's actual searches and source reads while implementing structural queries, transactions, comparison, and authenticated images on 2026-11-15. The authenticated snapshot is the compiler before typed-reference and emitted-byte improvements in f6986490. This is a fixed historical view; it does not assert that the working tree still matches it.

## Reproduction and limits

Run `bash bench/compiler-queries.sh ROOT BINARY STORE KEY SNAPSHOT`. The script runs from ROOT, uses source `src/main.lkj` and `--library-dir library`, requests depth 1, at most 96 facts, and at most 32,768 bytes. Every reply includes its protocol header and continuation line in the byte total. All twenty replies finished without a continuation.

The old-route byte column is a conservative, reproducible lower bound: the exact selected item's source span in this same authenticated snapshot. It excludes file-search results, surrounding context, navigation, and repeated reads. It is not a claim that historic tool responses consumed exactly that many bytes. Source-span paths and offsets are preserved in the TSV. A signature query can save a large body; an outline of a small type costs more bytes because it includes identities, levels, and the protocol. The aggregate here is 41687 source bytes versus 47370 reply bytes (1.136×); there is no aggregate byte saving on this deliberately varied set.

Source outlines, signatures, and item text are labelled `source-claim`; authority is labelled `checked-ir`. Reference coverage in this baseline is recorded sites, with explicit gaps, and instance coverage is demanded instances. No source claim is a checker proof.

## Twenty actual questions

| Question | Selected source bytes | Reply bytes | Facts | Whole ms | Producer ms | Index ms | Load before → after |
|---|---:|---:|---:|---:|---:|---:|---|
| `outline driver::Options` | 874 | 4486 | 30 | 268.538 | 260.358 | 8.162 | 7.88 → 7.88 |
| `outline driver::Memo` | 769 | 3692 | 23 | 266.715 | 259.632 | 7.062 | 7.88 → 7.88 |
| `item driver::compile` | 9045 | 11888 | 11 | 267.700 | 262.941 | 4.739 | 7.88 → 7.48 |
| `signature driver::execute_source` | 10348 | 733 | 1 | 257.755 | 256.375 | 1.359 | 7.48 → 7.48 |
| `signature driver::read_path` | 311 | 674 | 1 | 252.856 | 251.845 | 0.993 | 7.48 → 7.48 |
| `signature driver::diagnostic_source_path` | 434 | 709 | 1 | 251.474 | 250.565 | 0.891 | 7.48 → 7.48 |
| `outline ir::TypeKind` | 1797 | 4895 | 34 | 260.712 | 251.300 | 9.393 | 7.48 → 7.48 |
| `outline ir::Op` | 2356 | 5582 | 30 | 256.308 | 248.900 | 7.389 | 7.48 → 7.48 |
| `outline ir::Function` | 265 | 1498 | 8 | 262.631 | 260.080 | 2.533 | 7.48 → 7.48 |
| `outline ir::Program` | 264 | 1023 | 4 | 261.631 | 260.379 | 1.234 | 7.48 → 7.48 |
| `outline ir::Local` | 112 | 834 | 3 | 281.799 | 280.468 | 1.313 | 7.48 → 7.48 |
| `outline ir::ReturnSource` | 470 | 963 | 3 | 265.242 | 264.106 | 1.118 | 7.48 → 7.48 |
| `outline semantic::Catalog` | 424 | 2110 | 11 | 250.858 | 247.615 | 3.224 | 7.53 → 7.53 |
| `outline semantic::Item` | 516 | 2835 | 18 | 261.442 | 256.604 | 4.820 | 7.53 → 7.53 |
| `outline semantic::Reference` | 174 | 1267 | 6 | 255.082 | 253.152 | 1.911 | 7.53 → 7.53 |
| `outline reuse::AuthenticatedRoot` | 161 | 1317 | 6 | 263.325 | 260.939 | 2.368 | 7.53 → 7.53 |
| `signature test_runner::closed_contracts` | 360 | 674 | 1 | 261.538 | 260.626 | 0.895 | 7.53 → 7.53 |
| `signature source::load_report` | 10949 | 794 | 1 | 262.429 | 261.407 | 1.004 | 7.53 → 7.53 |
| `outline native::PreparedImage` | 78 | 785 | 2 | 248.241 | 246.718 | 1.505 | 7.53 → 7.53 |
| `authority authority::analyze` | 1980 | 611 | 1 | 267.014 | 265.956 | 1.040 | 7.53 → 7.53 |

The 100 ms whole-command target is **not met**. These commands took 248–282 ms. The authenticated page traversal took 0.89–9.39 ms; hashing the exact executing producer took 246.7–280.5 ms. Removing or memoizing that identity check across unauthenticated process starts would weaken the existing trust rule, so this implementation retains it. The native hash/code-generation work is being measured separately.

## Shared retrieval evidence and whole-file bounds

The parent session's actual IR read was `sed -n '1,175p;535,735p' src/ir.lkj`: **9,238 bytes shared across the six IR questions**, not six separate reads. Its current file SHA-256 exactly matches the pinned snapshot. The parent also reported a full driver read of approximately 130 kB followed by focused ranges; the exact historical response is not retained here, so it is not substituted for the lower-bound numbers. Several framework files were read in full while preparing guarded edits. Their same-snapshot whole-file sizes, where recoverable, give a realistic alternative to an oracle selecting just the desired item.

| Source shared by questions | Same-snapshot full-file bytes | Evidence |
|---|---:|---|
| driver (six questions) | unknown | Pinned file digest has no matching retained blob among the checked revisions; parent reports an approximately 130 kB full read. |
| ir (six questions) | 25,678 | Current bytes match snapshot; actual shared ranges were 9,238 bytes. |
| semantic (three questions) | 33,012 | Exact digest match in commit 40494fff. |
| reuse | 19,407 | Current bytes match snapshot. |
| test_runner | 45,550 | Current bytes match snapshot. |
| source | 20,125 | Current bytes match snapshot. |
| native | unknown | Pinned file digest has no matching retained blob among the checked revisions. |
| authority | 20,021 | Current bytes match snapshot. |

These shared reads are not added once per question or combined into a purported historic total. The lower-bound column above remains useful precisely because it separates answer content from the cost of locating it.

## Exact baseline identity

- Binary: `/home/coder/workspace/lkjscript/target/compiler-structure-indexed`, 18,256,053 bytes.
- Store: `/home/coder/workspace/lkjscript/target/compiler-structure-store`, explicit 2 GiB budget.
- Snapshot: `abc628654e12a0bec9c049b5da2c263452d315b05f0576c4e5f418e1c1e1b43b`.
- Source identity: `989aa1cfe0c9927bc259e8f7293b7d5d345d15586475d9a85631e0d259be1902`.
- Checked IR identity: `b475ba40bf758b424538d8a636af0a1f2120297ef8356e910a39b2d6add8afba`.

The parent stream measured the initial index build at 103.82 s wall, 96.34 s user plus 4.76 s system, peak 1,808,156 KiB, load 3.05 → 4.80. Catalog construction was 18.721 s, index publication 61.461 s, retained store payload 551,977,300 bytes. Indexing remains explicit because this cost does not justify enabling it for every ordinary build yet.

## Profile of the current catalog and index

A separate instrumented f6986490 compiler indexed its own source under an 8 GiB address-space limit and a 2 GiB store budget. That run took 117.06 s wall, 109.82 s user plus 4.90 s system, peak 2,115,984 KiB, load 1.52 → 3.87. Its source had 3,032 definitions, 10,862 IR functions, and 1,018,937 IR instructions. Catalog construction was 18.962 s; publication was 75.329 s; emitted image size was 17,789,724 bytes; retained store payload was 588,066,134 bytes. Instrumentation overhead and different source prohibit treating this as a regression against the parent baseline.

The 1 ms profile contained 114,298 samples. Inclusive samples attributed 18,627 to catalog construction, of which 15,180 were in `semantic::append`. The code slices the backing source `String` twice for each item; `String::as_str()` validates the entire backing UTF-8 string on every such slice. The resulting repeated whole-file validation, not token hashing (510 inclusive samples), is the first catalog wall. The repair validates just the selected byte ranges before creating owned item/signature strings.

Index publication accounted for 73,074 inclusive samples, including 38,724 in fact construction. B-tree key comparisons accounted for 23,890 inclusive samples and fact insertion for 22,589. These long source-identity keys are looked up repeatedly for each fact. SHA-256 compression accounted for 19,610 self samples. The largest remaining index costs are explicit rather than hidden behind the query traversal measurement.

Store `rejected`/`evicted` counters in these runs include probes of absent pages: the current `get` path opens an absent name for append, fails authentication on its empty contents, and discards it. Those counts alone are not evidence of actual retained-page eviction or semantic-key collisions. Publication now checks that every descendant remains retained; a store that cannot retain the complete snapshot refuses and invalidates its root/latest pointer.

## First catalog repair measured

Commit 79be7063 was applied as one checked structural transaction (nine requested operations, eight changed identities, three files): 92.30 s wall, load 4.60 → 7.07, peak 1,362,412 KiB using the older transaction binary. Its compiler build passed in 20.94 s, load 12.14 → 10.38; the focused query/attack suite passed in 12.69 s, load 7.01 → 6.85.

The rebuilt ordinary compiler indexed itself in 101.66 s wall, 93.23 s user plus 4.62 s system, peak 2,018,524 KiB, load 3.19 → 6.66. Full typed catalog construction was **5.609 s**, versus the earlier 18.721 s ordinary parent baseline and 18.962 s instrumented child run. Sources differ slightly (3,035 definitions, 10,871 IR functions, 1,019,337 instructions in this run), so this is a measured first-consumer improvement, not a fixed-input microbenchmark. Publication still cost **73.461 s** and retained 588,302,544 bytes. The earlier profile and code inspection identify the removed whole-file UTF-8 rescan; remaining index cost is not hidden by that gain.

The same commit exposes declaration-only capture for operations that only replace/add/remove whole declarations or bodies. This mode keeps exact item/body/file spans and digests, omits body bindings/call sites/reference discovery, sets `Catalog.references_recorded=false`, and marks item reference completeness false. It can reuse the ordinary frontend checkpoint; it must not authorize rename, refactoring, absence-of-uses claims, or the full query index. Its isolated cost has not yet been measured here.
