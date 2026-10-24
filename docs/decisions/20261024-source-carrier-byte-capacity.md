# Admit complete source carriers above the former byte quota

Date: 2026-10-24. Status: implemented in the maintained canonical compiler;
publication is bound to the exact staged-tree gates below.

## Decision and dependency

Remove the aggregate 8,388,608-byte rejection from `source-graph::validate` and
the matching backing-buffer rejection from `source-index::bounds`. Preserve
checked window admission, current wire identities, individual object restrictions,
entry-count bounds and explicit proof allowances. No second reader or temporary
format bridge is introduced.

The accepted predecessor is commit
`75fefba97ab25204db5f3bf1b3975190d4d2e45d`, tree
`30471092d7742dbfd9010cb738f6d79b5f792b30`. Its complete maintained source carrier
occupied 7,925,055 bytes, leaving only 463,553 bytes below the old quota. The next
source-driven E2/E3 additions replace canonical owners and therefore also add
retirement records. Live-source size alone cannot establish remaining carrier
capacity. This small producer/consumer closure removes that immediate admission
dependency before exact intrinsic and explicit Target support.

## Boundary classification and supported domain

| Boundary | Classification and treatment |
| --- | --- |
| Complete carrier at most 8,388,608 bytes | Default aggregate admission quota removed. |
| Index backing Bytes at most 8,388,608 bytes | Default aggregate admission quota removed; checked windows remain mandatory. |
| u32 carrier length, counts, offsets and spans | Representation restriction retained, with checked Rust serialization and exact native extent checks. |
| At most 100,000 combined owners, types and retirements | Independent implementation/profile bound retained. No count-dependent traversal is widened here. |
| Explicit proof allowance | Exact selected allowance remains enforced; required-minus-one still rejects. Its current signed-32-bit positive domain remains a separate finite restriction. |
| Owner/type envelopes and inline fields | Independent per-object bounds remain, including the 4 MiB owner payload, 1 MiB type payload and 64 KiB inline Text domains. |
| Structural/nominal cycles, endpoint kinds and binder scope | Semantic restrictions retained. The complete supplied inventory remains checked. |
| Native input transport | Existing checked dynamic growth and truthful allocation/read failures remain. |
| Native launcher deadline and output/image limits | Explicit harness policies retained. They are separate from source admission. |
| Mode-0 and mode-1 formats | Separate encoding, local/type/count and native displacement restrictions remain. This change does not widen them. |

For complete carrier length L and combined directory count N, the unchanged
minimum proof allowance is `4096 + 128*L + 1024*N`. The admitted allowance is at
most 2,147,483,647, so a necessary condition remains:

`L <= 16,777,183 - 8*N`.

For the measured maintained N=44,754 this necessary envelope is 16,419,151
bytes. It is not a guarantee that every carrier up to that size is admitted:
object layouts, semantic checks, other implementation bounds and selected
execution policies still apply. Increasing retirement count also changes N.
This slice does not claim unmetered source validation or removal of every
arbitrary product limit.

Actual carrier length must equal the encoded u32 extent before allowance
arithmetic. Even a malformed maximum u32 length with the admitted count domain
produces at most 549,858,217,856 in the minimum-allowance expression, within I64.
Index admission establishes `0 <= count <= 100000`, `1 <= width <= 32` and
`width <= stride <= 128`; the largest count/stride product is 12,800,000.
The range helper proves nonnegative arguments and a window inside the actual
backing length using `count <= end-at`, after checking `at <= end`.

## Complete affected path and progress

The Rust raw carrier serializer already uses checked u32 conversions and has no
aggregate 8 MiB rejection. It transports complete supplied owners, types and
retirements. The native source checker receives those raw bytes; Rust supplies
neither semantic tables nor lowering certificates to the checked path.

