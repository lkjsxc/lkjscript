# Six recent strict replay cases: predeclared protocol

This preparation has executed no compiler or replay. Launch is prohibited until
the root session explicitly releases one worker after the full gate. The gate
producer pin is recorded now so the later measurement cannot silently select a
different implementation.

## Frozen population and producer

The history horizon is `6a187b94fb763f7b0565436a98ab5feda3cd642f`.
The exact `git rev-list --max-count=300` output is
`compiler-strict-recent.window`, SHA-256
`bee88306e1c12adfb3e0f8dfc6a96bf71c6b8ad344d4d08c2cdb0b808293cd20`.
The six chronological revisions are fixed in `compiler-strict-recent.order`,
SHA-256 `dbefff80f9fd1164e81bf429a73a5978ef783868312309578e06cdf5ea1e4d21`.
There is no adaptive replacement, threshold search, retry, resume, or quota stop.

The released candidate is expected to be the root session's
`/home/coder/workspace/lkjscript/target/lkjscript`, 31,716,730 bytes,
SHA-256 `adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7`,
built from `4f8c213559b7de96b3b6bd6199da661dfc95f778` (production change
`08b94d9a`). The harness requires those exact producer and source pins, copies
the binary into its owned fixture, verifies the copy, and runs only that copy.
It records the supplied build source commit and its `src` and `library` Git
tree identities. That source association is the owner's gated-build provenance;
it is not inferred from an unsigned string embedded in an arbitrary executable.
The full gate is still pending at protocol preparation.

## Fixed sequence and source boundaries

For each revision in the fixed order, compare its first parent to that revision.
Materialize exact Git bytes from only `src/`, `library/`, `tools/`, and
`tests/`. Each case has separate `case-NNN/before` and `case-NNN/after`
directories. Keep all six case trees through independent evidence review.
Archives containing symlinks, missing required roots, or forbidden paths refuse.

Every case uses `src/main.lkj` and the physical historical
`library/lib.lkj` on each side. Within each selected pair the library Git tree
is unchanged. No source adaptation, legacy extension copy, synthetic root,
configuration switch, fallback library, omitted off-root language file, or
historical Rust compilation is allowed. The exact historical logic remains the
input to the current producer; an admission refusal is a result.

The compiler command for a case is exactly:

```text
COPIED_PRODUCER edit src/main.lkj
  --source-dir CASE/before
  --against CASE/after/src/main.lkj
  --library-dir CASE/before/library
  --against-library-dir CASE/after/library
  --inline-limit 0
  --store FIXTURE/store --store-bytes 2147483648
  --store-key FIXTURE/key --stats
```

The actual argument vector is retained as NUL-delimited bytes for every launched
command. There is no runtime selector and no native historical compiler image
is emitted or executed by this replay command. The contract compares original
checked programs through the canonical checked-meaning domain; explicit
`--inline-limit 0` also prevents a producer option change from obscuring that
choice. The physical source/library selectors and root are part of the record.

There is one initially empty 2 GiB authenticated store shared across the six
commands, with one random private key outside every historical tree. The store
is not reset between cases. Filesystem caches are not reset. These are
single-observation correctness replays, not independent cold timing samples.

## Resources, errors, and timing

One compiler worker, external 4 GiB address-space limit, and a 900 s deadline per
compiler command are fixed. Every command runs through GNU time, prlimit, and
timeout. The command has a private process group. After its direct child exits, and on
interruption, cleanup sends TERM, polls for up to one second, then sends KILL
and polls for up to two seconds. If a live member of that owned group remains,
further launches stop and the fixture is retained; that explicit failure is not
reported as successful cleanup. Exited zombies are excluded from the live-member
check. A stopped group does not launch a concurrent recovery command. The harness does not relax a limit or infer
out-of-memory from an unexplained process loss.

