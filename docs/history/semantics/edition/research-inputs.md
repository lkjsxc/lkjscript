# Edition 2: Research Inputs

[Authority](../edition.md)

## Purpose

Record the supplied research inputs and distinguish adopted mechanisms,
experiments, and rejected claims from lkjscript capability status.

## Status

**Accepted Target research record, not Current.** External mechanisms are design
inputs, not implementation evidence.

## Typed Incomplete Programs

Inputs:

- Blinn et al., *Statically Contextualizing Large Language Models with Typed
  Holes*, [arXiv:2409.00921](https://arxiv.org/html/2409.00921v1);
- Mündler et al., *Type-Constrained Code Generation with Language Models*,
  [arXiv:2504.09246](https://arxiv.org/abs/2504.09246);
- Hazel's meaningful incomplete-program and typed-hole work; and
- the Current Semantic Source, repository context, and one-shot protocol.

Adopt exact compiler-produced expected types, visible bindings, obligations,
and iterative validation under explicit candidate/work/output bounds. Candidate
lists report unsupported coverage and truncation. Full synthesis, trusted
client type facts, and silent full-language token masking are rejected.

## Patterns And Usefulness

Inputs:

- Luc Maranget, *Warnings for Pattern Matching*,
  [DOI 10.1017/S0956796807006223](https://doi.org/10.1017/S0956796807006223);
- the Rust compiler's
  [usefulness architecture](https://rustc-dev-guide.rust-lang.org/pat-exhaustive-checking.html);
- the Rust [pattern reference](https://doc.rust-lang.org/reference/patterns.html);
  and
- OCaml constructor spaces, redundancy, witnesses, and refutation handling.

Adopt constructor-space pattern matrices, specialization/default matrices,
usefulness, and bounded deterministic witnesses. Ad-hoc top-level variant-set
subtraction, assumed exhaustiveness after exhaustion, and backend-specific match
logic are rejected.

## Modes, Borrowing, And Memory

Inputs:

- *Oxidizing OCaml with Modal Memory Management*,
  [DOI 10.1145/3674642](https://doi.org/10.1145/3674642);
- *Data Race Freedom à la Mode*,
  [DOI 10.1145/3704859](https://doi.org/10.1145/3704859);
- *Fully-Automatic Type Inference for Borrows with Lifetimes*,
  [DOI 10.1145/3798221](https://doi.org/10.1145/3798221);
- *Pure Borrow*, [DOI 10.1145/3808259](https://doi.org/10.1145/3808259);
- Rust's supplied 2026 Borrow Checker Within and Polonius Alpha roadmap;
- *Reference Capabilities for Flexible Memory Management*,
  [DOI 10.1145/3622846](https://doi.org/10.1145/3622846); and
- the supplied 2026 Mode Crossing work and artifact.

Deep structural locality, uniqueness, portability, contention, place-lifetime,
view, region-isolation, and fallback facts are experimental future inputs. This
cycle preserves IDs and structural propagation points but does not claim a mode
system or weaken the Current ownership safe island.

## Effects And Capabilities

Inputs are *Rows and Capabilities as Modal Effects* (PACMPL POPL 2026) and
*Zero-Overhead Lexical Effect Handlers*,
[DOI 10.1145/3763177](https://doi.org/10.1145/3763177).

Adopt separation of observable effect facts, authority, and physical runtime
facts. Lexical capabilities and type-directed zero-mainline-overhead lowering
remain later inputs. General effect handlers and exception unwinding are
rejected for this cycle; typed `Result`, `Trap`, `Never`, and future capability
facts remain distinct.

## Representation And Layout

Inputs are current OxCaml documentation for
[unboxed layouts](https://oxcaml.org/documentation/unboxed-types/01-intro/),
[`or_null`](https://oxcaml.org/documentation/unboxed-types/02-or-null/),
[kinds](https://oxcaml.org/documentation/kinds/intro/),
[stack allocation](https://oxcaml.org/documentation/stack-allocation/intro/),
and [SIMD](https://oxcaml.org/documentation/simd/intro/).

Adopt semantic/layout separation, compiler-derived trace and niche facts, and
verified representation plans. Source-observable null, tags, pointer ABI,
unproved zero-allocation, and a target-specific source meaning are rejected.
Niches, unboxing, stack placement, modes, and SIMD are experimental until their
own vertical slices pass.
