# Derive mode-0 instruction capacity from the complete encoding

Date: 2026-10-23. Status: implemented in the maintained canonical compiler;
publication is bound to the exact staged-tree gates below.

## Decision and dependency

Remove the independent 32,768-instruction admission ceiling from the LKJCI1
mode-0 producer and its complete consumer chain. Keep the existing byte, function,
local, operand, type and machine representation domains. Derive early count
checks from the space available in that retained encoding, and check the complete
layout before allocating the Rust export buffer.

The accepted scalar source frontend was executable through the Rust native
bootstrap, but its combined frontend/backend closure could not be exported to
the maintained lkjscript-written backend: the exporter rejected its aggregate
instruction count. That concrete dependency, rather than a general desire for
larger tables, selects this change. The predecessor measurement recorded 560
functions and a 698,850-byte Rust-bootstrap ELF; it did not record the rejected
normalized count. The new measurement records and consumes the actual export.

## Boundary classification and finite domain

The removed count was an implementation admission ceiling. It is not a semantic
requirement, a selected execution quota or the width of an encoded field.
The following bounds remain independent:

| Boundary | Classification and treatment |
| --- | --- |
| Complete mode-0 input at most 8,388,608 bytes | Retained profile/implementation bound; the native layout argument below depends on it. |
| Function count 1 through 1,024 | Retained table/profile bound. |
| Positive total instruction count | Required shape of this executable profile. |
| u32 table counts, offsets and lengths | Representation restriction; checked Rust conversions and complete extent admission remain. |
| 128 locals, operands and record fields; 120 record types | Retained implementation and type-encoding restrictions. |
| Exact format proof allowance | Deterministic enforcement retained, including rejection at required allowance minus one. |
| Native launcher deadlines and output/image quotas | Explicit caller-selected limits remain in force. |
| Arithmetic and allocation failure | Checked layout arithmetic rejects; failure to reserve the Rust encoded output buffer returns `compiler_input_unsupported`. Earlier projection allocations are outside this new reservation guarantee. |

Let F count function rows, N count instructions, and B count blob bytes. The
complete input must satisfy

`2176 + 40*F + 32*N + B <= 8,388,608`, with `1 <= F <= 1024` and `N > 0`.

Before blob bytes are known, the necessary node capacity is
`floor((8,388,608 - 2176 - 40*F) / 32)`. This expression derives from the retained
complete-input bound; it introduces no replacement independent instruction quota.

The unchanged padded native image uses `65536 + 512*F + 128*N + B` bytes.
Maximizing this expression over the retained input domain gives a conservative
33,971,712-byte image bound. The maximizing table-only envelope is F=1024,
N=260796, B=0; actual program state and payload requirements can only reduce it.
The highest image address from base `0x400000` is at most 38,166,015. Relative
branches/calls fit signed rel32 and absolute data addresses fit u32. The retained
128-local domain bounds the frame adjustment and negative local displacement to
1,024 bytes. These are code-derived bounds, not observations of a maximum-sized
admitted executable. A later input-byte expansion must redo this argument.

The proof allowance is at most 2,146,912,321 over the same complete-input
inequality, below its retained 2,147,483,647 representation limit. The explicit
32 MiB native harness output quota is smaller than the conservative image bound;
it can still reject an extreme otherwise admitted input.

## Complete producer and consumer closure

`selfhost_input.rs` removes `MAX_NODES`. Its early per-function count check uses
the available node-table space after all function rows. `encode_records` checks
aggregate nodes, function/node sections, record fields and names, local bytes,
node payloads, initialization flags and operand states before reserving the
complete encoded output. The preflight uses checked additions/multiplications;
affected count and offset conversions use `u32::try_from`. Serialization must
end at exactly the preflighted length.

`flow::tables` requires positive N and rejects the fixed-table extent when it
exceeds the actual input before walking node records. Even a malformed u32-max
node count multiplied by 32 fits I64, so that rejection does not depend on
overflow behavior. Current magic/profile, checksum, type, flow, forward-branch,
local custody and complete-certificate checks remain in their existing order.

