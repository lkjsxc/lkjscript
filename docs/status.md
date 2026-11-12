# Status

Generation two is a Rust-bootstrap compiler with a memory64 WebAssembly backend.
Compiler closure, a fixed point, an owned CLI and Rust/Node independence remain
unfinished. The current work follows dependencies and corpus evidence, not source
file order. Generation one remains frozen at `05a9326c`.

## Works

The implemented subset includes fixed-width checked arithmetic, structs/enums,
generic definitions checked under rigid assumptions, retained method selection,
modules/privacy, references, partial moves, custom Drop, range/named patterns,
source Option/Result/String/vector helpers and a sorted-vector BTreeMap. The K
checker independently validates every emitted instance. `language.md` is the
semantic reference. The encoding, name, JSON and K representation components run
from their actual source; these component results are not whole-compiler closure.

Direct calls/constants/tuple constructors use resolved declaration identities.
Lexical binding lookup is indexed with shadow restoration. Persistent live/written
facts make branch snapshots share untouched state; cleanup visits live slots.
The five measured families are now approximately linear over the measured sizes.
Ownership joins are still equality joins; permissiveness has not changed.

`run`/`test` execute Wasm in a worker with a memory-derived virtual stack reservation.
The public Rust compiler adapter runs the pure pipeline on a scoped thread. The
source compiler graph excludes resource discovery. The CLI forwards binary output
before reporting failure and its Node child dies if its parent is terminated.

## Current measurements

Commands and machine-readable records live under `tools/measurements/`.
The integrated gate passes 371 Cargo tests, 103 conformance cases, eight pending,
zero failures, and 200 seeded generated programs in 23.129 s. Seed 601: value
generator 92 equal values / eight equal traps; verdict generator 68 ownership
rejections / 27 equal values / one equal trap / four valid-but-rejected (two move,
two borrow). No mismatch, unsound acceptance, crash or timeout was observed.

Release `check`, three repeats; medians in milliseconds at 8k / 16k / 32k:

| Family | Before | After | After doubling ratios |
| --- | --- | --- | --- |
| Chained functions | 767.914 / 2378.453 / 9322.127 | 86.259 / 185.694 / 355.106 | 2.153 / 1.912 |
| If statements | 276.210 / 758.660 / 3002.844 | 154.867 / 307.720 / 632.369 | 1.987 / 2.055 |
| Match arms | 204.010 / 763.533 / 2760.186 | 32.104 / 63.784 / 130.346 | 1.987 / 2.044 |
| Boxes with borrows | 276.820 / 902.912 / 3131.064 | 161.640 / 325.019 / 664.204 | 2.011 / 2.044 |
| Vec operations | 158.290 / 438.653 / 1251.692 | 131.667 / 266.012 / 546.230 | 2.020 / 2.053 |

Initial after-change samples overlapped the baseline job. At 32k match arms,
10 ms sampled process-tree peak RSS fell from 8,255,180,800 to 84,647,936 bytes.
128k chained functions checked in 1522.621 ms (three repeats). One million source
functions built a 105,727,609-byte module in 13.602 s at 5.79 GB sampled peak RSS
(single observation); Node rejected its 1,000,073 functions at engine load.

Ordinary debug CLI non-tail recursion ran at depth 10,000,000 in 499.69 ms after
load, peak whole-process RSS 1,837,502,464 bytes. Its worker reserved
168,023,928,832 bytes of virtual address space, not committed memory. No next-depth
failure was sought. Routine tests cover direct/mutual depth 100k, recursive custom
Drop depth 30k, and deep/malformed compilation from a 2 MiB caller stack.

The adopted outside corpus reproduces idioms 5/30, ownership 18/30, and rejection
of all 16 unsound examples. These scores are not language completeness measures.

## Missing and old paths

- Elaboration still combines type inference, method selection, cleanup and K
  construction. Generic normalization retains some name scans. No authoritative
  general typed IR exists yet; concrete K is checked per instance.
- Persistent fact sharing currently uses Rust Rc; an owned shared-node mechanism
  remains a compiler-library dependency. Active carrier/provenance/tag maps still
  clone their live contents. The repair
  removes historical-local costs, not every wide-live-loan cost.
- Join equality, one-source views, external reference-bearing mutation and
  references in owned containers reject some valid programs.
- World output, recovery/census, late inference, patterns, question-mark, closures,
  borrowed iteration and text breadth remain next work. No owned K machine yet.
- Rust host/library packing, Node loader/tools and emitter allocation primitives
  remain bridges. The source map still uses a sorted vector rather than a tree.

## Bootstrap and next action

`cargo build --release`; `bash tools/check.sh` is the routine gate.
`lkjscript run file.rs --entry probe` runs on Node; `build -o file.wasm` emits;
`k` prints checked K; `check file.k` directly reaches the K checker.
`node tools/corpus.mjs target/release/lkjscript` runs independent examples.
`node tools/scale/run.mjs target/release/lkjscript` measures the five families.

Next: finish World-derived byte output and output/termination conformance, parser
recovery and census, and stage timing. Then
make unification authoritative for ordinary inference and evolve ownership with
the independent K machine. Do not resume the file-by-file component campaign.
