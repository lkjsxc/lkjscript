# Producer inline64: one fixed-source attribution pair

The retained producer-inline64 step reduces sampled copying and call-boundary work on the fixed compiler witness. This profile is consistent with the separately accepted **13.466% uninstrumented wall reduction** in [the six-run campaign](compiler-native-inline64.md). It does not divide that gain into causal milliseconds, supply another speedup observation, or establish a backend floor.

The remaining machine-copy category is **1,568 of 15,122 samples (10.369%)**. A pass that splits only local, nonescaping Copy aggregates addresses an unknown subset of it. The current profiler cannot show a credible ≥10% whole-witness opportunity for that pass: its function and machine-kind tables are independent marginals, without copy size, IR operation, or source/destination role. Aggregate scalar replacement after inlining remains a specific candidate for an eligibility census and then a bounded experiment, not an optimization implemented by this measurement commit.

## Fixed protocol and identities

The isolated worktree began at `c4b62ce6`. The common emitter was decompressed from that checkout's `seed/lkjscript.gz`, copied into the owned fixture and verified before either construction. The mutable main-worktree target was not used.

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| Common emitter, declared source `ec3e2ce8` | 20,627,356 | `4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09` |
| Instrumented producer0 | 51,581,850 | `772b026072769852b5afb45a0299cbe20246f4256c910c10268b4a3d2f18a798` |
| Instrumented producer64 | 73,407,161 | `f9312d3f51929406bcc20fabe85f501059bbf4d0e925f5404c003aba69452147` |
| Expected frozen seed and both measured outputs | 16,228,028 | `8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132` |

Both producers and the computation they execute use frozen revision `26bf2c3c021dd79810a81fa1d9054d59f3768b06`, source tree `3431671024d0a550b3130f2401bde067026d9bd3`, library tree `e442fb3920db7eda2377f92fde0d92761f0562be`. The source argument is literally `src/main.lkj`; both library and runtime selectors point to the same archived physical tree. Runtime SHA-256 is `eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14`.

The predeclared sequence was:

1. Construct producer0 with `--inline-limit 0 --profile --stats --no-store`.
2. Construct producer64 with `--inline-limit 64 --profile --stats --no-store`.
3. Execute producer0 once, compiling the frozen source with output `--inline-limit 0 --no-store`.
4. Execute producer64 once with the identical output policy.
5. After both observations, validate both sample files with the existing compiler `profile` command and extract their full observed counters.

Every timed phase had one worker, a 4 GiB external address-space limit, a 900 s deadline and a 10 s termination grace. Free space before generation was 115,888,884 KiB, above the 3 GiB preflight requirement. File caches were not reset. There were no retries, refusals or partial measurement rows. Both outputs were byte-compared with the original frozen seed, and both executions had empty stdout and stderr. The matching original-IR optimizer oracle at limit64 was already run in the accepted six-run campaign; this attribution pair does not claim another oracle run.

The profiler requested a 1,000 µs CPU sampling period. Profile decoding took place only after the two instrumented executions. No fresh executable-hash workload or Rust optimization-level comparison was run.

## Construction and instrumented execution costs

These four rows are costs of this attribution exercise. In particular, the execution rows are **not** new uninstrumented speedup observations. Wall is GNU time's elapsed duration around `prlimit/timeout`; the raw captures also retain launch-inclusive nanosecond endpoints. Load columns are the 1/5/15-minute averages immediately before and after each phase.

| Phase | Wall s | User s | Kernel s | Peak RSS KiB | Load before → after |
|---|---:|---:|---:|---:|---|
| Construct profile0 | 17.63 | 17.36 | 0.05 | 627,920 | 1.95/2.24/2.54 → 2.63/2.37/2.57 |
| Construct profile64 | 28.65 | 28.13 | 0.10 | 1,010,736 | 2.63/2.37/2.57 → 2.93/2.48/2.60 |
| Execute profile0 | 17.64 | 17.32 | 0.06 | 632,208 | 2.93/2.48/2.60 → 3.39/2.60/2.64 |
| Execute profile64 | 15.39 | 15.08 | 0.06 | 662,352 | 3.39/2.60/2.64 → 3.08/2.57/2.63 |

Construction stats record 30,619 inlined calls in 5,617 functions, one accepted transformation, zero rejected transformations and 4,455 reused local checks for policy64. The original input has 10,072 IR functions and 928,672 instructions; those stats describe the original input, not a count of expanded instructions. The inlining stage took 556.150 ms; the aggregated body-check phase was 14,751.834 ms for policy64 versus 5,026.438 ms for policy0. The instrumented image sizes above include profiler data and must not replace the uninstrumented sizes in the earlier campaign.

## All sample totals and exclusive machine kinds

