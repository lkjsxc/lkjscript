# Native benchmarks

Run from the checkout after bootstrap:

```sh
sh bench/run.sh
```

The first row measures the complete compiler rebuilding itself. This is the
representative workload: small scalar kernels do not predict compiler speed.
The runner then builds each source and checks a small input before measuring its
default input. Every route must produce identical stdout bytes, empty stderr and
status zero. Timing is sequential and observational; no time is an acceptance limit.

| Program | Default workload | Observable result |
| --- | --- | --- |
| src/main.lkj | Complete self-build with explicit current library/runtime | Compiler artifact |
| aggregate64.lkj | 10 million 64-byte records passed/returned through three call levels | All eight fields in recurrence checksum |
| vector64.lkj | 1 million inline 64-byte records, then 10 million reads | All eight fields in sum |
| aggregate.lkj | 20 million calls taking/returning a two-word Copy struct | Recurrence checksum |
| scalar.lkj | 200 million multiply/add/xor iterations | Final word |
| scalar_add.lkj | 200 million add/xor/mask iterations | Final word |
| buffers.lkj | 200,000 fresh Vec<i64> buffers of 1,000 elements | Indexed checksum |
| vector.lkj | 5 million Vec<i64> pushes, then 100 million reads | Sum |
| http_parse.lkj | 200,000 changing HTTP/1.1 requests through the consumer parser | Header boundary, path byte/length, method, body length and persistence checksum |
| string_map.lkj | 4,096 owned qualified String keys; 500,000 lookups with replacement/removal/reinsertion | Lookup and final ordered traversal checksum |

Counts and seeds are runtime arguments. Call-chain functions use inline(never),
which rustc honors. lkjscript IR has no source inline hints: use --inline-limit 0
for retained-call ABI comparisons; an enabled pass may remove those calls. Choose
thresholds using the compiler and the HTTP/String-map rows, which carry no hints.
Vector timings include construction. Buffer reuse and large vector remapping remain separate
workloads. Reports include elapsed/user/kernel time, peak RSS and executable size;
transient results and images stay beneath target/bench/. Each timing includes the
1/5/15-minute load averages read immediately before that run on the shared host.

The HTTP row stages the current `examples/http_protocol.lkj` beside its native
source; the Rust wrapper imports that same example file. There is no duplicate
parser implementation. A path digit changes on every iteration, and the row also
checks HEAD, URL decoding, query stripping, connection close and missing Host
before its workload. It performs no socket or filesystem work.

The String-map row owns qualified names with shared module prefixes. Every
iteration looks up a key; one in eight replaces its value with a fresh owned key,
and one in eight removes and reinserts the entry. Construction and final ordered
traversal are included. Its small check independently verifies replacement,
lookup misses, removal, reinsertion and key order. Native uses the source library's
AVL arena and Rust uses `std::collections::BTreeMap`; this is an ordinary
String-map workload comparison, so its ratio includes collection implementation
costs as well as emitted-code costs. Neither new row applies inline hints.

Optional comparisons:

```sh
sh bench/run.sh --witness0 target/witness0/release/lkjscript --witness target/witness/release/lkjscript --rust
sh bench/run.sh --compiler target/lkjscript --previous target/previous-lkjscript
```

--inline-limit N selects the rechecked inliner threshold for the native compiler
and same-source compiler witnesses; zero disables it. Previous compiler routes
retain their own policy. Use a compiler artifact built at each candidate threshold
when measuring that threshold's self-build, so the producer code and output policy
are both represented. The runtime workload outputs still agree across every route.
The default is zero: the measured candidate-checking cost currently outweighs the
cold self-build speedup. For an explicit optimization comparison, add
`--inline-limit 16` to both compiler construction and this runner. The gate also
executes the full conformance and generated corpora at threshold 64 against the
original interpreter, regardless of the default.

--witness names the optimized Rust-built compiler of the same sources.
--witness0 adds an unoptimized Rust-built compiler. All self-build routes explicitly
disable the store, and both witness outputs must match native compiler bytes.
Either witness option also enables rustc optimization levels 0 and 3 for the small
workloads. --rust enables those two workload routes alone.
They use one codegen unit, the optional witness host adapter and checked arithmetic
with aborting panics. --previous names an earlier native compiler which can build these sources;
its compiler artifact may differ when emission changed. Default execution needs
neither Rust nor Node. GNU time, sh and ordinary file utilities provide measurement orchestration.

## Compiler triple and operation profiles

