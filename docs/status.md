# Current status

Updated 2026-11-01 (Asia/Tokyo). Accepted typed meaning is the sole editable
program authority. Source proposals are adopted through public transactions.

## Current milestone and delivery boundary

Milestone A is accepted on main: the maintained compiler consumes its complete
authoritative source closure, performs source admission and lowering, and
reproduces standalone ELF and canonical artifacts. The accepted implementation is
`010ce014ae97e87df17d2f6acbea728f73bf14d1`, tested tree
`7c4b670f4cabb5f50284f81b141fe81517a94e9d`. The documentation follow-up is
`114f7764638a40e4239038b865eb2b265d785219`. Its
[acceptance record](decisions/20261101-active-source-acceptance.json) binds the
historical 29 fresh gates, release-target result and exact implementation tree;
that seal does not cover subsequent phase-runtime changes.

The Milestone C candidate on `runtime/20261101-native-phases` has passed focused
production checks for repeated native invocation, scratch reclamation, independent
output ownership and recovery. Both the Rust-produced compiler and the
source-produced compiler were invoked repeatedly in a long-lived native process,
with surviving outputs checked after scratch disposal. The complete raw-source
fixed-point test also passed with `C2 = C3`, `Ahost = A1 = A2 = A3` and
`original_repository_unchanged=true`.

**Fresh staged-tree acceptance and integration of Milestone C are pending.**
The focused receipts below bind a test binary and the accepted source graph;
they are not a staged-tree seal. Full `P_self` support (Milestone B), general
lifecycle unbounding and repository-wide capacity freedom remain separate work.
No release archive or production deployment was published.

## Authoritative source and focused outputs

The following identities belong to the phase candidate's accepted graph and
completed focused fixed-point run, rather than to the earlier Milestone A seal.

| Identity | Value |
| --- | --- |
| Graph revision | `rev_4738276e107d5ba43239a48a5c93ad852c76ab850e8bbf43346ef4ac520c7d59` |
| Public HEAD SHA-256 | `46e8489a1033b25a7f4373016619b58172bf7b4069084123c781267454ac6eec` |
| Semantic state | `semantic_state_ac4cf31448b764ce6b563ba84cc1d2f0a241d00c5eaed0057657ed4799a43a46` |
| Complete source carrier | 19,130,594 bytes, profile 3 |
| Carrier SHA-256 | `1f8b1be6a27a6f8332643c0c4f9766f3f2771febcd34df899817266fc3882a6a` |
| ELF request | 19,130,626 bytes, request profile 2 |
| ELF request SHA-256 | `a924eee00c6b542b8a2efa5e995e4679f7a15abb3c8b904695af579f73c3a560` |
| Artifact request SHA-256 | `49a196929cbaf1f334d9e6e4f67f74be14ee3955b846b35ac1c0e3066382d0c4` |
| C2/C3 standalone Linux x86_64 ELF | 13,810,793 bytes |
| C2/C3 SHA-256 | `780d371d47abc53129d51bb0310e4735cf623df0ea3cc3cb1223a8a78a63f2ae` |
| Ahost/A1/A2/A3 canonical artifact | 17,709,805 bytes |
| Artifact SHA-256 | `459c500e77a93ea9306a5007d9e5aa486d75601d857686a1bb9b6721a59390bb` |

The current inventory contains 121 modules, 62,978 owners, 88 supplied types,
1,117 callables (1,090 pure and 27 external), 53,220 expressions, 4,571 lexical
bindings and 40,626 retirements. Parameter modes are 3,629 unrestricted,
306 consume and one borrow; all 34 generic application sites are inventoried.
Public readback covers all owners in 16 pages. Each emitted artifact generation
strictly loads 1,124 canonical units and 66,394 artifact objects, with exact
revision, semantic-state and complete carrier provenance.

The accepted 121-proposal snapshot is
`.artifacts/capacity-unbounding/native-phase-20261101/accepted-proposals-5.json`,
SHA-256 `61aa185c50f233fde194e9a339de44b7a9042d9fec34a07a5e74fda24c37e5bf`.
Public GC compacted 98 packs / 272,587,289 bytes into ten packs /
269,216,542 bytes while retaining the exact HEAD and all 357,004 stored objects.
The complete 23,628,696-byte transport is identical before and after GC, SHA-256
`212b52be4f04c0c849dc6d4d3a36a4eb25647eca789145630ac03bb07bca1208`.
The GC receipt is `gc-round2/receipt.json` under the same artifact root, SHA-256
`c5393dc67d72b0cae7e7c8fb32d2581b70a33ccbff4abb62a195bd60a6097b0d`.
These pack figures measure disk compaction, independently of native reclamation.

## Capabilities and phase scope

The accepted source compiler covers the active scalar and compound type closure,
structural record inference, fields, lists, lexical control flow, direct
recursion, closed I64 generic list substitutions, exact intrinsic signatures and
binary-filter entrypoints. Affine ByteBuffer admission tracks consumption,
synchronous borrowing, branch-balanced disposal, reverse lexical cleanup and
returned-owner transfer. Live owned locals prevent unsafe tail transfer.
Canonical unit, summary, interface, map, pack and artifact construction is
source-driven, with no host-normalized target input.

`LKJSG002` profile 3 and `LKJSRQ01` profile 2 remain the current source framing.
Mode-0 profile 6 and Mode-1 profile 7 remain the current backend projections;
the compiler constructs them from source. The phase implementation adds no
language intrinsic or semantic family to `F_active`. Language-level lexical
Regions, first-class closure support beyond this active profile, nominal owned
families and owned witnesses remain separate obligations.

