# Prospective fixed-witness copy-site attribution and cost

Draft for review, 2026-11-16. No workload has run under this protocol. This is
one distinct attribution cohort after the copy-site implementation passes its
gate and CI. It authorizes no optimization pass, threshold trial, Rust build,
additional observer, or retry. The root must first commit the final plan,
harness and release pins and explicitly release the single launch.

The question is which fixed-width native copies are sampled in the compiler
built with producer inlining 64, where they occur in its exact selected IR,
and what this opt-in profile/companion costs beside a matched ordinary image.
The earlier 1,568/15,122 copy samples (10.369%) remain a historical marginal,
not a predicted result or a denominator for this new cohort.

## Retained basis and recoverable inputs

The following existing files were read without running their harnesses:

| File | SHA-256 |
| --- | --- |
| bench/compiler-profile-inline64.md | 27780568606a28d4f51ee1f373a829cfaa82f5aef22b19b79b4a34a49c4beaf5 |
| bench/compiler_profile_inline64.sh | f7483e908d8992df07c68130e953c3b26c794ffd57b009687d06c307f65904ab |
| bench/compiler_attribution.md | 313dc959873de77393a7534e4e098c6ccb1e5e5d2c2a2a53abf96124bd7f7f49 |
| bench/compiler_attribution.sh | 3720f8120c3dec6f3929f394e609bae2a4e49916bd82241ff495c366b671af4f |
| bench/compiler-profile-copy-sites-plan.md | b9de2760c582d33eedea3060a59f0502263335ca0667e597c34d4082bc8d9a52 |
| bench/compiler-profile-copy-sites-clarification-20261116.md | 14844c567c84b501ce01bacb2703a1cafcd00a4b38c42c15cfe3c4e9a5c1ff90 |

The frozen computation is unchanged:

| Input | Required identity |
| --- | --- |
| Git commit | 26bf2c3c021dd79810a81fa1d9054d59f3768b06 |
| src tree | 3431671024d0a550b3130f2401bde067026d9bd3 |
| library tree | e442fb3920db7eda2377f92fde0d92761f0562be |
| library/runtime.lkj SHA-256 | eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14 |
| src/optimizer.lkj SHA-256 | 961d59858c83afd8c5e8e2f9ec00d0f3a90cc91953ba8903528589f41a9da705 |
| seed/lkjscript.gz, 3,145,429 bytes | 85b4e70de34dfa759aaf3cfb7e1009ba6b49b57499e01e4a3d49ae9182a6193c |
| Decompressed expected image, 16,228,028 bytes | 8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132 |

All 172 paths selected by Git archive of src, library and seed/lkjscript.gz
exist in the repository's frozen tree. Preparation must archive exactly this
set, record and check its path/content inventory, and decompress the expected
seed only as data. Both physical source-library and runtime selectors point
inside that archive. The source argument stays literally src/main.lkj.

The old inline64 profile fixture, constructed images, measured outputs and
binary LKJPROF3 samples were explicitly removed after acceptance. Its sample
hashes and textual counters remain; they cannot recover a joint table. The old
emitter is recoverable as the seed archive at c4b62ce6, whose compressed digest
is 6c4b72837d4982c1c90c24ee8212c0934f6a5b836bb7d06194f5a894be3cf306;
that is not the producer for this protocol. The frozen Rust witness tree
bb2bbc6ef1932e22fe26a9e9d479ec4e7e14d28c remains in Git but is not archived
or executed here. There is no Rust optimization-level rerun.

The old shell harnesses remain unchanged. They are recipe evidence, not a
launcher for this new format: compiler_profile_inline64.sh pins the old emitter
and constructs both 0/64 profiles. Its Perl decoder
compiler_profile_tables.pl (04f8fb4ebc0404159f7ac93382c151d54560593317b98aa252b21a657fa655e3)
expects LKJPROF3 and must not be applied to LKJPROF4 or quietly modified.

## Required prelaunch release, presently unfilled

