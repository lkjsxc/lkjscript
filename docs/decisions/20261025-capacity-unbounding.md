# Capacity removal: boundary census and integration ledger

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
| `persistent_map.rs::MapAdmission, MapWork` | Map load/scan/mutate/encode, compiler link, artifact, publication, package transport | 1/2. Optional page/byte/entry allowances, atomic checked preflight, independently merged saturation flags | Applied. Map filter passed 44 tests before later integration. Saturated cold-cache observations preserve exact warm-cache charges (4 cache tests passed). Witness/canonical parent cut is drafted |
| `kernel/validate.rs::FullValidator`, `kernel/work.rs::ValidationWork` | Full validation, inference, affine/callable/intrinsic admission, artifact validation | 1/2. Default work is None; exact finite charge; report saturation separately from exhaustion | Applied. Direct MAX/saturation, resumed inference above the former 10M threshold, affine and cancellation controls added; included in the 633-pass integrated candidate run; publication integration remains pending |
| `kernel/{callable_flow,composed_callable_flow,nominal_flow,owned_associated,owned_record}.rs` metadata admission | Whole-carrier callable/type evidence and compiler artifact consumers | 1/2. Remove hidden work and 64MiB metadata clamps; carry optional budgets and saturated modeled observations | Applied lower-level work/metadata cut; remaining reference/package/parallel-layout callers must be audited independently |
| `publication/validation.rs::REBUILD_LIMITS`, `publication/repository.rs` full oracle | Revision-pinned reconstruction and dependency validation; current read-view output | 1. Remove automatic store rebuild quotas and redundant 10M reconstruction ceiling; preserve structural checks | Applied; current validation output carries work-saturated. Rebuild/publication integration gates pending |
| `contributor.rs::SemanticInventory`, dev scale `OracleEvidence` | Typed inventory to JSON scale receipts | 1/3. Carry validation/map/store exactness flags; discriminate the changed scale receipt contract | Applied contract-4 inventory/scale observation patch; full dev gates pending |
| `change/budget.rs::ChangeBudget` and ten sub-admissions | CLI/control authoring, preparation, repair, selection, refresh, publication revalidation | 1/2/3. Replace 46 scalar default maxima and MAX sentinels with optional exact policies, remove engine clamping, preserve operation-zero semantics | Pending outer cut; lower-level optional APIs alone do not unbound authoring |
| `change/request{,/codec}.rs` budget and request frames | `LKJABUDG`, request digests, retry identity, logical plans, receipts | 1/3. Explicit optional policy encoding and new current-format discrimination; remove 10K-change/4MiB request ceilings with complete readers | Pending; bootstrap/public-republication path must precede persisted format changes |
| `control/compact.rs`, `control/change/{input,preflight,tx}.rs` | Compact records and structural authoring into ChangeRequest | 1/3. Remove aggregate input/record/token/value ceilings; checked extent parsing and complete diagnostics; retain syntax/schema invariants | Pending; current 4MiB/10K/64KiB/256-field restrictions remain |
| `control/logical_plan.rs`, `change/logical_plan.rs` | Logical-plan render/parse, request identities, revalidation | 1/3. Remove derived fixed record/byte maxima, represent optional policies exactly | Pending; numeric allowances must not disappear during round trips |
| `change/{footprint,relation_view,test_delta,repair}.rs`, request deletion/extraction | Refresh guards, relation/test prefix reads, repair/witness synthesis | 1/2. Remove hidden prefix/min clamps and finite generated budgets; preserve explicit selection and cancellation | Pending; relation fanout remains bounded above lower map APIs |
| `publication/{contract,history,repository,receipt}.rs` | History/refresh, transaction/receipt/semantic-diff/HEAD frames and durable work | 1/3. Unbound logical history and receipts, widen/change frames coherently; add honest persisted observation flags | Pending. Publication atomicity, idempotency, exact base revision, and create-new semantics remain mandatory |
| `normalized_query.rs`, `cli.rs`, `contract/registry.rs` | Query/function/context retrieval and continuation tokens | 1/2/3/4. Remove 10K/4MiB clamps and derived read caps; deterministic complete pagination/streaming; explicit caller limits exact | Pending. Page size may remain a presentation unit only with complete continuation. Context depth 8 and definition depth 256 need lifecycle audit |
| Kernel canonical codec: `contract.rs`, `meaning.rs`, `graph.rs`, summaries/facts/witness contracts | Owner/type/root/dependency/retirement objects, public authoring, source carrier admission | 1/3/4. Remove logical payload/children/resource/name/summary quotas with matching codecs and overflow/chunking where necessary | Pending. Current owner4MiB/type1MiB/children100K/resource1024 restrictions remain; do not delete matching source guards early |
| `package_interface.rs`, package transport source/oracle and normalized reference preparation | Dependency closure, source containers, representation/callable schemas, artifact loading | 1/2/3. Remove 16M visit/256MiB copy, package-count, metadata and work defaults; propagate exact policy through cached/prepared paths | Pending outer cut; adapted lower-level counters do not remove these caps |
| `builtin_standard.rs`, normalized supplier/source validation caches, publication validation/link caches | Evictable reuse only, followed by authoritative reconstruction | 4. Preserve bounded caches where a miss performs the same complete operation; do not confuse cache capacity with admission | Retain after checking fallback semantics; any admission-rejecting use belongs to category 1 |

