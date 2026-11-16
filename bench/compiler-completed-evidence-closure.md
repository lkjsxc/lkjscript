# Closure of completed compiler evidence

The released cleanup completed once at `646598856b9d13ea257940118eae397a398fc7ed`: **2,301 files, 218 directories and 484,039,701 logical bytes** were removed from 29 named compiler-owned roots. The exit status is zero and stderr is empty. This is completion of an evidence-retention task, not a compiler speed measurement.

| Scope | Files | Directories | Logical bytes |
| --- | ---: | ---: | ---: |
| Twenty-five completed evidence roots | 2,175 | 201 | 480,049,506 |
| Three original proposal mirrors | 85 | 13 | 1,527,984 |
| Completed preparation bundle mirror | 41 | 4 | 2,462,211 |
| **Total** | **2,301** | **218** | **484,039,701** |

The original scopes cover the fixed-witness copy-site cohort, complete twenty-body extraction, raw-row join and family census, their audits and earlier CI/cleanup preparation. Their exact literal paths are in the [committed manifest](compiler-completed-evidence-closure-data/manifest.json) and [completion result](compiler-completed-evidence-closure-result/result.json). The separate [seven-scope](compiler-legacy-root-cleanup.md) and [six-scope](compiler-copy-sites-root-cleanup.md) cleanups are not counted again.

## Recovery before removal

The [query](compiler-query-temp-closure-data/README.md), [native](compiler-native-temp-closure-data/README.md) and [combined](compiler-completed-evidence-closure-data/README.md) retention bundles were committed before execution. Every nondisposable file matched exact bytes recovered from a committed Git blob, archive member, gzip stream or specified byte range. The complete profile and selected-IR companion remain available in their committed compressed forms. This cleanup does not remove those retained artifacts or weaken the census's qualifications.

Only **34 files / 141,239,847 bytes** were explicitly disposable: 32 reviewed native preparation intermediates totaling 39,832,656 bytes, and the two held generated compiler images totaling 101,407,191 bytes. Their identities and producing evidence remain retained. Root independently matched this exact disposable set against the original disposition and two image pins.

The read-only preparation passed in **16.90 s**, with 16.49 s CPU and peak RSS 485,596 KiB; load was **0.52/0.93/1.40 → 0.79/0.96/1.40**. Root compared every byte of the final 41-file preparation bundle with its retained copy and verified the complete prepared inventory before committing it. That [copy audit](compiler-completed-evidence-closure-review/root-copy.json) is outside the exact bundle, preserving the self-mirror path-set rule.

The initial guard compilation failure remains in `guard.syntax-1.pl` and `syntax-1/`. The corrected guard passed its separate syntax check before the preparation and release; the failed source was never executed. No failed execution is overwritten or retried.

## Execution and result

| Observation | Result |
| --- | --- |
| Gateway execution / completion | `compiler-completed-closure-execute-v1367` / `compiler-completed-closure-poll-v1368` |
| Exact release commit | `646598856b9d13ea257940118eae397a398fc7ed` |
| UTC interval | 2026-11-16 16:03:42–16:04:01 |
| Wall / user / system | 18.34 / 16.22 / 1.72 s |
| Peak RSS | 532,260 KiB |
| Load before → after | 0.91/1.07/1.36 → 1.01/1.08/1.36 |
| External limits | 4 GiB address space; 300 s deadline; 10 s grace |
| Result | Exit 0; empty stderr; all 29 roots absent |

The guard authenticated its code and manifest against the exact release commit, compared all recovery bytes, and repeated the complete path-set check before the first unlink. It then rechecked each file's content, descriptor/path metadata and parent identity, removed only enumerated files, and removed only enumerated empty directories. The preparation mirror's metadata was frozen in this final read-only preflight; its file set came from the committed bundle. No self-referential digest or mutable recovery source was used.

The [raw result directory](compiler-completed-evidence-closure-result/) retains the full command, timing/load/status records, preflight, 5,040-event stream and result. [Root review records](compiler-completed-evidence-closure-review/) retain the outer invocation, exact-copy evidence and preservation checks. The [independent completion audit](compiler-completed-evidence-closure-review/independent-completion.json) verifies the full event sequence, all 29 current ENOENT results and the exact committed path sets and bytes of all three retention bundles. Its SHA-256 is `54e06c3e4099c63ffa78fc2f7328f761b4950a7660c0890e2083999c7c8567db`; its separately corrected audit-script syntax failure is disclosed in that record.

| Retained record | SHA-256 |
| --- | --- |
| Frozen manifest | `cbeab75fde4bd9b69da3b216e5db8a3310e6deb24f422995b306be17f579cdf9` |
| Preflight | `9a01c1617ae1626a2f5f6508c95562dee26f97fcf9fd8ebf00df114dd665b258` |
| Event stream | `71299ce20c02a6bd413d6984804ad04fe0ca828c31dcf0bb431cf7fd9081d5f9` |
| Final result | `005d0891230ea22cd209bdd401ee2b32d2d68f58a0acc81c265f9cbcf32cb04c` |

The canonical compiler remains `6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc`. Global store-key and unrelated operation-key metadata are unchanged; credential contents were not read. The current scalar experiment, other target siblings, services, data and other projects were outside every removal scope.

This is a cooperative cleanup, not an atomic filesystem transaction. A mid-run refusal would retain its partial result and require a distinct later release; no automatic recovery or retry is claimed. No process, descriptor, syscall or concurrent-mutation observer was added.

## Gate and CI attribution

Main's compiler, library, tools, tests, examples, witness and CI source remain identical to the gated `d461c2d9` source, and its seed remains `7d0731e9`. The already retained cold gate and fixed point therefore cover these documentation/evidence-only commits. [CI43894550526](https://github.com/lkjsxc/lkjscript/actions/runs/43894550526) at `e0b21e66` passed both gate and witness jobs; its exact [run metadata](compiler-copy-sites-census-acceptance/ci-43894550526.json) is retained. That baseline evidence does not validate the separate scalar candidate.
