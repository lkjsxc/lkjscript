# Compiler 30 controls for preserved predecessor meaning

These controls retain the canonical source and instructions from
`../requirement-predecessor/`, `../transaction-outcome-predecessor/` and
`../cell-participation-predecessor/`. Derived envelopes use Graph 30 compiler
meaning, compiler 30, bytecode 25 and artifact 37. Original source generations,
package identities, semantic revisions, artifacts and provenance remain unchanged.
The compiler 29 controls remain frozen and reject at their artifact rebuild cut.

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

The developer tool embeds these controls for exact offline execution and original
source-transport checks. Historical `new-guard.lkja` retains its original
old-runtime preflight meaning and is not converted.

| Control | SHA-256 |
| --- | --- |
| `requirements.lkja` | `cff4ed9a44992b41c531488895473231e21ac6633f2334e9dae75eef939e028d` |
| `transactions.lkja` | `d8313beca664fe25f72c86f013cd9f30e9aefa77db53e440a9a829b69e3c288c` |
| `participation.lkja` | `7fee41e7cef7d095179abba60d1f78594ff1fefac3e98642a9662ab5da4cdf30` |