## Representations and native/runtime execution

| Enforcer / symbols | Callers and serialized path | Category and coupled disposition | Candidate state / verification |
| --- | --- | --- | --- |
| `compiler/selfhost_input.rs`, selfhost `input/types/custody/flow/profile/values` | Mode0 producer, verifier, certificates, machine emitter, source-scalar pack | 1/3. Dynamic descriptor directory; wide independent type identity and custody qualifier; wide rows/offsets; clean profile cut | Applied profile6 producer/proposals: 160-byte header, dynamic32-byte descriptors,80-byte functions,64-byte instructions,16-byte identity/qualifier cells. Rust test binary builds; public adoption/native source checks pending |
| selfhost `machine/asm/elf`, `source-scalar-{plan,state,emit,pack}` | Local/stack state, sentinel, ABI frames, emitted branches | 1/3/5. Replace local128 sentinel and fixed slots together; checked wide frame addresses and deterministic wide control flow | Applied with profile6: owner-count sentinel, variable code layout, checked64-bit R11 transfers/frame addresses. Maintained-source execution after public adoption remains pending |
| `compiler/selfhost_artifact_input.rs`, selfhost `artifact-input/unit-wire/unit-map/artifact` | Mode1 profile5 projected units to canonical .lkja | 1/3/4. Widen u32 input/pool/frame offsets and 8MiB unit caps; retain hierarchical 256-way radix branching | Pending widths. Existing hierarchical map is already committed and must not be replaced with a fixed table |
| selfhost `source-graph/index/map/scope` and proof passes | Complete meaning-graph carrier, directory admission, scope/type/custody proof | 1/2/3/5. LKJSG002 tag + u64-limb allowance; unlimited skips synthetic proof estimate; structural cursor/DAG termination retained | Applied proposal + Rust test producers. Graph100K and map8MiB/100K caps removed. Whole-carrier native and malformed semantic controls added, not yet all executed |
| LKJSG002 lengths/counts/offsets and per-object codec guards | Carrier producer, every source pass, diagnostics | 3/5. Current u32 extents remain checked; eventual widening must replace all directory/payload consumers | Explicit remaining representation boundary. New policy width does not imply 64-bit carrier extents |
| `compiler/elf_x86_64::{Asm,compile,Emitter,elf}` | Rust native command compilation and ELF publication | 1/3/5/6. Fallible code/labels/worklists; usize label identity; absolute64 relocations/indirect transfers; checked wide frame and record offsets | Applied draft removes4096 functions/65536 instructions/32MiB image. Arithmetic, wide-offset,33MiB ELF and ordinary native controls passed. The public4097-function fixture is blocked before compilation by outer authoring policy; its public finite policy adapter is drafted |
| `owned_output.rs`, CLI native build, artifact preparation | Create-new executable stage, artifact bundle, release readers | 1/2/6. Default native publication must not inherit artifact byte quota; explicit output policies remain exact; preserve atomic stage cleanup | Applied Option<usize> publisher; ordinary CLI usesNone and explicit0/exact/one-over/MAX policies pass with atomic cleanup. Artifact preparation16GiB and release reader caps remain separate obligations |
| Native `heap/objects/runtime` and Rust ELF runtime | Byte allocation/copy/buffer growth/hash/derive/filter I/O | 1/3/5/6. Remove256MiB per-value checks; checked signed length, header/payload addition, geometric fallback, mmap extent checks | Applied proposals/Rust emitter. Arithmetic edge probes and existing native execution controls passed; new maintained proposals still need public adoption |
| Normalized `value/byte_buffer/owned_storage/vm/reference`, `bytes` | Allocation policy, immutable bytes and mutable buffers, VM/reference | 1/2/6. Remove hidden per-value cap; fallible reserve, exact RunPolicy, cancellation and honest merged observations | Applied. Finite0/exact/MAX, cancellation, and historical-bound arithmetic tests added |
| Native allocation lifetime: R14/R15, `heap::allocate`, buffer discard/freeze | Source prepare -> lower -> ELF, repeated process invocation | 4/6. Mapping registry; immutable input, phase scratch, persistent and output domains; explicit escape/copy protocol and munmap/reuse | Pending. Current refill already grows across mappings. Current process-exit release is not same-process phase reclamation |
| `execution/region{,/runtime,/persistence}.rs`, region value/VM/reference | Lexical region slots, relative pointers, persistence and native Region support | 1/3/5. Remove65,536 live-region cap and checked slot growth; widen u32 pointer/capacity/layout/slice fields coherently | Applied LKJREGN2/profile2 with64-bit header/cell/pointer/count fields, uncapped arena slots and iterative shared-DAG layout/codec/teardown. All46 region tests passed. Native Region/domain reclamation remains incomplete |
| Kernel depth guards versus recursive consumers | Normalize/lower/interpret, codecs, display, equality/hash/clone/drop, native call stack | 1/3/5/6. Remove synthetic counters only after recursive lifecycle is replaced by heap worklists or justified safe mechanism | Pending full lifecycle cut. Iterative validator loops are identified; 256/1024 depth removal alone is unsafe |
| Normalized `RunPolicy` call-depth/value-stack and raw admission items | Foreground/default execution, VM/reference, nested invocation | 1/2/5. Optional execution policies throughout; preserve cancellation/effects; stack-safe implementation before unbounded depth | Cumulative instruction/allocation policies already optional;4096 call-depth/1M stack/items still require cut |
| Normalized `list.rs`, `map.rs`, prepared/reference types/local-liveness | General values and schemas, persistent collection operations | 1/3/4/5. Expand list hierarchy beyond height3/1M items; optional schema/work policies; preserve structural balanced-map height proof | Pending. Map height2*usize::BITS is mathematical; list fanout32 is physical; current logical caps are not |
| `compiler/{unit,artifact,manifest,cache}.rs`, artifact loaders | Compilation units, .lkja manifest/segments/bundle, loader coherence | 1/3/4. Remove unit8MiB/1M-item and manifest/package/unit/bundle quotas with complete framed readers | Pending. Source-derived canonical units and host parity require this path, not just Mode1 projected-unit acceptance |
| `compiler/egraph_opt.rs` | Optional local optimization, subsequent ordinary emission | 4/5. Retain finite search strategy only when exhaustion falls back to correct unoptimized code | Audit fallback and measure; do not report an optimization search cutoff as a semantic admission ceiling |
| Hash widths, byte alphabet, checked extents, DAG/ownership rules | Every wire decoder, canonical hashing, source/native admission | 5. Preserve exact commitments, checksums, sortedness, bounds, cycle rejection and finite instantiation | Mandatory throughout; malformed fixtures recompute hashes so they reach their intended semantic layer |

