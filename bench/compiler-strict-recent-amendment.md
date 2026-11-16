# Prelaunch amendment v2: narrow desired-source grant

The original preparation is preserved at
`803c7367c8e7bd3858652001a98176ef68fdc15f`, including its original manifest
SHA-256 `22b6357b80bbfce2403f19ef354417640043f7f00e50a3c0272e164e72026b52`.
**No compiler or replay command from that preparation ran.**

Root's static prelaunch review found that its absolute desired-source argument
would have granted the desired loader a Directory for `/`. Although the key
was outside both historical filesystem trees, it was inside that broader source
grant. The original invocation therefore did not satisfy the private-key
placement rule and must not be launched.

## Pinned production-code trace

The inspected source files are byte-identical to the producer source at
`4f8c213559b7de96b3b6bd6199da661dfc95f778`:

| File | SHA-256 |
| --- | --- |
| `src/driver.lkj` | `0b9a7784c025264a8b63d5101cb1089e9567a4e735a19e6c014aac4f6088c1e1` |
| `src/driver/structure.lkj` | `b5511e8e6ee3ba68b4dd8e120f0e1d26d005b76b390efae9f3ed86b1a9afc411` |
| `src/structure_replay.lkj` | `70354495e313022941950de54a34750fc8f9ec7e823e9ad9e1f2734bb5d5f484` |
| `src/meaning.lkj` | `3079b5c92306abcdf9156e99bf970577174fadeda5818b4a80b5cce114adf6e1` |

`driver::structure::replay_target` copies the options, assigns the desired
`--against` path, and clears `source_directory`.
`driver::compile_snapshot` chooses Directory `/` for an absolute path when
that field is absent, and Directory `.` for a relative path. With an explicit
source directory it instead opens that directory and resolves the supplied entry
within it.

The amended command runs from the exact `CASE/after` directory:

```text
cwd = CASE/after
COPIED_PRODUCER edit src/main.lkj
  --source-dir CASE/before
  --against src/main.lkj
  --library-dir CASE/before/library
  --against-library-dir CASE/after/library
  --inline-limit 0
  --store FIXTURE/store --store-bytes 2147483648
  --store-key FIXTURE/key --stats
```

All `CASE/...` and `FIXTURE/...` arguments shown here are absolute paths.
The two entry arguments are the literal relative `src/main.lkj`.

| Use | Actual source/library Directory root |
| --- | --- |
| Before and candidate source | `CASE/before` |
| Desired source | `.` at physical cwd `CASE/after` |
| Before and candidate library | `CASE/before/library` |
| Desired library | `CASE/after/library` |

The private key remains `FIXTURE/key`, outside all four roots. Before each
launch, the harness resolves the five paths physically, verifies that each grant
is the exact requested directory, and refuses any key path equal to or beneath
a granted root. The path-only table is retained; it contains no key bytes.
The archives still reject symlinks.

## Desired program and proof contract

The relative desired entry names the same archived
`CASE/after/src/main.lkj` bytes. The loader's diagnostic paths become relative
for desired source files; before/candidate paths retain their absolute explicit
source prefix, and both library prefixes remain explicit.

This form is supported by the existing replay pairing logic.
`file_pairs_allowed` pairs the catalog roots, stable item identities, and
loaded module identities; it infers directory correspondence for include-only
inputs. It does not require before and desired paths to share the same absolute
prefix. These six cases add no external source file or new root.

`meaning::projection` removes source names and diagnostic file/line/column
positions before `meaning::identity` hashes domain
`lkjscript-checked-meaning-1`. It retains operations, literals, type graphs,
authority, ownership contracts, and call edges. This static trace explains why
diagnostic-path spelling does not itself alter the comparison domain. The
actual candidate and desired programs must still independently check and compare
equal in the eventual command. This amendment claims no measured equivalence or
successful replay.

The requested cwd and the physical cwd read inside the measured subprocess are
retained separately. A failed change-directory or unexpected physical cwd exits
before starting the compiler, even though the caller captures nonzero status. The independent auditor now requires both to equal the
case's exact after directory, verifies the literal relative desired argument,
and recomputes the complete source/library/key grant table. Recovery uses that
same cwd and explicit before root.

## Unchanged scope and evidence

The named `6a187b94` 300-commit horizon, all 24 historical proof memberships,
the exact six-case order, every required changed-language path, producer
`adc0b26f...`, producer source `4f8c2135...`, source/library byte materialization,
single worker, 4 GiB / 900 s limits, 2 GiB store, no retries, complete file facts,
and selected-root canonical checked-meaning acceptance all remain fixed.

The original preparation manifest remains byte-identical historical evidence.
Its old file versions can be checked through Git commit `803c7367`; it is not
the launch manifest for the amended working tree. The new launch manifest is
`compiler-strict-recent.prepared-v2.sha256`. The v2 amendment and checks precede
every compiler/replay execution. All runtime work still awaits root's explicit
worker release after a successful full gate.
