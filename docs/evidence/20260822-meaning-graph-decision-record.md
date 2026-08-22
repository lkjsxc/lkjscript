# Meaning-graph cutover decision record

Date: 2026-08-22 UTC. Starting Git commit:
`6747754bdcad4dc9000c3d7891db7ef207c8ec2f`. Campaign prompt:
`prompts/202608220253.md`. Root-policy SHA-256:
`642ea3f96a3801df4453efac178c58933eb9d0f6d457d0474b8f3528e40738aa`.

This record retains conclusions, alternatives, prototype observations, deletion consequences, and
reversal gates. It is historical evidence for the meaning-graph-1 campaign endpoint, not program
authority or current graph-4 documentation. Words such as “current” below refer to that recorded
endpoint unless a paragraph explicitly says otherwise.

## Reproduced starting point

The baseline maintained UTF-8 `.lkj` modules and `lkjscript.package.json`, with accepted source
history under `.lkjscript/source-v1`. Durable declaration ownership was name based. Public
lkjournal orientation emitted 1,356 bytes, service-module show emitted 40,322 bytes, and package
test emitted 311 bytes for 11 differential tests. Initial focused verification passed 5/5 at
`.artifacts/check/20260822T020817.956731Z-263295/receipt.json`; initial product verification passed
11/11 at `.artifacts/check/20260822T021226.484873Z-266884/receipt.json`.

The previous graph engine had preserved valuable exact publication and semantic-query properties,
but one exact-function query emitted 37,897 bytes, one orientation took about 998.5 ms, and its
full check took about 730.6 seconds. The direct cutover therefore retained graph authority while
rejecting its whole-graph payload and product-shaped mechanics.

## Foundational selections

| Question | Serious alternatives evaluated | Selection and observed reason | Direct deletion | Reversal gate |
|---|---|---|---|---|
| Logical granularity | monolithic typed root; one record per expression; editable text plus derived graph; semantic module tables | stable owner graph with packed module-local typed tables and canonical cross-owner relations; current consumers align with module locality without per-expression file/object overhead | editable text and dual authority | reconsider when one module dominates context/storage or cross-module edits rewrite disproportionate data |
| Stable identity | path/name tuples; content digests; coordinated counters; typed random IDs | distinct tagged 128-bit random semantic domains; deterministic domain-separated allocation only for tests/migration | name-derived ownership and cross-domain byte reuse | reconsider expression-site granularity if IDs dominate storage/merge without improving selection |
| Revision topology | linear counter; Git commit identity; operation journal; immutable DAG | content-derived revision DAG with exact parent record bindings and one atomic HEAD | source revision counters/index | retain while two-parent history remains independently reconstructable and bounded |
| Draft authority | invalid accepted graphs; ordinary accepted branch revisions; temporary files; typed operation delta | separate packed non-executable draft bound to one accepted base, with typed holes/conflicts | holes or conflicts in accepted state | add persistent merge-conflict drafts when current closed result is insufficient |
| Physical repository | embedded database plus export; append-only journal; one giant snapshot; canonical sharded tables; current snapshot plus history | hybrid immutable content-addressed packed module/root/revision/receipt tables plus small HEAD and derived index shards | `.lkjscript/source-v1` and predecessor graph stores | revisit when local edit or history/file count exceeds impact-proportional cost |
| Publication | whole-directory replacement; embedded transaction; append journal marker; immutable objects plus HEAD compare-and-publish | immutable unreachable objects, complete validation, filesystem durability, then synchronized atomic HEAD; Linux `syncfs` batches object durability | source manifest/record/HEAD publication | reverse batching if fault injection or filesystem evidence violates old/new-complete observation |
| Indexing | no indexes/full scans; one full index; always-resident daemon; broad plus local shards | full revision-bound relation index plus 256-way owner/name local shards; canonical reconstruction is oracle | source path/name scans as ordinary query | revisit if warm exact p95 exceeds 250 ms or shard bytes/complexity exceed measured gain |
| Incremental computation | full rebuild; timestamp cache; stable-owner invalidation | stable relation indexes are incremental query acceleration, but accepted mutation still performs full candidate validation and packed reconstruction | timestamp/source caches | this is incomplete: introduce exact dependency invalidation only with full-oracle sequence equivalence |
| Transaction request | raw node patches; generated source patches; natural language; high-level closed JSON | strict versioned JSON with exact base, typed IDs, ordered operations, preconditions, idempotency, and budgets | source apply and raw storage mutation | compact encoding may be added only if descriptive JSON becomes material provider/context cost |
| CLI response | full graph/schema; ad-hoc text; JSONL for every call; bounded strict JSON | one strict JSON envelope; selected fields; continuations; at most 64 inline affected owners; out-of-band deterministic files | unbounded module/graph output | revisit short handles or alternative encoding only with equal-task evidence |
| Query language | SQL over storage; host scripting; command-only fixed queries; closed declarative query | closed versioned selections/traversals with exact revision, deterministic order, budgets, continuation, and indexed/oracle paths | raw storage inspection | extend only for a named workflow without exposing physical layout |
| Text projection | regenerate editable source; round-trip source import; no text; non-authoritative deterministic review | span-free deterministic JSON review projection with no apply/import path; graph backup is the recovery oracle | maintained `.lkj` and source grammar as production input | reconsider a recovery grammar only if independent disaster-recovery value exceeds its security/maintenance cost |
| Git collaboration | line merge as authority; Git-object IDs as semantic revisions; custom remote database; Git transport plus semantic merge | Git transports immutable objects; stable-ID diff and exact-base three-way merge own meaning | compatibility line merge/readers | add PR automation only after branch allocation and conflict cases have public black-box coverage |
| Compiler boundary | render and parse text; keep source-derived AST; direct graph lowering | graph-native exact-closure artifact, direct preparation, bytecode production tier, independent semantic interpreter | production parser/compiler source handoff | retain bytecode only while differential equality and complete-workload value hold |
| Migration | permanent dual readers; compatibility edition; private maintained builder; one-time conversion and deletion | one-time test-backed source conversion produced graph artifacts; maintained stores were re-rooted from normalized graph snapshots; converter and source authority were deleted | descriptors, modules, source stores, migration binary | future graph contract changes use another exact direct cutover, never fallback |
| Receipt reuse | no receipts; receipts as graph meaning; unconditional pass cache; bound evidence records | transaction receipts are accepted history records; checker receipts are bounded evidence; final full check is fresh and pass reuse is absent | verbose passing logs | introduce discovery only when every semantic and operational input is bound |
| Process topology | mandatory daemon; embedded service database; per-command correctness; memory map | stateless per-command correctness with disposable disk indexes; no daemon owns meaning | source watch/ambient state | retain daemon only after complete-workflow warm latency improves materially and stale detection is exact |

