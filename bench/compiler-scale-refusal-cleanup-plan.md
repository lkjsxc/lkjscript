# Guarded completion of the current-producer scale refusal

This is preparation for cleanup, not an execution record. The original result
and historical evidence remain unchanged. Only the three exact owned paths
below are candidates; no compiler, generator, query, benchmark or profile is
part of this procedure.

| Candidate | Exact path |
| --- | --- |
| Run | `/home/coder/workspace/lkjscript-compiler-scale-refusal-20261116/target/compiler-scale-refusal-run.tCSZMe` |
| External key directory | `/tmp/compiler-scale-refusal-key.aD8jW6` |
| Target evidence mirror | `/home/coder/workspace/lkjscript-compiler-scale-refusal-20261116/target/compiler-scale-refusal-evidence.62fa76f4-1` |

All commands run in
`/home/coder/workspace/lkjscript-compiler-scale-refusal-20261116`.
The existing worktree remains; the canonical producer in the main worktree
and all other fixtures remain outside this procedure.

## Acceptance and immutable inputs

Root accepted result commits `a1120d9a9c6fcca11f207df0f2dfd79dc027b954` and
`379f456840d21f7a1441fb3fd0e8cf3a08f90acf` at main commit
`dc0a325e19367b3563bc2d35316804be8ee2fa51`. The acceptance document SHA-256 is
`ffbdd3def42c20a19d1dc3a4c06c6cb90f428550ec2d21385d4a868adc96bc1a`;
its 11-record root manifest is
`c2ac940ba56471f9f779d871ea21c559c5574c590eb91655ccbc8403d2a782e8`.
Root independently verified all six captured phases and 2,294 live input file
instances, and explicitly reported no remaining live-fixture dependency.
The measured result manifest remains the accepted 106-record
`24574f4eb2e85e21e5a8f3d190c7aa143c45012af140a5773e1230fa99bf3085`.

The already frozen cleanup entry point is
`bench/compiler-scale-refusal.sh`, SHA-256
`3953f766928e37ceb6321d90210e63c827c31821357c14fd73c033efa62f1a70`.
It removes the owned run and key after its existing path, ownership, dead
supervisor and live-reference checks. It writes six recovery records under
the target evidence directory before completing: four owner files,
`store-apparent-bytes`, and `action`.

The new [read-only guard](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-scale-refusal-cleanup-check.pl), SHA-256
`e8585f1416c8a671819e4deb8eec727bc60b4aaef8b8c443f5f967f930565a6e`,
performs no cleanup. Its `before` mode binds the accepted result and root
acceptance identities, verifies the 92 original mirror files against their
committed copies, and checks complete supervisor/ownership records. It checks
fixture/key/evidence metadata for symlinks, extra hard links, special files,
foreign ownership, cross-device entries and nested mounts. It checks absence
of store publication staging, the successful control's zero pending checkpoint,
and both empty refusal stores. It inventories live command-line references
with the established own-process/ancestor exclusion. Private key bytes are
never read or copied.

Its `mirror` mode additionally requires the run and key to be absent and
exactly six cleanup records to match committed counterparts. The final target
tree must contain exactly 98 regular files in its two expected directories:
92 original files and those six completion records. The guard rejects extra
files or directories, including empty unexpected directories.

These are bounded ownership and execution-time checks. No new observer or
concurrent-mutation experiment is introduced. Explicit root release remains
required before either removal step.

## Step 1: existing frozen run/key cleanup

After root reviews this exact preparation and releases the action, create the
new completion-record destination and run the read-only guard again:

```sh
set -euo pipefail
mkdir bench/compiler-scale-refusal-cleanup-data/execution-1
perl bench/compiler-scale-refusal-cleanup-check.pl before \
  > bench/compiler-scale-refusal-cleanup-data/execution-1/before.json
```

Then execute exactly this existing cleanup subcommand, once, retaining its
stdout, stderr and status under `execution-1`:

```sh
bash bench/compiler-scale-refusal.sh cleanup \
  target/compiler-scale-refusal-run.tCSZMe
```

A refusal or incomplete action stops the procedure. Do not retry with another
path or bypass a guard. Require successful exit and absence of both run and key.

## Step 2: commit the newly produced completion records

Copy only the six named files from the target evidence's `cleanup/` directory
to the new
`bench/compiler-scale-refusal-cleanup-data/frozen-cleanup/` directory.
Retain the complete Step 1 stdout/stderr/status and UTC completion observation
under `execution-1`. Check the action's start/completion records and exact
run/key paths, and record the post-action absence checks.

Create a separate completion manifest and commit those new records before
considering disposal of the target evidence mirror. Do not edit the original
92 captured files, the old launch record, the accepted result report or any
historical manifest. The original launch record correctly describes the state
at the end of measurement.

## Step 3: separately proven disposal of the mirror

Only after the six cleanup records are committed, and after the released
mirror-disposal condition is satisfied, run:

```sh
perl bench/compiler-scale-refusal-cleanup-check.pl mirror \
  > bench/compiler-scale-refusal-cleanup-data/mirror-before.json
```

On exit zero, remove only the exact target mirror:

```sh
rm -rf -- target/compiler-scale-refusal-evidence.62fa76f4-1
```

Retain the removal command's result, UTC completion and final absence checks
in a separate completion record and commit it with `mirror-before.json`.
The checked 92 original files and six recovery files remain in Git.

No cleanup has run as of this plan's creation. Preparation output is retained
under [compiler-scale-refusal-cleanup-data](compiler-scale-refusal-cleanup-data/).
A preparation success is not an authorization to remove anything.
