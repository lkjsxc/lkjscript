# Compiler 30 finite-callable rejection controls

These deliberately invalid artifacts retain the canonical source and instructions
from `../finite-callable-predecessor/expanding-{direct,named}.lkja`. Derived
envelopes use Graph 30 compiler meaning, compiler 30, bytecode 25 and artifact 37.
Original source generations, artifacts and provenance remain unchanged. Preserved
compiler 29 controls reject at their artifact rebuild cut.

The test owner is
`platform::compiler::tests::effect_tests::strict_artifact_rejects_fully_rehashed_expanding_canonical_applications`.
Its test-only frozen-instruction reader creates these containers without executing
their bodies. The independent current loader rejects both with
`semantic / kernel_callable_expansion`. The test compares their complete bytes
with these embedded controls and independently re-envelopes the frozen compiler
29 controls to verify unchanged source and instructions.

The source-matched library test executable generates create-new files when
`LKJSCRIPT_WRITE_FINITE_FIXTURES` names an existing empty directory:

```sh
LKJSCRIPT_WRITE_FINITE_FIXTURES=/absolute/empty-output \
  /absolute/source-matched-library-test-executable --exact \
  platform::compiler::tests::effect_tests::strict_artifact_rejects_fully_rehashed_expanding_canonical_applications \
  --nocapture
```

The developer tool embeds these artifacts for exact offline semantic-rejection
checks. It separately retains the original predecessor artifacts and their
`source / compiler_unit_contract` refusal. An envelope refusal does not prove
current semantic rejection.

| Fixture | SHA-256 |
| --- | --- |
| `expanding-direct.lkja` | `8c79c288f196afc4b799437b38a4af9f36c96aff1b7c3cd09a129e3ae19221ee` |
| `expanding-named.lkja` | `f749a3d8a683bc969caa16d6639cd0a2cff0cffc964e40956e2ef8bc00647082` |
