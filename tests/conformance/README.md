# Conformance

Run `node tools/conformance.mjs` after building the compiler. The default route
is native Linux x86-64; `LKJ_ENGINE=k` selects the independent machine and
`LKJ_ENGINE=node` selects WebAssembly. `LKJSCRIPT_BIN` selects a compiler.
The routine gate runs native and K; a three-route run uses
`LKJ_GATE_ENGINES=native,k,node bash tools/check.sh`.

Source cases expose an ordinary `probe` function with a printable scalar result,
or a World-taking `main` that writes bytes. Direct K fixtures exercise the
safety contract without the source front end. Each batch case has fresh program
state. The Rust oracle builds comparable positive cases together with overflow
checks enabled; cached references include source and compiler identity.
Rejection cases also check Rust's verdict and recorded reason code where applicable.

`cases.json` records the intended result or diagnostic, whether Rust can
express the case, and whether the lkjscript feature is still pending.
A pending case counts as pending only after an explicit `E_UNSUPPORTED` or
`E_PARSE` diagnostic. Acceptance with a wrong result, acceptance of a rejection
case, or an unrelated diagnostic fails the suite. Remove the pending flag as a
feature becomes implemented. Capability callback examples under `pending/`
remain proposals until their library interfaces and contracts become concrete.

The seeded generators compare bytes and termination. Sugar and closure generators
also produce explicit expansions, providing a reference that does not depend on
Rust accepting the surface form. Keep seeds and reduced failures in Git; the
runner writes detailed output and reference caches under `target/`.

Process time and output envelopes belong to the test runners. They impose no
language limits.
