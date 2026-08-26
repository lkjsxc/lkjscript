# Performance and economy evidence

Measurements are observations, not promises. The normalized-query section is current for query
contract 3 at its named commit. Unless another row explicitly says otherwise, the remaining
numbers predate meaning graph contracts 2, 3, and 4, persistent root pages, direct CLI v7, exact-ID
imports/targets, normalized query, or explicit generics. Those rows are historical baselines and
must not be presented as current-contract performance. Two graph-4 release workflows at 10,000
empty background modules are retained below on opposite sides of the semantic-fact cutover; no
current distribution, million-owner, or complete-service performance receipt exists yet.

The historical environment was Linux `7.0.0-29-generic` x86-64, `rustc 1.96.0`, Cargo 1.96.0.
CPU time, peak RSS, provider tokens/cache/requests/retries, and monetary telemetry were unavailable.
No token or monetary result is inferred from byte counts.

## Current normalized semantic query locality

At implementation commit `8ea897f7307d9726e57710c833a1596a9dd74127`, the release test
`ten_thousand_owner_and_high_fanout_relation_pages_remain_logically_local` constructed 10,000
live normalized owners and 9,999 canonical relations across persistent-map page boundaries. The
fixture uses the normalized first-party in-process builder so public request generation does not
dominate the locality measurement; an independent copied-release-binary test covers public
correctness and no-write behavior.

The command was `cargo test --release --locked --lib
ten_thousand_owner_and_high_fanout_relation_pages_remain_logically_local -- --nocapture`. It ran
on Linux `7.0.0-29-generic` x86-64 with Rust/Cargo 1.98.0, a warm release build, and a new
test process. The process completed in 0.754 seconds real, 0.461 seconds user, and 0.170 seconds
system time; Linux `VmHWM` reported 77,660 KiB peak RSS. `/usr/bin/time` was unavailable,
so Bash's process timer supplied CPU time. Filesystem cache state was uncontrolled.

| Scenario | Wall | Output bytes / records | Returned | Map pages / bytes / entries | Catalog / objects / store bytes | Canonical / witness | Continuation |
|---|---:|---:|---:|---:|---:|---:|---|
| first owner page | 707 us | 1,680 / 13 | 5 | 4 / 23,693 / 39 | 9 / 9 / 24,366 | 5 / 0 | yes |
| middle owner page | 595 us | 1,724 / 13 | 5 | 3 / 24,070 / 48 | 8 / 8 / 24,815 | 5 / 0 | yes |
| terminal owner page | 539 us | 1,472 / 12 | 5 | 3 / 23,422 / 36 | 8 / 8 / 24,167 | 5 / 0 | no |
| empty filtered owner page | 909 us | 969 / 8 | 0 | 10 / 38,171 / 295 | 10 / 10 / 38,171 | 0 / 0 | yes |
| resumed empty filtered page | 856 us | 966 / 8 | 0 | 9 / 36,982 / 275 | 9 / 9 / 36,982 | 0 / 0 | yes |
| exact namespace find | 731 us | 857 / 8 | 1 | 9 / 28,674 / 137 | 11 / 11 / 28,900 | 2 / 1 | no |
| first kind-prefixed relation page | 528 us | 2,229 / 13 | 5 | 4 / 22,543 / 39 | 5 / 5 / 22,620 | 1 / 5 | yes |
| middle kind-prefixed relation page | 527 us | 2,229 / 13 | 5 | 4 / 22,753 / 49 | 5 / 5 / 22,830 | 1 / 5 | yes |

Executable assertions bind exact find to bounded point reads, descend continuation pages from the
exclusive logical lower bound, cap entries visited by the selected scan quantum plus lookahead,
use a relation-kind map prefix, forbid the full reconstruction and query-index writer paths, and
compare full pagination with an independently sorted canonical oracle. Both the fixture counter
and repository inventory report zero repository bytes written. Raw structured evidence is
retained at `docs/evidence/20260826-normalized-query-scale-10000.json`; the full log is
`.artifacts/campaign-202608260032/query-scale-release.log`, SHA-256
`a0b809e5ee6d27485bffd3286924c7ab5c1c92f72cd1a3af86f75b59f10b4f2e`.

This is one topology and one warm-build/process-cold observation, not a latency or memory
distribution. It does not establish cold-cache, million-owner, filesystem-call, or full public
fixture-construction performance. Output bytes are not provider tokens, API requests, or monetary
cost.

## Historical source and predecessor baselines

At campaign start `6747754bdcad4dc9000c3d7891db7ef207c8ec2f`, the former source-authority
`lkjournal` workflow had retained warm p50 observations of 44.718854 ms and 1,356 output bytes
for orientation, 76.677093 ms and 311 bytes for package tests, and 84.051330 ms and 40,322 bytes for
a service-module body. Source authority has been deleted, so these figures are useful only as
historical comparison points.

