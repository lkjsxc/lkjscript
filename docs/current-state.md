# Current State

## Purpose

State implemented behavior, evidence boundaries, known defects, and accepted
next work without mixing them with long-term vision.

## Status

**Current** for the implementation section. Repairs and future products are
explicitly labeled **Accepted Target**, **Placeholder**, **Deferred**, or
**Rejected**.

## Current Implementation

- Repository: `https://github.com/lkjsxc/lkjscript`
- Canonical source: `.lkjscript`; other extensions are rejected without shims
- Corpus: 94 language files under `src`; nine executable roots cover the exact corpus closure
- Physical format: one column-one marker/atom per line with matched markers and
  raw `str/`, `name/`, and `import/` blocks
- Source limits: depth 8, form children 16, tokens 384, top-level forms 8,
  product fields 15, and 16 combined immediate files/directories per source
  directory
- Source-tree scope: the width rule applies to language source directories,
  not Rust, docs, metadata, `.git`, or generated Cargo output
- Imports: contained `std/`, `lib/`, `examples/`, and `./` paths with installed
  fallback through `LKJSCRIPT_ROOT`; absolute, parent, wrong-extension, cycle,
  and canonicalized symlink escapes fail
- Compiler boundary: one analysis pass collects immutable headers and produces
  owned, resolved typed HIR with explicit Main and Functions, BindingIds,
  local-slot references, MutableLocal/SetLocal, ProductIds, numeric field
  identities, source origins, exact operation/type facts, and conservative
  effects; bytecode consumes HIR without re-resolving source names or
  declarations
- Host implementation: six Rust workspace crates with no third-party Rust
  dependencies; unsafe Rust is confined to `lkjscript-sys`
- Quality gate: the complete Rust workspace is rustfmt-clean and passes strict
  Clippy for all targets/features; docs status/links, explicit `PLACEHOLDER`
  labels, and exact source-closure coverage are machine-checked
- Runtime: dense bytecode, contiguous stacks, precise non-moving mark-sweep,
  traced immutable product objects, and return-adjacent tail-frame reuse
- Semantics: executable roots have exactly one no-parameter typed main;
  imports contain declarations only; top-level `do` and runtime value defs are
  removed; `var` introduces one exactly typed mutable local and local-only
  `set` returns Unit; Unit, typed empty-list, and Option none have distinct
  singleton tags, while Option some is traced; `nil`, `Nil`, `nil?`, and
  `null?` are removed; `arg` returns `Option Str`; universal `eq`/`ne` are
  removed in favor of exact value, object-identity, bounded structural-list,
  and F64-bit equality families; nominal products have ordered named fields,
  exact construction, access, and immutable replacement
- Numerics: canonical I64/F64 only; complete I64 uses signed 61-bit immediates
  plus boxed wide values, F64 remains distinct, arithmetic/comparison is
  checked or IEEE as declared, and narrower host domains reject truncation
- CLI: `run`, real bytecode `disasm`, help, and version; the unlabeled REPL stub
  was removed
- Workloads: hello, native lkjscript Mandelbrot, Brainfuck interpreted by
  lkjscript, lkjedit, one-shot HTTP, and Leibniz comparison; Brainfuck,
  terminal, and editor state is passed explicitly in immutable nominal products
  and evolved through local vars
- Resource handles: integers are rejected, stdin uses a reserved borrowed token,
  owned file/socket tokens are monotonic, and closed tokens are never reused
- Terminal ABI: arbitrary ioctl is absent; fixed `sys-tty-get`/`sys-tty-set`
  operations validate the exact 60-byte Linux state before FFI and return Results
- System Results: open, path existence, close/read/write, `isatty`, time,
  socket, poll, terminal, and terminal-guard failures return operation-qualified
  `ResultErr` values; standard wrappers unwrap explicitly
- Canonical resource names: `stdin-handle`, `sys-close`, `sys-read-byte`,
  `sys-write-byte`, and `sys-isatty`; descriptor-era aliases are absent
- Send behavior: successful `sys-send` reports its byte count and uses Linux
  `MSG_NOSIGNAL` instead of risking process termination on a broken peer
- JIT seam: explicitly labeled **PLACEHOLDER** observation hook; there is no
  native compilation, engine selector, code object, execution handoff, OSR, or
  deoptimization
- Native backend decision: a measured called-code experiment selected a future
  owned Linux x86-64 byte encoder over Cranelift 0.134.2; this is an
  **Accepted Target**, not an implemented backend or product dependency
- Adaptive-performance contract: runtime JIT is the **Accepted Target** after
  semantic/outcome and typed-SSA prerequisites; the VM remains the cold tier
  and oracle, and minimal AOT emission is only a shared-backend test surface

## Known Defects

The source identity cutover does not make the runtime semantically complete.
The highest-priority defects are:

1. user-call effects are a safe all-effects over-approximation rather than
   fixed-point summaries;
2. strings and IO lack a lossless bulk byte contract, and some library file
   operations are per-byte or quadratic;
3. public malformed chunks are not prevalidated, although stack underflow,
   uninitialized slots, bad slot indexes, removed opcodes, non-Bool control,
   and malformed product metadata/descriptor/category/identity boundaries
   return VM errors instead of semantic fallback values;
