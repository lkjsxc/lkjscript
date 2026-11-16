# Current compiler queries on the gated container/origin producer

**All 50 predeclared bounded requests returned valid, nonempty recorded facts
below 100 ms.** The exact wrapper minimum/median/maximum was
**2.729156 / 4.461788 / 60.812180 ms**, with load
**1.88/1.50/2.19 -> 2.13/1.56/2.21** over the question segment.
Four replies carried continuations; they were retained and not followed.
This is a result for the fixed request population and explicit snapshot below,
not an exhaustive proof of source-reference coverage or arbitrary query latency.

The original harness stopped after its two successful native builds because of
an offline validator defect. No question ran in that attempt. A separately
reviewed, committed and released continuation then executed each of the original
50 questions for the **first time, once**. Both successful build measurements
and the failed original validation remain unchanged. The pause and its cache
qualification are part of this result.

## Identities and protocol

| Record | Value |
| --- | --- |
| Gated source | `62fa76f4f863a169b906094f0dd9998c8421e77a` |
| Source tree | `00d1109fcdc49041d13d7406ca74c8f5b4394e8d` |
| Library tree | `25bdb8ff8567b941790d3951c78e896b23e26e8e` |
| Runtime blob | `2fed59cd33d8a2ab80d562f160648b05f24b8c4c` |
| Producing/reading executable SHA-256 | `34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377` |
| Executable and each indexed image | 32,196,802 bytes; direct byte comparisons passed |
| Snapshot | `9ff914ad9f4ef15c9bc5c61cb5d0718abcfde00322243a7a52a73f6b08d3534e` |
| Source snapshot identity | `d01c7eb47d92c7aefc7490b83f9bbc05025f6197871a7f90c51dfcad1576bb2b` |
| Checked IR identity | `4613bd709ee1d9238dca31e51b25d091abcd21a77f018090fedb8db8032c0fa2` |
| Actual identity addendum commit | `c17b7ded308b5db7848db7b22c38d891e31bd14c` |
| Continuation preparation commit | `fc0147a6ceef94a3aea9eee6729205130b690393` |
| Question list SHA-256 | `417a4adfe357aab04915c5cdee3ebe2f691ee46ca2fb74663735e5d35a6e81e9` |
| Query limits | 96 facts, 32,768 complete response bytes; core depth 1, supplemental depth 2 |
| Resource limits | Builds: 4 GiB/900 s; queries: 2 GiB/30 s; 10 s termination grace |
| Store | Dedicated 2 GiB writer budget; queries retain the default per-extent read cap |
| Factory and indexed image policy | Inline limit 64 |

The canonical factory's exact source/library/runtime selectors and cwd, symbolic
temporary-path qualification, gate evidence and original 1,860-file inventory
are in [the identity addendum](compiler-query-current-identities-62fa76f4.md).
The random extracted-seed path was not recorded by bootstrap; no literal path
has been invented. The new indexed-image comparisons establish actual byte
identity with the retained gated executable.

The source archive contains only the frozen `src/library/tools/tests` trees.
Compiler cwd was the archive root, with relative `src/main.lkj`,
`--library-dir library`, `--runtime-source library/runtime.lkj` for builds,
and no `--source-dir`. The source grant was only `.`; executable, output and
store were siblings, and the private key was outside all source grants.

## Builds, the recorded stop, and continuation

| Build | Exact wrapper ns | GNU wall / user / system s | Peak RSS KiB | Reused image | Store writes | Load before -> after |
| --- | ---: | --- | ---: | ---: | ---: | --- |
| Cold indexed compiler | 109215156001 | 109.21 / 105.98 / 1.22 | 2554260 | 0 | 10 | 2.39/2.98/3.46 -> 2.24/2.83/3.36 |
| Unchanged indexed compiler | 691635177 | 0.68 / 0.65 / 0.00 | 44732 | 1 | 0 | 2.24/2.83/3.36 -> 2.24/2.83/3.36 |

