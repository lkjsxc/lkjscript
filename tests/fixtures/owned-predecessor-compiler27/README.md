# Compiler 27 controls for preserved predecessor meaning

The three controls retain exact canonical source and instruction forms from
`../requirement-predecessor/`, `../transaction-outcome-predecessor/` and
`../cell-participation-predecessor/`. Their derived envelopes use compiler 27,
bytecode 22, artifact 34 and compiler meaning contract Graph 27. Original source
generations, package identities and semantic revisions remain unchanged.

The generator and byte-for-byte verification owner is
`platform::compiler::tests::predecessor_attack_tests::current_predecessor_controls_retain_exact_source_and_instructions`.
It uses independent original-transport reconstruction. Every derived runtime
record must already exist in the preserved source; the generator cannot synthesize
canonical owners. It strictly admits the generated controls and probes their
original source binding before comparing complete bytes.

Generate into this directory using its create-new
`LKJSCRIPT_WRITE_PREDECESSOR_FIXTURES` writer after building the library-test
executable. Expected outputs are `requirements.lkja`, `transactions.lkja` and
`participation.lkja`. Each file must be absent before generation. Runtime file
reads allow building the generator first. Later developer-tool builds embed these
exact controls for independent offline checks.

All predecessor originals and compiler-24/25/26 controls remain preserved.
