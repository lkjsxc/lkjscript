# Capacity removal: boundary census and integration ledger

Updated 2026-11-01 (Asia/Tokyo).

## Current dependency-closed candidate

Phase 1, unbounded lists, Profile 7 and the stable artifact index sort are accepted
on main. The latest accepted implementation is the sort at
`67132509f820b8e6b1f5e7ba55112e310e0da162`, tree
`ba3b9359df57717a360d33fb3f6017679fdbc975`; documentation base
`71405a389a422a8fa51850ef70ba9f34040aca56` is separate.

The [active-source candidate](20261101-active-source-self-compilation.md) now passes
complete maintained raw-source self-compilation: C2=C3 and Ahost=A1=A2=A3, including
strict loading, source mutations and complete-inventory negatives. That is focused
Milestone A component evidence, not a full staged-tree seal or complete Tier 2.
The candidate's public graph has 59,883 owners, 88 types and 1,103 canonical units.
Current exact graph/input/output identities and acceptance state live in
[status](../status.md); historical sections below retain their original scope.

| Coupled family | Candidate result / remaining dependency |
| --- | --- |
| Package/source aggregate admission and offline reports | Ordinary work/read policies are None; finite caller policies remain exact, observations carry saturation, current semantic-2 reports require the new flags. Public former-boundary authoring/compile/export/native execution passed. |
| Publication preview staging | Removed the aggregate 64 MiB admission; staged-object validation, corruption/cancellation/refusal and atomic replacement controls passed. This does not remove individual object-domain limits. |
| Independent canonical-reference preparation | Removed implicit aggregate work/entry/metadata limits across schema, types, read views, associated proofs and dependency binding. Exact optional policies and saturation remain separate. The identical complete compiler tx that hit the old reference proof allowance is now publicly accepted and replayed. |
| Byte movement and native scalar traffic | Added three closed bulk/word intrinsics through both evaluators and both backends before compiler adoption; immutable scalar pooling lets the pinned artifact request complete under the same 24 GiB process policy where the prior allocator failed. This is not phase reclamation or a semantic integer cap. |
| Canonical unit / manifest / cache path | Still pending: per-unit byte/item limits, storage envelopes, metadata/preparation limits and narrow host IR identities must be removed through their complete consumers. Large aggregate artifacts do not prove large-unit admission. |
| Source/object envelopes | Still pending: u32 carrier extents and Owner/Type/root/history/blob domain limits plus child/text policies. New profile 3 binds original provenance but does not widen these independent coordinates. |
| Recursive lifecycle and native frames | Still pending: validation, lowering, codecs, clone/comparison/hash/debug and destruction must become stack safe together before depth guards disappear. Native CALL stack growth remains distinct. |
| Map key/value and catalog/GC topology | Still pending: overflow/page contracts, reachability and transport, hierarchy/manifest scaling, pack counts and exact layout versus passive totals. Physical split/compaction targets may remain. |
| Package/data/queue/HTTP/dev-tool policies | Continue dependency-closed audits; this compiler batch does not claim repository-wide quota removal. |

Final acceptance must freeze all candidate inputs, bootstrap that exact staged tree,
run the literal selfhost/native script and named native/source controls, pass all
29 fresh full gates and release target, then commit only that tree. Terminal
receipts and the acceptance commit own the final seal; no later edit inherits it.

Historical anchors: Phase 1 `e7a44d289ddd90daf81125a9f37c66fa17cd3c21`;
list slice `932f197ebcd9c17e2702372300ab73996bb20754`; Profile 7
`5e7e8d37831b33a9e44c3e50a0b733cfd049b64d`. Their detailed records below
are not reset targets or claims about unverified later work.

## Authority and accounting

The accepted typed graph in `tools/selfhost/project` remains the sole program authority.
Maintained `.lkjc` files are proposals. Changes to those proposals are adopted through public
authoring operations, with their actual publication receipts retained. No table below authorizes
direct store mutation or permits Rust to select source owners or lower a source request's TargetId.

An absent quota means unbounded policy. Zero means an explicit zero allowance. Every positive
value, including the largest representable value, means that exact finite allowance. A physical
page size, fixed hash width, or checked machine-layout boundary is not an unlimited-policy sentinel.
Finite admission uses checked arithmetic or remaining-allowance subtraction. Observation may
saturate only when its receipt also identifies that loss of exactness.

Categories used below:

1. **Default quota:** remove ordinary enforcement, preserve optional caller policy and observation.
2. **Caller policy:** preserve exact finite enforcement and propagate the complete optional state.
3. **Representation:** replace all producers and consumers together, with current-format discrimination.
4. **Physical unit:** retain a useful page, chunk, fanout, cache, or alphabet size; grow logically.
5. **Semantic or machine invariant:** preserve bounds, acyclicity, custody, and sound encodings.
6. **Resource failure:** preserve accurate allocation/I/O/cancellation failure and atomic cleanup.

The census is organized by coupled enforcement families rather than by repeated spelling of a
constant. The named-boundary scan covered `src/`, `tools/selfhost/`, and `tools/lkjscript-dev/`;
literal guards, narrowing casts, remaining-budget helpers, serialization, and consumers were then
reviewed separately. A constant inventory is not itself proof of end-to-end removal. A family is
complete only when its listed downstream dependencies and verification are complete.

## Policy, observations, and authoring

| Enforcer / symbols | Callers and serialized path | Category and coupled disposition | State / verification |
| --- | --- | --- | --- |
| `storage/object.rs::StoreReadLimits, StoreReadAdmission` | Directory/memory reads, public read views, package-source reads; joint caller budgets | 1/2. Three `Option<u64>` dimensions; exact remaining allowances, independent observation; combine nested policies by intersection | Applied. Zero/exact/one-over/MAX, atomic failure, and nested allowance tests; storage filter passed 52 tests before later integration |
| `storage/object.rs::StoreWork` | Directory, memory, publication, map consumers | 1/2. Saturating passive counters plus aggregate and per-counter exactness flags; never infer admission charges from saturated observations | Applied to direct StoreWork updates. Higher persisted work DTOs remain a separate cut |
| `persistent_map.rs::MapAdmission, MapWork` | Map load/scan/mutate/encode, compiler link, artifact, publication, package transport | 1/2. Optional page/byte/entry allowances, atomic checked preflight, independently merged saturation flags | Applied. Map filter passed 44 tests before later integration. Saturated cold-cache observations preserve exact warm-cache charges (4 cache tests passed). Witness rebuilding propagates optional per-owner fanout and per-test dependency admission |
| `kernel/validate.rs::FullValidator`, `kernel/work.rs::ValidationWork` | Full validation, inference, affine/callable/intrinsic admission, artifact validation | 1/2. Default work is None; exact finite charge; report saturation separately from exhaustion | Applied. Direct MAX/saturation, resumed inference above the former 10M threshold, affine and cancellation controls added. Focused validation and current-only republication succeeded; included in the accepted Phase 1 tree and fresh full seal |
| `kernel/{callable_flow,composed_callable_flow,nominal_flow,owned_associated,owned_record}.rs` metadata admission | Whole-carrier callable/type evidence and compiler artifact consumers | 1/2. Remove hidden work and 64MiB metadata clamps; carry optional budgets and saturated modeled observations | Applied lower-level work/metadata cut; remaining reference/package/parallel-layout callers must be audited independently |
| `publication/validation.rs::REBUILD_LIMITS`, `publication/repository.rs` full oracle | Revision-pinned reconstruction and dependency validation; current read-view output | 1. Remove automatic store rebuild quotas and redundant 10M reconstruction ceiling; preserve structural checks | Accepted in Phase 1; current validation output carries work-saturated. The exact tree passed all 29 fresh full gates |
| `contributor.rs::SemanticInventory`, dev scale `OracleEvidence` | Typed inventory to JSON scale receipts | 1/3. Carry validation/map/store exactness flags; discriminate the changed scale receipt contract | Accepted contract-4 inventory/scale observation cut; Phase 1 full dev gates passed |
| `change/budget.rs::ChangeBudget` and ten sub-admissions | CLI/control authoring, preparation, repair, selection, refresh, publication revalidation | 1/2/3. Replace 46 scalar default maxima and MAX sentinels with optional exact policies, remove engine clamping, preserve operation-zero semantics | Applied optional 46-dimension API with exact finite admission and passive saturation. Focused evidence and accepted Phase 1 seal below do not establish every caller's charging arithmetic; the later literal/test-retention caller audit is separate. |
| `change/request/{literal,test_retention}.rs::charge` | Authored function-literal edits and retention of accepted test expression trees | 2/3. Charge exact ownership work through `WorkSaturation`, preserve optional finite allowance and failure atomicity | Applied exact charge repairs after finding unflagged saturating additions in both callers. Two direct helper regressions, 16 literal-edit controls and one existing retention control passed; full repaired-tree acceptance remains pending. Prior all-dimension API controls did not cover these callers. |
| `change/request{,/codec}.rs` budget and request frames | `LKJABUDG`, request digests, retry identity, logical plans, receipts | 1/3. Explicit optional policy encoding and new current-format discrimination; remove 10K-change/4MiB request ceilings with complete readers | Applied LKJABDG2/profile 2 tagged optional allowances and unbounded aggregate requests. Six maintained repositories now use receipt profile 2 after public source republication. |
| `control/compact.rs`, `control/change/{input,preflight,tx}.rs` | Compact records and structural authoring into ChangeRequest | 1/3. Remove aggregate input/record/token/value ceilings; checked extent parsing and complete diagnostics; retain syntax/schema invariants | Applied absent aggregate input/record/token/value quotas and checked syntax accounting. Kernel expression/type depths remain pending. |
| `control/logical_plan.rs`, `change/logical_plan.rs` | Logical-plan render/parse, request identities, revalidation | 1/3. Remove derived fixed record/byte maxima, represent optional policies exactly | Applied absent aggregate byte/record quotas, complete optional-policy round trips and atomic explicit output allowance checks. |
| `change/{footprint,relation_view,test_delta,repair}.rs`, request deletion/extraction | Refresh guards, relation/test prefix reads, repair/witness synthesis | 1/2. Remove hidden prefix/min clamps and finite generated budgets; preserve explicit selection and cancellation | Applied exact prefix and cache policy propagation; clipped refresh scans report their binding resource dimension. Cached reads preflight before cloning; repair charges distinct per-owner fanout and per-test dependencies before insertion. |
| `publication/{contract,history,repository,receipt}.rs` | History/refresh, transaction/receipt/semantic-diff/HEAD frames and durable work | 1/3. Unbound logical history and receipts, widen/change frames coherently; add honest persisted observation flags | Applied receipt profile 2 and public six-project republication. Transaction/diff codecs now stream fallibly without 4 MiB / 100K edit quotas. Other history object and intent envelopes remain in the census. |
| `change/request/parameter_order.rs`, `control/change/{declarations,tx}.rs` | Complete callable edits, public compact requests and symbolic transactions | 3/5. Represent final value-parameter order explicitly; preserve retained child identities and require explicit deletion of removed members | Applied typed `SetParameterOrder` after deletion lowering, with exact surviving-inventory bijection and prepaid ownership work. Public `delete parameter` resolves callable/operation children. Check, Clippy and focused parameter-order controls passed. Public selfhost adoption succeeded; subsequent definition readback verified all 23 changed parameter orders. |
| `normalized_query.rs`, `cli.rs`, `contract/registry.rs` | Query/function/context retrieval and continuation tokens | 1/2/3/4. Remove default output and traversal quotas; deterministic complete pagination; explicit caller limits exact | Applied default output `None`, exact finite `--bytes` including `query find`, single-read suffix fitting and truthful read telemetry. Default 50-item pages remain resumable. Context uses canonical u64 depth and visited-set traversal without the former depth-8 limit. Definition recursion remains a separate lifecycle boundary. The integrated 48-library/3-CLI selection passed, and public adoption readback completed. |
| `compiler/{lower,parallel_lower,link}.rs` | Derived types, parallel call proofs and canonical reconstruction across one link | 1/2. Remove implicit 10M work and 64MiB derived-metadata defaults; precharge logical work independently of canonical I/O telemetry | Applied optional exact `CompilerWorkAdmission`, default `None`, count plus saturation, and preallocation worklist admission. The 4,097-function public fixture now reaches native compilation and agrees with VM/reference execution. |
| `compiler/{unit,artifact}.rs` proof work | Loan/read-view protocols; implementation, reference-type, nominal and interface-source inventories | 1/2/5. Remove implicit 10M proof work allowances while retaining structural termination, cycle rejection and independent exactness flags | Applied optional proof admission and passive observations. Nominal enter/exit worklists reject active-path cycles without losing repeated DAG occurrences. Artifact representation metadata still has a separate 64MiB boundary. |
| Kernel canonical codec: `contract.rs`, `meaning.rs`, `graph.rs`, summaries/facts/witness contracts | Owner/type/root/dependency/retirement objects, public authoring, source carrier admission | 1/3/4. Remove logical payload/children/resource/name/summary quotas with matching codecs and overflow/chunking where necessary | Pending. Current owner4MiB/type1MiB/children100K/resource1024 restrictions remain; do not delete matching source guards early |
| `package_interface.rs`, package transport source/oracle and normalized reference preparation | Dependency closure, source containers, representation/callable schemas, artifact loading | 1/2/3. Remove 16M visit/256MiB copy, package-count, metadata and work defaults; propagate exact policy through cached/prepared paths | Pending outer cut; adapted lower-level counters do not remove these caps |
| `builtin_standard.rs`, normalized supplier/source validation caches, publication validation/link caches | Evictable reuse only, followed by authoritative reconstruction | 4. Preserve bounded caches where a miss performs the same complete operation; do not confuse cache capacity with admission | Retain after checking fallback semantics; any admission-rejecting use belongs to category 1 |

