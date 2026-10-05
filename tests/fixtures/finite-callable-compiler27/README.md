# Compiler 27 finite-callable rejection controls

The controls retain exact canonical source and instruction forms from
`../finite-callable-predecessor/expanding-{direct,named}.lkja`. Their derived
envelopes use compiler 27, bytecode 22, artifact 34 and compiler meaning contract
Graph 27. Original source generations remain unchanged.

The generator and byte-for-byte verification owner is
`platform::compiler::tests::effect_tests::strict_artifact_rejects_fully_rehashed_expanding_canonical_applications`.
It requires original format refusal, decodes frozen instructions, replaces derived
containers and requires semantic `kernel_callable_expansion` rejection before
comparing complete bytes. No live effects run.

Generate into this directory using its create-new `LKJSCRIPT_WRITE_FINITE_FIXTURES`
writer after building the library-test executable. Expected outputs are
`expanding-direct.lkja` and `expanding-named.lkja`; each must be absent before
generation. Runtime file reads allow building the generator first. Later
developer-tool builds embed the exact rejection controls.

All predecessor originals and compiler-24/25/26 controls remain preserved.
