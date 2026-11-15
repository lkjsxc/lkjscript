# Source-shape scale ladder

Run the compiler source census and four generated sizes with one worker:

    bash bench/scale.sh --compiler target/lkjscript

The generator is written in lkjscript. The shell copies the current source
modules beside the generator as a temporary main, compiles it with the selected
compiler and uses the actual parser and resolver for the census. It does not
maintain a second lexer. The copied compiler modules are unchanged. The census
is a labelled source claim and has no checker authority.

The parsed source closure is the population: generic and ordinary function
bodies, source-file function counts, module depth, parameter count, resolved
call/method sites, aggregate fields, variant count and nominal type occurrences
in fields. Comments and source files not loaded by this entry are not counted.
Included source text stored as a string is data, not a parsed definition. The
sample file records every observation, allowing the distribution to be inspected
without rerunning the compiler.

## Workload model

A fixed seed samples observed function shapes and nonempty-file function counts.
Each generated file has an observed module depth; each function has sampled
parameter count and generic status. Generic calls use three integer type
arguments. Bodies contain sampled direct-call pressure plus scalar arithmetic
and branches. Dispatch functions with fan-out at most 32 make every body
reachable while avoiding a single enormous entry. They are generator scaffolding,
not a claim that the compiler's call graph has fan-out 32.

Aggregate field counts and nominal-edge pressure come from the measured types.
Vector edges are recursive and finite-layout; sampled enums exercise tags and
matches. This is not a graph-isomorphic copy of the compiler: nominal targets
are folded into the local recursive record, enum payloads use scalar words,
and generic bounds use Copy. Functions are acyclic in declaration order, and
the resulting runtime call DAG may repeat work. These are compilation workloads,
not runtime-speed benchmarks. The real compiler is measured separately.

Each requested rung has exactly the requested physical source-line count.
The final function may be shortened; any remaining small tail is explicit
constant declarations, counted in workload.txt. Source lines are not produced
by adding blank padding. The report gives actual files, functions, generic
functions, source bytes and type/field/edge/variant counts so changes in the
generator cannot silently masquerade as compiler improvements.

The edit target is edit::target in its own file. Exactly eight source functions
call it. The one-body edit changes one integer literal. The signature edit adds
one parameter and supplies its argument at those eight sites. Some generic
callers can have multiple checked instances; eight is the source dependent
count. Cached images for both edits must equal independently rebuilt no-store
images. The unchanged image must equal the cold one, and the body edit must
change the image.

Until the compiler exposes identity operations, the harness labels the edits
as projection edits and reports query/rename rows as pending. It never counts
sed as a checked semantic transaction. --query-driver supplies an executable
which receives compiler, source root, store, key path, line count, memory limit
and watchdog, and must report each bounded question separately. --operation-driver
receives compiler, source root, store and key path and is timed as the rename
transaction. Hooks are executable paths, not evaluated shell strings.

## Resource and measurement contract

One compiler or generator runs at a time. Every elapsed observation has
1/5/15-minute load averages from immediately before and after it. Build rows
include wall, user and kernel CPU time, peak RSS, image bytes, checked instance
count, retained store bytes and the explicit store budget. Images, stores,
source copies and generated trees are removed by the exit trap. The private
authentication key is created outside all source/library grants and removed
by the same trap. Each run records its owned directory, process identity, key
location and current phase before starting work. If the supervisor itself disappears,
`bash bench/scale.sh --cleanup RUN_DIRECTORY` refuses a live owner, verifies paired
ownership markers, preserves checkpoint/shape/workload/time/stderr/stdout in a
recovery directory and removes only that run and its key. An unavailable time
record remains unavailable; recovery does not infer an exit status or peak RSS.

The first rung uses a 2 GiB external address-space limit. Later limits predict
from the preceding successful peak RSS, line ratio and observed memory exponent
(with a floor of linear growth), then allow twice the prediction plus 256 MiB.
The default experiment ceiling is 8 GiB. This is RLIMIT_AS, not an RSS limit:
allocator reservations and address-space fragmentation can cause refusal at a
lower resident size. It is a harness boundary, never a language or tool policy.
--memory-ceiling-mib explicitly changes that experimental ceiling. The compiler
continues to discover actual mapping refusal normally.