Build both witnesses with one compilation job and one codegen unit, changing only
the optimization level and output directory:

```sh
CARGO_BUILD_JOBS=1 CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
    CARGO_TARGET_DIR="$PWD/target/witness0" CARGO_PROFILE_RELEASE_OPT_LEVEL=0 \
    cargo build --manifest-path witness/Cargo.toml --release --quiet
CARGO_BUILD_JOBS=1 CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
    CARGO_TARGET_DIR="$PWD/target/witness" CARGO_PROFILE_RELEASE_OPT_LEVEL=3 \
    cargo build --manifest-path witness/Cargo.toml --release --quiet
```

Use the same source snapshot for native, witness0 and witness3. The triple measures
execution of compiler code; warm image reuse is measured separately below.

A profiled compiler can attribute its own build by function and emitted-code kind:

```sh
target/lkjscript build src/main.lkj --library-dir library \
    --runtime-source library/runtime.lkj --profile --no-store -o target/bench/compiler-profile
rm -f target/bench/compiler-profile.samples.*
target/bench/compiler-profile build src/main.lkj --library-dir library \
    --runtime-source library/runtime.lkj --no-store -o target/bench/profile-output
target/lkjscript profile target/bench/compiler-profile.samples.*
```

The report has at most 37 lines: seven self functions, seven inclusive functions,
and sixteen exclusive operation categories. Every interrupted instruction belongs
to one category. Copy, checks, allocator and runtime regions own their internal
instructions. Outside them, argument/result setup, prologue/epilogue, return and
machine-call instructions are separate. Register-to-register moves, immediate
constants and LEA/address formation are separate from arithmetic/comparison and
conversion. Frame and other memory traffic, branches and unknown instructions
remain visible. LEA is an exact emitted category; integer ADD used for pointer
arithmetic is still arithmetic. Real frame chains supply inclusive counts.
Instrumentation and maps are opt-in, and cached fragments preserve their ranges.

## Checked-work reuse

Measure reuse with the current compiler or an explicit executable:

```sh
sh bench/reuse.sh
sh bench/reuse.sh --compiler target/lkjscript
```

The script copies `src/` and `library/` into a private snapshot beneath
`target/reuse-bench.*`. Every build uses that snapshot as its source Directory
and names its library and runtime explicitly. Preparation happens before timing;
the working tree is never edited. Five builds run sequentially:

| Row | Inputs and store |
| --- | --- |
| `no-store` | Original snapshot, reuse disabled |
| `cold` | Original snapshot, initially empty store |
| `warm` | Original snapshot, store populated by the cold build |
| `checker-edit` | One checker function body changed, populated store |
| `checker-edit-no-store` | The same changed snapshot, reuse disabled |

The edit changes the first diagnostic column in `check::error` from 1 to 2
without changing source line numbers. The warm row can reuse the complete image;
the edited row measures rebuilding after one body changes, with the compiler's
`--stats` counters showing reused checker verdicts and native fragments.
Both unchanged cached images must match the original no-store image byte for
byte. The changed cached image must match its changed no-store control and
differ from the original image.

Each row reports elapsed seconds, peak RSS in KiB, store bytes including directory
metadata, and the 1-, 5- and 15-minute load averages before and after the build.
Cold overhead is `100 * (cold elapsed / no-store elapsed - 1)` percent. Cold
means an empty lkjscript store; filesystem caches remain in their ordinary state.
Compare timings together with their loads on a shared host.

A fresh private 32-byte key is created through `tools/store-key.sh`, outside
the snapshot source/library grants and the store. The snapshot, key, store,
images and logs are removed on exit. Measurements are printed to stdout; the
script keeps no measurement ledger.

The phase report separates reading/parsing, resolution, definition inference,
concrete construction, identity/hash work, body checking, instruction emission,
relocation/ELF assembly and output publication. Store I/O includes authentication;
front checkpoint decoding/encoding is explicit, and other work remains visible.
Use the wall clock and load alongside these phase totals, including on unchanged
builds; the producer digest is never inferred from an executable filename.

Stable-topology edits to free nongeneric bodies can reuse authenticated front-end
checkpoints. Declarations, source/arena spans, source type spellings, expanded string/byte literals and external
includes, deterministic type/function catalogs and ordered IR storage/call
skeletons must agree. Changed bodies use fresh ordinary inference and construction;
the independent IR checker still admits the result. Changes to source length,
control/storage/call structure, string/byte literals, generic bodies or closure shape fall back to the
ordinary pipeline. A successful checkpoint advances, so later eligible edits
compare with the latest build. This is an optimization condition, not a source
admission limit. Compact binary IR retains exact representation tags; it never
constructs a checked program by itself.

