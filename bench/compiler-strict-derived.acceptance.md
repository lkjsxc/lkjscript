# Independent acceptance of the repaired historical case

The root agent accepted result commit `fdd47054346e280ba7b2a9711a158340109c4c6c` (landed as `d2e4cd11`) after independently executing the two frozen auditors against copied evidence and the still-live fixture. No compiler command or historical replay was repeated.

## Result and scope

Revision `0a410badb7e96ed9f6cf185df7d1f781d71c5fdb`, parent `579cf55e88177b9c9a4c59e2ff6c1bf7829e2878`, passed **25 operations and all five required changed-language file proofs**. Its sole compiler command took **67.18 s wall / 64.94 s user / 0.58 s system**, peak RSS **1,258,692 KiB**, load **1.39/1.77/2.03 → 2.12/1.94/2.07**. Candidate and desired values in domain `lkjscript-checked-meaning-1` are both:

`2c178793d5b3aaefe27c4fce4dd36f90a69745e4f9b82878a5218a80eae70802`

The accepted disjoint union is **30 revisions within the frozen `6a187b94fb763f7b0565436a98ab5feda3cd642f` last-300 window**: 23 imported complete-language proofs, one separate four-case probe success, five scoped successes and this new success. The original zero-of-six and scoped five-of-six campaign records remain unchanged.

The claim is complete changed-language coverage plus canonical checked-meaning equality for the selected `src/main.lkj` root and declared historical libraries/runtime. It does not assert raw serialized-IR byte equality, all possible roots/configurations or reproduction of non-language Git effects. The historical shell-test addition is outside this claim.

## Independent checks

Gateway request `compiler-v1118-strict-derived-root-audit` copied only the result TSV, metadata and raw logs to an owned audit directory, then ran these already-frozen auditors from the main repository:

```sh
perl bench/compiler-strict-derived-audit.pl --report /home/coder/workspace/lkjscript target/compiler-strict-derived-root-audit/results.tsv
perl bench/compiler-strict-derived-state.pl /home/coder/workspace/lkjscript target/compiler-strict-derived-root-audit/results.tsv
```

Each auditor ran under a 4 GiB address-space envelope and 900 s watchdog with 10 s termination grace. The proof audit took **0.10 s wall / 0.08 s user / 0.02 s system**, peak RSS **40,628 KiB**; the live archive audit took **0.20 s wall / 0.15 s user / 0.06 s system**, peak RSS **12,508 KiB**. The shared before/after load reading was **0.93/1.37/1.73 → 0.93/1.37/1.73**. Both exited zero.

The root audit verified the actual producer/argv/grants, the precise frozen population and prior proof union, all five preimage/publication facts, equal meaning values, complete time/load records and the single invocation. It also inspected **two archives and 3,695 file instances**: the 1,847-file before tree equals the parent with exactly five proven publication overrides; the 1,848-file desired tree equals its Git revision. Both exact path sets match, runtime contents/include expressions agree, and neither tree has a pending intent. All four derived tables are byte-identical to the agent's accepted tables.

The original 57-record result manifest was independently verified. The root additionally read the retained target-text diff: `source.lkj` and `structure_replay.lkj` match the desired text exactly; `ast.lkj`, `semantic.lkj` and `structure_edit.lkj` omit only four leading documentation-comment lines outside the replaced item spans. This is an actual text-different, checked-meaning-identical tree pair. It is not a comment-only parent-to-commit historical revision.

Raw root audit stdout, stderr, tables, GNU-time records, UTC bounds and load readings are retained under [compiler-strict-derived-root-audit-data](compiler-strict-derived-root-audit-data/). Its manifest binds this acceptance and the frozen auditors. Source, libraries, tools and tests remain identical to gated source `62fa76f4f863a169b906094f0dd9998c8421e77a`; this acceptance introduces benchmark evidence only.
