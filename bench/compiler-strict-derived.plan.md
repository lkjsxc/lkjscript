# One strict replay after validated derived-source provenance

This is a separately predeclared correctness campaign for the original scoped
case **3**, revision `0a410badb7e96ed9f6cf185df7d1f781d71c5fdb`.
It is prepared without running a compiler, test, or historical replay.
Execution remains blocked until the root supplies the combined gate's canonical
producer pin and explicitly releases one worker.

## Existing observations and intervention

The first six-case campaign remains **0/6**, under producer
`adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7`.
Its path-scope refusals, exact inputs, empty proofs and cleanup remain unchanged.

The separate explicit-source-scope campaign remains **5/6**, under producer
`ab2fa80787261e2fec6d872d989099d3b1dd0984c26ac59bf89c7237d9e97504`.
Its third case refused:

```text
"<ast::Program as std::prelude::Default>::default" E_EDIT_SYNTAX
expected `{`, found `struct`
```

That command did not publish a transaction. Its full source archives were
independently audited unchanged and subsequently cleaned after acceptance.
The five accepted cases retain their 45 operations and seven required file
facts. The original TSV, proof table, report and all refusal records are immutable
dependencies of this preparation.

The new producer must include the validated generated-source repair, source
commit `f6e5ea361dd9051da026ca0bf2bae71b2d874563`, integrated as
`68824804fb35ae1d1cc4109343a9abadafaa090c`. The repair preserves semantic
function, implementation and instance identities, validates the parser's
complete authored Item relation and exact Default expansion, separates
generated references from editable source references, and lets replay select
authored declarations. It does not manufacture function text from the defining
struct. Its focused and actual-compiler catalog controls are recorded separately
in [the repair report](compiler-generated-source.md).

This campaign uses a newly gated producer and records its complete source
identity, including other integrated changes such as the query container.
It does not isolate a performance effect of the provenance repair, and no
success is assumed from the static diagnosis. It is not a resumption,
replacement, or correction of either earlier campaign's observations.

## Exact selected history and scope

The named history horizon remains
`6a187b94fb763f7b0565436a98ab5feda3cd642f`, with its original 300-commit
window, SHA-256
`bee88306e1c12adfb3e0f8dfc6a96bf71c6b8ad344d4d08c2cdb0b808293cd20`.
This is a named historical window, not a claim about a later moving last-300
window.

The only selected revision and first parent are:

| Record | Value |
| --- | --- |
| Original ordinal | 3 |
| Revision | `0a410badb7e96ed9f6cf185df7d1f781d71c5fdb` |
| First parent | `579cf55e88177b9c9a4c59e2ff6c1bf7829e2878` |
| Selected root, both sides | `src/main.lkj` |
| Before src tree | `1988a6752cb324929d1dacad319341a99cf7ac94` |
| Desired src tree | `3755d0d418c1834c2303c410d4ef14bb3f2b5f2a` |
| Library tree, both sides | `e3ba335f2eeb20be7cf05404ecf97ba9c230e7ab` |

All five changed-language paths are required:

- `src/ast.lkj`
- `src/semantic.lkj`
- `src/source.lkj`
- `src/structure_edit.lkj`
- `src/structure_replay.lkj`

`compiler-strict-derived.candidates.tsv` is exactly the original candidates
header plus ordinal 3's complete row. Original ordinal 3 is retained in the
new output and fixture directory; there is one new observation, not three.
Both harness and auditor check this exact subset. No replacement candidate,
off-root omission, configuration change, language adaptation or compatibility
rewrite is permitted.

Archive exact Git bytes from `src/`, `library/`, `tools/` and `tests/`
into separate fresh `case-003/before` and `case-003/after` directories.
Reject forbidden paths, symlinks and missing roots. Do not emit or execute a
historical native compiler image.

## Frozen command and grants

Use the same explicit source-scope API as the five-pass campaign. The exact
compiler command, from physical cwd `CASE/after`, is:

```text
COPIED_PRODUCER edit src/main.lkj
  --source-dir CASE/before
  --against src/main.lkj
  --against-source-dir CASE/after
  --library-dir CASE/before/library
  --against-library-dir CASE/after/library
  --inline-limit 0
  --store FIXTURE/store --store-bytes 2147483648
  --store-key FIXTURE/key --stats
```

All CASE/FIXTURE paths are canonical absolute paths. Both entry arguments are
relative. Retain actual arguments as NUL-delimited bytes, requested cwd,
physical cwd captured inside the measured subprocess, and the canonical
four-grant table. A failed cd or mismatched physical cwd exits before compiler
execution. The key is outside both source roots and both library roots.
The desired source directory is explicit; no implicit `/` grant is used.

One fresh private key and an initially empty 2 GiB authenticated store are
owned by this single-case fixture. No earlier fixture, store, proof artifact or
source projection is reused as compiler input. Filesystem caches are not reset.
Unlike the earlier scoped case 3, this command is not preceded by two other
replays in the same store. Its single observed time cannot be treated as a
paired speed comparison with that earlier refusal.

## Producer pin and release

No placeholder producer is created during preparation.
After the combined gate succeeds, the root's addendum must provide exactly
these eight newline-terminated records in
`compiler-strict-derived.producer`:

```text
version=1
campaign=one-strict-case-validated-derived-origin
producer_sha256=FULL_SHA256
producer_bytes=DECIMAL_BYTES
producer_source_commit=FULL_GIT_COMMIT
producer_src_tree=FULL_GIT_TREE
producer_library_tree=FULL_GIT_TREE
gated_commit=FULL_GIT_COMMIT
```

The source and gate commits must have identical src and library trees.
The harness checks those identities, full supplied pins and actual executable
size/hash, then copies and rehashes the executing producer into its owned
fixture. The source-to-image association comes from the root's gated-build
record, not an unsigned binary self-description.

`compiler-strict-derived.prepared.sha256` binds this producer-independent
plan, harness, both auditors, exact candidate subset and prior immutable
population/proof dependencies. A future
`compiler-strict-derived.producer.md` records actual build/gate provenance
and the exact launch command. A separate
`compiler-strict-derived.launch-ready.sha256` then binds the unchanged
preparation manifest, producer record and addendum. All three are required
before launch; the root's explicit worker release is recorded separately.
No compiler invocation is authorized by this preparation alone.

## Resources, failures and retained evidence

Use one compiler worker with an external 4 GiB address-space limit and
900-second deadline, GNU time, prlimit, timeout and a private process group.
The inherited process cleanup sends TERM, polls for one second, then KILL
and polls for two seconds. A remaining live group member or failed process
inspection stops work; zombies are not running workers. Do not launch any
additional compiler or recovery command while the group remains active.

Before setup and the archive, retain free bytes and the conservative reserve:
twice the 2 GiB store budget, twice the filesystem-block-rounded pair of
archives with a 128 MiB floor, the copied producer and 128 MiB for logs.
Do not relax either limit or infer OOM from unexplained disappearance.

Exactly one compiler replay is planned. Preserve every setup refusal, partial
launch, process status, stdout/stderr, timing, load and proof failure. There are
no retries, resumption, substitutions, threshold choices, quota stops or timing
selection. Exit 1 with a recognized ordinary edit/replay/admission diagnostic
may be classified as a semantic refusal; allocation R0006 anywhere in stderr,
other process statuses and unclassified failures remain stopped failures.
Contradictory source/span facts or invalid success proofs are implementation
failures. None becomes a pass.

Record wall/user/system/peak RSS, launch timestamps, and all three load
averages immediately before and after the command. Launch timestamps include
wrapper overhead and bounded group cleanup; GNU time covers the measured
subprocess tree. Missing evidence remains incomplete.

If a failed command leaves a publication intent, one separately recorded
bounded `edit --recover --no-store` action may clean it using the same key,
cwd and explicit before source root. Recovery is not a replay and cannot
change the observation into a pass. No further replay follows it.

## Proof and union boundaries

A pass requires exit zero, exactly one applied transaction, a nonempty replay
proof, and equal desired/candidate canonical checked-meaning hashes.
Every changed path under src/library, plus every changed .lkj or .rs file
anywhere in the commit, must have a unique published file fact. All five
required files must be covered. Each before hash must equal its first-parent
Git blob; each after hash must equal actual published bytes. A pass has no
pending intent.

The proof auditor verifies complete Git scope, fixed ordinal/revision,
named-window membership, producer pin, exact argv/cwd/grants, raw timing/load
records and publication hashes. The separate archive auditor compares both
full archive path sets and every file: desired bytes equal Git, and before
bytes equal the first parent except for explicitly proven publication changes.
Runtime include expressions and runtime hashes are recorded separately.
Exact target-text equality is also recorded separately and is not an
acceptance prerequisite.

The claim remains **complete changed-language coverage plus selected-root
canonical checked-meaning identity**, domain
`lkjscript-checked-meaning-1`. It is not serialized IR byte identity,
equivalence across every configuration/root, or reproduction of non-language
Git effects.

The existing 23 imported strict proofs and the separate four-case probe's one
success retain their original `4c4537ea…99e09` producer. The scoped campaign's
five successes retain `ab2fa807…97504`. All 29 distinct revisions remain
inside the named window. The selected revision is disjoint from those
successes. The original 0/6 and scoped 5/6 totals never change.
**The union remains 29 unless this new observation passes; 30 is conditional
on its actual complete proof.**

## Launch, audit and cleanup

Only after the future producer addendum and explicit release:

```sh
bench/compiler-strict-derived.sh /home/coder/workspace/lkjscript \
  RELEASED_GATED_BINARY RELEASED_SHA256 RELEASED_SOURCE_COMMIT \
  bench/compiler-strict-derived.tsv \
  > bench/compiler-strict-derived.launch.log \
  2> bench/compiler-strict-derived.launch.err
perl bench/compiler-strict-derived-audit.pl --report \
  /home/coder/workspace/lkjscript bench/compiler-strict-derived.tsv \
  > bench/compiler-strict-derived.proofs.tsv \
  2> bench/compiler-strict-derived.audit.log
perl bench/compiler-strict-derived-state.pl \
  /home/coder/workspace/lkjscript bench/compiler-strict-derived.tsv \
  > bench/compiler-strict-derived.state-audit.log
```

These are new output paths. Earlier recent/scoped artifacts are immutable
inputs, never outputs. Preserve the newly owned
`target/compiler-strict-derived.XXXXXX` fixture until root acceptance of
both the proof and live archive audits. Only then use the inherited guarded
cleanup to remove that fixture's producer, archives, store and key after
checking no live user; preserve all textual evidence and record cleanup.