The watchdog similarly scales from the previous cold time; ordinary compiler
work gets a generous allowance. --prior-results can supply a preceding successful
rung from an earlier invocation for resource prediction. Disk space is checked
before each generated tree using measured source bytes per line and predicted
store retention. A resource refusal remains a result. Higher rungs are marked
not run after that refusal; they are not invented measurements.

--results selects the TSV, with .shape source observations and .phases actual
compiler statistics beside it. The generator's input seed and compiler SHA-256
are printed. Successful adjacent observations produce exponents
log(cost_after / cost_before) / log(lines_after / lines_before). Fixed compiler
startup cost, source-library cost and host load affect small-rung exponents;
an exponent below one does not prove asymptotically sublinear cold compilation.

## Baseline, source 1f48bd24

The compiler/source library bytes are unchanged from 1f48bd24. The measured
producer SHA-256 is
944d5e663bc6f7f703b8a399656f774a785966af4029ac56294e4b4c0fc2d2e7.
The parsed compiler+source-library closure has 77,589 lines, 3,007,295 bytes,
147 source files and 2,771 function bodies. This closure count does not include
unloaded test files or code embedded as text; it is not a revision of a whole
repository line count.

| Source observation | Median | p90 | p99 | Maximum | Mean |
| --- | ---: | ---: | ---: | ---: | ---: |
| Function physical lines | 9 | 62 | 257 | 984 | 25.450 |
| Resolved call/method sites per body | 4 | 33 | 162 | 557 | 14.104 |

There are 130 generic functions (4.691%); module depths 0/1/2/3 contain
1/1,254/1,466/50 function bodies. The 146 files with bodies average 18.979
functions and reach 138. The nominal graph census has 383 types, 1,880 fields,
1,454 nominal occurrences in field types and 793 variants. Nested type
arguments contribute nominal occurrences; these are not distinct graph edges.

One worker ran each measured row. Filesystem caches were not reset. The real
compiler used an isolated source checkout, so its embedded diagnostic paths and
image bytes differ from the producer's main-worktree image.

| Rung | Cold wall s | CPU s | Peak RSS KiB | External address-space MiB | Stored bytes | Image bytes | IR instances | Load before → after, 1/5/15 min |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| Real compiler, 77,589 parsed lines | 23.82 | 23.37 | 555,180 | 2,048 | 74,064,498 | 17,047,676 | 10,311 | 4.63/4.81/4.26 → 6.09/5.13/4.38 |
| 10,000 lines | 1.70 | 1.66 | 108,552 | 2,048 | 3,113,928 | 573,626 | 446 | 6.15/4.98/4.21 → 6.15/4.98/4.21 |
| 100,000 lines | 13.43 | 13.19 | 698,008 | 2,377 | 30,716,007 | 5,535,618 | 4,165 | 6.35/5.07/4.24 → 5.76/4.98/4.23 |
| 1,000,000 lines | unavailable | unavailable | unavailable | 8,192 | no published record | no image | unavailable | start 6.27/5.22/4.42; completion unavailable |
| 10,000,000 lines | not run after prior refusal | — | — | planned ceiling 8,192 | — | — | — | — |

At one million lines, the preserved compiler stderr reports
"library/runtime.lkj:13: R0006 heap allocation failed". The gateway's native
supervisor then disappeared; its completion result and GNU time file are absent.
This is evidence of a language allocation refusal under the explicit limit,
not a measured full-run duration or proof of host OOM. The last independent
process observation showed 3,999,252 KiB resident after 77 seconds at host load
28.45/10.80/6.34. That is an intermediate observation, not a peak or a completed
timing. The recovered host memory.events showed no OOM kill. The source tree,
store and private key were removed; the metadata and diagnostic were preserved.

One-million-line generation itself completed in 11.914 s
(load 6.09/5.13/4.38 → 6.27/5.22/4.42): 40,054,967 source bytes, 1,859 files,
36,766 source functions, 1,712 generic functions, 472,970 direct call sites,
2,233 nominal types, 7,210 record fields, 3,519 variants, 3,977 vector edges
and no tail constants. It is not a successful one-million-line build.

