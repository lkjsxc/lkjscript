# Completed cleanup of the accepted scale-refusal cohort

Root accepted the single measured cohort at
`dc0a325e19367b3563bc2d35316804be8ee2fa51`, reviewed the frozen cleanup and
read-only guard, and explicitly released the owned run/key cleanup plus
conditional disposal of its target evidence mirror. Both actions completed
once with exit zero. All three exact owned paths are absent.

The original result report and its 106-record manifest remain unchanged.
The original 92 raw evidence files and all six new cleanup records remain
committed outside the removed target paths.

## Actions and capture

| Action | Completion UTC | Wall / user / system s | Peak RSS KiB | Load before -> after, 1/5/15 min | Wrapper / GNU exit |
| --- | --- | --- | ---: | --- | --- |
| Frozen run/key cleanup | 2026-11-16T12:19:21Z | 0.05 / 0.00 / 0.04 | 3,876 | 1.96/1.92/1.79 -> 1.96/1.92/1.79 | 0 / 0 |
| Conditional target-mirror disposal | 2026-11-16T12:20:21Z | 0.00 / 0.00 / 0.00 | 1,620 | 1.67/1.87/1.78 -> 1.67/1.87/1.78 | 0 / 0 |

GNU time records two decimal places here; `0.00` retains that capture precision.
These are cleanup observations, not additional compiler measurements.
Raw stdout/stderr, exact command/cwd, status, UTC bounds, time and load records
are retained under
[execution-1](compiler-scale-refusal-cleanup-data/execution-1/) and
[mirror-1](compiler-scale-refusal-cleanup-data/mirror-1/).
Both stderr captures are empty and both gateway operations finished normally.

The commands were:

```sh
bash bench/compiler-scale-refusal.sh cleanup target/compiler-scale-refusal-run.tCSZMe
rm -rf -- target/compiler-scale-refusal-evidence.62fa76f4-1
```

Their gateway request identities are
`compiler-scale-refusal-cleanup-047-execute-owned-run-key` and
`compiler-scale-refusal-cleanup-049-execute-conditional-mirror`.
No command was retried and no broader path was removed.

## Metadata scope

| Removed owned path | Regular files | Directories | Logical file bytes |
| --- | ---: | ---: | ---: |
| `target/compiler-scale-refusal-run.tCSZMe` | 2,309 | 27 | 146,363,341 |
| `/tmp/compiler-scale-refusal-key.aD8jW6` | 2 | 1 | 137 |
| `target/compiler-scale-refusal-evidence.62fa76f4-1` | 98 | 2 | 309,526 |
| Total | 2,409 | 30 | 146,673,004 |

The counts and byte sums use file metadata only. Logical bytes are the sum of
regular-file `st_size`, excluding directory metadata; they do not assert a
measured amount of filesystem space recovery. Private key bytes were not read,
hashed or copied. The target mirror initially held 92 original files totalling
308,668 logical bytes; the frozen cleanup added six records totalling 858 bytes.

## Guards and retention order

Preparation was committed at `2b5e38007c4fc76d5899782fcd3e21a1c54a6513`.
The released `before` guard passed immediately before cleanup: exact accepted
manifest and root acceptance, byte-identical committed raw counterparts,
completed supervisor and ownership records, absent live ownership references,
no symlinks/hard-linked regular files/special files/nested mounts, and no
pending store publication metadata. The existing cleanup then removed only
its owned run and paired external key.

The six recovery records were copied byte-for-byte to
[frozen-cleanup](compiler-scale-refusal-cleanup-data/frozen-cleanup/) and
committed at **`4e4dcc7b15add9c8884936c2af1ab11629b12ee5`** before the mirror
guard ran. That guard required the run/key to be absent and verified exactly
98 files in the two expected directories against committed counterparts.
Its success satisfied root's previously released mirror-disposal condition.

The retained [completion record](compiler-scale-refusal-cleanup-data/completion.json)
derives the captures and metadata totals and records final absence checks.
The accepted original manifest, five-record preparation manifest and
21-record first-completion manifest all still verify. The separate final
cleanup manifest binds this report and all retained cleanup records.

No compiler, generator, query, test, profile or additional workload ran during
cleanup. The main canonical producer, other fixtures, worktrees and historical
evidence remain outside this cleanup.
