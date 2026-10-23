# Remove the default aggregate artifact-input byte quota

Date: 2026-10-22 (UTC). Baseline:
d225e8cbe643ee7e1e99b1d345a482e127e5b3b3.

## Dependency and scope

The maintained compiler supplied 467 canonical units in 7,746,716 mode-1
LKJCI1 bytes, leaving 641,892 bytes below the aggregate 8 MiB quota. Both native
stdin emitters already had an accepted growable mapping. This slice therefore
removes the mode-1 aggregate byte quota across its Rust producer, lkjscript
consumer, maintained semantic source and independent controls.

The 700-unit guard remains. unit-map::encode emits one physical leaf and the
independent canonical map reader limits each page to 65,536 bytes. With U entries,
17-byte keys, 65-byte values and common prefix p, that page occupies
107+p+U*(88-p) bytes. The worst 700-entry page is 61,707 bytes. Removing the count
guard requires hierarchical page emission and inclusion of every child page.

Rust still supplies admitted canonical unit fields in Tier 1 artifact mode.
This is not source-driven E2/E3. LKJSG001, canonical compilation units, map pages,
packs and LKJARTFC envelopes are unchanged. The
[active compiler inventory](20261022-active-compiler-profile.md) records the
complete source profile needed for the next semantic path.

## Boundary classification

| Boundary | Classification | Action |
| --- | --- | --- |
| Aggregate mode-1 8 MiB input | Default policy quota | Removed |
| Implicit signed-31-bit allowance ceiling | Narrow representation/policy coupling | Profile 5 uses unmetered zero or full positive u64 |
| Explicit positive allowance below the normative estimate | Caller-selected quota | Exact rejection retained; equality succeeds |
| Generic extent count/stride guards | Implementation protection | Division-before-multiplication range proof |
| 700 units | Flat-map implementation guard | Retained pending hierarchical emission |
| Type/custody correctness, references and canonical ordering | Semantic/encoding restrictions | Preserved |
| u32 offsets and serialized counts | Representation restriction | Checked before conversion and projection allocation |
| Native per-value 256 MiB and per-unit 8 MiB | Separate implementation limits | Retained and documented |
| Host address space and allocation | Physical/platform boundary | Projection reservation returns a diagnostic; other standard allocations retain host failure behavior |

## Current-format cutover and progress

Mode 1 requires profile 5. Mode 0 remains profile 4 with its 8 MiB input bound and
existing fuel/type/local representation. Each selector admits only its current
profile; stale mode-1 inputs reject with diagnostic 2. There is no dual reader.

The Rust producer uses Option<NonZeroU64>: None writes zero, and Some writes
the selected positive u64 unchanged. Native admission compares allowance halves
without narrowing a complete u64 to signed I64. Positive policies must cover
4096 + 128*L + 64*(U*U + O); zero bypasses that policy rejection.

This estimate is not a decremented execution counter. Unit and object indexes
advance to finite validated counts; spans partition the finite input; text cursors
advance; owner lookup reduces a binary-search interval. Duplicate identities,
invalid links, corrupt lengths and noncanonical spans still reject. No cancellation
checkpoint or output-publication rule is removed.

An extent proves nonnegative offset/count, positive stride and offset <= L,
then count <= (L-offset)/stride, before multiplying or adding. Metadata has a
validated extent of at least 312 bytes before its owner count is read; its exact
length is then checked as 312+56*R. Directory minimum extents precede allowance
arithmetic. Even the looser u32 layout domain bounds the estimate below 2^54;
selected allowances may use all 64 bits.

Projection::reserve checks multiplication/addition and u32 representability before
Vec::try_reserve and resize. Every serialized count/index/offset conversion is
checked. Explicit producer checks preserve the consumer's retained 700-unit,
100,000-source-object/owner-map, 32,768-table/instruction and 128-local/operand
restrictions. A framing range of 160 through u32::MAX does not promise successful
native execution throughout that range: value construction, individual units,
image layout, address space and allocation remain independent boundaries.
Standard collection/sorting allocations outside Projection are not newly fallible.

## Six-step bootstrap

1. **Builder:** the existing Rust reference/native backend builds replacement C1
   from the publicly admitted maintained compiler.
2. **Transition format:** the native closure remains mode-0 profile 4. Replacement
   C1 consumes mode-1 profile 5 for artifact emission.
3. **Prerequisite:** the growable native input reader is accepted at d225e8cbe.
   No indexed mutation or additional language intrinsic is needed.
4. **Regeneration:** public draft/plan/apply replaces four function bodies:
   artifact-check::extent, artifact-input::layout, artifact-input::validate and
   ci::validate. Existing declaration and parameter identities remain stable.
   The transaction creates 418 body owners, retires 384 and updates exactly four
   declaration records. Maintained proposal agreement remains a mandatory test.
