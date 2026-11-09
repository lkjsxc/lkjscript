# Native scalar allocation baseline

## Scope and source

The opt-in probe measures allocator requests for one publicly authored entry with
three runtime-selected paths: scalar arithmetic, a 17-byte Bytes slice, and an
ordinary identity call. Both native emitters consume the same normalized input.
The complete maintained raw-source carrier binds compiler context only; this is
Tier 1 emission, with no new raw-source generation or fixed-point claim.

The measurement is checkpointed on `measurement/20261109-native-scalar-costs`.
Its commit is `2f2c22e329d0fcedb51993f73f3158e0b743c9cb`, based on main's accepted
documentation commit `a7a58b2b`. Measurement tree:
`a5b5d475b8c7204bd836330add5190064e5071f5`.
Complete source snapshot SHA-256:
`18267a2cb46161825c775d4deb311a64e4431e70db58104f0c6e7e38659da6c1`.

At that checkpoint, this experimental test-only branch had passed the focused
checks below. Its original measurements retain that source and execution scope.
The probe was subsequently included in the separately accepted and integrated
[native raw-I64 implementation](20261109-native-i64-slots.md#acceptance-evidence-and-delivery).
The accepted [source-directory implementation](20261108-source-directory-merge.md#acceptance-evidence-and-delivery)
and its raw, full and release evidence remain separately attributed.

## Matched experiment

The fixture is `tests/fixtures/native-scalar-probe.lkjc`. Public authoring adopts
it into an in-memory GraphRepository before normalization. One image per emitter
handles both counts and all modes; mode, count, seed 4096 and step 1024 are runtime
input bytes. The result is independently computed as checked `seed + count*step`.

The explicit caller selected counts 4,096 and 8,192 and a 120-second deadline per
native process. Native address-space allowance is None. These are experiment
parameters; no production default quota or scalar-pool range changes.

Each of six inputs runs through the VM and reference interpreter, then both
native emitters. Each native process performs three invocations while preserving
earlier output. Totals: 12 native processes, 36 invocations and 132 observed frames.
Every invocation returned the exact expected eight-byte result: 4,198,400 or
8,392,704 as appropriate. Independent mapping/slab inspection, output survival,
callee-saved checks and joined cleanup passed; final owned mappings were zero.

## Measured allocator requests

Each table entry is the observed total for one invocation. All three invocation
samples independently produced that total.

| Backend and path | 4,096 iterations | 8,192 iterations |
| --- | ---: | ---: |
| Rust scalar | 12,297 | 24,585 |
| Rust Bytes slice | 16,393 | 32,777 |
| Rust ordinary identity call | 12,297 | 24,585 |
| Maintained scalar | 6 | 6 |
| Maintained Bytes slice | 4,102 | 8,198 |
| Maintained ordinary identity call | 6 | 6 |

The results agree with the source-derived predictions saved before execution.
For this fixture and counts above 255, Rust scalar/direct-call requests are
`3N+9`; Rust Bytes requests are `4N+9`. Maintained scalar/direct-call requests
are 6; maintained Bytes requests are `N+6`.

The Rust word read, result addition and cursor addition each box an I64 outside
the immutable -1..255 pool. The maintained backend transports those I64 values
as raw words. The slice adds one combined descriptor/payload request in either
backend. The ordinary identity call transports the existing value representation
and adds no allocator request here.

Both paths have six fixed requests for buffer construction, backing reservation,
freeze, structural result and output promotion. Rust additionally boxes the
entry count, seed and computed end. The agreement explains this fixture's request
traffic; it does not assign whole-compiler allocation shares.

### Observation meaning

A valid native invocation clears its 160-byte handle, including the request count
and saturation flag. Release preserves those observation fields; the next valid
invocation resets them. Samples at frames 0, 1 and 8 therefore represent three
independent invocations, not cumulative process traffic. All flags were clear.

Requests include scratch and output-promotion allocation. They are not allocated
bytes, mmap calls, copied bytes or peak scratch. Post-invocation mappings describe
surviving output after scratch reclamation. The ordinary-call path adds only one
transient call depth per iteration; it establishes no deep non-tail stack bound.

Control wall times include startup, three invocations, handshakes, observer map
walks and joining. No invocation-only timing, statistical speedup or performance
threshold is claimed.

## Focused verification and retained evidence

Release format and all-target/all-feature Clippy preflight passed. The exact
ignored measurement passed one test with no failures or ignored tests in its
selected run. The existing ordinary callable behavior/artifact test also passed
one selected test. Its C0, C1-first and C1-repeat baseline/empty-Bind pairs compare
all allocation observations, including the added output, diagnostic, saturation
and owned-mapping fields, for empty and 131,073-byte inputs.

Both runs bind the same complete source before/after and original Cargo-selected
release test image. Measurement compilation took 399.387 seconds; the probe took
54.270 seconds, including 51.846 seconds of authoring, compiler preparation,
emission and complete-carrier context export. The ordinary callable regression
took 55.919 seconds. Its Cargo preparation reused all 231 artifacts in 0.119
seconds. These are separately scoped observations, not paired compiler timings.

All paths below are under
`.artifacts/capacity-unbounding/nominal-flow-20261109/`.

| Record | SHA-256 |
| --- | --- |
| `native-scalar-measurement-1/receipt.json` | `58e781fd0e6f7b63fe6d4a9480eb521d538d114a86f054f638a9bb90eab79c0a` |
| `native-scalar-measurement-1/actual-session.json` | `b6376bbac654203ee2b59eccc283862a70be06e3fc104733a7f1debe9c0e47e7` |
| `native-scalar-measurement-analysis-1.json` | `c624be9baf71fb06f5d372f52c191186697a41861ce82d60527debf094206288` |
| `native-callable-regression-1/receipt.json` | `3b89d7f2c1380fccecd441b04fbfbc58721f701773f85bba72ca90693d3fe0ea` |
| `native-callable-regression-1/actual-session.json` | `4bfd917042e806a99cb37e18727642dfb697523fb1082efe8f12101bbb28d7c4` |
| `native-scalar-static-prediction-1.json` | `061383671e42da52fe0c768b884c904be318ad1f622008d87c9981a0324399ac` |
| `native-scalar-checkpoint-1.json` | `3112d7ae62188e0fbe9822a90be5e94cb11821e237b5a11b589b6adcec52deb3` |

The original test image is 215,560,008 bytes, SHA-256
`2663d4c84635bde2cc39eb4c69a14aa365ba19a5c8e2ea34261347db8a840f3c`.
The context record binds the complete 26,640,313-byte maintained carrier,
9,769-byte normalized input, 34,964-byte Rust image and 72,697-byte maintained
image. Full identities and actual stdout/stderr are retained in the records.
No gate executable was copied into the artifact directory.

## Structural dependency identified by the baseline

The following inventory describes the representation at the baseline. The
subsequent [native raw-I64 implementation](20261109-native-i64-slots.md)
provides the derived representation plan and adapters; its acceptance remains
separate from this measurement.

The native path retains exact function signatures and closed prepared types, but
NormalizedCode has no per-instruction local/operand type map. Canonical stack
admission proves depths; local liveness proves last use. Neither is a
representation proof.

The next complete implementation needs a derived native representation-flow
plan, using existing accepted types and control flow. It must cover:

- Branch joins, local redefinitions, variant payload destinations and the
  distinction between raw I64 zero and an empty or consumed local.
- Exact function/application context, direct arguments/results, callable
  signatures and capture/Invoke conversions at boundaries kept boxed.
- Optimized instructions as emitted, including arithmetic rewrites.
- Tail-frame ownership scans: `closures::owner_range` currently probes nonzero
  locals as value pointers. Raw scalar bits must never reach that dereference.
- Full signed-I64 meaning, overflow/trap order, affine custody, read-view
  provenance, evaluation order and cooperative cancellation, including checkpoints currently reached
  through allocation.

The plan is derived compiler data, not an editable meaning authority. Unproven
optimization paths should retain a coherent existing representation. A larger
finite scalar pool does not remove the per-value representation cost.

No native representation change, capacity closure or Tier 2 Milestone B
expansion is established by this measurement. Any subsequent compiler change
still requires its complete producer/consumer update, independent behavior and
negative controls, raw fixed-point/artifact equations and lifecycle acceptance.
