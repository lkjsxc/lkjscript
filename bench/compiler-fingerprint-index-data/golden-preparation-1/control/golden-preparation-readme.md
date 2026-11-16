# Concrete baseline dump and closed-test proposal

Preparation only. The original five parent files remain untouched.

- `incremental-fingerprint-module.txt` implements exactly the two input sets and17
  named variants from the frozen parent `fixtures.json`. It calls the production
  `elaborate::incremental::fingerprint` directly; it contains no fingerprint oracle
  implementation. All six prescribed None cases are asserted.
- `golden-main.txt` uses the existing semantic-main module declarations, replacing
  the semantic-tests import with `fingerprint_fixture`. It prints one deterministic
  complete dump and invokes no pending expected-vector function.
- `integration-proposal.json` gives the two owned baseline-overlay destinations
  and later semantic-test/operation integration points. It is a preparation
  description, not a serialized edit request or an execution release.

The wire starts `lkjscript-fingerprint-goldens 1`, then each ordinal/name followed
by `none` or `some SHAPE COUNT`, every indexed `body INDEX DIGEST|empty`, and
`end`. It terminates `complete 17`. Empty body keys are explicit. Root must
retain and review the complete old-function output before replacing `expected`;
the pending function deliberately panics, so premature closed-test adoption fails.

Raw `source::load` plus `resolve::resolve` follows the existing source-contract
test architecture. This is necessary for the fixed binary include containing0xff:
SourceBundle's UTF-8 String cannot carry those exact bytes. The loader closure uses
first matching input and no ambient file read. Include variants change their bytes
before parsing/expansion; duplicate/missing/metadata variants mutate after parsing
as the frozen specification requires.

The proposed four closed groups cover all17 cases once per run. Function-coordinate
permutation and malformed spans are helper-boundary fixtures, not assertions of
admitted AST semantics. The expected-vector item is the only intentionally pending
source item. No compiler/native invocation or source-form validation has run.
