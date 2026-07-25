# Experiment Registry: Commands

[Authority](../experiments.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

### Commands

Commands were run from the worktree above. `measure_command.py` used Python
`monotonic_ns` plus Linux `getrusage(RUSAGE_CHILDREN).ru_maxrss` and checked the
child status.

```sh
cargo generate-lockfile --manifest-path target/backend-spike-owned/Cargo.toml
cargo generate-lockfile --manifest-path target/backend-spike-cranelift/Cargo.toml
cargo fetch --locked --manifest-path target/backend-spike-cranelift/Cargo.toml
cargo fmt --manifest-path target/backend-spike-owned/Cargo.toml
cargo fmt --manifest-path target/backend-spike-cranelift/Cargo.toml
cargo clean --manifest-path target/backend-spike-owned/Cargo.toml
cargo clean --manifest-path target/backend-spike-cranelift/Cargo.toml
python3 target/backend-spike-harness/measure_command.py \
  target/backend-spike-harness/build-owned.json \
  target/backend-spike-harness/build-owned.log \
  cargo build --release --locked --manifest-path target/backend-spike-owned/Cargo.toml
python3 target/backend-spike-harness/measure_command.py \
  target/backend-spike-harness/build-cranelift.json \
  target/backend-spike-harness/build-cranelift.log \
  cargo build --release --locked --manifest-path target/backend-spike-cranelift/Cargo.toml
python3 target/backend-spike-harness/run.py
python3 target/backend-spike-harness/measure_command.py \
  target/backend-spike-harness/rss-owned.json target/backend-spike-harness/rss-owned.log \
  target/backend-spike-owned/target/release/backend-spike-owned sample rss
python3 target/backend-spike-harness/measure_command.py \
  target/backend-spike-harness/rss-cranelift.json target/backend-spike-harness/rss-cranelift.log \
  target/backend-spike-cranelift/target/release/backend-spike-cranelift sample rss
target/backend-spike-owned/target/release/backend-spike-owned wx
target/backend-spike-cranelift/target/release/backend-spike-cranelift wx
target/backend-spike-owned/target/release/backend-spike-owned inspect \
  target/backend-spike-harness/owned.bin
target/backend-spike-cranelift/target/release/backend-spike-cranelift inspect \
  target/backend-spike-harness/cranelift
objdump -D -b binary -m i386:x86-64 -M intel \
  target/backend-spike-harness/owned.bin
objdump -D -b binary -m i386:x86-64 -M intel \
  target/backend-spike-harness/cranelift-kernel.bin
python3 target/backend-spike-harness/dependency_report.py
target/backend-spike-tools/bin/cargo-audit audit \
  --file target/backend-spike-cranelift/Cargo.lock --json
```
### Results

Times are milliseconds. Median absolute deviation is MAD; p95 is nearest-rank.
Generated execution is primary.

| Candidate | Metric | Median | MAD | p95 | Min | Max |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| owned | generated execution | 48.406374 | 0.540016 | 58.985271 | 47.657486 | 60.117770 |
| Cranelift 0.134.2 | generated execution | 119.422902 | 0.566505 | 136.792513 | 118.847981 | 142.511744 |
| owned | compile + finalize | 0.007164 | 0.000591 | 0.015850 | 0.005741 | 0.017463 |
| Cranelift 0.134.2 | compile + finalize | 0.625476 | 0.026470 | 0.706368 | 0.549693 | 1.029094 |
| Cranelift 0.134.2 | excluded ISA/module setup | 0.021520 | 0.002925 | 0.026299 | 0.012473 | 0.034845 |

Cranelift execution took 2.467 times the owned median; equivalently, the owned
candidate reduced elapsed generated-execution time 59.47%. That materially
exceeded the predeclared threshold and dispersion. Inspection showed Cranelift
spilled and reloaded the loop-carried F64 value on each iteration, while the
owned encoder kept it in a register. This explains this generated code, not all
Cranelift workloads. Owned compilation was 87.31 times faster, but execution,
not compilation, decided the selection.

| Cost | Owned | Cranelift 0.134.2 |
| --- | ---: | ---: |
| generated code | 394 B total | 472 B total (72 B helper, 400 B kernel) |
| retained metadata visible to spike | 0 B | at least 80 B; 2 relocations, 0 traps, 0 stack maps |
| stripped release binary | 347,768 B | 4,516,424 B |
| clean locked release build wall | 1.935595 s | 65.213507 s |
| build child peak RSS | 160,792 KiB | 1,241,904 KiB |
| one extra sample child peak RSS | 9,844 KiB | 9,836 KiB |
| Linux runtime-normal dependencies | 0 | 38 |
| Linux build-only dependencies | 0 | 5 |
| all-target locked third-party packages | 0 | 61 |

The metadata figure is a lower bound from public relocation, trap, and outer
stack-map records; it excludes module allocator/table overhead. Owned labels
were fixed before installation and retained no metadata in this scalar spike,
but the production code-object contract still requires relocations,
safepoints, stack maps, and source/outcome maps. Peak build RSS is the maximum
waited child reported by Linux `ru_maxrss`, not aggregate process-tree RSS. The
one runtime RSS observation is secondary and too coarse to establish equality.

Both W^X probes inspected `/proc/self/maps`. The owned mapping was `rw-p` during
emission and `r-xp` after `mprotect`; no RWX protection was requested. A wrapper
around Cranelift's real `SystemMemoryProvider` observed both executable
allocations as `rw-p` from allocation and `r-xp` after
`finalize_definitions`; no RWX mapping was observed. All 31 ordinary finalized
samples also reported `r-xp`. These probes inspect temporary unsafe spikes and
do not establish the future safe `lkjscript-sys` contract.
### Dependency, License, Security, And Maintenance Review

The owned crate had no third-party dependency and its temporary source had 19
lines containing the token `unsafe`; production pure encoding need not be
unsafe, while mapping/call unsafety must move behind `lkjscript-sys`. The
Cranelift manifest pinned 0.134.2 for `cranelift-codegen`, `frontend`, `jit`,
`module`, and `native`; codegen used only `std`, `unwind`, and `host-arch`.
Its lock SHA-256 was
`243c224bd4e07a49d4a5c8614d6121506a142e23d6b38d90243d739f0900fc04`.
Because code generation would run inside the JIT process, the 38 normal
packages are runtime product dependencies; the five build-only packages do not
ship as linked runtime code.

Current-target license declarations were all permissive: 19
`Apache-2.0 WITH LLVM-exception`, 16 `MIT OR Apache-2.0`, three
`Apache-2.0 OR MIT`, two `MIT`, and one each `Apache-2.0 / MIT`,
`MIT/Apache-2.0`, and `Zlib`. `cargo-audit 0.22.2` reported zero
vulnerabilities and no warnings against RustSec database commit
`1abf7a8c1822223a38e99f652bc232071c44a86d` (1,169 advisories, updated
2026-07-23). The source-token proxy found 2,836 `unsafe` occurrences on 2,820
lines in 204 `src/**/*.rs` files across 27 runtime packages, plus one occurrence
in one build-only package. This deliberately overcounts comments and nested
source tests and is not a semantic unsafe audit; it makes the dependency safety
surface visible rather than claiming audited blocks.

Cranelift is maintained in the Bytecode Alliance Wasmtime repository, has a
published 0.134.2 API and documentation, an Apache-2.0-with-LLVM-exception
license, and requires Rust 1.94.0, within this experiment's Rust 1.96.0. That
maturity reduces instruction-selection/register-allocation work but couples the
runtime to a large, rapidly versioned release train. The owned path avoids that
coupling while accepting all Linux x86-64 encoder, allocator, relocation, ABI,
and metadata maintenance in this repository. The direct product dependency
classification would require a separate upgrade/security process if Cranelift
were reconsidered.

Exact all-target locked third-party versions were:

`allocator-api2 0.2.21; anyhow 1.0.104; arbitrary 1.4.2; bitflags 1.3.2; bumpalo 3.20.3; cfg-if
1.0.4; cranelift-assembler-x64 0.134.2; cranelift-assembler-x64-meta 0.134.2; cranelift-bforest
0.134.2; cranelift-bitset 0.134.2; cranelift-codegen 0.134.2; cranelift-codegen-meta 0.134.2;
cranelift-codegen-shared 0.134.2; cranelift-control 0.134.2; cranelift-entity 0.134.2;
cranelift-frontend 0.134.2; cranelift-isle 0.134.2; cranelift-jit 0.134.2; cranelift-module 0.134.2;
cranelift-native 0.134.2; cranelift-srcgen 0.134.2; equivalent 1.0.2; fnv 1.0.7; foldhash 0.2.0;
gimli 0.33.0; hashbrown 0.16.1; hashbrown 0.17.1; heck 0.5.0; indexmap 2.14.0; libc 0.2.189; libm
0.2.16; log 0.4.33; mach2 0.4.3; memmap2 0.9.11; proc-macro2 1.0.107; quote 1.0.47; regalloc2
0.15.2; region 3.0.2; rustc-hash 2.1.3; serde 1.0.229; serde_core 1.0.229; serde_derive 1.0.229;
smallvec 1.15.2; stable_deref_trait 1.2.1; syn 3.0.3; target-lexicon 0.13.5; unicode-ident 1.0.24;
wasmtime-internal-core 47.0.2; wasmtime-internal-jit-icache-coherence 47.0.2; windows-link 0.2.1;
windows-sys 0.52.0; windows-sys 0.61.2; windows-targets 0.52.6; windows_aarch64_gnullvm 0.52.6;
windows_aarch64_msvc 0.52.6; windows_i686_gnu 0.52.6; windows_i686_gnullvm 0.52.6; windows_i686_msvc
0.52.6; windows_x86_64_gnu 0.52.6; windows_x86_64_gnullvm 0.52.6; windows_x86_64_msvc 0.52.6`.
### Safepoints, Integration, And Decision

The scalar kernel had no GC references, so zero stack maps were correct.
Cranelift 0.134.2 exposes user stack maps at non-tail calls and frontend marking
that spills producer-identified live values, but its own documentation says the
CLIF producer remains responsible for identifying GC-managed values; a loop
backedge needs an explicit call/poll or additional support. The owned emitter
must expose exact register/spill/frame layout and generate repository-owned maps
itself. Allocation-free code may arrive first, and allocation-capable code
remains rejected until exact-map tests pass. Thus Cranelift has the easier fit,
but neither spike implements production GC integration.

The selected owned backend will consume verified typed SSA and return an
uninstalled byte image plus typed relocations and exact code-object metadata.
Executable memory remains a separate `lkjscript-sys` responsibility. This
boundary lets a later measured replacement remove the encoder without changing
SSA semantics, native/runtime ABI identities, tier state, or code-object
ownership. Cranelift is rejected for this production baseline selection but
conditionally retained if broad completed-SSA evidence reverses the runtime
result or the owned implementation fails correctness/maintenance gates.

The owned emitter is adopted by
[Linux x86-64 Native Backend](../../decisions/execution/linux-x86-64-native-backend.md).
No production backend or JIT is implemented by this experiment/decision commit.
A current-VM comparison was not practical: the repository has no typed SSA or
native ABI yet, and a separately authored source program would not isolate this
backend decision.
### Every Retained Sample

Values are exact nanoseconds in ordinal order. O/C means owned then Cranelift;
C/O means the reverse. Warmups are correctness/order records, not retained
timing samples.

- owned compile: `7695, 7855, 17463, 7734, 7494, 15850, 7384, 6221, 6953, 7193, 6382, 6863, 6692, 7735, 7164, 7915, 6161, 6432, 6572, 7474, 6983, 7154, 6573, 6211, 6873, 7454, 5741, 10530, 14948, 6503, 8666` <!-- LKJ-EXACT-DATA -->
- owned execution: `48089328, 58985271, 48476085, 48461037, 48154932, 48517934, 51366099, 48434998, 49007835, 47841002, 48406374, 48960426, 48028894, 48946390, 51698964, 60117770, 47684418, 47657486, 47869666, 48190459, 47816817, 48074000, 48121699, 49054714, 47851061, 47785297, 48490032, 48388461, 50507184, 47703113, 48508787` <!-- LKJ-EXACT-DATA -->
- Cranelift compile: `685418, 706368, 681331, 629995, 658067, 688314, 615658, 681311, 1029094, 617060, 658958, 568178, 647908, 583978, 569560, 700857, 615708, 618042, 626598, 610888, 602573, 613333, 592884, 585421, 610668, 625476, 634593, 549693, 620647, 626097, 651946` <!-- LKJ-EXACT-DATA -->
- Cranelift execution: `119585618, 121801292, 122019793, 119126464, 118847981, 118965883, 120353151, 142511744, 118852309, 118987834, 123994506, 119137916, 122609542, 119076110, 120690314, 127713116, 118856397, 120257812, 119130071, 119002422, 120184784, 119355935, 119079937, 118959922, 118940745, 119422902, 119862789, 136792513, 120465702, 119203208, 120386172` <!-- LKJ-EXACT-DATA -->
- Cranelift excluded setup: `22713, 22862, 24136, 12473, 22873, 24126, 15569, 26299, 34845, 18435, 25127, 16361, 20859, 17984, 13204, 25568, 22022, 18595, 21520, 20228, 19417, 23884, 14177, 17964, 17713, 19737, 26089, 12934, 23344, 24015, 22713` <!-- LKJ-EXACT-DATA -->
- retained pair order: `O/C, O/C, O/C, C/O, C/O, O/C, C/O, O/C, C/O, C/O, O/C, O/C, O/C, O/C, C/O, C/O, O/C, O/C, O/C, O/C, O/C, O/C, C/O, C/O, C/O, C/O, O/C, C/O, C/O, C/O, C/O` <!-- LKJ-EXACT-DATA -->
- warmup pair order: `O/C, C/O, C/O, O/C, O/C, C/O, C/O, O/C`
