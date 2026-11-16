# Independent acceptance of the semantic-container experiment

Root accepted the fixed result at `d06ff0553071c71cdcfb2630cd3fb66ec72c3233`
and integrated its result, current-query plan, and timing-boundary correction
through `70916ad21c5a8f9b103beb7548fa494ae686c975`.
The accepted report SHA-256 is
`e9ae888d32ce42ba9d6bbc4505d78c537197481fc983244fc09cbd2cc6ae90b3`;
its results manifest is
`31342eb6babad12d2e39ca1732c1766c2b4c32ff2422c7835412046f5cacdfd7`.
The separately frozen implementation manifest remains
`b8c0807b75121404e6bda0103d401a19df8ff543c31c824ef01e9922f2cba1dd`.

## Independent checks

Root inspected the frozen harness, full-map/query comparator and Store census,
including their authentication and allowed provenance-normalization boundaries.
Before cleanup, gateway operations `compiler-v1003-container-live-audit` and
its completion check verified the actual owned fixture and external-key paths
without disclosing the key, the frozen producing executable copies and source
inventories, and the final S1 source inventories. All four native S0/S1 pairs
compare byte-for-byte across producers; within each producer/rung the S0 and S1
image digests differ.

Root reran the committed bounded metadata census against each of the four live
stores and compared its JSON byte-for-byte with the retained final census.
All four match. This authenticates the signed Budget, manifest/physical lengths
and accounting; it does not by itself authenticate every stored node payload.
The separate experiment's four complete logical-map controls do authenticate
those rooted nodes and exact logical value payloads under each format's own pin.
Root did not rerun a build, timed question or full-map walk during acceptance.

| Live census | Wall / user / kernel s | Peak KiB | SHA-256 of matching JSON |
| --- | --- | ---: | --- |
| 10k v2 | 0.12 / 0.06 / 0.04 | 17,816 | 2b0c7ff2e115a2783ef1be8a24408854e3253e3c78eabf598b291d832fd4a79d |
| 10k v3 | 0.01 / 0.00 / 0.00 | 8,968 | 7ad8c45d2d0989938bf491e676e8fe8317c5d0b4560a81eccb7cbad9e9d3037a |
| 100k v2 | 1.07 / 0.65 / 0.37 | 96,736 | e689c5f684f8e53bf49364e9155d2091f94a0f3679b9ec0483b23d74e8a96195 |
| 100k v3 | 0.02 / 0.01 / 0.00 | 8,972 | 4e00a5249b6e16f3d79a16967e4a05066f5d3969c04469a5667d090691e1eba3 |

The load observations bracketing the audit were 3.29 / 3.58 / 4.55 before and
after (1/5/15 minutes). All censuses exited zero. These are audit costs, not
additional benchmark observations. The ten raw audit records are copied without
modification to [root-acceptance](compiler-query-container-data/root-acceptance/).
The original absolute paths in `census.sha256` identify the audited output
location; the new acceptance manifest hashes the retained copies.

The initial implementation-manifest check, gateway operation `compiler-v1006`,
used the repository directory even though that manifest's paths are relative to
`bench/compiler-query-container-data`. It failed on absent relative paths
without compiler execution or source mutation. Operation `compiler-v1007`
used the declared data directory and verified all 789 records. This was an
audit-command working-directory correction, not a repeated benchmark.

Operation `compiler-v1018` verified all 1,666 final result-manifest records
from their repository-relative directory and confirmed an exact recursive copy
of all 1,646 raw measurement files. It also checked the declared eight distinct
source dependents of `edit::target`; checked-instance call facts are not
double-counted as source dependents. The final wrapper-boundary clarification
changes only prose and its manifest entry, leaving every raw measurement intact.

## Accepted scope and remaining work

The fixed experiment has 20 once-only builds, four complete logical-map controls,
104 bounded queries and 52 exact normalized query-pair controls, with all eight
historical pins retained. It establishes the observed metadata reduction and
preserves the complete-snapshot storage cost. Its warm generated-rung queries
are not measurements of the integrated current compiler.

Root separately confirmed the source adoption boundary: 945 operation-backed
`src/` lines, plus 58 manually authored retained `.lkj` probe-specimen lines.
The latter remain uncredited, so this bundle contributes 945/1,003 to the
all-retained-language-lines metric, not 945/945.

Guarded cleanup of the experiment's owned fixtures, stores, private external key,
images and copied producers was released after acceptance. Durable edit metadata,
committed text evidence, the worktree and unrelated/global keys remain protected.
Cleanup completion belongs to its separate record. The combined source gate,
canonical seed refresh, CI and newly pinned current-compiler question campaign
remain pending at this acceptance point. The blocked new syscall/race observer
remains unexecuted; no new concurrent mutation or interruption-boundary result
is inferred from these checks.
