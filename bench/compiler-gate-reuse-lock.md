# Closed-gate lock fixture after no-create reads

The production lock lifetime is correct. The old fixture expected a genuinely
absent cache lookup to create `.budget-lock`; that expectation depended on the
read-miss creation bug removed by the regular, no-create cache reader. A missing
entry now returns without entering writer accounting. An opened regular entry
whose authentication fails still enters disposal and acquires the writer lock.
`gate_reuse_main::run` drops the lookup Store before executing its closed child.

`tools/gate-reuse-lock-test.sh` now tests both paths. It stops an infinite,
authority-free child after observing its process. For an absent lookup, the
store must remain completely empty. For a rejected regular proof, the fixture
independently derives the exact public, length-framed proof filename, supplies
unauthentic bytes, and requires that exact file to have been removed and the
accounting lock to exist. An independent nonblocking `flock` must then acquire
that lock while the child is still stopped. A future identity-format mismatch
therefore fails the disposal assertions instead of silently exercising only a
cold miss. The fixture never manufactures an authenticated success proof.

Both stopped children are killed and reaped; neither failed execution may
publish success. The existing busy-writer, subsequent serial publication and
serial reuse checks remain in place. No production source, authentication rule
or lock behavior changed.

## Frozen validation

The fixture branch starts at
`4f8c213559b7de96b3b6bd6199da661dfc95f778`. The compiler and closed runner were
copied without byte changes from the failed gate's artifacts:

| Artifact | SHA-256 |
| --- | --- |
| Compiler | `adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7` |
| Closed runner | `0d642bc58963fea9519e53de31906ad2274e888406cb6e7df11881fce9ea3b3e` |
| Repaired lock fixture | `a6b308455de35fa6b5dd6666cc3ed4594d0a4d3aec9e4d4f01241f69c4754df2` |

Every invocation used one sequential worker, a 4 GiB address-space limit and a
900 s watchdog with a 10 s kill allowance. GNU time reports wall/user/system
seconds and peak KiB below; load is the 1/5/15-minute tuple immediately before
and after the command. These are test observations, not a performance claim.

| Invocation | Result | Wall / user / system s | Peak KiB | Load before → after |
| --- | --- | --- | ---: | --- |
| Original lock fixture | Expected assertion failure: `cold lookup did not reach store accounting` | 0.19 / 0.16 / 0.00 | 47,768 | 11.28/10.20/9.62 → 11.28/10.20/9.62 |
| Repaired lock fixture | Passed | 0.26 / 0.23 / 0.02 | 42,928 | 8.02/9.41/9.39 → 8.02/9.41/9.39 |
| Enclosing proof suite, initial setup | Setup failed while appending to a copied runner | 0.42 / 0.37 / 0.03 | 43,448 | 8.06/9.38/9.38 → 8.06/9.38/9.38 |
| Enclosing proof suite, corrected setup | Passed, including repaired lock fixture | 1.80 / 1.64 / 0.10 | 48,584 | 8.05/9.29/9.35 → 7.64/9.19/9.31 |

The setup failure came from making the owned executable copies mode 0500.
The existing enclosing suite copies its runner and appends bytes to test a
changed producer; that copy inherited the unwritable mode. The owned copies
were restored to the originals' mode 0755 and their hashes reverified before
the corrected run. No test assertion or source was changed to bypass it.

The enclosing command was `bash tools/gate-reuse-test.sh
 target/compiler-gate-reuse-candidate target/compiler-gate-reuse-runner`.
It passed exact sealed bytes, producer and contract identity, authority/image/
entry matching, key and corruption attacks, cross-compiler reuse, failed-child
nonpublication, host exclusions and multi-entry parity/preflight.

Raw stdout, stderr, exit status, GNU-time and load records remain under
`target/compiler-gate-reuse-lock-baseline.*`,
`target/compiler-gate-reuse-lock-fixed.*`,
`target/compiler-gate-reuse-suite.*`, and
`target/compiler-gate-reuse-suite-corrected.*` in the isolated fixture worktree.
The initial full-gate failure log remains on main with SHA-256
`c2cc2a17de8133aae6d9691851ad3b75aefe306430529066ab466bc16940890c`.