4. source/import aggregate bytes, depth, count, constants, internal function
   slots, bytecode, VM fuel, heap, handles, output, and wall time are not
   comprehensively bounded;
5. process exit and terminal restoration remain process-global, and monotonic
   handle metadata remains until the VM ends.

## Evidence

Phase A implementation commit
`12836da90d886c9e741a5ac9f8148a17d00f0505`, based on `e4c1d0e`, was checked
on Linux x86-64 with Rust/Cargo 1.96.0. Evidence is command-specific; Docker, full Brainfuck
Mandelbrot, and performance are not implied.

| Command or check | Result |
| --- | --- |
| `cargo check --workspace --all-targets --locked` | passed |
| focused `lkjscript-compiler` and app HIR/numeric tests | passed; 37 compiler and 10 app integration tests |
| `cargo run --locked -p lkjscript-xtask --quiet -- quiet verify` | passed; docs, tree, exact source closure, rustfmt, strict Clippy, and 82 workspace tests |
| `check-sources` | passed for all 94 `.lkjscript` sources; the nine compiled executable closures equal the corpus exactly |
| HIR/local mutation conformance | explicit Main/Function, missing/duplicate/imported main, declaration-only imports, rejected top-level effects/value defs, stable BindingId/local-slot shadowing, initializer scope, local-only set rejection and exact typing, same-function isolation, ProductId/field resolution, and StoreLocal execution passed |
| `cargo build --workspace --release --locked` | passed |
| canonical hello | passed; output `3628800` |
| Mandelbrot | passed; 1,176 bytes, 24 lines, SHA-256 `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907` |
| Brainfuck smoke | direct and run-folded correctness/failure boundaries passed |
| lkjedit smoke | passed; existing-file insert/save/reopen, missing-file creation, CRLF redraw, and command paint |
| one-shot HTTP smoke | passed |
| native-backend decision spike | 8 randomized warmups plus 31 retained pairs; exact generated calls passed; owned execution median/MAD 48.406374/0.540016 ms versus Cranelift 0.134.2 119.422902/0.566505 ms; temporary artifacts removed; no production backend implemented |
| Phase A `check-docs` and `git diff --check` | passed |

Earlier decision-grade and diagnostic performance records remain in
[Experiment Registry](vision/experiments.md); they were not rerun for Phase A.
A gate that did not run did not pass.

## Accepted Next Target

The active engineering cycle must reach a real callable baseline JIT on Linux
x86-64; documentation, SSA scaffolding, machine-code emission, disassembly, or
an observation hook alone cannot complete it. The dependency sequence is:

Phase A is Current: explicit typed main, declaration-only imports, local-only
mutation, removal of source runtime globals, and product-threaded lkjedit,
terminal, and Brainfuck state are implemented. The remaining dependency
sequence is:

1. infer deterministic fixed-point function effects, validate complete chunks
   before execution, and replace process termination/string-only runtime errors
   with structured process-safe outcomes and explicit runtime limits;
2. lower HIR through verified typed SSA, an independent differential evaluator,
   and isolated non-speculative normalization; cut reference bytecode over to
   SSA before the native backend becomes authoritative;
3. implement the selected owned Linux x86-64 emitter with versioned SysV
   AMD64/runtime ABIs, bounded code objects, W^X memory in `lkjscript-sys`,
   exact stack-map gates, VM/native and native/native calls, and forced native
   execution;
4. expose truthful `vm`, `auto`, and `baseline-jit` modes only after generated
   code has been called. Forced mode never falls back; auto uses synchronous
   function-entry tiering and may remain in the VM for unsupported code.

OSR, background compilation, optimizing JIT, guards, deoptimization, persistent
profiles/caches, offline PGO, and non-Linux/non-x86-64 acceptance are outside
this cycle. The exact syntax, validation, outcome, SSA, backend-selection, ABI,
engine, safety, and evidence contract is
[Callable Linux x86-64 Baseline JIT Cycle](decisions/callable-baseline-jit.md).

The supporting contracts are [AI-First Semantic Core](decisions/semantic-core.md),
[Explicit Equality Families](decisions/equality-families.md),
[Immutable Nominal Products](decisions/immutable-nominal-products.md),
[Linux x86-64 Native Backend](decisions/linux-x86-64-native-backend.md),
[Typed Compiler Pipeline And Runtime JIT](decisions/compiler-pipeline.md),
[Runtime JIT Instead of Offline PGO](decisions/runtime-jit-instead-of-offline-pgo.md),
and the [Performance Scorecard](vision/performance-scorecard.md).

## Rejected

Offline PGO, instrumented training builds, profile generation/merging/use, and
persistent PGO artifacts are rejected by product decision, not measurement.
Persistent cross-run JIT profiles and native-code caches are not planned without
a later explicit decision. Current-process bounded JIT counters are local,
ephemeral, and not telemetry.

## Deferred

Native installation and update, package manifests/locks/registry,
supervisor/scheduler, adaptive or generational GC, background JIT compilation,
guarded runtime specialization/deoptimization, non-Linux native backends,
browser, general HTTP server/framework, and GUI runtime are later cycles.
Their documents are designs or experiments, not capability claims.