## Representations and native/runtime execution

| Enforcer / symbols | Callers and serialized path | Category and coupled disposition | State / verification |
| --- | --- | --- | --- |
| `compiler/selfhost_input.rs`, selfhost `input/types/custody/flow/profile/values` | Mode0 producer, verifier, certificates, machine emitter, source-scalar pack | 1/3. Dynamic descriptor directory; wide independent type identity and custody qualifier; wide rows/offsets; clean profile cut | Applied profile6 producer/proposals: 160-byte header, dynamic32-byte descriptors,80-byte functions,64-byte instructions,16-byte identity/qualifier cells. Publicly adopted. Focused Mode0 C2=C3, maintained proposal equivalence and native closure measurement passed; included in the accepted Phase 1 staged tree |
| selfhost `machine/asm/elf`, `source-scalar-{plan,state,emit,pack}` | Local/stack state, sentinel, ABI frames, emitted branches | 1/3/5. Replace local128 sentinel and fixed slots together; checked wide frame addresses and deterministic wide control flow | Applied with profile6: owner-count sentinel, variable code layout, checked 64-bit R11 transfers/frame addresses. Publicly adopted. Tier 1 native reproduction passed; raw maintained-source self-compilation remains unproved |
| `compiler/selfhost_artifact_input.rs`, selfhost `artifact-input/unit-wire/unit-map/artifact` | Mode1 profile7 projected fields to canonical .lkja | 1/3/4. Wide private input/pool/frame extents and optional exact u64 estimated-proof policy; retain hierarchical 256-way radix branching | Applied Profile7 widths and removal of projection-only guards. Twelve Rust producer controls, public adoption/readback, artifact28/replay1 and GC passed. The repaired exact tree passed native reproduction and all 29 fresh full gates, with zero reuse, and is integrated at 5e7e8d37. Canonical unit codec/indices and manifest/query limits remain separate; see the Profile7 decision |
| selfhost `artifact-order::sorted`, canonical artifact assembly and strict loaders | 56-byte ObjectKey rows, stable representative selection, VM/reference/native emission | 6. Replace quadratic rescans with stable O(N log N) index merging; preserve exact canonical order and explicit policies | Publicly adopted and focused controls passed. Pinned VM work fell 10.51%, reference work 11.67%; output bytes and all four evaluators agree. Full acceptance pending; see the sorting decision. No new logical capacity claim |
| selfhost `source-graph/index/map/scope` and proof passes | Complete meaning-graph carrier, directory admission, scope/type/custody proof | 1/2/3/5. LKJSG002 tag + u64-limb allowance; unlimited skips synthetic proof estimate; structural cursor/DAG termination retained | Publicly adopted proposals and matching Rust test producers. Graph100K and map8MiB/100K caps removed. Large-carrier positive controls and the corrected 100,001-retirement binding control passed, including the hash-consistent negative at native diagnostic 12; included in the accepted Phase 1 staged tree |
| LKJSG002 lengths/counts/offsets and per-object codec guards | Carrier producer, every source pass, diagnostics | 3/5. Current u32 extents remain checked; eventual widening must replace all directory/payload consumers | Explicit remaining representation boundary. New policy width does not imply 64-bit carrier extents |
| `compiler/elf_x86_64::{Asm,compile,Emitter,elf}` | Rust native command compilation and ELF publication | 1/3/5/6. Fallible code/labels/worklists; usize label identity; absolute64 relocations/indirect transfers; checked wide frame and record offsets | Applied removal of 4,096-function/65,536-instruction/32MiB-image quotas. Arithmetic, wide-offset, 33MiB ELF and ordinary native controls passed. The public 4,097-function fixture publishes and compiles beyond both former inventories; ELF, VM and reference each return 4,097. |
| `owned_output.rs`, CLI native build, artifact preparation | Create-new executable stage, artifact bundle, release readers | 1/2/6. Default native publication must not inherit artifact byte quota; explicit output policies remain exact; preserve atomic stage cleanup | Applied Option<usize> publisher; ordinary CLI usesNone and explicit0/exact/one-over/MAX policies pass with atomic cleanup. Artifact preparation16GiB and release reader caps remain separate obligations |
| Native `heap/objects/runtime` and Rust ELF runtime | Byte allocation/copy/buffer growth/hash/derive/filter I/O | 1/3/5/6. Remove256MiB per-value checks; checked signed length, header/payload addition, geometric fallback, mmap extent checks | Applied Rust emitter and publicly adopted proposals. Arithmetic edge probes, existing native controls and focused Tier 1 native reproduction passed; same-process reclamation remains unproved |
| Normalized `value/byte_buffer/owned_storage/vm/reference`, `bytes` | Allocation policy, immutable bytes and mutable buffers, VM/reference | 1/2/6. Remove hidden per-value cap; fallible reserve, exact RunPolicy, cancellation and honest merged observations | Applied. Finite0/exact/MAX, cancellation, and historical-bound arithmetic tests added |
| Native allocation lifetime: R14/R15, `heap::allocate`, buffer discard/freeze | Source prepare -> lower -> ELF, repeated process invocation | 4/6. Mapping registry; immutable input, phase scratch, persistent and output domains; explicit escape/copy protocol and munmap/reuse | Pending. Current refill already grows across mappings. Current process-exit release is not same-process phase reclamation |
| `execution/region{,/runtime,/persistence}.rs`, region value/VM/reference | Lexical region slots, relative pointers, persistence and native Region support | 1/3/5. Remove65,536 live-region cap and checked slot growth; widen u32 pointer/capacity/layout/slice fields coherently | Applied LKJREGN2/profile2 with64-bit header/cell/pointer/count fields, uncapped arena slots and iterative shared-DAG layout/codec/teardown. All46 region tests passed. Native Region/domain reclamation remains incomplete |
| Kernel depth guards versus recursive consumers | Normalize/lower/interpret, codecs, display, equality/hash/clone/drop, native call stack | 1/3/5/6. Remove synthetic counters only after recursive lifecycle is replaced by heap worklists or justified safe mechanism | Pending full lifecycle cut. Iterative validator loops are identified; 256/1024 depth removal alone is unsafe |
| Normalized `RunPolicy` call-depth/value-stack and raw admission items | Foreground/default execution, VM/reference, nested invocation | 1/2/5. Optional execution policies throughout; preserve cancellation/effects; stack-safe implementation before unbounded depth | Phase 1 accepted the four exact optional cumulative policies and independent saturation. The accepted list slice removes ordinary raw aggregate-item clamps; focused controls and all 29 fresh full gates passed. Default 4,096-call-depth, 1M-stack, nesting and remaining specialized item guards require separate cuts |
| Normalized `list.rs`, `list_bulk.rs`, ordinary value admission and list observations | Raw/checked construction, VM/reference, list traversal/disposal and public work metrics | 1/2/3/4/5. Optional exact length policy; explicit worklists derived from index width; unmetered ordinary raw admission; honest saturation | Accepted and integrated on 2026-10-30 at 932f197e. No logical 1M/height-3 limit; 32-way physical fanout retained. Focused runtime, supplemental proof/discovery and all 29 fresh full gates passed; exact seal below and structural argument in the list decision |
| Normalized `map.rs`, prepared/reference types/local-liveness and specialized storage | Ordered maps, schemas, owned/view storage and related admission | 1/3/4/5. Optional schema/work policies; preserve structural balanced-map height proof | Pending remaining map/schema/owned/view cuts. Map height2*usize::BITS is mathematical; specialized logical item guards remain |
| `compiler/{unit,artifact,manifest,cache}.rs`, artifact loaders | Compilation units, .lkja manifest/segments/bundle, loader coherence | 1/3/4. Remove unit8MiB/1M-item and manifest/package/unit/bundle quotas with complete framed readers | Pending. Source-derived canonical units and host parity require this path, not just Mode1 projected-unit acceptance |
| `compiler/egraph_opt.rs` | Optional local optimization, subsequent ordinary emission | 4/5. Retain finite search strategy only when exhaustion falls back to correct unoptimized code | Audit fallback and measure; do not report an optimization search cutoff as a semantic admission ceiling |
| Hash widths, byte alphabet, checked extents, DAG/ownership rules | Every wire decoder, canonical hashing, source/native admission | 5. Preserve exact commitments, checksums, sortedness, bounds, cycle rejection and finite instantiation | Mandatory throughout; malformed fixtures recompute hashes so they reach their intended semantic layer |

## Persistent storage, service surfaces, and development gates