| Rung / operation | Wall s | CPU s | Peak RSS KiB | Load before → after, 1/5/15 min |
| --- | ---: | ---: | ---: | --- |
| Real compiler unchanged | 0.58 | 0.55 | 27,580 | 6.09/5.13/4.38 → 6.09/5.13/4.38 |
| 10k unchanged | 0.27 | 0.25 | 7,464 | 6.15/4.98/4.21 → 6.15/4.98/4.21 |
| 10k one body | 0.95 | 0.93 | 102,448 | 6.15/4.98/4.21 → 6.38/5.05/4.23 |
| 10k signature, 8 direct source callers | 1.63 | 1.60 | 109,992 | 6.38/5.05/4.23 → 6.38/5.05/4.23 |
| 100k unchanged | 0.40 | 0.38 | 19,456 | 5.76/4.98/4.23 → 5.76/4.98/4.23 |
| 100k one body | 6.70 | 6.57 | 576,940 | 5.76/4.98/4.23 → 5.42/4.93/4.22 |
| 100k signature, 8 direct source callers | 13.78 | 13.53 | 703,976 | 5.42/4.95/4.23 → 6.00/5.11/4.30 |

The cached body/signature images matched their no-store controls. Those controls
took 1.13/1.16 s at 10k (loads 6.38→6.38 for each) and 11.36/10.96 s at 100k
(loads 5.42→5.42 and 6.00→5.68). Both unchanged images matched their cold image.
The separate 1,000-line fixture proves exact line accounting, same-seed byte
identity, changed-seed differences and acceptance by the compiler. Recovery
checks prove that an active owner is refused and interrupted diagnostic evidence
survives cleanup.

| Cost, 10k → 100k | Wall exponent | CPU exponent | RSS exponent |
| --- | ---: | ---: | ---: |
| Cold store build | 0.8976 | 0.9001 | 0.8082 |
| Unchanged build | 0.1707 | 0.1818 | 0.4161 |
| One-body edit | 0.8484 | 0.8491 | 0.7506 |
| Eight-caller signature edit | 0.9271 | 0.9272 | 0.8062 |
| No-store body control | 1.0023 | 1.0016 | 0.7788 |

These two successful points support near-linear cold CPU work, with a substantial
fixed startup/library term; they do not establish the top-rung asymptote.
For illustration only, extending the measured cold exponents predicts 106.1 s
and 4.280 GiB peak RSS at 1M, then 838.2 s and 27.524 GiB at 10M.
Those are extrapolations. The observed 1M address-space refusal prevents treating
them as actual resource requirements or completed timings. A conservative linear
RSS prediction is larger, which is why the benchmark ceiling was reached.

## First measured walls and checking-unit consequence

The 100k one-body edit reconstructs exactly one definition and one IR function,
and reuses 4,164 checker verdicts and 4,159 native fragments. It still takes
6.70 seconds: parse 1.984 s, keys/hashes 1.164 s, store I/O 1.133 s, checkpoint
restore 0.893 s and type bookkeeping 0.584 s dominate. The actual changed-body IR
construction is 1.425 ms. These phase times share load 5.76/4.98/4.23 →
5.42/4.93/4.22. Increasing machine-code speed alone cannot make this edit cost
independent of program size.

The eight-caller signature change invalidates every one of the 4,165 checked
instances and all native fragments. The shared graph namespace therefore
creates a second whole-program wall even before type elaboration is considered.
Cold time at 100k is 134.3 microseconds per physical generated line and peak RSS
is about 6.98 KiB per line; these constants include the fixed library/runtime
term and describe this workload only.

The evidence favors independently authenticated definition/instance records
with checked dependency interfaces, plus source-file-local syntax storage.
A changed body should load its source span, its signature/type/effect dependencies
and its own proof/code fragment; unchanged peer bodies should remain unopened.
A signature change should invalidate the exact dependent interfaces and bodies,
not the whole graph namespace. The IR checker remains the single trust boundary:
a stored index cannot confer proof, and authentication must bind the complete
identity and bytes of each lazily opened record.

This is the measured architectural recommendation, not an implementation claim.
The first walls are whole-program parse/checkpoint/hash work at 100k, whole-graph
signature invalidation at 10k already, and allocation refusal at the 1M experiment
ceiling. Query and identity-rename costs remain pending their production
interfaces; no target is declared met by these baseline observations.