Both native commands exited 0. Their snapshot records are byte-identical, and
both images directly compare equal to the gated canonical executable. Cold
statistics record 3,654 definitions, 12,793 IR functions and 1,240,173
instructions. Within that cold command, catalog construction was 8,336.727 ms
and index construction 63,922.057 ms; the command's load pair is the cold row
above. The cold reported retained-store counter was 751,986,967 bytes, not an
independent physical-store census.

The unchanged path reported no frontend/index construction timings or IR counts.
Those cells remain **NA**. Its zero retained-store counter is not evidence that
the store became empty. Its image reuse, zero writes, matching snapshot and
matching output image are separately observed.

The original validator incorrectly required cold frontend/index fields on that
cached path. Its shared JSON encoder was also switched into pretty mode by
`save()`, causing a later TSV error. Both failures and the original harness's
status 1 are frozen in [the first-attempt evidence](compiler-query-current-data/attempt-1).
The original 35-file manifest remains
`ab1b3b95fe8f14cd0202eaa6ffeeb01a274a43afd08102b4bb75ef252d91f963`.

The corrected validator uses common build requirements plus cold-only fields,
an independent pretty encoder for saved JSON, and an optional pinned-query
`producer_hash_ms` field. Missing hash timing stays **NA**; the mandatory
`producer_bytes_hashed=0` check remains. Two offline validations of captured
records passed without running a compiler or query. Their exact code and outputs
are retained separately; they do not replace the failed original validation.

The original two builds were **not repeated**. The first question began
**779.069631557 s** after the unchanged build ended, with load
**2.24/2.83/3.36 -> 1.88/1.50/2.19** across that interval. During the pause,
reviewers inspected text evidence, inventories and native images, corrected the
offline validator, and root cleaned seven unrelated repair artifacts. No query,
map walk, census, store repair or cache flush preceded the timed question set.
Operating-system cache state and host load could change during this pause.

The fifty-command segment spanned **1.352031809 s**, load
**1.88/1.50/2.19 -> 2.13/1.56/2.21**, from the first wrapper start to the last
wrapper finish. That segment includes between-command bookkeeping; each request's
acceptance uses its own exact wrapper interval below. The declared sequence is
cold index, unchanged index, validation stop and inspection pause, then the
separately released first-time questions. It is not described as uninterrupted.

## Query outcome and coverage

| Population | Valid replies / commands | Under 100 ms | Min / median / max wrapper ms | Load before -> after |
| --- | ---: | ---: | --- | --- |
| 40 owner-core and paired slices | 40/40 | 40 | 2.729156 / 4.8105645 / 60.81218 | 1.88/1.50/2.19 -> 2.13/1.56/2.21 |
| 10 supplemental | 10/10 | 10 | 2.765646 / 3.758419 / 22.557347 | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| All 50 | 50/50 | 50 | 2.729156 / 4.461788 / 60.81218 | 1.88/1.50/2.19 -> 2.13/1.56/2.21 |

All 50 commands exited 0 and passed the exact producer/snapshot/source/IR,
identity/kind/depth, v3 header and continuation, byte/fact bounds and label checks.
All report `producer_bytes_hashed=0`; absent `producer_hash_ms` values remain
NA. No request was retried or substituted, and no continuation was followed.

The internal `semantic_query_ms` minimum/median/maximum was
0.714 / 2.298 / 57.565 ms; the question segment load pair is
1.88/1.50/2.19 -> 2.13/1.56/2.21. The per-request snapshot-authentication values
and loads are retained in the full table. These phase timers are distinct from
the whole-command wrapper interval and cannot replace it.

There are 44 source-claim-only replies, five checked-IR-only replies, and one
reply containing source claims plus protocol notices. Authority, callers,
callees, instances and layout appear at the checked-IR level in the selected
requests. Declared field type and source relationships retain their source-claim
level. The dependent query includes two `empty-indexed-result` notices alongside
23 substantive source facts; it is not an empty response. Those 23 recorded
dependent facts are not an exhaustive proof of all dependents.

