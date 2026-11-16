# Six strict replays with an explicit desired source grant

This is a separate, predeclared campaign for the same six revisions as
[the completed six-refusal campaign](compiler-strict-recent.md). That campaign
remains **0/6**, under producer
`adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7`.
Its six original observations, empty proof table, archive audit and cleanup
records are unchanged. This preparation executes no compiler or replay.
The new campaign remains blocked until the root session supplies a successfully
gated producer pin and explicitly releases one worker.

## Intervention and provenance

The original commands gave the before tree an explicit absolute source grant,
but the desired tree used a relative entry under its process working directory.
The include-only runtime consequently had a relative desired provenance label,
while stable standard-library items used an absolute library label. All six
commands refused `E_REPLAY_FILES` before deriving or applying operations.
All twelve physical runtime files had identical SHA-256
`eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14`.
The path-spelling explanation is a source-code trace; the exact refusals and
physical file hashes are retained observations.

The new CLI capability is `--against-source-dir`, introduced in
`b41d3746496236e8a9d19973a04b1fd1ed81f5fb` and integrated as
`f48f14ffd4062e87f3cf16ebe29aa04298e66d63`. It gives edit replay and comparison
an explicit desired source Directory. An unavailable requested directory fails;
the option does not fall back to another source grant. No arbitrary filename
matching, historical source rewrite, or weakened meaning check is introduced.
The four checked item replacements and all fourteen retained language lines
were independently reconstructed from their parent Git blob and operation
request before this preparation.

The new condition adds `--against-source-dir CASE/after` to each command.
It uses a newly gated producer containing that API. The historical case
population, source bytes, root choices, library selectors, command order,
resource limits and acceptance rules remain fixed. This is a new API/provenance
experiment, not a continuation or replacement of the six failed observations.
Its timings must not be treated as paired speed measurements against them.

The exact producer is not selected during this preparation. A later
`compiler-strict-scoped.producer` file must contain exactly these eight
newline-terminated records, with actual full values supplied by the root's
successful-gate handoff:

```text
version=1
campaign=six-strict-cases-explicit-against-source-dir
producer_sha256=FULL_SHA256
producer_bytes=DECIMAL_BYTES
producer_source_commit=FULL_GIT_COMMIT
producer_src_tree=FULL_GIT_TREE
producer_library_tree=FULL_GIT_TREE
gated_commit=FULL_GIT_COMMIT
```

The associated `compiler-strict-scoped.producer.md` addendum records the
root's gate evidence, exact source/build association and release. The source and
gate commits must have identical `src` and `library` trees. The harness
checks those tree identities, supplied CLI pins, actual binary size and SHA-256,
then copies and rehashes the immutable executing producer. The association
between source and executable is the owner's gated-build provenance, not an
unsigned self-description from an arbitrary binary.

`compiler-strict-scoped.prepared.sha256` freezes the producer-independent
protocol, harness, auditors and historical population dependencies now.
After the gate, a separate `compiler-strict-scoped.launch-ready.sha256`
must bind that unchanged preparation manifest, the exact producer file and
the producer addendum. The harness refuses launch if either pin/addendum or
launch-ready manifest is absent or inconsistent. No placeholder producer file
is created during preparation.

## Frozen population and commands

The history horizon stays
`6a187b94fb763f7b0565436a98ab5feda3cd642f`.
The exact last-300-commit list remains `compiler-strict-recent.window`,
SHA-256 `bee88306e1c12adfb3e0f8dfc6a96bf71c6b8ad344d4d08c2cdb0b808293cd20`.
The unchanged six-ID order file is `compiler-strict-recent.order`,
SHA-256 `dbefff80f9fd1164e81bf429a73a5978ef783868312309578e06cdf5ea1e4d21`.

| Order | Revision | Complete changed-language paths |
| --- | --- | --- |
| 1 | `9047a677f9a353ae13883b4998adf9da2fc67b48` | `src/semantic_facts.lkj` |
| 2 | `de65c09bab9d336e10dcfe245058d4ee14b3aea2` | `src/structure_edit.lkj`, `src/structure_write.lkj` |
| 3 | `0a410badb7e96ed9f6cf185df7d1f781d71c5fdb` | `src/ast.lkj`, `src/semantic.lkj`, `src/source.lkj`, `src/structure_edit.lkj`, `src/structure_replay.lkj` |
| 4 | `c5f67a5583aaba6dd338ba3e7278fdc775f36731` | `src/structure_write.lkj` |
| 5 | `ba27e90add8c1a28e7d2c03984dbaa4aed1a8e72` | `src/semantic_store.lkj` |
| 6 | `71e821dfcaaf6a7940ed67bb7d2d3a721c169922` | `src/driver.lkj`, `src/store.lkj` |

Use each revision's first parent as the before tree. Archive exact Git bytes
from `src/`, `library/`, `tools/` and `tests/` into fresh,
separate `case-NNN/before` and `case-NNN/after` directories. Reject
symlinks, forbidden paths and missing roots. There is no historical-language
adaptation, filename projection, synthetic root, configuration switch, omitted
off-root language path, fallback library or historical Rust compilation.

Every command uses the existing `src/main.lkj` root and exact physical
historical library on each side. The library tree is unchanged within each pair.
The exact command, from physical cwd `CASE/after`, is:

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