`source-scalar-pack::node-capacity` checks F before multiplying it. The scalar
planner uses the one-function envelope for intermediate expression counts and
the actual F for aggregate function counts. Remaining-capacity comparisons
precede additions. The packer independently checks function/node section lengths
and the complete blob extent before assembling the normalized input. The native
backend then admits that input through its ordinary independent flow checker.

`machine`, `elf`, record-slot addressing and the native stack ABI require no
encoding change within this proven domain. Artifact mode's per-unit/table limits
and the Rust native backend's separate limits are unchanged.

## Termination and publication guarantees

No synthetic fuel, cancellation check, semantic cycle restriction or output
publication guarantee is removed. Validated flow and machine scans advance a
finite function/instruction index. Source item scans advance a finite reference
cursor; aggregate counting advances a finite function index. Count-resolution
rounds repeat only when the unresolved-owner count strictly decreases. Negative
diagnostics return before count rows or output images are published.

## Canonical adoption and bootstrap sequence

The public transaction uses the accepted scalar project revision
`rev_58c7fb8d64952a3b5f12e6637b4265657ff13b023a870af12af8b3f19153ba87`.
It updates six existing function bodies and adds `source-scalar-pack::node-capacity`
through the public authoring path. The accepted result is
`rev_9793beb62fcf1a8ffdf61e7bbf9a6c5a310e0d4ae3b77aa86f23cb023af26c3d`,
with 420 created owners, six updated owners and differential result `equal`.
Existing declaration and parameter identities are retained. Source projections
remain proposals; no direct canonical object write or history reseed occurs.

1. **Builder:** Build the public Rust CLI from accepted commit
   `2824d948a15d1974bd4a664207cc6518e4ebd528` before editing sources, use it for
   canonical adoption, and use the Rust native backend to bootstrap backend C1.
2. **Consumed input:** The Rust bootstrap consumes the existing normalized
   program. The produced backend consumes current `LKJCI001`, mode 0/profile 4;
   artifact mode remains profile 5. Raw source controls use complete `LKJSG001`.
3. **Prerequisite capability:** Existing checked arithmetic, finite scans and
   append buffers suffice. No new runtime primitive or compatibility bridge is
   introduced.
4. **Regeneration:** Public adoption regenerates the maintained semantic closure;
   measurement exports its complete source, normalized backend/frontend inputs
   and canonical artifact input and output. Owners, retirements and every byte
   count are measured separately.
5. **Independent verification:** The historical-node/fuel control, malformed
   extent control, native source-capacity control and maintained backend
   reproduction cover the wider path. Exact staged-tree bootstrap, the literal
   selfhost driver, fresh full acceptance and release-target inspection bind the
   final publication tree.
6. **Cleanup:** No temporary reader, format bridge, alternate target directory or
   copied gate executable remains. Only the current format is accepted.

## Behavioral controls and evidence boundaries

The normalized arithmetic witness grows from the historical 32,768 threshold to
32,770 nodes. It must emit an executable returning 16384 and reject exactly one
less than the required proof allowance with diagnostic 6. A separately
rechecksummed small input advertises u32-max nodes and must reject at the
structural extent boundary with diagnostic 3.

The independent source witness is publicly authored as a balanced Sequence tree
with 16,385 distinct literal leaves, at most 16 children per Sequence and four
Sequence levels. Rust transports the complete carrier without lowering that
target. Literals, Drops and the final Return derive exactly 32,770 instructions,
16,385 operand-state bytes and a 4,276,993-byte ELF. The emitted native program
must independently execute to 42. This is an E3 planner/packer/flow/codegen control.

This larger fixture selects an explicit bounded authoring policy through the
ordinary typed public request. Its owner envelope is `2*leaves + 5 = 32,775`;
the initial/final summary allowance is 65,550 and the selected relation allowance
is 524,400. Affected, summary-edit and behavior-owner allowances use the same
owner envelope. The fixture retains its 128 MiB memory store, 10-second native
filter deadline and 32 MiB output quota. This fixture policy is separate from
mode-0 admission; authoring defaults remain a future capacity obligation.

