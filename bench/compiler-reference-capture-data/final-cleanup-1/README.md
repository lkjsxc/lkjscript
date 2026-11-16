# Final finite reference-capture cleanup preparation

**Prepared only. The cleanup helper has not run, and no deletion or compression is released by this bundle.** The release template is deliberately false and contains a pending approval commit/reference. Syntax has not been executed during this preparation; root or its authorized reviewer must perform that check before a future run.

## Exact scope and retained recovery

The scope is the ten directory roots and nine literal top-level files in `expected-scope.json`, all directly under `/home/coder/workspace/lkjscript/target`. The expected population is **4,532 regular files, 354,054,095 logical bytes and 145 directories**. There is no prefix-based removal or recursive deletion primitive.

The seven original roots use the independently produced Perl map, SHA-256 `036832967ed5d4cc55e27c109bea315bac79877ad80636bfea5f81de3cca7bc1`. The three auxiliary roots and nine literal files use the direct committed coordinates in the expected scope. All current retention is pinned to `9326dbaab1bf6832b9f35dc156e12d0d62eccd9e`; original provider descriptors retain their exact `d84cac6e` and historical seed revisions.

The old Python proposal and its output files are inert data. The helper never executes that proposal or trusts its verdict. It independently reconstructs all selected recovery bytes using Git, Perl gzip decoding and bounded tar parsing, then compares every original file in full. No content is recompressed or copied into a new payload package.

The ninth direct copy is `bench/compiler-reference-capture-data/cleanup-mapping/compiler-reference-capture-source-commit-message.txt`, 905 bytes, SHA-256 `c17ba6faeef540b4eff3d8bee614df4320637400522ada508e2645801f423b6c`. It is bound directly to the retention commit rather than added retroactively to an earlier copy manifest.

## Guard behavior

The helper requires the exact repository cwd, exact release filename/arguments, explicit true execution and quiescence flags, a reviewed approval commit and matching guard/scope hashes. It compares the live guard, scope and auxiliary inventory with their exact Git blobs in that approval commit.

Before the first unlink, it verifies all ten complete directory populations, the nine literal files, every owner/mode/inode/link-count/size/time record and every full recovery byte. It retains directory descriptors for the repository, target parent and all selected directories, and rechecks the full population after the byte checks.

Deletion uses only enumerated `unlink` and deepest-first `rmdir`. Each file is checked again immediately before unlink. Parent descriptors and named directory identities are checked throughout. Only exact ENOENT results count as absence. Any new path, changed bytes/metadata, special file, hardlink, recovery mismatch or nonempty directory causes a stop.

These checks operate under root-confirmed cooperative quiescence. They do not claim an atomic transaction across nineteen filesystem scopes or an atomic defense against a concurrent hostile parent rename. Mutation intent and confirmed outcome records are separate; a stopped run preserves any pending intent whose exact final state may need read-only inspection. There is no automatic retry or rollback.

## Permanent capture and future invocation

The helper writes its invocation, copied release, progress and final/partial result beneath the permanent path:

`bench/compiler-reference-capture-data/final-cleanup-1/execution-1`

An existing execution directory refuses a subsequent run. No disposable target capture directory is created. Root should retain outer argv/cwd, UTC/nanoseconds, before/after load, GNU time, stdout/stderr and exit status under this same permanent bundle.

After root reviews, commits and explicitly releases the exact controls, the only supported invocation is:

```sh
perl bench/compiler-reference-capture-data/final-cleanup-1/guard.pl \
  --release /home/coder/workspace/lkjscript/bench/compiler-reference-capture-data/final-cleanup-1/release.json \
  --execute-after-root-release
```

The template must not be passed as an actual release. The final approval commit and explicit release reference are supplied by root after review.

The canonical `target/lkjscript`, unlisted target entries, credentials, stores outside the literal scopes, other projects and worktrees remain excluded. The helper performs no process, descriptor or syscall observation and no compiler/native execution.