A graph pack stores the current requested body proofs or relocatable fragments
under one HMAC instead of one filesystem object per function. Old body versions
are disposable misses. The default store policy permits 256 MiB of compiler-owned
published envelope and accounting bytes per explicit Directory; `--store-bytes`
changes that disposable bound. Eviction removes oldest producer generations first.
Valid reads need no writer lock; a writer holds a nonblocking lock and commits its
accounting once at the invocation boundary. A crashed or refused publication is
a miss, and owned orphan entries are discarded on the next writing invocation.
Corpus workers use stable worker-N subdirectories with one quarter of the parent
budget each, regardless of the current worker count. A default four-worker gate
retains at most 512 MiB across the base and worker stores; eight workers permit
768 MiB. Arbitrary external files are not compiler-owned cache entries. FIFO or
other stream-like entries are opened without waiting for a peer and miss.

The gate schedules the next weighted suite when any worker finishes and reports
request reuse per suite. Every ordinary proof still binds the exact compiler.
A test may reuse fixed stdout/stderr captures only when the authority query on
both the checked original and any checked native candidate reaches exclusively
world_stdout, world_stderr, stdout_write and stderr_write. The query includes
callees, both branches and owning cleanup. Runtime overrides, profiling, other
host effects and resource/model failures are excluded; declared authority is
unchanged. Output/trap replay and effects hidden in calls/Drop/branches are tested.
Five pure facts/binary-IR contracts also have a stable runner whose imports
exclude the checker. The trusted helper freshly checks one IR snapshot, emits
its no-authority report and sealed image, and binds their exact correspondence.
Only successful execution with empty argv, stdin and captures can cross compiler
generations when the runner and fresh ELF bytes match. Host/resource probes and
compiler verdicts do not cross generations. This does not assert current host
resource availability. The edit benchmark builds both versions of that root and
proves the five-case execution reuse; changed producer/image/contract, forged
entries, swapped reports and failures have permanent attack cases.

### Remaining reuse cost

The initial edit probe spent 20.241 s at load 3.85→4.45, with inference
6.391 s and IR construction 6.778 s its two largest raw phases. The current
coalesced compiler's stable-span checker edit measured 10.539 s against a
33.840 s cold build and 1.244 s unchanged build (one-minute load 19.68→18.58,
21.25→19.68 and 19.68→19.68). It reconstructed one of 2,714 definitions and
one of 10,072 concrete functions; 10,071 checks and 9,922 native fragments
were reused. All cached images matched the corresponding no-store images,
and the edited compiler built and ran the closed gate root with the same bytes.

The initial dominant inference/construction phases are now 0.242/0.015 s
within that edit interval. The 3 s target remains unmet: the present largest
costs are whole-graph authenticated rereads and identity work, 5.313/1.606 s.
Checkpoint restore/encode costs 0.690/0.195 s. These remaining phases have not
become proportional to the edit. The construction counters establish what
changed; heavily loaded wall observations are not an isolated speedup estimate.

Cold storage overhead was 10.51% against the 30.64 s no-store control
(load 21.25→19.68 versus 19.83→21.25). The 3.216 s phase-accounted extra wall time is:

- Dependency and producer identities: 1.629 s; these bind real bytes rather than paths or mtimes.
- Authentication and store I/O: 2.199 s; large reads authenticate before sized allocation, then authenticate the reread.
- Fragment reuse bookkeeping and materialization: 0.641 s over ordinary instruction emission.
- Checkpoint encoding and other publication preparation: 0.273 s extra, including 0.286 s encoding.
- Other ordinary-phase variation: −1.527 s in this pair of observations; changing host load affects the comparison, and timing is not a gate threshold.

Five published records replace the former per-function files. Measured store
bytes were 70,532,103 B after cold and 86,760,238 B after the edit, below the
explicit per-directory policy. The bound is on authenticated contents and its
index; filesystem metadata and transient atomic-publication files are not counted.

### Gate cost

The current emission gate measured below used four workers and the refreshed
seed with aggregate frame-slot coalescing. Expanded-include invalidation and the
Programs desk consumer remain in the gate. The cold run began with an empty
compiler store; the warm run used the same source, seed and populated store.
Filesystem caches were not reset. Loads are the one-minute average before and
after each timed interval, not peak load.

