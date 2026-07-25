# Experiment Registry: C6 Forced Proof-Optimizing Performance Gate: Adopted After Rejection

[Authority](../experiments.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## C6 Forced Proof-Optimizing Performance Gate: Adopted After Rejection

- Status: **Adopted** for forced first-tier performance at `cc967ff`, after the
  retained `063668e` run was **Rejected** by the scalar native sentinel. The
  forced proof-optimizing correctness slice remains Current; automatic
  promotion remains disabled and unmeasured.
- Question: does the first proof pipeline beat same-commit forced baseline by
  at least 1.20x beyond noise without hiding a greater-than-5% scalar sentinel
  regression?
- Baseline/candidate: the adopted rerun used implementation commit
  `cc967ff7e6f57a3225ae974d64ced6039ed8e9ae` and one clean locked release
  binary. The prior rejected run used
  `063668e08b92a97a2feae8397ff0d634887bd0b6`. Both compare their same-commit
  forced tiers and scalar sentinel with retained callable baseline commit
  `025cbb2feadbb18fbae51e68e38b9c849798d068`.
- Environment: both runs record Linux 7.0.0-27-generic x86-64/glibc 2.39, AMD
  Ryzen 9 9955HX, 20 logical CPUs available, 32 GiB RAM, Rust/Cargo 1.96.0,
  Git 2.43.0, and Python 3.12.3. The adopted 2,131,792-byte binary SHA-256 was
  `7116365712455ef43c180ed84d69f2e521b0da3783074c7324bf6cd7273955b7`.
- Build and command: the standard-library harness ran `cargo build --locked
  --workspace --release` itself, then `python3
  meta/benchmarks/jit/benchmark.py`. Normal VM/baseline/optimizing streams were
  empty. Timed runs opted into one `LKJSCRIPT_METRICS` JSON line.
- Oracle: a separate reference-VM process returned exact I64 `3333` for
  `src/examples/jit-optimizing`; Python independently computed scalar F64 bits
  `0x401af3ef5a48f5f0`; the allocation graph returned exact I64 `1`.
- Protocol: fixed seed `0x4c4b4a534f505449`, four warmups and 31 measured
  samples for optimizing-workload baseline, optimizing-workload optimizing,
  and scalar baseline in one deterministic randomized interleaving. Python
  monotonic nanoseconds covered process creation through collection; Linux
  `VmRSS` was polled about every 0.5 ms. P95 is nearest-rank, MAD is median
  absolute deviation, and no sample was removed.
- Predeclared adoption: at least 1.20x optimizing native speedup; median
  improvement greater than twice combined MAD, where combined is the sum of
  tier MADs; exact outcomes; nonzero optimizing entries and checked proof;
  zero optimizing-sample baseline entries/fallback; baseline tier integrity;
  W^X; allocation correctness; and both scalar native and process medians no
  more than 5% over the retained callable baseline. Every criterion was
  mandatory.

### Prior Rejected Run At `063668e`

Times below are median / MAD / nearest-rank p95 / minimum / maximum for the
first run.

| Case | Native execution ms | Process wall ms | Peak RSS KiB |
| --- | ---: | ---: | ---: |
| optimizing workload, baseline | 1.997375 / 0.016721 / 2.364345 / 1.972247 / 2.467920 | 3.584969 / 0.049302 / 4.415831 / 3.508204 / 4.879183 | 4,128 / 44 / 4,228 / 3,988 / 4,240 | <!-- LKJ-EXACT-DATA -->
| optimizing workload, optimizing | 0.681521 / 0.003567 / 0.843666 / 0.674839 / 0.978520 | 2.457400 / 0.053521 / 3.143670 / 2.391696 / 3.199314 | 4,048 / 28 / 4,136 / 3,908 / 4,148 | <!-- LKJ-EXACT-DATA -->
| scalar workload, baseline | 8.182742 / 0.044654 / 8.915169 / 8.073577 / 9.839367 | 9.340049 / 0.168027 / 11.450636 / 9.127458 / 12.328947 | 3,712 / 36 / 3,780 / 3,552 / 3,796 | <!-- LKJ-EXACT-DATA -->

The optimizer's native median was 2.930761x faster. Its 1.315854 ms improvement
was greater than twice combined MAD, 0.040576 ms. Optimization,
lowering/encoding, and install medians were 0.254879, 0.037590, and 0.047309
ms; baseline lowering and install medians were 0.083547 and 0.061566 ms. Exact
facts were invariant across every warmup and sample: baseline emitted 13,956
code and 16,489 metadata bytes with 10,001 baseline entries; optimizing emitted
2,724 code and 3,817 metadata bytes with 10,001 optimizing entries, 72
checked-I64 GVN records, 2,816 estimated certificate bytes, 35 executed
optimizing passes, zero baseline entries/fallback, and verified W^X.

The scalar native median was 8.182742 ms versus retained 7.647935 ms, ratio
1.069928: **failed** the 1.05 ceiling. Scalar process wall was 9.340049 versus
9.372036 ms, ratio 0.996587: passed. The source SHA-256 remained
`aa8acecbad8add81f7a3a79b19a69e8f503d36c8af6e1f503b572bfadd14157e`,
but compiler, metrics, native ABI, stack checks, binary, and surrounding
emitted code evolved across commits. This sentinel establishes a visible
regression against the retained measurement; it does not attribute that
regression to optimizing passes or compare algorithm-equivalent candidate
engines.

The one untimed allocation check reported 3 optimizing entries, 7 allocations,
259 estimated allocation bytes, 6 collections, 225 peak estimated live bytes,
maximum 3 roots, 14 attempted/14 successful heap calls, 6 barriers, zero
baseline entries/fallback, and verified W^X. All of its correctness/accounting
gates passed.

The first run's all-gates verdict was therefore **Rejected** despite the strong
local optimizer result. Its repository record also truthfully lists dirty
benchmark README, harness, and Python-cache paths. Every warmup, sample, order,
environment/tool record,
source/binary hash, full metric, and distribution remains retained under schema
`lkjscript.optimizing-jit-benchmark.v1` in
[rejected result][rejected-result],
SHA-256
`3e4341ffab5c0cbd976b3dc228d24dfdd8ff135247b91caafb74f0a571e71cec`.

### Adopted Recovery Run At `cc967ff`

The identical predeclared protocol then retained a clean run after generated
function entry polling was folded into ABI-2 frame registration. This removed
a separate runtime transition without reducing mandatory polls or changing the
optimizer proof pipeline.

| Case | Native execution median / MAD ns | Process wall median ns |
| --- | ---: | ---: |
| optimizing workload, baseline | 1,999,889 / 10,469 | 3,565,363 |
| optimizing workload, optimizing | 670,029 / 2,174 | 2,411,023 |
| scalar workload, baseline | 7,982,586 / 54,212 | 9,207,038 |

The optimizing native speedup was 2.984780x; the exact 1,329,860 ns improvement
exceeded the 25,286 ns twice-combined-MAD threshold. Process wall improved
1.478776x. All exact outcomes and silent-stream checks, tier entry/fallback,
proof, W^X, allocation, and baseline integrity gates passed. Scalar native was
7,982,586 versus retained 7,647,935 ns, ratio 1.043757; scalar process wall was
9,207,038 versus 9,372,036 ns, ratio 0.982395. Both passed the 1.05 ceiling, so
every exact criterion passed and forced first proof-optimizing performance is
**Adopted**.

The complete adopted record is
[adopted result][adopted-result],
SHA-256
`e71d1caf35b57ea50c094806372f6e8c991bf86418f44885c3d6fd0dcd4b082e`.
The recovery does not erase or reinterpret the rejected result, and the scalar
sentinel does not attribute either change to optimizing passes. No automatic
promotion threshold was selected or measured. Automatic promotion remains
disabled; OSR, deoptimization, and speculation remain absent and unclaimed.
Only compact retained JSON is committed; reproducible build and Python cache
artifacts are not evidence and are removed or remain in the shared ignored
target tree.

[adopted-result]: ../../../meta/benchmarks/jit/results/optimizing-jit-linux-x86_64.json
[rejected-result]: ../../../meta/benchmarks/jit/results/optimizing-jit-linux-x86_64-rejected-scalar-regression.json
