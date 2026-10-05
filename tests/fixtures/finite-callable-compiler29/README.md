# Compiler 29 finite-callable rejection controls

These artifacts retain the canonical source and instructions from
`../finite-callable-predecessor/expanding-{direct,named}.lkja`. Only derived
envelopes advance to compiler 29, bytecode 24, artifact 36 and compiler meaning
contract Graph 29. Original source generations remain unchanged.

The byte-comparison owner is
`platform::compiler::tests::effect_tests::strict_artifact_rejects_fully_rehashed_expanding_canonical_applications`.
Its frozen-instruction reader produces the controls without semantic rewriting or
execution. Both current artifacts reject with semantic `kernel_callable_expansion`;
the preserved compiler 28 controls reject at their artifact rebuild cut.

The source-matched library test executable generates these create-new files when
`LKJSCRIPT_WRITE_FINITE_FIXTURES` names an empty destination directory. The test
compares complete artifact bytes with these embedded controls, which are also
embedded by the developer tool for offline verification. Earlier fixtures remain
unchanged.