| Enforcer / symbols | Callers and serialized path | Category and coupled disposition | State / verification |
| --- | --- | --- | --- |
| `persistent_map.rs::MAXIMUM_{KEY,VALUE,PAGE}_BYTES` and tree depth | Canonical owner/type/unit maps; leaf encode/decode and scan | 1/3/4/5. Preserve64KiB physical pages; overflow/chunk value references and scalable keys/tree structure; checked reader extents | Pending. Current48KiB logical-value limit remains despite optional operation admission |
| `storage/contract.rs`, pack writer/reader | Pack entry64MiB, pack1GiB,2M entries; object lookup/reconstruction | 1/3/4/6. Remove logical pack quotas; checked 64-bit framing and actual file extents; physical target pack size remains a tuning unit | Applied removal of the three pack quotas, singleton oversized objects and progressive deflate output growth. The 32MiB compaction target is physical. Fallible allocation/I/O, digest verification and atomic publication remain; catalog limits are separate. |
| `storage/project.rs::MemProjectStore`, shared memory arena | Ordinary memory projects, compiler/publication fixtures, cloned handles and forks | 1/2/6. Optional exact aggregate byte policy; reserve atomically before growth | Applied `Option<usize>` with ordinary callers using `None`; finite zero/exact/one-under/MAX remain distinct. Five memory-policy tests passed, including shared-index reservation failure atomicity. |
| GC preparation work and retained canonical-byte observations | Reachability preparation, public GC observation and retained-object inventory | 3. Preserve incoming store exactness and mark local observation overflow independently of admission | Applied `preparation.saturated` and `canonical_bytes_retained_saturated`; `references_visited` still counts unique deduplicated queued bindings. All 12 selected GC tests passed in the green 14-library/6-public-CLI followup; public collection passed for all six maintained repositories with unchanged HEADs and source transports. Physical catalog/pack capacity work remains separate. |
| Storage catalog/segment/block/manifests | 8M entries,100K packs,32segments,level31, manifest64KiB, segment metadata/bytes | 1/3/4. Grow hierarchical directories/levels; remove total count quotas; preserve block/page invariants | Pending. Catalog observation saturation must also reach persisted receipts |
| `storage/directory/gc.rs` physical-byte observations and catalog guards | Physical pack collection and leftover inventory | 1/3. Separate passive byte observations from exact structural extents; remove logical pack/leftover ceilings with scalable catalog representation | Pending WS2: `checked_add` can still return `gc_pack_bytes`, and `MAXIMUM_CATALOG_PACKS` / `MAXIMUM_CATALOG_LEFTOVERS` remain. The GC preparation correction does not remove these boundaries. |
| `package.rs`, deployment/configuration/installation/release_container/packed | Public package/deployment descriptors, version inventory, archived executable transport | 1/2/3. Unbound logical modules/dependencies/targets/fields/versions and container bytes; keep semantic grants exact | Pending; packed128MiB and release128/256MiB reader limits remain |
| Data/queue/object/secrets/security modules | Transactions, scans/backups/history, payloads, attempts, randomness and secret values | 1/2/3/4/5. Separate exact caller policy, logical data size and physical chunks; preserve key/transaction/lease semantics | Pending. Data 4 MiB/4096 mutations/16MiB txn/10K scan/1M work and related catalog caps are explicit census members |
| HTTP/client/session/stream/runtime/worker modules | Bodies/headers/DNS, live connections/mailboxes/tasks/streams, deadlines | 1/2/3/4/6. Optional operational policy, streaming/backpressure and exact cancellation; fixed buffer/chunk sizes may remain physical | Pending. These project defaults are not justified merely by being service-facing |
| JSON and normalized data/region codecs | JSON nesting127, value depth128, byte/item admission, recursive serialization | 1/3/5. Stack-safe decoding/encoding and optional policy; preserve structural grammar and extent checks | Pending. Serde/bincode limits and destruction must be audited together |
| Dev process/output/read limits: service/distributed/stateful/offline/outbound/release tools | Candidate/verifier binaries, receipts, logs, artifacts, authority snapshots | 1/2/4/6. Preserve explicit test-selected limits; ordinary readers unbounded or streaming; keep complete logs behind excerpts | Pending. Candidate256MiB/verifier384MiB and response/artifact caps must not conceal a product ceiling |
| Dev check surface/policy/origin/workspace/authority inventories | Directory/file counts, ancestry/config/metadata, proof units, manifests, process descendants | 1/2/3/5. Remove silent omissions and default finite admission; maintain complete source/origin evidence and cycle-safe walks | Pending; listing excerpts may be bounded only with complete retained evidence |
| Dev concurrency/cache/optimization knobs | Worker count, gate scheduling, retained entries/bytes, failure excerpts | 4. Retain bounded parallelism/caches/excerpts when all work and complete evidence remain retrievable | Preserve semantics; distinguish scheduling from admission and cancellation |

## Bootstrap and verification ledger

The current Rust checkout remains the disclosed bootstrap. New language capabilities must be
implemented and tested there and in each required native output route before compiler proposals use
them. The first LKJSG002 compiler is built from publicly adopted source-budget/proof proposals.
Old LKJSG001 decoding is removed at that cut; this does not grant a host semantic-preprocessing route.
The applied Mode0 profile6 proposal cut replaces its producer, validators, certificates and
native consumers together and is publicly adopted into the maintained graph.

During iteration the coordinated target directory and `target/.lkjscript-check.lock` are reused.
Healthy builds are joined, never duplicated. Targeted test results above describe the exact earlier
candidate slice; later edits require their own integration evidence. Each subsequent slice requires
literal `tools/selfhost/verify.sh`, staged-tree bootstrap,
fresh-full/release gates and normal integration against the exact staged tree.
Public proposal adoption, definition readback and the focused Tier 1 source measurement have
completed. The Phase 1 acceptance below completed its required gates; later slices require their own proof.

## Accepted Phase 1 seal and normal integration

Implemented code and public graph adoption were sealed on precommit HEAD
`dad67f97c6b9b008300d59d83e3e4e7540acfa4f`, exact staged tree
`bc733630e4850e1cc50e91607029a24ca520c6e1`. The six-operation driver, postrun
collector and independent audit each joined with exit 0. Source bytes and modes
remained unchanged across 2,192 indexed files / 133,793,256 bytes. The checker
transient directory was absent, the bootstrap artifact directory retained only its
receipt, and the three pre-existing artifact ELF files were preserved with no new
stage ELF retained.

| Phase 1 seal operation | Observed terminal result |
| --- | --- |
| Independent staged-tree bootstrap | Exit 0; receipt SHA-256 `5b6432dd5d5dd4c0475115984411b68ae91df2924ebd177d44a9d898115f27fa`; verifier SHA-256 `8f315c1f9469cda5e7cf112e6c3f979062156f1ff7f7a81859b8e3d23c489117` |
| Literal selfhost script | 138 passed, 0 failed, 2 ignored, 2,697 filtered; 3,379.49s |
| Native filter in the literal script | 11 passed, 0 failed, 0 ignored, 2,826 filtered; 118.25s |
| Named maintained-native-closure measurement | 1 passed, 0 failed, 0 ignored, 2,836 filtered; 22.64s |
| Named whole-owner/type inventory | 1 passed, 0 failed, 0 ignored, 2,836 filtered; 13.01s |
| Full `--fresh --machine --jobs 4` | 29 passed, all fresh; zero failures, reuse or unrun gates; stable inputs; 3,790,577,053,911 ns |
| Release target | Exit 0; `x86_64-unknown-linux-musl`, static-musl target policy; no archive publication or deployment |

The public CLI harness in the full run reported 350 passed, zero failures, one ignored
and one filtered in 366.03s. All eight repaired regressions were directly observed
passing in that full run. The compact successful workspace gate proves its terminal
success; no main-library terminal count is inferred from a removed transient log.

| Retained Phase 1 evidence | SHA-256 |
| --- | --- |
| Accepted staged-tree record, 6,881 bytes | `9e2a4fcab31a42f56e1bf039aad871d7e17ef008defc0f7f8bf66dc1d64b9c02` |
| Complete compact full receipt, 24,663 bytes | `668c3548809c8abf9a063f017a6a0ed9a618754bcbb91749edc6065611955503` |
| Postrun observation | `db4569f071c85ad7bca81a61e228c174444a1f2f8bc20fda4d1052c94d02ae69` |
| Independent audit | `79d063fe332f8dbf71f6db50a009891b8e2574213351435a808c41da6eedd2ba` |
| Terminal integration record | `fcde1e7da229ffec013952191454a306317e250c6181692a9d33669190b4504b` |

Normal fast-forward integration delivered commit
`e7a44d289ddd90daf81125a9f37c66fa17cd3c21` to `main`; independent Git and GitHub
readback verified that commit, its parent and the accepted tree. The local main
worktree was clean. Evidence records reside under
`.artifacts/capacity-unbounding/seal-20261029-round4/`; the default checker retains
its one current compact receipt under `.artifacts/lkjscript-dev/latest-receipt.json`.
The acceptance commit binds the observed source and receipt identities independently
of later receipt replacement.

The literal verification reaffirmed Mode-0 profile-6 C2 = C3, SHA-256
`342ec8bcf60d548225d5818c3d51677d08f5fe987d59c601307c17be82202ed6`, and Mode-1
profile-5 artifact C1 = C2 = C3, SHA-256
`a441d5740bf81e9b292e0963ae9cae9917cc61473ac12b52eed3f768e74d598c`.
These remain Tier 1 equations using the disclosed Rust bootstrap and normalized or
projected inputs. Accepted Phase 1 gates do not prove full unbounding, raw-source
compiler fixed points, Tier 2/Tier 3, native reclamation or bounded same-process RSS.

## Accepted list slice after Phase 1

The list slice was developed on `capacity/20261030-unbounded-lists`, starting
from accepted commit `e7a44d289ddd90daf81125a9f37c66fa17cd3c21`, and is now
accepted on main at `932f197ebcd9c17e2702372300ab73996bb20754`. Its initial 21
Rust/tool changes have application receipt SHA-256
`2469dabd5b802ade4363d8990c820b524b7d7b660d77a7f84abfa81b1a315844`.

Implemented: optional exact u64 list length policy, removal of the logical one-million
length and height-3 limits, explicit radix worklists derived from index representation,
and removal of ordinary raw-value aggregate-item/visit-fuel clamps. The 32-way physical
fanout remains. Exact invocation/shared policies and checked storage extents remain
independent of passive list observations. Aggregate list saturation propagates to public
production/reference fields; pure-tail bounds require exact observations. The
supplemental stored matrix/mapping/boundary/cancellation proof checks require complete
typed, unsaturated `ListWork`. Serialized missing/malformed/true-flag controls and
public capability checks for all 18 numeric links and both flags are implemented.
The descriptor inventory is 72 distinct records; pure-tail selects 38 fields.
These supplemental checks passed in the separate terminal verification recorded below.

Implemented controls include the 22 core list test functions and VM/reference admission
at 1,048,609 I64 elements, with zero/exact/one-short/maximum finite policies and a wrong
final Bool. The initial focused run joined with driver exit 1 and Cargo exit 101:
107 passed, one failed, zero ignored and 2,735 filtered; tests took 6.80s and the
driver took 391,702,033,744 ns. Log SHA-256:
`ba3b2018257f7979f3148837532d68a2f92e130a879ff84563e5a117261be110`.

