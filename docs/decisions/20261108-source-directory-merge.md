# Linear raw-source directory disjointness

## Decision

The source compiler checks that a live owner is absent from the retirement
directory by merging the two strictly ordered key directories. Each iteration
compares the complete 17-byte domain-plus-identity key directly in the original
carrier. Equal keys retain semantic diagnostic 14. Otherwise the cursor for the
smaller key advances; exhausting either directory proves disjointness.

This replaces one retirement-directory binary search and one temporary key slice
per live owner. For N live owners and R retirements, the pass uses O(N + R) key
comparisons and constant cursor storage. Both recursive continuations remain in
tail position. The merge does not copy, normalize or filter either inventory.

## Admission and authority

Header/profile checks, checked directory extents, strict ordering and uniqueness,
all object-span bindings, and retirement-record checks still precede disjointness.
State and provenance checks still follow it. No input is admitted merely because
the new merge returns success. Malformed directory ordering remains framing
diagnostic 12; validly framed live/retired overlap remains diagnostic 14.

The current u32 carrier coordinates and I64 byte-index representation are
unchanged. Previously checked extents establish that both cursor addresses are
inside their directories. This change removes repeated search work; it does not
abolish a representation boundary or establish a new capacity milestone.

Explicit proof allowances and charge sites remain unchanged. None remains
unbounded; every Some value, including zero and u64::MAX, remains finite. Runtime
instruction and modeled allocation observations may change with the algorithm;
those observations do not become admission quotas.

The private disjoint function gains an I64 retirement cursor. The existing
indices function supplies the initial pair (0, 0). The maintained proposal and
accepted graph must change together through public change plan/change apply.
Existing named identities and contracts are preserved except for this explicit
private parameter extension. Framing, carrier profiles and artifact formats do
not change.

## Independent controls

A publicly authored nominal fixture supplies the unchanged live program and
type inventory. Small synthetic retirement histories exercise both nonempty
directory exhaustion directions, interleaving, a shared 16-byte identity in
different owner domains, same-domain keys differing only in the final byte,
a middle overlap and a final-key overlap. A HashSet intersection and the Rust
kernel validator determine independent semantic expectations; exact rebuilt
map commitments prevent a stale claim from explaining semantic rejection.

Separate byte mutants duplicate or reverse a nonempty retirement directory.
They retain the header and all other sections and must fail framing admission.
Every new case runs through read-carrier and read-graph in the VM, reference,
Rust-produced native and self-produced native implementations.

Existing empty-directory, retirement-only, explicit-allowance, framing and
complete-inventory controls remain required. Maintained proposal/body agreement
checks the public adoption. Fresh raw-source fixed-point evidence must compute
C2 from C1, C3 from C2, and A1/A2/A3 from their respective compilers, then check
C2=C3 and Ahost=A1=A2=A3 with strict loading and independent behavior/mutation
controls. The same raw test checks C1 and C2 output survival and scratch cleanup.

## Measurement contract

The unchanged baseline on main 9a804993 passed the complete ordinary nominal-flow
test: one passed, none failed or ignored, in 3,401.132 seconds. It exercised the
full maintained compiler, a small independent finite schema and a complete
compiler namespace mutant across all four engines, with unchanged source/image
bindings.

| Logged validation phase | Complete valid compiler | Namespace mutant |
| --- | ---: | ---: |
| VM | 247.055 s | 234.006 s |
| Reference interpreter | 1,398.984 s | 1,307.981 s |
| Rust-produced native filter | 10.849 s | 10.382 s |
| Self-produced native filter | 10.749 s | 10.105 s |

The mutant independently rejects with semantic diagnostic 16. Its native filters
return the expected data-error exit 65. The small fixture also passes. The
170.961-second residual after all logged phases is unattributed.

The complete baseline receipt is
.artifacts/capacity-unbounding/nominal-flow-20261109/baseline-1/receipt.json,
SHA-256
3322e7c73a5fac4f564e636f78920539ba99d673c08a7ddfc6a4af51f83c3296.
The measured original carrier SHA-256 is
9330ec01d87d6fa202a0642b06bd8de20194ad5d629374317f73d30dea7796c1.

The exact original carrier is retained under
.artifacts/capacity-unbounding/nominal-flow-20261109/pinned-current-carrier.bin.
The candidate must also validate those identical bytes using the existing pinned
carrier control. The newly adopted complete compiler has a different carrier;
its ordinary acceptance run is recorded separately from the identical-input pair.

A data-only replay over the original 81,456-owner and 61,339-retirement directories
counted 1,297,789 binary-search key comparisons and 142,786 merge comparisons.
Byte comparisons were 3,158,405 and 466,336 respectively. These counts describe
the selected pass only. They do not measure total VM instructions, actual
allocator traffic, elapsed time or whole-compiler speedup.

Phase timings measure the logged engine scopes. Native filter timing includes
process startup, input transport and joined cleanup but excludes reader-image
construction. Unattributed elapsed time includes preparation, mutation/oracle
work, encoding, copying and cleanup; it is not a measured cache cost.
Process VmPeak, VmHWM and VmRSS remain distinct from cumulative evaluator
allocation observations and native phase slab observations.

## Acceptance scope

This proposal does not establish acceptance. Actual joined measurements,
public graph readback, focused controls, genuine raw fixed points, unchanged
source/image proofs, fresh checker gates and release-target evidence must be
recorded before integration. Full Tier 2 Milestone B, general recursive-value
lifecycle closure and deep non-tail native call management remain separate work.