The cold gate took **513.135 s (load 10.98→16.30)** and reran all 4,477 counted
case requests. The warm gate took **147.070 s (load 16.30→9.43)**, reusing 4,454
requests and rerunning 23. Those counts exclude the 762 structural contracts,
128 mutation comparisons and uncounted host/proof suites, which still run.
The 23 warm misses consist of 11 conformance requests under each inline policy
without a reusable observation proof and one deliberately no-store library
override fixture. Reachable effects and completion classes outside fixed-capture
eligibility stay fresh. Compile failures other than source I/O failures can have
authenticated verdict proofs; negative cases do not all rerun.

First-landing CI run `43711410926`, at seed commit `e9b8c940`, separately
passed its seed-only, empty-store gate in **874.494 s (load 0.98→2.80)**,
with 0 reused / 4,475 rerun requests. Its optional witness job also passed.
That identified CI result predates the coalesced source measured locally here.

This is a same-producer warm measurement. Across a checker body edit, the
separate benchmark proves five closed contracts can skip execution after the
freshly rebuilt runner, image and authority bytes match. Ordinary compiler
verdicts remain producer-bound. The gate changes address completion-order
scheduling and fixed-output replay under the checked effect/completion rule;
host observations and resource availability are still tested afresh.

The following partition sums wall time without double-counting overlapping
suites. The first row comprises serial preparation plus the weighted host/proof
window and orchestration. Cold preparation was 63.374 s (load 10.98→11.07) and
the remaining window 80.796 s (load 11.07→9.32); warm preparation was 7.076 s
(load 16.30→17.87) and the remaining window 80.112 s (load 17.87→12.23).
The window is computed as the residual after serialized steps, so it also
includes scheduling and rounding overhead.

| Wall-clock group | Cold s (load 1m) | Warm s (load 1m) |
| --- | --- | --- |
| Preparation + overlapping host/proof suites | 144.170 (10.98→9.32) | 87.188 (16.30→12.23) |
| Conformance default / inline 64 | 168.753 (9.32→11.25 / 14.21→14.95) | 32.629 (12.23→11.41 / 11.41→9.73) |
| Generated default / inline 64 | 148.845 (11.25→14.21 / 14.95→16.09) | 1.337 (11.41→11.41 / 9.73→9.73) |
| Semantic structure | 26.727 (16.09→16.43) | 23.512 (9.73→7.81) |
| Semantic/reference cases + mutation | 24.640 (16.43→16.30) | 2.404 (7.81→9.43) |
| **Total** | **513.135 (10.98→16.30)** | **147.070 (16.30→9.43)** |

The cold 150 s goal remains unmet. The four corpus passes alone occupy
317.598 s (load 9.32→16.09) at four workers; cold construction, host proofs,
structure and the tail account for the rest as above. This is the current
source including the desk consumer, not an isolated comparison against the
older source described in the second compiler note.

Per-suite elapsed times follow. The 17 steps from closed-semantics through desk
overlap; their elapsed times must not be added to the wall-clock partition.
A dash means the suite does not publish counted case-request reuse. All such
suites ran on both passes, even where their internal compiler builds reused
authenticated artifacts.