## Persistent storage, service surfaces, and development gates

| Enforcer / symbols | Callers and serialized path | Category and coupled disposition | Candidate state / verification |
| --- | --- | --- | --- |
| `persistent_map.rs::MAXIMUM_{KEY,VALUE,PAGE}_BYTES` and tree depth | Canonical owner/type/unit maps; leaf encode/decode and scan | 1/3/4/5. Preserve64KiB physical pages; overflow/chunk value references and scalable keys/tree structure; checked reader extents | Pending. Current48KiB logical-value limit remains despite optional operation admission |
| `storage/contract.rs`, pack writer/reader | Pack entry64MiB, pack1GiB,2M entries; object lookup/reconstruction | 1/3/4/6. Widen/chunk frames and directory extents together; physical target pack size remains a tuning unit | Pending. Fallible allocation/I/O, digest verification and atomic publish retained |
| Storage catalog/segment/block/manifests | 8M entries,100K packs,32segments,level31, manifest64KiB, segment metadata/bytes | 1/3/4. Grow hierarchical directories/levels; remove total count quotas; preserve block/page invariants | Pending. Catalog observation saturation must also reach persisted receipts |
| `package.rs`, deployment/configuration/installation/release_container/packed | Public package/deployment descriptors, version inventory, archived executable transport | 1/2/3. Unbound logical modules/dependencies/targets/fields/versions and container bytes; keep semantic grants exact | Pending; packed128MiB and release128/256MiB reader limits remain |
| Data/queue/object/secrets/security modules | Transactions, scans/backups/history, payloads, attempts, randomness and secret values | 1/2/3/4/5. Separate exact caller policy, logical data size and physical chunks; preserve key/transaction/lease semantics | Pending. Data4MiB/4096 mutations/16MiB txn/10K scan/1M work and related catalog caps are explicit census members |
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
native consumers together; it is not yet adopted into the maintained graph.

