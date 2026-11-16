# Owned native-worktree temporary closure proposal

**Proposal only; none of these owned scopes has been deleted.** This is one closure bundle, with no new deletion guard or campaign.

The released main-target V2 cleanup completed successfully: **21,076 files / 112 directories / 779,287,389 logical bytes**, exit 0, **8.17 s**, load **0.96 / 1.57 / 1.95 → 0.97 / 1.55 / 1.94**. All six main-target absences were independently verified as ENOENT. Standard `target/lkjscript` remains 32,584,542 bytes, SHA-256 `6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc`. All 39 original committed retention records verify. The original compilation failure remains a separate preserved attempt.

## Remaining owned scope

The six paths below are relative to `/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116/target`.

| Scope | Files | Directories | Logical bytes |
|---|---:|---:|---:|
| compiler-copy-sites-cleanup-20261116 | 16 | 1 | 18,351 |
| compiler-legacy-root-inventory-20261116 | 106 | 10 | 16,931,625 |
| compiler-ci-43878716520-20261116 | 5 | 1 | 2,456 |
| compiler-copy-sites-root-inventory-20261116 | 131 | 7 | 61,577,567 |
| compiler-ci-964be9a4-20261116 | 6 | 1 | 736 |
| compiler-copy-sites-cleanup-guard-v2-20261116 | 19 | 1 | 46,031 |
| **Total** | **283** | **21** | **78,576,766** |

Allocated space at inventory was 79,339,520 bytes. No symlink, special entry, credential-like basename or multiple regular-file link was found. The inventory read only these owned paths and selected committed retention objects. It used no process observer, private key, compiler, image, or network query.

`inventory.tsv` records the exact paths, metadata and hashes. `disposition.tsv` binds each file to its disposition and, where available, the exact verified committed counterpart at `99ac7a6043cd9ee411970e55970634a34506a709`.

## Retain before closure

- **227 file instances / 38,720,533 bytes** already have exact committed counterparts. Those include the original proposal cleanup, legacy retention/outcome records, unchanged original root-cleanup bundle, first failure, and V2 preparation.
- The new completed V2 cleanup has **17 unmatched file instances / 20,389 bytes**; its remaining records are exact existing duplicates. This bundle retains **all 24 raw manifest records plus their original manifest**, preserving context and the complete successful result.
- The two historical CI reads have **7 unmatched files / 3,188 bytes**. This bundle retains **all 11 original CI capture files**, including their status/empty files and manifests. These are historical observations, not fresh status queries.

The copied retention totals **36 records / 188,818 bytes**. Every copy was compared to its source digest; the complete mapping is `new-retention.json`. Successful cleanup evidence is under `completed-cleanup-v2/`, and the two old CI snapshots are under `ci/`.

## Proposed disposable intermediates

The remaining **32 files / 39,832,656 bytes** are preparatory mapping/census intermediates: initial metadata tables, Git tree listings, intermediate match/unmatched tables, archive member indexes, and small inventory/packaging helpers. Their final checked inventories, guards, recovery mappings, retained inputs and raw outcomes are preserved in committed bundles or this proposed retention bundle.

Their exact names and hashes remain in `disposition.tsv` and `summary.json`. The proposal is to discard those intermediate copies after root accepts the retained final evidence. It does not claim that their hashes make their exact bytes recoverable.

## One later closure

After root commits this bundle and accepts its dispositions, the recommendation is one separately released cleanup of the six inventoried scopes. The new `compiler-native-temp-closure-20261116` proposal directory can also be closed once its exact committed copy is verified; it is not included in the six-scope census above.

The worktree, its source/Git metadata, the shared target parent, main's standard canonical, every other worktree/scope, credentials and services/data remain outside this proposal. No earlier release authorizes this later closure. No further deletion has occurred.
