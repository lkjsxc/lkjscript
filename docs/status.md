# Current status

Updated 2026-11-01 (Asia/Tokyo). Accepted typed meaning is the sole editable
program authority. Source proposals are adopted through public transactions.

## Current milestone and delivery boundary

The maintained compiler has passed the complete raw-source Milestone A component:
its native generations consume the exact authoritative source closure, perform
source admission and lowering, and reproduce both ELF and canonical artifacts.
The test establishes `C2 = C3` and `Ahost = A1 = A2 = A3`, with independent
behavior, source mutations, complete-inventory negatives and strict loading.
The original repository was byte-for-byte unchanged by the test.

This is focused component evidence, not the fresh full staged-tree acceptance
seal. The working branch is `compiler/20261031-source-active`, based on
`71405a389a422a8fa51850ef70ba9f34040aca56` (tree
`d1036ebca89238a914d221c7ba3d0d951b4f0f21`). That base is the last observed
local/remote mainline commit; its tree does not contain this candidate.
The [source self-compilation decision](decisions/20261101-active-source-self-compilation.md)
records exact component identities, measurements and evidence boundaries.
Terminal acceptance and integration must bind the finalized staged tree in the
acceptance commit and external receipt, without editing sealed inputs.

Tier 2 remains incomplete: full `P_self` semantics (Milestone B) and repeated
same-process native phase reclamation (Milestone C) are separate obligations.
No release archive or deployment has been published for this candidate.

## Authoritative source and exact outputs

| Identity | Current value |
| --- | --- |
| Graph revision | `rev_cd5e4e228db3c5abadec2e3e28bb2fbda602c3812177a6ab666fcf2240eb992a` |
| Public HEAD SHA-256 | `39d171a01e15cdf6fc9999c61dfcd0511cb7dc7a9600f274dcb3fa6b2d883082` |
| Semantic state | `semantic_state_01600719c76251652588c725f184f5b392b63d3a69f4ae36f52d18ba486fc606` |
| Complete source carrier | 17,598,834 bytes, profile 3 |
| Carrier SHA-256 | `404a5a97895b4156f67ef3c14c1f612007e4c238bed4054e88128677047e04bd` |
| C2/C3 standalone Linux x86_64 ELF | 3,869,405 bytes |
| C2/C3 SHA-256 | `ecef194cc2751ebb73d59ce4fed4e5c3c0353179e2affa7787a29aafd3d53b59` |
| Ahost/A1/A2/A3 canonical artifact | 16,947,150 bytes |
| Artifact SHA-256 | `59e1d8a6b9867e2ab2c43636dd8b75a0b4df8d2ca1ad7fe1d6607ef74f3430e1` |

The complete inventory contains 118 modules, 59,883 owners, 88 supplied types,
1,096 callables (1,069 pure and 27 external), 50,246 expressions, 4,514 lexical
bindings and 35,651 retirements. All 20 generic application sites and all
supplied, uncalled, untaken and unselected source are inventoried. Parameter modes
are 3,595 unrestricted, 300 consume and one borrow. Public readback covers all
owners in 15 pages. Complete proposal replay matches 1,103 canonical units.

Public GC retained the exact HEAD and complete source transport while compacting
200 packs / 199,266,800 bytes into seven packs / 191,469,725 bytes. These are disk
observations, not native memory reclamation. The 21,795,545-byte source transport
has SHA-256 `86e2aeda5ad71147deb5c1e4e5e2f88ffb13b92ad20c7aaa1ae2afd8d5d3cecc`.

## Implemented candidate capabilities

Source compilation covers the active scalar/compound type closure, structural
record inference, fields, lists, lexical control flow, direct recursion, closed
I64 generic list substitutions, exact intrinsic signatures and binary-filter
entrypoints. Affine ByteBuffer admission tracks consumption, synchronous borrowing,
branch-balanced disposal, reverse lexical cleanup and returned-owner transfer.
Owned locals prevent unsafe tail transfer; lowering emits call/cleanup/return
instead. Canonical unit, summary, interface, map, pack and artifact construction
is source-driven, not supplied as Rust-normalized target input.

