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

## Completed identical-carrier measurement

The unchanged baseline on main `9a804993` passed the complete ordinary
nominal-flow test: one passed, none failed or ignored, in 3,401.132 seconds.
It validated the complete maintained compiler, a finite-schema fixture and a
complete compiler namespace mutant across VM, reference, Rust-native and
self-produced native. The mutant independently rejected with diagnostic 16;
its native filters returned the expected data-error exit 65.

The candidate's separate pinned test passed on the exact earlier complete
26,611,855-byte carrier, with 81,456 live owners, 97 types and 61,339 retirements.
The measured entry is read-nominal, including its graph admission; read-graph was
not separately timed. Its expected result is diagnostic zero and the carrier's
32-byte semantic state.
Both arms retain their actual input, source, original Cargo-selected image,
stream and completed terminal bindings. Historical executable proofs do not
assert that an overwritten target path still contains the old image.

| Logged valid-carrier read-nominal phase | Before | After | Observed reduction |
| --- | ---: | ---: | ---: |
| VM | 247.055 s | 231.185 s | 6.42% |
| Reference interpreter | 1,398.984 s | 1,321.052 s | 5.57% |
| Rust-native filter | 10.849 s | 10.192 s | 6.06% |

| Evaluator observation | Before | After |
| --- | ---: | ---: |
| VM instructions | 1,971,934,665 | 1,832,896,106 |
| Reference expressions | 1,088,736,938 | 1,028,088,680 |
| VM cumulative modeled allocation, bytes | 164,857,066,319 | 154,821,477,752 |
| Reference cumulative modeled allocation, bytes | 184,201,495,692 | 171,180,581,884 |

These are one pair of the logged complete valid-carrier phases. The comparison
does not pair whole test durations, namespace-mutant executions or
self-produced-native executions; it establishes no statistical or whole-compiler
speed guarantee. Native filter timing includes startup, input transport and
joined cleanup, and excludes reader-image construction. No wall-clock improvement
threshold controls acceptance.

Cumulative modeled allocation is not RSS or live heap usage. Linux VmPeak is
peak virtual address space, VmHWM is peak RSS, and VmRSS is current RSS; these
remain separate from evaluator observations and native slab observations.
Unattributed test time is not assigned to a cache or allocation mechanism.

A separate data-only replay over the original directories counted 1,297,789
binary-search key comparisons and 142,786 merge comparisons; byte comparisons
were 3,158,405 and 466,336. These counts describe only this pass and do not
substitute for the engine measurements.

The comparison record is
`.artifacts/capacity-unbounding/nominal-flow-20261109/identical-carrier-comparison-1.json`,
SHA-256 `ff13ef9a304ad4b726c4171876ddc4c196213ece227f71a7f22182b97b7280ad`.
It binds the completed baseline and candidate receipts and actual observations.
The retained input is `pinned-current-carrier.bin` in that directory,
SHA-256 `9330ec01d87d6fa202a0642b06bd8de20194ad5d629374317f73d30dea7796c1`.

## Public adoption and complete current source

Public `change plan`/`change apply` adopted the proposal and its private
parameter-order change, followed by public inventory/signature readback.
The completed focused run passed all seven required controls. Its complete
public export has 81,458 live owners, 97 supplied types and 61,470 retirements,
including 135 modules and 1,409 callables. The carrier is 26,640,313 bytes.
These current source bytes differ from the retained measurement input.

| Identity | Exact value |
| --- | --- |
| Base commit | `9a8049930fb79abbae091968211d8d90a1e37a60` |
| Corrected implementation tree | `01c3406030eaa6ba4c28c4a929bd00028a40bc38` |
| Corrected complete source snapshot SHA-256 | `92d471a5b5a1604eccf17fe6463581ada41e45fe56bcdadb48a1f97ffa76ee0a` |
| Retained compiler-controls tree | `933f87a009caea8b5e61a150764dc57d1e5d5e47` |
| Retained controls complete source snapshot SHA-256 | `90b0af8aca131254186b2a54963a5aafa4e88428bef9a4f5efade97c628961bb` |
| Public graph HEAD file SHA-256 | `caa4220afdebf7072f40044cfbf7cf622c1fa8b9afca20a6addfa4039273fc31` |
| Accepted graph revision | `rev_bf2a4f329456d701daf9583247721b7040c6a83df067b7361d8d0f202aecbf74` |
| Semantic state | `semantic_state_ac6dc9f0db1b063491e846734ad60f110394c75d2a2969c9894eb966b6fbd32a` |
| Revision core SHA-256 | `4cf594d3f45bc5137803dc8221e0f39398adc4b51d9dfaf15ce25acad9046d5c` |
| Complete carrier SHA-256 | `acea4485c9b3e83198903ad69ad9ebe294499f9808bfc2064696cecec06b6cf2` |

