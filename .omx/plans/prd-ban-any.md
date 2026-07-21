# PRD: Ban Any — parametric types + sized numerics

## RALPLAN-DR
### Principles
1. No escape hatch — `Any` is not a type.
2. Precision via parameters — `List T`, `Result T E`, generic fns.
3. Annotations required for polymorphism — no full HM this cut.
4. Sized numbers are first-class — `I32`/`I64`/`U32`/`U64`/`F32`/`F64`.
5. Keep A–E honesty and thin host.

### Drivers
1. User policy: ban Any; parametric; annotation-driven; sized widths; print=Str.
2. Less future rework than monomorphic-only.
3. Gates must stay green; push authorized.

### Options
- A monomorphic only — rejected (user chose parametric).
- B parametric + traits now — rejected (user chose A for print, no traits).
- C parametric annotation-driven + sized numerics + Str print (CHOSEN).

### Pre-mortem
1. Token budgets explode with forall/sig — retune limits / split files.
2. Runtime width bugs — typecheck casts; add unit tests for narrow/widen.
3. List T migration misses a call site — corpus-wide rg + typecheck gate.

## Phases
1. Type AST: remove Any; add sized nums; Param/App/Forall.
2. Surface: forall + type application parsing.
3. Checker: unify with params; reject Any.
4. Runtime: sized value ops + casts.
5. Prelude + corpus migration.
6. Docs; verify; push.
