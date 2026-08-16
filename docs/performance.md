# Performance evidence

No performance leadership claim is made. These are bootstrap baselines whose purpose is to expose
costs before optimization.

## Protocol-v5 control-plane cutover

Source size was measured from the dirty working tree based on
`8d08f507d474b335512ea5afdba6be186e3b8517`. Current-file measurement used filesystem traversal,
not `git ls-files`, so it includes the untracked active sources `src/machine_contract.rs`,
`src/transport.rs`, and `tests/transport_json.rs`. The exact method was:

```sh
find src tests -type f -name '*.rs' -print0 | sort -z | xargs -0 wc -c
find src tests examples -type f \( -name '*.rs' -o -name '*.py' \) -print0 | sort -z | xargs -0 wc -c
git ls-tree -r --name-only HEAD -- src tests examples
# Each matching base path was measured with: git cat-file -s "HEAD:$path"
wc -c src/machine.rs src/machine_contract.rs src/protocol.rs src/transport.rs \
  src/codec.rs src/daemon.rs src/bin/lkjscript.rs
```

At the final documentation checkpoint, Rust source under `src/` and `tests/` had changed from
1,952,124 B at the base to 1,811,205 B including untracked active files, a reduction of 140,919 B
(7.2 percent). Rust and Python source under `src/`, `tests/`, and `examples/` changed from 2,013,995 B
to 1,904,442 B, a reduction of 109,553 B (5.4 percent) after adding the retained release-channel
replay. The named machine/control boundary changed from 602,276 B (`machine.rs`, `protocol.rs`,
`codec.rs`, `daemon.rs`, and the CLI at the base) to 434,493 B for those consumers plus the split
`machine_contract.rs` and new `transport.rs`, a reduction of 167,783 B (27.9 percent). These are
source-byte counts, not token, latency, memory, or runtime-performance claims.

Protocol/JSON and machine-schema identity are version 5. Durable artifacts remain format 3 under
`lkjscript-spg003`; the direct non-compatible publication-record cutover is `LKJHEAD4`. Focused tests
cover strict framed JSON, exact response correlation, dropped keyed-response replay, response
preflight before workspace or revision publication, restart/failure publication behavior, and
explicit `LKJHEAD3` rejection. Release-path interaction observations are recorded below.

## Semantic-proposal ergonomics milestone

The sealed black-box creation attempt that motivated this direct replacement was 19,567 compact
JSON bytes, declared 111 numeric handles, selected 38 bindings, and first failed on a string supplied
to the numeric local-handle field before a wrong-category retry. Of 60 expression results, 57 had one
use and 56 were both one-use and unselected. These are retained task observations, not model-quality
or token claims.

A reproduced debug-binary run of the migrated public job-policy driver emitted one 21,227-byte compact
creation envelope with 110 direct deterministic `DraftSymbol` labels and 32 selected bindings.
The driver deliberately keeps its broad flat expression listing. A focused inline-expression
prototype was not retained because validation and iterative flattening were not yet dependency-closed;
inline single-use values remain a measured gap. Symbol strings repeat at reference sites, so this
broad flat request is 1,660 bytes larger than the unequal sealed request and no byte-saving claim is
made. The verified benefit is removal of numeric-label/category authoring failures and support for
private omitted expression bindings. The run still created 188 nodes and produced byte-identical format-3 artifact
sizes (8,354 B, 8,373 B, and 8,379 B). After the endpoint-root correction, its final keyed symbolic
`LKJHEAD4` was 1,580 B, below the 16 KiB policy. Artifact format and semantic byte grammar were
unchanged.

A fresh debug-binary endpoint-root replay completed 44 agent RPCs plus two lifecycle shutdowns with
all job-policy oracles passing. The final shared-template replay observed 60,883 compact JSON request
bytes, 161,325 JSON stdout bytes, and 165,896,833 ns summed CLI/service wall time. The task discovery
response accounted for the intentional increase from complete endpoint wire definitions. This is one debug observation, not a
latency distribution or release-performance baseline.

## Controlled release-channel before and after evidence

Both isolated trials used the same release-channel task, the same machine, production release
binaries, fresh state, the same four allowed orientation files, and no private implementation or
existing example payload. The before transcript was sealed before implementation. The after trial
used a fresh model context and passed 43/43 machine assertions. Evidence is retained under
`/tmp/lkjscript-20260816-baseline/agent-trial/` and
`/tmp/lkjscript-20260816-after/agent-trial/`. These are single controlled observations, not a model
benchmark or performance distribution.

The root policy stayed byte-identical at 37,461 B. The four-file trial orientation set (policy,
README, status, and roadmap) grew from 62,664 B to 68,596 B. The broader policy, README, status,
roadmap, three specifications, architecture, and performance set grew from 159,442 B to 181,015 B.
This is documentation cost, not provider-token cost. Both trial agents opened the same four allowed
files and reported zero repository searches; neither opened specifications or implementation.

The direct compact contract comparison is:

| Projection | Protocol v4 before | Protocol v5 after | Change |
| --- | ---: | ---: | ---: |
| manifest result | 739 B | 1,241 B | +502 B (+67.9%) |
| task result | 86,009 B / six broad sections | 80,831 B / 12 agent-selected endpoint roots, 111 definitions | -5,178 B (-6.0%) |
| explicit full result | 126,888 B | 124,430 B | -2,458 B (-1.9%) |
| matching-digest unchanged result | 105 B | 105 B | no change |

The after agent did not need full or unchanged. Its actual compact downloads were the 1,241-byte
manifest and 80,831-byte task result. The before agent requested pretty output and downloaded 1,067
and 327,388 bytes, so the apparent 75.3-percent task-download reduction mixes formatting and is not
the equal compact comparison. The retained examples' fixed 12-root selection is 80,629 bytes. The
60-percent task-contract planning target was not met; exact envelopes, query batch/outcome layers,
typed and boundary errors, IDs, and limits were not omitted to manufacture a reduction.

Initial construction shows the remaining proposal cost:

| Accepted creation | Compact request | Explicit local labels | Selected bindings | Created nodes |
| --- | ---: | ---: | ---: | ---: |
| sealed numeric-handle before trial | 19,567 B | 111 | 38 | 216 |
| retained exact-graph symbolic replay | 22,247 B | 111 | 38 | 216 |
| isolated after agent | 23,582 B | 115 | 38 | 205 |

The retained replay is a direct symbolic transformation of the corrected accepted before proposal;
it preserves 17 operations, graph allocation count, revision artifact sizes of 9,387 B, 9,411 B, and
9,403 B, and all semantic oracles. It isolates a 2,680-byte (13.7-percent) JSON increase from repeated
symbol strings with no identifier-count reduction. The after agent selected the allowed additional
`score_client` helper, adding four expression drafts and making graph-size and creation-byte
comparison unequal. All 64 of its expression drafts needed explicit labels because inline
value-position expressions are not implemented. The 40-percent proposal planning target was not met.

The observed agent workflows were:

| Observation | Before | After | Change |
| --- | ---: | ---: | ---: |
| public commands | 42 | 36 | -6 |
| `lkjscript` launches | 38 | 33 | -5 |
| generic RPC attempts | 34 | 29 | -5 |
| compact RPC request bytes | 90,808 B | 47,501 B | -43,307 B (-47.7%) |
| compact RPC response bytes | 38,232 B | 30,299 B | -7,933 B (-20.7%) |
| unintended initial-creation rejections | 2 | 0 | -2 |
| required invalid-repair rejections | 1 | 1 | no change |
| malformed RPC JSON shapes | 1 | 1 | no change |
| elapsed task boundary | 567.696 s | 715.678 s | +147.982 s (+26.1%) |

