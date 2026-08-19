# Performance and design evidence

Measurements here are observations, not semantic authority. Canonical raw corpus receipts are
[`20260819-lkjwork-functional.json`](evidence/20260819-lkjwork-functional.json) and
[`20260819-lkjwork-representative.json`](evidence/20260819-lkjwork-representative.json). The campaign
summary is `docs/evidence/20260819-lkjwork-campaign.json`.

## Observation environment

- Linux `7.0.0-29-generic`, x86-64, ZFS container root;
- AMD Ryzen 9 9955HX, 16 cores / 32 logical CPUs, 64 MiB L3;
- 32 GiB RAM;
- rustc `1.96.0` / LLVM `22.1.2`, Cargo `1.96.0`, optimized release binaries;
- sequential samples on a warm host unless a row explicitly says initialization; and
- one unrecorded warm-up followed by five one-shot samples for each retained query/mutation shape.

Page cache was not dropped and these are not cold-machine results. Timing uses monotonic
`perf_counter_ns`; stage counters use Rust monotonic durations. Process RSS, provider tokens, and
provider prices were unavailable.

## Frozen product corpora

| profile | tasks | core mutation requests | edges | notes | attachments | pure queries | final revision |
|---|---:|---:|---:|---:|---:|---:|---:|
| functional | 25 | 75 | 30 | 50 | 5 | 100 | 85 |
| representative | 500 | 2,500 | 1,000 | 1,000 | 100 | 2,000 | 2,700 |

Both use seed `lkjwork-corpus-v1` and the independent Rust reference-model tests. Final semantic
digests are stable across runs: functional
`c69cedb5b247818f4ab1490fe289ee7f6f97600d5f032ea4ba48cfde1fd8b27e` and representative
`1c0004ff62a19423f0ce564ce3688b1d61ccb04f9c50c35400b9d23c8261d9e2`.

The retained stress shape is 2,000 tasks / 10,000 mutations. It was not executed; no stress service
claim is made.

## Representative service results

Times are median / p95 milliseconds. Session rows include all retained corpus samples; one-shot rows
are five samples after warm-up.

| operation | session | one-shot |
|---|---:|---:|
| publishing mutation | 53.8 / 106.1 | 69.8 / 75.1 on five explicit post-corpus publications |
| unchanged mutation | — | 44.5 / 49.7 |
| show | 51.7 / 54.8 | 31.1 / 32.5 |
| list (20, priority) | 222.4 / 234.7 | 193.1 / 194.4 |
| next (10) | 65.6 / 68.9 | 42.4 / 43.0 |
| summary | 70.2 / 74.1 | 47.9 / 49.6 |
| context (10 tasks) | 82.8 / 87.2 | 59.0 / 59.7 |
| export page (20) | 59.1 / 62.0 | 33.9 / 34.7 |
| retained history page | 94.8 / 98.8 | 73.1 / 73.9 |

Initialization was 44.1 ms. One complete genesis replay/deep audit of 2,701 records took 33.8 s in
the frozen corpus; an independent post-corpus audit of revision 2,705 took 32.8 s and reproduced its
exact state. Attachments, including semantic suspend, host publication, and resume, were 258.0 / 275.8
ms median/p95. All ordinary query and mutation targets are met.

Before the product-open correction, representative one-shot application queries took 1.65–1.88 s:
the product locator performed a complete instance inspection before invoking the exact query owner.
Removing that duplicated traversal and using the HEAD-bound current manifest reduced complete
one-shot query medians by 89–98 percent, depending on query shape. This is a complete-workflow
comparison; no interpreter-only speedup is claimed.

## Storage

Representative authority contains 2,701 journal records:

| category | bytes |
|---|---:|
| immutable record chain | 102,841,790 |
| records carrying 64-revision checkpoints | 96,203,455 |
| HEAD-bound current manifest | 1,588,695 |
| application | 163,670 |
| attempts / outcomes | 35,100 / 107,000 |
| blob objects | 5,000 |
| complete project files | 104,741,804 |

A copied revision-2,700 representative project received one non-checkpoint label mutation. Revision
2,701 published a 2,254-byte journal record, a 1,589,052-byte replacement current manifest, and a
308-byte HEAD: 1,591,614 logical file-payload bytes. The retained tree grew by 2,611 bytes, giving a
609.58x logical-payload/retained-growth ratio. This is a canonical file-payload observation, not a
claim about physical ZFS blocks, device writes, or power-loss amplification.

The current public state is large enough that checkpoints dominate retained bytes, but the complete
project remains below the 256 MiB journal ceiling. There is no semantic-history deletion or
compaction. Normal current operations validate HEAD, current record, manifest/state/index, and exact
accounting without replay. Missing/corrupt acceleration triggers complete chain validation plus
checkpoint reconstruction; deep audit always reexecutes genesis.

### History alternatives

| design | result |
|---|---|
| full state in every revision | rejected: representative write amplification and ordinary-open replay were the original dominant product costs |
| hash-linked event/host journal + periodic full checkpoints | selected: one exact chain, bounded current restart, simple deep oracle, and localized corruption |
| content-addressed state object per revision | rejected: adds object reachability/GC/publication obligations without beating the selected corpus result |
| structural-sharing full records | rejected: complicates retained accounting and portable decode while checkpoints already meet the ceiling |
| embedded database | rejected: no query-authority or concurrency consumer repays dependency, transaction, backup, and corruption obligations |