The only failure was the existing
`vm_map_tests::map_admission_keeps_depth_and_aggregate_item_bounds_in_foreground`
expectation that unmetered admission of 1,000 map entries with 1,000 list items each
would exhaust the removed aggregate ceiling. Actual execution returned `Ok(I64(1000))`.
The 1,048,609-element VM/reference positives and wrong-final-type controls passed.
The fixture now retains unmetered admission for both 999- and 1,000-item children
and checks zero/one-short/exact/maximum policies from each evaluator's separately
measured cumulative baseline. Depth-255/256, cancellation and deep-drop controls
remain. This map-policy correction has receipt SHA-256
`c2754e5088f8b8d2a58f30715cca255b1fa276e795e038ca10413378543c97d6`.

The corrected focused run joined at `capacity-resume-1030-045` with driver and
Cargo exit 0: 108 passed, zero failed, zero ignored and 2,735 filtered; tests took
7.43s and the driver took 376,388,950,567 ns. Its 38,365-byte log has SHA-256
`4cf89958aec4f7fa0adff9753f60bc0ef6bb150699a514d2dba677826c9caef5`.
This used the candidate with 22 changed source paths, including the map-policy correction, before
the later stored-proof/public-discovery repair, whose separate terminal verification
follows. The observed host is x86_64; 32-bit index coverage
is source/model only, with no 32-bit execution.
Those focused observations preceded the later full seal recorded below. The
[list decision](20261030-unbounded-lists.md) records the structural argument, controls
and remaining depth/map/owned/schema/codec boundaries.

The supplemental stored-proof/public-discovery repair has application receipt SHA-256
`693fcb2710f7de2379ca0fc05341d8e323893cb8ec79fc8bbb8ea89da2bf6dd4`.
Its first verification driver joined with exit 1 at `capacity-resume-1030-053`.
Clippy passed for that repaired 23-path candidate in 39.51s, with log SHA-256
`5c15f77215c5055fc1689d6658aff0f7baaacd88bf5db85deab3b89c7ce8d20d`.
The proof invocation selected the binary target, whose entry delegates to the
library. Cargo exited 0 after running zero tests; the driver rejected the missing
three-test inventory. The zero-test log SHA-256 is
`885469bc3fd38643568d0d56958afe114ee2052101534a7e4368002c58958337`,
and the driver receipt SHA-256 is
`d4a7b35de7b76b872a5129ed2fc08ab88e630b9c9bb45b22111e8f0494ed74e1`.

This was an incomplete test selection, not an observed test pass or a source defect.
No product source edit followed that refusal. The second attempt selected the actual
library target and explicitly reused only the same-source Clippy observation. Its
owning session joined with exit 0 at `capacity-resume-1030-064`. The 11,970-byte
receipt binds all 23 source hashes and has SHA-256
`3045f15a8b44023fab176458ea6ecc2be2baacc14f9e09777152a0f348603d78`.
Three active library proof controls passed with zero failures or ignored tests,
352 filtered and 0.00s test execution. The public capability control passed with zero
failures or ignored tests, 351 filtered and 0.10s test execution. Cargo selected both
current executables with `fresh: false` records and exact SHA-256 identities.

Pure-tail passed with 38 discovered fields, 82 commands and complete cleanup. Its
receipt SHA-256 is
`9e08a4a7907586468036188c1b67fb06816f0d5efd65899f4f66d008c9d746ac`.
Internal elapsed time was 48,092,616,427 ns; the driver operation took 48,564,108,076 ns
including process overhead. Source-batch authority before and after was identical:
14 files, 2,399,591 bytes, HEAD SHA-256
`aaefd72a3dbeb386c241711c10a4a8a1882cd06e5bba26deb5a0523b873da0df`.
Public discovery generation and verification both exited 0. All eight generated
documents were updated and verified, sharing the new registry header identity;
the operations document also carries the changed capability content. This focused
verification, including the explicitly disclosed Clippy reuse, grants no fresh-full
gate acceptance claim.

The later complete seal used precommit HEAD
`e7a44d289ddd90daf81125a9f37c66fa17cd3c21` and exact tree
`6717d8249c1db319169b2b32be0e088ef09dea21`. Its complete driver, postrun
collector and independent audit joined with exit 0. Raw bytes and modes remained
stable across 35 changed paths, 2,193 indexed files and 133,847,296 bytes.

| List seal operation | Observed terminal result |
| --- | --- |
| Staged-tree bootstrap | Exit 0; receipt SHA-256 `a196fcd784d6411da50296115d00595bf689707c530a3420dfdd6f6a7b7f4882`; verifier SHA-256 `d10f759ed500cc0dccc918e2939316cdbf160bda601c115f019a2a8f77aa5457` |
| Literal selfhost script | 138 passed, 0 failed, 2 ignored, 2,703 filtered; 3,367.50s |
| Native filter | 11 passed, 0 failed, 0 ignored, 2,832 filtered; 119.22s |
| Maintained native closure | 1 passed, 0 failed, 0 ignored, 2,842 filtered; 22.78s |
| Whole-owner/type inventory | 1 passed, 0 failed, 0 ignored, 2,842 filtered; 12.92s |
| Full `--fresh --machine --jobs 4` | All 29 passed fresh, zero reuse/unrun, stable inputs; 3,797,629,862,566 ns |
| Release target | Passed static-musl policy; no archive publication or deployment |

Full compact receipt SHA-256:
`4c701f56772b537dc3bcbfac758861e17c4b46c609d5e06df342cc7747a081b8`.
Accepted staged-tree record SHA-256:
`cd37083cdce85da2cc988804be00d4a420fee5ddaa6d96baa6daf33a9c4dc2f0`.
Postrun observation SHA-256:
`17df0f1be9e8c082acd5981df0f003a63d3e3bffa16146d86cf832507f08298a`.
Independent audit SHA-256:
`da307315b77736daeccc7c1ae576c61df4d5976f2a3c9361f64177f12563fec8`.
Integration record SHA-256:
`9a705b8413d125eb584d74e92bae285fb46d0cec41be94164c70b7ab75ede10b`.

Normal fast-forward delivery and independent Git/GitHub readback verified main
commit `932f197ebcd9c17e2702372300ab73996bb20754` and that tree. The source-bound
records reside under `.artifacts/capacity-unbounding/seal-list-20261030/`.
Gate freshness does not assert every Cargo unit was rebuilt. Exact workspace
harness totals were not captured before cleanup and are not inferred from prior
runs or gate inventory. Checker transient storage was absent, bootstrap retained
only its receipt, and exactly the three historical repository-artifact ELF files
remained; the collector did not inventory OS temporary files. Mode-0 profile-6
and Mode-1 profile-5 fixed-point identities remained the Phase 1 values above.
This acceptance does not prove Profile-7 behavior, full unbounding or Tier 2.
The accepted Phase 1 receipts do not accept later source edits. Historical correction
and failed-attempt records below retain the evidence available at their own checkpoints.

## Profile-7 projection/pool cut after list acceptance

Implemented on `capacity/20261030-artifact-profile7`: coordinated Rust producer,
selfhost admission/emission and fixture changes for current-only Mode-1 profile 7.
The header uses explicit `None`/`Some(u64)` policy; unit/source rows are 16/56
bytes, unit prefixes 272, table directories 160, signatures/parameters 40/48,
code/instruction records 32/48, and generated frames eight bytes. Unit facts are
138 bytes with unchanged 82-byte map bindings. Mode 0 remains profile 6; canonical
unit/map/pack bytes and semantic u32 index types are separate contracts.

Projection-only 4 GiB, 8 MiB, 100,000-entry, 32,768-table/instruction and
128-parameter/local/operand clamps are removed. Finite estimated proof admission
compares `A >= 64*(64 + 2*L + U*U + O)` by checked division/subtraction before
hashing or deep work; `None` does not evaluate that estimate. Measured projected
output is admitted before its fallible byte-buffer reservation. Earlier Rust
artifact admission, map reads, decoding and inventory preparation remain outside
that embedded policy. Checked object-byte/pack/container sums govern allocation;
passive telemetry never does. The four-way artifact probe helper is applied for
VM/reference/Rust-bootstrap ELF/selfhost-emitted ELF, with probe executables
cached by exact emission-payload hash. Its focused executable results are recorded
below, separately from native compiler reproduction and full acceptance.

Observed producer verification joined at `capacity-resume-1030-143` with exit 0:
12 passed, zero failed/ignored, 2,842 filtered, 0.01s execution; focused compile/test
operation 366,873,597,353 ns. Receipt SHA-256:
`abab2fe578d207b57d3606296619e268a672700d4df163f0310504cf97e2c0ee`.
Cargo selected an 80,050,464-byte product with `fresh: false`, SHA-256
`c25a6d576f62bfbe273ea26f1985bd5feaf2d5e45933fd51791c10c2b10ff6d2`.
The tests cover wide/nonzero-origin layouts, exact policy and a code projection
above 8 MiB; they do not remove the independent canonical-unit codec boundary.

Public adoption joined at `capacity-resume-1030-148` with exit 0: nine modules,
69 changed members, zero explicit deletion operations, 79,847 transaction bytes,
SHA-256 `c3f19cd392712801b4c011662f183e8cce614923851c974c2688fb3e1b7593f4`.
It reported 4,626 created/60 updated identities and equal authoring differential
results, with zero selected runtime tests. Accepted graph revision:
`rev_b14f1d9465fd9d67cad9cc5946607a7028d2a2d26a2a2883ad953422f23a1b05`;
HEAD SHA-256 `f66f443af7f9e46179c56fc9600113fa72dc65b2284ceb6e4450a83494eccc79`.
Readback joined at `capacity-resume-1030-150` with exit 0, covering 36,080 owners
in nine pages. No existing parameter order changed, so no changed-signature pages
were required. Its 1,065,885-byte receipt has SHA-256
`5155d7b8f5f991a15c8971c9c3daeb2086f6b12aec3a44ccb765013d92332e8e`.
The current 28,895-byte preparation/submission record has SHA-256
`a97bfbe66f62f81ea09e4e16d61919bfa41ef94a4142974e078a89452ea9aa4e`;
submission took 39,268,461,095 ns with empty stderr. Those proposals are now
authoritative; complete body replay subsequently passed separately.

The focused owner `capacity-resume-1030-152` joined through request 172 with
exit 0. Its 2,195-byte receipt has SHA-256
`81eecca6777b57239d65c3781770f57244826a5f9bf20849d2de2b1f8eb3c3b9`.
The artifact selection passed 28 tests, zero failed/ignored, 2,826 filtered, in
658.49s execution; the command took 1,021,724,923,634 ns and its log SHA-256 is
`92197843dedb6767b6ee239455489e98cc26f87815c50634518753501d189e89`.
The complete semantic replay passed one test, zero failed/ignored, 2,853 filtered,
in 46.63s execution; command duration 46,775,859,341 ns and log SHA-256
`71a69545b308820dc4fd762f2bc972c296003f0d60d7d685c26ccc5de2d64c90`.
Initial/final source inventory matched: 2,269 files / 154,423,333 bytes, SHA-256
`ea226ed609fadb3f8476e3dafdb166a0e3552d97d400ea8362599d2260f5ef21`, with unchanged
public HEAD. Run interval: 2026-10-30T06:14:07.602Z through 06:31:56.355Z.