## Physical prototype evidence

The initial content-addressed module design synchronized every new module separately. A public
10,000-module transaction took 20.523 seconds. The selected Linux durability path closes all
unreachable immutable files, calls `syncfs` once, then publishes the separately synchronized HEAD;
the identical transaction and revision digest took 0.462 seconds. The portable path retains
per-file synchronization.

The initial exact-query cache loaded one full 16 MiB index at 90,000 modules despite semantic work
count 1. Revision-bound local shards reduced final cold exact lookup to 0.630 seconds and warm exact
lookup to 0.138 seconds, still with work count 1. The local cache is derived and can be deleted or
corrupted without changing authority.

Final scale command:

```sh
tools/semantic-scale --modules 90000 --batch 10000
```

On Linux x86-64 with rustc/Cargo 1.96.0, nine 10,000-module publications took 0.469, 1.013, 1.798,
2.699, 4.006, 5.332, 7.007, 8.875, and 10.949 seconds. Each final response was 5,378 bytes. Deep
doctor checked 10 revisions and 450,120 retained module versions in 3.661 seconds. Build took 1.269
seconds and produced 10,370,515 bytes. Backup took 3.995 seconds and produced 44,315,558 bytes.
Canonical store size was 41,163,949 bytes; including derived indexes it was 82,844,955 bytes.

The increasing publication curve is attributed to complete graph reconstruction/validation and
checking retained immutable objects. It is recorded as an unimplemented incremental-engine limit,
not hidden by larger timeouts. Graph contract 1 caps one root at 100,000 modules, so a one-million
module run rejects under current resource policy and was not performed.

## Historical migration result and absence

Standard retained package ID `10000000000000000000000000000001`, 12 modules, and 6 tests.
lkjournal retained package ID `20000000000000000000000000000001`, 3 modules, 2 targets, and 11
tests. The exact standard dependency, HTTP and worker component requirements, routes, SQL,
authorization, objects, queues, and deterministic adapters survive through graph artifacts.

Current source stores, package descriptors, `.lkj` modules, source publication, name-tuple durable
identity, source-era artifacts, and migration-only executable paths were deleted. The test-only
parser/semantic builder remains an implementation-disjoint fixture oracle and has no public
maintained-project reader or writer.

## Economy limitations

Current-project release measurements and byte counts are recorded in `docs/performance.md`.
Provider input, cached input, output-token, request/retry, and monetary telemetry was unavailable.
No token or money reduction is inferred from response bytes. The equal tasks measured here cover
orientation, exact lookup/show, package testing, graph mutation, build, doctor, review, and backup;
the campaign did not reproduce every Appendix B application edit before source deletion.