The before harness labelled all 34 RPC attempts as connections/round trips even though its local
`invalid_json` attempt could not connect. The after evidence distinguishes 29 attempts, 28 real
connections, and one local rejection; a precise real-connection delta is therefore unavailable.
The request-byte reduction is largely the absence of two repeated broad creation attempts. The after
creation was accepted on its first attempt and had no unintended semantic rejection. A later
`incoming_uses` query guessed `{node,output}` instead of the described tagged `value_ref`, received a
precise local `invalid_json`, and succeeded after one correction. The after task took longer despite
fewer public interactions; no speed improvement is claimed. Neither isolated task exposed provider
token, retry-cost, or price telemetry, so no token or cost comparison is made.

A separate deterministic production release replay of the exact retained graph used 29 authoring
requests with 45,824 request bytes and 33,299 response bytes, plus four separately reported discovery
requests and two typed lifecycle requests. It passed creation, allocation rollback, repair identity,
all seven decisions, low-fuel laziness, rename, history, restart, and execution oracles. This replay
is the retained regression oracle; because it invokes no model, it is not substituted for the after
agent observation.

The fresh production build used by the after trial took 79.191 s in a new target directory. Its
client was 4,973,416 B and daemon 3,736,872 B, compared with the before fresh build's 56.818 s,
3,779,472 B client, and 2,245,680 B daemon. These one-build increases are retained control-plane cost,
not equal-work runtime regressions; maximum RSS is unavailable because `/usr/bin/time` is absent.

The first final-boundary attempt stopped at Clippy because the workspace-response-preflight review fix
left two production methods used only by unit tests. Applying `cfg(test)` to those helpers removed the
dead production surface. The complete boundary was then rerun from its start and passed:

- formatting check: 0.294 s;
- all-target/all-feature Clippy with warnings denied: 3.000 s;
- fresh all-target/all-feature test: 18.999 s, 165 passed and nine ignored in a 630,453 KiB target;
- fresh optimized release build: 81.095 s in a 57,272 KiB target;
- final release binaries: 4,973,416 B client and 3,737,640 B daemon;
- diff check: passed.

The final production job-policy, named-data, and release-channel drivers all passed through the real
CLI/service path in 0.134 s, 0.119 s, and 0.151 s respectively. The deterministic seed-1 malformed
artifact/framed-JSON/JSON boundary smoke passed all 10,000 cases; test execution was 0.03 s, while its
fresh release-test compilation made the command wall observation 137.161 s. `cargo miri --version`
failed because Miri is unavailable for the installed stable toolchain; no nightly or unsafe exception
was added. These are single build and workflow observations, not latency distributions.

## Plain-language and job-policy campaign

### Environment and method

Measurements were retained on 2026-08-16 from the dirty campaign worktree based on
`456ef91b692336ce0e8eaafc49bdf61d84a2db44` on `main`: `devbox`, Linux
7.0.0-29-generic x86-64, AMD Ryzen 9 9955HX, 20 logical CPUs visible, 32 GiB memory,
`rustc 1.96.0 (ac68faa20 2026-05-25)`, and Cargo 1.96.0. The cgroup memory ceiling was
34,359,738,368 B, CPU quota was unlimited, shell stack limit was 8 MiB, and virtual memory was
unlimited. Product-path measurements used optimized release binaries. `/usr/bin/time` remains
unavailable, so maximum RSS is unmeasured. The deterministic job-policy driver invokes no model, so
its request bytes are not tokens or API cost. Supplemental telemetry for the parent coding-agent
campaign session is reported separately below.

The retained public-path measurement is:

```sh
cargo test --release --test job_policy_json \
  job_policy_agent_interaction_cost_measurement --locked -- \
  --ignored --nocapture --test-threads=1
```

This is retained historical protocol-v4 evidence. It invokes `examples/job-policy/driver.py`, which
starts the real service and launches the strict generic CLI once per request. Compact JSON request
bytes exclude a newline; JSON stdout bytes include the production newline. The binary columns used
the then-production framed request/response codecs and do not describe the active protocol-v5 path. Wall time
uses `time.monotonic_ns` around each CLI process and includes local IPC and service work. Typed
shutdowns are lifecycle operations and excluded from the 44 agent-workflow totals. The retained test
prints every row as one machine-readable `JOB_POLICY_AGENT_COST` record.

### Complete job-policy interaction observation

