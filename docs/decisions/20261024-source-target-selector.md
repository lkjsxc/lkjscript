# Select an explicit Target from complete source

Date: 2026-10-24. Status: canonical adoption, targeted recovery, the root
recovery proof and isolated reachability passed. Exact-tree publication gates
remain required; no selector publication result is claimed.

## Decision and scope

The source compiler now accepts a caller-selected canonical Target identity in a
`LKJSRQ01` request. It admits the complete embedded `LKJSG001` carrier, resolves
the selected live Target and its Component/Port inside lkjscript, and derives the
normalized entry index before ELF emission. This removes the previous requirement
that a carrier contain exactly one command Target. It preserves the existing
scalar E2/E3 permission and whole-inventory checks.

The accepted intrinsic predecessor is commit
`4165156d4d4c9726af61199d34f9b1b2053b9ef3`, tree
`a08a988e92c8500102856f6d05fff8f31cdf86c4`. The
[source-input specification](../spec/selfhosting-source-input.md) owns the
request contract; the [intrinsic decision](20261024-source-scalar-intrinsics.md)
owns the eight admitted pure I64/Bool implementations and their evaluation and
trap semantics.

## Current request and admission order

The request header is exactly 32 bytes:

| Offset | Bytes | Meaning |
| --- | ---: | --- |
| 0 | 8 | ASCII `LKJSRQ01` |
| 8 | 4 | Little-endian u32 profile, exactly 1 |
| 12 | 17 | Canonical Target owner key: domain tag 12 followed by 16 identity bytes |
| 29 | 3 | Zero reserved bytes |
| 32 | Remaining | Exact complete `LKJSG001` carrier |

Target key domain 12 is distinct from owner-kind tag 20. A zero Target identity,
unknown magic/profile, wrong domain, nonzero reserved byte or incomplete header
rejects with diagnostic 12. There is no second payload length: existing carrier
length, partition and commitment validation rejects truncated or trailing bytes.
The public source-ELF route no longer accepts an unframed raw carrier. There is
one current request reader and no compatibility branch.

After header validation, E1 and E2 inspect every supplied owner and type before
looking up the selected key. Uncalled functions, untaken branches, unused types
and unselected targets retain the same checks. Each command Target must have no
runner arguments and select a function Port belonging to its Component, with a
pure `() -> I64` entry. A carrier with no Target rejects with 13. A missing or
retired selected key rejects with 19 only after the complete carrier is admitted;
negative lookup rows are never used for table access. An unsupported unused type
therefore still wins over a missing selector. Empty-effect task callables remain
unsupported tasks.

The host transports the caller's typed TargetId and the complete carrier. Rust
does not resolve the Component/Port, select a function, filter supplied owners,
or provide normalized instructions or semantic certificates for target
compilation. The initial native compiler remains Rust-bootstrapped. The normalized
representation used between source lowering and the unchanged backend is built
inside lkjscript and independently readmitted by that backend.

The new wrapper adds exactly 32 transport bytes. It changes neither LKJSG001 nor
LKJCI1 mode 0/profile 4, mode 1/profile 5, canonical .lkja encoding or the native
runtime. The aggregate 8 MiB carrier quota remains removed; current u32 layout,
directory/object restrictions and the positive-I32 proof domain remain. The
necessary header allowance condition is `L <= 16,777,183 - 8*N`; satisfying that
condition alone is not an admission proof. Generated mode-0 bytes, callable/local
encodings, native displacements and the independent 700-unit flat-map guard
retain their existing limits.

## One public adoption and six-step bootstrap

The public transaction advances
`rev_dd94b401eb30277e5289bbc9d651230337edcd6b1391e46fabcd92c0fe38d427`
to
`rev_22b1d6d49b74893bf402af9461931f8c372f820ada378524eb50ef892a9f03d9`.
Transaction
`transaction_29e89f57559c5c561017f8b88f05ebcefca4ea483b2c6526565b1212301609e4`
was accepted with differential equality, seven updates and 445 created canonical
owners in 13,749.371353 ms. It selected no differential tests; executable
verification is separate. The transaction ran once and was not replayed during
the Rust test-harness correction below.

