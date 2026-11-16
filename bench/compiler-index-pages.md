# Lazy serialization of semantic index pages

The candidate reduces measured indexed-build peak RSS by **18.535% at 10k**
and **21.223% at 100k**, while preserving every rooted logical page byte and the
emitted native image. The 100k observation falls from 2,699,892 to 2,126,904 KiB,
a measured reduction of 572,988 KiB (586,739,712 bytes). This is a memory
improvement. It is not a generated-code or compiler-speed milestone, and it does
not establish that the original 1M refusal was caused by indexing or has been removed.

The change leaves the persistent representation unchanged. Both 100k stores
contain exactly 651,302,294 bytes, including 601,936,928 bytes of authenticated
semantic-page payload. The census below explains that large constant: rendered
facts and repeated identities dominate it; a transitive all-pairs function graph
does not.

## Frozen experiment and limits

The [declared plan](compiler-index-pages-plan.md) was committed before measurement.
Production source is trial commit `767a341a6e8b1098cdfb5301d36abe5d7f2f7880`
on baseline `71e3c9637aab74fdb860dbe1d5ff2ab9631b2f31`; the post-build
namespace census was frozen in `c5c8b718`. The exact command scripts, raw
outputs, stats and hashes are in [compiler-index-pages-data](compiler-index-pages-data/).
The [SHA-256 manifest](compiler-index-pages-proof.sha256) covers the retained evidence.

| Role | SHA-256 |
|---|---|
| Baseline producing executable | `71b2fc066cd74e9e37cb3134173903c9b322b83472b9293f1a49df7c1d18f3b7` |
| Candidate producing executable | `40464dda053795b93dbba1af93e41633b915b2b625b0007a7b95ec4d22f0f200` |
| Frozen compiler-shaped source revision | `0b30ca8987bd4ba343ab45ab01751b6721c7151a` |
| Shape census | `0457f8f9424882d2e3160a568f280193d49b9b046a5fae2d6e2d0b6da047072c` |

The generator uses seed 20261116. Both producers read the same generated paths,
bytes and frozen library/runtime. The frozen library contains 28 language files
and 259,234 bytes; the generated-source byte counts below exclude that shared
library. Image emission uses `--inline-limit 0`. Each command has an external
4 GiB address-space ceiling and 900-second watchdog, each store has a 2 GiB
budget, and the preflight required 9,126,805,504 free bytes before allocation
(359,967,821,824 were available). One compute worker executed the sequence.
All commands completed in the first declared measurement attempt.

For each rung the order was baseline ordinary cold build in an empty store,
baseline indexed build in that warmed store, candidate ordinary cold build in
its separate empty store, candidate indexed build, full payload/image comparison,
then ten paired query checks. There was no unchanged-index timing, body edit,
new latency campaign or 1M run. Queries are correctness observations only.

This is a distinct current-producer experiment. It does not replace the original
54db01f5 observation in [compiler-scale-final.tsv](compiler-scale-final.tsv):
100k indexed build 111.57 s, 2,670,368 KiB, 613,720,812 store bytes, load
5.82→7.14. That earlier fixture was cleaned; no namespace/page-size listing,
full fact count or IR-local census survives. Its ordinary cold store was
26,658,808 bytes, so its observed index-era growth was 587,062,004 bytes.
The new census is not retroactively attributed to that old producer, path or run.

## Measurements

The table uses GNU time peak RSS and process user/kernel CPU. Load is the
one-minute value before→after; all three load averages remain in raw metadata.

