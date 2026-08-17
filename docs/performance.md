# Current evidence and architecture decisions

This file owns reproduced measurements, controlled observations, decisions, limitations, and
reversal conditions for the current checkout. It is not a campaign diary or a performance claim.
Exact machine-readable values are in
[`evidence/20260817-semantic-core.json`](evidence/20260817-semantic-core.json).

## Measurement boundary

The campaign began at commit `b9d3f97cd349edce8ecd830add4df9002ebe446b` on 2026-08-17. The
machine was Linux 7.0.0-29-generic x86-64 on an AMD Ryzen 9 9955HX with 32 logical CPUs, 32 GiB RAM,
and ZFS. The toolchain was rustc/cargo 1.96.0. `Cargo.lock` SHA-256 was
`d23b75fc162e485b7149d92f1e3349f3cca39f00420a9fef68f8abea6c405620`.

The unchanged starting checkout passed formatting, all-target/all-feature Clippy, tests, and release
build in 0.389, 0.081, 7.541, and 0.026 seconds respectively on the warm machine. All six retained
public examples passed. Times in this file are single observations unless a sample count says
otherwise; they are not distributions.

## Architecture comparison

The table distinguishes measured candidates from design-only elimination. Blank empirical cells are
reported as unavailable, not estimated.

| Candidate | Semantic success | Unintended corrections | Action bytes | Observation bytes | Provider token classes | Processes/connections | Storage growth | Restart | Source opened | Code/verification surface | Human review | Future branch/package fit | Decision |
|---|---|---:|---:|---:|---|---|---|---|---|---|---|---|---|
| A: universal IDs + mapped migration | Existing corpus passed | 0 in final workbench trial | initial plan 15,562; migration 9,035 | maintenance stdout 460,335 | prior workbench observation available | 82 CLI / 82 socket-era requests | r8 10,017 B; body churn allocates/tombstones | decodes all history | `machine.rs` 361,989 B; `transaction.rs` 321,887 B | smallest immediate change, permanent node/migration burden | exact but node-churn-heavy | poor body merge and dedup fit | rejected |
| B: stratified bodies, current store | Passed production and identity corpus | no provider trial | body replacement creates no durable identity | local/durable labels improve review | unavailable | direct engine compatible | 32 replacements remain 443 B each | still decodes history | identity/transaction/validation/compiler owners | moderate semantic cutover and broad tests | function continuity + structural body change | better branch/package basis | selected as semantic core |
| C: B + editable document | Passed maintenance and two migrations | no fresh provider trial | initial 15,663; migrations 9,100 and 1,085 | stdout 434,277; repeated context 38,485 -> 107 | unavailable | 81 CLI / 0 socket connections | same as B | reopen on every direct command passed | `workbench/document.rs` plus boundary tests | one strict parser and DTO-equivalence surface | deterministic scoped text | good proposal/merge input, not authority | selected |
| D: B + immutable object store | prototype not justified | unavailable | unavailable | unavailable | unavailable | topology-independent | expected dedup, not measured | expected reachable-closure win, not measured | new store/GC/retention owners | high crash, corruption, GC, and migration surface | neutral | strongest hypothetical fit | deferred |
| E: canonical text authority | design rejected | unavailable | potentially compact | requires full parse/reconstruction | unavailable | neutral | formatting/canonical text dominates | parse required for authority | parser becomes semantic TCB | weakens direct typed authority and partial exact loading | readable | text merge but ambiguous continuity | rejected |
| F: embedded database | not implemented | unavailable | neutral | neutral | unavailable | direct engine compatible | unknown | unknown | new dependency/build surface | transaction/corruption/export duplication | neutral | possible | rejected without scale evidence |
| G: direct Engine + session | All examples passed | not model-tested | semantic bytes unchanged | semantic bytes unchanged | unavailable | maintenance 81/0; canonicalizer 2 sessions | neutral | every command safely reopens | client links engine | one lock/dispatch path; client +88,960 B | neutral | local branch-ready lock boundary | selected primary |
| H: transparent/persistent service | optional adapter passes boundary tests | unavailable | unchanged | framing overhead remains | unavailable | persistent connection not implemented | neutral | warm-memory potential | daemon + transport | correlation/cancellation/reconnect cost | neutral | multi-client potential | not primary; diagnostic adapter only |

