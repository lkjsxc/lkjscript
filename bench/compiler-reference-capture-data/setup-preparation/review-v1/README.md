# Reference-capture input setup: frozen proposal

**Preparation and syntax checks only. No setup has run.** The root's four-worker gate is active. This harness contains no A/B comparison and cannot authorize one; it does not use the forthcoming candidate.

## Scope and future invocation

The only run root is `/home/coder/workspace/lkjscript/target/compiler-reference-capture-measurement-1`. It must be absent. The script never deletes a path, retries a phase, changes tracked source, reads an operation key, creates a store, scans processes/FDs, or starts an observer. It uses only the requested UTC/time/load/free-space observations.

After the root accepts and pins this complete control manifest, a distinct release may invoke:

```text
bash target/compiler-reference-capture-setup-preparation-1/setup.sh
  --execute-after-root-release EXACT_CONTROLS_MANIFEST_SHA256
```

This placeholder is not a release. Root should retain the outer command/cwd/stdout/stderr/status even for a refusal before the run root can be created. Once records exist, the EXIT trap retains overall status/completion without cleanup.

`setup.sh` verifies the supplied manifest hash and every control file, rejects an existing/symlinked run root, copies and re-verifies its controls, then runs five sequential phases. Every phase has a 4 GiB address-space limit, 900-second timeout and 10-second grace, and requires 3 GiB free. Only phases 3 and 4 execute lkjscript/native workload commands:

| Phase | Work |
| --- | --- |
| recover-inputs | Recover A from the exact Git seed; check compressed/decompressed identities; copy the exact source.tar/generator/shape; verify every source archive member against c887 Git blobs before extracting a private compiler input. |
| prepare-factory | Independently recover that same verified archive to a fresh factory; replace only factory/src/main.lkj with the exact generator; preserve original/desired bytes, full before/after inventories and the single-file diff. |
| generator-build | Exact recovered A builds src/main.lkj from the factory with explicit factory library/runtime, inline0, no store, and stats. |
| generate-100k | The resulting generator runs once: `generate RUN/generated-100k 100000 20261116`, with the exact shape on stdin. |
| freeze-artifacts | Recheck unchanged inputs/factory/A; validate build stats and generated report/census; archive and verify every generated member; retain the actual generator and a verified gzip; produce artifact hashes and setup summary. |

The generator build includes `--stats` to retain the requested construction statistics. That is setup-only instrumentation, not a new A/B observation. Generation reports its fixed workload text; it is not a compiler statistics route.

## Byte and path contracts

A is recovered from `7d0731e99aeaf241c4334fc5ff065a341fb388c8:seed/lkjscript.gz`; the mutable main target is never used. The compressed SHA is 505be947525d53b4db1bff6c85cb77622cb2f5dc7e7d7fe92b772ee3b8ffc3e2, and decoded A is 6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc/32,584,542B.

Compiler source.tar is 79aa24a9eb6f8e7940b4424fc20839759a79dd8d480b1b746f7e29b17171d806/10,741,760B. The helper obtains the complete expected src/library/tests file set from exact Git c887b7c5aca7bb80d70ca26d7dfdf1a44213a163. Every archive file must match its Git blob identity; all 1,742 regular paths and all expected parent directories must occur exactly once. It rejects traversal, absolute/control-character/backslash paths, links, special files, duplicates, unknown entries and set-ID modes. Standard Git global PAX commit metadata, if returned by Archive::Tar, is accepted only with the exact commit. Content is fully verified before extraction. Archive modes are recorded; extracted files use private owner-only modes while retaining the executable/non-executable distinction.

Generator source and the old documented shape are read from their exact pinned Git blobs in controls.json. No fresh census runs. The only factory difference must be src/main.lkj, with every other path/content/mode unchanged.

The new generated tree is independently counted and completely archived in sorted ustar form with zero mtime/numeric owner 0/group 0, then gzip-compressed with fixed header metadata. Every regular archived member is compared with its actual file bytes; the directory set must also match. The historical generated source was cleaned, so the summary does not claim recovery or identity with that old specimen.

Metadata readers cap individual files at 128 MiB; this guard covers the fixed 10.7 MiB archive and 32.6 MiB A without claiming an unconstrained workload limit. Any unexpected excess refuses and preserves partial records.

## Records and retention

Each phase retains exact argv and resource-wrapper argv, declared/actual cwd, stdin selector, free space, UTC/nanosecond start/end, three load averages before/after, raw stdout/stderr, GNU wall/user/system/RSS/exit, original status, extracted main stats and metadata-validation output. The validator requires complete timing/load/cwd captures. A nonzero command keeps its original status even if its timing validation also fails. The first failure stops; later phases are not launched.

All output stays in the owned run root. `artifacts/` contains exact source archive, copied generator/shape, factory before/after/diff, generated source tar and gzip, member inventory, workload, generator gzip and build stats. The actual A and generator remain at the run root. Source/factory recovery is exact source.tar plus the single pinned main replacement; gzip recovery is checked byte-for-byte. `artifacts.sha256` and `setup-summary.json` bind the completed recoverable artifacts.

Root must review and durably retain/commit the complete generated-input archive and controls before any A/B release. This harness makes no performance, old-specimen identity, 1M, full-gate or candidate-CI claim. No cleanup mode exists.
