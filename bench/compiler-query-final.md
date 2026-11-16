# Final compiler questions and bounded slices

**The 100 ms whole-command target is not met.** All forty recorded question and
slice commands exceed 100 ms with exact executing-producer authentication
included. The median bounded depth-one slice is **3,593 bytes**; two of twenty
subjects require a continuation. The recorded lookup outlier remains in the
results rather than being replaced by a faster repeat.

This is a follow-up measurement of the twenty real compiler subjects in
`bench/compiler-queries.sh`. It adds context-size evidence and measures the
retained native selectors on the larger integrated compiler. It does not
replace the earlier old-route byte comparison, and an item's own span is not
treated as the amount an agent would have read while searching files.

## Frozen identities and reproduction

The source and library were copied from revision
`0b30ca8987bd4ba343ab45ab01751b6721c7151a` before measurement.

| Identity | Value |
| --- | --- |
| Source tree | `b2496ac90633a09efac4bde2c35239f0a1d1eec7` |
| Library tree | `9962344d5071e103e937e892c662f3ff4609898d` |
| Producer SHA-256 | `54db01f51ca7b5802f932a8f4b76737ee2671cbe306f229c62e3c3ccae54c1e3` |
| Producer bytes | 19,584,049 |
| Source snapshot digest | `741ee185cdeb611171102b5d81750cc543926bfd918acdcc4b8213ea40bb6796` |
| Canonical checked IR | `489b0b4aeb49c5cd9f8f4ca5c6bb4e72665b5e1fe3871b96932a931452a244d6` |
| Authenticated index snapshot | `adb8bf5172d3b884938f3b9c687a83e7654bee526d16e033d1c963c0f9978513` |

The initial integrated producer had the new selectors in its source but was
itself emitted by the previous compiler. One extra self-build made its own
SHA implementation use the retained selectors. That build took
23.20 / 22.57 / 0.22 s wall/user/kernel, with 604,296 KiB peak RSS and
load 5.23/5.29/5.58 → 5.91/5.44/5.63.

Create an indexed build with a private key outside the source and library roots,
then run:

```sh
bash bench/compiler-slices.sh FROZEN_ROOT PRODUCER STORE PRIVATE_KEY SNAPSHOT RESULTS.tsv
```

The harness asks metadata through the authenticated index, then issues the
original question and a slice for each identity. It does not read source files
for discovery. Every command has depth 1, at most 96 facts, and at most 32,768
reply bytes. A continuation is reported, never silently followed without a
bound. Each measured command gets a 2 GiB external address-space limit and
30-second watchdog. The measurements include process startup and exact producer
hashing. No file-cache reset is claimed.

[The forty-row TSV](compiler-query-final.tsv) preserves external wall time,
compiler total, producer setup, index lookup, user/kernel CPU, RSS, both load
observations, facts, bytes and continuation status. The table below retains the
same twenty pairs; all three load averages happened to remain the same across
each pair. The TSV remains authoritative if displayed precision differs.

## Index construction and unchanged build

The index build used a 6 GiB external address-space ceiling, chosen from the
previous real compiler index's approximately 2.1 GiB resident peak and the
larger integrated source. This experimental ceiling limits the host run; it
does not change language allocation policy.

| Operation | Wall / user / kernel s | Peak RSS KiB | Load before → after, 1/5/15 min |
| --- | ---: | ---: | --- |
| Cold indexed build | 130.99 / 119.60 / 5.08 | 2,435,680 | 7.42/6.62/6.06 → 9.44/7.74/6.54 |
| Unchanged indexed build | 0.43 / 0.41 / 0.00 | 32,396 | 9.44/7.74/6.54 → 9.44/7.74/6.54 |

The cold build produced 12,117 IR functions containing 1,132,395 instructions
and retained 701,160,521 store bytes. The native image had the producer's same
19,584,049-byte size. The unchanged build returned the same snapshot with five
store hits, no writes and one reused build.

Within the cold row, source read was 12.633 ms, parse 1,022.712 ms, resolve
218.648 ms, type work 6,963.585 ms, IR construction 6,641.890 ms, body checking
7,788.638 ms, catalog construction 16,673.274 ms and index publication
88,293.382 ms. Producer authentication was 171.606 ms. These phases share the
cold row's paired load observation; they are not separate repeat measurements.
The `producer_hash` phase brackets all of `Memo::new`, including private-key and
store setup as well as executable hashing; its duration does not isolate hashing.
Index construction and publication therefore remain material cold-build costs.

## Twenty bounded pairs

