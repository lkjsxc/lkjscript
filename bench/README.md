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
| src/main.rs | Complete self-build with explicit current library/runtime | Compiler artifact |
| aggregate64.rs | 10 million 64-byte records passed/returned through three call levels | All eight fields in recurrence checksum |
| vector64.rs | 1 million inline 64-byte records, then 10 million reads | All eight fields in sum |
| aggregate.rs | 20 million calls taking/returning a two-word Copy struct | Recurrence checksum |
| scalar.rs | 200 million multiply/add/xor iterations | Final word |
| scalar_add.rs | 200 million add/xor/mask iterations | Final word |
| buffers.rs | 200,000 fresh Vec<i64> buffers of 1,000 elements | Indexed checksum |
| vector.rs | 5 million Vec<i64> pushes, then 100 million reads | Sum |
| http_parse.rs | 200,000 changing HTTP/1.1 requests through the consumer parser | Header boundary, path byte/length, method, body length and persistence checksum |
| string_map.rs | 4,096 owned qualified String keys; 500,000 lookups with replacement/removal/reinsertion | Lookup and final ordered traversal checksum |

Counts and seeds are runtime arguments. Call-chain functions use inline(never),
which rustc honors. lkjscript IR has no source inline hints: use --inline-limit 0
for retained-call ABI comparisons; an enabled pass may remove those calls. Choose
thresholds using the compiler and the HTTP/String-map rows, which carry no hints.
Vector timings include construction. Buffer reuse and large vector remapping remain separate
workloads. Reports include elapsed/user/kernel time, peak RSS and executable size;
transient results and images stay beneath target/bench/. Each timing includes the
1/5/15-minute load averages read immediately before that run on the shared host.

The HTTP row stages the current `examples/http_protocol.rs` beside its native
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
target/lkjscript build src/main.rs --library-dir library \
    --runtime-source library/runtime.rs --profile --no-store -o target/bench/compiler-profile
rm -f target/bench/compiler-profile.samples.*
target/bench/compiler-profile build src/main.rs --library-dir library \
    --runtime-source library/runtime.rs --no-store -o target/bench/profile-output
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

The initial checker edit measured 20.241 s (external wall 20.25 s, load
3.85→4.45). Inference at 6.391 s and IR construction at 6.778 s were its two
largest phases. The guarded stable-span checker edit measured 7.791 s against
a 23.011 s cold build and 0.953 s unchanged build (one-minute load 9.42→9.06, 11.72→9.42 and 9.42→9.42).
It reconstructed one of 2,713 definitions and one of 10,071 concrete functions;
10,070 checks and 9,921 native fragments were reused. All cached images matched
the corresponding no-store images, and the edited compiler built and ran the
closed gate root with the same bytes. This remains above the 3 s target.
The former inference/construction costs are now 0.163/0.007 s; whole-image
identity and authenticated rereads remain 1.084/3.998 s, and checkpoint
restore/encode remains 0.547/0.130 s. These costs are explicit remaining work.

Cold storage overhead was 13.12% against the 20.35 s no-store control
(load 11.72→9.42 versus 13.46→11.72). The 2.667 s phase-accounted extra wall time is:

- Dependency and producer identities: 1.084 s; these bind real bytes rather than paths or mtimes.
- Authentication and store I/O: 1.352 s; large reads authenticate before sized allocation, then authenticate the reread.
- Fragment reuse bookkeeping and materialization: 0.658 s over ordinary instruction emission.
- Checkpoint encoding and other publication preparation: 0.158 s extra, including 0.153 s encoding.
- Other ordinary-phase variation: −0.585 s in this pair of observations; the falling host load affects the comparison, and timing is not a gate threshold.

Five published records replace the former per-function files. Measured store
bytes were 71,676,829 B after cold and 88,449,732 B after the edit, below the
explicit per-directory policy. The bound is on authenticated contents and its
index; filesystem metadata and transient atomic-publication files are not counted.

### Gate cost

The guarded-checkpoint gate measured below used four workers, the refreshed
3,242,241-byte seed and the source including Programs commit `2d50e71b` and
the expanded-include invalidation regression. The cold run began with an empty
compiler store; the warm run used the same source, seed and populated store.
Filesystem caches were not reset. Loads are the one-minute average before and
after each timed interval, not peak load.

The cold gate took **535.033 s (load 6.71→10.97)** and reran all 4,475 counted
case requests. The warm gate took **156.966 s (load 10.97→18.47)**, reusing 4,452
requests and rerunning 23. Those counts exclude the 762 structural contracts,
128 mutation comparisons and uncounted host/proof suites, which still run.
The 23 warm misses consist of 11 conformance requests under each inline policy
without a reusable observation proof and one deliberately no-store library
override fixture. Compile failures other than source I/O failures can have
authenticated verdict proofs; negative cases do not all rerun.

This is a same-producer warm measurement. Across a checker body edit, the
separate benchmark proves five closed contracts can skip execution after the
freshly rebuilt runner, image and authority bytes match. Ordinary compiler
verdicts remain producer-bound. The gate changes address completion-order
scheduling and fixed-output replay under the checked effect/completion rule;
host observations and resource availability are still tested afresh.

The following partition sums wall time without double-counting overlapping
suites. The first row comprises serial preparation plus the weighted host/proof
window and orchestration. Cold preparation was 59.118 s (load 6.71→10.45) and
the remaining window 85.406 s (load 10.45→11.13); warm preparation was 5.676 s
(load 10.97→10.41) and the remaining window 79.996 s (load 10.41→13.72).
The window is computed as the residual after serialized steps, so it also
includes scheduling and rounding overhead.