| Module | New functions | Updated functions |
| --- | ---: | ---: |
| source-elf | 1 | 1 |
| source-scalar-profile | 0 | 1 |
| source-scalar-plan | 0 | 4 |
| source-scalar-emit | 0 | 1 |
| Total | 1 | 7 |

The new helper is `source-elf::compile-carrier(graph: Bytes, selector: Bytes)`.
The public `compile(graph: Bytes)` declaration keeps its original header and
identities. `finish`, `index-locals`, `prepare` and `lower` append one `target: I64`
parameter while retaining every existing parameter identity, type and order.
The other three updated headers are unchanged. The proposal omits 56 unchanged
functions and preserves the source-elf Component and Target. The one new callable
is distinct from the 445 created canonical owners.

1. **Builder:** The CLI from accepted intrinsic commit `4165156d...` performs
   public authoring. Cargo reported its actual executable fresh, SHA-256
   `2875b80183af2325a61ff5ea27a8e3f8cae19215b8a1007ecb895a42edcca426`.
   The Rust reference backend bootstraps the first replacement native compiler.
2. **Consumed input:** Bootstrap and maintained Tier 1 reproduction continue to
   consume existing normalized inputs. Source target compilation consumes only
   current `LKJSRQ01/profile1` requests carrying the complete raw source carrier.
3. **Prerequisite capability:** Existing canonical owner lookup, scalar admission,
   lowering and native ELF support already provide the required operations. No
   new intrinsic or runtime capability is introduced.
4. **Regeneration:** Public adoption regenerates the canonical source. Request
   fixtures and measurement callers frame their actual selected TargetId; complete
   carrier, request and inventory measurements are regenerated from that revision.
   Exact corrected source hashes bind the resumed targeted run.
5. **Independent verification:** Run the retained E2 VM/reference/native matrix,
   framed native E3 controls and independent executions, capacity and hash-consistent
   structural negatives, maintained normalized reproduction and complete profile.
   Full publication separately requires the exact-tree gates below.
6. **Cleanup:** No temporary input bridge or dual reader exists. Preserve the
   initial failed proof and source correction as evidence. The isolated
   data-only reachability copy passed public GC and preservation checks, then
   was removed; the original project was not compacted.

## Targeted controls and harness recovery

The five existing scalar test functions retain all 26 intrinsic cases and the
balanced 32,770-instruction witness. Selection controls compile different targets
from the same complete carrier, repeat the chosen request deterministically and
execute independent outputs. Invalid unselected targets still reject. The full
producer output is compared across language engines for the baseline, alternate
target and intrinsic-arithmetic cases.

Request controls cover legacy raw-only input, short prefixes, magic/profile/domain,
zero identity, each reserved byte, truncated/trailing payload, missing/retired
targets, no-target inventory and whole-inventory failure before missing selection.
The wrong-domain control retains a live Target's identity bytes. Existing
hash-consistent missing-endpoint and disconnected-cycle controls remain selected.
The resumed scalar suite passed all five tests, including these controls.

The first Rust library-test build failed before the targeted tests could run:
three measurement references used `super::selfhost_source_scalar_tests::`, although
the registered sibling module is `super::scalar`. The original four commands
(accepted-builder, format, proposal and adoption) succeeded; the scalar command
exited 101 after 28,940.803214 ms. The failed report, log and pre-correction source
remain preserved under `initial-attempt/`.

The correction replaces exactly those three module prefixes, then performs
exactly three formatter joins from `=\n        super::scalar::` to
`= super::scalar::`. No other source bytes change. The other nineteen targeted
files, canonical HEAD, index and stashes are unchanged. The recovery driver
retains the original four successful command objects and adoption, records the
failed stage separately, and resumes only the original three targeted commands.
It does not rebuild the accepted CLI, reformat, emit another proposal or replay
adoption. The root observed the resumed process join successfully as lkj-1470.
Its separately executed post-join proof also passed (lkj-1478), binding all eight
recovery obligations, the final continuation, exact correction, source inventory
and preserved prior packs: **5196441bfb8ddee34e9e3d36729760b286433a9001fd6ae59349d2d882767f80**.