An older graph-product build emitted 37,897 bytes for one exact-function query, took about 998.5 ms
for orientation, and took about 730.6 seconds for its complete verification profile. Those
observations motivated bounded exact query work; they do not describe the current product.

## Historical meaning-graph-1 command measurements

Seven warm release samples were taken per command on 2026-08-22 before the graph-2/3 and CLI-v3
cutovers. The release binary was 13,399,896 bytes with SHA-256
`f7d985142b4e019d22c5d3ab8a2bdfcdb99b7043a663edd5be2379aafefd13ab`. Fresh unique output files
were used for build, review, and backup.

| Historical command, now rejected or renamed | p50 | Output bytes | Derived file bytes |
|---|---:|---:|---:|
| `lkjournal semantic status` | 1.863 ms | 712 | — |
| `lkjournal semantic orient --limit 20` | 3.642 ms | 1,710 | — |
| `lkjournal semantic find handle --exact` | 1.684 ms | 645 | — |
| `lkjournal semantic show decl_ec993… --body` | 2.863 ms | 5,613 | — |
| `lkjournal semantic test` | 22.101 ms | 418 | — |
| `lkjournal semantic doctor --deep` | 5.217 ms | 299 | — |
| `lkjournal semantic build` | 17.505 ms | 607 | 160,195 |
| `lkjournal semantic text-project` | 14.639 ms | 630 | 1,328,003 |
| `lkjournal semantic backup` | 11.141 ms | 476 | 160,697 |

Those spellings remain here only to identify the historical evidence. Substituting current CLI-v7
names or normalized query semantics into old rows would fabricate a current measurement.

At that point canonical standard authority was 21,062 bytes and canonical `lkjournal` authority
was 160,419 bytes. Disposable query indexes added 50,858 and 790,720 bytes respectively. Those
sizes describe graph contract 1 and are not current graph-4 storage claims.

## Historical graph-1 scale topology

The retained `docs/evidence/20260822-semantic-scale.json` fixture constructed many empty modules
under the predecessor public ID-allocation, artifact-import, and transaction workflow. It did not
exercise persistent Merkle pages, explicit generics, dense declaration/call topologies, or CLI v4.

| Historical observation | 10,000 added modules | 90,000 added modules |
|---|---:|---:|
| first 10,000-module apply | 462 ms | 469 ms |
| final 10,000-module apply | 462 ms | 10.949 s |
| apply response | 5,378 bytes | 5,378 bytes each |
| cold exact local-index lookup | 73.5 ms | 630 ms |
| warm exact lookup, work = 1 | 15.8 ms | 138 ms |
| exact show | 13.7 ms | 134 ms |
| orientation | 129 ms | 1.283 s |
| deep doctor | 93.5 ms / 10,024 module versions | 3.661 s / 450,120 module versions |
| deterministic build | 132 ms / 1,330,505 bytes | 1.269 s / 10,370,515 bytes |
| backup | 98.4 ms / 2,183,176 bytes | 3.995 s / 44,315,558 bytes |
| canonical store | 1,832,489 bytes | 41,163,949 bytes |
| store with derived indexes | 6,631,953 bytes | 82,844,955 bytes |

The graph-1 final apply grew to 10.949 seconds because acceptance reconstructed, cloned, validated,
and reverified the complete candidate. Persistent-root contract 2 has replaced the flat physical
root with path-copied Merkle pages. Graph 3 additionally implements local preparation for
pure-body changes, independent module creation, module rename, and declaration rename, plus persisted authenticated
summary deltas. Other changes retain complete logical preparation, and a cold index rebuild can
still be broad. The one retained graph-4 10,000-module sample below is not a scale curve and does
not establish impact-proportional end-to-end mutation. The historical 90,000-module curve must not
be used to claim that it does.

Graph contract 1 also had a 100,000-module root ceiling, so its million-module case was not run.
Persistent-root contract 2 removes that flat-root semantic count from its root shape, but
one-million-owner creation, lookup, update, doctor, backup, restore, RSS, I/O, and interruption
behavior remain unmeasured. Persistent page unit/property tests are correctness evidence, not
complete-workflow scale evidence.

## Current persistent-root locality property

The graph-4/root-2 working tree now has an in-process differential test that constructs 10,000-
and 100,000-module persistent roots, changes one module-object binding, compares the delta root
with a complete rebuild, publishes the retained pages into the accepted base, and reconstructs the
exact changed logical root. A counting store observes physical base reads beneath overlay write
probes. At both sizes the test requires fewer than 64 physical page reads, less than one quarter of
base pages and bytes, and fewer than 32 retained pages. It also bounds 100,000-module reads to the
10,000-module observation plus eight pages, retained pages plus eight, and bytes to twice the
10,000-module observation. The focused debug test passed in 4.62 seconds on the shared warm
worktree. These are executable asymptotic bounds, not a release CLI latency or exact I/O sample.

