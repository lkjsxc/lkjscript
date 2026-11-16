# Current compiler query check after container integration

Status: **prepared, not executed**. This plan does not authorize a compiler process.
Execution requires the root's explicit release after the combined container/generated-origin gate and a committed identity addendum naming the actual canonical executable and exact source revision. No values from the earlier v2 or generated-rung cohorts are replaced. The question commands and reply bounds remain comparable, but different integrated source and producers do not support a causal latency-speed comparison with historical v2.

## Question and acceptance boundary

Measure the fourth note's bounded compiler-query requirement on one integrated compiler snapshot. The population preserves the historical 20 owner-table questions and their 20 paired slices, followed by the existing ten supplemental requests that cover the remaining question kinds. There are **50 requests**, each executed **once** in the order below. No query is used to discover substitute identities, warm metadata, or choose a faster case.

The primary latency field is the exact integer nanosecond subprocess-wrapper interval, compared directly with **100,000,000 ns**. GNU time wall/user/system/RSS and the compiler's `total_ms`, `snapshot_auth_ms`, `semantic_query_ms`, and `producer_bytes_hashed` are separate fields. Report both whole-command misses and phase values; a fast internal phase cannot erase a slow whole-command observation. Wrapper launch, resource-limit wrapper, timeout, and stdout/stderr capture are included. Command/cwd/load/checkpoint metadata is written before the clock starts; subsequent hash and validation work is excluded.

A result is counted as an answering observation only if the requested identity/kind/depth, explicit pin, actual producer, source and checked-IR identities, v3 header/continuation framing, byte/fact bounds, recorded fact labels, and exit status validate. Empty/missing or partially supported answers remain visible and are not relabeled as complete query coverage. Available continuations are retained; no continuation is followed in the timed population.

## Required prelaunch identity addendum

Before execution, record and commit:

- Gated source commit and exact `src`, `library`, and relevant runtime tree/blob identities.
- Canonical producing/reading executable SHA-256 and byte count, including its actual construction configuration and gate/seed provenance. Copy it to an owned immutable fixture and verify the copy before use.
- Full frozen source/runtime/library inventory, archive command, source grant paths, query selector, and harness/validator hashes.
- Exact build and query argv and working directories, process/resource controls, free-space result, output/evidence destinations, and the fixed question list hash.
- Actual current v3 header fields and the unchanged limits below. Any necessary prelaunch schema adaptation is recorded before commands run; it cannot select new questions or replace a failed observation.

The executable used for every command is the same frozen canonical producer. The source archive is the exact revision that producer's integrated gate checked, without historical filename projection, source rewriting, or an alternate library. Preserve the canonical compiler factory's exact source/library/runtime selection and diagnostic naming, recorded in its construction argv. The owned copy must use those same explicit selections with narrow grants; any path-dependent byte mismatch is retained as a setup/proof failure.

## Setup and isolation

Use one compiler worker after explicit release. Create an owned fixture with separate source, store, output, and executable directories. Archive only the declared `src`, `library`, `tools`, and `tests` trees needed by the compiler root. Do not copy `.generation-one`, live data, other projects, or main's mutable store.

Create the private 32-byte key in a distinct owned directory outside every source/library/runtime grant. Run compiler commands with cwd exactly the owned archived source root, relative entry `src/main.lkj`, explicit `--library-dir library`, and explicit `--runtime-source library/runtime.lkj` for builds. Omit `--source-dir` consistently for builds and queries. For this relative entry, `compile_snapshot` grants exactly `.`; it does not grant `/`. Store, output, and executable directories are siblings of the source root, and the private key is outside all source/library/runtime grants. Never invoke an absolute entry without a source grant.

This is a prelaunch clarification of the initial absolute-source-dir plan. `diagnostic_source_path` prepends any supplied source-directory spelling; `elaborate::source_paths` preserves it and `native::diagnostics::data` embeds it. Even `--source-dir .` adds a diagnostic `./` prefix. An absolute source-directory option therefore cannot preserve the canonical factory's image bytes. The approved narrow-cwd/relative-argument recipe preserves both the existing naming and its narrow `.` grant without changing production code.

Preflight at least **6 GiB** free before archive/store creation. Record the measured archive/executable sizes and exact available bytes. Store writer capacity is **2 GiB**. Each build has **4 GiB address space and 900 s**, plus a 10 s termination grace. Each query has **2 GiB address space and 30 s**, plus a 10 s grace. Queries use the default reader-cap setting rather than inheriting the writer's 2 GiB cap; per-extent read admission must suffice.

A setup/resource refusal is a result. Preserve it and stop dependent commands. Do not raise a limit, substitute a producer, rebuild a pin, or retry a timed request in this experiment. If a setup/build fails, the unexecuted request suffix is explicitly not run. If an individual query fails but the immutable fixture remains intact, continue the predeclared remaining queries once each, recording every failure. A missing/corrupt pin is not repaired during the query phase.

Retain fixture/store/key/images after completion until root audit acceptance. Commit text evidence only. Cleanup is a separate, owned-path action after acceptance, with process/ownership checks; do not remove an interrupted fixture while a command can still refer to it.

## Fixed warm sequence

