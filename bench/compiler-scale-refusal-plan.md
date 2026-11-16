# Prospective 1M ordinary-build refusal localization

Status: preparation only. No generator, compiler, profiler or benchmark has run
for this protocol. The preparation base is b628f5292779a62a0e73dd85eddf0ea56cf9b672.
The current gated producer, its source inventory, gate/fixed-point evidence,
current50 question result and isolated historical replay result must be bound by
a separate committed release before execution. Root controls that release and
the single compiler worker. This is a new cohort; it changes no prior result.

## Question and limits

Can one million lines of the accepted container specimen's frozen shape complete
an ordinary cold build under the justified experiment ceiling? If it refuses
with a located R0006/R0008 trap, does a separate cold check of the same source fit
under exactly the same bound?

This is not a full structural ladder. It does not run an index, queries, edits,
rename, a profile, a performance optimization or the 10M generator. The indexed
100k memory cost is already a separate reason to avoid adding that work here.
The result cannot establish size-independent edits or queries.

The original 1M observation used source population 1f48bd24 and producer
944d5e663bc6f7f703b8a399656f774a785966af4029ac56294e4b4c0fc2d2e7.
Its committed record is bench/scale.md, SHA256
962777da3bf071323cca0c1726eedacbd54aa28a14f0e787da737eb50fb3ecb6.
It reports library/runtime.lkj:13: R0006 heap allocation failed under an 8192 MiB
address-space limit. The native supervisor disappeared; completed wall/CPU/RSS,
exit and completion load are unavailable. The 3,999,252 KiB observation after
77 seconds is not a peak. No host OOM kill was established. Original metadata and
stderr were reported preserved, but the original raw 1M sidecar path was not found
in the present main bench/target inventory. Do not turn a new result into those
missing historical fields.

## Frozen specimen and factory

The generator source has remained byte-identical since f960f684:
bench/scale.lkj, SHA256
557d6b124fe739f17c678c77b92e5f6259d87ebca69c2a56aca411071c5ff1e9.

Use the accepted shape file from
bench/compiler-query-container-data/measurement-1/shape, SHA256
0457f8f9424882d2e3160a568f280193d49b9b046a5fae2d6e2d0b6da047072c.
It describes the parsed compiler/library population at
0b30ca8987bd4ba343ab45ab01751b6721c7151a: 83,992 source lines,
3,364,828 source bytes and 164 source files, followed by the retained individual
function/type/module observations. The complete distribution is the input;
it is not replaced by an average, padded lines or the evolving compiler source.

The generator factory is an archive of src/library at
a0da7d7fd98e0897856d7c94fe5622f949509c7f with src/main.lkj replaced by the exact
generator source. Build it using the decompressed 13c4e6fd:seed/lkjscript.gz,
whose raw SHA256 must be
adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7.
The factory command, executed from the factory directory, is:

    FACTORY_PRODUCER build src/main.lkj --source-dir FACTORY \
      --library-dir FACTORY/library --runtime-source FACTORY/library/runtime.lkj \
      --inline-limit 0 --no-store -o RUN/generator

Both rungs use seed 20261116 and exactly these generator arguments:

    RUN/generator generate RUN/source-100000 100000 20261116 < EVIDENCE/shape
    RUN/generator generate RUN/source-1000000 1000000 20261116 < EVIDENCE/shape

The specimen library/runtime are a separate archive of library at
0b30ca8987bd4ba343ab45ab01751b6721c7151a. The new generator image digest and both
factory/library manifests are captured before generation. Diagnostic path
embedding can change the new generator image bytes; it cannot change specimen
bytes without failing the control.

The generated 100k source manifest must equal the accepted
100000-S0-inputs.sha256 byte for byte. That reference manifest has SHA256
315f2c153190f8f1f0b12fa0bd8d4f0200f0b17c407f2e018617f278f820794e.
Its specimen has 3,923,070 source bytes, 169 files, 3,764 functions,
155 generic functions, 47,607 call sites, 196 nominal types, 637 fields,
204 variants, 375 nominal edges and six tail constants. Keep the new 1M
workload report and complete source manifest; its counts are not copied from
the old, differently shaped 1M specimen.

## Prospective order and stopping conditions

1. Validate the committed release and its producer/protocol/helper hashes.
   Record available disk, available host memory, load and experiment identity.
   Refuse before generation unless at least 16 GiB of disk and 16 GiB of
   MemAvailable remain. These are fixed experiment preflight conditions;
   compiler allocation policy does not inspect host RAM or these limits.
2. Materialize the fixed factory and specimen library; compile the generator
   once. Generate 100k once and require exact source-manifest parity.
   Setup commands each have 4 GiB RLIMIT_AS, 900 seconds and a 10-second
   termination grace. Setup times are separately labelled, not rung builds.
