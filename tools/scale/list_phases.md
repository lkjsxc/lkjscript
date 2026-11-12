# Boxed-list phase probe

`list_phases.mjs` uses the outside kit's `Option<Box<N>>` list shape. The
generated program receives `World`, writes four phase markers through
`Stdout`, builds the list in a loop, sums it with non-tail recursion, checks the
sum, and explicitly drops the list. The ordinary compiler builds checked K and
WebAssembly. The probe uses the selected compiler's byte writer and worker
constructor; stack reservation follows the Node bridge's memory, cgroup, and
address-space observations.

```sh
node tools/scale/list_phases.mjs <compiler-binary> \
  --sizes 1250000,2500000,5000000,10000000,20000000,35000000 \
  --repeat 3 --out target/list-phases
node tools/scale/list_phases.mjs <compiler-binary> \
  --sizes 20000000,35000000 --repeat 1 --trace-gc \
  --out target/list-phases-gc
```

Use `--bridge <directory>` when the host bridge is not two directories above the
binary's containing directory. The default fits Cargo's `target/{debug,release}`
layout, including isolated validation worktrees.

Each sample runs in a fresh process and worker. There is one large worker at a
time. `samples.jsonl` contains phase observations, host memory availability,
resource reservations, and errors. `runs.jsonl` contains medians and ratios,
compiler/probe/source/module hashes, commands, and compiler-stage timings.
`--timeout-ms` is the child-process observation envelope, not a language limit.

A marker ends the previous phase and starts the next. Timing excludes the
marker's write and observation overhead. CPU/fault counters cover the whole Node
process, including concurrent V8 workers; they are not single-thread CPU time.
RSS, reserved address space, and WebAssembly's logical memory extent are separate
observations. An incomplete phase and its trap are recorded as a failure.

`--trace-gc` counts process GC events and their printed pause durations between
markers. Its timings are instrumented observations and remain separate from
untraced medians. Raw GC logs are unnecessary for routine use.

The retained baseline records in `records/list-9d3d1172.jsonl` explain the
outside kit's large-list anomaly. Construction, rather than recursive sum or
drop, accounts for the timing jump. Above two GiB of linear memory, that Node
version reports one external-memory-pressure full collection per additional
64 KiB page. The old allocator grows by the immediately required pages.
Phase traces, a V8 sampling profile, and timed native stacks identify the
collection path through `WasmMemoryObject::Grow`. The record includes exact
profiling commands. Breakpoint attempts that did not stop at a hot call site
are excluded.

The growing structural-drop stack has a separate memory cost: each active
frame uses 32 bytes, and released old capacities remain in the allocator's free
list. Its geometric capacity explains the stepwise increase in logical memory;
it does not account for the measured construction-time GC cliff.

The growth counterfactual is retained in `records/list-growth-after.jsonl`.
The compared modules differ only in the allocator function body. Geometric
growth reduces the observed construction GC cliff; the 20-to-35-million-node
whole-run ratio becomes 1.84 for 1.75 times the input. The larger logical memory
extent is reserved capacity: resident peak memory stays approximately unchanged.
A 50-million-node run constructs and sums the full list, then reports `R0006`
during explicit drop at Node's 16 GiB linear-memory extent. This is a failed
whole program despite its successful recursive traversal.
