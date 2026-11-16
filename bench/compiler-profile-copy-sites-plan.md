# Planned copy-site attribution after inlining

**Prospective contract; no implementation or new measurement.** This plan is based on `38cac5f5488b51b57004d7cf6029bc7ceed9554f`. The released scale experiment remains first. Implementation, its actual producer/source pins and any attribution run require separate release.

The [retained inline64 profile](compiler-profile-inline64.md) records 1,568/15,122 samples (10.369%) in the machine-copy category. Its function and instruction-kind tables are independent marginals. They cannot identify a removable aggregate subset or support a ≥10% whole-witness optimization. The purpose here is to join sampled copy intervals to the **actual selected post-inline IR**, then assess a narrowly defined aggregate opportunity. Historical observations stay unchanged.

## Collection and IR correspondence

Reuse the existing `profile_path.is_some()` → `Assembler::enable_profile()` flag; add no second collection mode. Ordinary emission must not construct role/path metadata, collect sites, serialize/hash selected IR or add per-copy application instructions. Site vectors remain empty without allocations when profiling is disabled. Profile-only fragment fields are absent from ordinary encodings. Small fixed struct/control costs, if introduced, must be stated; no unmeasured zero-overhead claim.

Add `build --profile --profile-ir PATH`. Serialize the companion with `ir_text::write(checked.native().program())` from **the same Selection passed to native emission**. The profile-only native data block computes the digest of that same representation. If optimized checking rejects its candidate, both emission and companion use the original fallback and report the rejection. Do not reconstruct IR by another command: ordinary `ir` intentionally keeps `checked.original()` and bypasses optimization.

The first implementation **bypasses the early whole-image memo return whenever the companion is requested**. Existing frontend/check/fragment reuse may still operate. An image-only hit cannot supply this correspondence; an authenticated image-bound companion cache is deferred. Profile presence/output-path request keys and profile-discriminated fragment namespaces remain distinct.

Prepare both artifact destinations and reject an identical held parent identity plus basename, including directory aliases. No success/pair marker or successful image memo publication precedes both outputs. Each profile binds the exact companion digest; detail reporting refuses missing/mismatched companions. A partial publication therefore cannot be accepted as a corresponding image/IR pair. This is not a new two-file atomic transaction.

The companion is diagnostic text, not new admission authority or repository truth. Numeric function/type/body coordinates are scoped to its exact bytes, not stable source identities. Inlining preserves locations but does not retain an inline-call chain; this plan does not invent one.

## Fixed table representation

Use a new `LKJPROF4` format, with no older-format fallback in the new reader. Keep the sixteen existing kind counters and function self/inclusive marginals. Do not add a counter to every kind region: the historical policy64 file already has 3,106,452 × 16 = 49,703,232 bytes of kind regions in a 50,400,056-byte file; another word per region alone would add 24,851,616 bytes.

The 160-byte header retains the old field meanings/offsets through byte 79 (reserved byte 72 word stays zero), then adds:

| Offset | Bytes | Field |
| ---: | ---: | --- |
| 80 | 32 | SHA-256 of exact selected-IR companion bytes |
| 112 | 8 | Copy-site table offset |
| 120 | 8 | Copy-site row count C |
| 128 | 8 | Selected-function map offset |
| 136 | 8 | Map row count F |
| 144 | 8 | Unmapped-copy sample counter |
| 152 | 8 | Reserved zero |

Each copy-site row is nine little-endian u64 words, **72 bytes**:

| Offset | Field | Meaning |
| ---: | --- | --- |
| 0 / 8 | start / end | Nonempty half-open native address interval |
| 16 | samples | Self samples whose interrupted RIP lies in this interval |
| 24 | function | Full selected-program function index |
| 32 | instruction | Actual selected IR body index; all-ones sentinel for incoming parameter copies |
| 40 | subsite | Constructor field/array element index, retaining scalar gaps; parameter index for incoming copies; otherwise zero |
| 48 | type | Full selected-IR type index of the copied payload; source payload type where a representation-preserving transfer changes the destination type |
| 56 | width | Exact fixed copied byte width; positive, never inferred from a sample |
| 64 | tags | Finite origin/roles/place flags below; no truncated program indices |

The separate **16F-byte map** has (selected-program function index, final existing symbol-row index), one row per selected function. Unemitted functions use an all-ones symbol sentinel; a copy site's owner cannot. Resolve symbols through emitted function labels after final assembly: profile symbol order is native-address order and includes entry/drop/profiler/runtime helpers. Never persist a physical symbol-row ordinal in a reusable function fragment.

Tags are fixed as: bits 0–7 origin (`1=IR body`, `2=incoming parameter`); bits 8–15 source role; bits 16–23 destination role; bits 24–26 source-place flags; bits 32–34 destination-place flags; all other bits zero. Place flags are `1=local projection`, `2=dereference`, `4=collection projection`, may combine, and apply only to TypedPlace. Role values are `1=Local, 2=TypedPlace, 3=ConstructorField, 4=ArrayElement, 5=ABIIncoming, 6=ParamLocal, 7=ABIResult, 8=VecElement, 9=ResultVariantField, 10=RepeatedArrayElement, 11=BoxPayload, 12=Referent, 13=ViewHeader`. Unknown attribution goes to the explicit unmapped counter, not guessed tags.

Validate sizes with checked arithmetic before allocation; require nonoverlapping encoded sections, ordered nonoverlapping site intervals within code, valid function/map bounds, valid tags and zero reserved fields. The default summary retains that structural validation without requiring a companion. The explicit detail route additionally validates type/instruction/subsite coordinates and role consistency against the companion. A matching digest or profile file is not a checker certificate.

## Exact producer sites and exclusions