| Generated lines | Phase | Producer | Wall s | User + kernel s | Peak KiB | Load |
|---|---|---|---:|---:|---:|---|
| 10,000 | Ordinary cold | Baseline | 1.68 | 1.62 + 0.03 | 107,984 | 10.04→10.04 |
| 10,000 | Indexed, ordinary store warm | Baseline | 15.42 | 14.62 + 0.58 | 387,656 | 10.04→9.75 |
| 10,000 | Ordinary cold | Candidate | 1.62 | 1.55 + 0.04 | 104,884 | 9.75→9.75 |
| 10,000 | Indexed, ordinary store warm | Candidate | 15.72 | 14.78 + 0.52 | 315,804 | 9.75→9.16 |
| 100,000 | Ordinary cold | Baseline | 12.15 | 11.58 + 0.25 | 705,816 | 9.16→8.88 |
| 100,000 | Indexed, ordinary store warm | Baseline | 136.60 | 127.41 + 5.68 | 2,699,892 | 8.88→11.17 |
| 100,000 | Ordinary cold | Candidate | 14.48 | 13.98 + 0.21 | 705,024 | 11.72→12.49 |
| 100,000 | Indexed, ordinary store warm | Candidate | 132.81 | 123.87 + 5.80 | 2,126,904 | 12.49→10.53 |

Index wall time is slightly worse at 10k and slightly better at 100k. The host
load and ordinary-build dispersion make a speed claim unwarranted. There is
one observation of each condition, with no rerun or best-run selection.

| Generated lines | Source bytes / files | Generated functions / generics / calls | Checked instances / IR instructions | Image bytes | Store after ordinary / indexed build |
|---|---|---|---|---:|---:|
| 10,000 | 384,223 / 12 | 384 / 17 / 4,973 | 426 / 87,347 | 456,126 | 2,682,773 / 86,621,364 |
| 100,000 | 3,923,070 / 169 | 3,764 / 155 / 47,607 | 4,209 / 896,361 | 4,471,471 | 26,712,816 / 651,302,294 |

Those structural counts, emitted bytes and store totals match between producers.
Ordinary cold builds reused zero checked bodies; indexed builds reused the
ordinary store's 426 or 4,209 checked bodies. Each producer has its own store.

At 100k the compiler's measured catalog/index phases are 42.546/83.958 s for
baseline (load 8.88→11.17) and 42.036/80.244 s for candidate (load 12.49→10.53).
The catalog, global facts, checked IR and persistent representation remain
substantial costs. These phase timers are wall time, not separate CPU profiles.

## Exact retained representation census

[The comparator](compiler-index-pages-compare.pl) authenticates both producers'
snapshot roots and locators, walks every rooted semantic page, verifies its
content identity, and compares page payload bytes exactly. It also decodes
framing to count logical pages and rendered fact bytes.

| Count or bytes | 10k | 100k |
|---|---:|---:|
| Rendered facts | 343,561 | 2,902,333 |
| Logical fact pages | 90,529 | 570,154 |
| Physical radix pages | 6,890 | 70,211 |
| Branch / leaf pages | 463 / 6,427 | 4,402 / 65,809 |
| Maximum radix depth | 4 | 5 |
| Rendered fact UTF-8 bytes | 61,623,831 | 470,782,475 |
| Encoded logical-value bytes | 66,545,015 | 507,684,835 |
| Logical-key bytes | 6,418,169 | 36,214,989 |
| Authenticated page payload bytes | 81,701,336 | 601,936,928 |
| Page envelope bytes | 82,321,436 | 608,255,918 |

The 100k page payload decomposes exactly:

| Component | Bytes | Construction |
|---|---:|---|
| Rendered fact strings | 470,782,475 | Full identity, level, fact label and formatted detail per row |
| Logical page/line framing | 36,902,360 | 24 × logical pages + 8 × facts |
| Logical keys | 36,214,989 | Kind, identity and page number |
| Leaf entry hashes/framing | 50,173,552 | 88 × logical pages |
| Physical node headers | 2,246,752 | 32 × physical pages |
| Branch child references | 5,616,800 | 80 × (physical pages − 1) |
| **Total** | **601,936,928** | Exact sum |

Rendered strings are 78.211% of payload. Their distribution is:

| Fact family (visible and hidden traversal rows as stated) | Rows | UTF-8 bytes |
|---|---:|---:|
| References (visible + traversal) | 504,848 | 117,543,249 |
| Uses (visible + traversal) | 500,813 | 114,807,671 |
| Types and locals | 733,374 | 79,953,099 |
| Metadata | 159,876 | 56,661,083 |
| Signature and outline | 232,976 | 35,442,727 |
| Callers and callees | 406,296 | 26,811,721 |
| Item text | 82,580 | 20,879,398 |
| Dependents | 262,799 | 16,036,491 |
| Instances | 8,418 | 1,504,938 |
| Reachable authority | 8,418 | 826,228 |
| Layout | 941 | 214,031 |
| Implementations | 994 | 101,839 |
| **Total** | **2,902,333** | **470,782,475** |

Visible references, visible uses, types and metadata alone account for 77.756%
of rendered bytes. In [semantic_facts.lkj](../src/semantic_facts.lkj),
each source reference initially emits a visible and traversal row for
references, uses and dependents. Source locations and identities are formatted
repeatedly; sort/dedup occurs only after all facts have been constructed.
Call-site and binding IDs repeat their body epoch in rows and logical keys.
Every checked local, including temporaries, receives a rendered type row.
The table's type family contains both source and checked facts; it is not a
standalone count of IR locals.

An item contributes metadata, signature/outline, text and type facts. Signature
and outline intentionally duplicate the same signature text under different
questions. Checked call edges likewise have instance and source mappings.
Authority propagation stores finite effect summaries and direct call edges;
it does not materialize all-pairs transitive callers/callees. Radix branches
store child hashes, not copies of all descendant payloads. The full checked IR
is serialized once for the snapshot digest, not once per instance.

### Budget and collateral bookkeeping

The separate [namespace census](compiler-index-pages-census.pl) reads file
sizes and the first 90 bytes. It makes no authentication claim; the rooted
payload comparator supplies authentication for pages/root/locator.

| Namespace or total | 10k files | 10k stored bytes | 100k files | 100k stored bytes |
|---|---:|---:|---:|---:|
| semantic-page | 6,890 | 82,321,436 | 70,211 | 608,255,918 |
| semantic-root | 1 | 466 | 1 | 466 |
| semantic-latest | 1 | 466 | 1 | 466 |
| semantic-locator | 1 | 194 | 1 | 194 |
| .budget-index | 1 | 1,594,056 | 1 | 16,221,207 |
| Other regular files, including empty lock | 8 | 2,704,746 | 8 | 26,824,043 |
| **Total** | **6,902** | **86,621,364** | **70,223** | **651,302,294** |

Both producers have these exact counts and sizes. The 100k census's first-header
reads total 6,319,980 bytes, of which 6,318,990 are semantic-page headers.
`Store::writable` authenticates/decodes the budget index and visits indexed
entry headers on first write. Therefore an indexed store introduces a large
bookkeeping population for a later ordinary write even if that write does not
request a new index. This experiment does not time that later edit, and does
not assign a measured fraction of the old body-edit store-I/O timer to it.

The high rejected/evicted counters are not a count of lost authenticated pages:
all 70,211 rooted pages are retained and authenticated here. The current
`cache_file` path can create an empty file on a miss, which is subsequently
rejected/erased. Physical file census and rooted authentication distinguish
that behavior from budget eviction of a valid generation.

## Change and correctness boundary

[semantic_store.lkj](../src/semantic_store.lkj) now keeps immutable
`Entry { name, hash, record, start, end, next, value_bytes }` descriptors
and the sorted fact vectors. `node` borrows those vectors and calls the
unchanged logical-page encoder only when writing a radix leaf. Size accounting
uses `24 + Σ(8 + UTF-8 byte length)`, so partitioning and ordered leaf bytes
are unchanged. Records are dropped after the root has been constructed.