The automated coverage category is `nonempty-recorded` for all 50, with zero
observed unavailable or partial-marker rows. This label does not claim exhaustive
reference resolution: every header still states recorded-site source coverage,
demanded-instance coverage, authenticated-producing-compiler validation,
`current-check=not-performed`, and `freshness=not-asserted`.

The 50 replies contain **844 reported fact rows** (including the two protocol
notices) and **233,757 bytes**, counting complete header/continuation framing.
The 20 owner questions alone account for **42,230 reply bytes**.

Replies contain 767–32,530 bytes, 1–96 facts, and report 5–274 pages. The 20
first-page slices contain 1,491–32,530 bytes, median 3,738 bytes. Two slices and
two supplemental replies are bounded prefixes with retained continuation:

| # | Request | Bytes | Facts | Reported pages | Wrapper ms | Load before -> after |
| ---: | --- | ---: | ---: | ---: | ---: | --- |
| 8 | slice `driver::execute_source` | 32294 | 96 | 267 | 60.812180 | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 36 | slice `source::load_report` | 32530 | 95 | 274 | 58.526448 | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 41 | uses `driver::Memo` | 14702 | 96 | 89 | 22.557347 | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 43 | callees `driver::execute_source` | 22338 | 96 | 31 | 11.428691 | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |

The first two slice responses therefore do not purport to contain every
dependency signature in one page. The recorded continuation is the explicit
remaining-work boundary.

## Complete request table

Every request below was executed once. All statuses are 0, all protocol checks
pass, and every wrapper interval is below the exact 100,000,000 ns threshold.
`More=yes` means the captured continuation was available and was not followed.
The whole response byte count includes header and final continuation framing.