| Targeted filter | Required passed / ignored | Actual result | Whole command ms |
| --- | ---: | --- | ---: |
| `selfhost_source_scalar_` | 5 / 0 | 5 passed; 0 failed; 0 ignored | 559,348.815185 |
| `selfhost_source_measure_maintained_native_closure`, explicitly enabled | 1 / 0 | 1 passed; 0 failed; 0 ignored | 24,566.869172 |
| `selfhost_source_profile_inventories_every_owner_and_type` | 1 / 0 | 1 passed; 0 failed; 0 ignored | 11,579.212302 |

All use `cargo test --release --locked --lib`, `--test-threads=1` and
`--nocapture`; the maintained measurement additionally selects `--ignored`.
Test-reported seconds are separately **365.19 /
24.42 / 11.46**, respectively. The
scalar command includes the Rust test executable build; these command times are
not compiler execution times.

## Bound evidence

| Evidence | SHA-256 |
| --- | --- |
| Reviewed selector source package | `f801d6cfbbb307a48c59f0ddb650e6e48df32d7f14eb35176149904911ffe822` |
| Original failed continuation | `d7dc588b6834599ec77949be34f94487edff0d86fed6f8bca646771a71c64fac` |
| Original failed Rust build log | `f9c88e8197879d743dddc53404a85bb527aeb216740b6ee4ac3a52ea7f143518` |
| Exact module-path correction | `cf934fab129a2b0ced516fdc547dad7025c85f8b0cba780ad511349219619950` |
| Corrected formatted measurement source | `47995f079b22c7cd73e9d9cda483c35417554eb8cf47d17980944e62d7b36b3d` |
| Original continuation driver | `4d3b4c06719d42e595fa16b09ab34a827c20ed947e3e3bc8ae83e540f846de61` |
| Targeted recovery driver | `c772c5ec54df428326281c95932fca659d2a1004d29405d5c6ce60d5cc0ce484` |
| Joined recovered continuation | f2d8a6839299314c1068ba319b4b46b0b7d2f1ddc5df76973ea10b4d5559d837 |
| Root recovery proof | 5196441bfb8ddee34e9e3d36729760b286433a9001fd6ae59349d2d882767f80 |
| Maintained native measurement | d381d0bdf488a141ecb91f980defa920be3c81f43152575ce1e16762b32dec46 |
| Complete profile | d8621c96306da11b72de0827429825c7021a776cf79bd515bd964b9c8aa70c4f |
| Isolated reachability report / reviewed script | 8f1a844b8e3c782d0f848ce56cf591db825d6ff2ace8cae01151de8b36cbbcd1 / 2406b557d9dd4a578a9627f7bad1f057b9386e3a04555c476b11b56d388c2a9c |

The joined continuation binds all twenty corrected source paths and the actual
adopted revision. Fixed-point and request controls additionally bind the
complete carrier digest, complete request digest, typed Target key, compiler
executable, format/target profile and execution context. Semantic state alone
does not identify the complete supplied carrier.

## Separated maintained-closure measurements

The before column is the measured intrinsic revision `rev_dd94b401...`. The
after values come from the joined selector measurement, complete profile and
isolated reachability receipt. No value is inferred from edited text size,
function counts or prior fixed points.