| Wall-clock group | Cold s (load 1m) | Warm s (load 1m) |
| --- | --- | --- |
| Preparation + overlapping host/proof suites | 144.524 (6.71→11.13) | 85.672 (10.97→13.72) |
| Conformance default / inline 64 | 217.358 (11.13→12.16 / 14.66→11.82) | 39.642 (13.72→12.47 / 12.47→11.76) |
| Generated default / inline 64 | 130.144 (12.16→14.66 / 11.82→11.61) | 1.529 (12.47→12.47 / 11.76→11.76) |
| Semantic structure | 22.772 (11.61→10.29) | 27.907 (11.76→18.47) |
| Semantic/reference cases + mutation | 20.235 (10.29→10.97) | 2.216 (18.47→18.47) |
| **Total** | **535.033 (6.71→10.97)** | **156.966 (10.97→18.47)** |

The cold 150 s goal remains unmet. The four corpus passes alone occupy
347.502 s (load 11.13→11.61) at four workers; cold construction, host proofs,
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
| bootstrap | 27.290 (6.71→7.61) | 2.514 (10.97→10.41) | — | Fresh seed bootstrap; authenticated build images may reuse. |
| independence | 0.002 (7.61→7.61) | 0.002 (10.41→10.41) | — | Fresh repository independence check. |
| tool-build | 27.808 (7.61→10.66) | 1.517 (10.41→10.41) | — | Fresh build requests; authenticated images may reuse. |
| closed-build | 3.927 (10.66→10.45) | 1.562 (10.41→10.41) | — | Fresh same-IR authority report and sealed image. |
| generate | 0.091 (10.45→10.45) | 0.081 (10.41→10.41) | — | Fresh deterministic corpus generation. |
| closed-semantics | 0.413 (10.45→10.45) | 0.037 (10.41→10.41) | 0/5 → 5/0 | Successful exact runner/ELF/contract proof; five cases. |
| closed-proof-attacks | 0.727 (10.45→10.45) | 0.654 (10.41→10.41) | — | Fresh authentication, identity and report-swap attacks. |
| toolchain | 61.567 (10.45→10.52) | 60.847 (10.41→12.17) | — | Fresh host and toolchain integration proofs. |
| world | 31.498 (10.45→10.65) | 28.821 (10.41→9.56) | — | Fresh host capability observations. |
| profiler | 1.226 (10.45→10.45) | 1.110 (10.41→10.41) | — | Fresh sampling and profile-format proofs. |
| optimizer | 9.199 (10.45→10.84) | 7.636 (10.41→10.12) | — | Fresh optimizer admission and comparison proofs. |
| resources | 0.889 (10.45→10.45) | 0.591 (10.41→10.14) | — | Fresh resource and refusal observations. |
| runner | 5.388 (10.45→10.65) | 4.140 (10.14→10.14) | — | Fresh runner structural checks. |
| diagnostic-cases | 7.226 (10.65→10.94) | 0.904 (10.14→10.12) | 0/35 → 34/1 | Warm: 34 verdict proofs; one library-override fixture forces no store. |
| store | 45.482 (10.94→10.52) | 45.002 (10.12→12.80) | — | Fresh store corruption, invalidation, bounds and refusal attacks. |
| program-directory | 5.219 (10.52→10.64) | 4.831 (12.80→12.17) | — | Fresh manifest, grants, environment and directory proofs. |
| http | 19.954 (10.52→11.14) | 18.734 (12.17→13.72) | — | Fresh HTTP server/client observations. |
| journal | 2.359 (10.64→11.07) | 2.253 (12.17→13.28) | — | Fresh journal persistence and HTTP observations. |
| stop | 3.188 (11.07→11.07) | 3.064 (13.28→13.28) | — | Fresh native shutdown observations. |
| stop-interpreter | 7.315 (11.07→11.14) | 6.630 (13.28→13.17) | — | Fresh interpreter shutdown observations. |
| site | 2.187 (11.14→11.14) | 2.577 (13.17→13.17) | — | Fresh offline site consumer proof. |
| desk | 4.453 (11.14→11.13) | 4.554 (13.17→13.72) | — | Fresh desk consumer proof. |
| conformance | 107.976 (11.13→12.16) | 19.509 (13.72→12.47) | 0/778 → 767/11 | Warm: 11 requests have no reusable observation proof. |
| generated | 65.424 (12.16→14.66) | 0.903 (12.47→12.47) | 0/1215 → 1215/0 | Warm: all fixed captures have authenticated proofs. |
| optimized-conformance | 109.382 (14.66→11.82) | 20.133 (12.47→11.76) | 0/778 → 767/11 | Same rule, independently keyed inline-limit 64. |
| optimized-generated | 64.720 (11.82→11.61) | 0.626 (11.76→11.76) | 0/1215 → 1215/0 | Same fixed-capture rule, independently keyed inline-limit 64. |
| semantic-structure | 22.772 (11.61→10.29) | 27.907 (11.76→18.47) | — | 762 fresh structural contracts in each gate. |
| semantic-cases | 16.609 (10.29→11.49) | 0.881 (18.47→18.47) | 0/436 → 436/0 | Warm: all exact compiler/input proofs reused. |
| references | 3.047 (11.49→10.97) | 0.641 (18.47→18.47) | 0/13 → 13/0 | Warm: all exact compiler/input proofs reused. |
| ir-mutation | 0.579 (10.97→10.97) | 0.694 (18.47→18.47) | — | 128 fresh mutation comparisons; zero findings in each gate. |
