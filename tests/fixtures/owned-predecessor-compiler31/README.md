# Compiler 31 controls for preserved predecessor meaning

These controls retain the canonical source and instructions from
`../requirement-predecessor/`, `../transaction-outcome-predecessor/` and
`../cell-participation-predecessor/`. Derived envelopes use Graph 31 compiler
meaning, compiler 31, bytecode 26 and artifact 38. Original source generations,
package identities, semantic revisions, artifacts and provenance remain unchanged.
The compiler 30 controls remain frozen and reject at their artifact rebuild cut.

The test owner is
`platform::compiler::tests::predecessor_attack_tests::current_predecessor_controls_retain_exact_source_and_instructions`.
It reconstructs the exact original source transport, retains frozen instructions,
and verifies strict artifact admission and artifact-to-source binding. It compares
the complete current bytes with these controls and independently re-envelopes the
preserved compiler 29 controls to check unchanged source and instructions. Runtime
metadata selection may change; each selected canonical record must already exist
in the frozen source. No canonical owner is generated or edited.

The source-matched library test executable generates create-new files when
`LKJSCRIPT_WRITE_PREDECESSOR_FIXTURES` names an existing empty directory:

```sh
LKJSCRIPT_WRITE_PREDECESSOR_FIXTURES=/absolute/empty-output \
  /absolute/source-matched-library-test-executable --exact \
  platform::compiler::tests::predecessor_attack_tests::current_predecessor_controls_retain_exact_source_and_instructions \
  --nocapture
```

The initial generation may fail the byte comparison against the previous golden;
that failure is not acceptance. Install the additive controls and rerun the test.
The developer tool embeds these controls for exact offline execution and original
source-transport checks. Historical `new-guard.lkja` retains its original
old-runtime preflight meaning and is not converted.

| Control | SHA-256 |
| --- | --- |
| `requirements.lkja` | `5a0e1d6a0554166f87bef79ae1a6834df266f9321b0ff6efd32d9902518e8f08` |
| `transactions.lkja` | `b3de7f0297205b3da837eb71f5aec4002ed7065251c426cd3f869ebe3d41091a` |
| `participation.lkja` | `1f2c624fc257f04fd76d78120f2973a256d1a2f10757b351c69c318f5f769279` |