Candidate B was not benchmarked as a separately shipped interface from C; their semantic and storage
effects are isolated by focused tests, while application interaction reflects their combined
production cutover. D, E, F, and H were eliminated before production prototypes because current
measurements did not clear their trusted-surface gates.

## Multi-perspective revalidation

Each named lens from the campaign is covered below. Related lenses share evidence because they are
governed by one retained decision. “Cost” includes implementation and verification surface.

| Lenses | Current evidence and risk | Selected impact and cost | Retained invariant / oracle | Reversal condition |
|---|---|---|---|---|
| Product clarity; naming; accessibility; future GUI | `SPG` described a physical map and led with internals. Terminal review works without a GUI. | Lead with “typed semantic program model”; rename the spec; keep deterministic terminal/file projections. Documentation cutover cost only. | One typed authority; projections cannot validate themselves. | Reintroduce a narrower formal name only if it improves a stable public concept. |
| Agent cognition; weak-model usability; human review; version-control fit; action size; review size | Prior workbench achieved zero unintended correction, yet declaration editing was 9,035 B and review could reach 150,859 B. | One exact editable document, durable anchors, local binders, scoped review. Parser and equivalence tests are new cost. | Scope/base/schema binding; omission cannot edit; deterministic diff. | Reject or redesign the document if fresh equal-task trials increase correction or fail to reduce task cost. |
| Prompt-cache stability; provider cost; documentation locality; prompt hygiene | Root policy is 26,779 B; ten historical prompts had no active consumer. Prior workbench still used 473,927 input tokens. | Keep stable policy; delete ten old prompts; replace chronicles with current docs/evidence. | Git remains archive; no private transcript retained. | Retain a historical artifact only when a current fact or reproducible harness consumes it. |
| Tool-call economy; transport overhead; client lifecycle; operational simplicity; automation safety | Maintenance used 82 processes and explicit service lifecycle; one failed trial blocked on foreground launch. | Direct engine and line session remove sockets/manual lifecycle. CLI binary grows 88,960 B. Typed semantic errors remain successful responses. | One engine, exact publication boundary, no hidden retry. | Prefer a transparent service only after repeated cold-open/cache cost exceeds lifecycle and correlation cost. |
| Semantic authority; serialization; protocol; API stability; version identity; error stability | Multiple projections already normalized into typed requests. Old forms could otherwise be misread. | Keep JSON diagnostic and document authoring projections over one model; direct cutover to protocol 9/artifact 6/context 2. | Closed strict decoders, stable error codes, no compatibility reader. | Change an epoch only when accepted bytes or meaning changes. |
| Identity granularity; identity allocation; history semantics; diff quality; compiler input; debugging | Universal IDs made anonymous body churn durable. Current allocator remains deterministic and compact. | Durable entities/anchors plus function-local term IDs; retain authority-owned monotonic durable serial. Broad semantic cutover and origin tests. | Rejection consumes no ID; local refs cannot escape; history names exact revision/function/local origin. | Reopen allocation for actual parallel branches or imports with collision/remap pressure. |
| Refactoring; declaration migration; incomplete programs; type evolution; multi-turn work | Body refactors and holes need different continuity. Two migrations were structurally different. | Preserve function and hole-anchor identity; replace anonymous body; declaration-shape changes get new IDs; no inferred member continuity. | Explicit target decides continuity; blocked use/deletion oracle. | Add a narrow migration contract only after two independent migrations repeat the same safe mapping semantics. |
| Query locality; search; indexing; context invalidation; caching; apply-and-refresh | Full scans pass current workloads. Repeating one 38,485-B context was wasteful. | Exact known-digest response is 107 B; no persistent index or apply delta. Low code cost; packet recomputation remains. | Full scan and full packet are differential oracles; digest binds all inputs/omissions. | Add an index/delta only after scan or follow-up requests dominate a representative task. |
| Schema discovery; grammar evolution; machine contract; maintenance cost | Selected contract was 86,567 B and `machine.rs` 361,989 B. Field catalogue remains manually assembled. | Embed digest/help for normal work; move catalogue to `contract.rs`, codec to 13,352-B `machine.rs`, tests separately. | Strict codec-agreement and dependency-closed projection. | Derive field metadata when one real semantic addition still requires distant duplicate edits; this is the next locality gate. |
| Editable text; parser safety; locale and Unicode; determinism | Models edit text well, but text can become ambiguous authority. | Retain one ASCII-keyword/UTF-8-string bracketed document grammar with explicit frames and limits. | Parse to typed proposal then discard syntax; byte/line/column errors; canonical output order. | Reopen spelling only with equal-parser, equal-task evidence; normalization policy must be explicit before broader Unicode names. |
| Process topology; failure atomicity; cancellation; concurrency; idempotency; failure diagnosis | One synchronous writer and preflighted response were already sound; daemon lifecycle was not. | Move dispatch/lock to Engine; keep optional adapter. Disconnect does not retry/cancel publication; unknown outcome stops engine. | Publication failure injection, keyed replay/conflict, competing-authority rejection. | Add concurrency/cancellation only with a consumer and a complete state machine. |
| Branching; merge; parallel agents; randomness | Current monotonic IDs assume one publication authority; workspace IDs use OS entropy. No branches exist. | Keep simple allocator after local-body stratification; reserve explicit immutable commits/branch namespace for future work. | One unambiguous head; deterministic proposal allocation; entropy only for workspace namespace. | A retained parallel-agent workload requiring independent entity creation triggers branch-qualified IDs or explicit remapping. |
| Persistence; crash recovery; corruption; storage compaction; recovery time; memory footprint; large programs | Current eight artifacts total 70,896 B; identity churn corpus stays 443 B/revision. Reopen still reads history. | Retain full canonical snapshots. Avoid object/DB/GC trusted surface now. | Full reconstruction, canonical bytes, failure injection, strict old-format rejection. | Prototype incremental storage when growth/restart/live-memory is material on a larger corpus and at least two retained dimensions improve. |
| Garbage collection; hash trust; filesystem attacks | Full snapshots need no object GC. BLAKE3 hashes are integrity keys, not semantic IDs. | Retain bounded files, explicit paths, no symlink components, canonical names, checked hashes. | Collision/substitution is corruption; missing/nonregular/truncated/oversized state rejects. | An object store must first define roots, pins, interrupted GC, kind-separated digests, and cryptographic assumptions. |
| Package artifacts; dependency resolution; reproducibility; package ecosystem; distribution security; licensing; deployment | Workspace history contains development metadata and is not a distribution unit. No package consumer exists. | Keep package/executable artifact domains explicitly absent rather than mislabel snapshots. | Exact revision remains reproducible within current locked checkout; no package claim. | First multi-package application triggers manifest, identity/import, permission, provenance, and untrusted-decode contracts. |
| Cross-platform storage; cross-platform client; remote operation | Canonical bytes are endian/width independent, but only Linux x86-64 is verified; optional transport is Unix-specific. | Keep transport outside Engine and host paths out of semantic bytes. | Same logical requests/semantics across adapters. | Add a platform adapter before claiming support; network work requires authentication, tenancy, cancellation, and threat model. |
| IR design; runtime representation; performance roadmap; hot reload | Current Core IR is a complete executable oracle; no representative evidence supports bytecode/JIT. | Retain Core IR and interpreter; immutable revisions do not imply deployment hot reload. | Faster routes must differential-test exact traps/results. | Representative compile/run profiles may justify bytecode; deployment workload separately justifies hot reload. |
| Managed bytes; memory safety; stack safety; resource accounting; Miri and sanitizers | Ownership route copies/peaks at 23 B versus 32 B allocate-new. Local Rust forbids unsafe. | Retain verified plan and simple oracle; keep explicit frames and separate policies. Verification surface remains significant. | Same values, traps, fuel, cleanup, and bounded result in both modes. | Simplify when benefit is marginal, or redesign for a second managed class, escape, cycles, or external resources. |
| Effects; effect typing; external resources; runtime hosting; sandboxing; local access | Programs are pure; local filesystem permission is the bootstrap access boundary and not isolation. | Do not add host calls or conflate workspace engine with deployed runtime. | Explicit typed authority, partial-action, retry, cancellation, audit, and cleanup required before effects. | First retained effect/resource application supplies the contract and worker threat model. |
| Testing as product; first-class tests; documentation metadata; module system | Tests are external Rust/Python oracles; package/module nodes have only current structural meaning. | Defer semantic tests/docs/package visibility until a package/agent consumer exists. | External tests still use public paths and exact revisions. | Package publication or repeated agent context for expectations justifies first-class test/doc entities. |
| Observability; telemetry privacy; governance | Current metrics are derived and local; raw provider transcripts are not retained. | Keep bounded sanitized measurements outside meaning; expose permissions/dependencies when they exist. | Metrics/caches cannot change behavior or leak content by default. | Add opt-in profiles only with a named optimization/debug consumer and redaction contract. |
| Supply chain; build scripts; compile time; binary composition; crate graph | No dependency changed. Direct client is 7,011,520 B; daemon 4,260,656 B. One package avoids conversion/version duplication. | Retain one crate and existing lockfile; separate modules/tests, not artificial crates. | Dependency has a consumer; project has no build script; local unsafe forbidden. | Split crates when measured rebuild/binary composition wins without DTO duplication or cycles. |
| Source locality; test locality; repository onboarding; change review; deletion discipline | Production logic was buried beside giant catalogues/tests. `agent_repair_json.rs` was 162,565 B. | Split owner-local tests, isolate contract, rename campaign suite, delete stale ignored daemon benchmarks and old plan/prompts. | Complete diff and focused/full gates remain. | Further split only around a changed-together boundary; the remaining 104,471-B integration vertical remains named debt. |
| Collections; text values; generics; documentation metadata | Current applications do not need general data structures or Unicode runtime text. | Add none in this campaign. | Every type needs equality, allocation, lifetime, lowering, public value, and package contracts. | Two retained consumers expose a shared abstraction, or one real application cannot be expressed coherently. |
| Formal reasoning; fuzzing; determinism; time | Publication and identity state machines are small, but no model checker was run. Deterministic mutation tests exist; no fresh coverage-guided document fuzz run. | Preserve explicit state transitions, sorted output, and nondeterministic time outside meaning. | Seeded tests and reconstruction are reproducible; timestamps do not hash into semantics. | Add model checking/fuzzing when tooling is available or parser/publication changes deepen. |
| Governance; change review; roadmap quality | Old roadmap automatically selected mapped migration despite one consumer. | Rewrite roadmap from current misses and reversal gates; expose identity/replacement in review. | Evidence, not chronology or sunk cost, selects gates. | Any next gate must name a consumer, oracle, cost, and deletion/reversal rule. |