| # | Kind | Identity | Wrapper ns | Query phase ms | Snapshot auth ms | Bytes | Facts | Pages | More | Load before -> after |
| ---: | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| 1 | outline | `driver::Options` | 4125645 | 1.985 | 0.084 | 4642 | 30 | 9 | no | 1.88/1.50/2.19 -> 1.88/1.50/2.19 |
| 2 | slice | `driver::Options` | 9324951 | 6.564 | 0.091 | 3590 | 9 | 33 | no | 1.88/1.50/2.19 -> 1.88/1.50/2.19 |
| 3 | outline | `driver::Memo` | 3743536 | 1.96 | 0.079 | 4281 | 26 | 9 | no | 1.88/1.50/2.19 -> 1.88/1.50/2.19 |
| 4 | slice | `driver::Memo` | 9711028 | 7.267 | 0.074 | 5084 | 11 | 43 | no | 1.88/1.50/2.19 -> 1.88/1.50/2.19 |
| 5 | item | `driver::compile` | 3866828 | 1.42 | 0.086 | 1664 | 3 | 9 | no | 1.88/1.50/2.19 -> 2.13/1.56/2.21 |
| 6 | slice | `driver::compile` | 18350870 | 16.145 | 0.082 | 6776 | 21 | 79 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 7 | signature | `driver::execute_source` | 3099815 | 1.157 | 0.088 | 889 | 1 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 8 | slice | `driver::execute_source` | 60812180 | 57.565 | 0.103 | 32294 | 96 | 267 | yes | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 9 | signature | `driver::read_path` | 4060813 | 1.004 | 0.162 | 830 | 1 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 10 | slice | `driver::read_path` | 13809392 | 11.62 | 0.083 | 3886 | 16 | 62 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 11 | signature | `driver::diagnostic_source_path` | 2952477 | 0.917 | 0.088 | 865 | 1 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 12 | slice | `driver::diagnostic_source_path` | 12859404 | 10.821 | 0.078 | 4519 | 14 | 55 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 13 | outline | `ir::TypeKind` | 4895414 | 2.826 | 0.084 | 5647 | 39 | 13 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 14 | slice | `ir::TypeKind` | 6364219 | 4.329 | 0.088 | 4099 | 6 | 20 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 15 | outline | `ir::Op` | 4086041 | 1.89 | 0.087 | 5738 | 30 | 9 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 16 | slice | `ir::Op` | 7994367 | 5.984 | 0.075 | 8478 | 12 | 32 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 17 | outline | `ir::Function` | 3440866 | 0.843 | 0.083 | 1654 | 8 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 18 | slice | `ir::Function` | 7040472 | 5.03 | 0.077 | 3092 | 8 | 32 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 19 | outline | `ir::Program` | 3214901 | 0.906 | 0.088 | 1179 | 4 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 20 | slice | `ir::Program` | 6289378 | 4.285 | 0.076 | 2275 | 6 | 23 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 21 | outline | `ir::Local` | 3129700 | 0.835 | 0.084 | 990 | 3 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 22 | slice | `ir::Local` | 4797931 | 2.643 | 0.08 | 1491 | 4 | 16 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 23 | outline | `ir::ReturnSource` | 2729156 | 0.714 | 0.082 | 1119 | 3 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 24 | slice | `ir::ReturnSource` | 5977651 | 3.734 | 0.087 | 2250 | 5 | 20 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 25 | outline | `semantic::Catalog` | 3257040 | 1.112 | 0.086 | 2927 | 15 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 26 | slice | `semantic::Catalog` | 9539354 | 7.013 | 0.088 | 4308 | 9 | 37 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 27 | outline | `semantic::Item` | 4074379 | 1.86 | 0.088 | 3269 | 20 | 9 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 28 | slice | `semantic::Item` | 6134476 | 3.946 | 0.086 | 2382 | 5 | 20 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 29 | outline | `semantic::Reference` | 3039411 | 0.76 | 0.098 | 1423 | 6 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 30 | slice | `semantic::Reference` | 4995142 | 2.611 | 0.091 | 1629 | 4 | 16 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 31 | outline | `reuse::AuthenticatedRoot` | 2874861 | 0.849 | 0.102 | 1473 | 6 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 32 | slice | `reuse::AuthenticatedRoot` | 4823198 | 2.796 | 0.074 | 1631 | 4 | 17 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 33 | signature | `test_runner::closed_contracts` | 3055191 | 0.747 | 0.076 | 830 | 1 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 34 | slice | `test_runner::closed_contracts` | 7288679 | 5.42 | 0.075 | 2796 | 8 | 31 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 35 | signature | `source::load_report` | 3220071 | 0.795 | 0.075 | 950 | 1 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 36 | slice | `source::load_report` | 58526448 | 56.275 | 0.075 | 32530 | 95 | 274 | yes | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 37 | outline | `native::PreparedImage` | 3295983 | 1.039 | 0.092 | 1093 | 3 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 38 | slice | `native::PreparedImage` | 7009583 | 4.793 | 0.11 | 1864 | 4 | 16 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 39 | authority | `authority::analyze` | 3149007 | 0.949 | 0.093 | 767 | 1 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 40 | slice | `authority::analyze` | 43618688 | 40.558 | 0.104 | 16799 | 63 | 198 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 41 | uses | `driver::Memo` | 22557347 | 20.192 | 0.095 | 14702 | 96 | 89 | yes | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 42 | callers | `driver::compile` | 11916920 | 9.097 | 0.103 | 3403 | 16 | 39 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 43 | callees | `driver::execute_source` | 11428691 | 8.179 | 0.109 | 22338 | 96 | 31 | yes | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 44 | impls | `store::QueryAccess` | 2902674 | 0.899 | 0.095 | 951 | 2 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 45 | impls | `store::QueryReader` | 3087571 | 0.991 | 0.115 | 921 | 2 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 46 | instances | `semantic_store::open` | 3840578 | 0.925 | 0.112 | 1135 | 2 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 47 | type | `driver::Options::field:store_bytes` | 2765646 | 0.834 | 0.087 | 779 | 1 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 48 | layout | `driver::Options` | 2844273 | 0.81 | 0.095 | 1035 | 1 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 49 | dependents | `semantic_store::Snapshot` | 12597020 | 10.355 | 0.095 | 3371 | 25 | 42 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |
| 50 | text | `driver::compile` | 3676260 | 0.891 | 0.109 | 1119 | 1 | 5 | no | 2.13/1.56/2.21 -> 2.13/1.56/2.21 |

