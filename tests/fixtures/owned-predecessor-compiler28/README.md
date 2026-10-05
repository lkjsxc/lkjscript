# Compiler 28 controls for preserved predecessor meaning

These controls retain the exact canonical source and instructions from
`../requirement-predecessor/`, `../transaction-outcome-predecessor/` and
`../cell-participation-predecessor/`. Their derived envelopes use compiler 28,
bytecode 23, artifact 35 and compiler meaning contract Graph 28. Original source
generations, package identities and semantic revisions remain unchanged.

The generation and byte-comparison owner is
`platform::compiler::tests::predecessor_attack_tests::current_predecessor_controls_retain_exact_source_and_instructions`.
It reconstructs the original transported source independently, admits the new
controls and verifies their source binding. It does not generate canonical owners.

Set `LKJSCRIPT_WRITE_PREDECESSOR_FIXTURES` to this directory when invoking the
source-matched library test executable. Its create-new outputs are
`requirements.lkja`, `transactions.lkja` and `participation.lkja`; all must be
absent before generation. Subsequent runs compare complete bytes. Developer-tool
builds embed these controls for independent offline verification.

All predecessor originals and earlier compiler controls remain preserved.