## Semantic and storage measurements

### Identity-pressure corpus

A focused public transaction creates one small function and performs 32 committed body replacements.
Each replacement changes an anonymous constant while preserving the function entity.

| Measure | Result |
|---|---:|
| durable identities | 4 |
| function-local references | 4 |
| durable allocations per replacement | 0 |
| tombstones | 0 |
| minimum/maximum artifact | 443 / 443 B |

This is the acceptance evidence for identity stratification. It is not a large-program storage
benchmark.

### Eight-revision maintenance corpus

| Measure | Audited workbench | Current document/identity model | Change |
|---|---:|---:|---:|
| initial raw request | 21,924 B | 21,924 B | 0 |
| preferred initial proposal | 15,562 B plan | 15,663 B document | +101 B |
| preferred initial response | 2,500 B | 2,473 B | -27 B |
| declaration migration | 9,035 B | 9,100 B | +65 B |
| CLI processes | 82 | 81 | -1 |
| socket connections | 82-era route | 0 | direct engine |
| stdin bytes | 85,430 | 86,787 | +1,357 |
| stdout bytes | 460,335 | 434,277 | -26,058 |
| repeated orientation context | 35,734 B old packet | 38,485 B full / 107 B unchanged | digest reuse added |
| disposable context cache | 284,111 B | 296,194 B | +12,083 B |
| revision artifacts total | 71,648 B | 70,896 B | -752 B |
| revision 8 artifact | 10,017 B | 9,457 B | -560 B |

