# Release of one fixed-witness copy-site cohort

Root releases exactly one invocation of the committed protocol after inspecting
CI43872738366 and the local source gate/fixed point. This release is a technical
execution decision within the owner's existing authorization, not a request for
another owner confirmation. It changes no source, library, runtime or optimizer.

The complete input/tool/acceptance pins are in
[compiler-profile-copy-sites-release-20261116.json](compiler-profile-copy-sites-release-20261116.json).
The acceptance foundation is commit
`cd1a63435ee856a334fc08340b4748c987f4838b`; protocol/helper preparation is
`6fc9099ca48b17a7ff150cd7ca9e275f8a5867e4`. The source producer is the
32,584,542-byte fixed point `6cd52308…e7bdacc`, from source `d461c2d9` and
seed `7d0731e9`, not a moving target/lkjscript lookup.

## Exact launch

Gateway request ID: `compiler-copy-sites-fixed-witness-20261116-once`.
Working directory: `/home/coder/workspace/lkjscript`.

```sh
bash bench/compiler-profile-copy-sites.sh run bench/compiler-profile-copy-sites-release-20261116.json
```

Cohort: `compiler-copy-sites-20261116-1`.
Fresh run: `target/compiler-copy-sites-run.20261116-1`.
Fresh evidence: `target/compiler-copy-sites-evidence.20261116-1`.
The release JSON must be exact committed HEAD bytes when invoked. A failed
preflight consumes this attempt; the command is not a preflight dry run.

One worker performs the four declared compiler phases, then the two official
reads and one frozen helper invocation. Each uses 4 GiB/900 s/10 s grace.
Ordinary and profiled producer constructions both use inline64; their frozen
output computation explicitly uses inline0. Both resulting outputs must match
the 16,228,028-byte frozen seed exactly. Source/library/runtime selectors and
all paths are fixed by the archived 26bf2c3c witness and release JSON.

All raw captures precede validation. Any refusal, oracle/capture mismatch,
resource failure or incomplete output stops the sequence. There is no repair,
retry, extra page, alternate input, warm-up or changed bound within this cohort.
Complete raw profile and selected IR remain live and receive checked gzip-n
copies. Cleanup is withheld until independent acceptance.

The existing local compiler jobs are complete; no other compiler workload is
released in parallel. Old-scope cleanup is held until the measurements end.
The prospective whole-body review [rules](compiler-copy-sites-census-plan.md),
SHA `8a1f3e3d5354b3eca7fd9adb68bfee892f94360130a19b265f3f9ffb73c4dc15`,
were committed before this launch. That separate offline extraction still needs
its own accepted input/extractor pins and release; it is not an eighth phase.

This one ordinary-then-profile observation measures total opt-in instrumentation
cost under its actual loads. It is not a speedup trial or an isolated copy-table
cost, and it supplies no automatic optimization eligibility or Rust-O milestone.
