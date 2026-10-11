# Current status

Snapshot: 2026-10-11. Accepted meaning, source verification, mainline delivery,
finalized executable acceptance and public availability are separate boundaries.

## Mixed storage contracts under verification

Development **0.1.92** adds ordinary transferable data parameters to owned
contracts and generic implementation schemes. Self remains exactly Owned;
ordinary category, transferability and capture safety are checked independently,
including unused parameters and nested phantom arguments. The native
`IndexedData` library has Flat and private nominal Chunk32 implementations.
The dependency planner selects integer and Boolean storage independently in all
four combinations. See the [release notes](releases/v0.1.92.md),
[contract specification](spec/owned-contract-parameters.md) and
[native storage example](../examples/indexed-data-storage/README.md).

The selected implementation worktree is
`/home/coder/workspace/lkjscript-mixed-storage-20261011`, branch
`work/mixed-storage-contracts-20261011`, based on remote-main reporting commit
`0eaae23d2cd76f2b78b982170fd357fa08c8f8f5`.
Implementation and maintained native assets are complete. The current copied
library test executable passes all 45 focused mixed-contract checks, both witness
reservation checks and the ten repaired library checks from the preceding full
development run. All three copied-product storage tests pass, including 104
differential invocations and 98 production invocations after source deletion.
The planner and independent checker suites pass 3/3 and 9/9 tests respectively;
each covers 55,620 complete plans across both prior component carriers and all
four storage combinations, attached and detached. The checker also rejects
exhaustive false claims and targeted mutations, with joined cleanup and recovery.
No fresh full-source acceptance or mainline delivery of these changes is claimed yet.
Original build, test, native-asset and copied-product evidence is retained under
`.artifacts/mixed-storage/` in that worktree. Initial failed build and regeneration
attempts remain separate from corrected runs.

Matched storage measurements pass all 480 serial executions across 20 workloads,
with 60 warmups and 420 measured samples. Complete results, joined cleanup and
asset identities hold. On this shared host, indexed Flat/direct invocation ratios
were 1.111–1.241 and Chunk32/Flat ratios were 1.364–3.392. Preparation and invocation
remain separate observations; no portable speedup or API-cost saving is claimed.
The native example records the measured table, exact executable/artifact identities
and original evidence at `.artifacts/mixed-storage/costs/measurement-first/`.

The intentional cut selects Graph 33, interface 21, compiler 33, artifact 40,
authored request 37 and semantic validator 38. Bytecode remains 27. Supported
older meaning retains its original owner generations and interface projection;
new mixed declarations and invocation expressions require their applicable
Graph 33 boundaries. Maintained derived artifacts are regenerated through their
native owners. Finalized-candidate contract and workload generation 9 require
the complete new indexed-storage public witness family.

Next: freeze this source and run the complete fresh verification owner
(`lkjscript-dev check full --fresh --machine --jobs 2`, with Cargo jobs 2).
Integrate the exact
accepted source through ordinary mainline delivery before selecting a new
publication producer. Do not reuse predecessor acceptance for these changes.

## Existing accepted source and publication

Nominal-record source **`ccbed8cf132a633a81182d62192031d38b03b9ca`** is already on
remote main. Its original 26/26 fresh source gates, 2,455 passing workspace tests,
FIFO measurements and integration evidence retain their original bindings in
[the previous status](https://github.com/lkjsxc/lkjscript/blob/0eaae23d2cd76f2b78b982170fd357fa08c8f8f5/docs/status.md).
The old worktree `/home/coder/workspace/lkjscript-generalized-sequences-20261011`
and its ignored evidence remain retained through release closure.

The v0.1.91 candidate
[38116583723/1](https://github.com/lkjsxc/lkjscript/actions/runs/38116583723)
selected that exact accepted source and workflow 343592338. It failed at
08:18:38 UTC when the native-public aggregate reached its 1,800-second execution
deadline. Source verification and asset construction passed; 42 of 96 public
cases completed successfully, with no assertion failures observed, and 54 remained
uncompleted. Finalized-byte acceptance and publication were not reached. No tag,
scoped-control change, retry or promotion occurred.

Authenticated original diagnostic and terminal archives, receipt, harness logs
and compact diagnosis are retained in the old worktree's
`.artifacts/owned-nominals/delivery/failure-38116583723-1/`. The successor grants
only that serial execution aggregate 120 minutes; its separate build allowance
stays 30 minutes, and all coverage and outcome/cleanup admission remain required.
The failed producer is ineligible for promotion. It is superseded when 0.1.92
reaches independent fresh source acceptance and exact mainline integration; only
then dispatch one new producer from that accepted source.