`source-graph` continues to check carrier framing, sorted directories, object
spans, commitment hashes and canonical object envelopes. Each object reader
receives its checked object slice. State-map readers receive owner/retirement
directory slices rather than the entire enlarged carrier; with the retained
count bound those slices remain at most 5,000,000 and 4,900,000 bytes. The exact
`source-object` follow-up inspection confirms those bounded per-object routes.
The separate `source-map` implementation bound is therefore not silently widened.

Only the two aggregate guards change in production. Existing function headers,
parameter identities/order/custody and all other declaration forms are preserved.
No new recursive traversal, work-counter definition or indexed-mutation primitive
is introduced. No synthetic fuel, cancellation check or semantic cycle check is
removed. Existing carrier/directory passes advance finite byte or record cursors;
binary search shrinks its interval. Later graph/type proof machinery and its
termination restrictions remain intact. This is not a whole-pipeline linearity
claim, and repeated copying has not been benchmarked or optimized by this slice.

Native output is published only after the applicable checker succeeds. Invalid
windows still reject before their table walk. Hash-consistent unused types still
reach endpoint checking instead of being skipped or hidden by a commitment error.

## Canonical adoption and six-step bootstrap

The public transaction uses base
`rev_9793beb62fcf1a8ffdf61e7bbf9a6c5a310e0d4ae3b77aa86f23cb023af26c3d`
and produces
`rev_2ea581ffa2b517be5b5decabc899000f11f46d54f543374c47ff283a844de098`.
Its accepted transaction identity is
`transaction_6e8498df65c463e71bc54c6dd373cc2cc80048126a2ca9dce087b93cbc5fbf4a`.
Two existing bodies are updated, 73 owners are created and differential validation
is `equal`. No function is added. Existing symbolic declaration/parameter identity,
ancestry and retirement history are preserved.

1. **Builder:** Use the Rust public CLI built from accepted commit `75fefba97...`
   before applying changes. Its recorded SHA-256 is
   `2875b80183af2325a61ff5ea27a8e3f8cae19215b8a1007ecb895a42edcca426`.
   It performs public canonical authoring; the Rust native backend bootstraps C1.
2. **Consumed input:** Existing normalized programs build the first executable.
   Tier 1 backend mode 0/profile 4 and artifact mode 1/profile 5 are unchanged.
   Source validation consumes current `LKJSG001` bytes with unchanged framing.
3. **Prerequisite capability:** Previously accepted checked growable native stdin
   mappings already handle the larger carrier. No new runtime capability is
   adopted in this slice.
4. **Regeneration:** Public adoption updates the authoritative semantic project.
   Maintained carrier, artifact and native measurements are regenerated from that
   revision; proposals remain projections, not a second source of authority.
5. **Independent verification:** Larger-carrier, hash-consistent invalid-type,
   malformed-extent, exact-allowance and index-window controls exercise the change.
   Maintained Tier 1 fixed points and raw scalar controls are regenerated. Exact
   staged bootstrap, the literal selfhost driver, all fresh full gates and release
   target inspection bind publication.
6. **Cleanup:** No compatibility shim, alternative target directory or copied
   gate executable is retained. Reachability uses an independent data-only copy
   which is removed after its receipt and preservation checks are captured.

## Positive and negative controls

The new witness publicly authors 128 Text-returning functions with small seed
literals, then replaces their fixture-snapshot literals with 64 KiB canonical
inline values. Independent Rust kernel validation accepts the resulting snapshot.
Its 257 owners and one type produce a complete 8,433,806-byte carrier, above the
former aggregate quota. The self-produced native `read-graph` executable invokes
`source-links::validate` over that entire carrier and succeeds.

The negative variant adds a newly hashed, unused List type whose item digest is
absent. Its complete carrier is 8,433,931 bytes. Semantic-state header bytes remain
equal while the complete carrier digest changes. `read-carrier` accepts its
commitments; `read-graph` rejects with endpoint diagnostic 14. This establishes
that the complete supplied type inventory is checked beyond the former byte
threshold. It does not establish E2 permission for Text or List compilation.