The release must bind the exact future gated current producer, not a moving
target/lkjscript, the old 34491766 producer, or an ungated candidate. Record:

- producer path, byte size and SHA-256; producer source commit and src/library/
  runtime identities; its seed and bootstrap recipe identity;
- passed cold gate/fixed-point and CI evidence paths, commits and SHA-256 values,
  including the focused profile acceptance and any acknowledged coverage limits;
- final plan, new minimal harness and any offline structural-accounting helper
  hashes, plus their committed revision;
- exact owned worktree, fresh run directory and separately owned result directory;
  resolved cwd, artifact paths, full argv, fixed bounds and once-only cohort ID.

A missing pin or failed gate is a prelaunch refusal, not permission to substitute.
The new launcher must be separately reviewed before release. This draft is not
an executable harness and does not claim those gates have already passed.

Copy the pinned producer once into the owned fixture, verify its bytes, and
use that immutable copy for both constructions and subsequent decoding. Call
it P. Let S be the frozen source archive; E its decompressed expected image;
O the ordinary producer64; Q the profiled producer64; I its selected-IR
companion; U and V the two measured compiler outputs. All are fresh distinct
regular paths in the owned fixture, with no symlinks/hardlinks or prior samples.
The resolved cwd is the measurement worktree root for every command.

## Exact order and policies

Run one worker, sequentially, with no other compiler workload released in
parallel. These are exactly the four timed compiler phases, in this order.
Each logical line below is one argv array; P/S/O/Q/I/U/V are resolved by the
committed release, never by shell globbing or an unpinned current path.

~~~text
1. P build src/main.lkj --source-dir S --library-dir S/library --runtime-source S/library/runtime.lkj --no-store --inline-limit 64 --stats -o O
2. P build src/main.lkj --source-dir S --library-dir S/library --runtime-source S/library/runtime.lkj --no-store --inline-limit 64 --profile --profile-ir I --stats -o Q
3. O build src/main.lkj --source-dir S --library-dir S/library --runtime-source S/library/runtime.lkj --no-store --inline-limit 0 -o U
4. Q build src/main.lkj --source-dir S --library-dir S/library --runtime-source S/library/runtime.lkj --no-store --inline-limit 0 -o V
~~~

Construction inlining is 64 in both images; execution output inlining is
explicitly 0. These are different policies at different stages. Both compiler
executions perform the same frozen computation. Neither a store nor a semantic
index participates. There is no preliminary compilation, warm-up, output-policy
sweep, inline0 profile construction, oracle rerun, or additional execution.

Both successful construction stats must show an accepted inline transformation,
zero rejected inline transformations, and matching original input and inline
selection counts. A fallback or refusal ends the cohort: retain it, do not
describe the resulting image as the intended inline64 observation. Stats
describe the counters actually emitted; original instruction counts must not
be relabelled as expanded selected-IR counts. Matching stats alone are not an
independent semantic-equivalence proof.

After each execution and outside its timed interval, require exit 0, empty
stdout/stderr, and direct byte comparison with E. Hash U/V and record their
sizes as independent identity checks. No source rewrite is permitted to make
the frozen witness admit. If current compilation or the exact-output oracle
fails, stop with a compatibility/failure result and leave later phases not run.

I must be the companion written by phase 2 from the same selected checked IR
that Q emits. Do not replace it with ordinary ir output, which is original IR,
or regenerate it after observing the sample. Hold onto both original files.
The profile's digest is correspondence, not freshness or admission authority.
Partial publication returns nonzero and publishes no new image memo; it may
leave a destination replaced. This protocol neither assumes two-file atomicity
nor requests a publication race/kill experiment.

File caches are not reset. Construction precedes both execution observations;
the fixed ordinary-then-profile order is not counterbalanced. Capture this
limitation rather than introducing warm-up or extra runs after seeing timings.

## Bounds and raw phase capture