The document did not beat the prior plan on action size, and the process-count target was missed.
It remains selected because it is an exact editable scope, supports deterministic render/parse, and
shares one transaction vocabulary; those benefits require a fresh provider trial before any model
cost claim. Exact context reuse produces the clear observation-byte win.

The second, independently shaped variant migration uses a 1,085-byte document. It first proves old
declaration deletion is blocked, then creates a renamed/reordered/extended variant and replacement
entry, switches the entry, deletes old entities, and passes its Run oracle. The first migration's
9,100 bytes and this migration do not repeat a safe member-mapping abstraction; a specialized mapped
migration operation was therefore rejected.

Current artifact sizes by revision are 8,354, 8,373, 8,662, 8,867, 8,873, 9,155, 9,155, and 9,457
bytes. HEAD is 81 bytes. Reopen succeeded on every direct command. The final example-sweep
observation took 441,257,327 ns; it is not a latency distribution.

## Agent interaction evidence

No fresh provider call was made for document version 1. The valid prior sealed observation remains a
control, not evidence for the new grammar:

| Observation | Raw JSON | Workbench v8 |
|---|---:|---:|
| exact task and independent oracle | pass | pass |
| published revisions | 1, 2, 3 | 1, 2, 3 |
| unintended corrections | 4 | 0 |
| schema requests | 8 | 3 |
| context requests | 0 | 3 |
| shell / failed commands | 33 / 8 | 32 / 0 |
| wall seconds | 305.575 | 178.718 |
| provider input tokens | 1,253,055 | 473,927 |
| cached-input tokens | 1,177,984 | 437,888 |
| output tokens | 15,290 | 8,230 |
| reasoning-output tokens | 3,969 | 3,110 |
| JSONL event bytes | 581,889 | 147,992 |

