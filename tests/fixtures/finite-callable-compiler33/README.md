# Compiler 33 finite-callable rejection controls

These deliberately invalid artifacts retain the exact canonical source and
instructions from `../finite-callable-predecessor/expanding-{direct,named}.lkja`.
Derived envelopes use Graph 33 compiler meaning, compiler 33, bytecode 27 and
artifact 40. Original source generations, artifacts and provenance remain frozen.
The compiler 32 controls remain unchanged and reject at their artifact rebuild cut.

The test owner is
`platform::compiler::tests::effect_tests::strict_artifact_rejects_fully_rehashed_expanding_canonical_applications`.
Its test-only frozen-instruction reader creates these containers without executing
their bodies. The independent current loader rejects both with
`semantic / kernel_callable_expansion`. Complete current bytes are compared with
these embedded controls; an independent re-envelope of the compiler 29 controls
proves unchanged source and instructions.

Generate create-new files through the source-matched library test executable:

```sh
LKJSCRIPT_WRITE_FINITE_FIXTURES=/absolute/empty-output \
  /absolute/source-matched-library-test-executable --exact \
  platform::compiler::tests::effect_tests::strict_artifact_rejects_fully_rehashed_expanding_canonical_applications \
  --nocapture
```

The output directory must exist and be empty. Generation may fail the comparison
with the prior golden; that failure is not acceptance. Install additive controls
and rerun the source-matched test. An envelope refusal does not establish current
semantic rejection.

These files were emitted by the source-matched compiler 33 test writer on
2026-10-11. No historical artifact or canonical source was overwritten.

| Fixture | SHA-256 |
| --- | --- |
| `expanding-direct.lkja` | `8280a500d9e45503617056b2b22150468fccbacd5faf7d9983b8377dc7b7133d` |
| `expanding-named.lkja` | `d6a270adb2993d422c9039695e37ff0fe8900fc6def4632d8ae6ef919d76d93a` |