The artifact probes exercised VM, reference, Rust-bootstrap ELF and selfhost-emitted
ELF with independent canonical oracles. Complete compiler artifact emission reported
646 functions, 10,806,949 input bytes and a 10,338,816-byte artifact, SHA-256
`41dd8141cb12a9501e69c8092e0b8e5d7c020765967511ec051c50bacac18f23`.
Unit emission covered 650 units with kind tags 5, 6, 9 and 20. Reported VM/reference
work was 826,834,103 steps / 438,157,491 expressions, cumulative modeled allocation
86,155,453,261 / 97,782,508,693 bytes, and maximum call depth 21/22. These values
are work observations, not live retained memory or RSS, and the focused suite does
not establish repeated native C2/C3 compiler or artifact fixed points.

Public GC owner 173 joined through 174 with exit 0. Physical storage changed
from 79 packs / 85,802,127 bytes to three packs / 81,763,516 bytes, reclaiming
4,038,611 bytes. GC took 10,106,528,987 ns; its before/after export commands took
6,055,138,869 / 5,958,223,363 ns. Receipt SHA-256:
`947e765ec799733810bca182f26d8ce80be01a557786162ecf0702a834779de3`.
`preparation_saturated` and `canonical_bytes_retained_saturated` were false.
HEAD/revision remained unchanged; both complete transports were 15,330,787 bytes,
SHA-256 `8774b278e0859f0c092af119248fff73ad57d5ca27607a802d1e18e8a3cfc7a1`, digest
`package_transport_6866de46a96c9839de546be697ff2ee8fbcae78b1cb8882c85b07b5161cf85f9`.
The duplicate second container was removed. Physical compaction does not remove
retirement tombstones or preserve the earlier physical source-file inventory;
that inventory's stability belongs to the preceding focused run.

After the focused run, the artifact test helper replaced `.chunks_exact(138)`
with the equivalent `.as_chunks::<138>().0.iter()`. Final fmt and workspace,
all-target/all-feature release Clippy passed at joined request 187; durations
4,613,727,314 / 31,187,303,762 ns. Preflight receipt SHA-256:
`0dd326f7895d9650c57740acbadfc2139f1d561b3713a7e5a915658a08a9c4b5`;
Clippy log SHA-256
`b11a1884c6cc92803bff7bea6f906f5104d91d90aa694fbb4aface6ad11862ef`.
Discovery generation/verification passed in 876,021,424 / 857,130,798 ns with no
generated changes. The focused result retains its earlier source binding; the
final frozen-tree gates must exercise the corrected helper and documentation.

The first sealing owner, `capacity-resume-1030-195`, joined through request 231
with exit 1 against tree `13b09cc3e4be24802a6fcbd0433b8a96d45eb33d`. The literal
selfhost child exited 101 after 3,770,985,268,606 ns: 148 passed, one failed,
two ignored, zero measured, 2,703 filtered and 3,407.83s test execution. The
failed checkpoint completed at 2026-10-30T07:51:42.299Z and confirmed the frozen
tree with 2,194 files / 150,424,720 bytes. The 339,847-byte selfhost raw log has
SHA-256 `8ec6f51fcf7f230be4341bd081ca942a0856e1f015fb65a388f8e4674a54213f`;
the 3,947-byte checkpoint has SHA-256
`96a6c0958e3d876ab867ccbcc6f528e3f324bc1fcc42bde8cd1e1eaf6fbf969a`.
The 3,241-byte compact failed-seal observation has SHA-256
`ce5358f5c2070f5a65391fc65c79f4a10f7354b976346ab3c5b8ada0978e7e6c`.
No native subharness, named complete-compiler controls, full gates or release
target ran in this attempt.

Its native closure control within the literal harness passed:
294 functions, 18,261 nodes, 2,021,454 normalized-input bytes. Mode-0 profile-6
C2 = C3 was 1,249,170 bytes, SHA-256
`c613d7c604322d42f4cec6bd65d83520828d901fa7f620c792707aeb1caeedb7`.
Both native stages consumed the same 10,806,949-byte Profile-7 artifact input,
SHA-256 `c50451cd991e04fe64e38f5a7bae5a4d34497cbfd54e614ad2c9c137bde19791`, and
emitted the 10,338,816-byte artifact, SHA-256
`41dd8141cb12a9501e69c8092e0b8e5d7c020765967511ec051c50bacac18f23`.
The capacity input/output were 11,086,603 / 11,179,173 bytes, with SHA-256 values
`0e2919a816c9db5bb2c8aa17015fea250bf6ed42f90bea93be5e2f88558e1f6e` and
`737f18a830ae343e1c7715e840a22edc13507856a8475e7eb68dec745be552b3`.

The sole failed test was a stale generic fixture: it selected retired Profile 5 while
expecting framing diagnostic 10, so the current dispatcher returned selector
diagnostic 2 first. The applied correction selects current Profile 7 for the
structural negative and adds a separate former-Profile-5 rejection control,
recomputing both carrier hashes. The corrected malformed-input test passed in
VM/reference/Rust-bootstrap C1. Its framing diagnostic 10 precedes checksum and
deep-unit validation; this test has no selfhost-emitted C2 execution claim.

A separate audit found unflagged saturating ownership additions in the literal
and test-retention `charge` helpers. The applied repairs use exact
`WorkSaturation::charge` admission and direct helper controls for optional finite
boundaries, exact maximum/overflow, sticky saturation, finite refusal after
saturation and unchanged work/edits/HEAD/root on refusal. Both direct helper
regressions passed after independent static review and application. Prior
46-dimension counter API controls did not exercise these callers and do not prove
all accumulator callers exact.

Request 235 applied the three Rust corrections under the canonical lease;
application receipt SHA-256
`f35a1517fa9ea8c53811c9bb6222de152ccb280a680a40c6227b0e04e21226bd`.
Focused owner 254 joined through 257 with actual exit 0. From
2026-10-30T09:24:05.902Z to 09:25:46.368Z, all seven operations passed: formatting,
two direct charge tests, 16 literal-edit tests, one existing retention control,
one malformed-input control, formatting check and strict all-target/all-feature
release workspace Clippy. The four test selections contain 20 nonoverlapping
passes, zero failed/ignored/measured. Their execution times were 0.01s, 0.82s,
0.30s and 49.35s; filtered counts were 2,854, 2,840, 2,855 and 2,855. The
8,609-byte final focused receipt has SHA-256
`3fff28de686971dda8212a20f5a0458d51de1a2ac7d37a5e9fdd8024c2cbeedb`.
Initial/final source inventory matched: 2,194 files / 150,435,825 bytes, SHA-256
`1aed7e766c19e6e36558d0853964c1878354179a63efb411e09e0f8919dc25de`, with unchanged
public HEAD. These source bytes precede this documentation update and final freeze.
The first-tree native identities remain separate from these focused repair results.

Pending: finish source/documentation and affected maintained-output checks, then
freeze/bootstrap the repaired tree. Run
literal selfhost verification, both named complete compiler controls and both
native fixed points, then all 29 fresh full gates, release target, audit and
normal integration without source drift. The
[Profile-7 decision](20261030-artifact-profile7.md) holds the detailed scope and
local-model inventory. Individual first-run reproduction results and prior focused
success grant no repaired-tree acceptance, Tier 2, bounded-RSS or phase-reclamation
claim.

Canonical unit/packed-codec limits, manifest/cache/query inventory, kernel object
and storage envelopes, overflow map values and recursive lifecycle work remain
pending; these are not repaired merely by wider private transport. The next unit
cut must also cover the outer 4 MiB CompilerUnit and 64 KiB CompilationManifest
storage-domain allowances and nested recursive allocation/decode/drop obligations.

## Bootstrap and runtime correction evidence

The first six-document followup was applied with every guarded result hash verified.
The candidate was frozen at source `dad67f97c6b9b008300d59d83e3e4e7540acfa4f`, tree
`6ae957fb647b194b763a6e6eec5ff214628438f1`. Bootstrap exited 1 during receipt
construction after 201,634,911,689 ns. Complete staged status occupied 68,785 bytes;
the minimal JSON status occupied 69,571 bytes, exceeding the existing 65,536-byte
receipt ceiling. The failure receipt SHA-256 is
`8e508a522ba07742ac0a5d2adf4d5d16cdb9423f7ce3c276f8d5511d08fea7f6`.
No source drift was observed. Literal selfhost verification, changed checks and full
gates had not begun; this attempt supplies no passing acceptance evidence.

The three-file bootstrap repair, SHA-256
`bec184b896b7cd184d88d96576de6fa1e18db71adb4aba92e9c33cbc2ffc858b`,
was subsequently applied with all three before/after hashes verified. It removes the
final receipt-byte ceiling and captures complete porcelain status through an evidence
file and `jq --rawfile`, preserving prior trailing-LF semantics and the existing source,
context and binary bindings. The actual shell owner was exercised through
`bash tools/bootstrap-verifier.test.sh` in its owned fixture with a mock toolchain.
It passed with exit zero in 14.053s wall time (9.037s user, 5.351s system), including
status/receipt inventory above 128 KiB, default cleanup, archive retention, wrong-tree
refusal before Cargo and a late source-drift control. The log at
`.artifacts/capacity-unbounding/bootstrap-receipt-controls-20261029.log` has SHA-256
`efc2b6ba9d52f9e2b0d83a051951869daff240a342ae3c282e85f97cdebb060f`.
This is shell-owner execution evidence; it does not establish a new frozen-candidate
bootstrap or any full-gate result. Other dev-tool capacity boundaries remain separate.

A separate runtime review found
that `validate_policy` rejected `Some(0)` in all four cumulative policy fields before
execution, incorrectly rejecting pure computations for a zero capability allowance.
The required correction admits zero as an exact finite allowance and rejects a charge
that exceeds it before the associated work. A public CLI red control used a fresh command
project: the default descriptor ran `hello` with zero capability calls, while the same
descriptor with `maximum_capability_calls=0` exited 2 with
`deployment_execution_limit`, reporting that specified cumulative quotas must be
positive. The control receipt at
`.artifacts/capacity-unbounding/runtime-zero-20261029/receipt.json` has SHA-256
`4caf4de19cd74d87cd6a3c24de4de9816c0148103b80cb7f359d3fddeb813f6e`.
It used product binary
`d287ba4e4e95da5e4f31340657e31838af57089bb883afe93c9b4dbda6a304be`
and changed no maintained graph. The subsequent combined runtime patch, SHA-256
`24abb05504b51dbdcb9838f5a32dcdec1261d173c2aaae2bf9be04b6677b46e6`,
was applied across 24 paths with every before/after source hash verified. Its guarded
manifest SHA-256 is
`97bb8731929f2dc90eaa9ec78d6ff6560ae60070d885a863af2eca5574d47f9b`.