| Identity / question | Question bytes | Slice bytes / facts | Slice continuation | Question wall ms | Slice wall ms | Load before → after, both queries |
| --- | ---: | ---: | --- | ---: | ---: | --- |
| `driver::Options` / outline | 4486 | 3526 / 9 | none | 187.384 | 190.624 | 9.34/7.97/6.86 → 9.34/7.97/6.86 |
| `driver::Memo` / outline | 3981 | 4705 / 11 | none | 236.570 | 194.485 | 9.34/7.97/6.86 → 9.34/7.97/6.86 |
| `driver::compile` / item | 1600 | 6645 / 21 | none | 270.072 | 231.801 | 9.34/7.97/6.86 → 9.34/7.97/6.86 |
| `driver::execute_source` / signature | 733 | 31082 / 96 | available | 201.085 | 272.920 | 9.34/7.97/6.86 → 9.34/7.97/6.86 |
| `driver::read_path` / signature | 674 | 3822 / 16 | none | 179.356 | 193.401 | 9.34/7.97/6.86 → 9.34/7.97/6.86 |
| `driver::diagnostic_source_path` / signature | 709 | 4455 / 14 | none | 187.153 | 235.775 | 9.34/7.97/6.86 → 9.34/7.97/6.86 |
| `ir::TypeKind` / outline | 4895 | 3764 / 6 | none | 190.809 | 283.449 | 9.34/7.97/6.86 → 9.34/7.97/6.86 |
| `ir::Op` / outline | 5582 | 6686 / 10 | none | 207.978 | 214.774 | 9.16/7.96/6.86 → 9.16/7.96/6.86 |
| `ir::Function` / outline | 1498 | 3028 / 8 | none | 182.630 | 185.040 | 9.16/7.96/6.86 → 9.16/7.96/6.86 |
| `ir::Program` / outline | 1023 | 2211 / 6 | none | 180.003 | 185.327 | 9.16/7.96/6.86 → 9.16/7.96/6.86 |
| `ir::Local` / outline | 834 | 1427 / 4 | none | 210.166 | 222.569 | 9.16/7.96/6.86 → 9.16/7.96/6.86 |
| `ir::ReturnSource` / outline | 963 | 2186 / 5 | none | 227.806 | 1404.952 | 9.16/7.96/6.86 → 9.16/7.96/6.86 |
| `semantic::Catalog` / outline | 2452 | 3660 / 8 | none | 189.533 | 194.701 | 9.16/7.96/6.86 → 9.16/7.96/6.86 |
| `semantic::Item` / outline | 2972 | 2195 / 5 | none | 320.643 | 209.028 | 9.79/8.11/6.91 → 9.79/8.11/6.91 |
| `semantic::Reference` / outline | 1267 | 1565 / 4 | none | 231.964 | 276.540 | 9.79/8.11/6.91 → 9.79/8.11/6.91 |
| `reuse::AuthenticatedRoot` / outline | 1317 | 1567 / 4 | none | 199.374 | 244.697 | 9.79/8.11/6.91 → 9.79/8.11/6.91 |
| `test_runner::closed_contracts` / signature | 674 | 2732 / 8 | none | 202.737 | 212.607 | 9.79/8.11/6.91 → 9.79/8.11/6.91 |
| `source::load_report` / signature | 794 | 31635 / 96 | available | 209.287 | 305.001 | 9.79/8.11/6.91 → 9.79/8.11/6.91 |
| `native::PreparedImage` / outline | 937 | 1800 / 4 | none | 184.663 | 262.105 | 9.79/8.11/6.91 → 9.79/8.11/6.91 |
| `authority::analyze` / authority | 611 | 14779 / 61 | none | 192.031 | 301.159 | 9.79/8.11/6.91 → 9.79/8.11/6.91 |

The slice median is the mean of the tenth and eleventh sorted replies,
3,526 and 3,660 bytes. The minimum is 1,427 bytes and the maximum 31,635 bytes.
The two 96-fact continuations are `driver::execute_source` and
`source::load_report`. Their first pages obey the byte bound but are incomplete
slices; an agent must use the stated cursor or narrow its question.

External wall time ranges from 179.356 to 1,404.952 ms. Producer setup, including
exact executable hashing, ranges from 173.953 to 308.307 ms across these forty commands. The
`ir::ReturnSource` slice has 1,206.076 ms in the recorded index phase, 196.606 ms
in producer setup and 0.18 s of user CPU. The timing mechanism records elapsed
time in that phase. The cause of this outlier was not established; neither an
I/O explanation nor a scheduler explanation is claimed. It is included in the
range and in the failure of the latency target.

## Remaining measured wall

A separate twenty-repeat fixed-byte workload hashes this same producer in
178.761 ms per repetition (outer 3.57 / 3.50 / 0.02 s; load
7.13/6.04/5.81 → 6.96/6.02/5.80). Its profile attributes 98.6% of samples to SHA
compression. The subsequent exact copied-operands fusion trial improves its
paired mean by only 2.86% and is discarded; see [the hash attribution](hash.md).

Thus bounded authenticated fact retrieval is present, but these measurements
do not satisfy the requested 100 ms command bound. Nothing in this experiment
replaces producer bytes with a timestamp, unsigned build identifier or another
identity shortcut. The whole-compiler -O milestones are separate and also
remain unmet in [the fixed-source attribution](compiler_attribution.md).
