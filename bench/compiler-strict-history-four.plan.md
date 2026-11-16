# Declared four-candidate language-source probe

This is a separate experiment after the completed compiler-scoped campaign
at `5e63d79b03485344e18c0b5170b7aa97f7ccbfa3`. It does not resume or modify
that campaign, rerun any of its 103 attempts, or claim whole-commit reproduction.

## Frozen controls

- Population: the same last 300 commits at
  `a4fd685a9bb2e7736110a4ed2a43651dfe040d56`, with the same 197 ordered
  compiler-source candidates.
- Exactly four original ordinals, in order: **104, 107, 112, 158**.
  The [selected order](compiler-strict-history-four.order) retains full
  revisions, first parents and original weights.
- Producer: `4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09`.
- One compiler worker, 4 GiB address-space limit, 900 seconds per command,
  2 GiB store, and the same free-space reservation formula.
- Fresh owned fixture, key and cold store. Historical source projection,
  library selection, candidate checking, publication and target comparison
  follow the frozen scoped harness.
- No retry, resume, threshold change or producer change. Observe all four
  once, unless setup/resource failure or an implementation failure blocks the
  segment. Every refusal remains a recorded observation.
- Acceptance requires nonempty actual publication, equal canonical
  `lkjscript-checked-meaning-1` identities, coverage of every changed
  `src/` or `library/` path, and coverage of every changed `.lkj` or
  `.rs` path anywhere in the repository. Non-language documentation and
  host-tool changes remain outside this language-source claim.

The exact probe harness SHA-256 is
`acc598b8e21e644351494fe8e6c8f3ce9de7ea6daae97481f0b5ce9b436b0bb2`.
Runtime observations at this declaration: **none**.

## Imported evidence

Twenty-three existing successes satisfy the stronger language-source scope.
They are imported as prior observations, not rerun or treated as a continuous
execution. The [imported proof rows](compiler-strict-history-imported.tsv)
are an exact subset of the prior proof table, excluding the seven revisions
with changed language files outside the compiler scope.

| Input | SHA-256 |
| --- | --- |
| Prior raw-record manifest | `f1979bc70f3127080a0fd957772f4fb975f5e2257bad11dd0a0428d628ed3b8b` |
| Prior proof table | `8fd505fc658b051491ddc157e43783bff39befcde721e3ab4b97115661a93eae` |
| Prior outside-language paths | `e5a7ba7ffb4a2f60b241eabbbdfd41e354be9ed962d4c219b512161f92d5401e` |
| Imported 23 proof rows | `0aafd3a891bf224bd3113ab9de58219fecba44b84e0c5605a28f8ced0f25acb1` |
| Four selected original candidates | `1241ad75095c3492af9ab18b9435d88bc6797d74b03590622264498c5ce3f5bd` |

The harness validates these controls and the full prior raw manifest before
creating the probe fixture. It recomputes the frozen Git candidate order and
checks the selected four against the declaration.

## Remaining-window assessment

The [static assessment](compiler-strict-history-assessment.tsv) covers all
94 remaining candidates. Fifteen have no changed language files outside
`src/` and `library/`; 79 require changes outside that compiler scope.
Eleven of those fifteen add projected compiler-source files. They are listed
as [structurally blocked cases](compiler-strict-history-new-file-blockers.tsv)
under the current absent-path writer/planner contract. They are **not observed
runtime refusals** and are not executed in this probe.

Only four avoid projected file-set changes. Two were absent from the original
older-producer dataset; its observations for ordinal 107 and 158 were
`E_UNSUPPORTED` and `E_UNSUPPORTED_MACRO`. Those prior results are predictions
only because their producer differed.

With the retained producer and absent-path limitation, this probe can add at
most four successes to the imported 23: an upper bound of **27**, below thirty.
After these observations, any source-creation repair requires a separately
declared producer and experiment; it cannot alter the original scoped rows or
retroactively turn static blockers into observed results.