| Suite | Cold s (load 1m) | Warm s (load 1m) | Requests cold → warm (reused/rerun) | Rerun or reuse reason |
| --- | --- | --- | --- | --- |
| bootstrap | 29.329 (10.98→12.00) | 3.063 (16.30→18.03) | — | Fresh seed bootstrap; authenticated build images may reuse. |
| independence | 0.003 (12.00→12.00) | 0.003 (18.03→18.03) | — | Fresh repository independence check. |
| tool-build | 30.717 (12.00→11.59) | 1.948 (18.03→18.03) | — | Fresh build requests; authenticated images may reuse. |
| closed-build | 3.237 (11.59→11.07) | 1.944 (18.03→17.87) | — | Fresh same-IR authority report and sealed image. |
| generate | 0.088 (11.07→11.07) | 0.118 (17.87→17.87) | — | Fresh deterministic corpus generation. |
| closed-semantics | 0.345 (11.07→11.07) | 0.037 (17.87→17.87) | 0/5 → 5/0 | Successful exact runner/ELF/contract proof; five cases. |
| closed-proof-attacks | 0.576 (11.07→11.07) | 0.632 (17.87→17.87) | — | Fresh authentication, identity and report-swap attacks. |
| toolchain | 60.511 (11.07→9.14) | 62.858 (17.87→13.75) | — | Fresh host and toolchain integration proofs. |
| world | 29.504 (11.07→10.58) | 30.853 (17.87→16.47) | — | Fresh host capability observations. |
| profiler | 1.056 (11.07→11.07) | 1.180 (17.87→17.87) | — | Fresh sampling and profile-format proofs. |
| optimizer | 7.348 (11.07→10.58) | 8.664 (17.87→17.34) | — | Fresh optimizer admission and comparison proofs. |
| resources | 0.718 (11.07→11.07) | 0.806 (17.87→17.87) | — | Fresh resource and refusal observations. |
| runner | 4.290 (11.07→10.58) | 4.300 (17.87→17.72) | — | Fresh runner structural checks. |
| diagnostic-cases | 6.580 (10.58→10.85) | 0.895 (17.72→17.72) | 0/35 → 34/1 | Warm: 34 verdict proofs; one library-override fixture forces no store. |
| store | 43.402 (10.85→9.24) | 43.785 (17.34→14.08) | — | Fresh store corruption, invalidation, bounds and refusal attacks. |
| program-directory | 5.041 (9.24→9.14) | 5.200 (14.08→13.75) | — | Fresh manifest, grants, environment and directory proofs. |
| http | 18.351 (9.14→8.74) | 17.492 (13.75→12.42) | — | Fresh HTTP server/client observations. |
| journal | 2.008 (9.14→9.14) | 1.920 (13.75→13.13) | — | Fresh journal persistence and HTTP observations. |
| stop | 2.928 (9.14→9.05) | 2.874 (13.13→13.13) | — | Fresh native shutdown observations. |
| stop-interpreter | 5.889 (9.05→8.89) | 5.616 (13.13→12.42) | — | Fresh interpreter shutdown observations. |
| site | 2.418 (8.89→8.89) | 2.213 (12.42→12.42) | — | Fresh offline site consumer proof. |
| desk | 6.036 (8.89→9.32) | 4.543 (12.42→12.23) | — | Fresh desk consumer proof. |
| conformance | 82.891 (9.32→11.25) | 16.221 (12.23→11.41) | 0/779 → 768/11 | Warm: 11 requests have no reusable observation proof. |
| generated | 72.556 (11.25→14.21) | 0.746 (11.41→11.41) | 0/1215 → 1215/0 | Warm: all fixed captures have authenticated proofs. |
| optimized-conformance | 85.862 (14.21→14.95) | 16.408 (11.41→9.73) | 0/779 → 768/11 | Same rule, independently keyed inline-limit 64. |
| optimized-generated | 76.289 (14.95→16.09) | 0.591 (9.73→9.73) | 0/1215 → 1215/0 | Same fixed-capture rule, independently keyed inline-limit 64. |
| semantic-structure | 26.727 (16.09→16.43) | 23.512 (9.73→7.81) | — | 762 fresh structural contracts in each gate. |
| semantic-cases | 19.549 (16.43→16.50) | 0.956 (7.81→9.43) | 0/436 → 436/0 | Warm: all exact compiler/input proofs reused. |
| references | 4.407 (16.50→16.30) | 0.653 (9.43→9.43) | 0/13 → 13/0 | Warm: all exact compiler/input proofs reused. |
| ir-mutation | 0.684 (16.30→16.30) | 0.795 (9.43→9.43) | — | 128 fresh mutation comparisons; zero findings in each gate. |

After both gates, the separate 32,768-mutant campaign at inline limit 64
completed with zero findings in 67.973 s (load 9.43→15.71), using four workers.
It is not included in either gate's wall time or 128-mutant count.

## Native emission evidence

These observations use source commit `4d7bb759` and seed commit `e9b8c940`,
including the expanded-literal checkpoint guard, with the aggregate storage
candidate applied. The measurement snapshot's `src/`, `library/` and witness
bridge matched the working tree byte for byte after that application. The
baseline executable was reproduced from the same snapshot before this storage
change and matched the first landing's native compiler. Both executables then
built the same candidate sources with `--no-store --inline-limit 0`.

The candidate changes native frame-slot selection only. A whole-local aggregate
transfer can share its source's slot when the source has no view anywhere in the
function, its final access is this transfer, its first definition is earlier in
the same straight-line block, and the destination's first access is this write.
Types must match and scalar/empty storage is excluded. The original checked IR
is retained for interpretation. Chained transfers inherit the selected slot;
cleanup still follows the original IR and ownership flags.

