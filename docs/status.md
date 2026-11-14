# Status — 2026-11-14, routine native compilation

The installed native `lkj` command now handles declared source/library inputs,
module discovery, bundle packing, runtime preparation, checking, building,
execution and scalar evaluation. The frontend, K compiler and result reader
are ordinary lkjscript programs. Installation and a complete compiler generation
rebuild run without Rust or Node. Bash and standard Linux tools connect the
processes; Cargo, Rust comparisons, the independent K machine and Node test
orchestration remain development/reference tools.

## Counted checkpoint

| Observation | Result | Record |
| --- | --- | --- |
| Current frontend own-source K | 18,740,263 bytes; native hash identical to B0 | observations/2026-11-14-native-workflow.json |
| Frontend ELF fixed point | 51,430,946 bytes; seed and next generation byte-identical | observations/2026-11-14-native-workflow.json |
| K compiler ELF fixed point | 16,841,426 bytes; seed and next generation byte-identical | observations/2026-11-14-native-workflow.json |
| Result reader ELF fixed point | 531,762 bytes; seed and next generation byte-identical | observations/2026-11-14-native-workflow.json |
| Runtime K | 28,472 bytes; regenerated natively and byte-identical | observations/2026-11-14-native-workflow.json |
| Complete native rebuild | 443.56 s; 766 MiB peak RSS; four matching K hashes, four exact artifact comparisons | observations/2026-11-14-native-workflow.json |
| Native process interface | 41 outcomes; source/bundle/K identity, scalars, traps, streams, paths, controls and hashes all pass | observations/2026-11-14-native-gate.json |
| Launcher cancellation | 16 TERM/INT cases pass, including GNU time descendants; three shell contract suites pass | observations/2026-11-14-native-workflow.json |
| Full gate | 461 Rust tests, 17 harness tests, all-target Clippy clean; 71.597 s with eight workers | observations/2026-11-14-native-gate.json |
| Execution conformance | 883 pass and four pending per route; 573 Rust comparisons; 91 World observations | observations/2026-11-14-native-gate.json |

The complete rebuild used only the installed seed. All 105 compiler/library/script
inputs had identical before/after hashes; the forbidden Cargo/rustc/Node log was
empty. It selected an external 8 GiB address-space limit, inherited an 8 MiB
stack, and had no deadline. Those are experiment controls, not defaults.
Later launcher path/cancellation fixes preserve all 102 compiler/library inputs
and the compiler binaries; the final gate exercises the corrected launchers.
The release build was prepared before the timed gate and reused there.

## Measured source lookup improvement

A complete 99 Hz profile of the unchanged frontend found two repeated source
scans taking 13.29% of sampled CPU time. Generic ranges now have a file/name
index while retaining span containment; derived bounds have a declaration-name
index while retaining their union/any-match rules. No language, K, ownership,
encoder, runtime or source-library rule changed.

| Same frozen source input | Before | Candidate |
| --- | ---: | ---: |
| Complete profiled run | 248.42 s | 218.35 s |
| Peak RSS | 775,475,200 bytes | 800,636,928 bytes |
| Selected lookup union | 32.71 sampled CPU s | 1.41 sampled CPU s |
| K output | 18,489,341 bytes | Identical bytes |

This pair observed 12.10% lower wall time and 3.24% higher peak RSS. It is one
matched pair on a shared host; the candidate also includes direct-source input
support. All samples map to the exact executable and neither run lost records.
The candidate recorder failed to retain a live `/proc` limits snapshot; its
launcher stack and executed prlimit command remain recorded. The failed metadata
copy was disclosed and the existing profile decoded without repeating the run.
Full evidence and limits: observations/2026-11-14-native-profile.json.

## Routine interface

See [native.md](native.md) for installation, explicit toolchain selection and
rebuilding. `run` preserves ordinary process streams/status; `eval` additionally
prints scalar returns, with binary64 values represented by exact hexadecimal
bits. Outputs are published after successful compilation. Default execution
inherits Linux limits, and optional settings tighten only the target.
The source loader is shared by direct files and bundles. Runtime source is
compiled as normal K and sealed by the K compiler during emission.

The earlier ownership verdict, port, first fixed points and complete native
source corpus remain in their observations and Git history. Current K mutation
has zero findings, with its two-route timeout recorded separately. The four
pending source cases and one known conservative ownership rejection remain.
Runtime resource-trap filenames still lose source-file provenance at the host
boundary, and finite exhaustion locations can differ between representations.

## Next obligation

Use the native command for routine compiler work. The complete candidate profile
still attributes 24.64 sampled CPU seconds (11.43%) to `has_view`/`is_copy`, with
repeated full-type-table boolean vectors overlapping that cost. Investigate that
bookkeeping before another broad speed change. Keep substantial optimization in
rechecked K-to-K passes and retain the independent K machine. A direct K generator
is the next useful self-built development tool; unrelated surface width waits
for a real consumer.