| Purpose | Outcome / returned items | JSON request | JSON stdout | Binary request | Binary response | CLI + service wall |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| schema manifest | success / 7 | 107 B | 814 B | 17 B | 736 B | 2,327,648 ns |
| task contract sections | success / 6 | 269 B | 86,084 B | 31 B | 86,006 B | 2,504,260 ns |
| known fingerprint unchanged | success / 1 | 185 B | 180 B | 49 B | 48 B | 1,840,441 ns |
| workspace creation | success / 1 | 66 B | 322 B | 15 B | 120 B | 10,873,952 ns |
| job policy incomplete creation | success / 1; 32 bindings | 19,274 B | 1,926 B | 3,131 B | 1,050 B | 9,825,019 ns |
| score repair context | success / 1 | 373 B | 12,054 B | 98 B | 3,993 B | 714,694 ns |
| allocator probe before invalid repair | success / 1; 1 binding | 305 B | 531 B | 90 B | 182 B | 773,034 ns |
| invalid score repair | expected `type_mismatch` / 1 | 588 B | 352 B | 137 B | 161 B | 438,946 ns |
| allocator probe after invalid repair | success / 1; 1 binding | 305 B | 531 B | 90 B | 182 B | 655,082 ns |
| workspace after invalid repair | success / 1 | 189 B | 481 B | 56 B | 162 B | 333,617 ns |
| incomplete main run | expected `compile_incomplete` / 1 | 233 B | 340 B | 83 B | 170 B | 350,910 ns |
| valid identity preserving score repair | success / 1 | 630 B | 486 B | 138 B | 154 B | 8,279,082 ns |
| post repair identity context | success / 4 | 575 B | 5,030 B | 168 B | 1,643 B | 910,502 ns |
| refinement semantic diff | success / 1 | 223 B | 1,322 B | 69 B | 360 B | 541,659 ns |
| revision two main case h | success / 1 | 233 B | 282 B | 83 B | 90 B | 699,726 ns |
| revision two case a linux check | success / 1 | 1,296 B | 282 B | 512 B | 90 B | 654,782 ns |
| revision two case b wasm build | success / 1 | 1,296 B | 281 B | 512 B | 90 B | 570,613 ns |
| revision two case c cpu rejection | success / 1 | 1,296 B | 372 B | 512 B | 131 B | 551,878 ns |
| revision two case d memory rejection | success / 1 | 1,297 B | 372 B | 512 B | 131 B | 518,996 ns |
| revision two case e target rejection | success / 1 | 1,296 B | 372 B | 512 B | 131 B | 518,385 ns |
| revision two case f untrusted release | success / 1 | 1,297 B | 372 B | 512 B | 131 B | 504,589 ns |
| revision two case g trusted release | success / 1 | 1,296 B | 281 B | 512 B | 90 B | 508,527 ns |
| revision two lazy unsupported target | success / 1 | 1,303 B | 372 B | 512 B | 131 B | 496,304 ns |
| revision two lazy untrusted release | success / 1 | 1,304 B | 372 B | 512 B | 131 B | 506,734 ns |
| runtime named type context | success / 7 | 1,009 B | 6,143 B | 313 B | 2,530 B | 500,662 ns |
| resources memory display rename | success / 1 | 349 B | 485 B | 103 B | 154 B | 8,509,084 ns |
| rename diff and named type | success / 2 | 347 B | 1,242 B | 107 B | 505 B | 627,530 ns |
| renamed revision main | success / 1 | 233 B | 282 B | 83 B | 90 B | 877,190 ns |
| restart revision one identities | success / 32 | 3,468 B | 12,666 B | 1,135 B | 4,204 B | 516,832 ns |
| restart revision two identities | success / 32 | 3,468 B | 12,665 B | 1,135 B | 4,204 B | 441,270 ns |
| restart revision three identities | success / 32 | 3,481 B | 12,684 B | 1,135 B | 4,210 B | 457,220 ns |
| restart incomplete revision name | success / 1 | 268 B | 888 B | 85 B | 356 B | 380,986 ns |
| restart repaired revision name | success / 1 | 268 B | 888 B | 85 B | 356 B | 397,778 ns |
| restart renamed revision name | success / 1 | 268 B | 894 B | 85 B | 362 B | 362,272 ns |
| restart incomplete main | expected `compile_incomplete` / 1 | 233 B | 340 B | 83 B | 170 B | 576,524 ns |
| restart repaired main | success / 1 | 233 B | 282 B | 83 B | 90 B | 905,453 ns |
| restart current case a linux check | success / 1 | 1,296 B | 282 B | 512 B | 90 B | 675,600 ns |
| restart current case b wasm build | success / 1 | 1,296 B | 282 B | 512 B | 90 B | 841,803 ns |
| restart current case c cpu rejection | success / 1 | 1,296 B | 372 B | 512 B | 131 B | 592,064 ns |
| restart current case d memory rejection | success / 1 | 1,297 B | 374 B | 512 B | 131 B | 884,564 ns |
| restart current case e target rejection | success / 1 | 1,296 B | 373 B | 512 B | 131 B | 859,677 ns |
| restart current case f untrusted release | success / 1 | 1,297 B | 374 B | 512 B | 131 B | 757,675 ns |
| restart current case g trusted release | success / 1 | 1,296 B | 282 B | 512 B | 90 B | 648,540 ns |
| restart current main case h | success / 1 | 233 B | 281 B | 83 B | 90 B | 529,656 ns |
| **agent-workflow total** | **44 launches/round trips; 1 rejected proposal** | **58,168 B** | **165,890 B** | **16,962 B** | **114,228 B** | **66,241,760 ns** |

The two allocation probes are successful validate-only requests around the one rejected semantic
repair. Their predicted revision, hash, created count, and selected Node ID are identical. The two
`compile_incomplete` Runs are expected execution attempts, not rejected mutation proposals. Two typed
shutdowns add two lifecycle CLI launches but are excluded from the totals above. Two additional
1,000-fuel Runs use `cpu=100000` on unsupported-target and untrusted-release inputs. Both return the
expected rejection; accidentally entering `triangular(100000)` would exhaust fuel, so these are
public-path evidence that unselected match and condition work remains lazy.

The creation request has 17 public transaction operations, 110 explicit handles, and 32 selected
returned bindings. It creates 188 nodes, of which 78 are implied by structured expansion; the saved
model has 189 nodes including the initial workspace root, seven named declarations, seven functions,
and 87 canonical operation/terminator nodes. Revision artifacts are 8,354 B incomplete, 8,373 B
repaired, and 8,379 B renamed; `LKJHEAD3` is 1,164 B. One cold readiness observation was 16,058,182 ns
and one restart readiness observation was 6,437,077 ns.

The revision-2 main Run returned exact `Decision.accept(25)` and observed 127,830 ns for
validation/lowering/Core IR verification plus 24,075 ns for interpretation. The seven direct Cases
A-G together observed 561,977 ns compile and 77,989 ns interpretation. Eight current-revision Runs
after restart together observed 1,201,749 ns compile and 182,454 ns interpretation. These are one
workflow's unequal-work sums, not distributions or runtime speed comparisons.

### Machine-contract investigation

At that retained protocol-v4 measurement point, program meaning, protocol, and executable
descriptors had not changed, so the digest was
`983614734f16b5d2095279fb5e958814e839caaa7aa25a5a6963cfca44795e2d`, protocol/JSON were version 4,
and artifact format 3 / `lkjscript-spg003` / `LKJHEAD3` were unchanged. The direct compact-result
projection measurements remain 739 B manifest, 86,009 B for the six selected task sections, 126,888
B full, and 105 B unchanged (all without the local CLI newline).

The six-section response was material and the controlled baseline justified direct replacement.
The active v5 contract now uses exact roots plus iterative transitive definition closure. The first
root cutover observation was 1,029 B manifest, 66,957 B for 13 leaf/task roots and 96 definitions,
121,868 B full, and 105 B unchanged, but independent review proved that the leaf/query roots omitted
operational envelope, batch, and error layers. Those numbers are retained only as the rejected
under-specified observation.

The first operational correction cloned every wrapper per endpoint. It was exact but architecturally
rejected: 12 roots returned 254 definitions and 149,720 compact bytes, larger than its 123,145-byte
full description. Those figures are retained only as the rejected duplicated-wrapper observation.

The direct compact replacement projects one shared control template and one shared query template
from the executable broad descriptors. Endpoint bindings carry only exact selected leaf variants and
shared error/ID/limit references. The focused debug serialization test recorded 1,241 B manifest,
80,629 B for the retained examples' 12 endpoint roots and 111 unique closed definitions, 124,430 B
full, and 105 B unchanged, all excluding the CLI newline. Encoded response sizes were 1,319 B,
80,707 B, 124,508 B, and 183 B respectively. The active task result is 43,801 B smaller than full and
5,380 B (6.3 percent) smaller than the historical 86,009-byte six-section result. The 60-percent
reduction remains an unmet planning target, not a reason to omit wire facts. The observed active
digest was `dcd4e6473f0b746b0ab7b27b50a1408226eb50cb79105f13d6062239371fa13f`.
Bytes are not provider tokens.

### Repository instruction and documentation cost

The audited root policy at `456ef91b` was 50,989 B, 1,409 lines, 7,126 whitespace-delimited words,
and 40 second-level sections. The supplied steady-state replacement is 40,812 B, 566 lines, 5,384
words, and 30 second-level sections: 10,177 B, 843 lines, 1,742 words, and 10 sections fewer. These
are direct file measurements, not token estimates.

For the active task-reading set—root policy, README, status, roadmap, and all three specifications—the
audited checkout total was 113,289 B / 2,287 lines / 15,334 words. The final set is 105,520 B / 1,513
lines / 13,940 words. The product explanation and safety contract grew while duplicated policy was
removed; the net change is -7,769 B, -774 lines, and -1,394 words. Files use LF endings. Architecture
and performance remain linked fact owners rather than required wholesale task preamble.

### Supplemental coding-agent session telemetry