During iteration the coordinated target directory and `target/.lkjscript-check.lock` are reused.
Healthy builds are joined, never duplicated. Targeted test results above describe the exact earlier
candidate slice; later edits require their own integration evidence. Full acceptance still requires
public proposal adoption, source measurement, `tools/selfhost/verify.sh`, staged-tree bootstrap,
changed/fresh-full/release gates and normal integration against the exact staged tree.

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

## Recorded candidate checks before public adoption

These checks are not staged-tree acceptance or Tier2 proof. The Git base and all six maintained
project HEADs remain unchanged at this checkpoint. No canonical republication or proposal adoption
has succeeded yet. Public source transport exports for all six projects were completed and hashed
without changing their HEADs; the current-format republication operation is described in
[the publication cutover decision](20261025-policy-publication-cutover.md).

| Candidate check | Exact result | Material limitation |
| --- | --- | --- |
| Integrated kernel/map/storage/native/byte/source subset |633 passed,4 failed;81.48s runtime |One native fixture hit the outer1000-operation default; two source fixtures needed corrected logical commitments; accepted compiler graph still lacks the proposed source format/module |
| Public republication, executable publication, storage/map/native subset |118 passed,1 failed;80.70s runtime |The4097-function fixture still encountered the outer authoring default before its late override; the policy must be in the public input before decoding |
| Wide region suite |46 passed,0 failed;1.90s runtime after3m release build |Host VM/reference region storage and lifecycle only; not native compiler phase reclamation |
| Persistent-map read cache suite |4 passed,0 failed;0.01s runtime after2m59s release build |Checks exact warm admission after saturated cold observations, includingNone/0/31/32/MAX for a32-entry page |

The broader policy, compact authoring, logical-plan, read-adapter and persisted receipt cut is still
a draft awaiting a coordinated compile. Mode1 profile7 and storage catalog/object-policy changes
are separate drafts. Source-driven fixed points and full P_self have not been established.