| Counter | Policy0 | Policy64 |
|---|---:|---:|
| Completed samples | 17,354 | 15,122 |
| Header sample serial | 17,354 | 15,122 |
| Sum of function self counters | 17,354 | 15,122 |
| Unattributed function self samples | 0 | 0 |
| Unknown-kind samples | 0 | 0 |
| Symbols in profile | 11,401 | 5,890 |
| Symbols with a nonzero self or inclusive counter | 3,164 | 1,863 |
| Symbols with both counters zero | 8,237 | 4,027 |
| Encoded kind regions | 2,121,390 | 3,106,452 |

All sixteen exclusive kind counters are retained below and in the machine-readable table. Percentages use each policy's own completed-sample denominator. These are **emitted machine instruction categories**, not counts of similarly named IR operations: the IR expresses copies through `Read`, constructor and transfer operations.

| Machine category | Policy0 samples | Policy0 % | Policy64 samples | Policy64 % |
|---|---:|---:|---:|---:|
| Unknown | 0 | 0.000 | 0 | 0.000 |
| Inline copy | 2,810 | 16.192 | 1,568 | 10.369 |
| Checks | 239 | 1.377 | 254 | 1.680 |
| Call instruction | 138 | 0.795 | 106 | 0.701 |
| Frame load/store | 867 | 4.996 | 1,045 | 6.910 |
| Other memory | 1,918 | 11.052 | 2,114 | 13.980 |
| Arithmetic/comparison/conversion | 2,077 | 11.968 | 2,007 | 13.272 |
| Branch | 488 | 2.812 | 452 | 2.989 |
| Allocator | 1,607 | 9.260 | 1,757 | 11.619 |
| Runtime | 428 | 2.466 | 417 | 2.758 |
| Register-to-register move | 1,968 | 11.340 | 1,913 | 12.650 |
| Address formation | 1,256 | 7.238 | 1,310 | 8.663 |
| Immediate constant | 1,039 | 5.987 | 1,102 | 7.287 |
| Argument setup/result capture | 435 | 2.507 | 163 | 1.078 |
| Prologue/epilogue | 1,627 | 9.375 | 753 | 4.980 |
| Return | 457 | 2.633 | 161 | 1.065 |

Copy, prologue/epilogue, argument/result setup, return and call samples together fall from 5,467 to 2,751: 2,716 fewer samples. The remaining categories collectively increase by 484 samples, while the total falls by 2,232. These observations fit the existing inliner's removal of call boundaries and its proven argument/result-local forwarding. They do not isolate those mechanisms from changes in code shape, register allocation, sampling or host scheduling. It would be incorrect to turn each count difference into a corresponding fraction of the independently measured 13.466% gain.

## Function attribution

The companion function TSV preserves **every symbol with a nonzero self or inclusive counter**, including its address range and exact JSON-quoted symbol name. Zero-counter symbol totals are recorded separately. Its 5,027 function rows account for all exclusive samples. Inclusive counters overlap; they must not be summed. The existing profiler's complete reports, including its original truncated percentages, are retained in the raw log.

Selected functions show where the remaining computation occurs. Generic symbol suffixes are shortened only in this table; the TSV retains exact names. No meaning is inferred from numeric monomorphic type IDs.

| Function | Policy0 self | Policy64 self | Policy0 inclusive | Policy64 inclusive |
|---|---:|---:|---:|---:|
| `std::sequence::__lkjscript_slice_order<T=t5>` | 649 | 621 | 649 | 621 |
| `runtime::allocate` | 375 | 460 | 961 | 1,056 |
| `runtime::release` | 390 | 416 | 491 | 524 |
| `runtime::payload_class` | 375 | 399 | 375 | 399 |
| `runtime::valid_utf8_bytes` | 262 | 272 | 310 | 303 |
| `check::check_function_body` | 224 | 245 | 7,266 | 6,131 |
| `builtin:$order:t271` | 208 | 226 | 249 | 244 |
| `elaborate::borrow_cleanup::record_carriers` | 176 | 209 | 371 | 344 |
| `check::resolve_origin` | 128 | 188 | 1,330 | 1,189 |
| `check::access` | 106 | 137 | 2,275 | 1,848 |
| `check::place_type` | 78 | 111 | 154 | 111 |
| `elaborate::compile_function` | 2 | 1 | 6,110 | 5,581 |
| `elaborate::expr_inner` | 37 | 57 | 5,145 | 4,738 |
| `driver::compile` | 0 | 0 | 16,515 | 14,404 |

The checker body remains 40.544% inclusive and elaboration's `compile_function` 36.906% inclusive under policy64. These nested function totals are descriptive, not independent slices of a pie. Inlining also moves callee work into callers: for example, `std::bytes::append_bytes` has 480 self samples under policy0 and no observed symbol under policy64. Its absence does not prove that byte appending or vector growth disappeared.

Frozen-source inspection limits the aggregate hypothesis:

- `library/sequence.lkj::__lkjscript_slice_order`, the largest policy64 self symbol at 4.107%, is a scalar index/comparison loop. It does not construct a fresh aggregate in its main loop.
- `src/check.lkj::place_type` repeatedly matches type/projection descriptors and propagates `Option<usize>`. Constructor/tag/field forwarding is a plausible local opportunity, but its policy64 self share is only 0.734%.
- `check_function_body`, `resolve_origin` and `access` construct and transfer `Origin`, `SourceBound`, `Place`, `ResolvedOrigin`, loan records and `Result` values. Many contain owned vectors, sets or diagnostic data. They cannot be treated as destructor-free Copy records merely because their machine representation is copied.
- `elaborate::borrow_cleanup::record_carriers` visits `Read`, `Assign`, `Struct`, `Variant` and `Call` while maintaining reference dependencies. Its hotness concerns the compiler processing those IR operations; it is not evidence that those operation kinds account for a particular portion of this producer's native copies.

## Next rechecked IR candidate and missing evidence

The next specific candidate is **local aggregate scalar replacement after inline64**, beginning with straight-line construction/field use. Before predicting a ≥10% gain, an eligibility census must connect the post-inline checked IR to dynamically sampled native copy sites. The current two marginal tables cannot tell whether a sampled copy is a local record, a call argument, a returned diagnostic, a vector element or another materialization; they also do not report the copied byte count. A source count alone would not supply that dynamic evidence.

A bounded initial transformation could require a fresh `Struct` local in one straight-line block, recursively scalar Copy fields, no destructor, no reference or capability leaves, no address-taking, no aggregate escape and no branch/loop join in its lifetime. It could split that local into fresh field locals and replace exact `Field` reads. Inputs must still be consumed at the original constructor point; reusing a mutable input's storage would change subsequent field reads. Whole-value copies, projected mutation, cross-block liveness and escaping uses should initially refuse transformation rather than invent an alias or initialization proof.

A later `Variant`/`Discriminant`/`VariantField` extension would need a known tag and exact payload liveness. Ordinary `Result<T, Diagnostic>` handling frequently has owned error payloads and joins or early exits; simply dropping the enum shell can change conditional cleanup, moves or the checked tag proof. Such cases are outside the narrow scalar-Copy rule.

This must be an untrusted **IR-to-IR** proposal admitted again through the existing checker. It would retain function signatures, public layouts, authority and operation ordering, and preserve source locations for remaining traps. The original-IR interpreter must remain the independent oracle for values, trap code/location and observable effects. Adversarial cases should include input-local mutation after construction, reused destinations, duplicate/partial field uses, address-taking, live views, early exit, nested records, enum tag changes and destructor order; excluded cases must remain unchanged. A passed recheck establishes well-formedness and safety, not semantic equivalence by itself.

Only then would a single predeclared producer64-versus-producer64-plus-pass experiment be meaningful: same frozen source/library/runtime, output-inline0/no-store, every output byte-equal to the frozen seed, construction/recheck costs separated, and the ≥10% retention rule applied to uninstrumented whole-witness measurements. No such implementation or experiment occurred here.

The 10.369% surviving copy share makes a copy-only subset an insufficient basis for promising 10% overall. Scalar replacement may also remove address calculations, frame traffic and register moves, so that share is not a formal upper bound. Conversely, the existence of aggregate source code is not proof that those additional costs are removable. These profiles therefore establish neither a next ≥10% step nor an honest backend floor. The earlier inline16 miss, accepted inline64 result and fixed-source Rust optimization-level attribution remain separate evidence.

## Reproduction and retained evidence

Run `bash bench/compiler_profile_inline64.sh --seed PATH_TO_EXACT_4C_COMPRESSED_SEED --results NEW_PREFIX` from the repository. The harness refuses an emitter with any other digest and refuses existing result paths. Current or future seeds must not be silently substituted. It uses the unchanged compiler profiler first; the bench-only Perl decoder then reads LKJPROF3 counters and emits exact totals and full observed function rows. Harness SHA-256 is `f7483e908d8992df07c68130e953c3b26c794ffd57b009687d06c307f65904ab`; decoder SHA-256 is `04f8fb4ebc0404159f7ac93382c151d54560593317b98aa252b21a657fa655e3`.

Durable evidence is `compiler-profile-inline64.tsv`, `.meta`, `-parity.tsv`, `-counts.tsv`, `-kinds.tsv`, `-functions.tsv` and `-raw.log`. The raw log preserves arguments, launch captures, GNU time rows, construction stats, official profiler reports, final status and original-file hashes. The counts table retains sample-file hashes and byte sizes; the binary sample files are temporary artifacts rather than repository inputs.

Protocol, decoder and report received independent read-only review; no defect was found. All four planned phases finished successfully exactly once. Root independently verified the retained emitter and sample-category accounting and accepted the evidence. The owned fixture was then removed with the harness cleanup command, and its absence was verified. Its archived source, seed/emitter, constructed producers, measured outputs and binary profiles are gone; all textual evidence remains unchanged, and no worker remains active. This measurement commit changes no compiler, optimizer, bootstrap or seed policy.
