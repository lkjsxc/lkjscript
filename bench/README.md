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

The report has at most 37 lines: ten self functions, ten inclusive functions, and
ten exclusive operation categories. Each interrupted instruction contributes to
one category; frame walking still supplies independent inclusive function counts.
Allocator means checked allocator bodies and their runtime callees. Other sealed
runtime bodies and platform code are runtime. Ordinary call setup, entry and
return include ABI staging and stack guards, while aggregate copy sequences are
separate. Frame loads/stores outside those sequences are frame traffic; other
loads/stores, arithmetic/register operations, branches and checks are distinct.
Checks count comparisons and guarding/failure sequences, keeping useful arithmetic
in its own category. Unknown remains visible. Sampling and its address maps are
opt-in; ordinary artifacts contain neither. Cached profiled function fragments
preserve operation ranges and reproduce the same image bytes.

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
