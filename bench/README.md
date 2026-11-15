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

## Checked-work reuse

Measure reuse with the current compiler or an explicit executable:

```sh
sh bench/reuse.sh
sh bench/reuse.sh --compiler target/lkjscript
```

The script copies `src/` and `library/` into a private snapshot beneath
`target/reuse-bench.*`. Every build uses that snapshot as its source Directory
and names its library and runtime explicitly. Preparation happens before timing;
the working tree is never edited. Five builds run sequentially:

| Row | Inputs and store |
| --- | --- |
| `no-store` | Original snapshot, reuse disabled |
| `cold` | Original snapshot, initially empty store |
| `warm` | Original snapshot, store populated by the cold build |
| `checker-edit` | One checker function body changed, populated store |
| `checker-edit-no-store` | The same changed snapshot, reuse disabled |

The edit changes the first diagnostic column in `check::error` from 1 to 2
without changing source line numbers. The warm row can reuse the complete image;
the edited row measures rebuilding after one body changes, with the compiler's
`--stats` counters showing reused checker verdicts and native fragments.
Both unchanged cached images must match the original no-store image byte for
byte. The changed cached image must match its changed no-store control and
differ from the original image.

Each row reports elapsed seconds, peak RSS in KiB, store bytes including directory
metadata, and the 1-, 5- and 15-minute load averages before and after the build.
Cold overhead is `100 * (cold elapsed / no-store elapsed - 1)` percent. Cold
means an empty lkjscript store; filesystem caches remain in their ordinary state.
Compare timings together with their loads on a shared host.

A fresh private 32-byte key is created through `tools/store-key.sh`, outside
the snapshot source/library grants and the store. The snapshot, key, store,
images and logs are removed on exit. Measurements are printed to stdout; the
script keeps no measurement ledger.