Provider price was not exposed. No token value is inferred from byte counts. This two-run controlled
observation is not a benchmark distribution.

## Process and binary measurements

Direct commands create no Unix-socket connections. The binary canonicalizer executes 43 logical
calls in two direct sessions and performs four engine opens including lock/corruption probes. Its
dense boundary accepts 1,445 bytes and rejects 1,446 bytes. The optional daemon continues to exercise
private framing, correlation, deadline, and shutdown behavior.

| Release binary | Audited bytes | Current bytes | Change |
|---|---:|---:|---:|
| `lkjscript` | 6,922,560 | 7,011,520 | +88,960 |
| `lkjscriptd` | 4,204,616 | 4,260,656 | +56,040 |

The direct client links engine/compiler/runtime code. That lifecycle simplification has a measurable
binary-composition cost. A lightweight separate client crate or service becomes attractive only if
binary or rebuild cost dominates without reintroducing lifecycle failure or duplicate DTOs.

## Runtime measurements

The retained concat differential runs identical verified Core IR in allocate-new and ownership
modes:

| Measure | Allocate-new oracle | Ownership production |
|---|---:|---:|
| copied backing bytes | 32 | 23 |
| peak live backing bytes | 32 | 23 |
| reuse hits | 0 | 1 |

This one small workload demonstrates a 28.1 percent reduction in the two byte counts. It does not
establish general runtime leadership. The ownership route remains because behavior/fuel/cleanup are
differentially checked and the retained canonicalizer exercises reuse. Its reversal gate is a second
managed value class, escaping values, cycles, external resources, or representative workloads where
the benefit becomes marginal relative to compiler/verifier/runtime surface.

## Repository locality

| Owner | Audited production bytes | Current production bytes | Current owner-local tests/catalogue |
|---|---:|---:|---:|
| `machine.rs` | 361,989 | 13,352 | `contract.rs` 153,226; `machine/tests.rs` 179,745 |
| `transaction.rs` | 321,887 | 162,834 | `transaction/tests.rs` 159,628 |
| `interpret.rs` | 158,137 | 94,936 | `interpret/tests.rs` 56,667 |
| `query.rs` | 158,473 | 96,022 | `query/tests.rs` 55,598 |
| `compile.rs` | 117,694 | 48,175 | `compile/tests.rs` 62,314 |
| `persistence.rs` | 87,186 | 45,419 | `persistence/tests.rs` 38,408 |
| campaign invariant suite | 76,583 | deleted | `generated_invariant_tests.rs` 77,006 |
| `tests/agent_repair_json.rs` | 162,565 | 104,471 | active direct-engine verticals; stale ignored daemon measurements deleted |

Moving tests changes source-open locality, not total verification surface. Rust under `src/` and
`tests/` is currently 2,193,899 bytes and 62,103 lines. The root policy is 26,779 bytes; the active
campaign prompt is 154,550 bytes. Ten superseded prompt files were deleted from the active tree and
remain recoverable from Git history.

The executable contract now has an explicit owner, but it is not yet derived from DTO declarations.
No crate split or dependency change was made. This avoids build/type-conversion churn while leaving
contract generation and the broad JSON integration suite as the principal repository-locality
gates.

## Decision record summary

