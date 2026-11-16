# Release of one offline complete-body extraction

Root accepts [copy-site cohort 2](compiler-profile-copy-sites-cohort-2.md) and
its independent audit `adfb9a9837ec044cc1a03cec186a7ada643300bab7efe79b324df6e597bddcbe`.
All seven measurement phases passed, both complete compressed artifacts agree
with their originals, and all six independently reconstructed accounting tables
agree. The 175-file retained evidence inventory is
`30042317e420b1db9919ad7af4f4561f2731ec59b3414e3f2385be6300d88726`.

This releases exactly one invocation of the [frozen extractor](compiler-copy-sites-extractor.md).
It is separate from the completed measurement. No compiler, profiled execution,
official decoder, extra display page or optimization is part of this step.

## Inputs and population

| Input | Bytes | SHA-256 |
| --- | ---: | --- |
| Complete profile, identity only in this step | 55,464,197 | `7849d4dd69906491d07664f493eb0f4fd8340cfde3760f6e4f168dbbde95f302` |
| Exact checked selected companion | 57,649,044 | `32c0f43dbde125e43f2ca5a0c6d8548ba0f1774ce3b51bc62d94a6488af0fdca` |
| Sole official detail page | 53,808 | `06093752531ea752f7081548166f33cc95c71aad381e72d2713e35f5bcaab25d` |
| Extractor, commit 416da03c | 31,883 | `9ca679c6f24cfe1ad9fcfd14a54f27be0edef56a1c9a0faeb7c455ab76536517` |
| Existing capture wrapper, unchanged | 632 | `85dcbc04be568238d8b7f079292b5e4475567428fc7d1c8be326ed9e10eb5109` |

The population remains the first twenty distinct functions in first-appearance
order in the sole 200-row page. The page contains 825 samples; this is not the
full population of sites in those functions. No alternate population, additional
page or replacement for an oversized function is allowed.

The [census plan](compiler-copy-sites-census-plan.md) and all its per-function
bounds remain unchanged. Root explicitly accepts the extractor's additional
32 MiB structural-line guard and its documented export-versus-traversal
qualification. One process runs under 4 GiB / 300 s / 10 s grace; the extractor
requires 3 GiB free space before taking snapshots. Resource or grammar refusal
is an unassessed result. Partial output never establishes a complete body.

## Exact command

Gateway request: `compiler-copy-sites-census-extract-20261116-once`.
Working directory: `/home/coder/workspace/lkjscript`.

```sh
bash bench/compiler-copy-sites-census-extract-release-20261116.sh
```

The [committed launch script](compiler-copy-sites-census-extract-release-20261116.sh)
checks the exact input/tool/source pins and committed bytes before invoking the
unchanged capture wrapper. Capture directory:
`target/compiler-copy-sites-census-evidence.20261116-1`.
Output directory:
`target/compiler-copy-sites-census-output.20261116-1`.
Both must be absent. Captures preserve argv spelling, cwd, stdout/stderr, exit,
wall/CPU/RSS, UTC and load. The run is once-only; any refusal and partial output
remain retained without adaptive rerun.

## Semantic context pins

These are source d461c2d9 and its IR contract, not newly inferred facts.
No named source path from the companion will be read.

```text
af4c78b51063bce8a24e2a35bd364a48cbb3a888d01c1fd6219faa3d5983d4f8  src/ir_text.lkj
40f7ae357e53524e0e2e89aa5e0b697613705b7342fef10346f78673679c9655  src/ir.lkj
f88d1ff9a50ce796d2fc10bfa04cd4e8d73a30d8284d5a9b3bcd3488f724daf0  src/native/layout.lkj
71a23af143af92e8134f8453f34dd53f934b41d25736419f296c7458f5016e78  src/check.lkj
961d59858c83afd8c5e8e2f9ec00d0f3a90cc91953ba8903528589f41a9da705  src/optimizer.lkj
333d6ec927416917fa32269730a9a507fcfe31072a2191403a24161b5cf6c412  src/driver.lkj
4a42f7705edf85ad31641b0afec6f89192585b18ff0eca9e75b835ae623b64e3  src/reuse.lkj
029b2ebc01b3aa7fe87c06b663c51ff9f7533fd95d687c0ad10f3c37f844fd26  docs/ir.md
```

A successful extraction establishes exact complete material only. A later
read-only join must account for every raw copy row in the selected functions.
Eligibility remains governed by complete uses, type/layout/cleanup facts and
initialization/lifetime/escape rules; the original body bytes preserve consumed
Copy operands and field-snapshot obligations. This step does not classify a
family or authorize a transformation.