Before initial setup and each archive, record free bytes and a conservative
reserve: twice the 2 GiB store capacity, twice the filesystem-block-rounded
before/after archive sizes with a 128 MiB floor, one copied producer, and
128 MiB for logs. Previously retained cases already reduce the observed free
space. A failed reserve, archive/setup error, timeout, allocation refusal,
fatal process exit, or invalid success proof stops further launches. All later
fixed candidates are recorded as not run. Only exit 1 with a named ordinary edit/replay/admission diagnostic continues:
the exact accepted diagnostic list is fixed in the harness. All other statuses
and unclassified errors stop. Contradictory internal replay source/span facts
also stop. Every refusal remains in the table.

Retain stdout, stderr, exit status, GNU wall/user/system/peak-RSS values, launch
start/end nanoseconds, and all three load averages immediately before/after
every executed command. Launch-inclusive timestamps bracket wrapper overhead and the bounded group cleanup;
GNU time describes its measured subprocess tree. Do not select or retry a
latency. Preserve an interrupted command's partial logs and explicit incomplete
metadata.

If a failed command leaves a publication intent, attempt one bounded
`edit --recover --no-store` cleanup action with the same key, recording its
own arguments, output, time, and load. Recovery does not retry or accept the
failed replay. An unrecovered intent stops the sequence and leaves the owned
fixture for inspection.

## Acceptance and claim boundary

A launched case passes only when all of these hold:

1. The command succeeds and reports one applied transaction, with one nonempty
   replay proof whose desired and candidate canonical checked-meaning hashes
   are equal.
2. Every changed path under `src/` or `library/`, plus every changed
   `.lkj` or `.rs` path anywhere in the commit, is covered by an applied
   file fact. The complete Git changed-path list and required subset are both
   retained. None of the six selected cases requires a filename projection.
3. Each reported before hash equals the actual first-parent Git blob. Each
   reported after hash equals the published file bytes read immediately after
   the transaction. All facts are inside the exact historical before tree;
   duplicate or missing facts refuse. A successful transaction has no pending
   publication intent.
4. The post-run independent evidence auditor recomputes fixed-order membership,
   arguments, raw measurements, file hashes, proof counts, and the complete
   required path set.

This establishes **complete changed-language coverage plus selected-root
canonical checked-meaning identity**, in domain
`lkjscript-checked-meaning-1`. It does not establish raw serialized IR byte
identity, equivalence of every possible root/configuration, or reproduction of
non-language Git effects. Exact published text equality to the desired commit
is recorded separately; it is not an acceptance prerequisite.

The prior 23 imported strict proofs and the one success from the separate
four-case probe remain unchanged, with their original
`4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09`
producer. They all remain inside the newly named 300-commit horizon and are
disjoint from these six. The resulting union is `24 + accepted new cases`;
**30 is conditional on six actual passes**, not a preparation result. The
original four-case experiment and its three refusals are not rerun or erased.

## Run, audit, and cleanup

After explicit worker release, run the checked-in harness once with a new output:

```sh
bench/compiler-strict-recent.sh /home/coder/workspace/lkjscript \
  /home/coder/workspace/lkjscript/target/lkjscript \
  adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7 \
  4f8c213559b7de96b3b6bd6199da661dfc95f778 \
  bench/compiler-strict-recent.tsv \
  > bench/compiler-strict-recent.launch.log \
  2> bench/compiler-strict-recent.launch.err
perl bench/compiler-strict-recent-audit.pl --report \
  /home/coder/workspace/lkjscript bench/compiler-strict-recent.tsv \
  > bench/compiler-strict-recent.proofs.tsv \
  2> bench/compiler-strict-recent.audit.log
```

The immutable preparation manifest covers this protocol, the harness, auditor,
static inventory script, and frozen inventory files. Runtime records receive a
separate manifest and report; no frozen historical table or preparation evidence
is overwritten. The fixture remains until the root session accepts the evidence.
Then the harness's guarded `--cleanup OWNED_FIXTURE` mode removes only its owned
source trees, copied producer, store, and private key after confirming no live
user. Preserve textual arguments, requests, hashes, refusals, and all results.
