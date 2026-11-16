# Current compiler query campaign: guarded cleanup

This cleanup is limited to accepted result commit `6a834248a3b5e28912a034343de5640eff2f6a89` (integrated on main as `0e4c061b`) and its disposable owned artifacts. It is not another benchmark validation or execution. The completed historical case has released the conflicting workload; execution still requires the root agent's explicit release of the exact guard below.

## Exact scope

All paths except the private key directory are beneath `/home/coder/workspace/lkjscript-compiler-query-container-20261116/`. These metadata-only counts were observed before deletion; the guard requires the same owner, device/inode, counts and logical byte totals.

| Path | Files | Directories | Logical file bytes | Device / inode |
| --- | ---: | ---: | ---: | --- |
| `target/compiler-query-current-run.TDPm4g` | 1,879 | 50 | 891,045,312 | 144 / 36946644 |
| `/tmp/compiler-query-current-key.seAq58` | 2 | 1 | 139 | 144 / 36946645 |
| `target/compiler-query-current-62fa76f4` | 35 | 2 | 269,674 | 144 / 36946633 |
| `target/compiler-query-current-continuation-62fa76f4` | 380 | 2 | 1,308,050 | 144 / 36959687 |
| `target/compiler-query-current-revalidated-62fa76f4` | 22 | 2 | 46,274 | 144 / 36950088 |
| `target/compiler-query-current-revalidated-62fa76f4-2` | 22 | 2 | 46,278 | 144 / 36951040 |
| **Total** | **2,340** | **59** | **892,715,727** | UID 1001 throughout |

The two additional revalidated directories are owned offline-validation copies. Their 44 files (92,552 bytes) are checked against the committed `offline-validation-1` and `offline-validation-2` counterparts, not inferred from their names.

## Guard

`guarded-cleanup.pl` requires the explicit `--execute-reviewed-six-path-cleanup` argument and the exact accepted HEAD/cwd. Before any removal it:

- compares every file in the four disposable text trees (459 files) with both its accepted Git blob and retained working copy;
- checks the fixture/key reciprocal ownership markers, dead owner PID, and key metadata without reading any key bytes;
- checks the 1,860 source-file digests against the committed source manifest, and the four owned compiler/image copies against the accepted canonical digest;
- refuses symlinks, nonregular entries, changed scope counts/identities, or durable edit metadata in any removal tree;
- preserves and checks the four durable edit metadata files and the unrelated operation credential directory `/tmp/compiler-container-key.m4u6F6`;
- checks existing process argv/cwd references to the six paths, excluding only its verified ancestor chain; it neither signals nor traces a process.

The guard records its complete preflight before deleting the first path, records progress after each exact path, and checks preserved metadata afterward. Its process observation is a cooperative ownership check, not a guarantee against concurrent hostile replacement. The store is disposable cache; cleanup does not authenticate or read its whole container. No compiler, query, map walk, store census, retry or image execution is performed.

The root canonical executable, main worktree, all committed evidence/control files, global/main credentials, this worktree and durable edit metadata are outside the deletion scope. All failure outputs and any partial progress remain evidence.

After root release the single invocation is:

```sh
perl bench/compiler-query-current-cleanup/guarded-cleanup.pl --execute-reviewed-six-path-cleanup
```

Its wrapper records wall/resource time and load before/after. Only after successful cleanup will the result, exact absence records and proof manifest be committed.