The active harness exposed provider-reported telemetry for this campaign's parent coding-agent
session. The task boundary is the user campaign prompt in this session. At the retained cutoff
`2026-08-15T17:37:07.377Z` (before the final completion response), `openai-codex/gpt-5.6-sol` with
reported reasoning level `max` had 141 assistant provider responses and 167 tool calls over
2,369.301 s wall time. Provider usage fields were:

| Provider field | Reported value |
| --- | ---: |
| uncached input tokens | 473,353 |
| cached input tokens | 22,298,624 |
| cache-write tokens | 0 |
| output tokens | 74,261 |
| reasoning tokens (reported subset) | 25,389 |
| total tokens | 22,846,238 |
| input cost | $2.366765 |
| cached-input cost | $11.149312 |
| output cost | $2.227830 |
| total cost | $15.743907 |

No explicit provider retry count was surfaced. These values exclude separate reviewer-subagent
sessions and the final response, and they do not isolate model quality from tools, repository state,
prompt size, or orchestration. They are direct provider telemetry, not a byte conversion and not a
model comparison. The deterministic job-policy transport measurement remains independent of this
model session.

### Fresh build, test, and binaries

Fresh target directories under `/tmp` were newly allocated and removed only after recording sizes:

```sh
TIMEFORMAT='fresh_release_build_elapsed_s=%3R'
time CARGO_TARGET_DIR="$FRESH_RELEASE_TARGET" \
  cargo build --workspace --release --locked

time CARGO_TARGET_DIR="$FRESH_TEST_TARGET" \
  cargo test --workspace --all-targets --all-features --locked
```

The final fresh release build took 58.545 s and occupied 52,207 KiB. Release binaries are unchanged at
3,779,472 B (`lkjscript`) and 2,245,680 B (`lkjscriptd`). The separate final fresh full test took
19.396 s, occupied 439,411 KiB, and reported 167 active passes with nine ignored manual
measurements/smokes.
The new active job-policy test itself completed in 0.19 s in that debug-profile boundary. These are
single build observations; no regression ratio is claimed against an unequal prior checkout.

`cargo miri --version` was attempted and failed because the Miri component is unavailable for the
installed `stable-x86_64-unknown-linux-gnu` toolchain. No nightly configuration was added. A source
audit found no package `unsafe` block, unsafe-code lint exception, foreign linkage, or project
`build.rs`; dependency internals remain part of the trusted computing base described in architecture.

## Nominal-data campaign environment

Measurements were retained on 2026-08-15 from the dirty campaign working tree based on starting
commit `99d7ca5bbdac6bcf90fdd64721c13df1342ef67a` on `main`: `devbox`, Linux
7.0.0-29-generic x86-64, AMD Ryzen 9 9955HX (32 logical CPUs visible), 32 GiB memory,
`rustc 1.96.0 (ac68faa20 2026-05-25)`, and Cargo 1.96.0. The cgroup memory ceiling was
34,359,738,368 B, CPU quota was unlimited, shell stack limit was 8 MiB, and virtual memory was
unlimited. Runtime and schema harnesses used optimized release binaries. At that historical point the
machine schema had seven sections and canonical digest
`983614734f16b5d2095279fb5e958814e839caaa7aa25a5a6963cfca44795e2d`; the active contract now uses
roots and has a different digest.
`/usr/bin/time` is unavailable, so maximum RSS is unmeasured. No model was invoked; bytes are not
model tokens or API cost, and no token or performance-leadership claim is made.

## Closed machine-schema projections

The retained byte harness is:

```sh
cargo test --lib machine::tests::schema_projection_byte_measurements_are_retained \
  --locked -- --nocapture
```

It serializes compact result JSON without a newline and the production framed-JSON
`Response::DescribeSchema` with request ID 1. Historical section rows below remain labelled; the
active task projection requests 12 operational endpoint roots and returns their closed named definitions.

| Projection | Compact result JSON | Framed response | Local / daemon round trips |
| --- | ---: | ---: | ---: |
| old audited full (prior schema authority; unequal workload) | 21,516 B stdout | 9,166 B binary | 0 / 1 |
| historical section-era manifest | 739 B | 736 B binary | 0 / 1 |
| historical six nominal sections | 86,009 B | 86,006 B binary | 0 / 1 |
| historical section-era full | 126,888 B | 96,083 B binary | 0 / 1 |
| historical section-era unchanged | 105 B | 48 B binary | 0 / 1 |
| rejected under-specified root manifest | 1,029 B | 1,107 B framed JSON | 0 / 1 |
| rejected 13 leaf/task roots / 96 definitions | 66,957 B | 67,035 B framed JSON | 0 / 1 |
| rejected duplicated 12 endpoints / 254 definitions | 149,720 B | 149,798 B framed JSON | 0 / 1 |
| active endpoint root manifest | 1,241 B | 1,319 B framed JSON | 0 / 1 |
| active 12 endpoint roots / 111 definitions | 80,629 B | 80,707 B framed JSON | 0 / 1 |
| active full | 124,430 B | 124,508 B framed JSON | 0 / 1 |
| active unchanged | 105 B | 183 B framed JSON | 0 / 1 |

Historical rows are retained only as labelled observations and are not equal-schema or equal-transport
regression comparisons. `lkjscript schema` computes locally; daemon `DescribeSchema` returns the same
projection in one request/response. The current endpoint bytes come from the focused debug test above;
no release endpoint timing distribution was run.

The following displaced section-era timing retained one warm-up per command and 31 new release CLI
processes per projection. Python `time.monotonic_ns` included each CLI launch; percentiles use nearest
rank and stdout included the CLI newline. The removed section CLI is historical and is not an active
runnable command.

| Historical section-era local projection | Stdout | Samples | Median | p95 |
| --- | ---: | ---: | ---: | ---: |
| manifest | 740 B | 31 | 882,409 ns | 1,142,849 ns |
| six nominal sections | 86,010 B | 31 | 1,173,126 ns | 1,342,766 ns |
| full | 126,889 B | 31 | 1,201,760 ns | 1,511,603 ns |
| unchanged | 106 B | 31 | 852,934 ns | 980,233 ns |

## Nominal Reading/Input application

The retained real generic-CLI measurement is:

```sh
cargo test --release --test agent_repair_json nominal_agent_interaction_cost_measurement \
  --locked -- --ignored --nocapture --test-threads=1
```

It uses the production daemon and launches the strict generic CLI for each measured request. One
transaction creates Reading/Input plus seven functions, requests 18 selected bindings, expands to 97
canonical nodes, and publishes an incomplete revision. One intentional type error is followed by an
identity-preserving product refinement, layout and diff queries, Run, typed shutdown, restart, and a
retained-node query. Typed shutdowns are lifecycle requests and are explicitly excluded from agent
round trips.

