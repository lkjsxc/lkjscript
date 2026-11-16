# Four separate probes of complete language-source coverage

**One of four new probes passed.** Together with the 23 imported prior proofs,
there are **24 revisions with every changed language file represented in an
applied transaction and equal canonical checked meaning for the selected
compiler root**. The thirty-case target remains unmet. This is a separately
declared experiment, not a resumption of the earlier 103 attempts and not
whole-commit reproduction.

The [original frozen-window result](compiler-history.md) remains 8/179.
The [contemporaneous compiler-scoped result](compiler-self-history.md)
remains 30/103, with seven of those thirty excluded from the imported
language-scope subset because they changed language files elsewhere.
All eight older successes overlap the contemporaneous result; these
populations must not be added together.

## Controls declared before execution

The [plan](compiler-strict-history-four.plan.md) was frozen at SHA-256
`091bf61d5fd1da3cf869fdebc9c9f0a2e70ff2efff129e089193a464affe59e8`.
The exact [harness](compiler-strict-history-four.sh) was
`acc598b8e21e644351494fe8e6c8f3ce9de7ea6daae97481f0b5ce9b436b0bb2`.

The population is still the last 300 commits at
`a4fd685a9bb2e7736110a4ed2a43651dfe040d56`, ordered by the same compiler-source
change weight and commit hash. Exactly original ordinals **104, 107, 112, 158**
were selected in that order. No prior case was rerun, and no refusal was
retried.

The immutable producer was
`4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09`
(20,627,356 bytes). One worker used a fresh owned fixture, private key and
cold store, a 4 GiB address-space limit, a 900-second timeout per compiler
command and a 2 GiB store budget. Historical `.rs` files were copied
byte-for-byte under `.lkj` names; no source text was rewritten to gain
admission. Both sides used their historical library roots when available,
otherwise the same producer's frozen embedded library. The command's
working directory was the owned fixture.

The five free-space checks passed. Initial availability was
118,936,989,696 bytes against a 4,584,030,108-byte reservation; the minimum
observed availability was 118,911,537,152 bytes and the largest reservation
was 4,929,798,044 bytes. These are preflight observations, not peak disk-use
measurements.

Acceptance requires a nonempty applied transaction, matching
`lkjscript-checked-meaning-1` identities for candidate and desired trees,
and file facts for every changed `src/` or `library/` path plus every
changed `.lkj` or `.rs` path anywhere in the repository. Documentation,
host-tool changes and other non-language effects are outside this claim.
Canonical checked-meaning equality does not assert serialized IR identity
or general byte-for-byte reproduction of historical source.

## Four observations

The segment ran from 2026-11-16 03:53:04 to 03:55:33 UTC and exited normally.
Wrapper wall time was **149.27 s**, user 142.44 s, system 2.98 s, peak memory
1,165,260 KiB, load **1.68 → 2.66**. The wrapper includes materialization,
Git checks and audits performed by the harness. The table records only
each timed compiler command.

| Original ordinal | Revision | Result | Operations | Wall s | User s | System s | Peak KiB | Load |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| 104 | `39d18d44` | Accepted: candidate and desired canonical meaning equal; every required file covered | 11 | 67.22 | 65.14 | 0.54 | 1,086,608 | 1.68 → 2.03 |
| 107 | `20fd1793` | Refused: `E_UNSUPPORTED` | 0 | 0.70 | 0.66 | 0.00 | 67,164 | 2.03 → 2.03 |
| 112 | `89bb5bef` | Excluded: required file absent from the applied transaction | 2 | 70.40 | 68.29 | 0.57 | 1,165,260 | 2.03 → 2.52 |
| 158 | `5e7e8d37` | Refused: `E_UNSUPPORTED_MACRO` | 0 | 5.27 | 5.07 | 0.04 | 164,536 | 2.80 → 2.66 |

Ordinals 104 and 112 used the native language files, `src/main.lkj` and
historical libraries. Ordinal 107 used legacy filename projection,
`src/lib.lkj` and historical libraries. Ordinal 158 used legacy filename
projection, `src/lib.lkj` and the shared frozen embedded library.
There was no setup failure, resource refusal, interruption or retry.

The accepted case has four file facts. All four before hashes equal their
exact historical Git blobs, and all four published after hashes happen to
equal the desired historical text. Its common candidate/desired canonical
checked-meaning identity is:

```text
f3c13c0ff209c50b94e9747266e8de3af0274de284f69c19b1a1aa1cb398304b
```

Ordinal 112 published two operations and proved the selected compiler root
equal, but did not represent the changed
`src/structure_refactor_cases.lkj`. The stronger coverage check correctly
excluded it. The retained selected-root equality is
`2a1fae52002bebc3d0f4d48da3f43527ebe6845b9ecfdef17f85afd6b8d47b3d`;
it is narrower evidence and is not counted as a successful probe.

The first diagnostic for ordinal 107 is
`src/resolve.lkj:948:18: E_UNSUPPORTED unsupported construct: impl`.
Ordinal 158 first reports
`src/lib.lkj:11:35: E_UNSUPPORTED_MACRO unsupported macro: env!`.
Their complete diagnostics, including subsequent source errors, remain in
the raw records. The rows identify the first diagnostic rather than
claiming it is the only incompatibility.

## Imported evidence and resulting bound

