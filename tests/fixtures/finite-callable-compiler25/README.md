# Compiler 25 finite-callable rejection fixtures

These invalid artifacts retain the exact canonical source and instruction forms
from `../finite-callable-predecessor/expanding-{direct,named}.lkja`. Their derived
envelopes use compiler 25, bytecode 20 and artifact 32 with compiler meaning
contract Graph 25. Frozen source generations remain unchanged.

The test-only owner is
`platform::compiler::tests::effect_tests::strict_artifact_rejects_fully_rehashed_expanding_canonical_applications`.
It decodes the frozen original instructions, re-encodes only derived containers,
requires `semantic / kernel_callable_expansion`, and compares complete generated
bytes to these retained fixtures. The offline consumer independently requires the
same current semantic refusal and the original format refusal without execution.

Generated on 2026-10-04 through the owner's create-new
`LKJSCRIPT_WRITE_FINITE_FIXTURES` writer, using immutable library-test executable
SHA-256 `179dbbf0cf4797d1fe6238d8d4198f5333b45c5bea0dccb2546f835c55ae33bc`.
Original generation logs remain under
`.artifacts/20261004-owned-sequences/compiler-preparation-test04/finite-writer.log`;
the output directory is `compiler25-finite-writer/` under the same evidence root.
The bootstrap run reached both expected semantic refusals before failing its
comparison against the previous generation's retained bytes.

| Fixture | SHA-256 |
| --- | --- |
| `expanding-direct.lkja` | `24f133384ad0a9e8bb4ef12a89713cbe828f1e6cff667c90c26f6f4fad522198` |
| `expanding-named.lkja` | `6c7ea5220105d004d7e20840dd6bbb541c6415798d14955a696ea2282d034087` |

The authenticated predecessor originals and earlier compiler-24 controls in
`../finite-callable-current/` remain unchanged. These new paths prevent a format
cut from overwriting previously retained evidence. Regeneration still requires a
caller-selected empty directory and the existing create-new writer; no product
loader exposes a conversion or admission bypass.
