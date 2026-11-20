# Independent acceptance of the current-producer scale refusal

The root agent accepts result commits `a1120d9a9c6fcca11f207df0f2dfd79dc027b954` and `379f456840d21f7a1441fb3fd0e8cf3a08f90acf` as the single prospectively released cohort. Its 106-record manifest is `24574f4eb2e85e21e5a8f3d190c7aa143c45012af140a5773e1230fa99bf3085`. No compiler or generator was repeated during acceptance.

## What the evidence establishes

| Command | Outcome | Wall / user / system s | Peak RSS KiB | Load before → after, 1/5/15 min |
| --- | --- | --- | ---: | --- |
| Fresh ordinary 100k build | Success, exit 0 | 11.20 / 10.74 / 0.18 | 713,512 | 0.85/1.35/1.63 → 0.95/1.35/1.63 |
| Primary ordinary 1M build | Located R0008, exit 1 | 77.04 / 74.07 / 1.43 | 5,224,960 | 1.17/1.39/1.63 → 1.94/1.56/1.67 |
| Separate cold 1M check | Located R0008, exit 1 | 76.24 / 73.49 / 1.42 | 5,220,184 | 1.94/1.56/1.67 → 2.34/1.77/1.74 |

Both refusals report exactly `src/elaborate.lkj:2618: R0008 stack exhausted`. The fresh 100k observation independently reconstructs the frozen **14,192 MiB requested / 8,192 MiB applied** address-space bound and **456 s** deadline. All six phases, including generator setup, have complete raw timing/load and mutually matching wrapper/GNU exit records.

The successful control has 4,209 checked functions, 896,361 instructions, a 4,472,259-byte image and 26,716,768 reported retained store bytes. The refused commands supply no compiler phase/instance/retained-byte statistics. Their elapsed times are time to refusal; there is no new completed-build exponent. The 10M rung was not run after the 1M resource refusal.

The two independent refusals name a shared compile/native-stack wall in this envelope. The producer already queues body instantiation, and the generator's shallow statement templates/32-way dispatch do not establish increasing callee elaboration depth. Failed native stack growth discards its final mapping reason, so actual stack/frame demand, address-space pressure and an occupied adjacent mapping remain distinct possible causes. The original incomplete R0006 observation remains unchanged.

## Root reconstruction

A separately written [auditor](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-scale-refusal-root-audit.pl), SHA-256 `15031ad6b550422372ee7747d61f056eea77ca776238128cf95f84b3a7dccdce`, verified all 106 manifest records, reconstructed all six exact argv/cwd pairs, checked raw time/status/load versus captured JSON, enforced the released limits and conditional refusal rule, rechecked successful-control statistics and recomputed the bound.

Its live input audit read **2,294 file instances across four exact path sets**: 215 factory inputs, 28 frozen library inputs, 170 100k inputs and 1,881 1M inputs. It checked every digest and independently counted precisely 100,000 / 1,000,000 source lines and 3,923,070 / 40,327,556 source bytes. Workload reports are included in file counts but excluded from source line/byte totals. It also verified the copied canonical producer `34491766…c0d377`, successful image `cf6568ce…70bea`, absent 1M image and completed refused supervisor.

Gateway request `compiler-v1148-independent-scale-root-audit` ran the auditor once under 4 GiB/900 s with ten-second termination grace. It passed in **0.21 s wall / 0.19 s user / 0.01 s system**, peak RSS **11,372 KiB**, load **1.99/1.76/1.66 → 1.99/1.76/1.66**. These are audit costs, not new compiler observations. Private key contents were not read.

The live fixture/key remained available until this acceptance; later cleanup is separate. Raw root output, stderr, status, time, UTC bounds and load readings are retained in [compiler-scale-refusal-root-audit-data](compiler-scale-refusal-root-audit-data/), whose manifest binds this report and the auditor.