| Request | JSON request | JSON stdout | Binary request | Binary response | CLI + daemon wall |
| --- | ---: | ---: | ---: | ---: | ---: |
| schema manifest | 110 B | 817 B | 17 B | 736 B | 1,697,093 ns |
| six schema sections | 272 B | 86,087 B | 31 B | 86,006 B | 2,343,348 ns |
| known digest unchanged | 354 B | 183 B | 63 B | 48 B | 1,624,797 ns |
| workspace creation | 69 B | 325 B | 15 B | 120 B | 11,220,107 ns |
| structured nominal creation | 8,297 B | 1,256 B | 1,517 B | 658 B | 8,975,464 ns |
| Reading repair context | 376 B | 6,556 B | 98 B | 2,298 B | 1,365,880 ns |
| invalid identity-keyed repair | 882 B | 329 B | 221 B | 142 B | 849,348 ns |
| valid identity-keyed repair | 935 B | 488 B | 237 B | 154 B | 8,626,779 ns |
| Reading layout | 266 B | 741 B | 85 B | 292 B | 483,539 ns |
| semantic diff | 224 B | 1,547 B | 69 B | 464 B | 519,407 ns |
| main Run | 234 B | 156 B | 83 B | 40 B | 459,544 ns |
| retained refined-hole query after restart | 247 B | 548 B | 81 B | 185 B | 447,061 ns |
| **total** | **12,266 B** | **99,033 B** | **2,517 B** | **91,143 B** | **38,612,367 ns** |

There are 12 measured CLI invocations and daemon round trips: 11 successful semantics and one expected
semantic error. Cold daemon readiness was 5,358,601 ns and restart readiness was 5,422,811 ns.
Revision-1 and revision-2 artifacts are 4,213 B and 4,256 B; HEAD is 268 B. The Reading layout oracle
is size 16, alignment 8, and two runtime cells. The measured main Run returned `i64(42)` with
52,298 ns compile/lower/verify and 11,471 ns interpreter execution. These timing rows are one
observation each, not distributions.

The retained repeated harness is:

```sh
cargo test --release --test agent_repair_json nominal_reading_performance_measurement \
  --locked -- --ignored --nocapture --test-threads=1
```

It performs one warm-up and 31 measured generic-CLI Run requests per route.

| Measurement | Samples | Median | p95 |
| --- | ---: | ---: | ---: |
| main request wall | 31 | 414,680 ns | 510,490 ns |
| main compile/lower/verify | 31 | 19,987 ns | 60,103 ns |
| main interpreter execution | 31 | 3,186 ns | 10,851 ns |
| nominal Input match request wall | 31 | 423,266 ns | 533,664 ns |
| nominal Reading output request wall | 31 | 414,580 ns | 498,878 ns |

The typed oracles are main `42`, sample payload `5`, and returned Reading value `9`. A separate single
layout query took 445,448 ns and reasserted size 16/alignment 8/two cells. A single restart took
5,341,389 ns and reasserted the exact retained layout; no median or p95 is claimed for either single
observation.

### Fresh build, test, binary, and boundary evidence

Fresh targets were newly allocated under `/tmp`; no Cargo target was cleaned or reused:

```sh
FRESH_RELEASE_TARGET=/tmp/lkjscript-final2-release.lrOvgc
FRESH_TEST_TARGET=/tmp/lkjscript-final2-test.MCHhFp
TIMEFORMAT='fresh_release_build_elapsed_s=%3R'
time CARGO_TARGET_DIR="$FRESH_RELEASE_TARGET" cargo build --workspace --release --locked
TIMEFORMAT='fresh_full_test_elapsed_s=%3R'
time CARGO_TARGET_DIR="$FRESH_TEST_TARGET" \
  cargo test --workspace --all-targets --all-features --locked
du -sk "$FRESH_RELEASE_TARGET" "$FRESH_TEST_TARGET"
stat -c '%n %s' "$FRESH_RELEASE_TARGET/release/lkjscript" \
  "$FRESH_RELEASE_TARGET/release/lkjscriptd"
TIMEFORMAT='incremental_release_build_elapsed_s=%3R'
time cargo build --workspace --release --locked
```

The final fresh release build took 56.476 s and occupied 52,207 KiB. Its client was 3,779,472 B and
daemon 2,245,680 B. The separate fresh full test took 17.948 s, occupied 457,561 KiB, and reported 166
active passes with eight ignored manual measurement/smoke tests. The unchanged incremental release
build took 0.024 s; repository release binaries had the same sizes. The temporary targets were removed
only after sizes were recorded. Compared with the separately retained structured and reset baselines
below, these are accepted capability/build regressions, not equal-work performance claims.

The full boundary specifically passed malformed Core aggregate/switch rejection, exhaustive selected-arm
execution, strict malformed JSON and protocol rejection, exact aggregate copy fuel, selected-large-arm
fuel exhaustion, entry/callee/live-cell exhaustion, nominal restart, exact cycle-participant
selection, and iterative deep match/type traversal tests. `examples/named-data/run.sh` additionally
proves an overflowing arm is lazy when
unselected and traps when selected through the public Run path. The final deterministic boundary command was:

```sh
LKJSCRIPT_MUTATION_SEED=1 LKJSCRIPT_MUTATION_CASES=10000 \
  cargo test --release --lib campaign_tests::boundary_mutation_smoke --locked -- \
  --ignored --nocapture --test-threads=1
```

It passed one test in 0.03 s and printed `seed=1 cases=10000`; this is bounded deterministic mutation
testing, not coverage-guided fuzzing.

## Structured pure-program campaign (retained older baseline)

### Environment and method

Measurements were retained on 2026-08-15 from the final campaign tree based on
`dc541eb3ebb7a54006e8057d0f76b0596cf012e4`: `devbox`, Linux 7.0.0-29-generic x86-64,
Rust/Cargo 1.96.0, AMD Ryzen 9 9955HX, 32 GiB memory. Runtime harnesses use release binaries and one
warm-up before reported samples. Oracles are typed `i64(5050)`, `i64(0)`, `i64(55)`,
finite-recursion `i64(1)`, and `execution_fuel_exhausted`. Percentiles use nearest rank. Shell
`time` measured fresh commands because `/usr/bin/time` is unavailable; maximum RSS therefore
remains unmeasured.

The retained real generic-CLI interaction harness is:

```sh
cargo test --release --test agent_repair_json structured_agent_interaction_cost_measurement \
  --locked -- --ignored --nocapture --test-threads=1
```

It launches the production CLI for every request. The one structured creation has 6 public
transaction items, requests 4 explicit bindings, and expands to 36 canonical nodes. Revision-1 and
revision-2 artifacts are 1,584 B and 1,601 B. The complete measured sequence uses 11 CLI
invocations/daemon round trips: schema discovery, workspace creation, structured creation, repair
context, invalid repair, valid repair, semantic diff, three runs, and one retained query after
restart.

| Request | JSON request | JSON stdout | Binary request | Binary response | CLI wall |
| --- | ---: | ---: | ---: | ---: | ---: |
| schema discovery (then-default full) | 67 B | 21,516 B | 15 B | 9,166 B | 655,402 ns |
| workspace creation | 68 B | 324 B | 15 B | 120 B | 10,098,864 ns |
| structured creation | 3,106 B | 660 B | 527 B | 266 B | 8,934,375 ns |
| repair context | 375 B | 5,472 B | 98 B | 1,761 B | 440,939 ns |
| invalid repair | 375 B | 310 B | 85 B | 137 B | 430,550 ns |
| valid repair | 577 B | 487 B | 136 B | 154 B | 9,040,735 ns |
| semantic diff | 224 B | 1,215 B | 69 B | 356 B | 477,839 ns |
| main run | 234 B | 157 B | 83 B | 40 B | 411,544 ns |
| direct `-3` run | 258 B | 151 B | 92 B | 40 B | 318,298 ns |
| direct `11` run | 258 B | 153 B | 92 B | 40 B | 301,657 ns |
| restart retained query | 246 B | 503 B | 81 B | 161 B | 385,344 ns |
| **total** | **5,788 B** | **30,948 B** | **1,293 B** | **12,241 B** | **31,495,547 ns** |