The balanced shape bounds repeated copies of a Sequence parent's source bytes.
It is not an optimization to production source traversal and does not establish
a whole-compiler complexity or memory result. Derived copy components, compiler
wall time and requested allocation/RSS are different quantities.

The maintained measurement additionally exports the combined source frontend to
backend C1 and C2, compares their emitted frontend images, and runs the same raw
scalar source controls through those images and the Rust bootstrap. A valid
unused Bytes type changes the complete carrier while retaining semantic-state
header bytes; the scalar frontend must reject it with diagnostic 13. The
complete carrier, executable, profile, target and execution context identify each
observation.

These checks establish Tier 1 backend reproduction and source-driven scalar
execution. The maintained source compiler still cannot compile its own full
source profile. This change establishes neither source-driven compiler C2=C3,
Rust/C1/C2 canonical A1=A2=A3, nor native same-process reclamation.

## Measurements and acceptance

The before column is accepted scalar commit `2824d948`, revision
`rev_58c7fb8...`. The after column is the public revision `rev_9793beb...`
identified above. This is a maintained-closure comparison, not a same-input
speed benchmark. No fuel accounting definition changes in this slice.

| Observation | Before | After |
| --- | ---: | ---: |
| Modules / callable declarations | 72 / 576 | 72 / 577 |
| Live canonical owners | 33,640 | 33,666 |
| Retirement records | 10,586 | 10,980 |
| Supplied type descriptors | 35 | 35 |
| Complete LKJSG001 carrier bytes | 7,836,565 | 7,925,055 |
| Remaining source-carrier byte headroom | 552,043 | 463,553 |
| Backend mode-0 bytes / functions / nodes | 832,748 / 244 / 16,460 | 832,576 / 244 / 16,456 |
| Combined frontend mode-0 bytes / functions / nodes | Rejected by node guard | 1,661,846 / 561 / 34,214 |
| Combined frontend mode-0 record types | Not measured | 31 |
| Mode-1 bytes / canonical units | 9,081,896 / 580 | 9,089,215 / 581 |
| Emitted canonical artifact bytes | 9,598,554 | 9,606,367 |
| Physical pack count / bytes | 233 / 390,397,862 | 277 / 401,520,402 |
| Reachable historical objects / canonical payload bytes | 223,454 / 469,578,376 | 226,375 / 480,434,611 |
| Retained physical pack bytes in isolated GC copy | 375,771,777 | 384,013,851 |
| GC preparation modeled retained bytes | 526,782,600 | 538,386,611 |
| Direct local full-kernel work | 3,091,132 | 3,094,806 |
| Composed callable work | 7,985,578 | 7,998,497 |
| Composed callable modeled metadata bytes | 11,392,635 | 11,405,716 |
| Maintained native measurement wall time before report encoding | 21,591 ms | 24,123 ms |
| Native requested allocation / live retained bytes | Unmeasured | Unmeasured |
| Native mapped capacity / resident memory | Unmeasured | Unmeasured |

Replacing six bodies adds historical retirements. The 26 additional live owners
therefore coexist with an 88,490-byte carrier increase. All 233 previous packs
remain unchanged; 44 additions account for 11,122,540 physical bytes. The isolated
public GC observes ancestry, receipts, transactions and source-object roots,
without derived-cache or operational-selection roots. It runs for 174,843.189095
ms, checks disk counters against independent inventories, preserves the original
project and checkout, and removes its joined fixture. Modeled GC retention is
neither requested allocation nor native resident memory.

The earlier public rebuilt-current-validation total of 11,111,393 was an
aggregate over several analyses, not headroom beneath one kernel ceiling. This
slice records direct and composed work separately and does not rerun that public
aggregate observation. The maintained-native after measurement also performs new
frontend reproduction and execution controls; its longer total has a different
work inventory. No same-input old-versus-new implementation benchmark or speedup
is claimed.