## Corrective test-only change and evidence scope

The first changed attempt was rejected: Clippy exited 101, workspace execution
was skipped, and 27 other gates passed fresh. The overall command exited 1 and
left its source unchanged. Its receipt and actual terminal remain retained; the
attempt supplies no workspace pass or candidate acceptance.

The only correction changes the reverse-directory iterator header inside
selfhost_source_graph_disjoint_directories_match_independent_set_oracle in
`src/platform/execution/normalized/selfhost_source_graph_tests.rs`.
The pair of `chunks_exact_mut(49)` and reversed `chunks_exact(49)` iterators
becomes `as_chunks_mut::<49>().0.iter_mut()` and reversed
`as_chunks::<49>().0.iter()`. The already exact 49-byte rows, copy body, fixture,
assertions and expected framing diagnostic remain unchanged. Production,
maintained graph, raw-control and pinned-control source remain unchanged.

The corrected release library was rebuilt, and the exact affected test passed:
one passed, none failed or ignored, with matching corrected source snapshots
before and after. Corrected bootstrap and the typed verifier bridge also passed.
The seven focused controls, genuine raw generations, independent raw audit and
identical-carrier candidate measurement retain their original source tree and
image bindings. They are not described as reexecuted on the corrected tree.

The final acceptance record below verifies the exact source relation, the
fresh corrected gates and release-target result while retaining the original
compiler-control executions and the rejected attempt's actual outcome.

| Retained corrective evidence | SHA-256 |
| --- | --- |
| Exact private-test correction diff | `f4e74fc907a42e422ed25824e33961d74eef3ec6a7adc54d9a3578f501fbd1da` |
| Private test file before correction | `71bce8bb5f99a98cc135bb55aa5224b94ac19c99be7f7f0900598916b794e1ea` |
| Private test file after correction | `3e26b742ef42ad8fd1b516dfe0cbcacdb53303864a9f55249a8c9693bace679c` |
| Rejected changed attempt, compact receipt | `8f666b60362ab185072622b4a736f5fcf7424cdca4b6e4fb8910ed0a6ec42c3e` |
| Corrected focused-test receipt | `12df29b396412b9e6bffb79ba3725e225fdca9a491bdc6727d53bc38c7515664` |

The acceptance record binds the full paths, original selected images, actual
terminals, source snapshots and corrected bootstrap/bridge receipts. Historical target
proofs remain historical even when their pathname is later reused.

## Acceptance evidence and delivery

Acceptance status/date: accepted on 2026-11-09.
Implementation commit: `0dbb92336396b1dae9a9f75247ade76b1f68ffeb`.
Delivery and advertised remote observation: fast-forwarded to `main` and
pushed to `origin`; independently advertised as the implementation commit at
2026-11-09T00:31:12.704Z, with fetched remote ancestry verified.
The delivery record is
`.artifacts/capacity-unbounding/nominal-flow-20261109/implementation-delivery-1.json`,
SHA-256 `383af29f356a93a7be13541fc18000344f1845b655bcc6ee2a2cee2066603918`.
Canonical acceptance receipt: `.artifacts/capacity-unbounding/nominal-flow-20261109/acceptance-2/receipt.json`,
SHA-256 `43553937401810c1901076373a348603c828b882c6f88785d76f0a3279600803`.

