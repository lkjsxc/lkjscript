# Independent acceptance of generated-source provenance

The repair at `f6e5ea361dd9051da026ca0bf2bae71b2d874563` is integrated
as `68824804`. Its frozen report SHA-256 is
`eef02aebc5d142e00f36fd7ff16357e9368e31da8ea4b3939a8ee07d3b314f2c`;
its 437-record manifest is
`01b52d89523885650f9467f648a14ab655d4f27c526022c7d6fe767fdfa74346`.
The isolated final candidate is not the integrated canonical compiler or seed.

Root read the origin classifier, catalog projection, fact labels, edit guards,
replay exclusion and gate regression. An independent reviewer also checked these
paths and standalone test-root compatibility. The review found and corrected
two concrete gaps before acceptance: exact generated implementation ownership,
and canonical helper/trait identity comparisons. The latter resolve fresh
`std::default` paths in the recorded scope before comparing declarations, so
a malformed pre-filled declaration does not bypass resolution and valid
re-export aliases remain accepted. Both redirection controls are in the suite.
New provenance and generated-reference answers remain source claims; checked
authority, effects and layouts still come from checked IR.

## Operation and retained-line audit

Gateway operation `compiler-v1041-origin-independent-operation-audit`
independently reran the retained auditor against the still-present repair tree.
It verified the initial parent Git blobs, both unchanged refusals, every
before/after chain, all published file facts, the exact new module projection,
and final source bytes. Its output is byte-identical to the original audit:

```
609c6415b136cfaa6be5bb4e7c4690fe99f1de2c5acfc9976484b033a2982d4f
```

The accepted chain has 26 applied operations and eleven file facts over five
production files. Root independently checked that the changed source inventory
contains exactly those five files. The audit took 0.12 s wall / 0.11 s user /
0.00 s kernel, peak RSS 9,100 KiB, at load 5.51 / 5.71 / 5.44 before and after
(1/5/15 minutes). It ran under 4 GiB address space and a 900-second deadline;
no compiler or test was invoked during this audit.

Root's retained-line table also matches the integrated commit exactly:
484 added/deleted production `.lkj` lines, all covered by the operation chain;
438 additional lines in twelve retained original/contract specimens, uncredited.
This contributes 484/922 to the all-retained-language-lines metric, and 484/484
to the `src/` and `library/` subset. The cumulative lower bounds become
8,074/10,690 (75.5%) overall and 8,072/9,930 (81.3%) in that production subset.
No copied specimen was renamed or dropped to change the denominator.

## Frozen-manifest interpretation after integration

All 437 frozen manifest entries verified in the original clean repair tree.
The integrated source and test hashes remain exact. Git also merged the new
origin contract with main's already accepted v3 container contract, so
`docs/structure.md` intentionally differs from the repair branch's earlier
v2-based document.

To keep that frozen evidence independently reproducible, the exact branch
contract is retained as
[root-acceptance/branch-structure.txt](compiler-generated-source-data/root-acceptance/branch-structure.txt),
SHA-256 `93c0893fb6d81b71eba6bcef209dc813da825242751bbe9dc742e3159800692f`.
Root's integration audit verifies 436 entries directly at their integrated
paths and that one entry against this preserved branch contract. The original
manifest remains unchanged. The integrated contract is
`71d90b4f7dbcb0085a4d3da1caf4e2e35a11bd16d8104e88734b12764d441541`.

The root audit output, timings, loads, inventory, retained-line table and mapping
confirmation are kept in [root-acceptance](compiler-generated-source-data/root-acceptance/).
Their separate manifest covers this acceptance note and those retained files.

## Validation boundary and next step

The final focused suite passed in 20.14 s (load 3.46→3.72), including generated
item/call refusals, actual regeneration, shared authored type/bound references
and malformed-origin controls. The compiler's own fresh declaration-only no-op
dry-run passed in 41.50 s (3.72→3.31), with 12,757 fresh checked instances,
identical checked meaning/source bytes and no authority delta. All development
failures and the subsequent existing-query-script launch failure (126, zero
cases) remain recorded. The existing query/replay suites will run through the
combined gate; no pre-gate repetition is claimed.

The strict historical count remains 29. Neither the original 0/6 campaign nor
the scoped 5/6 campaign was rerun or rewritten by this repair. A new historical
case needs a distinct frozen producer and campaign. The combined source gate,
canonical seed refresh and both CI jobs remain pending at this acceptance point.
