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
charging source, HIR, or SSA shape to a budget ledger. The lexer-token, children-per-form, and
top-level-forms validity quotas have been removed; generated coverage compiles and executes one
program with 64 helper declarations, 128 children in one `do`, and more than 1,000 tokens. Compile
metrics are observational phase timings and source-file counts only. Package manifests and
prepared-program identities likewise do not carry compiler-profile identity.

A Semantic Source service already exposes snapshots, stable node queries, typed holes,
diagnostics, transactions, and a local stdio session. It uses direct boundary-local byte and
request-count policy for untrusted framing and does not admit programs through compiler node/work
profiles. It currently mirrors the text-oriented source tree and the compiler still recompiles
from text, so it is a bootstrap editing service, not yet the intended semantic program authority.

## Tested platform

The broad local suite and native paths are exercised on Linux x86-64. Portable Rust components
may build elsewhere, but no other host or native target is currently claimed as tested.

## Known gaps

- The parser still imposes a nesting safety ceiling while recursive parsing and destruction remain;
  removing it requires the separate stack-safe depth cutover. Source-foundation per-file and
  aggregate byte ceilings and the source-unit ceiling also remain. HIR, ownership, memory-plan,
  SSA, and structural-value paths retain other arbitrary count or recursion ceilings. Compact
  executable bytecode operands and indexes retain width ceilings. These are follow-up validity and
  representation gaps, not host policy.
- Several user-controlled compiler trees and analyses remain recursive or have poor large-input
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