1. Record source/producer/harness identities and resource/free-space preflight.
2. Run exactly one indexed build into an empty dedicated store: `build src/main.lkj --library-dir library --runtime-source library/runtime.lkj ... --index --inline-limit 64 --stats`, with `LKJSCRIPT_STORE_IO=1`.
3. Run exactly one unchanged indexed build with the same arguments and store.
4. Validate the two captured build replies without a metadata query: snapshot/source/checked-IR IDs and both output images must agree; the indexed image must also be byte-identical to the gated canonical executable, and the unchanged build must report genuine image reuse. Record any retained store writes; do not silently assume a zero-write invariant that a changed current retention contract does not guarantee.
5. Execute the 40 owner-core requests in paired order, then the ten supplemental requests. There is no full-map walk, store census, metadata prequery, old-reader oracle, or benchmark query before these 50 observations.
6. Only after all requests, validate captured outputs and summarize them. An optional static namespace/extent census uses the already frozen independent metadata decoder under the same resource limits; it is labeled post-measurement and cannot warm a timed reply. It does not authenticate all node payloads.
7. Verify the source and executable inventories are unchanged, preserve raw evidence, and report the exact observed coverage.

Builds naturally write/read the index, and the unchanged build authenticates its retention metadata. The operating-system file cache is not flushed. The report must state this actual warm sequence; it must not call these queries cold reads.

Both indexed compiler images are generated at **inline limit 64**, matching the canonical compiler factory, and are compared byte-for-byte with the eventual gated canonical executable. The historical v2 query cohorts used **inline limit 0 for the indexed output image**. This is an explicitly declared configuration difference; it can change emitted native-size facts and construction costs. Together with the changed source and producer, it rules out treating the new latency table as a causal before/after speed comparison. The unchanged command identities and bounds support a current contract check.

## Owner-core population: 40 commands

Use depth **1**, fact limit **96**, total reply byte limit **32,768**. For each row, execute the original question first, then one `slice` for the same identity. No duplicate is removed after seeing results.

| Pair | Original kind | Identity | Second command |
| --- | --- | --- | --- |
| 1 | outline | `driver::Options` | slice |
| 2 | outline | `driver::Memo` | slice |
| 3 | item | `driver::compile` | slice |
| 4 | signature | `driver::execute_source` | slice |
| 5 | signature | `driver::read_path` | slice |
| 6 | signature | `driver::diagnostic_source_path` | slice |
| 7 | outline | `ir::TypeKind` | slice |
| 8 | outline | `ir::Op` | slice |
| 9 | outline | `ir::Function` | slice |
| 10 | outline | `ir::Program` | slice |
| 11 | outline | `ir::Local` | slice |
| 12 | outline | `ir::ReturnSource` | slice |
| 13 | outline | `semantic::Catalog` | slice |
| 14 | outline | `semantic::Item` | slice |
| 15 | outline | `semantic::Reference` | slice |
| 16 | outline | `reuse::AuthenticatedRoot` | slice |
| 17 | signature | `test_runner::closed_contracts` | slice |
| 18 | signature | `source::load_report` | slice |
| 19 | outline | `native::PreparedImage` | slice |
| 20 | authority | `authority::analyze` | slice |

This is the list in `bench/compiler-query-v2.sh`, not a new selection of convenient current names. If a selected identity has changed or disappeared in the integrated source, retain that outcome.

## Supplemental population: ten commands

These retain the existing `bench/compiler-query-kinds.sh` question identities/order. Use depth **2**, fact limit **96**, total reply byte limit **32,768**. They cover the uses/call graph/implementation/instance/type/layout/dependent/text requests absent from the owner-core list.

| Ordinal | Kind | Identity |
| --- | --- | --- |
| 41 | uses | `driver::Memo` |
| 42 | callers | `driver::compile` |
| 43 | callees | `driver::execute_source` |
| 44 | impls | `store::QueryAccess` |
| 45 | impls | `store::QueryReader` |
| 46 | instances | `semantic_store::open` |
| 47 | type | `driver::Options::field:store_bytes` |
| 48 | layout | `driver::Options` |
| 49 | dependents | `semantic_store::Snapshot` |
| 50 | text | `driver::compile` |

## Validation and complete evidence

For each build retain exact argv/cwd/environment, status/stdout/stderr, start/end wrapper nanoseconds, GNU time values, all load averages before/after, compiler statistics including optional Store subcounters, output image hash/bytes, and all emitted snapshot/source/IR pins. Require the Store I/O summary for these two indexed builds. Preserve its disjoint counter semantics: indexed publication may lie within `semantic_index`, and the exclusive Store sum is not identical to the driver's `store_io` interval.

For every query retain raw stdout/stderr, status, exact wrapper nanoseconds, GNU time values, phase statistics, all three load averages before/after, actual question/pin, stdout digest and bytes, fact count, available continuation, pages read, traversal steps, and observed fact levels/kinds. Validate the entire header against the exact pin/producer/selection. Require `producer_bytes_hashed=0`; an unexpected hash is a recorded protocol/cost failure, not a reason to discard timing.

Validate the final continuation record is unique and framed correctly, all emitted fact rows account for its count, and the complete encoded output including header/cursor fits 32,768 bytes. Keep generated source claims explicitly labeled; authority/layout/ownership facts retain their checked-IR levels. Unsupported or unresolved source coverage remains visible. No generated definition is counted as editable text merely because it has a definition anchor.

Use no cross-format or cross-producer equality oracle here: the integrated source has intentionally changed, and historical reply bytes are not expected to match. The current gate's existing parser/authentication/extent/cursor tests are separate correctness evidence. Postprocessing is restricted to the captured outputs and recorded source/provenance metadata, with no timed question reruns.

Publish every observation and every refusal, plus counts under and at/over 100 ms for the 40-core, ten-supplemental, and complete 50 populations. Keep failures and missing/partial answers separate from successful latency counts. Report min/median/max only beside the full per-question table; do not erase the original 207.807/102.204 ms v2 outliers, later cache results, or the generated-rung measurements. This is one new integrated-source cohort, not a retroactive replacement.