| Decision | Alternatives considered | Selected option and why | Direct-cutover deletion | Known limit / reversal |
|---|---|---|---|---|
| formal model name | Semantic Program Graph; source program; typed model | typed semantic program model; does not preselect physical graph/storage | old spec path and leading SPG wording | choose a more formal name only when it denotes stable semantics |
| durable identities | all nodes; declarations only; random all-entity IDs | continuity-bearing entities/anchors; anonymous terms local | universal durable body allocation/tombstones | expand only for a named cross-revision consumer |
| body representation | durable node body; immutable tree; local-ID typed body | typed body using deterministic function-local IDs within current snapshot map | body-wide durable churn | move to separate immutable body objects if storage/package evidence wins |
| entity allocation | monotonic; random 128-bit; branch counter; proposal-derived | random workspace + monotonic durable serial | none | branch/parallel workload triggers remap comparison |
| editable grammar | current plan; source-like; S-expression/bracketed; line-tagged; JSON | one strict bracketed `document` grammar, closest to proven parser and exact shared references | `plan` root/module | fresh provider or equal-parser evidence may reverse it |
| edit vocabulary | fine-grained only; generic rewrite; high-level replacements | fine-grained declarations plus `replace_function_body` | no alternate body path | add only repeated domain semantics |
| mapped migration | specialized member map; general rewrite; explicit replacement | explicit replacement; two migrations did not share safe semantics | no endpoint added | two future migrations repeating exact mapping can reopen |
| storage | full snapshots; deltas/checkpoints; object store; DB | full canonical snapshots after stratification | old artifact 5/HEAD7 readers | larger corpus must improve at least two dimensions |
| revision identity | sequence only; content commit; sequence + snapshot hash | exact workspace/revision/hash tuple; immutable sequence remains public | old hash domains | branches/packages may justify content-verifiable commit objects |
| package boundary | treat snapshot as package; package graph now; defer | explicit separate future package artifact | no false package claim | first dependency workload triggers it |
| process topology | explicit daemon; transparent service; direct; session | direct Engine primary plus direct session; optional diagnostic daemon | manual daemon from examples/primary docs | measured open/cache cost or multi-client need |
| contract derivation | giant manual catalogue; macro/derive/IDL; local manual owner | local catalogue + agreement tests as an intermediate convergence | catalogue removed from `machine.rs` | next semantic addition tests derive/macro payoff |
| module/crate graph | one monolith; many crates; owner modules | one package, owner-local modules/tests | campaign test owner | split only with measured compile/binary benefit |
| runtime values | ownership planner; invocation arena; `Arc`; typed tree | verified ownership route plus allocate-new oracle | none | second managed class or marginal benefit reopens |
| prompt retention | retain ten histories; archive all in tree; Git archive | active prompt only | ten superseded prompts | retain a prompt only for an explicit current consumer |

## Tool-assisted verification

- The complete all-target/all-feature suite passes 218 tests; five explicitly manual measurement or
  mutation tests remain ignored by the ordinary boundary.
- Seed-1 release mutation smoke passed 10,000 artifact/framing/JSON cases and 10,000 workbench
  document/packet cases. These are deterministic mutation tests, not coverage-guided fuzzing.
- Nightly Miri `0.1.0` (`rustc 1.99.0-nightly 9f36de775b`) passed three identity-domain tests, four
  document-parser/normalization tests, the 32-revision body-replacement test, and the managed-byte
  differential. The document group took 1,996.05 seconds because each digest-bound case interprets
  the full contract catalogue; body replacement took 79.59 seconds. Stable-toolchain Miri itself is
  unavailable.
- Nightly AddressSanitizer with `ASAN_OPTIONS=detect_leaks=1` passed the same identity, document,
  body-replacement, and managed-byte focused paths.
- No `cargo-fuzz` executable or repository fuzz target exists. No model checker was available or run.

## Evidence not obtained

- No fresh provider trial or price telemetry for editable document version 1.
- No independent production parser was built for every rejected document spelling; the grammar
  comparison therefore has design and existing-parser evidence, not equal-parser implementation
  measurements.
- No package workload, cross-platform build/run, large-history resident-memory distribution, or
  database/object-store prototype was run.
- No coverage-guided document-parser fuzzing or publication model checking was run.

These misses constrain claims and define future gates; they do not weaken the deterministic tests
that did run.
