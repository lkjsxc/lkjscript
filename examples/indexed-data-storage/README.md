# Generic indexed ordinary data in owned storage

`IndexedData<Self: Owned, Item: Transferable>` combines affine storage custody
with unrestricted immutable items. The library is an ordinary native request,
authored and exported before any storage implementation or concrete item exists.
It exposes `build`, `snapshot` and `exercise` algorithms through exact witnesses.

| Method | Inputs | Result |
| --- | --- | --- |
| `empty` | none | `Self` |
| `length` | borrowed `Self` | `I64` |
| `push` | unrestricted `Item`, consumed `Self` | `Self` |
| `get` | unrestricted `I64`, borrowed `Self` | `Item` |
| `replace` | unrestricted `I64` and `Item`, consumed `Self` | owned product `{rest: Self, value: Item}` |
| `discard` | consumed `Self` | `Unit` |

All methods are pure. `get` returns an ordinary immutable value whose validity
continues after replacement and destruction of the owner. `replace` returns the
displaced value along with renewed storage custody. A transferable owned resource
such as `ByteBuffer` does not qualify as an ordinary `Item`. Complete ordinary
admission also checks nested and unused nominal type arguments.

`flat.lkjc` maps these methods to the standard `data-sequence-*` functions.
`chunked.lkjc` defines public nominal `Chunk32<T>` with module-private `chunks`
and `size`. Every chunk except the last contains 32 items. Reads borrow the outer
owner and selected chunk. Replacement exchanges the selected chunk with an empty
placeholder, updates that chunk, restores it, and explicitly discards the returned
placeholder. No native mutable borrowing or runtime primitive is added. Indexed
operations contain no collection-prefix traversal or element-copy loop.

`forward.lkjc` exports `Forward<T, W>` with an exact prerequisite witness. Its
methods forward both ordinary and owned parameters. Nested forwarding preserves
the full implementation identity. `CaptureRead` and `Captured<T, W>` additionally
exercise the `capture-safe transferable` ordinary bound. `indexed-flat::Reverse`
is an identity probe with reversed reads on the same owner type; it demonstrates
that witness selection is semantic, and is not a conventional indexed-storage
ordering law.

Author `library.lkjc` into a fresh minimal project against the exact builtin
standard and export its transport. Author `flat.lkjc`, `chunked.lkjc` and
`forward.lkjc` into three separate projects, each importing the exact `indexed-data`
package and standard selections. Export and stage those transports into a fresh
command project. Bind exact package IDs and package revisions in `use` declarations
for `indexed-data`, `indexed-flat`, `indexed-chunked` and `indexed-forward`, then
author `application.lkjc` and `costs.lkjc`. Each file is a separate `change plan`
and `change apply` request with the current base revision and returned plan token.
Run `check`, require canonical drafts to plan unchanged, and build
`indexed-data.lkja` beside the deployment descriptors.

The public tests use copied executables outside the checkout. They retain complete
independent expected results for integers, booleans, nested immutable values and
nominal ordinary records. They compare all snapshots and displaced values after
explicit owner destruction, including chunk boundaries, nested forwarding and
same-type witness identity. Artifact bytes are deterministic and execution remains
available after all disposable source projects, literal requests and transports
are deleted. Rejections preserve accepted meaning; traps and finite execution
refusals expose no successful partial result and permit a following healthy run.

`indexed-item-flat` and `indexed-item-chunked` retain a complete displaced `Item`
containing nested lists, records and maps, explicitly discard storage, then return
that item and observations of its children. `indexed-item-nominal` consumes a
data-only nominal owner with several ordinary fields and reconstructs the complete
item after consuming the parent. Authored pack/unpack field order differs from
layout order. These are maintained consumers of
[checked extraction](../../docs/decisions/checked-owned-extraction.md), with
independent full-result expectations and malformed nested input coverage.

The reference runner has a fixed cumulative 256 MiB allocation budget. The tests
compare all six witness results differentially through 65 items, then compare
complete Flat and Chunk32 results separately at 257 and 1024 items. Detached
production keeps the complete six-result aggregate at every size, with the same
independent snapshot and displaced-value oracle. An observed aggregate
reference-budget refusal and following healthy runs are retained in development
evidence; the suite does not require that incidental capacity ceiling to persist.
All matched cost workloads remain differential at their full sizes.

## Matched costs

`indexed-cost-direct`, `indexed-cost-flat` and `indexed-cost-chunked` take
`[INPUT_I64_LIST, UPDATE_COUNT]` and return `{checksum, length}`. All three build
the same authored input, replace positions in round-robin order, read each new
value, sum displaced and new values, then discard storage. `direct` calls standard
functions directly; the other lanes invoke exactly selected storage witnesses.

Use count zero to observe growth and cleanup without indexed updates; use an empty
input for fixed preparation/invocation/cleanup costs. Compare equal nonempty inputs
and update counts to observe indexed updates, retaining all slower results. Read
preparation and invocation observations separately; process startup, input admission,
allocation and output encoding are separate costs. Modeled charges are not RSS,
allocator traffic or API billing. These fixtures establish workloads, not a speedup.

Set `LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE=1` for public tests to retain copied
executables, artifacts, descriptors, argument files and execution observations.
The dependency planner adopts independent integer and Boolean witnesses in all
four flat/chunked combinations; its complete result is still checked by the
separately authored [native verifier](../dependency-plan/verification/README.md).

### Observed results, 2026-10-11

The matched batch retained **480 executions across 20 workloads**: one warmup
and seven measured samples per lane, rotating all six lane orders. Empty inputs
requested zero or 64 updates; nonempty lengths 1, 31, 32, 33, 128 and 512 used
zero, one full round or four rounds of replacement. Every complete result matched
the independent oracle; cleanup joined and executable, artifact and descriptor
hashes remained unchanged.
The [summary](../../.artifacts/mixed-storage/costs/measurement-first/summary.json),
[all samples](../../.artifacts/mixed-storage/costs/measurement-first/samples.jsonl)
and [identity](../../.artifacts/mixed-storage/costs/measurement-first/identity.json)
bind one copied development executable and one source-deleted consumer artifact.
The executable uses optimization level 2 with debug assertions and overflow
checks; these observations do not attest finalized distribution bytes.

For nonempty indexed-update workloads, median invocation ratios were
**Flat/direct 1.111–1.241** and **Chunk32/Flat 1.364–3.392**. Growth-only
Chunk32/Flat ratios were **1.157–3.765**. All cases, including empty workloads
and unfavorable results, remain in the summary. At 512 items and 2,048 updates:

| Lane | Median invocation, ms | Modeled allocated bytes | Allocation charges |
| --- | ---: | ---: | ---: |
| Direct | 20.449 | 14,243,933 | 136,250 |
| Flat | 23.929 | 14,537,797 | 141,383 |
| Chunk32 | 78.899 | 40,271,093 | 440,914 |

Modeled counters repeated exactly within each workload/lane. All lanes admitted
the same complete artifact, charging 3,848,059 metadata bytes and 96,584 type
derivation steps. Across the batch, preparation medians were 127.856–140.162 ms
and process-wall medians 144.605–233.318 ms; invocation, preparation and result
encoding are retained separately. This was one **shared-host** batch with
unrelated workloads preserved. The [42 host observations](../../.artifacts/mixed-storage/costs/measurement-first/host-observations.jsonl)
record one-minute loads of 3.37–4.19, available memory of 53.89–54.07 GiB and
about 233.12 GiB free disk. They establish no dedicated CPU, portable speedup,
allocator traffic, RSS reduction or external API billing result.