The live encoded logical value is at most 16,384 bytes, alongside a radix-leaf
buffer at most 32,768 bytes. These are payload bounds, not an RSS bound.
The old implementation consumed and dropped each fact vector while encoded
values accumulated; the new implementation retains all facts through
`node`. It trades allocation lifetimes rather than eliminating two
simultaneously live complete copies. Allocator pool retention can affect RSS.
The experiment establishes the two measured peaks, not a universal memory
bound or an exact allocator-level decomposition of their difference.

There is no wire-format, trust, source/IR level, admission, snapshot, cursor,
page-retention or output change. Source facts remain claims and checked facts
retain their existing checked-IR provenance.

Correctness evidence has two layers:

- The test-only frozen eager publisher and production lazy publisher run in
  one executable with one producer identity. Empty records, empty strings,
  the maximum admitted single fact, 16 multibyte lines, radix branching and
  long UTF-8 text/signatures yield the exact same root, Snapshot identity and
  70 complete replies across nine kinds, including continuation cursors
  (1,626 facts). This oracle passed in 24.50 s, load 8.06→9.62.
  Production contains no duplicate eager algorithm.
- Both measured rungs emit byte-identical native images and exact rooted
  page payloads. Producer-specific envelopes authenticate independently.
  Snapshot identities differ because producing executable identity is part
  of their preimage; source, checked-IR identity, selector, root and fact count
  match. Ten declared questions per rung match every stdout byte after
  independently verifying and normalizing only the header's producer and
  snapshot fields. All twenty paired questions are complete within their
  declared bounds. The full payload comparisons took 1.30 s at 10k
  (load 9.16→9.16) and 9.55 s at 100k (load 10.53→9.90).

The 100k native-image SHA-256 is
`b975f191ca7adc58b77c84d4436f968f9b03a52ec418eb584fe64c3a505e2016`.
The shared 100k page root is
`c91db2d36600b732846d472aebb8b6aa924faf60352922f3ded69d0dc042019d`.
Full pins for both rungs are retained in their payload-oracle JSON outputs.

## Checked edit and retained refusals

All 27 retained production-language line changes (20 added, 7 removed) were
made by three checked replace-item operations. Request SHA-256:
`3f2f690873776d7658892d46a690f4a826cf50ebacb8e03f4e205a8a36aeb6cd`.
The complete before/after monitored-source manifests and request/output remain
under `compiler-index-pages-data/operation/`.

`src/semantic_store.lkj` before:
`5cf9b7ed04ddc1c12d5f49f46da760de5d5fa44ecb17612697f5076b5751bb1e`;
after:
`6e8fe0882e0a3cebb8a480be9231bc0f1393f67d053c9f04a05402f27d304b34`.

| Preparation or oracle attempt | Result | Wall s | Peak KiB | Load |
|---|---|---:|---:|---|
| Checked three-operation apply | Passed full candidate check and publication | 52.93 | 1,106,056 | 7.76→9.64 |
| Candidate native build | Passed | 41.00 | 1,048,008 | 10.12→10.09 |
| Oracle attempt 1 | Fixture omitted library required by embedded includes | 1.16 | 106,572 | 13.38→13.38 |
| Oracle attempt 2 | Fixture inferred an unsized text vector; explicit owned annotations added | 20.59 | 659,744 | 11.40→11.54 |
| Oracle attempt 3 | Fixture moved a reply while its cursor slice was borrowed; scope corrected | 24.58 | 669,632 | 10.60→10.23 |
| Oracle attempt 4 | Exact root/Snapshot and all 70 replies passed | 24.50 | 694,044 | 8.06→9.62 |

The three refusals concern test fixture setup/type/borrow checking; each original
output is retained. They did not partially apply a production source change.
No timed measurement condition was retried.

The remaining cost is explicit: global catalog/fact materialization, long
rendered identities and locations, hundreds of megabytes of unchanged stored
facts, and budget bookkeeping over tens of thousands of pages. A future
representation change would need its own compatibility and trust proof. This
patch does not alter those contracts or infer a larger rung's behavior.