Implemented scope: all four cumulative policies admit exact zero; public VM/reference
observations distinguish exact maximum from saturation with four companion flags;
structured-child merges, test receipt totals, compact CLI fields and HTTP projections
preserve those flags. Allocation/item counts, flags and shared finite reservations
commit together. Per-record field-name storage uses a checked physical byte sum
instead of a saturated observation delta. Canonical grant allowances and positive
structural call-depth/value-stack validation remain binding. Other general ValueWork
counters and structural limits are outside this four-counter repair.

The production flags are `instructions_saturated`, `allocated_bytes_saturated`,
`collection_items_saturated` and `capability_calls_saturated`; the reference uses
`expressions_saturated` and the same other three names. Public counter semantics are
`per-counter-saturation-flag-true-means-lower-bound`. Root integration also updates
the discovery registry, real public CLI controls and HTTP observation/projector paths.
The first combined formatting/Clippy run exited 101 with exactly two `E0063` errors:
the complete root VM and reference observation initializers each omitted the four new
false saturation flags. No library or public CLI tests ran. The log at
`.artifacts/capacity-unbounding/capacity-policy-controls-20261029.log` has SHA-256
`b5587a6057a044c73653db06d0045263bdcfafd23e7d2ad773ae969e74d5d401`.
A guarded two-file correction added the eight false initializer fields. Its manifest at
`.artifacts/capacity-unbounding/runtime-observation-initializers-20261029.json`
has SHA-256 `ef61becf9527df60df17eb119c4e3f8b92ee9c0c23baa7c0a47ace4e43a34bc7`.
The second attempt exited 101 before tests with 24 Clippy
`bool_assert_comparison` errors in the two child-observation tests (12 per file).
Its log SHA-256 is `749c67263e4e0c0d3d0bb5d19c50c6a9d4b9a1f59e045fa4806c7753852ef6a1`.
A guarded correction changed only those assertions to `assert!`/negated `assert!`,
preserving their substantive checks; the correction manifest SHA-256 is
`74697af59cad6b5daaab7396a63773ca5c344478fd890913e7dc8c332a141686`.
The third attempt exited zero. Release workspace/all-target/all-feature Clippy with
warnings denied passed in 39.40s. After an 8m15s release build, 82 library tests passed
in 1.42s and three real public CLI tests passed in 2.43s, with zero failures. The
selection covers all 46 change-budget dimensions, store/map/kernel boundaries,
VM/reference zero/exact/maximum policies, shared reservations, child saturation flags,
atomic refusal and public policy reporting/invalid-policy controls. After a 3m20s
dev-owner build, `shared_bootstrap_admits_only_source_bound_cold_artifacts` passed
in 14.01s. The log at
`.artifacts/capacity-unbounding/capacity-policy-controls-20261029-round3.log`
has SHA-256 `a015f9a11beb4ad251bebe36524cf5c26a19a6883981a084957250e5a163e6e4`.
The resulting product executable has SHA-256
`1e02a3c983a448ca9ff54cabd6aee8edec4f691fb16cd1034f8f5cd3fc3d4420`.
Public replay of the same historical descriptor subsequently passed. Default execution
and an explicit zero capability allowance both returned `hello`, with three instructions,
1,279,697 allocated bytes, zero collection items and zero capability calls; all four
saturation flags were false. The runs took 81,130,102 ns and 72,852,359 ns respectively.
The descriptor SHA-256
`765c06b594ef7803f205b9432d2326c40d3c2eb1b2a9d310d860021c6d9c55cb`
and project HEAD SHA-256
`003ec04e639cad0ecbf1530a6f10c8f7a83ee292a2cc9366c17f1a093ea0bdfe`
remained unchanged. The green receipt at
`.artifacts/capacity-unbounding/runtime-zero-green-20261029/receipt.json`
has SHA-256 `c729f8c607d455fc84a7133dae53ddef7f4665e0a432e7036573b34982c87b16`.
Discovery was regenerated after the runtime correction, as recorded below. Those focused
results precede the exact-tree attempt and fixture correction recorded next.

## Type-scope fixture and verification scheduling correction

Implemented: the source type-scope policy fixture now requires finite zero/one
refusal and finite `u32::MAX` success with the expected semantic-state bytes in both
ordinary and self-produced paths. Its four-byte prefix always selects a finite
allowance; it does not encode `None` or `u64::MAX`. Malformed-carrier diagnostic 12,
unsupported-type diagnostic 13 and the separate exact-boundary control remain.
No production policy ceiling or maintained graph was changed for this correction.
The literal selfhost script now honors positive `RUST_TEST_THREADS`, defaults to two
only when unset, rejects empty/nonpositive/nondecimal selections before work, and
records the selected count. Both filters, their order, the target lease and complete
input hashing remain unchanged. The next seal explicitly selects eight threads;
this is test scheduling, with no claimed speedup or peak-memory bound.

Observed: the second frozen attempt used source
`dad67f97c6b9b008300d59d83e3e4e7540acfa4f`, tree
`a5f1cb9b45dd0fa5613b2d21e18cf6950a4a95f2`. Bootstrap passed in
201,260,609,952 ns and emitted a 71,236-byte receipt, SHA-256
`ed039e8cca755230008ae693d8fef580c660e91eb2a07176ef23cd5b9eb4c7a0`.
The selected verifier SHA-256 was
`3d5b74bfaaec12f63a21e3318ef0280c4a2ca088154dcbfdb111ef2a734fb315`;
its Cargo output was reused, without claiming cold compilation. Literal selfhost
verification then reported 137 passed, one failed and two ignored in 6,947.29s.
The sole failure expected diagnostic 6 for the ample finite `u32::MAX` prefix;
the validator correctly returned diagnostic 0 with semantic-state bytes. The script
exited 101 and the sealing driver joined with exit 1. Its checkpoint SHA-256 is
`ded2211dd3c190817b6c62944fcfb00d5d901d93386047524239bec8af0abe23`.
The 330,183-byte log preserved at
`.artifacts/capacity-unbounding/seal-20261029-round2/failed-selfhost-latest.log`
has SHA-256 `b1548e6a42a90a0c78eef957a65acf96c6d71d37c946a731d7d8cc72aaa24682`.
The script's `native_elf_` filter, separate named controls, changed/full checks and
release target did not run in that attempt.

After the fixture correction, both selected type-scope tests passed, with zero failed
or ignored and 2,835 filtered out, in 354.23s after a 6m06s release build. The Cargo
command took 720,608,089,478 ns including compilation. The exact composed allowance
was 3,108: 3,107 refused and 3,108/3,109 succeeded in the ordinary and self-produced
paths. Zero/one refusal, maximum-prefix success and diagnostics 12/13 also passed.
Shell syntax and workspace formatting checks passed; the four explicit thread values
empty, `0`, `-1` and `invalid` each returned exit 2 before work. The 3,825-byte log at
`.artifacts/capacity-unbounding/type-scope-fixture-controls-20261029.log` has SHA-256
`d82b5b6553ec3ecb93cf1b89e247f21be6649ce564c5c07c323a6bbac9fe5bae`.
Source and driver identities are recorded in the source adoption evidence.

At the end of round 2, pending work was to freeze the repaired tree, bootstrap it and rerun the complete literal
selfhost/native script, both named controls, all 29 fresh full gates
and release target. The failed round-2 tree and focused repair results do not accept
the repaired tree. Unproven: complete unbounding, Tier 2 source-driven fixed points,
native phase reclamation, bounded same-process RSS and test-throughput scaling.

## Public policy fixtures and observation-discovery correction

Implemented: the seven-file repair replaces stale authoring and CLI expectations without
restoring default quotas. The raw-reference control distinguishes omitted and explicit
unbounded policy from finite operations allowances 0, 1,000, 1,001 and u64::MAX.
Its 1,000 raw aliases still count before selector deduplication. Discovery checks require
all 46 absent default change allowances, and an explicit query byte refusal retains its
resource diagnostic and exit status 4. Requests cross the former 4 MiB byte and
10,000-selector boundaries; malformed trailing input and all existing namespace,
pagination, continuation and nonpublication checks remain. Cumulative authoring succeeds
without a policy and refuses an explicitly selected 100,000-identity allowance.
Type expansion uses checked host-size overflow as its representability negative.
Repeated effect references reach dependency-integrity admission. The authored-type-node
policy continues to count distinct canonical interned types, not decoder copies.

The public native compiler fixture now authors the complete 73-file compiler in one
reviewed public plan/apply and independently constructs a 896-byte Mode-0 profile-6
input. It retains detached execution and the malformed-magic diagnostic-1 control.
This is a fixture/protocol correction; it does not add a legacy reader or bypass the
authoritative graph. The guarded repair bundle SHA-256 is
`19cb55d69b5c4c15eab16955fed98432821c20b00a9ef57e2bd64931792aeed8`.
The guarded replacements across seven paths and formatting completed under the target lease with exit 0.
The application receipt SHA-256 is
`0b95b2e962415b34be2f9392df2c31a467ee20196f31b33b27231d5deef728e0`.
The corrected cases subsequently passed across the focused iterations recorded below.

Observed: the third frozen attempt used source
`dad67f97c6b9b008300d59d83e3e4e7540acfa4f`, tree
`82cc35da153202dca6e06d40e52c955c0a229bfa`. Bootstrap passed in
9,259,626,220 ns and emitted a 71,339-byte receipt, SHA-256
`61c9133aea4de1b5c4d2ec4e1f1321096b2de2d3cf3709f9158763e09ed8feb9`.
It reused four verified Cargo outputs; this is separate from gate reuse.
Literal selfhost verification selected eight test threads and passed 138 tests,
with zero failed and two ignored, in 3,368.31s. Its native filter passed all 11 tests
in 119.60s. The explicitly selected maintained-native-closure measurement passed in
22.83s and the whole-owner/type inventory passed in 13.09s.

The changed profile selected and executed all 29 gates. Its terminal receipt reports
27 fresh passes, zero reused passes, two failures and no unrun gates, with stable input
digests. The failed gates were `workspace_tests` and `pure_tail`; the sealing driver
joined with exit 1. The main library result was 2,829 passed, one failed and seven
ignored in 3,272.73s. That exact terminal result is retained in the compact receipt's
stderr excerpt; it was not inferred from the removed transient directory. The public
CLI harness reported 343 passed, seven failed, one ignored and one filtered out in
1,872.10s. The development library separately passed 338 tests with 16 ignored in
15.10s. The driver did not start the fresh full check or release target.

The workspace failures included stale quota/discovery expectations and the old
profile-4 positive packet. The native fixture's actual rejection was exit 65 with
`compiler_rejected:2`, distinct from its preserved malformed-magic diagnostic 1.
The `pure_tail` failure exposed a production discovery defect: allocation-byte and
collection-item scalar observations were each emitted by both the existing unit
description and the new saturation description. Its uniqueness assertion correctly
rejected the duplicates. The captured discovery contained 74 records for 70 distinct
(record, field) keys, with four duplicate keys. A subsequent production correction
combines unit and exactness metadata into one record per key and adds a public
discovery regression. It preserves all 70 distinct keys and the strict consumer
assertion. The repair and formatting joined with exit 0; its receipt SHA-256 is
`7d77a0a74ae0c5877e927e4f2fb91c55f7b80c8809dad7c664f62737cefb282e`.
The actual public runner then reported 70 records, 70 distinct keys and zero duplicates,
retaining the four cumulative units and their linked exactness flags. Its 2,530-byte
observation receipt has SHA-256
`f7d231bfc9e7290432cfb067491e58d35ff2781fa702ee65cd07d2d6304eb8fb`.
The product executable SHA-256 is
`40ac2639720ff6aafeb7a216acb13b81f73582dc960026367dc4175f19f72b72`.

