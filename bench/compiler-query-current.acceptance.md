# Independent acceptance of the current fifty-query cohort

The committed [current-query report](compiler-query-current.md) is accepted
with its original validator failure and separately released continuation intact.
The two native builds and all fifty question commands retain their original
captures; this audit ran no compiler or query and read no semantic-store node.

## Independent checks

Root's separate auditor, [compiler-query-current-root-audit.pl](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-query-current-root-audit.pl),
checks each raw argv/cwd, status, chronological clock interval, exact query
identity/kind/depth and producer/source/IR/snapshot header, fact framing/count,
checked-fact level, complete output byte bound, stdout digest and the mandatory
zero producer-hash counter. It independently recomputes the latency distribution,
continuations, reply totals and pause from raw captures, then compares those
facts with the derived row records.

The audit also hashes all 1,860 live source/library/tools/tests paths against
the gated inventory. The copied reader and both native indexed outputs each
have 32,196,802 bytes and canonical SHA-256
`34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377`.
The cohort itself performed direct image comparisons; root rehashed those
three live files independently.

| Raw observation | Independently reconstructed value |
| --- | --- |
| Query commands / zero statuses | 50 / 50 |
| Below 100,000,000 ns | 50 |
| Minimum / median / maximum wrapper ns | 2,729,156 / 4,461,788 / 60,812,180 |
| Query load before → after, 1/5/15m | 1.88/1.50/2.19 → 2.13/1.56/2.21 |
| Complete reply bytes / fact rows | 233,757 / 844 |
| Twenty owner reply bytes | 42,230 |
| Twenty slice median bytes | 3,738 |
| Unfollowed continuation ordinals | 8, 36, 41, 43 |
| Pause after unchanged build | 779.069631557 s; load 2.24/2.83/3.36 → 1.88/1.50/2.19 |
| Original-attempt query files | None |
| Live gated source-file digests | 1,860 matches |
| Reader/native-output identities | Three matches |
| Reader producer bytes hashed | Zero for every query |

This is bounded recorded coverage. Four replies remain prefixes; the dependent
query's 23 substantive source facts and two empty-index notices do not prove
exhaustive dependency resolution. Historical old-route source-span minima and
the current replies describe different source revisions, so their difference
does not establish a context-saving result.

## Audit execution and retained evidence

The independent audit was gateway request
`compiler-v1096-independent-current50-raw-audit`. It ran once under a 4 GiB
address-space limit and 900-second watchdog, with ten-second termination grace.
It passed in **0.26 s wall / 0.23 s user / 0.02 s kernel**, peak RSS
**10,656 KiB**, load **1.16/1.77/2.17 → 1.16/1.77/2.17**.
These are audit costs, not another latency observation.

Raw audit stdout/stderr, exit, GNU time, load and exact command are retained in
[the root audit directory](compiler-query-current-root-audit-data/).
Root also verified all 118 continuation-control records and all 403 final-bundle
records before landing. The original two offline invocation records preserve
their trailing whitespace as evidence bytes; the historical controls were not
rewritten to make a formatting check pass.

The source/query fixture and external key remained live through this audit.
Any later cleanup is separately recorded and cannot alter the retained
measurement captures or this acceptance.