| Observation | Before | After |
| --- | ---: | --- |
| Modules / callable declarations | 72 / 582 | 72 / 583 |
| Pure functions / externals | 559 / 23 | 560 / 23 |
| Live canonical owners | 33,919 | 34,006 |
| Retirement records | 11,651 | 12,009 |
| Supplied type objects | 35 | 35 |
| Complete LKJSG001 carrier bytes | 8,110,321 | 8,201,410 |
| Mode-1 LKJCI1 bytes / units | 9,155,057 / 586 | 9,177,461 / 587 |
| Emitted .lkja bytes / units | 9,675,994 / 586 | 9,699,624 / 587 |
| Backend mode-0 bytes / functions / nodes | 832,576 / 244 / 16,456 | 832,576 / 244 / 16,456 |
| Combined source frontend mode-0 bytes / functions / nodes | 1,672,927 / 566 / 34,469 | 1,676,546 / 567 / 34,555 |
| Combined source frontend normalized type entries | 31 | 31 |
| Direct local full-kernel work | 3,120,526 | 3,130,736 |
| Composed callable telemetry work | 8,076,365 | 8,105,847 |
| Composed callable modeled metadata bytes | 11,499,321 | 11,531,962 |
| Physical source packs / bytes | 346 / 419,166,252 | 390 / 430,408,188 |
| Reachable historical objects | 231,731 | 234,781 |
| Reachable historical canonical bytes | 497,593,841 | 508,557,991 |
| Isolated GC retained pack bytes | 397,154,190 | 405,487,133 |
| Isolated GC modeled preparation retention bytes | 556,916,977 | 568,661,927 |
| Maintained native measurement wall time, ms | 23,710 | 24,412 |
| Native requested allocation, live retention, mapped capacity and RSS | Unmeasured | Unmeasured |

Current carrier digest: **db0ae8fd2a2c1d529bc43297bd29fa5308224e1ed5c379bc62c31d0a4d335ee4**. Current semantic state:
**semantic_state_0626b9fe9d87ae871e734eafa2e45f60533f163388f1ee7e9a705b0921ba8599**. Current mode-1 input digest:
**604cec92dcda1f77cd706915f57179be41af106f1b1d12c76b657b970f7cfd76**. The measured
changes are +87 live owners,
+358 retirement records and +91,089 complete-carrier bytes. Mode-1 input grows by
22,404 bytes and one unit; the emitted artifact grows by 23,630 bytes. The source
frontend normalized input grows by 3,619 bytes and 86 nodes. Original physical
storage grows by 44 packs and 11,241,936 bytes; all 346 prior packs are preserved.

Isolated public GC retained 234,781 of 235,565 objects and reclaimed 784. It
replaced 390 packs with 16 in the copy, reclaiming 24,921,055 physical pack bytes.
The GC CLI took 180,613.176497 ms and reported 9,055,817 catalog lookups/object
reads, 2,705,026,603 bytes read and 428 visited references. Modeled preparation
retention was 568,661,927 bytes. Compared with the intrinsic baseline, reachable
canonical history grows by 10,964,150 bytes and retained pack storage by
8,332,943 bytes; neither is live compiler memory.

The report covers versioned semantic HEAD ancestry, receipts, transactions and
source objects, excluding derived-cache and operational selection roots. The
copy uses independent files without hardlinks. Its public GC receipt was checked,
and the original 390 packs / 430,408,188 bytes, project HEAD, all twenty targeted
sources, index, branch and stashes remain unchanged. Cleanup succeeded with no
errors. The report is bound to the joined continuation at
`.artifacts/selfhost/source-target-selector/reachability-f2d8a6839299314c1068ba319b4b46b0b7d2f1ddc5df76973ea10b4d5559d837/report.json`.

These compare different maintained closures, not old/new implementations on
one pinned input. No test pruning or work-counter redefinition occurs and no
speedup is claimed. Composed work is passive telemetry under the accepted explicit
unmetered policy. Physical storage, reachable canonical history, modeled metadata
and actual native memory remain separate. Isolated public GC is not same-process
compiler reclamation.

## Proof scope, publication and next frontier

The new maintained measurement records the following byte identities. All three
maintained stages consume Rust-produced normalized inputs; the complete carrier
digest above binds the measured source inventory but is not their compilation
input.