| Witness | SHA-256 |
| --- | --- |
| Valid complete carrier | `16d2fc0aaf3581111991bd08f88e8a9dd7d2f94ec9096571489c2fb90f4fd8b4` |
| Hash-consistent invalid complete carrier | `7a8aa2d1dc4f6a2bd92382e79bce51a33083e31601b1d065c3caadb207a6f4ee` |
| Self-produced `read-graph` ELF, 1,435,517 bytes | `f9a9d829ef810598380569de53976c8710107980ef1be1f27916ce2f5d32df9e` |
| Self-produced `read-carrier` ELF, 1,148,613 bytes | `afff761442bd5c40ed3873045fb0cc0bd89e6039c31091b307d07f539526b627` |

The index controls use an 8,388,609-byte backing buffer with small valid owner/type
windows and still reject a u32-max offset with -2. Existing corrupt lengths,
directory ordering, object forgery, empty-directory and exact allowance controls
also pass. All six targeted test commands ran successfully: one larger-carrier,
one malformed-extent, one allowance, three index, one explicitly enabled native
measurement and one complete-profile test. The targeted driver joined with exit 0.

## Separated maintained-closure measurements

The before column is accepted revision `rev_9793beb...` at commit `75fefba97...`.
The after column is `rev_2ea581ff...`. These are different maintained closures.
No same-input performance benchmark was run, no speedup is claimed, and no work
counter definition changes. Existing scalar behavioral inputs are reproduced,
but their timings are not a controlled old/new performance comparison.

| Observation | Before | After |
| --- | ---: | ---: |
| Modules / callable declarations | 72 / 577 | 72 / 577 |
| Live canonical owners | 33,666 | 33,655 |
| Retirement records | 10,980 | 11,064 |
| Supplied type objects | 35 | 35 |
| Complete LKJSG001 carrier bytes | 7,925,055 | 7,941,194 |
| Mode-1 LKJCI1 bytes / units | 9,089,215 / 581 | 9,086,555 / 581 |
| Emitted `.lkja` bytes | 9,606,367 | 9,603,621 |
| Backend mode-0 bytes / functions / nodes | 832,576 / 244 / 16,456 | 832,576 / 244 / 16,456 |
| Combined source frontend mode-0 bytes / functions / nodes | 1,661,846 / 561 / 34,214 | 1,661,372 / 561 / 34,201 |
| Direct local full-kernel work | 3,094,806 | 3,094,141 |
| Composed callable telemetry work | 7,998,497 | 7,995,020 |
| Composed callable modeled metadata bytes | 11,405,716 | 11,401,876 |
| Physical source packs / bytes | 277 / 401,520,402 | 297 / 406,592,267 |
| Reachable historical objects | 226,375 | 227,308 |
| Reachable historical canonical bytes | 480,434,611 | 485,420,559 |
| Isolated GC retained pack bytes | 384,013,851 | 387,778,025 |
| Isolated GC modeled preparation retention bytes | 538,386,611 | 543,611,407 |
| Maintained native measurement wall time, ms | 24,123 | 23,085 |
| Native requested allocation, live retention, mapped capacity and RSS | Unmeasured | Unmeasured |

Live owners fall by 11, but 84 additional retirements make the full carrier grow
by 16,139 bytes. This directly illustrates why retired history and live source
must be reported separately. The pack inventory is physical storage, reachable
canonical bytes are logical historical payload, and modeled GC retention is an
accounting estimate. None is a native runtime or resident-memory measurement.

The complete carrier identity is
`a2dbf5a6604c4c3cbbce3df4fe3b003256eea3e4af454832181da1c5002759bf`;
semantic state is
`semantic_state_26e8c812bea771560ad512c8f83febbe2da12dc63fc927fa480a9814e1e04972`.
Mode-1 input SHA-256 is
`601cc2d58a0def2399a41012dd7684de5dba921c6f284f70caf067d5b7fc0223`.
The complete-profile receipt records all supplied owners/types, exact intrinsic
signatures and closed substitutions, use modes, lexical bindings, recursion and
tail contexts. It reports 23 externals, 554 pure functions, 1 borrow parameter,
159 consume parameters and 14 generic application sites. These observations are
not source E2 permission or an E3 cleanup certificate.