| Terminal evidence | SHA-256 |
| --- | --- |
| Round-3 checkpoint, 9,348 bytes | `210f4d6ab04351edace177afb4c11b4980b7e54c1595926997785c54e76cebbf` |
| Failed changed-profile compact receipt, 26,187 bytes | `70fc42b66fa4ada6e89bd0341a181d537c12d2ce32627e93aa42941498cf2a65` |
| Failed-seal observation, 8,718 bytes | `f55c53c9645a7771301218b5169add3ff4e911b60790cdc7c845f67d8df6c349` |
| Preserved public CLI stdout, 253,631 bytes | `365ec22f6a65dfb8346cf68fc0f52245359da89d8fa8f676ca5f97dab020ac74` |
| Preserved public CLI stderr, 163,969 bytes | `85008a2007b2cd3a031598b5e75680658410a38d9075a1e1aeecb20771b193d2` |

### Focused repair iterations and scoped verification

The first focused attempt failed during compilation with E0603 because the public CLI
test named the private `platform::change::ChangeBudget` type. Cargo exited 101 after
482,115,972,051 ns; the driver joined with exit 1 and no tests ran. The test now lists
the 46 public policy names while retaining each default, syntax and finite-policy
assertion. This changed no production API. Its application receipt SHA-256 is
`549e2df694a57366fffd61f83e3ce8709b41747c3dddf0e2de58a79805859026`.

The second focused attempt passed the library control in 0.63s and six CLI controls;
its CLI harness reported six passed and one failed in 52.40s. The remaining capability
test stopped at the old operation inventory. Actual public discovery confirmed
`set.parameter-order`, 187 operation fields, 37 field forms and its two optional
function/operation selectors. Those four expectations were corrected; the application
receipt SHA-256 is
`6b2664b53071d0ded659c07daceb5337acaa21bd00aa37f616763d02e1cf2c6a`.
The detached native compiler case passed in this selection, including the generated
program's result 42, malformed-magic diagnostic 1 and unchanged planning/building HEADs.

The third focused attempt selected only the remaining capability test: one passed,
zero failed or ignored and 351 filtered out in 0.09s. Thus all eight repaired cases
have observed passes across the second and third iterations; there was no single
eight-pass run. The six-operation verification driver joined with exit 0.
The release workspace/all-target/all-feature Clippy command with warnings denied took
38,477,898,328 ns and the verifier build took 189,970,776,717 ns; both passed.
These focused iterations are not additional frozen-tree acceptance attempts.

| Focused verification receipt | Bytes | SHA-256 |
| --- | ---: | --- |
| First, compile failure before tests | 1,902 | `b7773c5695e24230367a80a019eff6f78118b282e6e3c7164c8387de97c55b0c` |
| Second, seven passes and remaining capability failure | 2,015 | `f6cb270bc9fb9a17ebfa644a968a2b1e7995117ed3456bed452d1432836d982b` |
| Third, remaining capability and complete semantic/discovery checks | 7,881 | `781c060d2465944d4301039f7e86b4ac4a9acd177ece9d7434fbdc698a3766eb` |

The complete pure-tail semantic command passed in 48,700,706,647 ns. Its semantic
receipt records 48,229,136,030 ns, 82 commands, 111 source-batch result rows and
36 checked-value discovery fields. Source authority was unchanged; cleanup completed
and the temporary root was absent. The 10,366,172-byte receipt has SHA-256
`ff9c0e9aa46211f01d9ea74b73f4548c195811e59a01ae7d7bbac1d52478581c`,
and the verifier SHA-256 is
`ce1530f6d3370aed0bfbc02f9ed43845dc832087a9828187bf75408a11a60019`.
This establishes the separately invoked pure-tail semantic check; the required full gate
run remains separate.
Public discovery published and verified all eight generated documents with capabilities
digest `8f83c580617ae792d2781460722bfa0d3c3a2a8c480ab62719ae5984a232a85c`.
The source adoption evidence records every generated file's exact byte count and SHA-256,
the operation logs and the final hashes of all seven changed source paths.

A separate disposable-project probe verified complete compiler publication and native
build from the 962,626-byte request. Public planning took 10,371,992,126 ns, applying
took 16,155,444,085 ns and native building took 29,810,377,200 ns. Planning and building
preserved HEAD; the temporary graph and ELF were removed. Its receipt SHA-256 is
`deb66f381d9514e7101d612a74242eb8798bfad6d99909ba873aa72ebf949590`.
The emitted 431,620-byte C1 image had SHA-256
`c68646c5486588a60362824f0d58c3cccc98bfdd4234ba1ece19782782602e95`.
This probe establishes setup/build behavior, not detached packet acceptance.

At the end of the failed round-3 attempt, integration still required freezing the repaired staged tree and bootstrapping the independent
verifier, running the complete literal selfhost/native script and both named controls,
all 29 fresh full gates with zero reuse, and release target. The prepared next driver had six
operations; it omitted an optional changed-profile repetition because that selection already
covered all 29 gates. The required fresh full proof was unchanged. Its prepared
driver SHA-256 is
`5d7edad40956f414804999f2acb10627fdc15435171e1f40d83dd87dec3e0a84`.

At that historical checkpoint, acceptance of the repaired staged tree was unproven.
The subsequent Phase 1 seal resolved that acceptance claim only. Complete unbounding,
Tier 2 source-driven fixed points, native phase reclamation, bounded same-process RSS
and test-throughput scaling remain unproven.

## Tier 2 evidence boundaries

The maintained source inventory (`S_compiler`), its used features (`F_active`), and the independent
required language profile (`P_self`) remain three different inventories. Rust-normalized backend
reproduction, a restricted scalar raw-source fixture and rejection of an unused invalid descriptor
are useful controls; none proves whole maintained-source self-compilation.

The required immutable raw-source equations remain:
`C1=C0(S,E)`, `C2=C1(S,E)`, `C3=C2(S,E)`, with `bytes(C2)==bytes(C3)`.
Canonical source-derived artifacts must satisfy `A_host==A1==A2==A3`.
No new native compiler stage may receive a Rust-selected owner subset, lowered entry index,
projected compilation-unit stream, or host semantic certificate in place of the complete carrier.
The request TargetId is transported verbatim and resolved inside lkjscript after whole-carrier
admission.

Still required: general containers/nominal forms/generic substitutions, closures and recursion,
witness/associated/read-view semantics, lexical regions, same-process phase reclamation,
independent interpreter positives, hash-consistent unused/unselected/untaken negatives, meaningful
source mutations across stages, and strict ELF/.lkja loader checks. This ledger does not narrow
that acceptance profile.

## Scoped pre-acceptance evidence and adoption state

The checks in this table are earlier focused executions; the accepted Phase 1 full seal is
recorded separately above. They do not establish Tier 2 proof. All six
maintained repositories have completed public current-format republication. Their canonical object
inventories and semantic-state commitments were independently compared with their authenticated
transport exports. Exact bindings and stale-HEAD refusal evidence are retained in
[the republication record](20261027-policy-publication-evidence.json) and interpreted by
[the publication cutover decision](20261025-policy-publication-cutover.md).

| Candidate check | Observed result | Material limitation |
| --- | --- | --- |
| Compiler admission, loan/read-view protocols, artifact inventories and native capacity selection | 88 library tests passed, 0 failed, 0 ignored; 118.43s runtime after an 8m16s release build | Includes publication and native execution of 4,097 functions beyond 65,536 instructions; does not exercise every repository gate |
| Public CLI symbolic deletion control | 1 passed, 0 failed; 4.80s runtime in the same Cargo invocation | Proves the existing executable transaction/deletion boundary, before the new parameter-order controls |
| Compiler correction workspace/all-target Clippy | Passed with `-D warnings`; 39.33s | Static integration of that candidate, before the subsequent parameter/query changes |
| Integrated parameter-order and query workspace/all-target checks | Check passed in 58.39s; Clippy with `-D warnings` passed in 46.18s, then 16.70s after descriptor correction | Static integration; separate execution evidence follows |
| Parameter order, transactions, capability descriptors and normalized query | 48 library tests passed in 6.40s and 3 real public CLI tests passed in 7.36s; 0 failures or ignored tests after an 8m16s release build | Exact finite/unbounded query policies, suffix continuation, final parameter inventories and atomic deletion; full gates remain separate |
| Post-adoption selfhost selection | 7 library tests passed, 1 failed; 1,335.91s runtime after an 8m13s release build; Clippy passed in 16.43s | Maintained replay, optional policies, large-byte carrier, whole-owner/type profile and native closure passed. The entity-capacity negative fixture failed during Rust construction before its native diagnostic; the subsequent corrected run below reaches and verifies that diagnostic |
| Corrected retirement fixture and complete replay selection | 5 library tests passed, 2 failed; 332.88s runtime after an 8m19s release build; Clippy passed in 17.49s | Registry definition, complete 641-unit/native-input replay, whole inventory, native measurement and retirement entity control passed. Interim nested-record stubs and unscoped wide-probe entry selection exposed two fixture defects; the following green run verifies both corrections |
| Nested-record reconstruction, scoped wide-address probe, GC and public parameter/type followup | 14 library tests passed, 0 failed, in 600.12s; 6 real public CLI tests passed, 0 failed, in 36.54s; 8m16s release build | Both repaired selfhost tests and all 12 selected GC controls passed. Workspace/all-target/all-feature Clippy with `-D warnings` passed in 39.43s. Public maintained-project GC, asset comparison and complete staged-tree acceptance remain separate |
| Public adoption readback | 9 owner pages, 35,952 live owners, all 2,832 expected namespace entries and 23 changed signatures verified | Read-only evidence tied to the accepted revision and a separately identified verification executable; no full-gate or raw-source fixed-point claim |
| Optional memory-project admission | 5 passed, 0 failed | Explicit finite policies and shared reservation atomicity; no claim about native process RSS |
| Wide region suite | 46 passed, 0 failed; 1.90s runtime after a 3m release build | Host VM/reference region storage and lifecycle only; native compiler phase reclamation remains pending |
| Persistent-map read cache suite | 4 passed, 0 failed; 0.01s runtime after a 2m59s release build | Exact warm admission after saturated cold observations, including `None`/0/31/32/MAX for a 32-entry page |

The first public compiler-proposal transaction was rejected atomically with
`source/change_unit_order`: an existing callable gained a parameter in the middle of its signature,
while public value-parameter additions could only append. The accepted selfhost `HEAD` remained
`088ae65054a7c63617e45f3f82decd04e18a116390c342e10d3eb6defa45c781` before and after that refusal.
No proposed compiler body was adopted by the failed transaction.

