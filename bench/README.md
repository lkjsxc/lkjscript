# Native benchmarks

Run from the checkout:

```sh
node bench/run.mjs
```

The runner compiles the same sources with the current lkjscript, the preserved
previous compiler when explicitly supplied, and rustc at optimization levels 0 and 3.
It runs each executable once, sequentially, checks identical output bytes and
exit status, and prints elapsed/user/kernel time, peak RSS, and ELF size. Results and
executables stay beneath target/bench/. A previous invocation's current-native
timing is shown when available.

| Program | Default workload | Observable result |
| --- | --- | --- |
| aggregate.rs | 20 million calls taking/returning a two-word Copy struct | Recurrence checksum |
| scalar.rs | 200 million multiply/add/xor iterations | Final word |
| scalar_add.rs | 200 million add/xor/mask iterations | Final word |
| buffers.rs | 200,000 fresh Vec<i64> buffers of 1,000 elements | Indexed checksum |
| vector.rs | 5 million Vec<i64> pushes, then 100 million indexed reads | Sum |

Counts and seeds come from process arguments. The scalar recurrence has a
loop-carried dependency. The aggregate step is marked inline(never), which rustc
honors and lkjscript's plain backend already implements. All routes consume the
same argument bytes. The runner also checks a small workload before timing. Source inputs are
bench/*.rs; each native compiler supplies its own embedded source library and
sealed runtime. Rust uses bridge/output.rs to grant the same World inputs.
Every route must produce identical complete stdout, empty stderr and status zero.

These programs retain the review's operation counts, but use explicit recurrence
work and runtime inputs; their timings are new measurements, not a replay of the
review's unavailable source programs. Vector timing includes its initial pushes.
The Rust builds enable overflow checks and aborting panics.

The repeated-buffer program exercises reuse separately from large vector growth.
Large vector growth exercises dedicated mapping remaps; the timing report includes
kernel CPU time so repeated mapping costs remain visible.

Optional paths:

```sh
node bench/run.mjs --compiler target/release/lkjscript \
  --previous target/previous-lkjscript --out target/bench
```

Node and the rustc host wrapper are temporary witnesses while repository tools
move into lkjscript.