5. **Independent verification:** VM/reference/native allowance and extent controls,
   a publicly authored input above 8 MiB, strict object/unit equality, native
   C2=C3 and canonical artifact equality across the maintained execution stages.
6. **Cleanup:** only profile-5 mode-1 readers/producers remain. No temporary bridge
   exists. The reusable inventory and opt-in native measurement are retained as
   explicit test tools; neither supplies semantic tables to the compiler.

The accepted source revision is
rev_dc8bb4c50291e16e49a520baf65b2c93dc039d1305fc0f9c0e2a961767b03c70;
semantic state is
semantic_state_db9cdb516c961897effbf694efb3530f047cab7de3faa46dffae067760576b0a.
The complete LKJSG001 SHA-256 is
6a29a86a8d10164f5d302c6abea284e071bbe1c426e0e3c7df088eecfb5948b0.
No history truncation, retirement deletion or project reseed occurs.

## Independent controls

The focused selfhost_artifact_input_ filter passes seven tests in 49.96 seconds:
checked arithmetic, retained count boundaries, complete-bundle corruption,
unmetered/exact positive allowance, overflow-safe extents and actual capacity.
High-bit allowances include 2^31, 2^32, 2^63 and u64::MAX. Additional
(2^32)+1 and (2^63)+1 cases detect a lost high word that would otherwise resemble
unmetered zero. Exact fit succeeds and one less rejects. Stale profile-4 inputs
reject with both zero and positive allowance after checksum regeneration.
Five mode-0 admission controls pass in 27.06 seconds.

The malformed-input matrix distinguishes the mode marker from the format profile.
Changing only a mode-0 packet's marker to mode 1 rejects with diagnostic 2;
changing its profile to 5 as well reaches artifact layout admission and rejects
with diagnostic 10. The oversized mode-0 control preserves its valid magic and
mode header, so it reaches that mode's retained 8 MiB boundary instead of an
earlier magic rejection. The first frozen acceptance attempt exposed the obsolete
single-marker diagnostic expectation and was stopped. Its log is retained in
attempt-916fedda. The corrected complete malformed-input matrix passes on the VM,
reference interpreter and native C1 in 38.44 seconds. Publication requires the
complete gates on the corrected tree.

The capacity fixture uses ordinary public authoring in three modules, with
699 distinct 6,144-byte Text literals and a separate I64 entry returning 42.
It has exactly 700 units and satisfies every retained unit/count bound.
Expected declarations and literal associations come from the authored snapshot.
Strict loading must recover the exact canonical unit/source object key sets
and bytes. The entry executes in both VM and reference.

| Capacity control | Value |
| --- | --- |
| Mode-1 input bytes | 9,567,313 |
| Input SHA-256 | 143b41be5905ea373342dfed02f3c0c91d7b67fb478af70c4e117722dbcaf119 |
| Emitted artifact bytes | 9,739,154 |
| Artifact SHA-256 | 034cdfd9e1a647dec7ca0203028dfae861fbe7ec2a828bf64e6657362d8b2ef1 |
| C1 native compilation harness | 1,406 ms |
| Public authoring | 2,003 ms |

The maintained fixed-point test also sends this same cached input to C2 and C3
and requires complete byte equality with C1's artifact. The large compilation is
not redundantly interpreted. This tests capacity and preservation of the complete
supplied Tier 1 inventory; it does not claim E2 typing of arbitrary Text programs.

## Separated maintained-closure measurements

These columns compare successive maintained compiler closures, not two
implementations running the same pinned input. Retirement growth is independent
of the net change in live owners.

| Measurement | d225e8cbe baseline | Current |
| --- | ---: | ---: |
| Modules / callables including externals | 62 / 465 | 62 / 465 |
| Live canonical owners | 28,550 | 28,584 |
| Structural type objects | 27 | 27 |
| Retirement records | 10,202 | 10,586 |
| Complete LKJSG001 bytes | 6,898,767 | 6,985,830 |
| Mode-1 LKJCI1 bytes / units | 7,746,716 / 467 | 7,754,834 / 467 |
| Selfhost canonical artifact bytes | 8,183,095 | 8,191,345 |
| Mode-0 bytes / functions / nodes | 830,072 / 244 / 16,417 | 832,748 / 244 / 16,460 |
| Tier 1 C2=C3 ELF bytes | 2,584,632 | 2,591,436 |
| Direct local full-kernel work | 2,572,299 | 2,573,097 |
| Public rebuilt aggregate semantic work | 9,173,945 | 9,177,359 |
| Physical pack files / bytes | 51 / 343,959,058 | 88 / 353,464,648 |
| Reachable historical canonical payload bytes | Not remeasured | Not remeasured |
| Native requested allocation / retained storage / mapped capacity / RSS | Not measured | Not measured |

