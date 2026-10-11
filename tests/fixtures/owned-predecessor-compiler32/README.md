# Compiler 32 controls for preserved predecessor meaning

These controls retain the exact canonical source and instructions from
`../requirement-predecessor/`, `../transaction-outcome-predecessor/` and
`../cell-participation-predecessor/`. Derived envelopes use Graph 32 compiler
meaning, compiler 32, bytecode 27 and artifact 39. Original source generations,
package identities, semantic revisions, artifacts and provenance remain frozen.
The compiler 31 controls remain unchanged and reject at their artifact rebuild cut.

The test owner is
`platform::compiler::tests::predecessor_attack_tests::current_predecessor_controls_retain_exact_source_and_instructions`.
It reconstructs the original immutable source transport, preserves frozen
instructions, and verifies strict artifact admission and artifact-to-source
binding. The complete current bytes are compared with these controls; an
independent re-envelope of the compiler 29 controls proves unchanged source and
instructions. Runtime metadata selection may change, but each selected canonical
record must already exist in the frozen source. No canonical owner is edited.

Generate create-new files through the source-matched library test executable:

```sh
LKJSCRIPT_WRITE_PREDECESSOR_FIXTURES=/absolute/empty-output \
  /absolute/source-matched-library-test-executable --exact \
  platform::compiler::tests::predecessor_attack_tests::current_predecessor_controls_retain_exact_source_and_instructions \
  --nocapture
```

The output directory must exist and be empty. Generation may fail the comparison
with the prior golden; that failure is not acceptance. Install additive controls
and rerun the source-matched test. Historical `new-guard.lkja` is not converted.

| Control | SHA-256 |
| --- | --- |
| `requirements.lkja` | `c3a03ba023c8bfd375cfe960c4e6be08bddb614a0b52eb296c3679f3eb5bf68f` |
| `transactions.lkja` | `eddceb729a2c559bc37ce1ef1ae4f477f4254b1be067f9d64cc29c77d75a2134` |
| `participation.lkja` | `3bc10226b4d3aa37c1b961103902cfbd2f82923c2560a1f8a6acc3d6f0e90c6f` |
