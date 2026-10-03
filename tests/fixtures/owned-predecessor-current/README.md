# Current controls for preserved predecessor meaning

These three fixtures are derived controls, not republished official artifacts.
Their unchanged originals and provenance remain under `../requirement-predecessor/`,
`../transaction-outcome-predecessor/` and `../cell-participation-predecessor/`.

Current derivation uses compiler 23 / bytecode 18 / artifact 30 under semantic
validator 28 and requires rebuilding predecessor compiler units. Original canonical
source generations remain unchanged; Graph 23 is the compiler's current meaning
contract. The offline workflow therefore distinguishes exact original format
refusal from execution of the same canonical source with current derived envelopes. It keeps all public
rebuild/import paths and independent output, store, authority and cleanup checks.

The test-only owner is
`platform::compiler::tests::predecessor_attack_tests::current_predecessor_controls_retain_exact_source_and_instructions`.
It first checks the exact original format rejection. The existing frozen readers
then decode the original instructions without changing their operands or order.
Only derived unit keys, envelope generations, compilation maps, runtime metadata
selection and enclosing identities change. Every runtime metadata record must
already exist in the frozen source/runtime inventory; no canonical owner is
synthesized or edited. Serialized instruction tags differ between generations;
preserving instruction forms does not mean preserving their old serialized bytes.

Each new control must independently pass strict current artifact admission and
`strict_artifact_source_probe` against its exact original source transport. The
test reconstructs and compares the complete retained control bytes. Public rebuilds
may project a new interface/package revision from the same accepted source, so their
execution must retain the original package and semantic revision but need not retain
the old derived package-revision digest. Imported wrapper roots have their own source
identity. Exact artifact identity, original source preservation and all output/store
assertions remain mandatory in every corresponding workflow. The normal
product loader does not contain an old-artifact converter or an admission bypass.

| Control | Original artifact | Original rejection |
| --- | --- | --- |
| `requirements.lkja` | requirement-predecessor / generation 18 | source / compiler_unit_contract |
| `transactions.lkja` | transaction-outcome-predecessor / generation 19 | source / compiler_unit_contract |
| `participation.lkja` | cell-participation-predecessor / generation 21 | source / artifact_bundle_contract |

The historical `new-guard.lkja` is deliberately not converted: it belongs to an
official old-runtime before-secret rejection observation, not a current execution
control. Its reader retains exact bytes, provenance, diagnostic and exit status.

Generation uses a caller-selected empty directory and create-new writes:

```sh
LKJSCRIPT_WRITE_PREDECESSOR_FIXTURES=/absolute/empty-output \
  cargo test --locked -p lkjscript --all-features --lib \
  current_predecessor_controls_retain_exact_source_and_instructions -- --nocapture
```

Review and install the resulting controls here, then rerun without the variable.
An initial generation can fail retained-byte comparison until the new files are
installed; that failure is not acceptance. No original fixture may be overwritten.

The current envelopes were regenerated on 2026-10-03 with the supported test owner
for the owned-effect application successor. Every regenerated control passed
strict artifact admission and its exact original source-transport probe before the
retained-byte comparison.

| Current control | SHA-256 |
| --- | --- |
| `requirements.lkja` | `a4902a11d6e72d8186effe1a7922ae4872d1be95614c80c05b8ab445f7d71d9a` |
| `transactions.lkja` | `19a9be3ee1a4f6ac65520faed6d7a180ca831364e4a2b6cdeaee3c3f9ee07042` |
| `participation.lkja` | `333c51a3649f004134723f760c84a7992ad2fba4ebd2b968a52b997d48a78454` |

The execution and receipt owners require original and current material separately.
Omitting either, substituting an otherwise valid control, changing the selected
runtime/descriptor/grants, or replacing a format refusal with later failure must
not satisfy the corresponding evidence. Receipt fault checks do not replay live
effects. Full frozen-source acceptance remains separate from this fixture test.
