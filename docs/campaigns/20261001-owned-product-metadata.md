# Owned-product metadata reads — 2026-10-01

## Mandate

Original user instruction, unchanged:

> [https://github.com/lkjsxc/lkjscript](https://github.com/lkjsxc/lkjscript) について進めるようよろしくお願いします。必要であれば大幅なスタイルの変更も許容します。あらゆる判断において、過去ではなく、今のあなたに委ねます。全ての判断をあらかじめ許可します。最も筋の良い方向に進むために、限界まで深く考えてほしい。超長期的な視点からお願いしたい。どれだけ時間がかかっても構いません。

Continuation: `Continue`.

## Selection and semantic boundary

Complete the saved ordinary-metadata prototype before extending owned outcomes.
A policy helper should inspect a packet's ordinary tag without consuming and
repacking its payload. This is useful synchronous composition, not a new memory
carrier, a general reference type, or a substitute for recoverable owned outcomes.

`(field (local packet) (name tag))` creates a short whole-product read loan and
returns closed ordinary data. The local can own the product or be a borrowed
parameter; reborrowing is synchronous. The parent remains available after the
read, and the ordinary result can survive parent consumption. Owned children,
partial moves, temporary-product projections and mutable/escaping loans remain
unsupported. Ordinary records keep their existing projection behavior.

The independent ownership oracle retains rights rather than deriving its answer
from the production flow state. The VM resolves canonical field order by binary
search; the reference interpreter resolves the exact selected field independently.
Both check invocation origin, live read mode, ordinary type and returned value.
The storage owner reserves an inline sum/option box spine before copying.
Immutable backing remains shared; there is no copied owned payload or global
zero-copy, RSS, allocation-free, or throughput claim.

Graph 19 and native request layouts remain unchanged: previously valid source
meaning is not reinterpreted. Semantic validator 20 / owned-product feature 2
admit the extension; compiler 17, bytecode 13 and artifact 24 select changed
lowering/execution contracts. Old derived artifacts must be rebuilt from accepted
source. Product identifier 0.1.63 has no stability or compatibility semantics.

## Resumption and preservation

Resumption observed `ff7cdce3e31ad4d92b372c8a685c67979bcc528f` at main,
origin/main and the existing detached worktree
`/home/coder/workspace/lkjscript-owned-generics-20260930` in
`lkjsxc/tomato-ocelot-73`. The saved prototype had 22 modified tracked files and
two new literal fixture files. No compiler/checker job was running.
Reuse that lineage rather than replacing it. The main checkout's two unrelated
untracked files, earlier stash, other worktrees and running services are retained.

The copied pre-change optimized executable reports 0.1.62 and is retained at
`.artifacts/20261001-owned-product-metadata/predecessor-0.1.62`.
This is a host-source control, not a finalized distribution or a new public release.

## Observed attempts

`focused-resume.log` records the first resumed owned-product run: 24 passed,
two failed. Metadata/reborrow, retained result, token, quota and cancellation
checks passed. The two failures reached the test-only hostile-container writer,
which did not yet support artifact 24 (`unexpected forged-artifact generation 24`).
Extend that writer's explicit generation inventory; do not weaken artifact
admission or remove the two tests. The complete rerun also includes a new
consistently rehashed bytecode control changing the metadata loan to copy or
consume, and a non-local temporary-projection rejection.

Logs, generated fixture candidates and copied executables belong under
`.artifacts/20261001-owned-product-metadata/`. They are not tracked narrative.
Subsequent observations below bind their actual source rather than relabelling
earlier failed or incomplete attempts.

## First frozen-source full run and correction

The isolated implementation was committed as
`fc11fcfaf41ca1a6ee85793b6f58330d9fa69b1a`, tree
`a0f487510e203ea5e83c6b399d59377eef994c14`. It includes the generated native
artifacts, four new native first-use packs and five current-envelope fixtures.
`full-01.log` records a completed fresh full run: 24 of 26 gates passed, zero
reuse. Receipt `.artifacts/lkjscript-dev/check/1790856817189373973-835869-0/receipt.json`
(68,789 bytes, `verification_61af51688fa411be5042457c04a0820762fa097bb0e4809d4c3eeb6cd1f834d7`)
remains failed. The workspace library ran 1,032 passing tests, four failing tests
and eight existing ignored cases; later workspace test executables were not reached.

Three failing assertions still expected artifact 23 rather than the deliberately
selected artifact 24. The fourth still expected structural-owned-product feature
revision 1 instead of 2. Correct those explicit expectations, not the historical
fixture inputs or the new semantic contracts. The second failed gate was service
acceptance: its independently pinned lkjournal artifact SHA-256 was still the prior
artifact's digest. This same run had already rebuilt and byte-compared both maintained
standard and lkjournal artifacts successfully. Bind service acceptance and its current
README to the regenerated 1,375,404-byte lkjournal artifact:
`40c7c2760e2cfe38bd07a0e733d0193391c53f83afbfbf70327281756e9a9601`.
Historical source and release records retain the earlier digest. This pin change
is not itself proof of service behavior; the corrected full run must execute it.

The focused correction run passed all 29 `f64_` cases, including the three prior
failures. The witness control then reached its next obsolete expectation: the exact
validator digest still named the predecessor feature inventory. Preserve that
predecessor digest as an explicit non-reuse assertion and bind the new exact digest,
while retaining the feature-by-feature checks. `witness-corrected.log` records this
intermediate failure. Full source acceptance must be renewed after these corrections.

## Accepted source and independent public witness

Correction source `c2c50d3adc4ecd17126f0decfac9e0022fe68ad0`, tree
`fe8d73bbc4db5d8570f0c6104bfa5589db66a229`, completed fresh full-source acceptance
on 2026-10-01 at 22:18:36 JST. `full-02.log` records 26/26 freshly passed gates,
zero reuse and no failure. Original receipt
`.artifacts/lkjscript-dev/check/1790859877476285620-930581-0/receipt.json`
is 65,356 bytes with digest
`verification_affaade129f27ecaafadfac1c5041a5ef985d124eb162b426240b8fb90c90f5b`.
The receipt binds that exact source and equal initial/final worktree digests
`verification_41bfb4421918e26d661224e245bb178c620afa20864321fbe2d85ea2ae542374`,
with `input_stable=true`. The run used pinned Rust/Cargo 1.98.0, Linux x86-64,
checker jobs 2 and Cargo jobs 4. Its environment was explicitly limited to HOME,
PATH, LANG, CARGO_NET_OFFLINE and CARGO_BUILD_JOBS.

Workspace top-level tests passed 1,475, failed none and retained 29 pre-existing
ignored cases. The constituent passing totals are root library 1,036, data-key
integration 10, data-pagination integration 7, service integration 9, public CLI
184, structural ingress 12, contributor library 208, site library 8 and site CLI 1.
Nested subprocess controls are not counted a second time. The full run also passed
Clippy, generated guides, no-Python/product-surface gates, package artifact comparisons,
offline packages, pure tail, distributed/outbound/stateful HTTP and standalone service/data
acceptance. In particular the corrected service pin led to actual successful
service behavior, not merely a changed expected hash.

The final release-command-lifecycle producer supplied the copied host executable
`.artifacts/20261001-owned-product-metadata/host-c2c50d3a`, 28,375,080 bytes,
SHA-256 `729297ee7049f6d21e26e2cb5fbb9fe4089be3c46feb611dbf510f2804fb061c`.
Its bytes differ from the earlier `candidate-0.1.63`, so earlier binary-bound public
results were not substituted for final-host proof. `public-final-host.log` records
all 19 cases selected by `native_owned_`, `native_byte_buffer_`, `native_byte_ranges_`
and `resident_policy` freshly passing from `/tmp`, with an empty environment/PATH
apart from `LKJSCRIPT_RELEASE_CANDIDATE`. The exact frozen test executable is retained
as `metadata-public-tests`; the earlier 19-case `public-owned-composition.log` binds
its explicitly different prototype executable. Neither is a finalized musl archive.

An independent literal program and its public plan remain in
`/tmp/lkjscript-metadata-independent-20261001.lDray3`. It packs a ByteBuffer with a
structural metadata record containing Text and I64. A generic borrowed helper
returns the metadata; the parent is then unpacked, its buffer frozen, and the
retained metadata used after consumption. Project execution at 73 returns exact
payload byte 128, label `retained metadata` and sequence 73, with equal production
and reference results. Canonical drafting re-enters unchanged without owner recreation.
The source project is moved to `source-retained`, not deleted. Final-host detached
execution at I64 minimum, zero and I64 maximum preserves each exact sequence, label
and byte; each result is byte-compared with the earlier 0.1.63 output, which
was independently checked against the literal program's expected values. Literal
inputs, plan, artifact, deployment, output files and both copied executables remain
at that location. A separately authored type-correct attempt to
return an Owned child from a borrowed product rejects with `kernel_buffer_ownership`
and leaves the original accepted revision
`rev_c83d62ad45fb0d857ff1cc69a2e79580c5929da12b8fc44d96dde5b6153d57fd` unchanged.

The existing `cold-source.log` records the independent tracked-source copy at
`/tmp/lkjscript-owned-metadata-20261001.srnjQf/source`: all 144 pack path/content
entries and four accepted HEADs stay unchanged after public check/build, all 274
native tests pass with equal reference results, and all four generated artifacts
match maintained bytes. The correction commit changes none of those HEAD, pack,
program or generated artifact inputs; the application README's pin is presentation.
This input-inventory proof is reused with that limited scope, not claimed as a
second fresh full-source run. The final copied host independently verifies all
eight generated pages (`final-guides.log` in the independent witness directory).

## Mainline and distribution boundary

The next reporting descendant changes documentation and candidate selection only;
it does not replace the accepted source or relabel failed receipts. The source
implementation and required generated assets are complete. Mainline delivery and
candidate dispatch are recorded below after their actual operations.

Select consolidated 0.1.63 rather than restart the frozen failed 0.1.61 producer.
The native input defect is corrected and the successor includes the intervening
numbering and metadata work. Inspecting remote runs found no healthy pending producer;
exact 0.1.63 release lookup returned 404, and its remote tag was absent. The normal
read-only candidate workflow must still accept its own event source and finalized
archive. Before any promotion, its exact extracted executable must pass the
19-case supplementary gate in [release policy](../release.md#selected-consolidated-0163-successor).
Public/latest remains immutable v0.1.60. No running service, operational data,
credentials, permissions or existing publication identity is changed.
