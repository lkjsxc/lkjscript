# Status — 2026-11-14

## Current milestone: 6.2 complete; next 6.3

One static lkjscript executable contains frontend, checker, backend, independent
owned-value interpreter, CLI and embedded library/runtime sources. Source and
text IR cross one gate; stages exchange checked IR in memory. World derives
process/clock/directory capabilities with explicit child arguments and streams.
Interpreter self-execution prepares a checked native image lazily. Raw scalar
results and trap diagnostics use a private descriptor, preserving program stderr.

Stored seed: 2,385,358 compressed bytes. sh tools/bootstrap.sh builds the tree,
checks every library body and verifies byte reproduction without Rust or Node.
Final bare Debian bootstrap/toolchain/exact-float checks passed in 55.21 s;
cargo/rustc/node were absent. Its compiler bytes match the host and Rust witness.
The executable is static; compiled hello runs under a 4 GiB address-space limit.

Eight-worker gate: 183.357 s. 453 Rust tests, 17 harness tests, 889 conformance
passes plus four pending per route, 577 Rust comparisons and 114 World observations.
Source/IR boundary: 568 verdicts, 64 rejections, 504 identical artifacts, two pending.
Fixed mutation slice: 128, zero findings. Long campaign: 32,768, zero findings;
26,073 rejected, 6,136 equal values, 458 equal traps, 101 did not terminate within
their two-second allowance; execution 54.13 s with six workers.

Deleted the component launcher, installer/rebuild scripts, source bundles,
result reader/packet modules, redundant Rust CLI and old shell suites.
All user bodies are checked; library body checking follows demand and reuses
source indexes. Bootstrap checks the complete library. No optimizer was added.
Next: expectation headers and native case discovery/workers, then port generators,
mutation/reduction and reference tests; move the optional Rust witness out of root.

## Compiler measurements

| Work | Self-built | Rust-built witness | 6.1 |
| --- | ---: | ---: | ---: |
| Hello build | 0.095 s | 0.014 s | 0.37 s self-built |
| Complete compiler build | 21.32–24.14 s | 2.05–2.09 s | 45.38 s component phase sum |
| Complete compiler peak RSS | 340.8 MiB | 245.1 MiB | 287.3 MiB frontend only |
| Executable bytes | 11,725,962 | Identical | 11,759,226 two components |
| Hello IR | 1 function / 25 instructions | Identical | — |

First unified hello: 0.425 s, 108 functions / 4,434 instructions; typing dominated.
The final 24.14 s rebuild ran beside the campaign: 7.55 s inference, 8.40 s IR
construction, 6.81 s checking, 0.56 s emission. The self-build/witness gap remains above tenfold.

| bench/ workload | Native | rustc opt=0 | rustc -O | 6.1 native |
| --- | ---: | ---: | ---: | ---: |
| 20 M aggregate calls | 0.065 s | 0.023 s | 0.018 s | 0.066 s |
| 200 M scalar multiply/xor | 0.259 s | 0.257 s | 0.256 s | 0.262 s |
| 200 M scalar add/xor/mask | 0.228 s | 0.132 s | 0.130 s | 0.177 s |
| 200,000 fresh 1,000-i64 buffers | 0.391 s | 0.459 s | 0.090 s | 0.344 s |
| 5 M pushes + 100 M Vec reads | 0.156 s | 0.252 s | 0.072 s | 0.180 s |

Single shared-host runs; equal outputs. Buffer kernel CPU <0.01 s; vector RSS 41.1 MiB.

## Known gaps

Pending: concurrency_moved/shared/alias, borrowed_closure_escape and one conservative ownership rejection.
Deep reborrow chains remain slow in the interpreter. The native stress cases stay explicit.
The gate/generators still require Rust/Node; bootstrap and the complete compiler do not.
