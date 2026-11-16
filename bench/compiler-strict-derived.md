# One strict historical replay after generated-source provenance repair

The separately released replay of original case **3** passed: **25 structural
operations covered all five changed-language files**, and the desired and
published selected-root programs have equal canonical checked meaning.
Both frozen proof and live archive auditors passed.

This adds one distinct revision to the previously audited 29 at the named
history horizon `6a187b94fb763f7b0565436a98ab5feda3cd642f`: the union is
**30**. The original six-case campaign remains **0/6** and the explicit-source
campaign remains **5/6**. Their observations, refusals and producers are not
rewritten. This is a new one-case observation under the gated provenance repair,
not a retry within either earlier campaign or a claim about a moving current
last-300 window.

## Exact case, producer and command

| Record | Value |
| --- | --- |
| Original ordinal | 3 |
| Revision | `0a410badb7e96ed9f6cf185df7d1f781d71c5fdb` |
| First parent | `579cf55e88177b9c9a4c59e2ff6c1bf7829e2878` |
| Entry, both sides | `src/main.lkj` |
| Library, both sides | Exact physical historical library, unchanged within the pair |
| Producer/source and gate | `62fa76f4f863a169b906094f0dd9998c8421e77a` |
| Executing producer SHA-256 | `34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377` |
| Executing producer bytes | 32,196,802 |
| Harness SHA-256 | `ed641768023b0b0fedfcc9b07a2d7116d8d18ecd01a9c0d89e2474ac2cbdd614` |
| Launch-ready manifest SHA-256 | `809feabb14573b3a66241ebf48b2fa7205f614de33d68125835055b05473811e` |

The [prepared plan](compiler-strict-derived.plan.md) and
[actual producer addendum](compiler-strict-derived.producer.md) were committed
before execution. The addendum binds the successful local gate, refreshed seed,
completed CI, and the completed current50 checkpoint continuation. Root then
independently checked the manifests and released exactly this command.
The release record is [retained separately](compiler-strict-derived.release.md).

The harness copied and rehashed the canonical producer, created one private key
outside all four explicit source/library grants, and used a fresh initially
empty 2 GiB store. Both entry arguments were relative and the actual physical cwd
was the desired tree. Before/desired source grants and both library grants were
canonical absolute paths; no implicit root-directory grant was used.
The exact NUL-delimited argv, cwd and grants are in the per-case records.

There was one compiler command, one worker, an external 4 GiB address-space
limit and the frozen 900-second deadline/grace. No source adaptation,
configuration switch, alternative root, filename projection, store reuse,
retry, replacement revision or recovery invocation occurred. The earlier
case3 had followed two other replays in a shared store; this new observation
started cold, so its time is not a paired performance comparison.

## Measured result

| Observation | Result |
| --- | ---: |
| Compiler exit | 0 |
| Applied operations | 25 |
| Required changed-language file facts | 5 / 5 |
| GNU wall | 67.18 s |
| User / system CPU | 64.94 / 0.58 s |
| Peak RSS | 1,258,692 KiB |
| Load before, 1 / 5 / 15 minutes | 1.39 / 1.77 / 2.03 |
| Load after, 1 / 5 / 15 minutes | 2.12 / 1.94 / 2.07 |
| Raw transaction stdout | 99 lines |
| Private process group after completion | Drained |
| Pending intent | Absent |

Raw GNU time, timestamp capture and loads are retained in
`compiler-strict-derived.tsv.logs/003-0a410badb7e96ed9f6cf185df7d1f781d71c5fdb.*`.
The outer gateway completed normally. No timing is inferred from partial output.

## Equality and complete language scope

The emitted replay proof reports both desired and published checked meaning as:

```text
2c178793d5b3aaefe27c4fce4dd36f90a69745e4f9b82878a5218a80eae70802
```

Its identity domain is `lkjscript-checked-meaning-1`.
The exact source snapshot before the transaction was
`4b0fa976d9dea035b01292771da50bd1153c2182a691a0de73e2783ad7d99838`;
after publication it was
`a98fe9d52544c223023264d7ee15f84a0ba321a7342d1582ba6d0a721fb44725`.

