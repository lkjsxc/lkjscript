# Native benchmarks

Run from the checkout after bootstrap:

```sh
sh bench/run.sh
```

The first row measures the complete compiler rebuilding itself. This is the
representative workload: small scalar kernels do not predict compiler speed.
The runner then builds each source and checks a small input before measuring its
default input. Every route must produce identical stdout bytes, empty stderr and
status zero. Timing is sequential and observational; no time is an acceptance limit.

| Program | Default workload | Observable result |
| --- | --- | --- |
| src/main.rs | Complete self-build with explicit current library/runtime | Compiler artifact |
| aggregate64.rs | 10 million 64-byte records passed/returned through three call levels | All eight fields in recurrence checksum |
| vector64.rs | 1 million inline 64-byte records, then 10 million reads | All eight fields in sum |
| aggregate.rs | 20 million calls taking/returning a two-word Copy struct | Recurrence checksum |
| scalar.rs | 200 million multiply/add/xor iterations | Final word |
| scalar_add.rs | 200 million add/xor/mask iterations | Final word |
| buffers.rs | 200,000 fresh Vec<i64> buffers of 1,000 elements | Indexed checksum |
| vector.rs | 5 million Vec<i64> pushes, then 100 million reads | Sum |

Counts and seeds are runtime arguments. Call-chain functions use inline(never),
which rustc honors and the plain native backend already implements. Vector timings
include construction. Buffer reuse and large vector remapping remain separate
workloads. Reports include elapsed/user/kernel time, peak RSS and executable size;
transient results and images stay beneath target/bench/.

Optional comparisons:

```sh
sh bench/run.sh --witness target/witness/release/lkjscript --rust
sh bench/run.sh --compiler target/lkjscript --previous target/previous-lkjscript
```

--witness names a Rust-built compiler of the same sources. Its self-build must
produce identical compiler bytes, and it also enables rustc optimization levels
0 and 3 for the small workloads. --rust enables those two workload routes alone.
They use the optional witness host adapter and checked arithmetic with aborting
panics. --previous names an earlier native compiler which can build these sources;
its compiler artifact may differ when emission changed. Default execution needs
neither Rust nor Node. GNU time, sh and ordinary file utilities provide measurement orchestration.
