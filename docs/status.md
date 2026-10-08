# Status

Current snapshot: 2026-10-08 (Asia/Tokyo). This page owns availability and unfinished
acceptance. [Direction](direction.md), [specifications](spec/) and [roadmap](roadmap.md)
own contracts and future work. External states below are observations, not promises.

## Accepted mainline: v0.1.88 with native dependency components

Selected checkout: `/home/coder/workspace/lkjscript`, branch `main`.
Accepted source is `90d9230945317dd90c924fd29676218fd1bdb117`, tree
`b40de144ad48a50f0b682622e9204fa6e49f2acf`. Normal main push and an independent
GitHub ref read confirmed this exact source. A subsequent status-only reporting
commit does not change the accepted implementation and is not a new full receipt.

Fresh full source verification passed all 26 selected gates, with stable inputs,
zero reused gates and no unrun gates. Workspace tests passed 2,206 cases, failed
zero and retained 29 existing ignored cases. The 234 public CLI successes are
included; two filtered child probes are counted separately. The original receipt is
`.artifacts/lkjscript-dev/check/1791431678798445473-2526622-0/receipt.json`.
Receipt digest:
`verification_97e3fddfe66171745572a0c3a110349251fff8baa87ff9a1359c3b4c78247f9c`.
Receipt SHA-256:
`cd33ea2c9abbc574e3d85ede5fb911546f38c31a91ae280f9471e2cf2c7710a2`.
The evidence index and a preserved full-run copy are under
`.artifacts/20261008-native-components-acceptance/`. Some optional cache writes
reported `cache_store:infrastructure`; the fresh gate executions passed and their
original evidence is retained. No cached proof was used for this acceptance.

## Native dependency-component capability

[Native dependency components](../examples/dependency-components/README.md) extend
the provisional graph validator with complete strongly connected components,
cyclic/acyclic classification and the retained reachability partition. The
[decision](decisions/native-dependency-components.md) owns the scope. All five
implementation modules are literal lkjscript inputs, authored through ordinary
public operations; the host supplies inputs and an independent expected-result
oracle, not program output. There is no new intrinsic, privileged graph writer,
runtime/type-system feature or production compiler substitution.

The inherited test error is corrected: execution-budget refusal now checks exact
class `resource` and code `normalized_instruction_steps`, not explanatory text.
Two native empty-batch tests and two detached public empty-batch runs close the
previously untested batch boundary. Focused public regression and its full-suite
rerun both passed. The focused public native check passed 111 tests with production
and independent reference equality: 105 preexisting, four component tests and two
new empty-batch tests.

The public regression compares 532 input cases, including all 512 directed
three-vertex graphs, under flat and chunked storage both before and after deleting
its disposable authoring projects and transports. The suite repeats a known valid
proposal after invalid proposals; 532 does not mean 532 distinct graphs. These
are 2,128 complete case comparisons, plus two single-example and two empty-batch
runs. It checks exact typed invalid/capacity outcomes, wrong-type and explicit
runtime refusals without result files, unchanged artifact/executable bytes,
zero remaining owners and joined cleanup. The first traversal needs LIFO behavior;
a matching Worklist type alone does not prove that algebraic law. Helper
preconditions are established by the complete application validator.

The focused copied executable SHA-256 is
`1c22ea09ddb9a16756a15e02bace2953e41e42f73402bd5c58645bd087f6ffa9`;
its detached artifact SHA-256 is
`d05c09b01e39336608bf962e3800b69c4474f80662401c4098f8e222d8299e2d`.
All four batch results and their independent expected-output file have SHA-256
`f90964a1a1c2d1117fe8fef18fac2149b55f4879edf0c52b42a53b8e61eba6d9`.
Literal requests, inputs, outputs, observations and cleanup evidence remain in
`.artifacts/20261008-native-components-acceptance/focused-public-proof/`.
Original failures remain unchanged at
`.artifacts/20261008-native-components/handoff.json` and its linked originals;
the former failed full run is not relabeled as successful.

The matched workload retains maximum call depth seven. Flat storage observed
78,313 input-admission nodes and 50,768,886 raw-result-admission nodes; chunked
storage observed 51,024,684 raw-result-admission nodes. These count work, not RSS
or a causal wall-time contribution. No runtime optimization or speedup is claimed.
Next isolate which raw-result boundaries dominate, then evaluate propagation of
already checked immutable evidence without weakening complete raw ingress,
origin/type/ownership checks, cancellation, capacity accounting or cleanup.
Production compiler integration separately needs a checked input/result contract
and cannot acquire meaning-publication authority from successful execution.

## Distribution remains separate

A fresh authenticated API read still identifies immutable
[v0.1.83](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.83), release
`404980756`, as latest. Its accepted source is
`65b3d00428d36827f9d65bf92f7900cf52719d21`.

The existing v0.1.88 producer
[37643217328/1](https://github.com/lkjsxc/lkjscript/actions/runs/37643217328)
previously reached `candidate_accepted` for source
`6a15fdab20cdbf89ebc8683e8aa1d8b0f64b7142`; publication and public verification
were skipped. Original authenticated terminal/acceptance ZIPs and jobs metadata
remain in `.artifacts/20261008-native-components/prior-candidate-*`.
Archive SHA-256: `4d19bb0c33625fd806ba91a643ac172104819f56a19d57d739091972b6a197ba`.
Executable SHA-256: `b25a42987e2bbf4e5e57bcb28fe41e26010171160b11e3b073357d9430320a8a`.
Service asset retention begins expiring at `2026-10-21T17:30:55Z`.
These are original candidate observations, not finalized-byte acceptance for the
new native witness. Do not rebuild or duplicate a healthy accepted producer.
Promotion requires the maintained publication/public-verification boundary with
fresh occupancy and authority checks. No release operation or deployment was
performed in this source-only continuation. Product version, runtime, accepted
meaning, interface and artifact encodings are unchanged; no migration is required
by this change.

Existing stashes, unrelated worktrees, immutable/failed history, services and
operational data remain preserved. Source acceptance, finalized distribution
acceptance and public availability remain separate claims.
