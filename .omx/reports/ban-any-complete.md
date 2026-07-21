# ban-any complete

- `Any` removed; sized `I32`/`I64`/`U32`/`U64`/`F32`/`F64` (+ Int/Float aliases)
- Annotation-driven `forall`; List/Result parametric helpers
- `print` is Str-only; `str-from-i64` for numbers
- Float literals: source `.` → F64 (fixes mandel)
- Corpus + editor/HTTP typed; gates green

Gates: quiet verify, editor-smoke, http-smoke, docker verify — ok
