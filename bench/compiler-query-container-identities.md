# Semantic container v3: frozen identities and measurement addendum

This addendum fixes the producing executables and the implementation used by
the experiment in compiler-query-container-plan.md (protocol commit
8f28f9bf, SHA256
2a50ceedfb067d0b380a659cd5718cfa0744d94dd73f24b770fe640f76ef2c91).
The 10k/100k measurement has not run when this addendum is written.
The independent static review and the separate tiny helper validation precede
any measured rung. Historical benchmark observations are unchanged.

## Producing executables

| Role | Raw executable SHA256 | Bytes | Compiler build inline limit |
| --- | --- | ---: | ---: |
| v2 baseline | adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7 | 31,716,730 | 64 |
| v3 candidate, generation 1 | eed6ca32edcc182a92c4e697f4702ec685dff8e7778b0374a27552aa9c938947 | 31,922,848 | 64 |
| v3 candidate, generation 2 | eed6ca32edcc182a92c4e697f4702ec685dff8e7778b0374a27552aa9c938947 | 31,922,848 | 64 |

The baseline was materialized into the owned
target/compiler-container-baseline from the exact retained Git blob
13c4e6fd:seed/lkjscript.gz. This is the provenance correction for the earlier
plan to copy the live main executable: main had already refreshed that path,
so it was not used. The decompressed bytes match the previously pinned ADC
digest. No main seed or executable was changed.

The candidate factory used the ADC executable, the owned worktree's src/main.lkj,
explicit library and runtime source, no store and inline limit 64. Generation 1
then built generation 2 with the same arguments. Both commands ran from the
owned repository with relative source paths; private operation keys were outside
that grant. The commands were:

    target/compiler-container-baseline build src/main.lkj --library-dir library --runtime-source library/runtime.lkj --inline-limit 64 --no-store -o target/compiler-container-inline64-g1
    target/compiler-container-inline64-g1 build src/main.lkj --library-dir library --runtime-source library/runtime.lkj --inline-limit 64 --no-store -o target/compiler-container-inline64-g2

The two output files were compared byte for byte. Generation 2 is the fixed
candidate copied for the measurement. The factory manifest
target/compiler-container-evidence/inline64-source.sha256 has SHA256
69872250391b73a87bbd0e97869821bb2ee0a6e945abbefb383cb9c5e9cefb65.
The retained generation-hash file has SHA256
44c4072a73b66ee0cbb2040b4da31eab99c113b83d116241e2a29388defa866f.

| Factory command | Wall s | User s | System s | Peak KiB | Load before → after |
| --- | ---: | ---: | ---: | ---: | --- |
| ADC → generation 1 | 40.63 | 39.12 | 0.44 | 1,049,240 | 9.57 → 12.46 |
| generation 1 → generation 2 | 42.99 | 41.26 | 0.48 | 1,061,776 | 12.46 → 13.57 |

These factory observations have the same 4 GiB address-space and 900-second
per-command limits as the later experiment. They are not rung timings.

Two earlier correctness candidates were built at the default inline limit 0.
Their results remain recorded, but neither is a timing baseline or candidate:

- Initial candidate 73314aa80758acbd62f6aabbcce6c16dc8c3e6f48463c319763e014c3e9d1140:
  build 28.55 s, peak 679,008 KiB, load 12.75 → 15.92.
  Its first query test failed in 0.54 s, load 14.63 → 14.63, because the cursor
  encoder had version 3 while the decoder still required version 2.
- Corrected candidate a72d386bbb6af3bf0a1782503ed4ae7f1ea9bf7bb8c3078eb4682acfe2f9c5ad:
  21,145,666 bytes; build 21.84 s, peak 676,284 KiB, load 8.25 → 8.44.
  Its query suite passed in 5.28 s, load 8.44 → 9.28, peak 77,504 KiB.

The later standalone test-fixture correction does not belong to the compiler
main closure. Consequently the factory manifest contains the earlier case-file
hash da84cff7..., while the executed standalone case is now 53314714....
This is stated explicitly; the factory is not described as a build of every
file in the final checkout.

## Source and operation boundaries

The source baseline is a0da7d7fd98e0897856d7c94fe5622f949509c7f
(the source at protocol-only child 8f28f9bf is identical).
The four production files are frozen at:

| File | SHA256 |
| --- | --- |
| src/store.lkj | 40a9f63ee6c9f60563c3e2de25eab19a78dc7308c8104d59f68cce9319932bbe |
| src/semantic_store.lkj | b906e93b08a4375133ef56875d2782c5ddd1315c133bf4d0009977f1ca027bdf |
| src/semantic_query.lkj | 6db89d85392bed48974928dc2949a0a184005f3cfe599dd443889d1b0d948908 |
| src/driver.lkj | d2a33614d1949052de50cf94e0dea11ff9fed97ef9bf1dd83943b5e687014e96 |

The main transaction audit credits all 802 retained production .lkj lines:
671 Store, 117 semantic Store, 8 query and 6 driver lines. There were 62
successful requested operations. The separate standalone case migration adds
143 retained lines (109 added, 34 removed), independently reconstructed from
five case operations and one later case correction. Its repeated 47 Store
operations validate the standalone dependency closure and earn no duplicate
source credit. The combined retained src .lkj delta is 945/945 operation-backed
lines, subject to no subsequent source change.

The failed first main apply and first standalone apply are retained with
unchanged before/after inventories. No manual production-code repair is hidden
in these totals. Root's main audit is
compiler-container-operation-audit-2.txt, SHA256
0886552f3e5b5dee5eebbaf29f24cfc5eccdcf32079b642028ede42012b2f0c3.
The standalone review also checks the archived dependency closure, both failed
and successful requests, exact transfer bytes and the final correction.

## Historical cursor fixture correction

The initial migrated test built a v2-shaped cursor using a wrongly framed
semantic-cursor-question-2 key prefix. The actual historical derivation starts
with the raw bytes lkjscript-semantic-cursor-question-2, then length-frames the
raw producer digest and question identity. Request
93aae9a9624f01f554415dfb8866292de252f8fb1d03572b857add7fa0c2345e
corrected only this standalone fixture.

Earlier passing runs prove rejection of that wrongly keyed v2-shaped envelope;
they do not prove rejection of a valid historical v2 cursor. Their other
current-v3, v1, producer, pin, bound, capacity, migration and poisoned-writer
observations remain unchanged. The corrected case file has SHA256
53314714f4f2d2cc45b89bbee52185c9f6976332722c35181f4eab6268fa87fb.

The fixed inline64 measurement producer subsequently passed both required
focused suites, including the correctly derived historical cursor:

| Check | Wall s | Peak KiB | Load before → after |
| --- | ---: | ---: | --- |
| Corrected query-store fixture | 2.76 | 76,888 | 7.85 → 8.19 |
| Migrated semantic-query suite | 6.26 | 81,660 | 9.72 → 9.78 |

Complete commands, hashes, output and load triples are retained under
target/compiler-container-evidence/store-inline64* and query-inline64*.
The fixtures use ordinary existing compiler/parser/authentication/CLI facilities.
No stopped syscall/race observer was recreated.

## Separate helper preflight

Before rung execution, the shell helpers and both Perl helpers were syntax
checked. A separate two-function module fixture was indexed once by each fixed
producer with explicit narrow source scope, an external private key, 4 GiB /
900-second limits and a 64 MiB writer budget. This is a small helper validation,
not an extra rung and not a retry of measured input.

The independent comparator authenticated both roots, locators and all nodes:
17 physical nodes, 169 logical pages, 206 facts, 35,987 exact logical-value
bytes. All continuation chains were complete. The v3 container was 58,788 bytes;
the two native images were byte-identical. The logical-map fingerprint was
69a0cbafb5abbf0b4ead252a7c692c60651c7e2e675013676118ba1a310b5dfb.

One outline request used limit 1 and required a nonempty authenticated cursor;
another used limit 96. Both pairs had exact facts and exact decoded traversal
state after only verified format-version/provenance normalization.
The comparator normalizes only the verified final continuation record and
requires exactly one substitution. It never discards an unverified cursor.

All preflight stages had load 6.93 → 6.93:
baseline index 0.35 s, peak 21,752 KiB; candidate index 0.41 s, peak 27,656 KiB;
each map/query comparator was 0.01 s. These coarse GNU elapsed values are
helper-validation observations, not latency claims. Full evidence is
target/compiler-container-helper-preflight-1; fixtures and key are retained
until acceptance.