The candidate [native phase contract](spec/native-phases.md) exports recoverable
invocation for a complete pure `(Bytes) -> {diagnostic: I64, image: Bytes}`
closure. Each invocation has separate scratch and output slab chains. The entire
surviving record and image are copied into the output context before scratch is
unmapped. Handles support independent retained outputs and release; failed
cleanup retains exact ownership for retry. Cancellation, allocation refusal,
runtime rejection and cleanup failure have separate completion statuses.
The [decision](decisions/20261101-native-phase-reclamation.md) records the
implementation and evidence boundaries.

This work does not turn native calls into heap-backed frames or remove existing
recursive lifecycle guards. General arbitrary-value promotion is not part of the
binary-filter ABI. The earlier scalar pool and bulk byte intrinsics remain
supported; their prior memory observations do not measure this phase candidate's
peak live data, allocation traffic or performance improvement.

## Focused Milestone C evidence

All four selections passed under the shared `target/.lkjscript-check.lock`
lease with `CARGO_TARGET_DIR="$PWD/target"`, `cargo test --release --locked --lib`
and `--nocapture --test-threads=1`. The last two explicitly used `--ignored`.
Elapsed figures below are whole command durations, including any compilation.

| Selection | Passed / failed / ignored | Elapsed |
| --- | ---: | ---: |
| `native_elf_` | 19 / 0 / 0 | 581.884431769s |
| `selfhost_source_profile_inventories_every_owner_and_type` | 1 / 0 / 0 | 39.198342628s |
| `selfhost_source_measure_maintained_native_closure` | 1 / 0 / 0 | 55.722975938s |
| `selfhost_source_artifact_whole_maintained_raw_source_fixed_point` | 1 / 0 / 0 | 1315.264746685s |

Receipts are under
`.artifacts/capacity-unbounding/native-phase-20261101/`:

| Receipt | SHA-256 |
| --- | --- |
| `focused-native-round3/receipt.json` | `4885d2f849d79230a39ea1f1f3b3cd2558a1aaa257c70e7052dd68f61cf09bb1` |
| `focused-profile-round1/receipt.json` | `4c4fdd03e9eee44125dd516f2fe96c1b8d03b39485d454c8a3034729acb03570` |
| `focused-native-closure-round1/receipt.json` | `a961242045d37ceebdb05572f6ebcc21aa087f7ea4482ebe5df5e52cbdbff80a` |
| `focused-raw-fixed-point-round1/receipt.json` | `2742d08f36ffc9f5fa01605333d0ca980d07265c776bd00088bc46515d35772e` |

All four receipts identify source base
`114f7764638a40e4239038b865eb2b265d785219`, the graph HEAD above and test binary
SHA-256 `a8cc9f82ee4f13ee0932cc1010c1b4dcd2e89f0e239fa9bd16af3e6bb4c1be5a`.
They explicitly describe focused working-tree controls and contain no candidate
Git tree seal.

The whole-compiler test exercised 17 phase observation frames for C1-Rust and
17 for C2-source, in one native process per generation. Both sequences completed
with production instructions unchanged, scratch ownership empty, independent
mapping checks passed and caller callee-saved registers preserved. Retained
artifacts matched the independent host oracle after scratch unmapping and
subsequent invocations. Final output release left zero owned mapped bytes and
zero anonymous writable mapping bytes attributed by the observer. C3 was
byte-identical to C2; a separate C3 phase process is not claimed.

The native selection additionally covers allocation-free cancellation, actual
mapping and promotion refusal, runtime traps, cleanup refusal followed by retry,
malformed handles, passive saturation and exclusion of task callables with empty
effect rows. A control which clears counters while leaving mappings live is
rejected by the independent process observer. These checks establish more than
counter reset or process-exit cleanup.

Historical focused logs retain a stale `same_process_reclamation=false` summary
label. Their explicit production phase records above establish which checks
actually ran. The source of that label has been corrected; the pending full seal
must cover that correction and the subsequent Rust guard completion-status fix.
The immutable focused receipts remain evidence for their original test binary.

## Pending finalization and next work

Final preparation passed formatting (5.168s), Clippy (43.417s), current CLI
build (175.336s), discovery generation (0.860s) and discovery verification
(0.889s). The current CLI SHA-256 is
`ac9ab6365c8930479848cd568616b5066ed8ce31370acf0c4ca8170e85f931f1`;
preparation receipt SHA-256 is
`bd7f57070e50d773da6f62b58468d936c306454402ee8faf33dd122dec8fc1a5`.
These preparation checks are separate from the focused test binary and the
pending staged-tree seal.

Finish documentation finalization, then freeze the intended tracked tree with
zero unstaged or non-ignored untracked changes.
Capture `expected_source=$(git rev-parse HEAD)` and
`expected_tree=$(git write-tree)`; maintain working-tree and index quiescence
through the staged verifier bootstrap, complete selfhost selection, named
maintained-native-closure / whole-owner-type-inventory / raw-source controls,
all 29 fresh full gates and release-target validation. Commit and integrate
exactly that verified tree, binding terminal receipt hashes without a post-seal
tracked edit. The current focused result does not pre-empt any of these gates.

After Milestone C integration, advance the next dependency-closed Milestone B
semantic batch. Closed pure callable values, Bind and Invoke require coherent
source admission, both backend projections, evaluator capture validation and
environment lifetime handling before adoption into maintained compiler source.
Nominal owned records/choices and Match, explicit owned witnesses/read views and
lexical Regions remain required by `P_self`.

The [capacity ledger](decisions/20261025-capacity-unbounding.md) continues to track
canonical-unit/object envelopes, lifecycle and stack safety, map key/value scale,
catalog/GC topology and package/data/queue/HTTP/tooling families. Neither full
Tier 2 nor Tier 3 sovereignty is established by this candidate.