Daemon cold readiness was 6,429,787 ns and restart readiness with the retained workspace was
4,294,382 ns. These are byte, process, and round-trip measurements, not model-token or API-cost
measurements. That campaign's then-default full schema discovery is intentionally included because
the runtime-generated schema is the agent's authoritative vocabulary; the current default is the
compact manifest measured above.

The retained repeated product-path harness is:

```sh
cargo test --release --test agent_repair_json structured_product_path_performance_measurement \
  --locked -- --ignored --nocapture --test-threads=1
```

| Measurement | Median | p95 | Samples |
| --- | ---: | ---: | ---: |
| daemon cold start | 5,315,642 ns | 5,334,448 ns | 11 |
| workspace creation, generic CLI wall | 10,313,819 ns | 10,988,907 ns | 31 |
| structured incomplete commit, generic CLI wall | 9,015,217 ns | 9,959,452 ns | 11 |
| nested repair context, generic CLI wall | 353,384 ns | 459,944 ns | 31 |
| main request wall | 297,600 ns | 490,022 ns | 31 |
| main compile/lower/verify | 3,927 ns | 15,068 ns | 31 |
| main interpreter execution | 14,898 ns | 17,633 ns | 31 |
| direct parameterized run wall | 295,776 ns | 393,129 ns | 31 |
| finite recursion wall | 274,135 ns | 358,234 ns | 31 |
| controlled fuel exhaustion wall | 293,812 ns | 348,896 ns | 31 |
| daemon restart with retained workspaces | 5,327,895 ns | 6,374,162 ns | 11 |

CLI startup dominates sub-millisecond request rows. Main execution is materially larger than the old
scalar `42` interpreter micro-observation because it performs calls, a conditional, 101 loop
iterations, checked increments, and branch transfers; this is a different representative workload,
not a regression ratio for equal work.

### Fresh build, test, and binary observations

Fresh targets were separate `mktemp` directories; repository targets were not deleted or reused:

```sh
FRESH_RELEASE_TARGET="$(mktemp -d /tmp/lkjscript-structured-build.XXXXXX)"
FRESH_TEST_TARGET="$(mktemp -d /tmp/lkjscript-structured-test.XXXXXX)"
printf '%s\n' "$FRESH_RELEASE_TARGET" "$FRESH_TEST_TARGET"

TIMEFORMAT='fresh_release_build_elapsed_s=%3R'
time CARGO_TARGET_DIR="$FRESH_RELEASE_TARGET" cargo build --workspace --release --locked

TIMEFORMAT='fresh_full_test_elapsed_s=%3R'
time CARGO_TARGET_DIR="$FRESH_TEST_TARGET" \
  cargo test --workspace --all-targets --all-features --locked

du -sk "$FRESH_RELEASE_TARGET" "$FRESH_TEST_TARGET"
stat -c '%n %s' "$FRESH_RELEASE_TARGET/release/lkjscript" \
  "$FRESH_RELEASE_TARGET/release/lkjscriptd"
rm -rf -- "$FRESH_RELEASE_TARGET" "$FRESH_TEST_TARGET"

# Unchanged-worktree incremental release observation:
TIMEFORMAT='incremental_release_build_elapsed_s=%3R'
time cargo build --workspace --release --locked
stat -c '%n %s' target/release/lkjscript target/release/lkjscriptd
```

The final fresh release build was 28.519 s and 38,095 KiB. Its separate final fresh full test was
13.212 s and 249,828 KiB, with 121 active tests passing and 6 ignored manual tests. Unchanged
incremental release build was 0.033 s. Release binaries are `lkjscript` 1,786,936 B and
`lkjscriptd` 1,184,712 B.

Against the audited reset baseline, fresh release time increased from 6.665 s to 28.519 s and target
size from 7.9 MiB to about 37.2 MiB. Fresh test time increased from 2.122 s to 13.212 s. The client
binary increased from 418,416 B by 1,368,520 B (+327.1%); the daemon increased from 806,920 B by
377,792 B (+46.8%). Against the prior JSON/repair campaign observation, the client grew 384,984 B
(+27.5%), the daemon 263,112 B (+28.5%), and incremental release moved from 0.023 s to 0.033 s.
These are accepted capability/build regressions, not performance wins. Reversal conditions remain a
material distribution/build constraint plus an alternative that preserves strict JSON, generated
schema, structured semantics, and one executable route.

### Authoring baseline comparison

The audited scalar low-level baseline required 11 public transaction items for its small `40 + 2`
program. The new representative request uses 6 items (45.5% fewer) while defining three functions,
two parameters, calls, an `if`, a loop, and a nested hole. This item-count comparison is semantic
interaction evidence across different capabilities, not equal-work latency or wire evidence. Calls
and structured control were not expressible, so there is no honest production old wire measurement
for the complete representative program.

A disposable detached worktree at `dc541eb3` was temporarily instrumented and removed after measuring
the largest directly comparable old production subset used here: one package, one module, and three
zero-parameter functions, each with an explicit region, block, constant, return terminator, and body
attachment; `main` returns a constant `5050`. The old product had no parameters, calls, `if`, loop,
block arguments, or nested hole with which to encode the missing representative meaning. One release
sample through the real generic CLI/daemon produced:

| Old production subset dimension | Observation |
| --- | ---: |
| public transaction items | 21 |
| returned handles / created nodes | 17 / 17 |
| semantic nodes including initial workspace root | 18 |
| functions / regions / blocks | 3 / 3 / 3 |
| operation nodes / return terminators | 6 / 3 |
| function-body / package-entry attachments | 3 / 1 |
| compact JSON request / stdout response | 2,849 B / 1,210 B |
| framed binary request / response | 477 B / 630 B |
| one transaction CLI wall | 8,961,607 ns |
| revision artifact | 813 B |

The disposable procedure was `git worktree add --detach "$OLD_WT" dc541eb3`, append the temporary
focused test `temporary_old_low_level_representative_subset_measurement` to
`tests/agent_repair_json.rs`, run `cargo test --release --test agent_repair_json
 temporary_old_low_level_representative_subset_measurement --locked -- --nocapture
 --test-threads=1`, then run `git worktree remove --force "$OLD_WT"` and `git worktree prune`.
Temporary instrumentation and its build directory were removed; the table retains its complete typed
shape assumptions and observed dimensions without turning measurement code into an active old API.

The implemented structured creation uses 6 items, returns only 4 selected bindings, creates 36 nodes,
and measures 3,106/660 JSON request/response bytes, 527/266 binary request/response bytes,
8,971,224 ns one-transaction CLI wall, and a 1,584 B incomplete artifact. It therefore uses 71.4%
fewer public items while expressing materially more semantics. Its request is 9.0% larger in JSON and
10.5% larger in binary than the inexpressive old subset; its selected-binding response is 45.5%
smaller in JSON and 57.8% smaller in binary. Artifact and latency observations are different-workload
context, not equal-work regressions or wins.

No exact old binary dimension exists for the nested call/control scaffolding: the old production
binary codec has no variant tags or payload grammar for parameters, calls, `if`, loops, block
arguments, or structured holes. An "exact synthetic binary" would therefore require inventing a
non-production protocol and is intentionally not reported. The JSON structural model below is kept
explicitly synthetic because JSON can at least state named hypothetical records without claiming the
old decoder accepted them.