Fixed 64-revision checkpoints were selected over adaptive/user-triggered policies because cadence is
canonical, interruption cannot change semantics, and at most 63 post-checkpoint transitions are
needed on fallback. A retained-byte trigger is a reversal gate if larger states approach the journal
ceiling.

## Semantic design selections

### Text

First-class immutable validated UTF-8 text won over client-only bytes, a nominal wrapper, and a
separate nominal declaration kind. It is the only design that makes invalid product text
unrepresentable across workspace, release, application, instance, query, and JSON boundaries without
duplicating a validator. Exact UTF-8 byte equality and no normalization keep the contract small.

### Sequences and operations

Nominal homogeneous immutable sequences won over serialized bytes, recursive cons products,
application-specific fixed arrays, structural generics, and host tables. Nominal identity reuses the
existing release reference model and permits managed indirection without a generic declaration
system. Empty/length/get/append/replace plus counted loops were sufficient; higher-order operations,
mutable builders, maps, sets, and iterators had no product consumer.

Exact `i64` equality and boolean not/and/or were retained because lifecycle, filtering, pagination,
and dependency paths repeatedly used them. Broader arithmetic, polymorphic equality, traits, and
operator syntax lost for lack of a second domain.

### Values and memory

A unified safe managed byte store remains for bytes/text. Safe immutable `Arc` sequence elements won
over tracing GC and mutable arenas because accepted values cannot form pointer cycles and canonical
allocate-new retained accounting remains simple. Representation sharing is invisible and cannot evade
limits. No unsafe Rust or new dependency was introduced.

### Responses and queries

Application-owned typed response and query types directly replaced byte responses and read-as-event.
The four decision variants were selected over full-state no-op records and client preflight because
they preserve application policy while publishing zero revisions for domain decline/no-change.
No-publication receipts are not retained; exact retry is bounded by the unchanged base.

First-class pure queries won over client state decode and a second query database. Exact result
digests permit cache-independent unchanged responses. Current queries publish no filesystem facts,
as verified by complete tree comparison and fault tests.

## Packaging, source, CLI, and topology decisions

- Embedded validated application bytes plus checked-in public-command recipe won over runtime
  generation, mutable artifact lookup, and required user paths. The product remains operable without
  development authority.
- The deterministic Python recipe issues only public `lkjscript` commands and uses named local
  symbols. A new textual language and private Rust builder lost because the existing transaction /
  editable-document path already provides validation and reproducibility.
- Manual standard-library CLI parsing was retained. No parser dependency was needed to deliver closed
  grammar, help examples, strict machine mode, and bounded errors; JSON-only lost the human product
  requirement.
- Parent locator discovery with explicit override won over directory-name identity and global
  registries. Exact operation validation remains in the instance owner.
- A caller-owned product session was retained: one process handles roughly 4,700 representative
  operations instead of one process per request, comfortably exceeding the 50 percent process gate.
  A daemon, worker pool, and socket supervisor have no consumer.
- One session-local exact-HEAD application/current-state cache was retained because repeated query
  medians meet the product ceiling and miss/restart remain correct. No persistent cache was built.
- The activation interface, durable-controller driver, and standalone blob-publisher driver were
  deleted. The product supplies the current stateful/blob consumer and focused tests retain failure
  invariants.

## Agent-cost observation

`lkjwork context` selects active/actionable work and exact blockers under task/note/dependency/text
bounds, reporting omissions and an exact result digest. The representative run used one product
session for 2,000 query observations and recorded 796,923 request bytes and 7,717,826 response bytes.
The five measured one-shot query/mutation groups used 48 fresh processes; the corpus itself used two.
Exact known-result digests eliminate unchanged result serialization without persistent handles.

The maintained application is split into one recipe, one artifact, one binding manifest, and bounded
Rust product owners. Current maintained source/artifact fixtures under `src`, `tests`, `applications`,
and `examples` comprise 95 files / 3,804,716 bytes at measurement time. No provider token classes or
prices were exposed; bytes are not reported as tokens or money.

## Build observations and reversal gates

The observed optimized rebuild after central semantic changes took 2m10s; a later product-only
incremental optimized rebuild took 14.39s. A detached worktree at the baseline with the exact tracked
patch and new files applied built into an empty target directory in 132.84s wall time (Cargo reported
2m12s), then passed the complete public acceptance story and reproduced byte-identical application
and binding artifacts. The page and compiler caches were still host-warm; this is an isolated-target
build, not a cold-machine claim.
The final release binaries are 2,795,368 bytes (`lkjwork`) and 9,706,336 bytes (`lkjscript`).

Reopen design only when a complete current consumer crosses a gate:

- checkpoint cadence or compaction: representative store approaches 256 MiB or fallback/deep service
  becomes unacceptable;
- persistent cache: restart benefit improves complete workload by at least 20 percent and repays its
  hostile format/lifecycle;
- bytecode/native tier: execution dominates and complete work improves at least 30 percent;
- daemon/concurrency: a real multi-client or unattended workflow cannot use the foreground session;
- database/index: accepted query or concurrency needs cannot be met by bounded application scans;
- broader text/collections/search: a complete product workflow requires exact new semantics.