The [23 imported rows](compiler-strict-history-imported.tsv) are an exact
subset of the prior thirty proof rows, excluding the seven revisions listed
in the [outside-language audit](compiler-self-history.outside-language.tsv).
Their SHA-256 is
`0aafd3a891bf224bd3113ab9de58219fecba44b84e0c5605a28f8ced0f25acb1`.
They retain their original timings, producer, stdout/stderr hashes and
canonical identities; no timing is attributed to this probe.

Those 23 prior rows contain 126 operations and 42 file facts. Adding the
single new success yields **24 cases, 137 operations and 46 file facts**.
The new segment contributes only 11 operations and four file facts.
The scope-excluded ordinal 112 contributes nothing to that aggregate.

The static assessment of all 94 candidates remaining after the earlier
103 attempts found 79 with changed language files outside the compiler
scope, eleven with new projected compiler-source paths, and these four
without either issue. The predeclared upper bound of 23 + 4 = 27 was
therefore already below thirty. After the four actual observations, the
accepted count under these controls is 24. The eleven unexecuted
new-file cases remain potential work for a different producer and a
separately declared experiment; they are not hidden successes.

## Eleven static source-creation blockers

These are **structural blockers inferred from the absent-path planner and
writer contract**, not observed runtime refusals in this probe. All add
projected compiler-source files, and none was executed here. They require
37 added paths in total. Some also have older-producer admission failures,
preserved in the [assessment table](compiler-strict-history-assessment.tsv);
those earlier observations cannot predict acceptance under a repaired
producer.

| Original ordinal | Revision | Added paths | Exact projected paths |
| --- | --- | ---: | --- |
| 106 | `892b46da` | 2 | `src/platform/compiler/selfhost_input.lkj`, `src/platform/execution/normalized/selfhost_tests.lkj` |
| 108 | `81048410` | 1 | `src/semantic_tests/optimizer/forwarding.lkj` |
| 110 | `7b6d333b` | 3 | `src/elaborate/definition_types/semantic.lkj`, `src/elaborate/semantic.lkj`, `src/semantic/references.lkj` |
| 115 | `47e98941` | 2 | `src/platform/compiler/link_projection_measurement_tests.lkj`, `src/platform/compiler/link_projection_tests.lkj` |
| 122 | `52e3e11c` | 1 | `src/platform/execution/normalized/typed_compiler_input_fixture_tests.lkj` |
| 127 | `685764ae` | 4 | `src/driver/structure.lkj`, `src/meaning.lkj`, `src/structure_edit.lkj`, `src/structure_write.lkj` |
| 144 | `16055fc0` | 5 | `src/native/semantic.lkj`, `src/semantic.lkj`, `src/semantic_facts.lkj`, `src/semantic_query.lkj`, `src/semantic_store.lkj` |
| 152 | `d6ff7809` | 2 | `src/structure_refactor.lkj`, `src/structure_refactor_cases.lkj` |
| 154 | `3556e582` | 2 | `src/platform/execution/normalized/selfhost_source_profile_flow_tests.lkj`, `src/platform/execution/normalized/selfhost_source_profile_tests.lkj` |
| 157 | `acf74027` | 4 | `src/platform/compiler/elf_x86_64/scalar_abi.lkj`, `src/platform/compiler/elf_x86_64/scalar_flow.lkj`, `src/platform/compiler/elf_x86_64/scalar_lower.lkj`, `src/platform/execution/normalized/native_scalar_boundary_tests.lkj` |
| 184 | `411886df` | 11 | `src/platform/compiler/artifact_capacity_tests.lkj`, `src/platform/compiler/unit_codec.lkj`, `src/platform/compiler/unit_codec_tests.lkj`, `src/platform/execution/normalized/selfhost_source_sequence_preparation_tests.lkj`, `src/platform/execution/normalized/selfhost_source_sequence_tests.lkj`, `src/platform/execution/normalized/vm_preparation_capacity_tests.lkj`, `src/platform/kernel/codec/linear.lkj`, `src/platform/kernel/codec/linear/expression.lkj`, `src/platform/kernel/codec/linear/owner.lkj`, `src/platform/kernel/codec/linear/tests.lkj`, `src/platform/package_transport/source_capacity_tests.lkj` |

Adding absent files is therefore a concrete operation gap. Ordinal 112
also demonstrates a separate coverage boundary for existing source outside
the selected root. Neither issue should be solved by dropping required
paths or counting a narrower root equality as full changed-language
coverage.

## Evidence and audit

The [result rows](compiler-strict-history-four.tsv), [new proof row](compiler-strict-history-four.proofs.tsv),
[metadata](compiler-strict-history-four.tsv.meta), [space checks](compiler-strict-history-four.tsv.space),
[wrapper metrics](compiler-strict-history-four.run.time) and all 26 raw
files (433,433 bytes) are retained. The original scoped manifest and all
103 prior rows were reverified unchanged.

The [read-only auditor](compiler-strict-history-four-audit.pl) validates the
imported subset, frozen Git population, declared order, actual Git changed
paths, required-file coverage, nonempty publication, canonical equality
and exact historical before hashes. It prints the
[audit result](compiler-strict-history-four.audit.log). It never invokes
the compiler. A manifest records the hashes of the new artifacts separately
from the immutable earlier manifest.

After acceptance, the owned fixture (332,021,266 bytes before deletion),
private key, store and both producer copies were removed at 03:57:53 UTC.
The [cleanup record](compiler-strict-history-four.cleanup.meta) confirms
that no stop marker remains. Intentional text evidence is preserved, and
no compiler job remains.
