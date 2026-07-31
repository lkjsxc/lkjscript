# Segmented List Capacity Evidence

[Memory authority](../authoritative-memory-plan.md) | [Structural domains](../structural-ownership-domains.md)

## Status

**Current selection evidence.** Capacity `32` is the selected production segment
capacity for the Current session-region copy-element list slice. Capacities `8`
and `128` are retained rejected alternatives. Flat key-free list snapshots and
pair-family removal are Current. Immutable structural element witnesses,
residual generic witnesses, and final collector deletion are not Current.

## Hypothesis And Adoption Criteria

One segment stores append-only entries `(element, tail-key, exact-list-type)`.
Existing tails remain immutable because every handle names one entry. The session
arena owns all segments and releases them in bulk, so there is no per-entry,
per-cons-node, atomic, `Rc`, or `Arc` ownership count. One process-wide atomic
issues nonreusable arena generations; it is identity allocation, not liveness.

The production candidate must satisfy all of:

1. exact semantics and failure-atomic bounds at capacities `8`, `32`, and `128`;
2. p95 time no more than 5% above the fastest measured candidate on the combined
   100,000-entry linear prepend/traverse/equality plus 20,000 retained-tail branch
   construction workload;
3. at most 2 KiB reserved entry storage for a representative eight-entry short
   list;
4. fewer segment allocations than capacity `8` on the combined workload.

Criterion 3 protects the common short-list case from a long-list-only selection.
The estimated one-segment storage is 344 bytes at capacity 8, 1,304 bytes at 32,
and 5,144 bytes at 128.

## Environment And Exact Command

Base commit: `d851d6b4224bfeb9ede3bcf9c7609562d91cbea0` plus the uncommitted candidate
implementation whose benchmark source SHA-256 was
`861670633ebd6b7177f7ffc9814192e10bced8a52e00d4f31b566d3964f45f83`.
The production arena source SHA-256 at measurement was
`9ad62bed5a26a9b1d11417c2716e7e382350d2a41e0c37bfd98fa0e30160f1ad`.

Environment: Linux x86-64 `7.0.0-27-generic`; Rust
`1.96.0 (ac68faa20 2026-05-25)`; LLVM `22.1.2`.

```sh
cargo run --locked --release -p lkjscript-core \
  --example segmented-list-candidates
```

The removed measurement driver ran 30 fresh-arena samples per candidate. Each
sample performed 100,000 linear prepends, full traversal, 20,000 branches from a
retained tail, and exact 100,000-element equality. `black_box` retained results.
The driver was removed after retaining these compact results.

## Results

| capacity | p50 ns | p95 ns | p99 ns | segments/allocations | estimated bytes |
|---:|---:|---:|---:|---:|---:|
| 8 | 2,377,880 | 2,661,542 | 3,131,315 | 15,000 | 5,160,000 |
| **32** | **2,148,839** | **2,453,502** | **2,704,764** | **3,750** | **4,890,000** |
| 128 | 2,089,548 | 2,420,831 | 2,503,004 | 938 | 4,825,072 |

Capacity 32 is 1.35% above the fastest p95 and uses 1,304 bytes for the
representative short list. Capacity 128 is fastest on the long workload but
fails the 2 KiB short-list bound. Capacity 8 passes the short-list bound but has
8.48% slower p95 than 128 and four times the segment allocations of 32.

## Source Audits

The pair-family decrement reran these exact source audits:

```sh
grep -RInE '(^|[^[:alnum:]_])(Rc|Arc)([^[:alnum:]_]|$)' \
  crates/lkjscript-core/src/structural/segmented_list \
  crates/lkjscript-ir/src/eval \
  crates/lkjscript-vm/src/run/data/lists.rs \
  crates/lkjscript-jit/src/heap/lists.rs \
  crates/lkjscript-jit/src/session/list_snapshot.rs
grep -RIn --include='*.rs' 'HeapObj::Pair' crates
```

Both searches produced no matches:

```text
NO_PER_NODE_RC=PASS
NO_HEAP_PAIR=PASS
```

The process-wide atomic arena generation remains identity allocation, not node
liveness. Flat snapshots use one owned node table with no runtime arena key.

## Selection

Adopt capacity `32`. Retain one implementation with a checked configurable
capacity in the core substrate so tests can cover rejected candidates; production
memory witnesses close the selected value `32`. Re-open selection only with a
new workload suite and explicit criteria. No rejected candidate remains as a
production fallback.
