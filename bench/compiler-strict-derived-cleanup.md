# Completed cleanup of the accepted strict-derived replay

The separately released cleanup completed **once, successfully**, after the
prospective scale measurements ended. Only the accepted owned fixture was
removed:

```text
/home/coder/workspace/lkjscript-compiler-strict-derived-20261116/target/compiler-strict-derived.ZQv7cu
```

It contained **3,706 regular files and 71,949,767 logical bytes**: the copied
producer, two archived trees, private store/key and owner/completed transaction
metadata. The parent `target/` directory remains and is empty. No other fixture,
canonical image, source tree, service, key or directory was removed.

## Invocation and measured cleanup cost

Root reviewed the [prepared guard and plan](compiler-strict-derived-cleanup.plan.md)
at commit `d74b638e4f20b9772d9f34f1f144148a021ccac4` and explicitly released:

```sh
perl bench/compiler-strict-derived-cleanup.pl --execute-after-root-release
```

The invocation ran once through gateway request
`compiler-strict-derived-cleanup-once-20261116-035`, under the retained
GNU-time wrapper. No refusal, fallback deletion, compiler command or recovery
invocation occurred.

| Recorded observation | Value |
| --- | ---: |
| Exit status | 0 |
| GNU elapsed / user / system | 0.29 / 0.16 / 0.09 s |
| Peak RSS | 10,524 KiB |
| Load before, 1 / 5 / 15 minutes | 0.89 / 1.41 / 1.60 |
| Load after, 1 / 5 / 15 minutes | 0.89 / 1.41 / 1.60 |
| UTC before / after | 2026-11-16 11:58:47 / 11:58:47 |
| Internal guard interval | 0.285055876 s |
| Launch-inclusive wrapper timestamp interval | 0.301524734 s |
| Same-user processes / descriptors inspected | 40 / 580 |

These are cleanup costs only. Raw launch command, release, stdout/stderr,
status, GNU time, nanosecond timestamps and paired loads are retained in
[the completion records](compiler-strict-derived-cleanup-results/).

## Guards, credentials and preserved records

The guard matched the prepared owned inode/content inventory, owner marker and
original PID absence; rejected mounts, links and special entries; required no
pending intent or staging leaf; and found no live same-user process or
descriptor reference. It did not read the private key contents. The key was
deleted together with its private store after the accepted transaction's
recovery obligation had ended.

The byte-identical completed metadata copies remain outside the removed
fixture:

| Preserved leaf | Bytes | SHA-256 |
| --- | ---: | --- |
| `compiler-strict-derived-cleanup-data/metadata/before.lock` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `compiler-strict-derived-cleanup-data/metadata/before.generation` | 64 | `f52b6b5cabe22492800a3d6060dd8b52cc8070c1df2b7e373e17fcdc58301851` |

Their before/after hash lists compare exactly. The prepared source excerpts
bind the credential conclusion to the actual pinned writer: a completed
generation comes from entropy, and `stamp` reads it without a private key.
Pending intents would require the key; none existed and no pending metadata
was removed as an exception.

Post-removal verification passed for all **57 result records**, the
35 preparation records, nine producer/source-control records, nine cleanup
preparation records and 22 historical-screen manifest entries. These counts
are manifest entries and overlap; they are not a sum of unique files.
Fixture absence was checked by both the guard and the separate post-check.

The earlier result report, raw replay proof, all old zero-of-six/five-of-six
records, the static historical investigation and every frozen manifest retain
their original bytes. This completion record updates retention status without
rewriting a campaign observation. The historical source archives and key can
no longer be used for another replay; the accepted textual proof remains.