All CASE/FIXTURE paths are canonical absolute paths. Both entry arguments remain
relative. Retain each actual argument vector as NUL-delimited bytes, requested
cwd, physical cwd captured inside the measured subprocess, and canonical grant
table. A failed `cd` or physical-cwd mismatch exits before executing the
compiler. The key must be outside both explicit source roots and both explicit
library roots. No desired source Directory `/` or implicit source grant is
used. Keeping cwd equal to the desired tree holds the remaining invocation
convention constant.

There is no runtime selector and no native historical compiler image is emitted
or executed. Replay compares original checked programs through the canonical
checked-meaning domain. A single initially empty 2 GiB authenticated store and
one private key are shared across the six commands. Nothing from the original
campaign's removed fixture is reused. Neither the store nor filesystem caches
are reset between cases. These are single-observation correctness replays,
not independent cold-build timing samples.

## Resources and failure policy

One compiler worker, an external 4 GiB address-space limit and a 900-second
deadline per compiler command are fixed. Every command uses GNU time, prlimit,
timeout and a private process group. After the direct child exits, or on
interruption, cleanup sends TERM and polls for one second, then KILL and polls
for two seconds. A remaining live group member or failed inspection stops the
campaign and retains the fixture. Exited zombies are not live workers.
No additional compiler or recovery command starts while a group remains active.

Before setup and each archive, record available bytes and the same conservative
reserve as the prior campaign: twice the 2 GiB store budget, twice the
filesystem-block-rounded before/after archive size with a 128 MiB floor, one
copied producer and 128 MiB for logs. Previously retained cases already reduce
reported free space. Do not relax limits, infer OOM from unexplained loss, or
replace a refused case.

Only exit 1 with a named ordinary edit/replay/admission diagnostic may continue;
the exact recognized list remains in the harness. Allocation refusal R0006
anywhere in stderr, every other process status, unclassified failure,
contradictory internal source/span facts, invalid success proof, setup/resource
refusal or undrained process group stops further launches. Later fixed cases are
recorded as not run. No retries, resumption, case substitutions, latency
selection or quota stop is allowed.

Retain every stdout/stderr, exit code, GNU wall/user/system/peak-RSS value,
launch timestamp pair and all three load averages immediately before and after
each command. Launch timestamps include wrapper overhead and bounded group
cleanup; GNU time covers its measured subprocess tree. Missing launch/time
evidence is retained as incomplete, never manufactured into a timing.
Preserve interrupted partial records.

A failed command with a pending publication intent may have one separately
recorded, bounded `edit --recover --no-store` cleanup action with the same
key, cwd and explicit before root. It cannot convert a failed replay into a
pass. A remaining intent stops the sequence.

## Acceptance and count boundary

A case passes only if the command exits zero, reports exactly one applied
transaction, and reports a nonempty replay proof with equal desired/candidate
canonical checked-meaning hashes. Every changed path under `src/` or
`library/`, plus every changed `.lkj` or `.rs` path anywhere in
the commit, must have a published file fact. All twelve selected path
occurrences remain required; no off-root file can be dropped.

For every fact, the before hash must equal the first-parent Git blob and the
after hash must equal the actual published bytes. Facts must be unique and
inside the exact before tree. A pass has no pending intent. The independent
auditor checks the complete Git changed set, fixed order/window, producer pin,
exact argv/cwd/grants, raw time/load records and every publication hash.
The separate retained-archive auditor compares desired archives to Git and
before archives to their Git parent plus only the explicitly proven published
file changes; every other archived byte/path must be unchanged.

The claim is **complete changed-language coverage plus selected-root canonical
checked-meaning identity**, domain `lkjscript-checked-meaning-1`. It is not
raw serialized IR byte identity, equivalence of all roots/configurations or
reproduction of non-language Git effects. Exact published target-text equality
is recorded separately and is not an acceptance prerequisite.

The original 23 imported strict proofs and separate four-case probe's one
success retain their exact `4c4537ea…99e09` producer and original evidence.
All 24 remain in the named window and are disjoint from these six. The old
six-refusal cohort contributes zero. The possible unique union is
`24 + newly accepted cases`; **30 requires six actual new passes**.
The four-case probe and both its successes/refusals are unchanged.

## Launch, audit and cleanup

After the future producer addendum and explicit worker release, invoke the new
harness exactly once with the released binary and full source/hash pins:

```sh
bench/compiler-strict-scoped.sh /home/coder/workspace/lkjscript \
  RELEASED_GATED_BINARY RELEASED_SHA256 RELEASED_SOURCE_COMMIT \
  bench/compiler-strict-scoped.tsv \
  > bench/compiler-strict-scoped.launch.log \
  2> bench/compiler-strict-scoped.launch.err
perl bench/compiler-strict-scoped-audit.pl --report \
  /home/coder/workspace/lkjscript bench/compiler-strict-scoped.tsv \
  > bench/compiler-strict-scoped.proofs.tsv \
  2> bench/compiler-strict-scoped.audit.log
perl bench/compiler-strict-scoped-state.pl \
  /home/coder/workspace/lkjscript bench/compiler-strict-scoped.tsv \
  > bench/compiler-strict-scoped.state-audit.log
```

The original harness, TSV, proofs, reports and manifests under
`compiler-strict-recent*` are immutable dependencies, never output paths.
New results receive their own report and manifest. Keep the new owned
`target/compiler-strict-scoped.XXXXXX` fixture until root accepts its
evidence. Then its guarded cleanup mode may remove only that fixture's copied
producer, archives, store and key after confirming no live user; preserve all
textual evidence and record actual cleanup.
