# Native compiler producer inlining at 64

The single preselected producer-inline64 campaign clears the ten-percent
continuation rule: mean fixed-source execution falls from **17.2533 to 14.9300
seconds, a 13.466% wall reduction**. Mean CPU falls from **16.8667 to 14.5833
seconds, a 13.538% reduction**. Each of the three preselected pairs improves
by 12.806–13.821% in wall time. All six outputs are byte-identical to the frozen
seed, and the matching original-IR optimizer oracle passes.

This is a separate threshold hypothesis using the existing rechecked IR pass.
The [inline16 trial](compiler-native-inline.md) remains a recorded 7.03% miss.
This measurement commit makes no compiler default, bootstrap or seed policy
changes. Any subsequent producer policy adoption is a separate change.
The [fixed-source Rust-O comparison](compiler_attribution.md) is not rerun here;
this campaign establishes no 4×, 3× or 2× milestone and no backend floor.
Exact-byte producer hashing is not measured here; the runtime result does not
establish faster unpinned queries or admission hashing for the larger executable.

## Fixed computation and identities

One immutable copied emitter constructs both producers from the exact source
behind the original compiler attribution. Only producer construction changes
from inline limit 0 to 64. Every timed producer execution builds the same frozen
source with **`--inline-limit 0 --no-store`**. Construction, transform/checking
cost and the independent oracle are separate rows, excluded from execution means.

| Input or artifact | Identity |
| --- | --- |
| Frozen revision | `26bf2c3c021dd79810a81fa1d9054d59f3768b06` |
| Frozen source tree | `3431671024d0a550b3130f2401bde067026d9bd3` |
| Frozen library tree | `e442fb3920db7eda2377f92fde0d92761f0562be` |
| Emitter source revision supplied by the completed gate | `ec3e2ce8` |
| Copied emitter, 20,627,356 bytes | SHA-256 `4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09` |
| Inline0 producer, 16,197,882 bytes | SHA-256 `48569219ef287dadb6d6721179166856d6a723a2bbbf2b7c8f862b94baa8f60b` |
| Inline64 producer, 22,939,898 bytes | SHA-256 `af076a155736a399baa6eda647faca30d8183d1f587719ec0466ec1cbdfce459` |
| Every measured output and frozen seed, 16,228,028 bytes | SHA-256 `8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132` |
| Explicit frozen runtime source | SHA-256 `eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14` |

The source argument is `src/main.lkj`; source directory, physical library and
physical runtime selectors point into the same owned archive. The producing
executable and its emitted compiler are different artifacts. The common output
digest verifies the frozen computation; it does not require the two producers
to have equal bytes or claim a new current-compiler fixed point.

## All six runtime observations

The order was fixed before launch as **0,64,64,0,0,64**, representing
0/64, 64/0, 0/64 pairs. No runtime observation was repeated or discarded.
Filesystem caches were not reset. The construction reads therefore precede
the six execution observations for both policies. No lkjscript store participates
in construction or these execution rows.

| Producer / sample | Wall / user / kernel s | Peak RSS KiB | Load before → after, 1/5/15 min |
| --- | ---: | ---: | --- |
| inline0 / 1 | 17.36 / 16.88 / 0.04 | 564,936 | 2.78/2.55/2.86 → 2.83/2.58/2.86 |
| inline64 / 1 | 14.97 / 14.59 / 0.04 | 571,804 | 2.83/2.58/2.86 → 2.91/2.61/2.87 |
| inline64 / 2 | 14.84 / 14.48 / 0.05 | 570,960 | 2.91/2.61/2.87 → 2.55/2.55/2.85 |
| inline0 / 2 | 17.22 / 16.78 / 0.05 | 566,156 | 2.55/2.55/2.85 → 2.84/2.61/2.86 |
| inline0 / 3 | 17.18 / 16.81 / 0.04 | 565,060 | 2.84/2.61/2.86 → 2.51/2.55/2.83 |
| inline64 / 3 | 14.98 / 14.54 / 0.05 | 571,680 | 2.51/2.55/2.83 → 2.39/2.52/2.82 |

The wall and CPU means use exactly these rows and their load windows. Primary
elapsed values are GNU time's reported seconds, matching the earlier producer
trial protocol. Nanosecond launch/exit captures are retained separately; they
are not substituted for selected rows. Each wall observation includes the
`prlimit`/`timeout` launch wrapper; image hashes and byte comparisons happen
after timing. The means are summaries, not additional timed observations.

Pair wall reductions, in their preselected order, are 13.767%, 13.821% and 12.806%.
With three samples per policy on a shared host, these are bounded observations,
not a general statistical guarantee. The prior inline16 campaign used a different
emitter and different load windows, so comparing its mean directly with this
inline64 mean would not isolate the threshold. This campaign's own 0/64 control
does isolate the chosen producer policy while holding source and output policy fixed.

## Construction and independent admission

