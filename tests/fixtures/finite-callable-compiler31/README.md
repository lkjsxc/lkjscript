# Compiler 31 finite-callable rejection controls

These deliberately invalid artifacts retain the canonical source and instructions
from `../finite-callable-predecessor/expanding-{direct,named}.lkja`. Derived
envelopes use Graph 31 compiler meaning, compiler 31, bytecode 26 and artifact 38.
Original source generations, artifacts and provenance remain unchanged. Preserved
compiler 30 controls reject at their artifact rebuild cut.

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

The initial generation may fail the byte comparison against the previous golden;
that failure is not acceptance. Install the additive controls and rerun the test.
The developer tool embeds these artifacts for exact offline semantic-rejection
checks. An envelope refusal does not prove current semantic rejection.

| Fixture | SHA-256 |
| --- | --- |
| `expanding-direct.lkja` | `7f7f055735351d183d4a5f75d6b2b005493225193e8ae6dc556b46591cbfa01a` |
| `expanding-named.lkja` | `8f162d0248827c94caaed98b911e289a179739dd1ae10e9bd304e570480f1a50` |
