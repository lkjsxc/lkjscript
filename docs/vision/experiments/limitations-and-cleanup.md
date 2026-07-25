# Experiment Registry: Limitations And Cleanup

[Authority](../experiments.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

### Limitations And Cleanup

This was one scalar/check-heavy kernel on one CPU, OS, and Cranelift version. It
did not test allocation, GC, exception/unwind behavior, structured language
outcomes, VM/native transitions, recursion, code-cache concurrency, cold-cache
control, multiple target CPUs, or a full typed SSA/ABI. Cranelift's register
spill may not generalize; the owned code was manually register-assigned and
therefore understates a general allocator's implementation cost. Execution
samples include loop/check/call work but exclude process startup and native
compilation. The build RSS method is not aggregate tree RSS. No benchmark
sample was discarded or tuned after observation.

After the compact record and hashes were prepared, the reproducible
`target/backend-spike-owned`, `target/backend-spike-cranelift`,
`target/backend-spike-harness`, and `target/backend-spike-tools` trees were
deleted. Only this documentation evidence remains. No temporary dependency,
lockfile, generated code, result JSON, or binary is retained.
## C4 Callable Scalar Baseline JIT: Adopted

- Status: **Adopted** for retained instrumentation, the scalar baseline tier,
  ordinary `auto`, and a 64-entry default. This does not adopt OSR, native
  references/allocation, recursion, host IO, or an optimizing tier.
- Question: does source-derived generated execution materially beat the
  same-commit VM after making compile/startup/memory costs visible, and which
  conservative function-entry threshold should ordinary runs use?
- Baseline/candidate: same-commit VM, forced, and auto used clean implementation
  commit `025cbb2feadbb18fbae51e68e38b9c849798d068`; the instrumentation/default
  landed in `56535c589998eeefa045fca622720662a2f78662` and time-to-first-entry in
  `025cbb2`. The diagnostic historical VM baseline was
  `c4c96094260072323f9399fe7f0f7b4a14d1eef6`.
- Predeclared completion target: exact oracle and stream silence, forced nonzero
  native entry with zero fallback/failure, at least four warmups and 31
  randomized samples, every sample retained, and aspirational generated
  execution at least 5x same-commit VM. Compilation, process wall, RSS, code,
  metadata, fallback, and break-even remain visible even if unfavorable.
- Workload: canonical `src/examples/jit-scalar`, 100,000 F64 loop iterations and
  direct calls, main SHA-256
  `aa8acecbad8add81f7a3a79b19a69e8f503d36c8af6e1f503b572bfadd14157e`.
  Independent Python IEEE evaluation required returned F64 bits
  `0x401af3ef5a48f5f0`; runtime output was not hard-coded.
- Environment: isolated `/tmp/pi-agent-278f3a26-771b-4bc-ddc2e7ec`, Linux
  7.0.0-27-generic x86-64/glibc 2.39, AMD Ryzen 9 9955HX, 20 logical CPUs
  available, 32 GiB RAM, Rust/Cargo 1.96.0, Python 3.12.3. Release binary
  SHA-256 was
  `94dec3b623f07333ed57659c67d8461c8ac30e7c13684f147700b72cefd9a638`,
  size 1,448,584 bytes. The harness SHA-256 was
  `c8005324d9389194b441988ad09582f50e54ea6b573c808881ed143d22b99c49`.
- Protocol: fixed seed `0x4c4b4a534d455452` (decimal
  5,497,569,491,774,952,530), one verified pre-timing process per variant, four
  randomized warmups per variant, then 31 randomized measured samples per
  variant. Python `monotonic_ns` surrounded process creation through wait;
  `/proc/<pid>/status` `VmRSS` was polled about every 0.5 ms. P95 is nearest
  rank. No sample was removed.

### Primary Result

Times are milliseconds; each cell is median / MAD / p95 / min / max. Process
wall includes process startup, source compilation, engine work, and shutdown.
Internal timers use Rust monotonic `Instant` directly.

| Variant | Process wall | Engine execution | Peak RSS KiB |
| --- | ---: | ---: | ---: |
| VM | 354.533038 / 4.711766 / 362.572659 / 347.360647 / 369.390164 | 352.918824 / 4.019214 / 361.367864 / 345.805002 / 367.389793 | 2,736 / 48 / 2,880 / 2,664 / 2,900 | <!-- LKJ-EXACT-DATA -->
| forced baseline | 9.372036 / 0.467328 / 10.364211 / 8.711153 / 10.472645 | 7.752251 / 0.065453 / 9.158604 / 7.680937 / 9.239046 | 2,724 / 52 / 2,812 / 2,572 / 2,860 | <!-- LKJ-EXACT-DATA -->
| auto, threshold 64 | 214.482019 / 3.352331 / 226.691819 / 206.949992 / 228.798658 | 212.908863 / 3.337173 / 225.098072 / 205.642925 / 227.326952 | 2,808 / 48 / 2,900 / 2,720 / 2,916 | <!-- LKJ-EXACT-DATA -->

Forced native execution after installation was 7.647935 / 0.068429 / 9.067563 /
7.570590 / 9.137916 ms versus VM execution 352.918413 / 4.019445 /
361.367684 / 345.804802 / 367.389623 ms. The median generated-execution
speedup is **46.1456x**, passing 5x. Forced process wall is 37.8288x faster than
VM; auto process wall is 1.6530x faster, a 39.50% reduction. These claims are
specific to this supported scalar workload.

Median source compiler total was 0.368733 ms VM, 0.370507 ms forced, and
0.371899 ms auto. The VM sample's phase medians were source loading 0.104205,
parse 0.022483, HIR analysis 0.046507, effect analysis 0.003045, SSA construction
0.019016, SSA verification 0.019266, normalization 0.076243, bytecode lowering
0.026790, and bytecode validation 0.035898 ms. Full distributions for every
phase and variant remain in JSON.

Forced native lowering/encoding median was 0.040096 ms and relocation/W^X
installation 0.036558 ms, 0.076654 ms combined. Time from JIT-session creation
to first native entry was 0.080141 ms. Its first call was the complete workload
and took 7.647935 ms. Against the 345.270478 ms median execution saved per
whole-workload invocation, compile/install breaks even in one repeated
invocation. Forced counts were one object, 100,001 native entries, 100,000
unboxed direct calls, 300,002 PollV1 calls, zero VM fallback, and zero compile
failure. The object retained 1,926 code bytes, 2,618 metadata bytes, 4,096
accounted mapping bytes, eight relocations, and eight scalar safepoints.

Auto compiled only `scalar-step`; `main` stayed VM, so this is not OSR. Median
time to first native entry was 0.297720 ms and first scalar native-call duration
0.000691 ms. Exactly 64 initial VM entries were recorded, followed by 99,936
native entries and PollV1 calls. There were zero compilation failures and no
unexpected fallback. Its one object retained 751 code bytes, 1,074 metadata
bytes, and a 4,096-byte accounted mapping.

### Threshold Decision

Threshold alternatives used the same clean commit, oracle, binary hash,
warmup/sample count, random seed, and complete three-variant protocol. Auto
results are:

| Threshold | Process wall median / MAD / p95 ms | Time to native entry median | Initial VM entries | Native entries |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 211.286082 / 1.961277 / 220.026771 | 0.081463 ms | 1 | 99,999 |
| **64** | 214.482019 / 3.352331 / 226.691819 | 0.297720 ms | 64 | 99,936 |
| 1,024 | 211.901028 / 1.875576 / 218.506574 | 3.556024 ms | 1,024 | 98,976 |

The process distributions overlap; no threshold is called fastest. Threshold 1
pays compilation on the first observed scalar call. Threshold 1,024 delays
native entry more than tenfold versus 64 without a retained process benefit.
The existing middle value 64 is therefore adopted conservatively: tiny calls
may finish in VM, while this repeated-call workload tiers promptly. One long
main invocation remains VM until OSR exists.

### Pre-JIT VM Diagnostic

The retained `meta/benchmarks/jit/pre-jit-workload` source is accepted by both
commits and checks the exact F64 bits in-program, exiting 1 on mismatch. This
works around the older CLI's lack of returned-value metrics without adding
benchmark output. Both binaries ran with empty stdout/stderr and status 0. Seed
`0xc4c9609426007232`, four randomized warmups, 31 randomized samples per
variant, and the same process/RSS method were used; no sample was removed.

| VM | Process wall median / MAD / p95 / min / max ms | RSS median / MAD / p95 / min / max KiB |
| --- | ---: | ---: |
| pre-JIT `c4c9609` | 364.419240 / 3.637667 / 370.862130 / 359.489554 / 371.563278 | 2,272 / 8 / 2,336 / 2,192 / 2,344 |
| current `025cbb2` | 357.510855 / 3.938523 / 376.733275 / 349.049752 / 401.131845 | 2,756 / 60 / 2,864 / 2,664 / 2,916 | <!-- LKJ-EXACT-DATA -->

The current/pre-JIT median ratio was 0.981. The 6.908 ms difference is below
twice the larger MAD (7.877 ms), so this diagnostic supports neither a VM
regression nor an improvement claim. Current RSS median was 484 KiB higher.
The old binary was 1,129,440 bytes, SHA-256
`b5e8be566cc72456e90534a4483d86bca6a1a3e11b58357cca10e90c4a0dafd5`;
the current binary was 319,144 bytes/28.26% larger. The binary and RSS costs are
retained negative evidence.

### Commands, Samples, Limitations, And Cleanup

Correctness preceded timing:

```sh
cargo run --locked -q -p lkjscript-xtask -- quiet verify
cargo build --workspace --release --locked
python3 meta/benchmarks/jit/benchmark.py
python3 meta/benchmarks/jit/benchmark.py --auto-threshold 1 \
  --output /tmp/jit-threshold-1.json
python3 meta/benchmarks/jit/benchmark.py --auto-threshold 1024 \
  --output /tmp/jit-threshold-1024.json
```

Every primary/threshold/pre-JIT sample is committed respectively in:

- `meta/benchmarks/jit/results/callable-baseline-jit-linux-x86_64.json`, SHA-256
  `ffdc3671857e44dfba2cbd2d1c287514a12c5a4c58a07124bfd99645c3bb86a7`;
- `meta/benchmarks/jit/results/auto-threshold-1.json`, SHA-256
  `51d5eccd1705248e9e3a761f9de7e413ef1853e6f7e9befc8f7a006d2db207b0`;
- `meta/benchmarks/jit/results/auto-threshold-1024.json`, SHA-256
  `f99d70bbf3af16d6fb83c1febd453a98620ae1feaa47078f33d14dbed50cdf25`;
- `meta/benchmarks/jit/results/pre-jit-c4-vm-comparison.json`, SHA-256
  `771d5206229f6452f96f5186d82353b80111b455f9e09c894c7c531206246884`.

For the old comparison, one Cargo artifact tree was used sequentially with a
clean between incompatible same-version worktrees; the old binary was copied
only for measurement. The detached `c4c9609` worktree, copied binary, temporary
source copy, and temporary JSON were removed. The current release artifact tree
was rebuilt afterward. The retained source hashes match the removed copy.

This result does not cover unsupported native Str/Symbol/list/buffer/handle,
product/Option/Result/reference/allocation/host paths, recursion, indirect
calls, OSR, background compilation, optimizing tiers, non-Linux targets, or
server behavior. The 5x target passed, so no post-failure profiling or
code-layout manipulation was performed. Docker and the 20-minute Brainfuck
Mandelbrot were not run. Full Brainfuck Mandelbrot remains a later OSR workload.