Keep the previous inline64 attribution envelope: 4,096 MiB external address
space, 900 seconds per command, 10 seconds termination grace, one worker.
Use GNU time around prlimit/timeout, with the exact wrapper recorded:

~~~text
/usr/bin/time -f '%e\t%U\t%S\t%M\t%x' -o PHASE.time prlimit --as=4294967296 -- timeout --kill-after=10s 900s COMMAND...
~~~

The old profile64 construction used 1,010,736 KiB peak RSS, wall 28.65 s,
load 2.63/2.37/2.57 -> 2.93/2.48/2.60. Its execution used 662,352 KiB,
wall 15.39 s, load 3.39/2.60/2.64 -> 3.08/2.57/2.63. Those justify trying the
same bounded envelope; they do not predict the new metadata/IR-serialization
peak, virtual address requirement, or runtime. A resource refusal is the result;
neither memory nor deadline may increase in this cohort.

Require at least 3 GiB free on the owned filesystem before preparation and
recheck that floor before each construction. Before durable copying require
both that floor and room for the exact pending original byte count plus 1 GiB
of headroom; no optimistic compression ratio is assumed.
The historical profile64 image was 73,407,161 bytes and its sample file
50,400,056 bytes; the new C and selected-IR size E_IR are unknown. The space
floor is an operating precondition, not a prediction or an output-size promise.
A failed space check stops the next phase. Do not remove unrelated data.

Persist before each launch: phase label, cwd, exact argv as a structured array,
resolved inputs and output destinations, start UTC/nanoseconds, and all three
load averages. After it ends, persist raw stdout/stderr, GNU time output,
wrapper exit/signal, end UTC/nanoseconds and all three load averages before
parsing stats or evaluating the oracle. Include the external limits and tool
identities in the release; keep elapsed/user/system/RSS and exit separately.
GNU time wall is primary, including its wrappers. Launch-inclusive nanoseconds
remain raw evidence, not replacements chosen for selected observations.

Timeout/signal/launcher failure and incomplete or inconsistent captures take
priority over located trap text. Record any R0006/R0008 and location verbatim,
without inferring its cause or allowing it to resume execution. Missing metrics
stay unavailable; no zero substitutes or successful-looking partial rows.

## Post-execution validation and retention

Only after phases 1-4 and both output oracles succeed, enumerate Q.samples.*
under the owned directory. Require exactly one regular new file and a completed
execution, with no remaining writer. An absent, extra or malformed sample ends
analysis without re-executing Q. Its name/PID, byte size and digest are retained.

Run exactly one default summary and one bounded detail report with P, separately
capturing the same time/exit/load fields under the same envelope. These are
analysis costs, excluded from both construction and execution comparisons:

~~~text
5. P profile SAMPLE
6. P profile SAMPLE --copy-sites --profile-ir I --limit 200 --bytes 1048576
~~~

Retain each exact reply, stderr, digest and status. Phase 6 independently checks
the companion and validates every site's IR coordinates/layout before emitting
the bounded page. Record its continuation. No second page or larger bound is
automatically released; a bound refusal is retained. Two hundred displayed rows
are not a census, and omitted rows are not treated as zero. The complete raw
table remains available for a separately scoped eligibility review.

The durable evidence must retain the complete raw LKJPROF4 file, exact I bytes
and the embedded selected-function-to-symbol map, including zero-count sites.
Keep content-addressed gzip -n copies of SAMPLE and I with manifests giving
both compressed and original lengths/digests; verify that decompression yields
the original bytes before acceptance. Gzip is preservation, not a new sampling
or compiler workload. Hashes alone are insufficient retention for these two
artifacts. Preserve original raw files until root independently accepts their
durable copies.

The source archive, P/O/Q, E/U/V, metadata, raw phases, summary/detail and
ownership/completion records remain available for root review. Source/producer
inventories are checked again after execution. Only after independent acceptance
may guarded cleanup remove explicitly owned temporary artifacts. A later cleanup
must prove recorded ownership, dead owner/no live references, regular path
identities and retained counterparts; it cannot follow a broad target glob.
Do not auto-clean on failure. No key or authenticated store is needed here.

