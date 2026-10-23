# Source-driven scalar admission and native lowering

Date: 2026-10-23 (UTC). Baseline:
3556e5824c77ddfe2d18322fe35d5f6de947a679.

## Dependency and scope

This slice adds the first executable E2/E3 path from complete LKJSG001 source:
pure monomorphic Unit/Bool/I64 functions, lexical expressions and direct calls
are checked and lowered inside lkjscript. The native filter returns a standalone
Linux x86_64 ELF image. Rust bootstraps the compiler executable and transports
raw carriers; it supplies no target semantic tables, normalized instructions or
flow certificates to this route.

Adopting the planner exposed the next actual capacity blocker. The unchanged
25,286-byte public transaction in adoption-plan-only.lkjc, SHA-256
1625ba51332604fef0e3b466c337edd16171c99d779573cb590c2ffdc8dd9eea,
failed with kernel_callable_flow_work under the ordinary reference preparation
chain. Its failure log is bound by
2d12b2ec54eae09ac1e68be1d0bb6baaefda670895a1108a1d4ae413af2bef4b.
The same transaction and base revision succeeded after the scoped work-policy
change below. No larger format rewrite or history truncation was needed to
execute this scalar milestone.

The maintained compiler still needs Bytes/Text intrinsics, structural records,
closed List substitutions and complete ByteBuffer custody/cleanup lowering.
The combined frontend/backend also exceeds the current mode-0 aggregate
32,768-instruction guard. This decision does not claim compiler-source fixed
points, canonical artifact parity or native memory reclamation.

## Boundary classification and composed-work policy

| Boundary | Classification | Handling |
| --- | --- | --- |
| Ordinary composed callable-work rejection | Default policy quota | None makes the tally observational |
| Explicit composed work allowance | Caller-selected quota | Some(limit) keeps checked exact-boundary enforcement |
| Finite specialization, canonical binding and custody | Semantic restrictions | Preserved |
| Composed metadata and recursive type/selection depth | Separate implementation protection | 64 MiB metadata and depth 256 retained |
| Independent associated/read-view/Region preparation | Downstream implementation chain | Existing shared work policy retained |
| Package-source collecting reads and transported-source work | Separate bounded consumer chain | Existing selected bounds retained |
| LKJSG001 and mode-0 size/count/type/local fields | Representation and implementation restrictions | Unchanged in this slice |
| Arithmetic, allocation and host address space | Physical/platform boundaries | Existing checked construction and host failure behavior retained |

validate_callable_closure and its snapshot adapters now accept Option<usize>.
Every steps call performs cancellation first. None uses saturating addition
only for a passive observation. Some(limit) uses checked addition, rejects
overflow even for usize::MAX, and preserves the exact fit/one-short diagnostic.
Neither the observation nor a saturated value sizes buffers, grants a reservation,
establishes an index or determines semantic permission.

Artifact composed analysis, candidate publication's composed proof and ordinary
reference-schema composed analysis select None. Publication keeps collecting
reads independently bounded and returns a saturating sum of prior semantic work,
collecting visits and composed work for reporting. Artifact work is likewise
reported, not charged back into admission.

The reference path shares one requires_associated_preparation predicate between
source reconstruction and type completion. Associated type forms, read views,
Region forms and declaration read/associated metadata select the existing
bounded preparation chain, including unused supplied types and dependency types.
ByteBuffer ownership and a borrow parameter alone do not select that chain.
Tests cover both snapshot orders, local/dependency type tables and unused
declaration metadata.

That independent chain still debits source work into associated preparation,
offset reconstruction, dependency binders, canonical contexts/slots and an
independent recursive term/witness model. Its recursion and cumulative derived
witness depth require a coherent follow-up audit; granting a fresh budget or
passing a saturated observation downstream would be incorrect. Package transport
also keeps Some(remaining) because it charges the result into a bounded collector.
Local FullValidator and dependency admission keep their existing separate limits.
This is a scoped removal, not a repository-wide unmetered-validation claim.

### Progress without synthetic composed-work rejection

The composed analysis receives structurally admitted snapshots. Prerequisite
syntax visitation is finite. Its iterative SCC proof rejects recursive witness
constructor growth before exact specialization. Complete context/selection keys
are interned once and queued once; the admitted specialization domain is finite.
Cumulative selection/type depth remains checked.