Loop safety also depends on the checker's existing exact backedge restoration,
not just a linear interval. A destination first initialized within a loop cannot
remain live at its backedge when it was absent at entry; loan holders and origins
must also match the entry state. Dropping that destination while a view survives
is rejected. Thus a view newly borrowed from the destination cannot survive into
the next iteration and observe an earlier source initialization. This was
reviewed against both `End` and targeted `Continue` validation. The permanent
`aggregate_transfer_storage.lkj` fixture compares the baseline native route,
candidate native route and original-IR interpreter, including Copy chains,
source redefinitions, branches, loops, a later destination borrow and owned
aggregate cleanup. A by-value Copy parameter plus a separately supplied mutable
view keeps its value snapshot; a read-only callee parameter is not automatically
safe to alias to caller storage.

The candidate's native fixed point is **16,227,926 bytes**, against the
baseline's **16,772,694 bytes** (544,768 bytes, 3.25%, smaller). The candidate
matches the compiler subsequently reproduced by the main checkout. Rust
witnesses at optimization levels 0 and 3, built with one codegen unit, produced
exactly the same compiler output bytes as the candidate native executable.
Profiling each compiler also preserved its corresponding unprofiled output
bytes. Baseline output can differ from candidate output because storage
selection changed.

### Compiler and workload measurements

Each row below is one sequential observation, with one worker allocated to this
measurement process. The root stream used at most one other compiler worker
during the suite; the host was shared and heavily loaded. Load is the
1/5/15-minute average immediately before that row. User and kernel time are
shown separately from elapsed time. These are not isolated causal estimates,
and no timing is an acceptance limit.

| Compiler route | Wall s | User s | Kernel s | Peak RSS KiB | Executable bytes | Load 1/5/15 |
| --- | --- | --- | --- | --- | --- | --- |
| native candidate | 27.24 | 25.57 | 0.25 | 497988 | 16227926 | 13.21/11.92/11.16 |
| Rust witness opt0 | 15.35 | 14.60 | 0.19 | 367040 | 15965856 | 19.14/13.32/11.64 |
| Rust witness opt3 | 4.55 | 4.23 | 0.16 | 358264 | 6027856 | 17.88/13.34/11.67 |
| native baseline | 31.76 | 30.09 | 0.30 | 496848 | 16772694 | 17.57/13.35/11.68 |

The native candidate measured **1.775×** the opt0 witness in wall time
(27.24/15.35 s), and **1.746×** in user-plus-kernel time
(25.82/14.79 s), at the row loads above. Against opt3 it measured **5.987×**
wall time and **5.882×** CPU time. The baseline observation was 31.76 s wall
and 30.39 s CPU. Both routes have the same self-build input snapshot, but their
loads differ; the observations do not establish that every reduction was caused
by frame-slot sharing.

Every microbenchmark route passed its small-input check and produced identical
stdout, empty stderr and status zero on the measured input. All nine
baseline/candidate native executable pairs differ byte for byte, even where
their sizes match.