## What the single cohort may report

Report the four actual costs, their individual load windows, image bytes, raw
profile bytes, companion bytes and the two separately measured decode costs.
The matched ordinary/profile differences are one-pair observations, not an
optimization speedup, confidence interval, or isolated incremental-table cost.
They include all opt-in effects: sampling setup/handler, data mappings,
copy-site lookup/accounting, changed native layout and companion serialization.
This protocol cannot subtract the old profiler to isolate only the new rows.

From the retained profile and checked companion report S serial, T completed
kind samples, K copy-kind samples, mapped/unmapped counts, possible in-flight
detail count, C copy-site rows, F selected-function map rows, and E_IR exact
companion bytes. Preserve the full 16-kind and function marginals. The complete raw
joint table preserves these diagnostic observations; ordinary summaries may
truncate displayed percentages, so derive ratios from integer counts. Let D
be mapped plus unmapped. Require S-T in {0,1}, D-K in {0,1}, and D-K=1 only
when S-T=1; keep that possible interrupted-handler sample explicit. Reject
reversed counts/overflow and do not decrement a convenient row to force equality.

The prelaunch bundle must include a separately reviewed read-only binary
accounting helper for C/F, exact table offsets/bytes, full integer marginals
and width/role buckets from that retained file, without executing the measured
image. Freeze its code/hash before launch and run it once after the official
reports. It must check LKJPROF4 section bounds, checked-size arithmetic and
counter consistency, cross-check the official header/accounting, and keep
unknown/unmapped/in-flight counts explicit. Do not repurpose the old LKJPROF3
decoder or let this helper replace the official companion/IR check. Its own
failure stops derivation while preserving the accepted raw observations.
Any static SROA eligibility decision still requires complete selected bodies,
including freshness, escape, ownership/destructors and control-flow joins.

The expected encoded increment over an otherwise equal LKJPROF3 schema is
72*C + 16*F + 80 bytes plus stated alignment; calculate it from measured C/F.
It is a format calculation, not a measured same-producer old/new file delta.
Approximate construction vectors/serialization explain possible memory demand;
the RSS difference does not measure each allocation category separately.

Existing KIND_COPY excludes runtime/allocator overrides, variable-width p.copy,
Replace's byte swap, and scalar/frame/address work outside copy intervals.
Samples multiplied by width are neither dynamic byte traffic nor removable
time. A mapped aggregate may own resources or escape. Numeric function/type/
instruction coordinates identify this companion only, without an inline call
chain. An admitted diagnostic IR and a corresponding profile do not establish
execution authority or a trusted optimization proof.

The current emitter, its backend and profiling representation differ from the
historical 4c4537ea cohort even though frozen source/output remain identical.
Therefore comparing new counts/timings to 10.369% or 15.39 s does not isolate the
new instrumentation or any transformation. The old Rust-O and accepted inline64
campaigns stay separate evidence. The byte oracle checks this computation, not
a newly interpreted full compiler, current fixed point, or general semantic
equivalence. No >=10% opportunity, pass decision, Rust-O milestone or backend
floor is established by this attribution alone.

## Once-only completion rule

The launcher atomically claims one new result destination before any work,
records its owner PID/start identity and starts each declared phase at most
once. Existing results or partial fixture ownership cause refusal. Any nonzero
phase, trap, mismatch, missing/inconsistent capture or failed precondition ends
the sequence. Preserve the phase already attempted and label every later one
not run with its exact dependency. Offline capture preservation may continue;
compiler execution, repaired-input reruns and adaptive limit/bound changes may
not. An interruption is not an implicit continuation release.

The cohort ends with a complete success or a retained refusal/partial result.
A new experiment needs a new prospective record and separate root release.
This document is the proposed protocol, not a second benchmark history ledger.