Syntax visitation keys include expression, loan set and establishing binding.
Recursive and final type SCC passes use explicit worklists with advancing
adjacency cursors. Positive constructor cycles remain semantic errors. Demanded
requests and associated queries use complete context/path/member/scope identities;
transfers consume finite path suffixes. The retained recursive selection helper
remains depth guarded. Checked metadata reservation is independent of the work
observation. Cancellation and publication checks remain in place.

These invariants justify removing the composed work policy for already admitted
inputs. They do not make this one analyzer an independent structural-cycle
validator or remove the downstream reference evaluator's recursion restrictions.

## Complete scalar E2 and E3 contract

The [source-input specification](../spec/selfhosting-source-input.md#scalar-source-admission-and-elf-lowering)
owns the exact admitted profile. E1 carrier, reference, containment, lexical,
namespace, structural/contextual type and nominal checks run before scalar
permission. Profile admission checks every owner and supplied type. Inference
checks every expression and binding, then every declared function result.
Uncalled functions, untaken branches, unused bindings and unused descriptors
receive the same checks.

The scalar profile admits Unit/Bool/I64 literals, parameter/binding locals, If,
Let, nonempty Sequence and direct monomorphic calls. All parameters are
unrestricted. Function descriptors describe scalar signatures; function-valued
expressions remain unsupported. Exactly one command Target selects a function
Port of its own Component with signature () -> I64 and no runner arguments.
Additional zero-requirement Components are permitted. Empty-row task callables
remain tasks and reject with unsupported diagnostic 13. Scalar type, arity,
annotation, result or target/port inconsistencies reject with 19.

Inference stores unresolved or completed scalar facts. Each full pass resolves
everything, strictly decreases the unresolved count, or rejects. Facts do not
change type. Call inference reads declared signatures rather than expanding
function bodies, permitting direct and mutual recursion. Repeated passes may
have quadratic work; no whole-pipeline linear-time claim is made.

The planner orders all supplied Functions by canonical owner identity, assigns
parameter slots in declaration order and binding slots deterministically, and
computes instruction counts. The cursor follows admitted containment, preserving
operand, initializer and sequence order. State/trace passes derive lexical local
availability and incoming operand prefixes. Trace construction uses append-only
ByteBuffer accumulation and finite extraction passes. This does not remove
all existing prefix copying or claim an overall compiler speedup.

Lowering emits literals, loads/stores, drops, forward branches, direct Call and
Return. The generated mode-0/profile-4 payload and flow certificates are produced
inside lkjscript and independently admitted by the existing ELF backend. Calls
in tail position use ordinary Call/Return. No owned values occur in this profile,
so asymmetric consumption, borrowed-call cleanup, returned owners and tail-call
cleanup fallback remain required future E3 work.

The source carrier retains its 8 MiB bound, 100,000 directory-entry bound and
existing proof allowance. Generated mode 0 retains 8 MiB, 1,024 functions,
32,768 instructions, 128 locals/parameters and operand depth 128. Its generated
allowance must fit signed 31 bits. These protections are not silently interpreted
as full u32 capacity or removed by the new scalar path.

## Six-step bootstrap and canonical adoption

1. **Builder:** the current Rust reference/native backend builds source-elf::compile
   from the publicly admitted maintained semantic compiler project.
2. **Consumed formats:** bootstrap uses the current canonical project; target
   programs enter C1 only as complete LKJSG001. E3 generates current profile-4
   mode 0 internally. Existing Tier 1 artifact mode remains profile 5.
3. **Prerequisite capability:** existing read-only Bytes, append-only ByteBuffer,
   native filter transport and ELF emission suffice. No indexed mutation or
   new runtime intrinsic is adopted.
4. **Regeneration:** three successful public transactions add all ten final
   modules, then their maintained proposal projections and test inventory are
   updated. They create 1,976, 1,267 and 1,813 owners respectively, with no replaced
   declarations or added retirements. Failed proposals did not publish HEAD.
5. **Independent verification:** whole-carrier E1/E2 controls, scalar source-to-ELF
   execution, hash-consistent raw structural controls and maintained Tier 1
   native/artifact fixed points run separately with complete identities.
6. **Cleanup:** no temporary bridge, alternate format reader or Rust target
   lowering fallback is present. Ignored measurements remain explicit observation
   tools; the public project is the sole authority.

Accepted semantic revision:
rev_58c7fb8d64952a3b5f12e6637b4265657ff13b023a870af12af8b3f19153ba87.
Semantic state:
semantic_state_a82dd2ca1863cd9923f8f417570f7d4fd23bdbf7db94ab96328e67a5e866ca71.
Complete carrier SHA-256:
8a9a3da1fa795170436ae4ec1f3716425ef988e08a2f81901475133005846e33.

## Independent controls and identities

The four selfhost_source_scalar tests pass in 83.12 seconds.
The final fixture SHA-256 is
0966b5f1600003f699692093945efff2e27f6e52c51a3bdf0b65bef94ed6450b;
the targeted log SHA-256 is
3fbb4b9114f62087bb9dbf0fa99182cb32887c69fb640a9a376a37bbb20fc391.
Their shared matrix has eight positive cases, eight semantic diagnostic-19
cases and two unsupported diagnostic-13 cases. All 18 shape-valid carriers pass
the composed E1 check first. E2 runs through VM, independent reference and native
inspection. E3 compiles every case natively; its baseline emitted bytes also
agree across VM/reference/native. The eight positive images execute with
independent expected results.

The baseline includes nested Let scopes and operand prefixes, Sequence drops,
zero-arity calls, Bool and Unit parameter/result calls, both branch paths, and
terminating direct/mutual recursion. Positive variants exercise zero, negative,
I64 minimum/maximum, false-branch selection, a different valid component port
and an unused supported function descriptor. Semantic controls cover an unused
wrong return, an untaken branch mismatch, non-Bool condition, unused call
arity/order, unused binding annotation and target/port disagreements. Unsupported
controls include an unused Bytes descriptor and an unused publicly authored
empty-row TaskFunction.

Two further carriers are constructed directly from independent owner records,
without public authoring or kernel normalization. A missing expression endpoint
passes carrier commitments and rejects at links with 14. A disconnected
two-expression cycle passes carrier, link and parent-table checks before scope
admission rejects with 15. Native E3 returns the exact rejection, exit 65 and no
ELF bytes. Shared neutral canonical codecs/digests frame those fixtures; this is
not a claim of an independently implemented wire codec.

| Scalar object | Bytes | SHA-256 |
| --- | ---: | --- |
| Native source compiler C1 | 698,850 | 0de404c4eb21d5530985898603c21b5a4bdb6ecb275165b4b3477c06f2239ce8 |
| Native E2 inspector | 328,722 | f509c70ed5e5d0df91e2386dd03f1521da6fef41f98e2f613f8cca62758ede8d |
| Baseline target carrier | 18,634 | 6792237fa75814aa9e71b64eaa37d00520fbd9f40c2749400240eadf31fa16bf |
| Baseline emitted ELF | 83,995 | 17c2ee3b21caf0d9fbdf9da029137aeecd44230c88c625eb1b8a5c8e842635ae |

Logs bind the complete compiler carrier and target carrier, compiler executable,
Linux x86_64, LKJSG001/profile-4 ELF, empty environment, no arguments, 127-byte
stdin chunks, 10-second filter deadline and 5-second program deadline. Adding a
valid unused type preserves semantic_state while changing the complete carrier
digest; this control also preserves emitted behavior. No semantic-state-only
fixed-point or cache identity is accepted.

The composed policy tests pass 70 cases, canonical associated tests 18 cases
and the preparation-routing tests two cases. They retain exact quota boundaries,
checked overflow, cancellation and semantic expansion/custody failures.
An inherited observation above ten million is a policy test, not a claim that
that fixture actually performed ten million operations. The unchanged public
planner retry is the concrete former-threshold acceptance evidence.

## Reproducible maintained inventory

The existing complete-source inventory test now reports 72 modules and 576
callables: 553 pure functions and the same 23 external intrinsics. All 578
declarations, two components/ports/targets and 35 supplied type descriptors are
included. The 12,845,269-byte report SHA-256 is
0a56aba47429beaf5b118bf131026a6aa12066d1d160b7c738ed54b168af0817.
Reproduce with the commands in the
[active-profile decision](20261022-active-compiler-profile.md), using an absent
output path and the shared Cargo-target lease.

The ten added modules are source-scalar-profile, source-scalar-infer,
source-scalar, source-scalar-pack, source-scalar-plan, source-scalar-cursor,
source-scalar-state, source-scalar-trace, source-scalar-emit and source-elf.
The remaining 62 modules and intrinsic declarations retain their baseline
inventory. Exact declarations, signatures, substitutions and identities are
derived from the accepted snapshot, not a pre-filtered proposal list.

| Current expression constructor | Count |
| --- | ---: |
| Local | 10,583 |
| Call | 7,096 |
| I64 | 6,677 |
| If | 1,938 |
| Bool | 803 |
| Let | 853 |
| Field | 374 |
| Text | 138 |
| Record | 130 |
| List | 91 |

Types comprise Bool, I64, Bytes, ByteBuffer and Text; three List descriptors,
24 structural records, one pure Function descriptor and two type-parameter
descriptors. The report retains all 23 instantiated intrinsic signatures and
14 generic application sites. Parameter uses are 1,792 unrestricted, 159 consume
and one borrow. There are no dependency packages, external blobs, task callables,
associated/read-view/Region types or effect/requirement applications in this
maintained closure. Its requires_associated_preparation observation is false.

The lexical/call census records 2,349 binding owners, 853 Let scopes, maximum
61 enclosing lexical bindings, 3,710 distinct direct call edges, 171 recursive
components involving 173 declarations, and 754 syntactic tail-call contexts.
These are source observations, not precomputed E2/E3 permission or cleanup
certificates. Full Tier 2 still includes nominal owned forms, closures, witnesses,
associated types/read views and lexical Regions beyond this active profile.

## Separated measurements

These columns compare maintained closures. They are not a same-input performance
comparison. The work-policy change is not an algorithmic speedup, and failed
adoption wall time is not a valid throughput baseline.

| Measurement | 3556e582 baseline | Current |
| --- | ---: | ---: |
| Modules / callables | 62 / 465 | 72 / 576 |
| Live canonical owners | 28,584 | 33,640 |
| Type descriptors | 27 | 35 |
| Retirement records | 10,586 | 10,586 |
| Complete LKJSG001 bytes | 6,985,830 | 7,836,565 |
| Mode-1 LKJCI1 bytes / units | 7,754,834 / 467 | 9,081,896 / 580 |
| Selfhost canonical artifact bytes | 8,191,345 | 9,598,554 |
| Tier 1 mode-0 bytes / functions / nodes | 832,748 / 244 / 16,460 | 832,748 / 244 / 16,460 |
| Tier 1 C2=C3 ELF bytes | 2,591,436 | 2,591,436 |
| Direct local full-kernel work | 2,573,097 | 3,091,132 |
| Public rebuilt aggregate semantic work | 9,177,359 | 11,111,393 |
| Composed work alone | Not separately measured | 7,985,578 |
| Composed modeled metadata bytes | Not separately measured | 11,392,635 |
| Physical pack files / bytes | 88 / 353,464,648 | 233 / 390,397,862 |
| Reachable historical canonical payload bytes | 434,247,127 | 469,578,376 |
| Native requested allocation bytes | Not measured | Not measured |
| Native live retained storage / mapped capacity / RSS | Not measured | Not measured |

The current public aggregate sums separate local and composed operations; it is
not headroom under one remaining ten-million limit. Composed work alone is below
ten million, while the old reference policy also carried earlier source work.
The current source carrier retains 552,043 bytes of headroom below 8 MiB.
Mode 1 already exceeds its former aggregate limit and succeeds.

Historical reachability is measured by public GC on two isolated data fixtures
containing the exact versioned HEAD and immutable packs. Both retain complete
ancestry, receipts, transactions and source objects, without operational
selection/cache roots. The original project's HEAD and every pack hash are
checked unchanged; both joined fixture directories are removed. Physical pack
bytes include compression, framing and duplicate/unreachable payload; they are
not reachable historical canonical bytes. The baseline retains 205,689 objects
and the candidate 223,454. GC wall times are 157,321 and 168,885 ms respectively;
modeled preparation retention is 486,903,511 and 526,782,600 bytes, not measured
allocator use or RSS. The complete measurement report SHA-256 is
ee5b8c6b32abcb5bf634f4c5a7995ea27cd2c15e30923d5efb0aa38b1f87984c. There is no original-project repack, retirement deletion,
ancestry squash or reseed.

The unchanged planner request failed in 10.763 seconds and succeeded in
13.622 seconds after the policy correction. These outcomes demonstrate admission,
not a speedup. Complete inventory passes in 11.61 seconds. The opt-in maintained
native measurement passes in 21.60 seconds, with 21,591 ms before report encoding:
10,736 ms initialization, 92 ms carrier export, 249 ms local full validation,
5/74 ms mode-0/mode-1 export and 470 ms strict artifact loading. Native filter
harness wall times are 247/580 ms for C1/C2 mode 0 and 1,355/2,726/2,732 ms for
C1/C2/C3 mode 1. They include publication, process creation, transport, execution,
wait and cleanup. Native allocation and resident memory have not been measured;
no same-process reclamation claim follows from successful short-lived processes.

## Maintained Tier 1 fixed points and remaining frontier

| Object | SHA-256 |
| --- | --- |
| Mode-0 profile-4 input | 40aba89a73cb8c7cee48bceb4123df2292249c9b6a32b400369d62b63d253c94 |
| C1 native backend | cad0b144ff4be1744db8087ec0bb321ab893520ed2aaa359ce69c8f9bb6323da |
| C2=C3 native backend | de7202a5769efec3325c1de9f2b57d13f98bd714a4e61aa869ff5e2fb05193ca |
| Mode-1 profile-5 input | da7d1705163f9e7111800745aa117e3ebe983f1ae292a4e9e547cd9666d5169d |
| C1/C2/C3 selfhost artifact | baed9bb54a375464e6e4f334d5afb3b207525cb29bf13fc0d292aaea125e4d1f |

These Tier 1 identities bind to the complete maintained carrier above, the
compile-elf entry, profiles 4/5, Linux x86_64, empty native environment, 4,093-byte
stdin chunks and the recorded harness limits. Rust supplied normalized input.
The equality does not establish source-driven C2=C3 or equality with an
independently packaged Rust canonical artifact. Scalar source execution is
separate evidence; full A1=A2=A3 and maintained source self-compilation remain open.

The additional combined-frontend capacity observation builds
source-elf::compile with the Rust native backend: 560 functions, zero rewrites,
698,850 ELF bytes. The current mode-0 exporter rejects at encode_records'
aggregate guard. Earlier function/type checks exclude those alternatives:
the normalized closure exceeds 32,768 instructions. The exact node count is not
yet recorded. The Rust backend has a separate 65,536-instruction bound.

Next remove that actual mode-0 node blocker through its complete exporter,
flow/planner/packing and native-address consumer chain, using checked layout
bounds and a true above-threshold control. Do not merely double a default quota.
The source carrier's separate bound and the 700-unit flat artifact map remain
nearby dependencies. Then extend scalar admission/lowering through Bytes/Text,
structural records, closed List instantiations and complete owned-buffer cleanup.
No broad infrastructure cleanup must delay those semantic steps.

## Frozen-tree publication

Publication requires exact staged-tree bootstrap, the literal
tools/selfhost/verify.sh, all 29 full --fresh gates and release-target inspection.
The current checker owner is tools/bootstrap-verifier.sh and lkjscript-dev;
this checkout has no root ./verify. Source/test paths widen changed selection
to the complete matrix, so that matrix runs once without a duplicate changed run.
Focused default-feature selfhost/native tests and the full all-feature workspace
matrix have distinct coverage and both remain required.

All acceptance runs use the frozen staged tree and one leased target. Evidence
is under .artifacts/selfhost/source-scalar; the official compact receipt is under
.artifacts/lkjscript-dev/latest-receipt.json. The two opt-in measurement tests
remain ignored in ordinary acceptance. Exact terminal receipts, stable inputs and
the tested tree are required before commit and normal main integration. Git owns
the final integration identity. No product release or deployment is implied.
