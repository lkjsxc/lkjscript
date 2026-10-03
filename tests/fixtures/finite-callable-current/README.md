# Current-envelope finite-callable rejection fixtures

These two deliberately invalid artifacts preserve the canonical source and
instructions of `../finite-callable-predecessor/expanding-{direct,named}.lkja`.
Only derived compiler-unit, compilation-map/manifest, pack and artifact envelopes
are re-encoded for graph 22 / compiler 22 / bytecode 17 / artifact 29 under
semantic validator 27.

The test-only owner is
`platform::compiler::tests::effect_tests::strict_artifact_rejects_fully_rehashed_expanding_canonical_applications`.
Its independent loader must reject each current fixture as
`semantic / kernel_callable_expansion`. It also compares the complete retained
bytes against `predecessor_attack_tests::current_derived_fixture`, which retains
the original source and instructions instead of compiling an altered program.

The original predecessor fixtures remain unchanged. They now reject as
`source / compiler_unit_contract`, before semantic application admission.
The offline contributor runs all four cases and independently checks the exact
expected stage, copied fixture bytes, selected executable and absence of execution.
A format rejection is not used as evidence of current semantic rejection.

To regenerate into an existing, empty, test-owned directory:

```sh
LKJSCRIPT_WRITE_FINITE_FIXTURES=/absolute/test-output \
  cargo test --locked -p lkjscript --lib \
  strict_artifact_rejects_fully_rehashed_expanding_canonical_applications -- --nocapture
```

The optional writer uses create-new and cannot overwrite existing files.
Review and copy the outputs here, then rerun the test without the variable.
On a format change, the retained-byte assertion may fail until the new outputs
are installed; that failure is not acceptance. No product command or production
loader exposes this test-only conversion.

First generated on 2026-09-30 from source `caeb9229` plus the recorded contributor
alignment edits. SHA-256: direct
`5d7c2c5c2ccf341b8d10e7d2622f163a47adbd46b2fa9c806ddf4c081195d140`;
named `012ab4b4a7221a60379f2bd3abf858e30c4503699ee88d9e09bd61c58cddcc76`.

The current envelopes were regenerated on 2026-10-03 with the supported test owner
for the explicit-transferable parallel successor. Both derived artifacts reached
`semantic / kernel_callable_expansion` before the retained-byte comparison.

| Current fixture | SHA-256 |
| --- | --- |
| `expanding-direct.lkja` | `8191d0a8663ceac76e9d85ca364cb6838746a95e653c3fd7fc06549972e3e4e2` |
| `expanding-named.lkja` | `bb2de0e94cb9fbe0f08a5469b4945a7d5e828cb829b541ea21f8e290615c8292` |