| Producer constructed | Wall / user / kernel s | Peak RSS KiB | Image bytes | Load before → after, 1/5/15 min |
| --- | ---: | ---: | ---: | --- |
| inline0 | 17.60 / 17.10 / 0.05 | 572,488 | 16,197,882 | 2.62/2.51/2.87 → 2.70/2.54/2.87 |
| inline64 | 27.98 / 27.34 / 0.07 | 886,276 | 22,939,898 | 2.70/2.54/2.87 → 2.78/2.55/2.86 |

Producer image size grows **6,742,016 bytes (41.623%)**. Construction wall
cost grows **58.977%** in the recorded construction load windows. These costs
remain separate from the runtime improvement; the earlier cold-build default
decision is not silently replaced by this producer experiment.

Both construction reports describe 10,072 original instances and 928,672
original IR instructions. At 64 the inliner removes 30,619 calls in 5,617 bodies;
one candidate is accepted, none rejected, and 4,455 exact unchanged local body
proofs are reused. There is no stored proof reuse. These instruction counters
describe the original program, not an unreported optimized instruction count.

Within the inline64 construction load window above, the transform takes
558.499 ms. The body-check phase is 5,179.350 ms for inline0 and 14,675.897 ms
for inline64. Its 9,496.547 ms difference is an across-build observation:
the latter phase includes original and candidate admission, so the subtraction
is not an independently isolated candidate-check timer. The separate
`check_optimized_ms=0.006` is orchestration, not the rechecking cost.

## Original-IR oracle

The unchanged frozen `tools/optimizer-test.sh` passes with the inline64
producer in **5.36 s wall / 5.13 s user / 0.14 s kernel**, peak RSS **42,692 KiB**,
load **2.39/2.52/2.82 → 2.28/2.50/2.81**. This validation runs after all six
timed executions and is excluded from their means.

The script retains rejection before optimization, observations of original
IR and authority, original interpretation versus checked native execution,
exact custom and cross-file trap identities, proof-reuse/full-check equivalence,
native/profile image comparisons, self-image behavior and its 128-request
mutation slice with zero findings and nonzero accepted/executed rewrites.
The archive includes the script's actual generator and IR seed corpus; no
assertion or frozen compiler source was changed to fit current diagnostics.
This bounded oracle does not claim that the compiler self-build was interpreted.

| Oracle input | SHA-256 |
| --- | --- |
| Frozen optimizer script | `53b16740dfc28ba7a46bcf35d6d96f186d992b34ad15e4295d3610c575a6ed67` |
| Frozen and emitter optimizer implementation | `961d59858c83afd8c5e8e2f9ec00d0f3a90cc91953ba8903528589f41a9da705` |

## Resource and attempt accounting

One worker runs sequentially. Every measured command has an external 4 GiB
address-space ceiling, 900-second watchdog and 10-second termination grace.
These are experimental shared-host limits, not language resource policy.
The initial free-space check observed 115,877,176 KiB available and required
3 GiB. No resource refusal or watchdog termination occurred. Peak observed
construction RSS was 886,276 KiB; that is not a claim about unconstrained virtual
address-space requirements at arbitrary thresholds.

The first outer launcher failed before the harness started because the new
worktree lacked `target/` for its log redirection. It started no compiler and
produced zero timed observations. Creating that directory allowed the sole
campaign to start. [The exact setup failure](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-native-inline64-attempts.log)
is retained. No measured phase was retried, and no other threshold was selected
after seeing the result.

The harness retains its owned source archive, copied emitter, producers and
expected image for evidence review. Its explicit cleanup mode checks the
recorded owner and other processes before removing only that owned fixture.
The oracle removes its own temporary store, key, sources and test images.
After independent review, root recomputed every summary and rehashed the copied
emitter and both producers, then accepted the evidence. The owned fixture was
removed and its absence verified, including source archive, copied executables
and expected image. Only intentional textual results and captures remain; the
recorded source paths identify the completed experiment, not a retained live tree.
All raw timing, parity, metadata and capture bytes are unchanged.

## Reproduction and durable evidence

```sh
bash bench/compiler_native_inline64.sh \
  --compiler target/lkjscript \
  --expect-compiler-sha SHA256_OF_THAT_EXECUTABLE \
  --emitter-revision COMMIT_FOR_THAT_EXECUTABLE \
  --results target/compiler-native-inline64.tsv \
  --memory-mib 4096 --deadline 900

# After inspecting the preserved evidence and stopping every fixture user:
bash bench/compiler_native_inline64.sh --cleanup OWNED_DIRECTORY
```

The frozen revision, candidate 64, execution sequence and output 0 policy are
fixed in the harness. Existing result paths are refused. A new emitter creates
a new separately identified campaign; it does not overwrite this observation.

- [All nine timing rows](compiler-native-inline64.tsv).
- [Six exact-output parity rows](compiler-native-inline64-parity.tsv).
- [Protocol and input metadata](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-native-inline64.meta).
- [Complete phase captures, arguments, stdout and stderr](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-native-inline64-raw.log).
- [Setup-attempt and final outcome records](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-native-inline64-attempts.log).

This measurement commit adds only the harness, report and measurement evidence.
It makes no changes to the inliner, native emitter, source language, compiler
defaults, bootstrap or seed. Policy adoption and its cold gate, fixed point and
CI are separate changes.
