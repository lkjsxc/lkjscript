# Capacity removal: boundary census and integration ledger

Updated 2026-10-29 (Asia/Tokyo).
Status: active implementation; the tables distinguish applied candidate changes from remaining work.
Reference source: `47e33ae90ec77b176597e45d7917694749526aba`.
Reference tree: `32ff68b548b2fcb5aecd8c01229b8a7c757490d5`.
Final commit and exact staged-tree verification belong to Git and the acceptance receipts.

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

| Enforcer / symbols | Callers and serialized path | Category and coupled disposition | Candidate state / verification |
| --- | --- | --- | --- |
| `storage/object.rs::StoreReadLimits, StoreReadAdmission` | Directory/memory reads, public read views, package-source reads; joint caller budgets | 1/2. Three `Option<u64>` dimensions; exact remaining allowances, independent observation; combine nested policies by intersection | Applied. Zero/exact/one-over/MAX, atomic failure, and nested allowance tests; storage filter passed 52 tests before later integration |
| `storage/object.rs::StoreWork` | Directory, memory, publication, map consumers | 1/2. Saturating passive counters plus aggregate and per-counter exactness flags; never infer admission charges from saturated observations | Applied to direct StoreWork updates. Higher persisted work DTOs remain a separate cut |
| `persistent_map.rs::MapAdmission, MapWork` | Map load/scan/mutate/encode, compiler link, artifact, publication, package transport | 1/2. Optional page/byte/entry allowances, atomic checked preflight, independently merged saturation flags | Applied. Map filter passed 44 tests before later integration. Saturated cold-cache observations preserve exact warm-cache charges (4 cache tests passed). Witness rebuilding propagates optional per-owner fanout and per-test dependency admission |
| `kernel/validate.rs::FullValidator`, `kernel/work.rs::ValidationWork` | Full validation, inference, affine/callable/intrinsic admission, artifact validation | 1/2. Default work is None; exact finite charge; report saturation separately from exhaustion | Applied. Direct MAX/saturation, resumed inference above the former 10M threshold, affine and cancellation controls added. Focused validation and current-only republication succeeded; full staged-tree acceptance remains pending |
| `kernel/{callable_flow,composed_callable_flow,nominal_flow,owned_associated,owned_record}.rs` metadata admission | Whole-carrier callable/type evidence and compiler artifact consumers | 1/2. Remove hidden work and 64MiB metadata clamps; carry optional budgets and saturated modeled observations | Applied lower-level work/metadata cut; remaining reference/package/parallel-layout callers must be audited independently |
| `publication/validation.rs::REBUILD_LIMITS`, `publication/repository.rs` full oracle | Revision-pinned reconstruction and dependency validation; current read-view output | 1. Remove automatic store rebuild quotas and redundant 10M reconstruction ceiling; preserve structural checks | Applied; current validation output carries work-saturated. Rebuild/publication integration gates pending |
| `contributor.rs::SemanticInventory`, dev scale `OracleEvidence` | Typed inventory to JSON scale receipts | 1/3. Carry validation/map/store exactness flags; discriminate the changed scale receipt contract | Applied contract-4 inventory/scale observation patch; full dev gates pending |
| `change/budget.rs::ChangeBudget` and ten sub-admissions | CLI/control authoring, preparation, repair, selection, refresh, publication revalidation | 1/2/3. Replace 46 scalar default maxima and MAX sentinels with optional exact policies, remove engine clamping, preserve operation-zero semantics | Applied optional 46-dimension policy with exact finite admission and passive saturation. Focused integration evidence is recorded below; complete gate acceptance remains pending. |
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