`LKJSG002` profile 3 carries the original revision core and claim; `LKJSRQ01`
profile 2 carries caller TargetId and ELF/artifact mode. Predecessor selectors
reject. Unused supplied types remain bound by the complete carrier hash even when
they do not change semantic-state identity. Canonical meaning stays in its current
schema epoch. Mode-0 profile 6 and Mode-1 profile 7 remain the current backend
projections, which the source compiler now constructs itself.

Ordinary package/source admission, publication previews and canonical-reference
proof preparation no longer impose their former aggregate default work/storage
quotas. Exact optional finite policies, independent saturation flags, cancellation,
structural checks and atomic publication remain. The identical complete compiler
transaction that previously exhausted the reference proof allowance was publicly
accepted after that path was repaired. Recursive lifecycle guards are not removed.

Three closed byte intrinsics are supported by both evaluators and both native
compiler paths: `core.buffer.append-slice`, `core.buffer.append-i64-le` and
`core.bytes.read-i64-le`. Source support preceded adoption in maintained helpers.
Bulk operations avoid per-octet boxed temporaries. Immutable native scalars -1
through 255 and both Booleans reuse read-only objects; other I64 values retain
checked allocation. Neither mechanism limits the semantic integer domain.

## Focused evidence and measured cost

The complete raw-source fixed-point test passed with 1 passed / 0 failed /
0 ignored in 524.24s test time. Receipt SHA-256:
`29c9087d37072113fb723469e2c0cc195366a412259e1e1e045f29cb177fb551`.
Its source-file inventory binds the working candidate independently of the base
Git commit. The test includes 82 active-profile cases (26 positive, 56 negative)
on both C2/C3 and both output modes; bulk-byte controls; four-engine behavior;
layer-reaching diagnostics 6 and 12–19; and a compiler-source mutation changing
an independently expected diagnostic from 12 to 77. Every artifact generation is
strictly loaded with complete unit and provenance comparison.

Additional completed focused selections include 224 canonical-reference/admission
tests; 27 source semantic tests with two explicit measurement tests ignored; six
artifact tests with the whole-source control separately invoked; and four
inventory/replay controls. Release workspace/all-target/all-feature Clippy with
`-D warnings`, product build and generated-discovery verification passed before
finalization. These selections do not substitute for fresh full acceptance.

On the same pinned 17,263,085-byte raw artifact request and source graph, the old
native scalar allocator exhausted an explicit 24 GiB process address-space policy
without output. The new scalar pool completed in 18.304s, producing the exact
16,783,727-byte oracle artifact; sampled peak VmSize/RSS was 16,041,940 /
15,798,460 KiB. This establishes completion under the same explicit process policy,
not a timing speedup against the failed run. Current-source artifact emission
completed in 19.303s with sampled peak RSS 16,452,192 KiB. Actual allocation traffic,
bytes copied and peak live language data were not measured. General native scratch
allocations still remain until process exit.

## Next acceptance and architectural work

Finalize and stage this dependency-closed candidate; bootstrap the exact staged
tree; run the literal selfhost/native script, named maintained-native-closure and
whole-owner/type controls, the explicit raw-source fixed point, all 29 fresh full
gates and release target; commit only the tested tree and integrate normally.
The named historical closure control now uses unsupported F64 rather than the
newly supported Bytes type for its unused-descriptor negative; the corrected
control passed in 45.48s test time (1 passed, zero failed/ignored).

After integration, prioritize separate child phase allocation contexts and sound
survivor promotion, with repeated native invocations in one process, including
failure and cancellation. Process exit, scalar interning and bulk-copy efficiency
are not that proof. Then close remaining `P_self` nominal owned records/choices and
Match, closures/Invoke, explicit owned witnesses/read views and lexical Regions.
The [capacity ledger](decisions/20261025-capacity-unbounding.md) retains the coupled
canonical-unit/object envelopes, lifecycle/stack safety, map key/value, catalog/GC
and package/data/queue/HTTP/tooling families. Repository-wide capacity freedom,
full Tier 2 and Tier 3 sovereignty remain unproved.
