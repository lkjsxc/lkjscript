# Current status

Snapshot: 2026-10-08. Source acceptance, distribution acceptance and running
applications are separate boundaries. Product identifier components remain opaque.

## Accepted source

Accepted source: `3389860cd34b1997a4f9a0a7737588ba2697f63a`, tree
`39c51f302171d9ef91479e68d1bab4c9c8a46909`. Its fresh full source verification passed
26/26 gates, with stable inputs, zero reused evidence and zero unrun gates.
Workspace tests passed 2,228, failed 0 and retained 29 existing ignored tests.
The 234 public CLI tests are included in that total; two filtered child probes
are separate. The status-only reporting descendant is not the tested source.

Receipt:
`.artifacts/lkjscript-dev/check/1791442547071589942-2964229-0/receipt.json`, digest
`verification_0a99bd38eff0a4420c41f4ce44d837d24c3b69273f4bd06c2f0c3a516d4ee259`.
The receipt and its complete run directory are also retained outside checker
rotation in `.artifacts/20261008-borrowed-metadata/full-source-verification/`.
The evidence index is `.artifacts/20261008-borrowed-metadata/evidence-index.json`.

## Borrowed immutable metadata

Production borrowed product field selection now retains its live allocation's
exact prepared-program admission instead of recursively re-admitting immutable
metadata at each read. Exact type/field selection, current read domain and live
admission remain required. Raw input, packing, consuming unpack, captures and
ownership transfer retain their existing admission. The reference evaluator is
unchanged. See the [decision](decisions/borrowed-immutable-metadata.md) and
[native workload](../examples/owned-metadata-costs/README.md).

Eleven new boundary probes pass, covering foreign program/slot classifications,
unadmitted and revoked storage, restored raw shape, failed adoption, scoped read
domains, reservation refusal, cancellation and owner-independent immutable results.
The detached public matrix passes 100 conditions plus its input/refusal recovery
checks. The stricter regression intentionally rejects the predecessor and accepts
the candidate; that expected rejection is not a candidate test failure.

At N=1,024 and K=128, borrowed metadata raw-result admission drops from 132,354
to 1,026 nodes; complete input admission stays 1,027. Pack/unpack and pack/drop
remain 262,656 and 131,328. Matched invocation medians are 7.066 to 0.465 ms at
N=1,024 and 103.848 to 2.458 ms at N=16,384, with K=128. Preparation remains a
separate roughly 74–77 ms cost. These are not overall application speedups.

The dependency-component witness still matches 532 independent expected graphs
across two carriers and source-present/deleted phases: 2,128 batch comparisons,
plus two individual and two empty runs. Its raw work is unchanged: flat 50,768,886
and chunked 51,024,684 nodes. Matched invocation medians are 7.194 to 7.291 seconds
for flat and 18.376 to 18.254 seconds for chunked. These overlapping samples do not
establish a speed improvement. All 288 metadata and 24 component measurements,
including warmups and unfavorable results, are retained in the evidence root.

The focused executable and the final full-verification release producer have
identical SHA-256 `b14bd646f96061f172e0947f46c0743038a51c0344a1069019afb61441869f72`.
No encoding, native intrinsic, dependency, version identifier or application
source changes are introduced by this step; development remains `0.1.88`.

## Publication and next boundary

The latest independently observed public release remains immutable `v0.1.83`.
Existing `v0.1.88` candidate producer `37643217328/1` completed successfully for
source `6a15fdab20cdbf89ebc8683e8aa1d8b0f64b7142`, not this accepted source. This
step does not replace its artifacts, promote a release or switch running apps.
A later selected distribution must independently bind its own final source and
transferred-executable evidence; the existing candidate must not be relabeled.

Next, retain the exact admitted parent's proof through consuming extraction,
keeping the extracted value and its field proof inseparable. Only then extend
checked construction where needed: a free-value classification alone does not
prove an exact field type. Do not remove raw validation merely to lower counters,
add an address cache, or claim the remaining SCC admission bottleneck is solved.