| Workload | Route | Wall s | User s | Kernel s | Peak RSS KiB | Executable bytes | Load 1/5/15 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| aggregate64 | native candidate | 0.41 | 0.39 | 0.00 | 264 | 30138 | 23.39/15.26/12.38 |
| aggregate64 | native baseline | 0.42 | 0.39 | 0.00 | 264 | 30138 | 23.39/15.26/12.38 |
| aggregate64 | rustc opt0 | 0.35 | 0.34 | 0.00 | 1964 | 4438584 | 23.39/15.26/12.38 |
| aggregate64 | rustc opt3 | 0.11 | 0.10 | 0.00 | 1720 | 4363752 | 23.39/15.26/12.38 |
| vector64 | native candidate | 0.10 | 0.06 | 0.02 | 65588 | 30138 | 23.39/15.26/12.38 |
| vector64 | native baseline | 0.07 | 0.05 | 0.01 | 64596 | 30138 | 23.39/15.26/12.38 |
| vector64 | rustc opt0 | 0.09 | 0.07 | 0.01 | 64692 | 4440776 | 23.39/15.26/12.38 |
| vector64 | rustc opt3 | 0.03 | 0.01 | 0.01 | 64460 | 4364016 | 23.39/15.26/12.38 |
| aggregate | native candidate | 0.10 | 0.10 | 0.00 | 264 | 26042 | 22.24/15.15/12.36 |
| aggregate | native baseline | 0.11 | 0.10 | 0.00 | 264 | 30138 | 22.24/15.15/12.36 |
| aggregate | rustc opt0 | 0.02 | 0.02 | 0.00 | 1972 | 4438008 | 22.24/15.15/12.36 |
| aggregate | rustc opt3 | 0.02 | 0.01 | 0.00 | 2040 | 4363336 | 22.24/15.15/12.36 |
| scalar | native candidate | 0.37 | 0.37 | 0.00 | 264 | 26042 | 22.24/15.15/12.36 |
| scalar | native baseline | 0.49 | 0.48 | 0.00 | 264 | 26042 | 22.24/15.15/12.36 |
| scalar | rustc opt0 | 0.29 | 0.29 | 0.00 | 1740 | 4437808 | 22.24/15.15/12.36 |
| scalar | rustc opt3 | 0.31 | 0.28 | 0.00 | 1756 | 4363104 | 22.24/15.15/12.36 |
| scalar_add | native candidate | 0.28 | 0.25 | 0.00 | 264 | 26042 | 22.24/15.15/12.36 |
| scalar_add | native baseline | 0.24 | 0.22 | 0.00 | 264 | 26042 | 22.14/15.25/12.41 |
| scalar_add | rustc opt0 | 0.30 | 0.28 | 0.00 | 1908 | 4437880 | 22.14/15.25/12.41 |
| scalar_add | rustc opt3 | 0.16 | 0.14 | 0.00 | 1964 | 4363104 | 22.14/15.25/12.41 |
| buffers | native candidate | 0.54 | 0.52 | 0.00 | 264 | 30138 | 22.14/15.25/12.41 |
| buffers | native baseline | 0.48 | 0.46 | 0.00 | 264 | 30138 | 22.14/15.25/12.41 |
| buffers | rustc opt0 | 0.67 | 0.64 | 0.00 | 1916 | 4440512 | 22.14/15.25/12.41 |
| buffers | rustc opt3 | 0.14 | 0.10 | 0.00 | 1968 | 4364080 | 22.14/15.25/12.41 |
| vector | native candidate | 0.23 | 0.21 | 0.01 | 41804 | 30138 | 22.14/15.25/12.41 |
| vector | native baseline | 0.25 | 0.24 | 0.00 | 41808 | 30138 | 22.14/15.25/12.41 |
| vector | rustc opt0 | 0.27 | 0.26 | 0.00 | 41124 | 4440368 | 21.25/15.18/12.40 |
| vector | rustc opt3 | 0.09 | 0.07 | 0.00 | 40664 | 4363792 | 21.25/15.18/12.40 |
| http_parse | native candidate | 1.09 | 1.06 | 0.00 | 264 | 88260 | 21.25/15.18/12.40 |
| http_parse | native baseline | 1.31 | 1.28 | 0.00 | 264 | 92356 | 21.25/15.18/12.40 |
| http_parse | rustc opt0 | 0.51 | 0.50 | 0.00 | 1984 | 4480016 | 21.25/15.18/12.40 |
| http_parse | rustc opt3 | 0.05 | 0.04 | 0.00 | 1768 | 4382912 | 20.27/15.08/12.38 |
| string_map | native candidate | 1.80 | 1.77 | 0.00 | 3112 | 71606 | 20.27/15.08/12.38 |
| string_map | native baseline | 1.81 | 1.71 | 0.00 | 3116 | 71606 | 20.27/15.08/12.38 |
| string_map | rustc opt0 | 0.25 | 0.24 | 0.00 | 2496 | 4523120 | 20.89/15.29/12.47 |
| string_map | rustc opt3 | 0.06 | 0.05 | 0.00 | 2460 | 4382328 | 20.89/15.29/12.47 |

The regressions remain visible: candidate versus baseline wall/CPU observations
were 0.10/0.08 versus 0.07/0.06 s for `vector64`,
0.28/0.25 versus 0.24/0.22 s for `scalar_add`, and
0.54/0.52 versus 0.48/0.46 s for `buffers`, at their listed loads.
`string_map` was 1.80/1.77 versus 1.81/1.71 s, so its CPU observation also
increased. Subsecond rows and changing host load limit interpretation; the
results are retained without repeating the broad suite to select better numbers.
The String-map witness still uses Rust's BTreeMap while native uses the source
AVL implementation, as described above.

### Exclusive kind split and remaining work

