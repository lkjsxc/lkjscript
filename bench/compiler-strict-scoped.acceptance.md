# Independent acceptance of the scoped strict replay evidence

Root accepted the frozen `bb01dd98a284b2e53b11e3f2c2d8b0e533bf186a` result
and integrated it as `85af89c51b2c3f88539f043b41f690e397e9e01c`.
The original six-refusal campaign and both frozen scoped manifests remain unchanged.

Gateway operations `compiler-v989-scoped-root-audit` and
`compiler-v990-scoped-root-audit-result` verified both result manifests,
copied the scoped TSV, metadata and logs into a separate root-owned audit directory,
and reran both committed auditors against the still-present source archives.
The state auditor wrote only the copied output prefix. All four resulting proof,
archive, runtime-input and refusal tables compare byte-for-byte with the frozen
tables. Neither auditor executes the compiler.

The record audit confirms six once-only launches, five passes, 45 operations,
seven required changed-language file facts, five exact target texts and two
text-different targets with equal selected-root canonical checked meaning.
Together with the 24 disjoint prior proofs this is 29 within the named
`6a187b94fb763f7b0565436a98ab5feda3cd642f` last-300-commit horizon.
It does not establish serialized-IR identity, all-root equivalence or reproduction
of non-language Git effects. Thirty remains unmet.

The live-state audit confirms twelve complete archives, 22,202 file instances,
six unchanged runtime-input pairs, seven proven published-file overrides and
no pending intent. Case 3 remains a refusal with no source mutation. Root independently
reproduced these table hashes:

| Table | SHA-256 |
| --- | --- |
| proofs | dbff3d431b1a33375116eabc90091ba3c68feecff53bb7f8c5961a20c4934667 |
| archives | 5e14f2bb9505087e0a0809c839eefb6d2d91a9af5f4f33f3516acd36cd3599c5 |
| runtime inputs | c96b3838640cc45fff1cbcb2310b1212fdb4be0e77aaf656147fe58af5329664 |
| refusals | a870a024b3430948533be192ffdb25cda24ca8ea828d4a5e0b11f39ac4763165 |

The record auditor took 0.13 s wall / 0.09 s user / 0.03 s kernel,
peak RSS 40,056 KiB. The full archive-state auditor took 2.19 / 1.14 / 0.55 s,
peak RSS 12,216 KiB. The load observations bracketing both audits were
4.82 / 4.31 / 6.00 before and 4.99 / 4.36 / 6.00 after (1/5/15 minutes).
Both ran with a 4 GiB address-space ceiling and 900-second deadline and exited zero.
These are audit costs, not replay or build measurements.

The owned fixture remains accepted for the pre-reviewed guarded cleanup.
Cleanup is deferred until the separate container experiment ends to avoid disk
interference with its timings. The frozen textual evidence will remain.
No new replay, compiler execution or recovery was authorized by this acceptance.