The normalized arithmetic witness runs through the VM, independent language
interpreter and native backend. VM telemetry reports 126,758,073 steps and
13,798,636,179 cumulative allocation bytes; the reference reports 70,018,658
expressions and 16,908,865,035 cumulative allocation bytes. Those observations do
not measure native live memory or reclamation. The source witness compiles
natively in 1,735 ms from a 2,555,726-byte carrier and independently executes to
`42`. Its carrier SHA-256 is
`ffed6ad54aec24f96c052663efd8ab6939e80d9aea6562cddec19a16d3f3398b`;
the 4,276,993-byte emitted ELF is
`72e4afa715661f28062c854ca99d0fee14d1271c9c8c0c6f19d81b5b6cd25077`.

The maintained complete carrier SHA-256 is
`99916677a11a0216a68c127c97f75d18f783531c5103b76089a36d623db6aedd`;
its semantic state is
`semantic_state_46901f9d2d9b88d28aa1b8237ce5f52a5a78dfc5cc1b147f281f599950f9ca8e`.
The Tier 1 backend reproduces C2=C3 at
`55eaea3fcc2604a386f68a5dffd5bb9c2ca8e936ca0d2bf94235e1770c2487d5`
(2,590,880 bytes), and C1/C2/C3 emit artifact bytes with SHA-256
`fb1bca32c7de598fb1fa8e99400f433fd70762cc26bfad5c6f4a4b5c9a280364`.
These remain Rust-normalized-input proofs. Both backends also emit identical
5,274,542-byte source frontends, SHA-256
`486f810af4aecc00212d9f026e5e21df509582c6e14f5106c7d298aa3b4a6b34`.
Those frontends and the Rust-bootstrap frontend compile the same raw positive
carrier to the same independently executed scalar program and reject the
hash-consistent unused Bytes descriptor.

Evidence under `.artifacts/selfhost/mode0-node-capacity/` binds the exact source,
complete carrier, execution context and public adoption:

| Evidence | SHA-256 |
| --- | --- |
| `continuation.json` | `e441e877afaebcf6c0a69ec1891633881b91209446b69ac93bd03b18717948b0` |
| `maintained-native.json` | `cba93b740a88ab6d3ec480d2d7f7cca7b5d4c88c3d0b7b49e33a1ca904587a09` |
| `complete-profile.json` | `17a3229dbd8d10a19c934250007fab83e1c7e7fcb530093c17ffbedcbc438ee8` |
| `reachability-<continuation hash>/report.json` | `bce261ff43de8ef7b8446846e00de36f6dda5846032b914a1e6a33e7d3434f84` |

The continuation preserves the first targeted attempt as failed, identifies the
single fixture-authoring correction, and binds fresh logs for all five targeted
commands. It does not replay the accepted public transaction. The final run
executes one historical-node/fuel test, one malformed-extent test, five scalar
tests, one opted-in maintained-native measurement and one complete-profile test;
all pass. Logs retain test durations separately from build/driver time.

Before publication, `.artifacts/selfhost/mode0-node-capacity-acceptance/` must
bind the exact staged tree, targeted and reachability receipts, fresh gates and
normal integration. The gate contract requires staged-tree bootstrap, literal
`tools/selfhost/verify.sh` with 126 passing selfhost tests and two opt-in tests
ignored plus ten native tests, all 29 fresh full gates without reuse, and
release-target inspection. Changed-path selection reaches the full matrix, so
it runs once through `check full --fresh --machine`. Owning terminal receipts
determine publication; expected counts never substitute for successful results.
No root `./verify` executable exists.

## Next frontier

Use actual post-adoption source-carrier headroom to decide whether capacity must
advance before the next semantic addition. The next prepared scalar capability
admits exact I64/Bool intrinsic declarations and lowers their calls; explicit
Target selection and immutable Bytes/flat records follow. Closed List
substitutions and complete ByteBuffer cleanup are required before the maintained
compiler can self-compile from source. Whole-carrier admission, complete identity
binding, full Tier 2 families and measured native reclamation remain obligations.