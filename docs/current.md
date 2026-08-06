# Current implementation

This document describes implemented behavior in the checkout. It is not a promise of backward
compatibility.

## User-visible capability

- `.lkjscript` is the only accepted source suffix. Its current one-marker-per-line notation is a
  bootstrap projection, not the permanent semantic schema.
- Packages and modules use checked manifests, lock data, deterministic import resolution, path
  containment, and cycle rejection.
- The implemented language includes typed functions and calls, local bindings and mutation,
  conditionals and loops, products, enums, exhaustive matching, generic `Option` and `Result`,
  numeric conversions, bytes, byte vectors, lists, typed errors, and explicit capabilities.
- The command-line runtime supports a default automatic path and diagnostic `vm`, `baseline-jit`,
  and `optimizing-jit` selections. Forced native selections preflight support and report failure
  rather than claiming generated execution after fallback.
- Host adapters cover standard I/O and selected filesystem, TCP, hashing, terminal, and SQLite
  operations behind typed capability checks.
- Ordinary runtime memory is collector-free. Unique storage, invocation regions, structural
  values, and explicit host-resource ownership are implemented for the supported value families.

The executable examples under `src/examples/` and compiler/runtime tests own the exact supported
surface.

## Compiler and runtime foundations

Current text compilation follows this path:

```text
line-oriented text and package files
    -> source tree and resolved typed HIR
    -> ownership, effect, and memory-plan checks
    -> independently verified and normalized SSA
    -> validated bytecode
    -> default VM/native execution path
```

Trusted compiler entry points compile directly without selecting a compiler resource profile or
charging source, HIR, or SSA shape to a budget ledger. The lexer-token, children-per-form,
top-level-form, source nesting, 16 MiB per-file source, 256 MiB aggregate-source, and 65,536
source-unit validity ceilings have been removed. The parser uses fallibly grown explicit frames;
source projection, identity flattening, formatting, module rewriting, clone, and destruction use
iterative work stacks. Remaining recursive expression analysis, HIR memory planning, and SSA
lowering call sites use localized repeatable heap-backed stack segments rather than a finite depth
admission rule. Trusted validation, loading, package analysis, and compilation select an explicit
unrestricted source-byte policy. Source files are read to EOF in checked, fallibly reserved chunks;
metadata is a capacity and change-detection hint, not admission. Ownership analysis no longer
pre-scans HIR merely to reject an aggregate expression count. HIR memory-plan expression work is
checked observational `u64` telemetry rather than admission. Generated coverage compiles, creates
a verified HIR memory plan, creates verified and normalized SSA, validates bytecode, executes
through the VM, and destroys a program with 20,000 nested `do` expressions on a 256 KiB native
thread stack. That fixture records exactly 20,001 memory-plan expressions, 20,003 entries, and
40,045 verifier steps. A nested product-match fixture reaches a physical marker depth above 50,
and malformed 8,192-deep mismatched and unclosed input produces deterministic diagnostics and
drops partial trees on the same small stack. Other generated coverage compiles and executes a
source 1,024 bytes beyond the former 16 MiB boundary and exercises the source authority with 65,537
in-memory units. Checked accounting crosses 256 MiB; the exact 258 MiB compile-and-execute geometry
is retained as an opt-in stress test and has not been run as part of normal verification. Compile
metrics are observational phase timings and source-file counts only. Package manifests and
prepared-program identities likewise do not carry compiler-profile identity.

A Semantic Source service already exposes snapshots, stable node queries, typed holes,
diagnostics, transactions, and a local stdio session. It supplies an explicit limited aggregate
source-byte policy at its untrusted boundary; the same policy checks staged transaction source
bytes before publication. It has no source-unit, token, node, or work admission quota. Other
boundary-local byte and request-count policies remain for untrusted framing and persistence. It
currently mirrors the text-oriented source tree and the compiler still recompiles from text, so it
is a bootstrap editing service, not yet the intended semantic program authority.

## Tested platform

The broad local suite and native paths are exercised on Linux x86-64. Portable Rust components
may build elsewhere, but no other host or native target is currently claimed as tested.

## Known gaps

- Source spans, positions, and snapshot-local node indexes remain `u32`, so an individual source
  or source tree beyond those addressable ranges fails at a representation boundary. HIR, SSA,
  recursive type/trait/enum, and structural-value paths retain other arbitrary count or recursion
  ceilings. HIR memory planning still has independently triggered quotas for functions, entries,
  uses, loans, constants, calls, obligations, type nodes and edges, witnesses, aggregate shape,
  destinations, borrow scopes, drop paths, and deterministic verifier/SCC work. The
  20,001-expression fixture does not cross those tables: it has one function, 20,003 entries, one
  constant and type
  fact, no uses, loans, calls, obligations, destinations, or borrow scopes, and 40,045 verifier
  steps. Compact executable bytecode operands and indexes retain width ceilings. These are
  follow-up validity and representation gaps, not host policy.
- Recursive compiler paths not exercised by the ordinary deep-expression production vertical,
  including parts of type, trait, enum, semantic-schema, and transaction processing, still need
  explicit work-stack conversion or equivalent evidence. Some analyses retain poor large-input
  complexity.
- The compiler cannot yet consume a syntax-independent semantic snapshot directly.
- Semantic edits still publish text files, and stable identity remains coupled to the current
  source representation in important paths.
- The evaluator, VM, baseline native path, and proof-oriented optimizing path still multiply the
  implementation surface. Their long-term roles have not yet been selected by representative
  measurement.
- Some host-resource cleanup obligations remain explicit rather than compiler-inserted on every
  implemented outcome.
- The daemon, process-cell, scheduler, and database foundations exist, but they are not required
  to validate the local language and compiler architecture.