## Frozen rung execution choices

The original sequence and stopping rule remain in force. There are exactly two
rungs, 10,000 then 100,000 lines, and separate baseline/candidate stores.
Each producer executes ordinary S0 cold, indexed S0, unchanged indexed S0,
ordinary S1 body build and indexed S1 once, in the interleaved order specified
by the protocol. All four stores are retained. S0 and S1 native images and
snapshot IDs are retained separately.

The generator source is bench/scale.lkj at the declared base, SHA256
557d6b124fe739f17c678c77b92e5f6259d87ebca69c2a56aca411071c5ff1e9.
Its compilation closure is a Git archive of src/library at a0da7d7f with
src/main.lkj replaced by that exact generator source. It uses the archived
factory library/runtime and a relative src/main.lkj under an explicit factory
root, so legitimate sibling library includes remain inside the grant.
The source-shape archive and specimen library/runtime are the separately frozen
0b30ca8987bd4ba343ab45ab01751b6721c7151a tree.
All archive and generated-source manifests are captured before use.
The expected shape digest remains
0457f8f9424882d2e3160a568f280193d49b9b046a5fae2d6e2d0b6da047072c.

Both producing compilers use inline 64. Generated images use inline 0.
Every measured build explicitly enables LKJSCRIPT_STORE_IO=1 and must emit
the summary plus category records. A missing attribution record is a failed
control, not a zero. Explicit-pinned correctness queries may omit Store I/O
records, but must report producer_bytes_hashed=0.

The same 4 GiB / 900-second / 10-second-grace envelope also bounds the metadata
census and full logical-map/query controls. A census failure is captured
separately and cannot overwrite a compiler's recorded exit. A successful
build must have an authenticated Budget, full physical lengths matching its
entries, no missing or unindexed regular records, no nonregular leftovers,
and an accounted total within the writer capacity.

The untimed census is intentionally between commands. It reads file metadata,
fixed headers, snapshot fronts and the signed Budget, so subsequent commands
are not claims about a flushed filesystem cache. It authenticates no node
content. The full oracle later authenticates every rooted node and compares
every logical key/value and continuation relationship. This distinction is
retained in its machine-readable output.

GNU wall/user/system/RSS are recorded per command. The separate nanosecond
wrapper interval begins after command/load metadata and ends immediately after
the resource launcher returns; it includes time/prlimit/timeout launch and
stdout/stderr capture, but excludes later census, hashing and validation.
Each duration is accompanied by the full before/after load triples.

An unchanged indexed build must retain the exact S0 pin. A changed indexed
build must name a different S1 pin. After all timed builds, root/locator presence
for both states is recorded independently for both formats; an evicted pin is
reported, never rebuilt to improve coverage. Full paired logical-map comparison
requires both pins. Bounded questions execute once for each surviving pin;
unavailable comparisons remain explicit.

The 13 question kinds, identities, depth 1, limit 96 and 32,768-byte bounds
are unchanged. Query --store-bytes is deliberately absent: the default reader
allocation cap must suffice even when a v3 container exceeds 256 MiB.
No question is replaced, silently resumed or rerun to improve results.

Every harness/helper hash and the producer copies are captured before launch.
The final harness hash is recorded in the prelaunch review and launch metadata;
it is not embedded in the harness itself. The canonical protocol permits no
producer/threshold change, retry or rung expansion after the first run.

## Limits that remain

The container owns a complete copy for each changed indexed snapshot. The
experiment must report two-snapshot duplication and exact eviction, rather than
credit reduced entry count as fact compression. It cannot establish constant
time edits, eliminate global fact/IR allocations, diagnose the prior 1M refusal
or claim a speed milestone.

Parser/authentication, no-create, capacity, deterministic failed-eviction,
legacy retirement, eager-v3 parity and late-Memo retention facilities have been
exercised. New concurrent mutation and interruption boundaries that would have
required the automatically stopped observer remain explicit validation gaps.
The immutable publication policy is not a kernel seal and does not assert
working-tree freshness. Text remains the repository truth.
