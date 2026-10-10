# Current status

Snapshot: 2026-10-10. Product identifiers are opaque. Accepted source, delivered
mainline, exact host bytes, finalized publication and running applications remain
separate boundaries.

## Delivered runtime and failed candidate

Remote main was independently reread through Git and the connected GitHub API as
`b2bc935ed25c90c23a2b61a86271c2f792e7789f`. The delivered development 0.1.90 runtime
remains source `5f46e96a8f0d648f286c58ac1617bbd9bf4e4515`, tree
`2f832367988dc8391157d721f56119a6f4506ad4`, normally merged by `a065f740` through
PR #12. Its original full source and exact-host native acceptance remain separate
from this verification-tool correction. The host executable SHA256 is
`9e2e2d8a49049974d246efdac955e22187cfb5396445f7fe4cb489e95d6cd6d6`.

The non-publishing 0.1.90 candidate `38034392926/1`, source `a065f740`, failed its
source-specific workspace-test deadline. The original source receipt reports
19/20 fresh passes and the workspace timeout; final assets were not built or
accepted. This is not a published defective archive. Original candidate diagnostics
remain in `.artifacts/release-38034392926/` under
`/home/coder/workspace/lkjscript-compiler-dependency-bridge-20261010`.

Public/latest was last independently observed as immutable **v0.1.89**, release
407859944, source `1b7e95badb13e5ddcc9d557027c1977fd4f2a271`, producer
`37910899478/1` and promotion `37925038733/1`. Its executable SHA256 remains
`e24c054f9b3e44a71a8875aba5385aad82dc391a1eae88a6c115fca75bc5c9a9`.
No replacement tag, promotion, local runtime selection or application deployment
has been performed by this continuation.

## Selected correction: executable output origin

Current isolated worktree:
`/home/coder/workspace/lkjscript-cargo-output-binding-20261010`, branch
`work/cargo-output-binding-20261010`, based on corrected profile source
`2916090e5c3a25d90cbb8bcfbcb5b2f5a0f5f77c`. This correction requires its own full
source acceptance; the parent's observations do not certify the new verifier.

The source checker formerly accepted a declared regular output after successful
Cargo exit even when Cargo wrote its actual executable to a different target
location. A real independent Cargo fixture and the actual gate executor reproduce
this: the original negative control reports one expected failure and one pass;
the stale 19-byte file was incorrectly marked Passed and retained as gate output.
This does not claim that a complete release was accepted with that fixture.

The [origin boundary](development-verification.md#cargo-output-origin-not-a-pre-existing-path)
now checks the observed Cargo compiler-artifact and successful build-finished
records: exact root manifest, binary target, non-test profile and declared executable
path in both executable/filenames. Missing, duplicate, conflicting and malformed
claims fail independently of file presence. Cargo's own fresh compilation outputs
remain usable. The two maintained executable producers run Cargo each time rather
than replaying gate-result evidence, so external output selection cannot be hidden
by an old producer receipt. No Cargo configuration is silently overridden.

The generic verification-cache store/load paths and the independent source reader
also recheck the relation. Their negative controls consistently rehash modified
logs, commands and dependent evidence, so a digest mismatch is not the only reason
for refusal. Cargo/test output after build-finished cannot supply a missing build
artifact. Cargo and its environment remain trusted; this is not a hostile-build
sandbox or proof of arbitrary compiler behavior.

Focused validation passes **66/66**, with no failures or ignored selections.
The complete development-tool library passes **281**, fails **0**, and preserves
**19 existing ignored tests**; the 66 are included, not additional. Clippy passes
all development-tool targets/features with warnings denied. A working predecessor
executable is also exercised: it still prints its old result while the redirected
current executable prints its new result, and the old path is refused as the
current producer's output. These are not yet a new 26-gate full-source receipt.

Original evidence is `.artifacts/cargo-output-binding/` in this worktree. It retains
all focused stdout/stderr/exit/time files, the original expected failure, and
`pre-fix-fixture.tar.gz` with its SHA256. The original failed fixture remains at
`/tmp/.tmpqb10nu`; the archive is preserved on persistent storage. A reader fixture's
initial positive-control failure serialized an OsStr as tagged data rather than a
UTF-8 target name. Its failed originals remain separate; the fixture was corrected,
not an admission condition weakened.

## Economical verification and original custody

The parent separates ordinary `profile.test` (optimization 2, line tables,
debug assertions and overflow checks retained, no Rust incremental cache) from
explicit `profile.test-debug` (optimization 0, full debug information, incremental).
Development defaults, release optimization, coverage, gate inventory and deadlines
are not reduced. The matched seven-fold observations, command/feature correction,
immutable comparison archive and original failed measurement remain under
`.artifacts/verification-profile/` in the preceding worktree. They are narrow
same-workload observations, not a universal runtime or cold-build speedup.

The original full ac167709 execution terminal was recovered: exit zero, 26/26 fresh
passes, no reuse; receipt digest
`verification_796e6051816f0f03dcc37b12577261bea485b8ebdf5bde068495e8771d801374`.
Its tmpfs source and receipt are absent. The recovered summary is an observation,
not a fabricated replacement for missing originals or portable source acceptance.

A replacement full run for **2916090e** is executing in the persistent preceding
worktree through its immutable source-matched checker. Original session:
`lkj-elealanfojcdejaojlplgleiecnfpnin`; launch request
`lkjscript-persistent-fresh-full-acceptance-20261010-1014`.
Receipt owner:
`.artifacts/lkjscript-dev/check/1791640266982142068-3867459-0/`.
Recovery logs, immutable checker and environment are in
`.artifacts/verification-profile/resumed-full/`. CPU affinity is 0-7, Cargo jobs 8,
one gate worker. At this snapshot it has reached the workspace-tests gate; its
actual terminal and complete original receipt still need collection. Do not run
another copy or alter its tracked source while it is active.

Capacity was reclaimed only from the original owned checkout's inactive Rust
`target/debug/incremental`, after checking for build owners. Logical cache bytes
were 508,947,001,344; filesystem available bytes rose from 4,591,673,344 to
297,698,336,768. Source, Git state, executables, original proof, other worktrees and
applications were retained. Before/after observations remain in resumed-full.

## Completion boundary

Collect and validate the parent's original full terminal without relabelling it.
Then execute the complete fresh source profile for this origin correction with
persistent evidence and a checker compiled for this exact checkout. Normally
integrate accepted source, recheck the independent main ref, and only then select
another configured non-publishing 0.1.90 candidate if the release identity remains
unoccupied. Finalized static bytes, target owners, userlands, installation/recovery,
native-public evidence and original-reader acceptance precede unchanged-asset
promotion under [the release procedure](release.md).

The independent native FIFO study in the preceding checkout contains only a copied
existing standard baseline and an already accepted executable. It has no new queue
implementation or accepted queue tests and is not a delivered language feature.
Explicitly denied compiler/planner and owned-type specification inspections were
not retried by another route. Existing stashes and unrelated services remain intact.