All changed paths under src/library and all changed .lkj/.rs paths anywhere
in the revision were covered. These are exactly the five required files:

| File | Before equals first-parent Git blob | Published bytes equal after proof | Exact desired text |
| --- | --- | --- | --- |
| src/ast.lkj | Yes | Yes | No |
| src/semantic.lkj | Yes | Yes | No |
| src/source.lkj | Yes | Yes | Yes |
| src/structure_edit.lkj | Yes | Yes | No |
| src/structure_replay.lkj | Yes | Yes | Yes |

Every full before/proof-after/published/desired SHA-256 is retained in the
per-case `.files.tsv`. Publication facts are unique and remain inside the
explicit before source root. The non-language shell-test addition in the
historical commit is retained in the full changed-path inventory, but it is
outside this language-effect claim.

The three textual differences are **four leading documentation-comment
lines**: one before `ast::LoadedModule`, one before
`semantic::ModuleSource`, and two before `structure_edit::Projection`.
The item-span projection omits those unattached leading comments.
The exact desired-versus-published [diff is retained](compiler-strict-derived.target-text.diff).
There is no other desired/published difference in those five source files.
Exact target-text equality was never the acceptance criterion.

This comment-only residual is an observed difference between the desired
historical tree and the actually published replay tree, backed by this equality
proof. It is not a comment-only parent-to-commit history change and does not
supply a rename-only historical example.

The claim is **complete changed-language coverage plus selected-root canonical
checked-meaning identity**. It is not raw serialized IR byte identity,
equivalence across all configurations/roots, reproduction of non-language Git
effects, or a claim that every historical commit can be replayed.

## Independent checks and named-window union

The committed proof auditor verified the producer/pin and source associations,
exact argv/cwd/key exclusion, timestamp/load records, one applied transaction,
canonical meaning equality, every required path and all publication digests.
Its output is [the proof table](compiler-strict-derived.proofs.tsv), with its
[full audit log](compiler-strict-derived.audit.log).

The separate live archive auditor checked **two archives and 3,695 file
instances**: 1,847 before files and 1,848 desired files. Every desired file/path
matched the historical Git tree. Every before file/path matched the parent
except the five explicitly proven published changes; no unexplained file
changed. Both archives had no pending intent. The differing archive file counts
include the historical non-language shell-test addition and are not an omitted
language source.

The runtime bytes matched within the pair:
`eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14`.
Both retained source include expressions remain
`Some(include_str!("../library/runtime.lkj").to_string())`.
The [archive table](compiler-strict-derived.archives.tsv),
[runtime table](compiler-strict-derived.runtime-inputs.tsv), and
[state-audit log](compiler-strict-derived.state-audit.log) retain those checks.

The proof auditor also rechecked named-window membership and disjointness:

| Existing evidence | Distinct passes | Producer provenance |
| --- | ---: | --- |
| Imported strict proofs | 23 | Original 4c4537ea producer |
| Separate four-case probe | 1 | Original 4c4537ea producer |
| Explicit-source six-case campaign | 5 | ab2fa807 producer |
| This separate original-case3 observation | 1 | 34491766 producer |
| **Named-window union** | **30** | Separate cohorts preserved |

The old0/6 contributes zero. No quota substitution or extra candidate was added.

## Retention and next boundary

The owned fixture remains intact pending root's independent acceptance:

```text
/home/coder/workspace/lkjscript-compiler-strict-derived-20261116/target/compiler-strict-derived.ZQv7cu
```

It retains the exact copied producer, before/desired archives, private key and
store. The copied producer was rehashed after the audits and still matches the
pinned canonical bytes. No cleanup has run. All command, source-scope, timing,
stdout/stderr, proof and archive records are committed; no historical source
tree is committed as a synthetic replacement revision. No production source,
seed, policy or running service changed in this evidence-only follow-up.