| Obligation | Recorded outcome |
| --- | --- |
| Public adoption and seven independent focused controls | Passed on retained compiler-controls tree; complete current profile and readback bound |
| Exact corrected private framing test | Passed on corrected implementation tree; one passed, none failed or ignored |
| Identical-carrier read-nominal VM/reference/Rust-native control | Passed on retained compiler-controls tree; one exact successful ignored test |
| Actual ignored whole-maintained-source raw test and independent audit | Passed on retained compiler-controls tree |
| Genuine C2=C3 | 22,005,155 bytes; SHA-256 `aa8c368b224fb4b31bb6b34e001c11d5f821fbd7f85bb6f961869139ab9db474` |
| Strict canonical Ahost=A1=A2=A3 | 22,909,104 bytes; SHA-256 `3b5fa68fb045b971062e106093a120835fa69b6528c406b92fd1c75fe032b996` |
| Independent behavior, source mutation, semantic/custody rejection | Passed under the retained raw audit |
| Same-process C1/C2 survival, cancellation/retry and scratch cleanup | Passed; 17 frames per process, final owned mappings zero |
| Corrected staged bootstrap and exact typed verifier bridge | Passed on corrected implementation tree |
| First changed attempt | Rejected; Clippy 101, workspace skipped, overall exit 1 |
| Corrected changed | Passed: 29 fresh, 0 reused/unrun; actual exit 0 |
| Corrected fresh full | Passed: 29 fresh, 0 reused/unrun; actual exit 0 |
| Exact ordinary nominal-flow test in both corrected root harness streams | Passed once in each; both root summaries: 3,086 passed, 0 failed, 21 ignored |
| Release target and final source/image/terminal audit | Passed; source/index/verifier unchanged and all actual terminals joined |

The completed raw audit is
`.artifacts/capacity-unbounding/nominal-flow-20261109/raw-audit-1/receipt.json`,
SHA-256 `979d3ef368549c9a523f39ce3af4ae7d6841837a383001f951fdd576e298a141`.
It binds the complete current carrier and type inventory, original/copy/final
public source records, genuine generation ordering, independent controls and
both native phase processes to the retained compiler-controls tree.

Final acceptance preserved each execution's actual complete source, index,
image and terminal bindings. The retained controls remain on their original
tree; the exact private-test correction and its fresh affected test establish
the narrowly reviewed relationship to the corrected implementation. Corrected
changed and full each passed all 29 fresh gates with zero reused/unrun gates
and actual completed terminals. Release target and the final acceptance audit
also passed. The rejected attempt supplies no successful gate credit.
Workspace inventories and result counts came from current producers and retained
streams. No cached output replaced a generation being proved.

Later status/acceptance prose is a separate two-file documentation tree. Its
bootstrap, bridge, changed-gate outcome and eventual documentation commit belong
to the final machine handoff; they do not relabel implementation evidence or
introduce a self-referential commit identity into this record.

## Next measurement: native scalar, Bytes and direct-call costs

The statically reviewed proposal in
`.artifacts/capacity-unbounding/nominal-flow-20261109/native-scalar-probe-preparation-1/`
remains unapplied and unexecuted. Its guarded `proposal.patch` SHA-256 is
`2202c66bee06a1a021e507762667a0efe64db6f0574ab406898045eb4c863e5c`;
`inputs.json` SHA-256 is
`33baef4bb0c9e78ffc6331556d89f5d47d8ecd7f44c890f2480ec281d2566cf5`.

It proposes one publicly authored fixture, normalized input and a single target
image per emitter, comparing the Rust emitter with the maintained backend at
Tier 1. Runtime-selected scalar, Bytes and ordinary non-tail direct-call modes
use two explicit caller-selected counts. Existing lazy compiler fixtures avoid
initializing a raw C1/C2/C3 chain solely for this experiment.

AllocationIdentity observations would bind actual outputs and nonsaturated
per-handle request counters, timing scopes and final owned mappings. They count
allocator requests rather than allocation bytes, mmap calls or peak scratch.
The call control preserves an ordinary stack call so call-frame work can be
distinguished from heap requests. Values outside the scalar pool come from
runtime data; the proposal neither widens the pool nor introduces a default
work quota. Its successful-invocation mode supplies no new cancellation/OOM
recovery or deep native-stack proof.

Static scalar/Bytes representation differences are hypotheses for measurement.
No result, causal allocation attribution, performance improvement or architecture
change is claimed from this unexecuted proposal.

## Remaining obligations

This algorithm change introduces no new language family or capacity milestone.
Full Tier 2 Milestone B remains open for affine OwnedRecord/OwnedChoice and
deconstruction, general `List<T>`, owned witnesses and source-tied read views,
lexical Regions, generic/recursive nominal values, general specialization,
task/effectful callables and resource/owned captures.

General recursive-value lifecycle work, boxed-spine Clone admission, type/AST
and owned-storage escape paths, independent codec/layout bounds and default
execution stack policies remain open. Source and IR coordinate widths are
unchanged. Deep non-tail native CALL still grows the OS stack; heap-backed
activation and its evaluation-order, custody, cancellation and cleanup proofs
are not established. Native phase evidence remains scoped to C1/C2 processes;
C3 byte equality alone does not prove a separate C3 phase process.