The signature audit identifies 23 changed existing callable parameter lists, 21 added parameters,
and three removed parameters. The implemented correction is a typed final-order operation after
explicit deletion lowering. It requires an exact bijection of surviving parameters, supports new
symbols, rejects duplicate or foreign members, and changes only the parent's ordered inventory.
Retained parameter identities and records remain unchanged. The corrected public transaction
explicitly deleted those three parameters and two functions, and accepted all selected proposals at
`rev_e5fa56ece633987cb2f62a3e7cdda4aec58fa74aa9cc7008f71682ca289b2f51`. Its accepted HEAD is
`f5436f724c2a88123f5db0ade85ec23af808a9dc70635beb490198c688027507`. The executable reported
13,654 created owners and 239 updates, with zero selected project tests. Nine public owner pages
matched all 2,832 expected namespace entries among 35,952 live owners.

Definition readback exposed an omitted `projection.saturated` capability field. The renderer's
field is now advertised. V3 verification completed at 2026-10-28T08:48:25.740Z with status
`revision-inventory-and-signatures-verified`, checking all nine owner pages, all 2,832 expected
namespace entries and all 23 changed parameter orders. Its receipt SHA-256 is
`1aaa3d3ef1e5fe5906f75a0fd27d253bd0d7b80a8614cd5b3fc0164a1c50c428`.
The verification executable is recorded separately from the historical publication executable.
Readback and public transport export preserved the accepted HEAD; the exported container SHA-256
is `cce8632f8a18faa2da549a3df134a3b679966f0fba5cfb903fa440f3e7166981`.

The accepted source has 35,952 owners, 41 types, 24,864 retirements and 29 structural record shapes,
with semantic state
`semantic_state_66d01c7280e302fa390e1449e148893c23b0aea46b1fb7c7dc9033cc81a318ec`.
Maintained proposal replay matched all 641 canonical units and the native backend closure.
The whole-owner/type inventory and maintained native-closure measurement passed. The latter's
report SHA-256 is `e97491880214ae5e586996230d46f6b4b7f7cff5fe8455951a2cafb0a9ce1478`.

| Focused native reproduction | Bytes | Observed SHA-256 |
| --- | ---: | --- |
| Mode0 profile6 C2 = C3 compiler | 1,246,043 | `342ec8bcf60d548225d5818c3d51677d08f5fe987d59c601307c17be82202ed6` |
| Mode1 profile5 C1 = C2 = C3 artifact, 641 canonical units | 10,292,518 | `a441d5740bf81e9b292e0963ae9cae9917cc61473ac12b52eed3f768e74d598c` |

These measurements cover Tier 1 reproduction using Rust-normalized inputs and the disclosed
C1 bootstrap. They do not measure native live retention, mapped capacity, RSS, phase reclamation
or maintained-compiler raw-source self-compilation.

The post-adoption eight-test selection finished with seven passes and one failure in 1,335.91s.
The retirement-capacity positive native case admitted 100,001 retirement rows. Constructing its
negative carrier then failed at the strict Rust `semantic_state_digest` oracle with
`kernel_state_retirement_key`: the deliberately changed retirement owner disagreed with its map
key before the native reader ran. The applied fixture correction retains that strict oracle for ordinary
inputs, explicitly commits the invalid fixture's actual map rows, and independently checks state
and object binding before requiring native diagnostic 12. The corrected run passed the positive
and layer-reaching negative controls described below. The applied replay correction
consolidates complete source reapplication into one public transaction in unmetered memory storage
while retaining every canonical-unit and native-input comparison. Region-expression definition
descriptors now advertise the emitted `type` and `capability` fields. These source corrections were
applied with all ten patch result hashes verified. Clippy passed in 17.49s after those changes;
release compilation finished in 8m19s. The seven-test selection finished with five passes and two
fixture failures in 332.88s, with log SHA-256
`9bb64aed64a8f4639f90e3b5af37cd4374ea325015894d9bd6eb58e6bf91cfcc`.

The 100,001-retirement positive carrier contains 19,500,337 bytes, SHA-256
`4771532162eecf63f570fc1cb67ca196be322dc589729dfa41d990f82f2f265e`.
The negative carrier has the same size and row count, SHA-256
`3b08495e1bf99679bc47fe9aa7b94d92e9f94bba36c528566cf0a7452f15aecb`, and state
`semantic_state_23e9d0ea5afc99134e6882f8d43790c6c08fc3ce725aeb73afdd0b2cedda1892`.
Its independent `read-state` commitment check and matching-key retirement binding passed;
changing only the binding key produced diagnostic 12, as did `read-carrier` and `read-graph`.
The run also reconfirmed all 641 canonical units/native input, the complete owner/type profile
and the same Tier 1 compiler/artifact fixed-point hashes recorded above.

Two remaining fixture defects were isolated: interim function reconstruction did not support
nested structural-record results, and the wide-address control selected an unscoped `assemble`
from the compiler instead of the fixture's module. Applied corrections use an explicit heap
worklist for nested-record stubs and select `wide-encoding-probe::assemble` within its module.
The complete proposal inventory contains 73 files, 613 functions and 29 record shapes; 18 functions
return nested records, with observed maximum depth two and Bytes/I64/Bool leaves. These counts
describe the inspected source and introduce no traversal quota.
The deterministic mutation fixture reconstructs its baseline and mutant independently: each
starts with the initial `ci` graph and attempts 255 incremental transactions, comprising 74
header/target transactions and 181 body groups. The 510 total attempts are a static fixture
inventory, separate from the one complete transaction used by maintained-definition replay;
an attempt count does not establish completion or accepted publication count.

GC preparation now propagates `StoreWork.saturated`, marks local observation overflow, and exposes
`canonical_bytes_retained_saturated` separately. `references_visited` retains its unique,
deduplicated queued-binding meaning. These observations do not change admission. The combined
five-file fixture/GC patch, SHA-256
`e707561785ae1886df454f5d4c33f8146de830fae37f793756e3d537c250f37f`, was applied with all result
hashes matched. Workspace/all-target/all-feature Clippy with `-D warnings` passed in 39.43s on
that patch. The completed followup passed all 14 library tests in 600.12s and all six real public
CLI parameter/type tests in 36.54s, after an 8m16s release build; the process exited zero.
Both repaired selfhost controls and all 12 selected GC tests passed. The complete log SHA-256 is
`f12545ff7c565318773b64384728cfe689ce20a4398adcf580b9c617f40ec1d3`; the resulting product binary
SHA-256 is `d287ba4e4e95da5e4f31340657e31838af57089bb883afe93c9b4dbda6a304be`.
Public maintained-project GC, asset comparison and discovery generation subsequently completed
with exit zero. The GC receipt SHA-256 is
`915238fdee4c3f12ad0bd64acce6c9f465a9cd8e991b482d8333d2b9fa0c24b7`.
All six repositories retained their exact HEAD bytes and complete transport SHA-256, package
transport digest and revision. Both preparation and retained canonical-byte saturation flags
were false. Physical packs decreased from 662 files / 499,175,240 bytes to eight files /
79,757,990 bytes, reclaiming 419,417,250 bytes. The selfhost repository decreased from
584 files / 483,831,900 bytes to three files / 65,267,662 bytes. These measurements describe
pack storage; they establish neither native memory reclamation nor bounded RSS.

The maintained-asset comparison receipt SHA-256 is
`2e11c863ec3f76f71795464daf3ae9935312b59ae05e102d6601f24a7c13608f`.
All five comparisons matched exact bytes; no maintained asset was replaced. The three checked
HEADs, four maintained assets and product binary stayed unchanged. Public package checks
passed 116 standard, 44 application and 176 package-plan tests, with zero failures and equal
differential results. Discovery regenerated all eight documents through the public owner;
the capabilities digest is
`19b49f284181ba6e41ef96ed7128f8995f94c0aaf66e4da85a09ea9534483aed`, and generation stdout
SHA-256 is `70717d5bf71a2de4006b4e66bb81b1b2135129dc936d763a59ea5755ba7f28fa`.
After the cumulative runtime policy/telemetry correction, the public owner regenerated
all eight documents again. The current capabilities digest is
`b007ea80efedbddc7994804207dc458d910bfe92879ac247903838d54b54fb38`;
`.artifacts/capacity-unbounding/generated-public-20261029-round2.log` has SHA-256
`738bc61adf60aa6c5a26e9e5a1b88357bada990845013e88f15bbc15fb86c368`.
The curated integration record preserves per-project GC identities and exact asset hashes.
Physical pack-byte accounting and catalog pack/leftover limits remain explicit WS2 work.

Mode0 profile6 and LKJSG002 proposals are publicly adopted. Mode1 profile7, remaining
storage catalog/object-policy changes, source-driven fixed points and full `P_self` remain separate
obligations. No focused result above proves them.


## Coupled history, storage and compiler correction

Implemented changes: the 4,097-function authoring fixture reached a 4,343,417-byte normalized
transaction and exposed a 4 MiB publication payload ceiling, followed by the same domain ceiling
in immutable storage. Transaction and semantic-diff envelopes now use audited progressive
collection decoders and fallible streaming encoders with optional exact payload policies. These
types have no default byte or inline edit-count quota. Canonical byte equality, checksum, epoch
and exact framing checks remain. Unrelated recursive codecs retain their separately cataloged
lifecycle boundaries; this is not a repository-wide codec-completion claim.

Immutable reads now take an optional per-object allowance. The transaction and semantic-diff
object domains have no logical byte ceiling. Pack payload/entry/count quotas are removed; pack
framing uses checked 64-bit coordinates and exact file extents. The 32 MiB packing target remains
a physical grouping choice. Footer header/codec/extent checks precede index allocation. Deflate
decoding grows only from actual produced output in 64 KiB chunks instead of preallocating an
untrusted claimed raw length. Catalog aggregate limits and other bounded object domains remain
pending independent cuts.

The same fixture next exposed an explicitly finite 128MiB memory-project policy and an implicit
10M compiler reconstruction allowance. Ordinary memory-project callers now select `None` through
an optional exact aggregate policy. Compiler reconstruction and parallel proofs share logical
admission independently of canonical-read telemetry, with default `None`; derived-type metadata
also has no implicit 64MiB default. Root and child worklist growth is prepaid before allocation.
Unit loan/read-view proofs and artifact implementation/reference-type/nominal/interface-source
inventories now report count plus saturation under optional exact admission. Active-path cycle
checks and protocol diagnostics remain semantic requirements. The focused compiler run above
observed the completed 4,097-function publication, compilation and three-way execution result.

Query output admission likewise distinguishes omitted policy from every finite allowance. A page
uses one repository range read, measures its actual response, and removes suffix records only if
that response exceeds the explicit byte policy. Trimming retains all observed read work and uses
the last emitted logical key for continuation; it does not rescan or fabricate worst-case telemetry
to decide whether a page fits. A complete terminal page is tested before introducing a continuation
envelope, which can be larger than the terminal response. The focused runtime controls above
passed alongside capability-descriptor and public transaction controls.

The source-bound Phase 1 seal and normal integration are complete and bound above.
The subsequent list candidate requires its own frozen tree, independent bootstrap, literal
selfhost/native script, named closure/inventory controls, all 29 fresh full gates with zero
reuse, release target and terminal integration readback. Focused evidence for a later edit
does not inherit acceptance from Phase 1. Record terminal outcomes in the acceptance
commit and external record without editing frozen source to add a passed claim. Tier 2
completion and bounded native RSS remain unproved.