| Enforcer / symbols | Callers and serialized path | Category and coupled disposition | Candidate state / verification |
| --- | --- | --- | --- |
| `compiler/selfhost_input.rs`, selfhost `input/types/custody/flow/profile/values` | Mode0 producer, verifier, certificates, machine emitter, source-scalar pack | 1/3. Dynamic descriptor directory; wide independent type identity and custody qualifier; wide rows/offsets; clean profile cut | Applied profile6 producer/proposals: 160-byte header, dynamic32-byte descriptors,80-byte functions,64-byte instructions,16-byte identity/qualifier cells. Publicly adopted. Focused Mode0 C2=C3, maintained proposal equivalence and native closure measurement passed; complete staged-tree acceptance remains pending |
| selfhost `machine/asm/elf`, `source-scalar-{plan,state,emit,pack}` | Local/stack state, sentinel, ABI frames, emitted branches | 1/3/5. Replace local128 sentinel and fixed slots together; checked wide frame addresses and deterministic wide control flow | Applied with profile6: owner-count sentinel, variable code layout, checked 64-bit R11 transfers/frame addresses. Publicly adopted. Tier 1 native reproduction passed; raw maintained-source self-compilation remains unproved |
| `compiler/selfhost_artifact_input.rs`, selfhost `artifact-input/unit-wire/unit-map/artifact` | Mode1 profile5 projected units to canonical .lkja | 1/3/4. Widen u32 input/pool/frame offsets and 8MiB unit caps; retain hierarchical 256-way radix branching | Pending widths. Existing hierarchical map is already committed and must not be replaced with a fixed table |
| selfhost `source-graph/index/map/scope` and proof passes | Complete meaning-graph carrier, directory admission, scope/type/custody proof | 1/2/3/5. LKJSG002 tag + u64-limb allowance; unlimited skips synthetic proof estimate; structural cursor/DAG termination retained | Publicly adopted proposals and matching Rust test producers. Graph100K and map8MiB/100K caps removed. Large-carrier positive controls and the corrected 100,001-retirement binding control passed, including the hash-consistent negative at native diagnostic 12; complete staged-tree acceptance remains pending |
| LKJSG002 lengths/counts/offsets and per-object codec guards | Carrier producer, every source pass, diagnostics | 3/5. Current u32 extents remain checked; eventual widening must replace all directory/payload consumers | Explicit remaining representation boundary. New policy width does not imply 64-bit carrier extents |
| `compiler/elf_x86_64::{Asm,compile,Emitter,elf}` | Rust native command compilation and ELF publication | 1/3/5/6. Fallible code/labels/worklists; usize label identity; absolute64 relocations/indirect transfers; checked wide frame and record offsets | Applied removal of 4,096-function/65,536-instruction/32MiB-image quotas. Arithmetic, wide-offset, 33MiB ELF and ordinary native controls passed. The public 4,097-function fixture publishes and compiles beyond both former inventories; ELF, VM and reference each return 4,097. |
| `owned_output.rs`, CLI native build, artifact preparation | Create-new executable stage, artifact bundle, release readers | 1/2/6. Default native publication must not inherit artifact byte quota; explicit output policies remain exact; preserve atomic stage cleanup | Applied Option<usize> publisher; ordinary CLI usesNone and explicit0/exact/one-over/MAX policies pass with atomic cleanup. Artifact preparation16GiB and release reader caps remain separate obligations |
| Native `heap/objects/runtime` and Rust ELF runtime | Byte allocation/copy/buffer growth/hash/derive/filter I/O | 1/3/5/6. Remove256MiB per-value checks; checked signed length, header/payload addition, geometric fallback, mmap extent checks | Applied Rust emitter and publicly adopted proposals. Arithmetic edge probes, existing native controls and focused Tier 1 native reproduction passed; same-process reclamation remains unproved |
| Normalized `value/byte_buffer/owned_storage/vm/reference`, `bytes` | Allocation policy, immutable bytes and mutable buffers, VM/reference | 1/2/6. Remove hidden per-value cap; fallible reserve, exact RunPolicy, cancellation and honest merged observations | Applied. Finite0/exact/MAX, cancellation, and historical-bound arithmetic tests added |
| Native allocation lifetime: R14/R15, `heap::allocate`, buffer discard/freeze | Source prepare -> lower -> ELF, repeated process invocation | 4/6. Mapping registry; immutable input, phase scratch, persistent and output domains; explicit escape/copy protocol and munmap/reuse | Pending. Current refill already grows across mappings. Current process-exit release is not same-process phase reclamation |
| `execution/region{,/runtime,/persistence}.rs`, region value/VM/reference | Lexical region slots, relative pointers, persistence and native Region support | 1/3/5. Remove65,536 live-region cap and checked slot growth; widen u32 pointer/capacity/layout/slice fields coherently | Applied LKJREGN2/profile2 with64-bit header/cell/pointer/count fields, uncapped arena slots and iterative shared-DAG layout/codec/teardown. All46 region tests passed. Native Region/domain reclamation remains incomplete |
| Kernel depth guards versus recursive consumers | Normalize/lower/interpret, codecs, display, equality/hash/clone/drop, native call stack | 1/3/5/6. Remove synthetic counters only after recursive lifecycle is replaced by heap worklists or justified safe mechanism | Pending full lifecycle cut. Iterative validator loops are identified; 256/1024 depth removal alone is unsafe |
| Normalized `RunPolicy` call-depth/value-stack and raw admission items | Foreground/default execution, VM/reference, nested invocation | 1/2/5. Optional execution policies throughout; preserve cancellation/effects; stack-safe implementation before unbounded depth | All four cumulative policies admit exact optional limits including zero, with independent saturation flags and focused controls. Default 4,096-call-depth and 1M-stack limits, plus per-value item ceilings, still require a separate cut |
| Normalized `list.rs`, `map.rs`, prepared/reference types/local-liveness | General values and schemas, persistent collection operations | 1/3/4/5. Expand list hierarchy beyond height3/1M items; optional schema/work policies; preserve structural balanced-map height proof | Pending. Map height2*usize::BITS is mathematical; list fanout32 is physical; current logical caps are not |
| `compiler/{unit,artifact,manifest,cache}.rs`, artifact loaders | Compilation units, .lkja manifest/segments/bundle, loader coherence | 1/3/4. Remove unit8MiB/1M-item and manifest/package/unit/bundle quotas with complete framed readers | Pending. Source-derived canonical units and host parity require this path, not just Mode1 projected-unit acceptance |
| `compiler/egraph_opt.rs` | Optional local optimization, subsequent ordinary emission | 4/5. Retain finite search strategy only when exhaustion falls back to correct unoptimized code | Audit fallback and measure; do not report an optimization search cutoff as a semantic admission ceiling |
| Hash widths, byte alphabet, checked extents, DAG/ownership rules | Every wire decoder, canonical hashing, source/native admission | 5. Preserve exact commitments, checksums, sortedness, bounds, cycle rejection and finite instantiation | Mandatory throughout; malformed fixtures recompute hashes so they reach their intended semantic layer |