The staged-page tests also retain an interrupted-publication counterexample in which a generated
ancestor already exists physically but one generated child does not. The overlay keeps the reused
ancestor, extraction retains its reachable child, and exhaustive reconstruction succeeds. A
separate corruption fixture proves staged and exhaustive traversal reject a parent edge that does
not match the child prefix. Million-owner complete-workflow, RSS, filesystem-call, and cold-cache
evidence remains unavailable.

One fresh-temporary-project release workflow then created 10,000 empty background modules through
one public change, created one local module, renamed it, queried it, ran deep doctor, built, and
backed up. Raw structured evidence is retained at
`docs/evidence/20260822-graph4-scale-10000.json`. The 10,000-module batch took 44.913 seconds and
returned 794,397 bytes. After that background, one-module creation took 22.781 ms and module rename
took 45.653 ms; each checked one module, but their store deltas wrote 842,666 and 843,519 derived
index bytes respectively. Exact lookup reported semantic work one in 2.397 ms for the first process
and 1.966 ms for the second. Orientation took 165.291 ms, deep doctor took 281.345 ms over four
revisions and 30,006 retained module versions, build took 199.282 ms, and segmented backup took
21.216 seconds for 3,053,899 payload bytes.

This is one warm-build, uncontrolled-filesystem-cache sample, not a distribution. It demonstrates
bounded local semantic validation and exact lookup, while exposing two prerequisites before larger
public runs: inefficient 10,000-operation bulk construction and a revision-bound semantic index
whose rewritten bytes grow with total modules.

After batched persistent-map mutation, single-use prepared root publication, and semantic-fact
contract 3 replaced those paths, the same public 10,000-module workflow was repeated from a fresh
temporary project at implementation commit `8ec09e24efc9968d900cfd3a4fa9ef63035a06d8`. Raw evidence
is retained at `docs/evidence/20260822-graph4-fact3-scale-10000.json`. The batch took 1.160 seconds
and returned the same 794,397-byte class of allocation receipt. Local module creation and rename
took 22.683 ms and 36.486 ms, checked one module each with full-oracle equality, and wrote 51,263
and 52,116 derived-index bytes. Exact lookup reported work one in 2.354 ms cold and 1.756 ms warm;
orientation took 155.361 ms, deep doctor took 316.332 ms across 30,006 retained module versions,
build took 203.360 ms, and segmented backup took 21.162 seconds for 3,052,823 payload bytes.

Compared only with the preceding single sample, the observed batch elapsed time fell by 38.7x and
local derived-index bytes fell by about 94%. These are equal-topology point observations, not a
latency distribution or a general performance guarantee. Backup remains the dominant measured
operation. A 100,000-module attempt reached backup, but its final JSON was lost when the completed
execution session exceeded the orchestration capture context and could not be reopened; it is
classified unavailable rather than pass. The one-million-owner public workflow was not run.

## Historical compiler, service, and verification receipts

Before the current cutover, graph packages passed 6 standard and 11 `lkjournal` tests with
bytecode/reference equality. The PostgreSQL service/worker acceptance completed in 4.431 seconds at
`.artifacts/service/20260822T061313.861237Z-420089/receipt.json`, covering 13 route/failure
observations, update/history, a 200,000-byte object, one worker iteration, shutdown, PostgreSQL
backup/restore, restart, and an equal restored read. It used plaintext HTTP and PostgreSQL
`NoTls`. It is one historical complete observation, not a current latency distribution.

The predecessor full profile passed 15/15 gates in 94.743 seconds at
`.artifacts/check/20260822T061143.565460Z-417922/receipt.json`, bound to worktree input SHA-256
`d822a652537d07e1859f0faea1e481b6d0e037f93f0eab3452416147074d0910`. It is not proof for the
current tree.

Current `lkjscript-dev check` models gates and retained exact inputs separately and labels fresh versus
reused evidence; the authoritative full policy requires fresh execution. A working-tree graph-4 /
CLI-v4 full profile passed 17/17 fresh gates in 6.078 seconds at
`.artifacts/check/20260822T131807.529974Z-726352/receipt.json` after a first run exposed and retained
an executable-writer DAG race. This is not yet final commit-bound or fresh-checkout evidence. No
current claim is made for provider cost, correction depth, startup, RSS, incremental compilation,
or million-owner complete-workflow performance. The one measured working-tree release binary was
15,031,768 bytes; it is an observation, not a size regression curve.
