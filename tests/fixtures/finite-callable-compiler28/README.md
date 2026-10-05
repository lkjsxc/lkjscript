# Compiler 28 finite-callable rejection controls

These controls retain the exact canonical source and instructions from
`../finite-callable-predecessor/expanding-{direct,named}.lkja`. Their derived
envelopes use compiler 28, bytecode 23, artifact 35 and compiler meaning contract
Graph 28. Original source generations remain unchanged.

The generation and byte-comparison owner is
`platform::compiler::tests::effect_tests::strict_artifact_rejects_fully_rehashed_expanding_canonical_applications`.
It checks original format refusal, reconstructs frozen instructions, replaces only
derived containers and requires semantic `kernel_callable_expansion` rejection.
No live effects execute.

Set `LKJSCRIPT_WRITE_FINITE_FIXTURES` to this directory when invoking the
source-matched library test executable. Its create-new outputs are
`expanding-direct.lkja` and `expanding-named.lkja`; both must be absent before
generation. Subsequent runs compare complete bytes. Developer-tool builds embed
these controls for independent offline verification.

All predecessor originals and earlier compiler controls remain preserved.
