# Current compiler query identity addendum: 62fa76f4

Status: **identities frozen; this cohort has not run**. This addendum records the
passed local combined gate. It does not release a compiler or query process.
Root's separate execution release after seed publication/push remains required.
The fixed 50 questions, order, bounds and single-launch policy are unchanged.

## Actual gated identities

| Record | Identity |
| --- | --- |
| Gated source commit | `62fa76f4f863a169b906094f0dd9998c8421e77a` |
| `src` tree | `00d1109fcdc49041d13d7406ca74c8f5b4394e8d` |
| `library` tree | `25bdb8ff8567b941790d3951c78e896b23e26e8e` |
| Runtime blob | `2fed59cd33d8a2ab80d562f160648b05f24b8c4c` |
| Canonical executable SHA-256 | `34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377` |
| Canonical executable bytes | 32,196,802 |
| Canonical executable retained by root | `/home/coder/workspace/lkjscript/target/compiler-origin-container-gate.uQZd6X/canonical` |
| Factory inline limit | 64 |
| Indexed compiler image inline limit | 64 |
| Fixed question-list SHA-256 | `417a4adfe357aab04915c5cdee3ebe2f691ee46ca2fb74663735e5d35a6e81e9` |

The exact JSON is `bench/compiler-query-current-identities-62fa76f4.json`.
Its evidence records are byte-for-byte copies of root's
`bench/compiler-origin-container-integration-data` records. The copied source
inventory has **1,860 records**: 184 under `src`, 32 under `library`,
118 under `tools`, and 1,526 under `tests`. Its SHA-256 is
`4b4b74b2dbfa418a2ecfaddd3ae7e2ecb2ea891f09880c4163e4f2fec9511b05`.
The run will archive precisely those four trees from the gated revision and
record its own complete inventory before executing commands.

The local gate passed in **606.794 s**, load **1.15 -> 3.70**, with 5,448 fresh
case requests and zero reused requests. Its mutation suite reported 128 mutants
and zero findings. Bootstrap passed in **76.437 s**, load **1.15 -> 1.63**.
The separate mutation detail records **2.410 s**; the enclosing gate suite was
**2.413 s**, load **3.76 -> 3.70**. These are prior gate observations, not
measurements from the prospective query cohort.

## Factory recipe and the missing temporary-path record

The factory is the exact copied `bootstrap.sh`, SHA-256
`1e8371471dd3b8cba3a4a5c2d6b0bef99d89efa811b4dea7bd46dd762fd5cdc3`, from the gated source.
Its actual cwd was `/home/coder/workspace/lkjscript`. The source selectors are
exact: relative `src/main.lkj`, `--library-dir library`,
`--runtime-source library/runtime.lkj`, `--inline-limit 64`, and no
`--source-dir`.

The bootstrap detail log contains only `ok bootstrap`. It does **not** retain
an expanded process argv or the random `target/bootstrap.XXXXXX` directory.
That temporary directory has been removed. Therefore the JSON's
`factory_argv` and `factory_reproduction_argv` are explicitly **the versioned
script recipe with symbolic substitutions**, not reconstructed literal process
commands:

```text
<EXTRACTED_SEED_AB2FA807> build src/main.lkj --library-dir library --runtime-source library/runtime.lkj --inline-limit 64 -o target/lkjscript <BOOTSTRAP_STORE_ARGUMENTS>
target/lkjscript build src/main.lkj --library-dir library --runtime-source library/runtime.lkj --inline-limit 64 -o <BOOTSTRAP_DIRECTORY>/reproduced <BOOTSTRAP_STORE_ARGUMENTS>
```

`<EXTRACTED_SEED_AB2FA807>` denotes the executable extracted from the gated
revision's seed blob `52405cfc76d93a702a97b3e04f5d5edb11da0679`.
The compressed bytes have SHA-256 `13c601190b8bb3926a03d2b1bd40f875c357b68d3376f073f3db74e3f569faca`;
the raw seed is bound by root's retained provenance to
`ab2fa80787261e2fec6d872d989099d3b1dd0984c26ac59bf89c7237d9e97504`.
`<BOOTSTRAP_STORE_ARGUMENTS>` denotes the script's conditional
`--store "$bootstrap_store" --store-key "$bootstrap_key"` expansion.
The gate metadata records the store and key paths; it is not a per-process argv
trace. No key contents are copied. No nonexistent literal temporary path is
claimed.

The script requires two consecutive current-emitter generations to match.
The retained log and canonical metadata report success with **no bridge**.
This qualification does not weaken the new cohort's acceptance: both indexed
images must still be byte-identical to the actual 32,196,802-byte canonical
executable, with matching snapshot/source/checked-IR identities and genuine
unchanged image reuse. A mismatch is a setup/proof failure; there is no repair,
different producer, alternate naming, or retry in this cohort.

## Prospective invocation and exact source grant

After root's separate release, the prepared harness invocation is:

```sh
bash bench/compiler-query-current.sh \
  --compiler /home/coder/workspace/lkjscript/target/compiler-origin-container-gate.uQZd6X/canonical \
  --identities bench/compiler-query-current-identities-62fa76f4.json \
  --results target/compiler-query-current-62fa76f4
```

No fixture name, store, output image or key directory has been allocated by this
addendum. The harness will retain the actual generated paths before commands
run. In the owned archive it uses cwd `SOURCE_ROOT`, relative entry
`src/main.lkj`, relative `library` and `library/runtime.lkj`, and omits
`--source-dir`. This grants only `.`; store/output/bin are sibling directories,
and the private key is outside all source/library/runtime grants. This preserves
the canonical diagnostic naming without granting `/`.

The unchanged prepared controls remain 6 GiB free-space preflight, a 2 GiB
writer store, 4 GiB/900 s per build and 2 GiB/30 s per query, each with a 10 s
termination grace. Run one cold indexed build, one unchanged indexed build,
then exactly the 50 captured questions without prequeries, map walks, oracle
warmups or retries. Keep latency and answer coverage separate.

## Frozen implementation

| File | SHA-256 |
| --- | --- |
| Plan | `a1c7949723c3402786379d4d5d71656f7387f2a36cbf1208e1caab04b324a4e8` |
| Harness | `e09435c6fd30b00ce33f5aa21af5e012f83c1705de1a9f6ac41259ed44bf934c` |
| Offline validator | `bc57855884da6aa7dc2aede7cb856e58032d9b488b1e3537bc3fc89669f0d1da` |
| Question list | `417a4adfe357aab04915c5cdee3ebe2f691ee46ca2fb74663735e5d35a6e81e9` |

The original preparation manifest and all historical v2/container observations
are unchanged. Different integrated source, producer and indexed-output inline
policy preclude a causal latency comparison with historical v2.