## Persistent storage, service surfaces, and development gates

| Enforcer / symbols | Callers and serialized path | Category and coupled disposition | Candidate state / verification |
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
candidate slice; later edits require their own integration evidence. Full acceptance still requires
literal `tools/selfhost/verify.sh`, staged-tree bootstrap,
fresh-full/release gates and normal integration against the exact staged tree.
Public proposal adoption, definition readback and the focused Tier 1 source measurement have
completed; those observations do not replace the required acceptance gates.

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

Pending: freeze the repaired tree, bootstrap it and rerun the complete literal
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

Required integration proof: freeze the repaired staged tree, bootstrap the independent
verifier, run the complete literal selfhost/native script and both named controls, run
all 29 fresh full gates with zero reuse, and run release target. The next driver has six
operations; it omits an optional changed-profile repetition because that selection already
covered all 29 gates. The required fresh full proof remains unchanged. Its prepared
driver SHA-256 is
`5d7edad40956f414804999f2acb10627fdc15435171e1f40d83dd87dec3e0a84`.

Unproven: acceptance of the repaired staged tree, complete unbounding, Tier 2
source-driven fixed points, native phase reclamation, bounded same-process RSS and
test-throughput scaling. A failed gate run and successful component checks do not
establish any of those claims.

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

## Current candidate evidence and adoption state

These checks are focused candidate evidence, not staged-tree acceptance or Tier 2 proof. All six
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

Pending verification: freeze the intended staged tree, bootstrap the independent verifier with
exact expected source/tree, run literal `bash tools/selfhost/verify.sh`, explicitly run the
ignored maintained-native-closure measurement and whole-owner/type inventory, run all
29 fresh full gates with zero reuse, and release target. Those exact-tree runs must
cover the remaining source and native controls. Bind the resulting source/tree and receipts
in the terminal acceptance record and acceptance commit without changing the frozen tree
merely to add a passed claim. Focused Tier 1 fixed points and definition readback are observed
above. No full acceptance, Tier 2 completion or bounded native RSS is claimed.