The machine-readable [query TSV](compiler-query-current-results-62fa76f4/queries.tsv)
also retains GNU wall/user/system/RSS, total and phase times, exact fact levels,
fact kinds, traversal counts, response digests and errors. GNU time has
hundredth-second resolution here; its many 0.00 fields are not zero-cost claims.
The exact nanosecond wrapper is the latency criterion.

## Historical old-route byte comparison

The [original twenty-question baseline](compiler-query-baseline.md) remains the
evidence for the old text-reading route. Its **41,687 selected source bytes**
were an oracle lower bound from exact item spans in that historical snapshot,
excluding search/navigation/context and repeated reads. Its framed replies
totalled **47,370 bytes**, so that comparison showed **no aggregate byte saving**.
The documented 9,238-byte IR range retrieval was shared once across six
questions; it must not be counted six times.

The present 20 owner replies total 42,230 bytes on a changed compiler source.
This cohort did not remeasure the text-search route. Subtracting these current
replies from the old snapshot's span lower bound, or treating the historical
whole-file alternatives as independent per-question reads, would not establish
a current agent-context saving. The stable identities and requested bounds
preserve the question population; they do not make source bodies or reply bytes
identical across revisions.

## Evidence and interpretation

- Original attempt and failed validator: `compiler-query-current-data/attempt-1`
  plus original controls under `attempt-1-controls`.
- Separately frozen continuation:
  [addendum](compiler-query-current-continuation-62fa76f4.md),
  [checkpoint](compiler-query-current-continuation-62fa76f4.json),
  harness `compiler-query-current-continue.sh` and corrected validator
  `compiler-query-current-continuation-validate.pl`.
- Continuation raw/derived capture:
  `compiler-query-current-results-62fa76f4`, an exact copy of **380**
  produced text records, including all 50 argv/cwd/stdout/stderr/time/capture
  records and their offline validations.
- Final bundle inventory: `compiler-query-current-results.manifest.sha256`;
  exact raw-copy correspondence is retained under `compiler-query-current-result-controls`.
- Query TSV SHA-256:
  `21dd0f8bbe8d6806d58df6b7cb6306f4109ffbd59fa73d474b7b68710c23bea8`.
- Summary SHA-256:
  `c7aaf9418e51f8c8aa064f5a71a9b2d3201352f883b11bbc2d477a8b9af56a95`.
- Continuation ended at `2026-11-16T11:14:39Z`, status 0. The source
  path/digest mapping and executable identity remained unchanged. Raw inventory
  ordering is preserved; canonical comparison handles the differing sort order.
- Launches were `compiler-current50-launch-1068` and
  `compiler-current50-continuation-launch-1087`. Only the latter ran questions.

The wrapper includes the subprocess/resource launch, timeout wrapper, and
stdout/stderr capture. It excludes command/cwd/load metadata written before the
clock and subsequent hashing/validation. Both harnesses use the byte-identical
measurement function, SHA-256
`10a53d053791540b475640f7c3c962350ca81e60c1fb39eb1b6ad850cd52b966`.

This supports the below-100-ms contract for the fixed current-compiler question
set under the recorded conditions. It establishes neither size-independent
editing nor universal completeness of source claims. Building this index still
takes program-wide work. Source, producer, indexed-output inline policy and warm
sequence differ from historical v2; no causal speedup is assigned. The original
v2 207.807/102.204 ms misses and subsequent historical cache measurements remain
unchanged in their own reports.

The owned fixture, store, images and external key remain intact pending root's
live audit and explicit cleanup acceptance. No new source implementation was
changed for this cohort; the only repair was to the offline benchmark validator.
