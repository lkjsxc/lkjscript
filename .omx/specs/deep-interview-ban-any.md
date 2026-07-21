# Deep Interview Spec: ban Any + parametric types + sized numerics

## Metadata
- Profile: standard; rounds: 3; final ambiguity ~0.12
- Context: `.omx/context/ban-any-20260721T074340Z.md`
- Push: authorized anytime gates green

## Intent
Eliminate `Any` as a language type; replace with precise parametric types and sized numerics.

## Desired Outcome
- `Any` rejected in source and removed from the type checker/prelude.
- Parametric polymorphism: `List T`, `Result T E`, generic fns — **annotation-driven** (explicit type params; no full HM).
- Sized numerics: at least `I32`, `I64`, `U32`, `U64`, `F32`, `F64` (Rust/Zig-inspired spellings).
- `print` is `Str -> Nil` (+ convert helpers); no traits/typeclasses in this cut.
- Corpus + honesty gates green; push to origin.

## In-Scope
1. Remove `Type::Any` / name `Any`.
2. Type-param syntax on `fn` (agent chooses slash spelling, e.g. `forall/ T /forall` or `sig/ [T] …`).
3. Prelude rewritten with polymorphic list/Result helpers and sized numeric ops (casts between widths explicit).
4. Migrate all `.lkjscript` off `Any`.
5. Runtime support sufficient for sized types (type-level enforcement + matching ops; widen/narrow via named casts).
6. Docs/ADR update; push.

## Out-of-Scope / Non-goals
- Full HM inference
- Traits/typeclasses (`Show`, `Num`)
- Baseline JIT productization
- Multi-OS backends

## Decision Boundaries (agent)
- Exact slash spelling for type params and type applications (`List T` vs `List/ T /List`).
- Exact cast helper names (`i64-from-i32`, etc.).
- Whether runtime stores all ints as i64 with checked casts vs distinct tags — prefer distinct tags/heap where needed without crates.io.

## Constraints
Prior A–E spirit; no third-party crates; unsafe only in sys; Docker/quiet/editor/http green.

## Acceptance
1. `rg '\bAny\b' src examples` → no type uses; checker rejects `Any`.
2. Polymorphic std list ops typecheck with concrete instantiations or annotated forall.
3. Mandel/editor/http use sized numerics / Str print path as needed.
4. quiet + editor + http + Docker green; pushed to origin.

## Transcript condensed
1. Parametric polymorphism.
2. Annotation-driven (not full HM).
3. A for print/numerics; plus bit-width types `i32`/`u64`/`f64` etc.
