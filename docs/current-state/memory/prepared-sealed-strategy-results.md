# Prepared Sealed-Value Strategy Results

## Status

**Current evidence for the accepted revision-17 prepared sealed-value vertical.**
The broader polymorphic value plane and source-visible sealed values remain
Experimental or Accepted Targets as their authorities state.

## Scope

This revision-17 evidence compares detached deep clone, coarse sealed sharing,
one-node domains, private non-atomic per-node reference counting, and eligible
unique fusion. Candidate implementations outside coarse ownership and unique
fusion exist only in `sealed_strategy_comparison`; no per-node ownership code is
in production. The real structural runtime separately proves equal coarse
release-planning work for 8-node and 2,048-node images.

Environment: Linux x86-64, `rustc 1.96.0 (ac68faa20 2026-05-25)`.

Command:

```sh
CARGO_TARGET_DIR=/home/lkjsxc/workspace/lkjscript/target/prepared-sealed-release \
  cargo test --locked --release -p lkjscript-core \
  --test sealed_strategy_comparison -- --nocapture
```

Result: **passed**. Each p99 is nanoseconds over 257 measured iterations after
eight warmups. Bytes
are normalized logical bytes for 2,048 24-byte nodes. `ops` is coarse owner
operations; `node-ops` is per-node ownership/domain work. Atomics were zero in
every candidate.

| workload | strategy | alloc | live bytes | copied | ops | node-ops | release | p99 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| two-owner | detached-clone | 2 | 98304 | 98304 | 0 | 0 | 4096 | 901 |
| two-owner | coarse-sealed | 1 | 49152 | 0 | 3 | 0 | 2 | 501 |
| two-owner | one-node-domains | 2048 | 49152 | 0 | 0 | 8192 | 2048 | 561 |
| two-owner | private-per-node-rc | 2048 | 57344 | 0 | 0 | 6144 | 2048 | 792 |
| four-owner | detached-clone | 4 | 196608 | 196608 | 0 | 0 | 8192 | 1533 |
| four-owner | coarse-sealed | 1 | 49152 | 0 | 7 | 0 | 4 | 421 |
| four-owner | one-node-domains | 2048 | 49152 | 0 | 0 | 16384 | 2048 | 661 |
| four-owner | private-per-node-rc | 2048 | 57344 | 0 | 0 | 14336 | 2048 | 1523 |
| branch | detached-clone | 2 | 98304 | 98304 | 0 | 0 | 4096 | 882 |
| branch | coarse-sealed | 1 | 49152 | 0 | 3 | 0 | 2 | 401 |
| branch | one-node-domains | 2048 | 49152 | 0 | 0 | 8192 | 2048 | 531 |
| branch | private-per-node-rc | 2048 | 57344 | 0 | 0 | 6144 | 2048 | 772 |
| process | detached-clone | 2 | 98304 | 196608 | 0 | 0 | 4096 | 1112 |
| process | coarse-sealed | 1 | 49152 | 98304 | 3 | 0 | 2 | 401 |
| process | one-node-domains | 2048 | 49152 | 98304 | 0 | 8192 | 2048 | 531 |
| process | private-per-node-rc | 2048 | 57344 | 98304 | 0 | 6144 | 2048 | 772 |
| single | detached-clone | 1 | 49152 | 49152 | 0 | 0 | 2048 | 542 |
| single | coarse-sealed | 1 | 49152 | 0 | 1 | 0 | 1 | 411 |
| single | one-node-domains | 2048 | 49152 | 0 | 0 | 4096 | 2048 | 531 |
| single | private-per-node-rc | 2048 | 57344 | 0 | 0 | 2048 | 2048 | 852 |
| single | unique-fusion | 0 | 49152 | 0 | 0 | 0 | 1 | 401 |
| borrow | detached-clone | 1 | 49152 | 49152 | 0 | 0 | 2048 | 651 |
| borrow | coarse-sealed | 1 | 49152 | 0 | 1 | 0 | 1 | 401 |
| borrow | one-node-domains | 2048 | 49152 | 0 | 0 | 4096 | 2048 | 531 |
| borrow | private-per-node-rc | 2048 | 57344 | 0 | 0 | 2048 | 2048 | 601 |
| borrow | unique-fusion | 0 | 49152 | 0 | 0 | 0 | 1 | 401 |
| move | detached-clone | 1 | 49152 | 49152 | 0 | 0 | 2048 | 542 |
| move | coarse-sealed | 1 | 49152 | 0 | 1 | 0 | 1 | 401 |
| move | one-node-domains | 2048 | 49152 | 0 | 0 | 4096 | 2048 | 531 |
| move | private-per-node-rc | 2048 | 57344 | 0 | 0 | 2048 | 2048 | 592 |
| move | unique-fusion | 0 | 49152 | 0 | 0 | 0 | 1 | 401 |

## Decision

Coarse sealed sharing passes the predeclared p99, live-byte, and copied-byte
gates for every multi-owner workload and uses release work proportional to
owners, not nodes. Detached clone loses multi-owner byte/copy gates. One-node
domains and private per-node RC retain 2,048 allocations and node-proportional
release work; per-node RC is rejected because a valid non-node candidate passes.
Unique fusion wins single-owner, borrow, and move operation/allocation counts
and therefore remains ahead of sealing in placement precedence. Process codec
copy cost remains explicit and is not misreported as in-process publication.