Both profiles execute the same candidate-source self-build at the requested
1,000 µs sampling period. Totals are 28,527 baseline and 25,029 candidate CPU
samples. Every sample belongs to exactly one of the following sixteen categories;
both sums were checked, and unknown is zero. Percentages here are calculated
from those integer counts and rounded to two decimals, rather than the report's
one-decimal truncation.

| Profile route | Wall s | User s | Kernel s | Peak RSS KiB | Load 1/5/15 before → after |
| --- | --- | --- | --- | --- | --- |
| Baseline | 30.43 | 28.28 | 0.34 | 553408 | 20.89/15.29/12.47 → 19.07/15.38/12.59 |
| Candidate | 26.23 | 24.89 | 0.24 | 559620 | 19.07/15.38/12.59 → 16.24/15.02/12.54 |

| Exclusive kind | Baseline samples | Baseline % | Candidate samples | Candidate % |
| --- | --- | --- | --- | --- |
| unknown | 0 | 0.00 | 0 | 0.00 |
| inline copy | 4740 | 16.62 | 3365 | 13.44 |
| checks | 325 | 1.14 | 347 | 1.39 |
| call instruction | 208 | 0.73 | 203 | 0.81 |
| frame load/store | 1319 | 4.62 | 1158 | 4.63 |
| other memory | 2872 | 10.07 | 2604 | 10.40 |
| arithmetic / comparison / conversion | 3024 | 10.60 | 2830 | 11.31 |
| branch | 706 | 2.47 | 658 | 2.63 |
| allocator | 2901 | 10.17 | 2755 | 11.01 |
| runtime | 698 | 2.45 | 686 | 2.74 |
| register-to-register move | 4044 | 14.18 | 3663 | 14.64 |
| address formation (LEA) | 1754 | 6.15 | 1490 | 5.95 |
| immediate constant | 1674 | 5.87 | 1503 | 6.01 |
| argument setup / result capture | 599 | 2.10 | 562 | 2.25 |
| prologue / epilogue | 2869 | 10.06 | 2513 | 10.04 |
| return | 794 | 2.78 | 692 | 2.76 |
| **Total** | **28527** | **100.00** | **25029** | **100.00** |

Copy regions include their own address calculation, moves and memory
instructions; allocator/runtime regions similarly own their internal instructions.
Argument/result setup and prologue/epilogue regions have precedence over the
low-level arithmetic/move classes. Consequently these are exclusive emitted
regions, not overlapping estimates of all moves or all loads. The low-level
`call instruction` row alone is not total calling cost.

The largest candidate self functions returned by the same profile were:

| Function | Self samples | Self % | Inclusive % |
| --- | --- | --- | --- |
| `$std::sequence::__lkjscript_slice_order<T=t5;>` | 966 | 3.8 | 3.8 |
| `$runtime::allocate` | 718 | 2.8 | 6.7 |
| `$std::bytes::append_bytes` | 711 | 2.8 | 4.4 |
| `$runtime::release` | 620 | 2.4 | 3.2 |
| `$runtime::payload_class` | 564 | 2.2 | 2.2 |
| `check::check_function_body` | 406 | 1.6 | 41.8 |
| `$runtime::valid_utf8_bytes` | 393 | 1.5 | 1.8 |

These function percentages preserve the report's truncation. Inclusive counts
overlap and must not be added: `driver::compile` contains 23,874 samples
(95.3%), while `check::check_all` contains 10,464 (41.8%). The observed self
cost is distributed across checking, ordering, byte handling and allocation.

The 1.0× opt0 milestone is **not met**. The copy share fell from 16.62% to
13.44%, while register-to-register moves alone are 14.64%. Copy plus every
call-related category totals 7,335/25,029 samples, **29.31%**. Matching opt0
would require reducing the observed native wall time by **43.65%**, or CPU time
by **42.72%**. If those sample shares represent the unprofiled workload, even
eliminating all copy and call-category cost would leave about **1.255×** the
opt0 wall time and **1.234×** its CPU time. This is an optimistic inference from
sampling, not a demonstrated lower bound or a promise that such elimination is
possible.

Register moves, frame traffic and other memory together are 29.67%; much of that
may be necessary work. Existing scalar allocation already carries some values
across blocks and calls. Closing the remaining gap requires broader investigation
of storage/register traffic or library/container work; this table does not prove
that a new register allocator is the only route. The scoped aggregate change
stops here, with the regressions, opt0 gap and opt3 horizon recorded rather than
opening another optimizer project.