The isolated historical measurement invokes public GC only on copied HEAD and
pack data, with no hardlinks or copied executable. It verifies original project,
source, index, branch, stashes, builder and evidence unchanged; cleanup succeeds.
Its GC wall time is 175158.659 ms. The source project is not compacted or
reseeded by the measurement.

## Fixed-point scope and publication evidence

The maintained Tier 1 native fixed point remains C2=C3, 2,590,880 bytes with
SHA-256 `55eaea3fcc2604a386f68a5dffd5bb9c2ca8e936ca0d2bf94235e1770c2487d5`.
Artifact bytes from C1/C2/C3 agree at 9,603,621 bytes with SHA-256
`481693fae479a330ee7563742f59a8bbf86352acfe16fc30cf506070be2e1711`.
These observations are bound to the full maintained carrier above, executable
identities, Linux x86_64, current profiles and the recorded harness context.
They still consume Rust-normalized compiler inputs. They do not prove full
maintained-source C2=C3 or reference/C1/C2 canonical A1=A2=A3.

Backend C1 and C2 emit the same 5,272,820-byte combined frontend, SHA-256
`45b44982cfc5b79382feec76241aa969fec2fe4c15a9eb5ae1be7d9bf5b1f67e`.
Both and the 699,354-byte Rust-bootstrap frontend accept the pinned complete
scalar carrier, emit the same independently executed program returning 42, and
reject the hash-consistent unused Bytes-type control with diagnostic 13.
The compiler's full active source profile remains unsupported at E2/E3.

Targeted evidence is under `.artifacts/selfhost/source-carrier-byte-capacity`:

| Receipt | SHA-256 |
| --- | --- |
| Targeted continuation | `3746d8493f16ec37c15444c5d58cd4725346bc4b9d1580e4e7dc5392e6d051cd` |
| Maintained native measurement | `b740d9aa928c3261ef0566c64c7ac44ab37da71fb38b4b6f1425612683aca9c2` |
| Complete profile | `34e1f3a7f67fc724d745320f1627ed090dd7618e3af1d331b2f31e73bb56b39e` |
| Isolated reachability report | `d5002a3bef5edf539cdbb20b1fc4a0ae048c6c4f0f3d4638c99dfedfbbd7af66` |

The frozen candidate manifest and gate receipts belong under
`.artifacts/selfhost/source-carrier-byte-capacity-acceptance`. Publication requires
the exact staged tree built by `tools/bootstrap-verifier.sh`, literal
`tools/selfhost/verify.sh` (127 selfhost passes, two ignored; ten native passes),
all 29 gates in `lkjscript-dev check full --fresh --machine --jobs 4` with zero
reuse and stable inputs, and `lkjscript-dev release target`. Gate owners retain
the single canonical target lease. No root `./verify` executable exists.

After root reviews the receipts, publication commits the tested tree, fast-forwards
main without a tree change and confirms an ordinary push to the same remote
commit. Git and the publication receipt own those terminal identities; no
prospective commit hash or successful gate result is invented here.

## Immediate continuation

Next close exact I64/Bool intrinsic E2/E3 paths and explicit Target selection.
Then extend immutable Bytes/records, Text, closed List substitutions and the
complete ByteBuffer custody/cleanup contract. Continue to check whole inventory
and bind complete carriers. Revisit the remaining proof representation before it
becomes the next byte-capacity blocker, and replace flat artifact-map emission
before the 700-unit guard blocks an otherwise coherent source slice.

Same-process native reclamation, broader nominal/closure/witness/read-view/Region
support, source-driven fixed points and full P_self Tier 2 acceptance remain
separate required work. Their local implementation drafts and static reviews do
not constitute runtime, semantic or publication acceptance.
