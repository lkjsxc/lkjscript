# Release of copy-site cohort 2 after preflight repair

Root releases one new cohort using the corrected launcher. Cohort 1 remains
closed at its [retained preflight refusal](compiler-profile-copy-sites-cohort-1-refusal.md),
with zero compiler phases and no run directory. This new release is a separate
prospective attempt, permitted by the original protocol's once-only completion
rule. It does not resume, overwrite or reclassify that failure.

The repair and complete failure/diagnostic evidence are committed at
`d4af2448dcec6aa932f2119016fe6c9ff4319ac5`. Root reviewed the exact difference:
only die_if's two-scalar prototype and explicit arity diagnostic change.
Twelve pure controls and one corrected read-only preflight pass. None executed
a compiler or the full launcher. The producer, source/library/runtime, seed,
passed gate/CI and accounting helper remain unchanged.

## Exact prospective change

Compared with release 7440f30b for cohort 1, the JSON changes exactly five fields:
cohort ID, release filename, fresh evidence directory, fresh run directory and
launcher SHA-256. All source/tool/producer/acceptance pins, bounds, worker count,
seven-phase order, inline policies and exact output oracle are byte-equivalent
JSON values. The corrected launcher is
`abf52e1d5f22370a999e6e7765c4b28fe34671ff16430aa93da8d7112fd71bd6`.

## Exact launch

Gateway request ID: `compiler-copy-sites-fixed-witness-20261116-cohort2-once`.
Working directory: `/home/coder/workspace/lkjscript`.

```sh
bash bench/compiler-profile-copy-sites.sh run bench/compiler-profile-copy-sites-release-20261116-2.json
```

Cohort: `compiler-copy-sites-20261116-2`.
Fresh run: `target/compiler-copy-sites-run.20261116-2`.
Fresh evidence: `target/compiler-copy-sites-evidence.20261116-2`.
[The complete release JSON](compiler-profile-copy-sites-release-20261116-2.json)
must be exact committed HEAD bytes before this invocation.

The unchanged [measurement protocol](compiler-profile-copy-sites-measurement-plan.md)
uses one worker, 4 GiB/900 s/10 s per phase, fixed source 26bf2c3c, inline64
producer constructions and inline0 output computation. It stops at any failed
precondition, compiler/read/helper refusal, resource termination, capture defect
or byte-oracle mismatch. Raw evidence precedes validation. There is no warm-up,
extra page, adaptive bound, alternative input or retry within this cohort.
Complete profile and companion originals plus checked gzip-n copies remain for
independent acceptance before cleanup.

The seven old root-owned scopes have finished their separately released cleanup;
no cleanup or other compiler workload will overlap this measurement. The same
pre-data whole-body census plan 8a1f3e3d remains frozen, and later extraction
requires its own exact-pin release. One matched ordinary/profile pair describes
total instrumentation cost under its actual load, with all earlier caveats
unchanged; it is not an optimization-retention or Rust-O comparison.