Thread owner/current instruction through `native::emit_function` and `Body::emit`. Use the loop's actual `at`: rotation recognition can consume seven IR operations at once. Incoming ABI copies have the explicit synthetic origin. Constructors use original field/element indices rather than counting only fields that happen to emit copies. Zero-size and storage-coalesced transfers emit no site.

Supply roles at typed selector call sites, through `copy_value`, `copy_local`, `copy_from_local`, `copy_bytes` and intrinsic callers:

| Transfer | Source → destination |
| --- | --- |
| Whole-local Read / Assign | Local → Local |
| Projected Read / Assign | TypedPlace → Local / Local → TypedPlace |
| Struct / Variant / Array construction | Local → ConstructorField / ArrayElement |
| Incoming parameter / Return | ABIIncoming → ParamLocal / Local → ABIResult |
| VecPush / VecPop | Local → VecElement / VecElement → ResultVariantField |
| ArrayRepeat / BoxNew | Local → RepeatedArrayElement / BoxPayload |
| Clone / StrAsBytes | Referent → Local / ViewHeader → ViewHeader |

Collect only intervals classified as existing **KIND_COPY**. Runtime/allocator region overrides exclude their internal copies. Variable-width `p.copy`, Replace's byte-swap, scalar moves, address formation, ownership and frame work outside those intervals remain excluded. Internal overlap decisions and REP setup inside `copy_bytes` remain included. Roles come from typed operands, never DI/SI or width. The companion supplies exact paths and whole-body context.

## Reuse and handler invariants

Extend `Assembler`, `Checkpoint` and `Fragment` with profile-only relative site metadata. Extraction rebases both interval endpoints; append preflights the entire table before changing bytes, labels or symbols. Require contained/nonoverlapping intervals, valid local coordinates and profile-mode agreement. Current reuse graph keys bind ordered types/signatures; do not weaken that binding.

Bump the native fragment codec/domain to `LKJCODE3` / native-functions version 3. Ordinary fragments keep empty kind/site metadata and encode **no site-count field or rows**; profiled fragments encode site count/rows after the existing kind section. Decode is unambiguous from the existing profile-kind presence rule, and rejects old/malformed fragments rather than silently losing attribution. Hot fragment reuse must reproduce cold metadata after relocation.

The existing SIGPROF handler performs an allocation-free O(log C) lookup **only after KIND_COPY is selected**. Increment the matching site's counter, or explicit unmapped-copy counter, **before** the existing KIND_COPY counter. Preserve RIP/base/stack-unwind registers and the existing blocked-SIGPROF rule. No per-execution application counter and no new observer are needed.

Let D be the sum of site counters plus unmapped, K the existing copy-kind counter, T all completed kind samples and S the serial. Require S−T ∈ {0,1}. Normally D=K; D=K+1 is accepted only when S−T=1 and is reported as one possibly in-flight detail sample. Reject D<K, larger differences or arithmetic overflow. Do not decrement an arbitrary row to fabricate exact equality.

Storage grows by 72C + 16F + 80 header bytes plus stated alignment. Approximate construction metadata is 64C before vector capacity; table serialization duplicates it. Companion E adds disk and construction serialization memory, not sampled-process mapping. C, E and new runtime costs are **unknown until measured**.

## Bounded detail interface

Keep the default `profile SAMPLES` summary unchanged. Add an explicit route:

~~~text
profile SAMPLES --copy-sites --profile-ir COMPANION --limit N --bytes B [--cursor TOKEN]
~~~

Require explicit positive row/byte bounds for this route. Validate the whole input/companion before producing output. Emit a bounded header with profile/companion identities, total/mapped/unmapped/in-flight counts and exclusions; rows sort by descending samples, then the full (function, instruction, subsite, start, end) tuple. Quote names deterministically. A continuation binds both exact file digests, ordering/version and next ordinal; limits may change without changing selection. Bound the complete encoded reply, including header/cursor, and refuse a bound too small to make progress. Do not silently truncate or imply unreported rows are zero. Parsing the complete diagnostic artifacts is not an answer-size-bounded semantic-store query.

## Targeted acceptance and later measurement

Before a released attribution run, check:

- adjacent, empty, elided and coalesced metadata; every listed role, including owned/view values; widths 1/2/4/8, 16/24/32, 33–128 and 129+, with overlap behavior compared to original-IR interpretation;
- cold/warm profiled fragment range/coordinate parity and malformed/truncated/overflow/index/tag/range refusal **before assembler mutation**; ordinary emission/IR bytes unchanged and no disabled-mode site rows or selected-IR serialization;
- selected-IR digest agreement, optimizer-rejection fallback, whole-image-cache bypass, aliased destination refusal and partial-publication mismatch refusal;
- synthetic one-handler accounting states, malformed profile sections and bounded pagination; one ordinary profiler fixture with nonzero known copy-site observations, without a fixed-percent assertion. No kill/race campaign or new OS observer.

A separate prelaunch addendum must freeze producer/source/library/runtime bytes, exact selected-IR serialization, construction inline policy64, execution output policy, image oracle, command order, resource limits, load/timing boundaries and once-only failure retention. Reuse the historical fixed compiler witness where it admits unchanged; any different source must be declared before launch and prevents causal comparison with the old marginal. Include a matched ordinary control to state instrumentation/construction costs separately. Retain raw joint records, exact companion and mapping before guarded cleanup; never reconstruct this joint table from old marginals.

The later aggregate census must inspect complete selected bodies for freshness, escape, control-flow joins, ownership and destructors. Copy-like machine work is not proof of a Copy type or nonescape. Samples × width are neither dynamic byte traffic nor removable time. This attribution authorizes no optimization pass, ≥10% speed claim, Rust-O milestone or backend floor.
