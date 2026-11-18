# Replace-call integration on the corrected public parent

The integrated candidate passed the reserved local gate and standalone
real-program validation. The [validation receipt](validation.json) binds the
exact input, approved seed, reproduced compiler, original witness and raw logs.
Final exact candidate review and the external next action belong to lead.

## Frozen identities

| Item | Exact identity |
| --- | --- |
| Verified public parent | `e414aeddbb2ec6985fd564c96a44a4f583b6ba1b` |
| Reviewed compiler source | `ecd0ff6d0e2210be5b7eaed9770b675b7c070554` |
| Original code/evidence branch | `7f87d6d7b73365a6d9976c367823d48343a6b347` |
| History-preserving merge | `db299790d539f75f5bc3cde25399628fd4952737` |
| Separate seed-only commit | `20935b4c66f87e9e52a68ce79bf5e60603b75198` |
| Exact clean gate input | `b38830634201d4a8974c6a2b7afc05bc76b3927a` |
| Compiler / decoded-seed SHA-256 | `103557eddf30d24e046cb09f6a881e549829283495c8648c0ac9dcd1d72a5d35` |
| Seed gzip SHA-256 | `65408daa78849909a0a3ddcf0a2cab29120b9ddfc095ffb2095c422261f6a6ee` |

The gzip is 8,204,839 bytes; its decoded compiler is 40,989,463 bytes. Both
approved hashes were checked before applying the original proposal, and its
decoded bytes compared exactly with the reviewed producer. The new bootstrap
again reproduced those bytes and matches the installed seed byte-for-byte.

## Scope and preserved histories

The merge completed automatically without conflicts. The public parent and
original candidate are both ancestors, permitting a fast-forward from public
main. Production changes relative to public main remain exactly
`src/driver/help.lkj`, `src/structure_edit.lkj` and `src/syntax.lkj`.
The complete production patch still has SHA-256
`9c90f2ed55346a356a919a1cedcaa6195524ddf8ba5867647aeb4f13b9b43bac`,
identical to the [reviewed patch](../landing/production-source.patch.gz).

Compiler and library source bytes remain identical to the reviewed source.
The public macro-origin harness is preserved exactly, including phase diagnostics,
cleanup behavior and the 1,800-second watchdog. Relative to public main,
`tools/check.sh` adds only the reviewed call-suite registration. The public
Programs status section is unchanged. Type defaults and namespace repairs are
outside this integration.

The [complete non-seed input diff](complete-nonseed-input.patch.gz),
[input path list](input-delta-paths.txt), [input tree manifest](input-manifest.txt)
and [source hashes](current-source.sha256) retain the review boundary. The seed
binary is independently bound to the approved identities above.

## Completed validation

| Check | Result |
| --- | --- |
| Full four-worker gate | Pass, **1,196.524 s**, exit 0 |
| Whole-case requests | **6,575 rerun; 0 reused** |
| Ordinary conformance | 1,357 cases; 2,712 route checks; 2 pending route entries |
| Optimized conformance | 1,357 cases; 2,712 route checks; 2 pending route entries |
| IR mutation | 128 mutants; 104 rejected, 22 values, 1 trap, 1 nontermination; 0 resource outcomes; **0 findings** |
| Standalone real programs | Pass, **3.45 s**, exit 0 |
| Bootstrap / installed seed | Exact byte equality |
| Prior Rust witness binding | Same 2,508 relevant inputs and identical compiler bytes |

The [gate](full-gate.log) ran from 2026-11-18 17:45:24 UTC to 18:05:21 UTC.
The store was initially absent, and the exact tracked/untracked checkout was
clean before the gate, after it, and after the standalone run. The owned
[wrapper](run-serial-gate.sh) acquired the shared mkdir lock and released it through
its EXIT trap after standalone validation ended at 18:05:24 UTC. No competing
full gate was started by this stream.

Zero request reuse means no complete test request was skipped. Ordinary
authenticated instance/object reuse inside commands remained enabled. The
[GNU time receipt](gate.time) reports 1,196.52 s wall, 3,082.49 s user, 82.94 s
system, and 1,566,576 KiB maximum RSS; that maximum is not the sum of simultaneous
worker memory. The gate's own timer reports 1,196.524 s. This is a single
shared-host observation, not a performance-comparison claim.

The two pending entries in each conformance mode are the native and interpreter
routes of `tests/conformance/fail/borrowed_closure_escape.lkj`. Callable
`impl Trait` results are not yet elaborated; escaping a borrowed capture must
eventually be rejected. This integration does not claim that case is completed.

The [standalone receipt](standalone.log) retains the original upstream source
hashes, 18 provenance controls, eight tests per engine, pure algorithm authority,
and all 7,239 native corpus rows. Its [time receipt](standalone.time) reports
3.45 s wall, 2.86 s user, 0.44 s system and 63,252 KiB maximum RSS.

## Exact historical Rust witness binding

The preserved [original witness](../landing/witness.log) passed 499 values/output
cases and 652 verdicts, including the independent compiler-byte comparison.
It was not rerun for this integration. Both the reviewed and integrated
[witness input manifests](witness-inputs.txt) are byte-identical, with SHA-256
`a554d8b81b0acd9840efa481dd060589a1f36dc1c6ff04fbad320b390b52c17d`.

The [path list](witness-input-paths.txt) covers all compiler, library, witness,
test and example files, directly invoked host scripts, their C fixtures and the
Levenshtein provenance verifier. Neither changed gate-only harness is invoked
by `witness/check.sh`. The retained Rust-host compiler still hashes to
`fbebd08a32c9823642625e240164c2590ce4fdb443b8f886230b963ab36f4ac6`;
its previously emitted native bytes match the new bootstrap exactly. The
[original toolchain](../landing/toolchain.txt) and
[time record](../landing/witness.time) retain that older run's context; the binding is not a new timing measurement.

## Evidence and publication

All 111 [original packet checksums](original-checksums.log) and 15
[reviewer checksums](reviewer-checksums.log) pass. The historical landing packet,
original full gate and unfamiliar-agent archives remain unchanged. The archive
review verified all 59 allowlisted regular members across three archives; no
private keys, stores, configurations or compiler binaries are included there.

The [public-parent CI receipt](public-parent-ci.json) independently verifies
run 44352321149 on exact `e414aeddb`: both gate and witness jobs succeeded.
There is no CI result for this local integrated candidate because it has not
been pushed. Lead receives the final exact candidate and receipt for review
before owning the external next action.

Check this packet from its directory with `sha256sum --check SHA256SUMS`.
From the repository root, the following command binds the current report,
status, contract text and progress file to this evidence commit:

```sh
sha256sum --check bench/compiler-call-data/integration/report-files.sha256
```
