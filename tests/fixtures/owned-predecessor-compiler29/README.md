# Compiler 29 controls for preserved predecessor meaning

These artifacts retain the canonical source and instructions from
`../requirement-predecessor/`, `../transaction-outcome-predecessor/` and
`../cell-participation-predecessor/`. Only derived envelopes advance to compiler
29, bytecode 24, artifact 36 and compiler meaning contract Graph 29. Original source
generations, package identities and semantic revisions remain unchanged.

The byte-comparison owner is
`platform::compiler::tests::predecessor_attack_tests::current_predecessor_controls_retain_exact_source_and_instructions`.
It independently reconstructs the original transported source, retains frozen
instructions and verifies strict artifact-to-source binding. No canonical owners
are generated or edited. Preserved compiler 28 controls remain refused at their
artifact rebuild cut.

The source-matched library test executable generates `requirements.lkja`,
`transactions.lkja` and `participation.lkja` as create-new files when
`LKJSCRIPT_WRITE_PREDECESSOR_FIXTURES` names an empty destination directory. The
test compares complete artifact bytes with these embedded controls; the developer
tool embeds them for offline verification. Earlier fixtures remain unchanged.
