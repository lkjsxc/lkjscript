# Conformance

Run `node tools/conformance.mjs` after building the compiler. Set
`LKJSCRIPT_BIN` to select a different compiler. The fixture entry is
`pub fn probe() -> i64`, an ordinary Rust function. The runner builds one
Rust oracle with overflow checks enabled and compares results with generation
two running on Node. Rejection cases also check Rust's verdict where applicable.

`cases.json` records the intended result or diagnostic, whether Rust can
express the case, and whether the lkjscript feature is still pending.
A pending case counts as pending only after an explicit `E_UNSUPPORTED` or
`E_PARSE` diagnostic. It never counts as a semantic pass. Acceptance with a
wrong result, acceptance of a rejection case, or an unrelated diagnostic fails
the suite. Remove the pending flag as each feature is completed.

The two-input view case intentionally uses the initial one-origin rule;
Rust accepts its shared lifetime, while the initial lkjscript rule rejects it.
The three fixtures under `pending/` propose capability callback APIs and must
be updated when those library interfaces become concrete.

Process time and output envelopes belong to this runner. They impose no
language limits.
