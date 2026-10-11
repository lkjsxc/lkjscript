# Compiler 33 controls for preserved predecessor meaning

These controls retain the exact canonical source and instructions from
`../requirement-predecessor/`, `../transaction-outcome-predecessor/` and
`../cell-participation-predecessor/`. Derived envelopes use Graph 33 compiler
meaning, compiler 33, bytecode 27 and artifact 40. Original source generations,
package identities, semantic revisions, artifacts and provenance remain frozen.
The compiler 32 controls remain unchanged and reject at their artifact rebuild cut.

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

These files were emitted by the source-matched compiler 33 test writer on
2026-10-11. No historical artifact or canonical source was overwritten.

| Control | SHA-256 |
| --- | --- |
| `requirements.lkja` | `3ce273959935acd5818482bdab27cedee9b1d4dcdb86f005ca9dfe09db49fc4c` |
| `transactions.lkja` | `01751d0372e738399989e15c3c6b8636506ad48640ecafcd75f4d1d2b2f12531` |
| `participation.lkja` | `2afb966c8f539cf6bf1b5eb55063b3ff82d1700d9267d32dea810f7696253988` |