The 37 new public-authoring packs occupy 9,505,590 bytes. Every baseline pack is
preserved. Physical pack bytes are not transitive reachable historical payload.
The public aggregate sums separate local and composed-source validation operations;
it is not headroom under a single ten-million-work ceiling. The direct kernel
counter is measured separately and is not a redefinition of the public counter.

The canonical inventory test takes 9.77 seconds and emits a 10,771,886-byte report.
Its complete feature and control-flow census is in the companion decision.
The opt-in native measurement passes in 17.98 seconds, with 17,973 ms before
report encoding: compiler initialization 9,068 ms, carrier export 78 ms, direct
local validation 185 ms, mode-0 export 6 ms, mode-1 export 61 ms, C1 construction
3 ms and strict artifact loading 351 ms. Native filter harness wall times are
238/577 ms for C1/C2 native compilation and 1,130/2,177/2,174 ms for C1/C2/C3
artifact compilation. They include executable publication, process creation,
transport, execution, waiting and cleanup.

Reproduce that measurement in a fresh exact-test process on a frozen canonical
project, with one test thread:

~~~sh
LKJSCRIPT_SELFHOST_NATIVE_MEASUREMENT_OUTPUT=/tmp/lkjscript-native-measurement.json \
  cargo test --release --locked --lib \
  selfhost_source_measure_maintained_native_closure \
  -- --ignored --test-threads=1 --nocapture
~~~

Use an absent output path and the shared Cargo-target lease. The helper uses the
same cached compiler fixture as the suite; the fresh-process/frozen-project
precondition binds its revision and cold initialization observation to that
snapshot. Omit the output variable during mandatory acceptance.

The native-only measurement intentionally avoids VM/reference compilation. Its
17.98 seconds cannot be compared as a speedup against the earlier 1,489.59-second
VM/reference/native fixed-point test. No same-input runtime optimization, test
pruning or native reclamation is claimed.

## Tier 1 identities and publication gates

| Object | SHA-256 |
| --- | --- |
| Mode-0 profile-4 input | 40aba89a73cb8c7cee48bceb4123df2292249c9b6a32b400369d62b63d253c94 |
| C1 native compiler | cad0b144ff4be1744db8087ec0bb321ab893520ed2aaa359ce69c8f9bb6323da |
| C2=C3 native compiler | de7202a5769efec3325c1de9f2b57d13f98bd714a4e61aa869ff5e2fb05193ca |
| Mode-1 profile-5 input | 691fc59a356b2644c5476a8f52b911a9ee5fe35e26c5182ad0c603f64ffeaf46 |
| C1/C2/C3 selfhost artifact | cddf9425a091992086fe7127ab4aeff4a2643bdd3000236995d80ea2af8640ec |

These identities are bound to the complete carrier above, the exact compile-elf
entry, Linux x86_64, mode profiles 4/5 and the recorded empty native environment.
They remain Rust-normalized Tier 1 evidence. Equality with an independently
packaged Rust artifact is not asserted. Mandatory selfhost tests separately own
VM/reference equality, behavioral controls and compiler-source mutations.

Publication requires exact staged-tree bootstrap, the literal
tools/selfhost/verify.sh script, all 29 fresh full gates and release-target
inspection. This checkout has no root ./verify executable; its current verifier
owner is tools/bootstrap-verifier.sh followed by lkjscript-dev check full --fresh.
Source/test paths widen changed selection to that same complete matrix, so do
not stack another duplicate changed/full run.

The focused script runs all nonignored selfhost and native-ELF tests with default
features and two test threads, then checks every declared input hash. Full
acceptance adds all-feature workspace tests, CLI, lint, product, package, service,
policy and release checks. The two opt-in measurement tests remain explicitly
ignored in ordinary suites. Frozen-tree receipts and terminal logs are retained
under .artifacts/selfhost/mode1-byte-capacity; the official compact receipt stays
under .artifacts/lkjscript-dev/latest-receipt.json. Git records exact normal
integration. No product release or deployment is included.

## Next frontier

The immediate aggregate artifact-input byte quota is removed. Start the first
source-driven scalar E2/E3 path from the reproducible inventory: whole-carrier
admission of pure Unit/Bool/I64 functions, all signatures/descriptors and all
expression bodies, followed by deterministic lowering within lkjscript.
Unsupported unused types or empty-row task callables must reject at the semantic
boundary. Rust transports the raw carrier and orchestrates independent controls.

The maintained compiler's remaining Bytes/Text/List/record/buffer semantics then
extend that vertical path. LKJSG001 still has 1,402,778 bytes below its separate
8 MiB bound; its count/fuel limits and downstream recursion need their own audits.
The flat artifact unit map, finite type/local encodings and kernel work policies
remain separate dependencies. Measure the combined source-checker/backend closure
before declaring a representation rewrite necessary. Source-driven fixed points,
full Tier 2 and same-process native reclamation remain open.