| Measured output | Bytes | SHA-256 |
| --- | ---: | --- |
| Tier 1 backend ELF C2 = C3 | 2,590,880 | `55eaea3fcc2604a386f68a5dffd5bb9c2ca8e936ca0d2bf94235e1770c2487d5` |
| Tier 1 artifact from C1 = C2 = C3 | 9,699,624 | `2f2ce9aedd41988db53042561c4ea1b3a08af8a4927ab849694875204a4f22ca` |
| Source compiler ELF emitted by backend C1 = C2 | 5,324,810 | `5d873b2343a735e0c060b0d47318775562cca7390297d593eb236a7c53393442` |
| Rust-bootstrap source compiler ELF | 707,194 | `6f7060c79fc3ca5a0bfbd093bd8acda391c31021a65384f2e9d8b649ed242c0d` |

Each of the three source compiler executables accepts the same 18,878-byte
LKJSRQ01/profile1 request, containing an 18,846-byte carrier and selected
`target_d1ba5904740cee7f2d7b773d39f6f0d5`. The encoded key is
`0cd1ba5904740cee7f2d7b773d39f6f0d5`. They emit identical 83,995-byte programs
which exit zero and print `42`. All reject the 18,970-byte request containing a
hash-consistent unused Bytes descriptor with diagnostic 13.

| Request-control object | SHA-256 |
| --- | --- |
| Accepted complete carrier | `26aec1dd8cd0f883158464687ea0263ec29068d09ed3e0ad2e6358b829c39f7e` |
| Accepted complete request | `d312edb1f266a6a615d66f586f1e7111445bddc6b421666da3fa30e2505cefc7` |
| Rejected complete carrier | `7a91a6c8f307fb1414bc2ba6eaeab162707a62967b562485bf46b21ede8d8a00` |
| Rejected complete request | `1e6c7bec39e07956a1979bc9255e2499b6ceb0704e355a4997f4b4581e499457` |
| Emitted program | `2e7b9d2e1a47f72f78df96d794ada12c0581259471c233827ed6521ae6225444` |

These native request controls use Linux x86_64, an empty native environment, a
10-second child deadline, a 33,554,432-byte harness output bound and 4,093-byte
input chunks. Their compilation times are 26 / 56 / 54 ms and rejection times
14 / 22 / 22 ms for the Rust-bootstrap and two backend-produced source compilers.

No maintained compiler-source C2=C3, canonical Rust-reference A1=A2=A3 or native
lifetime reclamation is established. The artifact equality above is between
outputs of the existing normalized-input chain; it is not a source-produced
canonical semantic-unit parity proof.

Before selector publication, freeze the intended staged tree and run:

1. `bash tools/bootstrap-verifier.sh --expected-source <actual accepted base> --expected-tree <actual staged tree> --output <owned evidence directory>`;
   record its stdout-derived verifier root and use that root's `lkjscript-dev`.
2. Literal `bash tools/selfhost/verify.sh`: expected 127 selfhost passes plus two
   ignored, and ten native passes with none ignored; bind the actual inventory.
3. The same bootstrapped verifier's
   `check full --fresh --machine --jobs 4`: all 29 gates fresh, zero reuse.
4. That verifier's `release target`, then normal exact-tree integration and remote
   confirmation. Future selector commit/tree/acceptance receipts are unfilled.

These are required gates, not observed selector results. There is no root
`./verify` executable. One checkout and the canonical target remain under the
appropriate exclusive lease. Isolated reachability completed against the exact
targeted source snapshot before these status/decision updates are applied.

The next semantic frontier is Bytes and flat records, followed by Text, closed
List substitutions and complete ByteBuffer custody/cleanup. Reconcile each actual
successor rather than treating a prepared local draft as accepted. The accepted
before inventory has 586 mode-1 units; the selector inventory has 587. Replace
flat map emission before a measured successor would exceed its 700-unit limit.
Track retirement growth, complete-carrier bytes and proof-domain restrictions
separately. Full source/artifact fixed points, native reclamation and the remaining
nominal, closure, witness, read-view and Region families remain open.