3. Run exactly one fresh-store ordinary 100k build using the released current
   producer, with 4 GiB RLIMIT_AS and 900 seconds. Require complete metrics,
   success, positive checked-instance/image counts, zero reuse and zero
   inlining, plus complete error-free Store I/O summary counters. If this
   control fails, stop: no 1M generation, build or check.
4. Derive the 1M bound from that new completed control, then repeat the fixed
   disk/memory preflight. Generate 1M once. Preserve its report and source
   manifest before the primary command.
5. Run exactly one fresh-store ordinary 1M build. Success must satisfy the same
   statistics controls as 100k. If it succeeds, the conditional check is
   explicitly not run. Stop after this primary result; 10M remains unrequested.
6. Only a nonzero primary exit with a located R0006/R0008 diagnostic and
   complete wall/CPU/RSS/exit/load capture releases one separate fresh-store
   check of the unchanged 1M source. Check uses the same producer, source,
   library, key, budget, memory bound and deadline. Verify the source manifest
   again first, and repeat the same fixed host-headroom preflight. If that
   preflight refuses, record the check as not run. Neither store is reused or
   copied into the other.
7. A watchdog, unlocated failure, missing metrics, statistics failure or failed
   generation is its recorded result and stops further compiler work. There
   is no retry, memory escalation, reduced source, alternative seed or silent
   repair. The conditional check itself runs once and stops on any outcome.

All compiler commands use a copied, read-only executable with the exact released
raw SHA256. The producer was built at inline limit 64; specimen output uses
inline limit 0. The harness refuses an unreleased template. It never uses the
mutable target/lkjscript implicitly.

From the corresponding source directory, each primary command is:

    env LKJSCRIPT_STORE_IO=1 PRODUCER build main.lkj --source-dir SOURCE \
      --library-dir RUN/frozen/library \
      --runtime-source RUN/frozen/library/runtime.lkj \
      --store FRESH_BUILD_STORE --store-key EXTERNAL_KEY \
      --store-bytes 2147483648 --inline-limit 0 --stats -o IMAGE

The conditional command uses check in place of build, a distinct initially
empty FRESH_CHECK_STORE, and no output option. The shell records the expanded
argv, cwd and stdin path for every command. Its launcher is:

    /usr/bin/time -q -f '%e\t%U\t%S\t%M\t%x' -o PHASE.time \
      prlimit --as=MEMORY_BYTES -- timeout --kill-after=10s DEADLINEs COMMAND...

No other compiler/profile/generator command belongs to this protocol. A new
question or repair requires a separately declared experiment.

## Resource calculation and historical planning evidence

The accepted container producer is
eed6ca32edcc182a92c4e697f4702ec685dff8e7778b0374a27552aa9c938947.
Its 100k observations below are historical planning inputs, not measurements
of the future gated producer. Raw timing table:
bench/compiler-query-container-data/measurement-1/timings.tsv, SHA256
80f1954ce0eb1a484e1651071225b5e84eb9685eba9281c7f0a7a267d89e39cd.

| Historical v3 phase | Wall s | CPU s | Peak RSS KiB | Load before -> after |
| --- | ---: | ---: | ---: | --- |
| Ordinary 100k cold build | 10.86 | 10.73 | 767,272 | 4.34/4.79/5.85 -> 3.89/4.67/5.79 |
| Indexed 100k S0 build | 105.67 | 103.90 | 2,330,656 | 3.20/4.04/5.43 -> 5.43/4.56/5.46 |

The new ordinary 100k control supplies R (peak KiB) and T (wall seconds).
Use a linear floor; a two-point exponent below one does not justify a smaller
address-space allowance:

    predicted_RSS_KiB = R * 10
    requested_MiB = ceil((2 * predicted_RSS_KiB + 262144) / 1024)
    actual_MiB = min(8192, requested_MiB)
    deadline_seconds = min(900, ceil(T * 10 * 3 + 120))

The memory ceiling is an external experimental limit, not a resource prediction.
RLIMIT_AS is address space, not resident memory; reservation and fragmentation
can refuse below the RSS prediction. No run raises the 8192 MiB ceiling.

For scale only, the historical ordinary control gives 7.317 GiB linearly
predicted RSS and about 14.885 GiB after headroom, so the 8 GiB ceiling binds;
its deadline formula gives 446 seconds. The indexed control would predict
22.227 GiB of RSS before headroom. This supports separating index allocation
from the ordinary-build refusal question. The actual bound is recomputed
only from the new current-producer 100k control and preserved in 1m-bound.json.