For structural comparison only, an exact synthetic compact-JSON encoder counted the representative
canonical shape as 36 created nodes: 1 package, 1 module, 3 functions, 2 parameters, 6 regions,
6 blocks, 2 block arguments, and 15 operations/terminators. Under the stated assumption that the old
shape needed one explicit create item per canonical node, three explicit function-body attachments,
and one entry selection, it produced 40 transaction items and 6,804 compact JSON bytes. The
synthetic payload used full field names (`create_canonical_node`, `local_handle`,
`canonical_node_kind`, `owner`, `owner_slot`), excluded the semantic payloads for calls/control that
the old product could not express, and omitted a versioned envelope. It is therefore a lower-bound
structural estimate, not old production bytes and not an old binary measurement.

The implemented structured request uses 6 semantic transaction items and 3,106 JSON bytes including
the versioned envelope and all call/control payloads: 85.0% fewer public items than the synthetic
shape and 54.4% fewer bytes than that deliberately incomplete structural lower bound. This is
semantic compression from implied regions/blocks/arguments/terminators and aggregate function
bodies, not abbreviated naming. No token saving is inferred.

## Retained reset baseline

### Environment

- date: 2026-08-14;
- code state: `5785f650cbe11c38350a8005ba17f4bd40bb84b6`;
- host: `devbox`, AMD Ryzen 9 9955HX, 20 logical CPUs visible, 32 GiB memory;
- OS: Linux 7.0.0-29-generic x86-64, glibc 2.39;
- Rust: `rustc 1.96.0 (ac68faa20 2026-05-25)`;
- mode: release for runtime measurements;
- workload: one 10-node source-free workspace whose entry computes `40 + 2`;
- oracle: typed result must equal `i64(42)` and artifact round-trip must be byte-identical.

### Build and test baseline

A fresh target directory avoided deleting or reusing repository build state:

```sh
fresh_target=$(mktemp -d /tmp/lkjscript-fresh-target.XXXXXX)
TIMEFORMAT='fresh_release_build_elapsed_s=%3R'
time CARGO_TARGET_DIR="$fresh_target" cargo build --workspace --release --locked
```

Observed elapsed time: 6.665 s. The fresh release directory occupied 7.9 MiB.

A separate fresh target ran the complete test boundary:

```sh
fresh_target=$(mktemp -d /tmp/lkjscript-fresh-test.XXXXXX)
TIMEFORMAT='fresh_full_test_elapsed_s=%3R'
time CARGO_TARGET_DIR="$fresh_target" \
  cargo test --workspace --all-targets --all-features --locked
```

Observed elapsed time: 2.122 s; 29 tests passed and the manual performance baseline was ignored.
`/usr/bin/time` was unavailable, so retained maximum-RSS evidence is not available.

### Product-path scalar baseline

The retained ignored integration test starts the real foreground daemon binary in a new temporary
directory and sends production protocol requests. It performs one warm-up, samples summary and run
31 times, shuts down, then samples workspace restart 11 times. Percentiles use nearest-rank
ordering. Request wall time includes a new Unix-socket connection and one frame each way but not a
new client process. Internal compile/execute timings exclude IPC.

```sh
cargo test --release --test semantic_vertical product_path_performance_baseline -- \
  --ignored --nocapture --test-threads=1
```

| Measurement | Median | p95 | Samples |
| --- | ---: | ---: | ---: |
| workspace query, request wall | 7.504 us | 9.769 us | 31 |
| run, request wall | 7.344 us | 9.087 us | 31 |
| direct SPG validation/lowering/Core IR verification | 0.481 us | 0.782 us | 31 |
| interpreter execution | 0.070 us | 0.130 us | 31 |
| daemon restart with one retained workspace | 5,314.193 us | 5,347.456 us | 11 |

The retained run began at `2026-08-14T09:53:48Z`. Single observations: initial daemon readiness
5,319.614 us; durable workspace creation 10,093.611 us; durable bootstrap transaction 7,900.848 us.
The revision-1 artifact was 501 bytes. Its full wall time was 5.650 s including a release rebuild.

## Pre-change hole-discovery baseline

The required pre-change observation used a detached disposable worktree at
`a503c0b1269ed3e149f83bb0f8ad8d4f75550cbc`. A temporary ignored integration test reused the real
foreground daemon, old protocol-v1 `Client`, and the existing incomplete-hole fixture. Fixture
creation was excluded. For each discovery request it measured production framed binary bytes with
`encoded_request_size`/`encoded_response_size` and wall time around `Client::request`; the worktree
and instrumentation were removed afterward.

```sh
git worktree add --detach /tmp/lkjscript-prechange a503c0b1269ed3e149f83bb0f8ad8d4f75550cbc
# append disposable prechange_hole_discovery_measurement to tests/semantic_vertical.rs
cargo test --release --test semantic_vertical prechange_hole_discovery_measurement --locked -- \
  --ignored --nocapture --test-threads=1
git worktree remove --force /tmp/lkjscript-prechange
```

| Old request | Request | Response | Elapsed | Whole node record |
| --- | ---: | ---: | ---: | --- |
| blockers | 39 B | 99 B | 15,850 ns | no |
| hole exact view | 64 B | 130 B | 15,219 ns | yes |
| owner block exact view | 64 B | 168 B | 11,872 ns | yes |
| owner region summary | 64 B | 118 B | 9,979 ns | no |
| function exact view | 64 B | 178 B | 10,931 ns | yes |
| prior operation exact view | 64 B | 137 B | 8,807 ns | yes |
| following operation exact view | 64 B | 149 B | 8,476 ns | yes |
| **total** | **423 B** | **979 B** | **81,134 ns** | **5 of 7** |

The largest old response was 178 B. Seven daemon requests were required to identify the hole and
expected type, read its owner block, reconstruct nearby order, walk manually to the function, and
inspect the immediate preceding/following operations. The simple fixture's one visible `i64`
candidate and one incoming `add_i64` use could then be inferred manually from those exact records.
The old protocol had no visibility/dominance query and no incoming-use query, so those two facts were
not directly available in the general case without recursively traversing and interpreting node
records. It also had no legal-constructor fact. No whole-workspace dump was requested, but five
whole node records were required. No model was invoked and no token estimate was made.

The current correct workflow is three real generic-CLI requests (repair context, refinement, run).
Its one context response directly composes typed owner/body/visibility/use/constructor facts; it
requires no whole node record. The old 7-request discovery-only number and current 3-request
completion number are therefore direct round-trip evidence, not an equal-payload latency or byte
comparison.

## Agent-repair campaign environment

Measurements below use the uncommitted campaign tree based on
`a503c0b1269ed3e149f83bb0f8ad8d4f75550cbc`, Linux 7.0.0-29-generic x86-64, 32 logical CPUs visible,
AMD Ryzen 9 9955HX, 32 GiB memory, and Rust 1.96.0. They are single observations unless a sample
count is shown. Shell `time` supplied elapsed time; `/usr/bin/time` remains unavailable, so maximum
RSS was not measured.

### Real generic-CLI repair cost

`agent_repair_cost_measurement` starts the real daemon and launches the real generic JSON CLI once
per request. After an incomplete 11-node workspace exists, the simple correct workflow is exactly
repair context, `RefineHole`, and revision-bound run. The result oracle is `i64(42)`. The separate
invalid edit wires `bool` into `add_i64`, rejects once, and does not count toward the three-request
correct workflow. Blocker discovery, one complete diff page, and workspace summary are also reported
separately. No whole-workspace dump was requested. The retained observation was rerun after final
request- and response-side EOF hardening.

