# Supplemental query kinds at depth two

**All ten separately selected commands finish below 100 ms and return nonempty
facts with the expected trust labels.** Minimum / median / maximum
launch-inclusive wall time is **2.338 / 3.0245 / 21.521 ms**.
All ten subsequent old-reader validations produce **byte-identical stdout**,
including continuation tokens. No failed, empty or mislabelled result was
discarded or substituted.

This is a separate series following the [forty cached-reader requests and their
forty parity checks](compiler-query-cache.md). It covers uses, callers, callees,
implementations of a trait and for a type, generic instances, declared type,
verified layout, dependents and item text on the actual compiler source.
The identities were confirmed from frozen source/API definitions, without
discovery queries. The original twenty-subject latency table is not expanded
or rewritten with these observations.

## Pins, order and bounds

The source is frozen at `20d5f7e98fe4501d441b3ac357733a8a7ba942e7`.
The **snapshot producer** is SHA-256
`51708066a1c150ba51800697751b887ad279792f8b4e4d4a57d5df3e291d934c`; the **query reader**
is `fac8db084310bc8168493cb040944d8e463652a1f680534c91c56a6d0da3bc1a`.
Both source and checked-IR identities, the actual physical selector paths and
snapshot `cc171466d0b2b5ff3319737d33b7ff633bd572a749f0916fc7e4babfe3a85280`
are preserved in [metadata](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-query-kinds.meta). This reuses the same
single index as the forty-request campaign; no new index was built.

Each timed request ran **once** at depth **2**, with a **96-fact** limit and
**32,768-byte** reply limit, using a 2 GiB external address-space ceiling and
30-second watchdog. All ten timings completed before the first old-reader
validation. Those ten oracle requests are explicitly non-benchmark; their
latencies do not contribute to the timing table. There were no metadata
prequeries, retries, continuation traversals or file-cache resets.

All ten timing windows were **3.07/4.20/4.55 → 3.07/4.20/4.55**, the 1/5/15-minute
loads before and after each command. The wall envelope includes checkpoint
writing, GNU time/resource-wrapper launch, the compiler process and output.
The raw table separately retains coarse GNU time subprocess elapsed values,
user/kernel CPU, peak RSS and internal compiler phases.

## Measured coverage

| Kind | Identity | Returned fact level | Bytes / facts | Continuation | Wall ms | Query phase ms | Raw pages |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: |
| uses | `driver::Memo` | source-claim | 25471 / 96 | available | 21.521 | 19.593 | 104 |
| callers | `driver::compile` | checked-ir | 3403 / 16 | none | 7.838 | 5.938 | 39 |
| callees | `driver::execute_source` | checked-ir | 22703 / 96 | available | 9.845 | 7.87 | 31 |
| impls | `store::QueryAccess` | source-claim | 951 / 2 | none | 2.900 | 0.617 | 5 |
| impls | `store::QueryReader` | source-claim | 921 / 2 | none | 2.468 | 0.733 | 5 |
| instances | `semantic_store::open` | checked-ir | 1133 / 2 | none | 2.552 | 0.722 | 5 |
| type | `driver::Options::field:store_bytes` | source-claim | 779 / 1 | none | 3.149 | 0.663 | 5 |
| layout | `driver::Options` | checked-ir | 1035 / 1 | none | 2.338 | 0.62 | 5 |
| dependents | `semantic_store::Snapshot` | source-claim | 3120 / 23 | none | 7.886 | 5.562 | 35 |
| text | `driver::compile` | source-claim | 1119 / 1 | none | 2.583 | 0.746 | 5 |

The instance query returns two demanded `semantic_store::open` instances;
their native-size values retain the explicit `producer-measurement` label.
The field-type query reports declared `usize` as a source claim.
The Options layout and the emitted caller/callee relationships come from
checked IR. Uses, implementations and conservative signature dependents remain
source claims. These labels are validated as part of the captured-output check;
nonempty output is not treated as a proof that every source reference is covered.

The `uses` and `callees` requests each stop at 96 facts with an authenticated
continuation. The latter's caller/callee mapping retains its source-claim
qualification where present in the verified-instance fact.

Key/locator opening takes 0.068–0.082 ms. The query phase takes
0.617–19.593 ms and includes page authentication, decoding, traversal, formatting
and stdout; it is not an isolated I/O or CPU measurement. Peak RSS ranges from
4,188 to 4,956 KiB. Every command reports zero executing-producer bytes hashed
and the exact old producing compiler digest, with
`current-check=not-performed` and `freshness=not-asserted`.

## Evidence

[The ten-row TSV](compiler-query-kinds.tsv) preserves provenance, timing,
coverage, trust labels and both load observations.
[The ten-row parity table](compiler-query-kinds-parity.tsv) records the subsequent
old-reader comparisons, with identical answer digests for every row.
`bench/compiler-query-kinds.sh` uses the same pinned fixture arguments as
`bench/compiler-query-cache.sh`, but runs only this fixed depth-two list.

Together with the separate forty-request series, these are fifty selected
measurements and fifty subsequent exact-output validations. Every selected
timed command is below 100 ms. This is observed coverage at the stated bounds
and warm sequence, not a guarantee for arbitrary requests or host load.