One worker runs sequentially after root release. A 16 GiB available-memory
preflight leaves at least one additional 8 GiB ceiling's worth of headroom at
launch. It neither reserves that memory nor guarantees other shared workloads
will remain unchanged. Load triples accompany every actual time. Disk
preflight covers three 2 GiB stores with publication headroom, the generated
sources, factory and evidence. No filesystem cache is flushed.

## What the conditional check can establish

The ordinary check route returns after the shared parse, resolution, type
elaboration, original-IR construction/checking and associated store work.
With inline limit 0, the build also retains original checked IR; it adds
build-specific source/cache handling, runtime preparation, native emission
and image/store publication.

If check succeeds after build refuses, report that the cold shared compile
route fits this envelope and a build-specific difference remains. Do not
attribute the refusal to a particular later pass: work and simultaneous object
lifetimes also differ. If check refuses, report that the shared compile route
also fails in its independent invocation. This does not prove the two commands
failed at the same allocation.

Stats::mark records phase values in memory; Stats::finish emits them after
normal completion. A runtime allocation trap can bypass finish. Preserve any
actual statistics and the full located diagnostic, but never describe
runtime.lkj:13 as an allocation call site in the frontend or as an identified
compiler phase. This protocol introduces no phase instrumentation, profiler,
syscall/race observer or process sampler.

## Durable records and recovery

The evidence directory is new and separate from the disposable run. Before
each child starts, retain its argv/cwd/stdin, start time, memory/deadline,
producer identity and load. Direct regular files receive stdout, full stderr
and GNU time output. The shell and time process remain outside the child's
address-space bound and timeout process group. After return, save raw wrapper
exit, GNU exit, wall/user/system/RSS and completion load even on a normal
resource refusal. Derived JSON/TSV and validation failures never overwrite
raw command exits.

The shell's EXIT trap writes completion metadata; it does not delete evidence,
source, images, stores or the external private key. A killed supervisor or
host loss can still remove the opportunity to obtain completion metrics.
Such fields remain unavailable. A recovered checkpoint is not a peak or
exit status. No independent observer is introduced to fabricate those fields.

Retain exact source/factory/library manifests, shape, workload reports, release,
producer/helper hashes, all raw failures, completed metrics, store apparent
bytes, compiler-reported retained bytes and any observed image size/digest.
An image present after a failed command is labelled observed output, not a
successful build. Store apparent bytes and authenticated retained-byte claims
are distinct columns; this harness adds no store-authentication claim.
Private key bytes are never copied into evidence.

After independent review, guarded cleanup verifies the repository/run prefix,
dead owner, absence of processes referring to the run/key, paired external-key
ownership and evidence location. It preserves ownership and cleanup extents
before deleting only that run and its key. Ambiguous ownership refuses cleanup.
Keep raw evidence in bench before landing results; generated trees stay temporary.
No compiler or generator is launched by cleanup.

## 10M cells and exponents

10M is not generated or executed by this protocol, even if 1M succeeds.
Its observed wall/CPU/RSS/store/image/instances/questions/rename cells are
not run, with no load sample. After a 1M refusal, use the reason
after_one_million_resource_refusal. After success, use not_requested_by_protocol.
Do not print zero, a guessed exit or the planned 8 GiB ceiling as measured RSS.

The accepted v3 10k/100k ordinary cold exponents are wall 0.853986 and RSS
0.788985. Extending only that historical two-point model to 10M gives about
554.37 seconds and 27.69 GiB; linear RSS planning gives 73.17 GiB.
For indexed S0 the analogous historical extrapolation is about 7,799 seconds
and 105.38 GiB; linear RSS planning gives 222.27 GiB. These are labelled model
illustrations from the older producer, not current runs or fit guarantees.
No actual 10M source/image/store/instance count follows from them.

Only two successful adjacent new-cohort builds permit a new 100k-to-1M
wall/CPU/RSS exponent. A refusal supplies no completed-build point; do not
combine its elapsed-to-failure with completed-build timings or mix the older
container producer into a current-producer exponent. Any later extrapolation
must name its two observations and remain separate from measured cells.

## Release and invocation

Copy compiler-scale-refusal-release.example.json to a new committed bench
release file only after root reviews the current gated producer, matching
source inventory/fixed point, current50 result and isolated historical case.
Fill all exact hashes and record paths; then set released to true.
The helper checks bytes and required fields. The root release is the review
that establishes the records actually describe the claimed producer and
completed prerequisites; a file digest alone is not that review.

    bash bench/compiler-scale-refusal.sh run \
      bench/compiler-scale-refusal-release-N.json /absolute/frozen/producer \
      target/compiler-scale-refusal-evidence.N

A finished or interrupted run is cleaned only after review:

    bash bench/compiler-scale-refusal.sh cleanup /absolute/owned/RUN_DIRECTORY

Before release, allowed validation is shell/Perl syntax and independent static
review. No actual result is claimed by preparing these files.