```sh
cargo test --release --test agent_repair_json agent_repair_cost_measurement --locked -- \
  --ignored --nocapture --test-threads=1
```

| Correct workflow request | Elapsed | JSON request | JSON stdout | Binary request | Binary response |
| --- | ---: | ---: | ---: | ---: | ---: |
| repair context | 399,271 ns | 373 B | 3,993 B | 98 B | 1,274 B |
| refine | 8,639,555 ns | 680 B | 485 B | 154 B | 154 B |
| run | 343,576 ns | 167 B | 150 B | 63 B | 40 B |
| **total (3 CLI invocations/round trips)** | **9,382,402 ns** | **1,220 B** | **4,628 B** | **315 B** | **1,468 B** |

Largest correct-workflow JSON response and context response were both 3,993 B; compact receipt was
485 B. Separate observations: blockers 408,448 ns / 207 B request / 386 B stdout; deterministic
invalid edit 426,873 ns / 627 B / 325 B; diff page 371,639 ns / 223 B / 1,317 B; workspace summary
287,200 ns / 188 B / 478 B. The invalid scenario had one rejected edit before success. Exact
production binary sizes for those four responses were 127 B, 141 B, 346 B, and 162 B respectively.

These are byte and round-trip measurements, not token measurements. No model was invoked and no
model-token conversion is claimed. CLI process startup dominates the correct refinement observation;
the measurement intentionally includes it because that is the external agent boundary.

### Scan-query cost

`query_performance_measurement` warms each direct derived query once and samples it 31 times in a
release test. Median is sorted sample 16 and p95 is sample 30. JSON result bytes are compact
`serde_json` encoding of the typed result and exclude envelope/framing. The implementation uses full
scans and no index/cache.

```sh
cargo test --release query_performance_measurement --locked -- \
  --ignored --nocapture --test-threads=1
```

| Workload/query | Nodes | Median | p95 | Result bytes |
| --- | ---: | ---: | ---: | ---: |
| scalar workspace summary | 11 | 60 ns | 61 ns | 282 B |
| scalar body | 11 | 170 ns | 280 ns | 1,057 B |
| scalar incoming uses | 11 | 220 ns | 290 ns | 372 B |
| scalar repair context | 11 | 922 ns | 1,412 ns | 3,799 B |
| scalar adjacent diff | 11 | 681 ns | 751 ns | 293 B |
| scalar non-adjacent diff, revision 0 to 2 | 11 | 1,032 ns | 1,423 ns | 1,295 B |
| scalar four-item validated query batch | 11 | 1,483 ns | 2,415 ns | 5,515 B |
| first 256 of body after 3,000 added operations | 3,011 | 12,734 ns | 14,558 ns | 49,546 B |
| repair context in that body | 3,011 | 180,018 ns | 205,497 ns | 8,713 B |
| repair context with 3,000 unrelated packages | 3,011 | 16,621 ns | 17,653 ns | 3,796 B |
| incoming uses with 3,000 unrelated packages | 3,011 | 3,226 ns | 3,336 ns | 372 B |

The non-adjacent row is a follow-up run of the same 31-sample release harness after adding explicit
revision-0-to-2 coverage; the other rows retain the prior campaign observation. The full-scan trend
is visible but remains sub-millisecond at 3,011 nodes on this microbenchmark. Reversal condition:
add one narrow derived index only after representative repeated agent workloads
show scan cost material to end-to-end latency, retain scan differential tests as oracle, and prove
exact invalidation. Do not infer asymptotic or application performance from these fixtures.

### Current scalar product-path rerun

The retained scalar test was rerun without replacing the old baseline: startup 11,714.256 us,
workspace creation 10,267.075 us, durable transaction 8,054.705 us; request-wall summary median/p95
10.029/13.595 us; run 8.936/10.570 us; compile 0.390/0.621 us; execute 0.070/0.100 us; restart
median/p95 5,341.233/5,413.289 us (11 samples); artifact 501 B.

### Build, dependencies, and binaries

Fresh target directories avoided cleaning repository state. Current fresh release build took
21.790 s and 34 MiB, compared with retained 6.665 s / 7.9 MiB. A separate fresh full test took
6.556 s and 119 MiB, compared with 2.122 s. Those were historical intermediate campaign
observations; their test counts are superseded by the structured campaign boundary above. The
historical unchanged incremental release observation was 0.023 s.

Direct normal dependencies increased from 3 to 5; unique normal `cargo tree` packages from 10 to 21.
The two direct additions are `serde` with only `derive,std` and `serde_json` with only `std`, both
with default features disabled. Resolved versions were 1.0.229 and 1.0.151. The 11-package normal
transitive delta is serde/core/derive, serde_json, itoa, memchr, zmij, proc-macro2, quote, syn, and
unicode-ident. Licenses are MIT or Apache-2.0 compatible with this Apache-2.0 project (memchr also
offers Unlicense; unicode-ident also carries Unicode-3.0). Their current named consumer is strict,
closed, exhaustively tested machine JSON projection. A local JSON parser/serializer would duplicate
Unicode, numeric, escaping, recursion, duplicate/unknown-field, and streaming-output security work
at higher maintenance and review risk.

| Release binary | Retained baseline | Current | Delta |
| --- | ---: | ---: | ---: |
| `lkjscript` | 418,416 B | 1,401,952 B | +983,536 B (+235.1%) |
| `lkjscriptd` | 806,920 B | 921,600 B | +114,680 B (+14.2%) |

The client size and fresh-build regressions are accepted campaign costs for the generic strict JSON
boundary, not performance wins. Reversal condition: reconsider serde feature use or projection
placement only if distribution/build constraints become material while retaining identical strict
coverage and one typed authority.

### Durability, mutation, and remaining baselines

Compact default receipts are preflighted independently of full diff size, and `LKJHEAD4` remains under
16 KiB even at maximum 64 selected bindings (the focused test asserts it is under 4 KiB). A
moderate 200-create transaction proves receipt size remains selected-projection bounded. Persistence
still clones the full snapshot, materializes the full semantic diff during preparation, and rewrites
a full canonical artifact; the scalar transaction/restart observations above do not justify a
journal, database, or incremental store.

The final deterministic malformed-boundary release smoke completed 10,000 cases with seed 1 in
0.03 s of reported test time (release compilation excluded):

```sh
LKJSCRIPT_MUTATION_SEED=1 LKJSCRIPT_MUTATION_CASES=10000 \
  cargo test --release boundary_mutation_smoke --locked -- \
  --ignored --nocapture --test-threads=1
```

It mutates artifact, binary protocol, and JSON byte corpora deterministically. Typed transaction
atomicity is exercised by a separate deterministic generated-sequence test in the normal suite; it
is not part of the seed/case byte-mutation loop. The smoke is explicitly not coverage-guided
fuzzing. A future coverage-guided harness remains warranted for the three byte decoders; this
bounded smoke does not support a fuzz-coverage claim.

Full snapshot cloning/recomputation, full diff materialization, full artifact rewrite, and retained
full history remain deliberate baselines. Reverse them only with representative workload evidence,
an unchanged semantic oracle, deterministic artifacts/receipts, and durability failure-injection
evidence.
