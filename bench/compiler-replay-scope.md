# Explicit source grants for replay and comparison

The six fixed recent-history commands against the previously gated producer
refused before producing any transaction or checked-meaning equality proof.
Their unchanged runtime include bytes had different source spellings: the
before tree used an explicit absolute source Directory, while the desired tree
used its working directory. Stable library declarations supplied an absolute
library correspondence that did not name the desired relative include-only
input. The complete failed campaign is retained separately; it does not increase
the 24 historical strict proofs.

The structural CLI now accepts `--against-source-dir DIRECTORY` for
`edit` and `compare`. It assigns the other source Directory explicitly,
just as `--against-library-dir` assigns its library. The primary
`--source-dir` still owns the editable tree. The new option requires
`--against SOURCE`; an unavailable Directory refuses without falling back
to the current directory. No filename matching, include alias inference,
checker rule or repository representation changes.

For example, from any working directory:

```sh
lkjscript edit src/main.lkj --source-dir /trees/before \
  --library-dir /trees/before/library --against src/main.lkj \
  --against-source-dir /trees/after --against-library-dir /trees/after/library \
  --store-key /private/compiler.key --dry-run
```

Both source and library grants must exclude the private key. Without the new
option, the existing entry-based source selection is unchanged. The documented
conservative file correspondence can still refuse differently spelled aliases.

## Applied operation and retained-line audit

Baseline: `2bdab38837e54c3057489891e0e33b7a8beea178`.
Four `replace-item` operations address
`driver::structure::{Request,parse,compare,replay_target}`.
The [request](compiler-replay-scope-data/request.json) has SHA-256
`cc92ff50f86eb5479bcfb19f08bffbe9d9b8a33140f1ac0a434d1e092c55ff3d`.
Each item carries its original exact-text SHA-256 as `expect`.
The preparation records each original and replacement item independently.

Only `src/driver/structure.lkj` changes:
`b5511e8e6ee3ba68b4dd8e120f0e1d26d005b76b390efae9f3ed86b1a9afc411`
→ `2b02455269ae3226ca027da6c41bfa7cfc09c11cf023a983f850297dd71c3c49`.
The retained delta is **10 added and four deleted language lines, all 14 applied
by those four operations**, with no manual language-file change. Before and
after manifests cover all 1,206 tracked `.lkj` files. An independent read-only
audit checks all 2,412 before/after Git blobs, reconstructs the exact committed
file from the four original items and replacements, and verifies the request's
text expectations. Its [raw result](compiler-replay-scope-data/independent-audit.log)
is retained with the other operation evidence.

The operation used producer
`adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7`,
a private key outside the explicit source/library grants and no reusable store.
It passed in **50.42 s (load 3.24 → 12.84)**, 48.09 user seconds,
0.61 system seconds and peak RSS 1,149,276 KiB. Its report records four
sites/identities, one file, no authority delta, 12,692 fresh checked instances,
zero proof reuse, zero frontend resumption and demanded-library validation.

## Focused validation

The directly built candidate is 31,736,147 bytes, SHA-256
`3c802509e8c37bc4016e32615b6266ab9be635a157fd5819cd07e33bd5149dd3`.
Its build took **33.50 s (load 3.72 → 3.66)**, 32.64 user seconds,
0.22 system seconds and peak RSS 1,091,504 KiB. This focused candidate was
compiled with absolute source/library diagnostic paths; a later canonical
bootstrap is a separate producer and will be recorded with the full gate.

The extended existing `tools/replay-test.sh` executes 17 compiler commands.
Its nine original observations retain checked target equality, multiple
operations, exact kept text, dry run, module/include data handling, import/frame
refusal and private-key exclusion. Their normal source grants are now explicit
on both sides, including the two value executions.

Eight added observations cover the compiler-shaped sibling runtime include:

- Without an explicit desired source root, the mixed-spelling case still
  refuses at that include, with no stdout or source publication.
- Explicit roots allow dry run and application; the candidate and desired
  checked meanings agree and the final source bytes match.
- Comparison through the explicit other root reports identical checked IR.
- Both commands refuse an unavailable explicit root despite a valid entry in
  their working directory, with no output or source change.
- Both commands reject the source-root option when `--against` is absent.

The accepted suite passed in **4.02 s (load 4.47 → 4.47)**,
2.60 user seconds, 0.07 system seconds and peak RSS 75,028 KiB.
All owned fixture trees and keys were removed and the process drained.

Two earlier focused observations remain in the data directory. The first
failed in 2.87 s (load 3.66 → 3.53); a diagnostic run failed in 2.81 s
(3.82 → 3.82). The diagnostic run shows successful scoped replay and comparison before a test
expectation that the existing missing-scope diagnostic contained the pathname.
It actually reports
`E_EDIT_IO open source transaction scope: errno 2`.
Only that assertion was corrected; the production source and candidate bytes
were unchanged. The first run's exact internal stopping point was not retained. The first run retained only exit/time/load because the old
cleanup trap removed redirected case diagnostics; the diagnostic run retains
the command trace and actual refusal.

This is a CLI scope correction, not a performance result or new historical
replay proof. The complete four-worker gate, canonical fixed point and CI are
required before a separately frozen new replay campaign.

## Accepted full gate and canonical seed

The full empty-store gate at `830896f51b449532e99c203a6531844fa399fb50`
passed in **657.932 s (load 6.68 → 9.29)**, with 5,448 fresh case requests,
zero case reuse and 128 mutants with zero findings. GNU time reports 657.93 wall
seconds, 1,629.96 user seconds, 34.69 system seconds and peak RSS
1,213,464 KiB. Four workers ran under a 4 GiB address-space limit and a
2,400-second watchdog. The fresh store occupied 316,084,405 bytes before
its directory and external private key were removed; the process was drained.

The canonical bootstrap passed in **86.797 s (load 6.68 → 9.68)** and
reproduced the 31,724,996-byte executable with SHA-256
`ab2fa80787261e2fec6d872d989099d3b1dd0984c26ac59bf89c7237d9e97504`.
This is the producer for any later frozen scope campaign, distinct from the
focused absolute-path build above. The source/library trees remain unchanged
through the evidence-only and seed commits.

The [full log](compiler-replay-scope-data/cold-gate.log) has SHA-256
`aff46525d057e1195fa3d5fb2dda2d324d5a2884ab791069d485451f0294ed46`.
The final mutation detail records 104 rejections, 22 values, one trap, one
nontermination and no resource findings. Its 15 checked and three emitted
reuses are internal mutation-campaign reuse, separate from zero gate-case reuse.

Seed-only commit `8007e47d` stores that exact canonical producer with
`gzip -n -9`: 6,367,232 compressed bytes, SHA-256
`13c601190b8bb3926a03d2b1bd40f875c357b68d3376f073f3db74e3f569faca`.
Independent repeated compression compares exactly, and decompression matches
the gated executable. New scope CI and any later historical replay outcomes
remain separate acceptance steps.
