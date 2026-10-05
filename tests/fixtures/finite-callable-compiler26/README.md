# Compiler 26 finite-callable rejection controls

These controls retain exact canonical source and instruction forms from
`../finite-callable-predecessor/expanding-{direct,named}.lkja`. Their derived
envelopes use compiler 26, bytecode 21, artifact 33 and compiler meaning contract
Graph 26. Original source generations remain unchanged.

The generator and byte-for-byte verification owner is
`platform::compiler::tests::effect_tests::strict_artifact_rejects_fully_rehashed_expanding_canonical_applications`.
It requires original format refusal, decodes frozen instructions, replaces only
derived containers and requires semantic `kernel_callable_expansion` rejection
before comparing complete bytes against retained controls. No live effects run.

Generate into this directory using its create-new `LKJSCRIPT_WRITE_FINITE_FIXTURES`
writer after building only the library-test executable. Expected outputs are
`expanding-direct.lkja` and `expanding-named.lkja`; each must be absent before
generation. Runtime file reads allow building the generator before these outputs
exist. A later developer-tool build embeds the exact rejection controls.

Generated on 2026-10-05 through the create-new writer above, using the retained
library-test executable `.artifacts/20261005-owned-contracts/lib-tests-02`, SHA-256
`9223fab8108dbec3bde17b7e4408fede3bbe7cd199a7c10ee5b77205671e067a`.
The complete passing log is
`.artifacts/20261005-owned-contracts/fixture-finite.log`; its exact selector ran
with `--exact --nocapture --test-threads=1`. Both regenerated controls reached the
expected `kernel_callable_expansion` refusal and matched the retained bytes.

| Fixture | SHA-256 |
| --- | --- |
| `expanding-direct.lkja` | `6d61f325267c1d111b2ea9bc405514af06c7e3f2837f6c5e9f740727dccbe8c4` |
| `expanding-named.lkja` | `0970a98d8b8646bbf133a1602115d621f27597ca28beff298e1c5cbaac3a52f0` |

The exact executable and output digest records are
`.artifacts/20261005-owned-contracts/lib-tests-02.sha256` and `fixtures.sha256`.
All predecessor originals and earlier compiler-24/25 controls remain unchanged.
