# Compiler 32 finite-callable rejection controls

These deliberately invalid artifacts retain the exact canonical source and
instructions from `../finite-callable-predecessor/expanding-{direct,named}.lkja`.
Derived envelopes use Graph 32 compiler meaning, compiler 32, bytecode 27 and
artifact 39. Original source generations, artifacts and provenance remain frozen.
The compiler 31 controls remain unchanged and reject at their artifact rebuild cut.

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

| Fixture | SHA-256 |
| --- | --- |
| `expanding-direct.lkja` | `a36298c0f8fc6c68d2f399fc84b3a2946d5fc36ac07883ddce05caa7f163be85` |
| `expanding-named.lkja` | `b6b0524ac6dc6ff9b380618222546069b7e1cff4e39d2a1ddf3021fd2922de09` |
