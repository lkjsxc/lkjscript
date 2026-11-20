# Rejected field-decomposition experiment

The pass is discarded under the fixed 10% rule. On the preselected six-run
compiler workload, mean wall time fell from **18.733333 s to 18.223333 s
(2.722420%)**. No optimizer change or command flag from this experiment is
retained in production. The implementation survives only as the reproducible
[producer patch](compiler-scalar-replace-data/producer.patch).

## Fixed comparison

Both producers compile source revision
`26bf2c3c021dd79810a81fa1d9054d59f3768b06`. The control is constructed with
`--inline-limit 64`; the candidate adds `--scalar-replace`. Every measured
execution builds the same frozen compiler with inlining disabled and no store.
The order, three repetitions per policy, 4 GiB address-space allowance, 900 s
watchdog and 10% threshold were fixed before execution. There were no warmups,
retries, discarded timing rows, samples or census.

| Order | Producer | Wall seconds | User + kernel seconds |
| --- | --- | ---: | ---: |
| 1 | inline64 | 21.31 | 20.85 |
| 2 | inline64 + fields | 18.45 | 18.12 |
| 3 | inline64 + fields | 17.47 | 17.16 |
| 4 | inline64 | 17.61 | 17.23 |
| 5 | inline64 | 17.28 | 16.97 |
| 6 | inline64 + fields | 18.75 | 18.38 |

Mean CPU time is 18.350000 versus 17.886667 s. A separately constructed Rust
`-O` producer took 3.65 s on the same computation, so the candidate is
**4.992694× Rust `-O`**, outside the 2× target. This is one shared-host cohort;
the recorded one-minute load during the six executions ranges from 9.39 to
13.04. These are descriptive timings, with no cross-revision or significance
claim. The complete [rows](compiler-scalar-replace-data/rows.tsv) retain every
phase, RSS observation and load boundary.

All six native outputs and the Rust output are byte-identical to the frozen
16,228,028-byte image, SHA-256
`8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132`.
[Parity](compiler-scalar-replace-data/parity.tsv), the
[summary](compiler-scalar-replace-data/summary) and exact
[configuration](compiler-scalar-replace-data/metadata) are retained.

## What was built

This is an actual untrusted IR transformation, including Move aggregates,
field loans, partial moves and reconstruction at whole-value call/return
boundaries. Whole-address-taken and custom-destructor roots stay intact.
The old aggregate local becomes unused unit storage. All signatures, types,
return-view contracts and destructor definitions remain unchanged; final
candidate IR must independently pass the checker, otherwise selection returns
the original checked program.

The fixed compiler candidate decomposed 5,393 locals in 816 functions and passed
admission without fallback. Its image grew from 22,937,501 to 23,527,325 bytes.
Construction wall time was 32.66 versus 33.13 s; peak RSS was 804,968 versus
1,162,472 KiB. The field transformation itself took 719.234 ms.

Existing phase counters accumulate original admission and optimized admission:
body checking totals are 16,605.273 versus 16,208.657 ms. They do **not** isolate
candidate-only rechecking. The `check_optimized_ms=0.007` fields measure only
bookkeeping after nested phases, and are not a recheck-cost claim. Exact
[control](compiler-scalar-replace-data/raw/construct.sra0.1.stderr) and
[candidate](compiler-scalar-replace-data/raw/construct.sra1.1.stderr) statistics
are retained separately from execution timings.

Focused semantic tests covered eight observations in six groups: Move partial
cleanup in loops, custom destruction order, whole calls and results, copied
snapshots, field loans, returned view origins, bounds and custom traps. They
compare original independent interpretation and exact original/candidate native
stdout and private result/trap records. After the exhaustive instruction-arm
rewrite, the final producer passed the bounded CLI
[controls](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-scalar-replace-data/cli.stdout), including original
admission before optimization, unit-command refusal, cache separation,
`--no-opt`, original IR/authority and worker-option propagation. The frozen
optimizer [oracle](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-scalar-replace-data/raw/oracle.sra1.1.stdout),
including its mutation slice, also passed. No full gate or fixed-point claim is
made for the rejected experimental source.

## Reproduction and decision

[Producer identity](compiler-scalar-replace-data/producer.identity) pins the
measured pre-rebase source `8744c126b84a65f7ca77cfe5cf60a85c1ed5a732`,
complete source delta, emitter bytes and harness. That base object is not in
published ancestry. Reproduce it from published
`91af342f48c154b2924d9d48a04eda6755c803d6` in an isolated worktree: first apply
the exact [prerequisite patch](compiler-scalar-replace-data/prerequisite.patch),
then the producer patch. The prerequisite changes only `library/iter.lkj`
and `witness/check.sh`; `src` is identical between the bases. Its SHA-256 is
`75dc98f8d65d5d70302e80763733c8923bca2e152027569eb19aa41bcd2a9498`.
This reconstructs the measured source without requiring the unpublished object.
Copy the retained [CLI control](compiler-scalar-replace-data/cli-control.sh)
to `tools/scalar-replacement-test.sh`, and rebuild the experimental compiler.
The [harness](compiler_scalar_replace.sh) takes its exact compiler digest and
uses the same frozen computation. Recorded arguments, GNU times, streams and
capture boundaries are in [raw evidence](compiler-scalar-replace-data/raw/).
The harness verified unchanged frozen source/runtime inputs and emitter bytes.

The result supports removing this local-only pass, not claiming that scalar
replacement cannot help. Whole aggregate parameters still cross the function
ABI unchanged. A different implementation that also removes those call-boundary
copies is a separate candidate and must meet the same threshold against
unchanged inline64. No adoption credit is claimed for this rejected code.
