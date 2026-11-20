# Copy-site integration cleanup preparation

**Prepared only. None of the six scopes has been deleted, and no cleanup invocation has been released.** The inventory and recovery mappings are pinned to main `416da03c86b6073832c9b2cdbb319ff8a3b8c991`. The source archive baseline is `27c94440646d787efc4e7b43a712405288c22c61`; the completed gate's source is `d461c2d98e93aeed6061508ee80ab36b9eeaa04f`.

This bundle proposes one enumerated cleanup after root reviews and commits the exact bundle, confirms that its consumers have finished, and explicitly releases the one invocation. It contains no new compiler measurement, source change, process/FD/syscall observer, private-key read, or cleanup result.

## Exact scope

All paths below are beneath `/home/coder/workspace/lkjscript/target`. Every directory and regular file is listed in [full-inventory.tsv](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-copy-sites-root-cleanup-data/full-inventory.tsv); each regular file has SHA-256 and Git blob identity, plus device, inode, ownership, mode, link count, size, block count, mtime and ctime.

| Scope | Files | Directories | Logical bytes | Allocated bytes |
|---|---:|---:|---:|---:|
| compiler-copy-sites-integrated | 1,937 | 55 | 11,191,303 | 17,301,504 |
| compiler-copy-sites-driver-candidate | 4 | 1 | 154,506 | 167,936 |
| compiler-copy-sites-integration-evidence | 187 | 26 | 34,411,659 | 35,028,992 |
| compiler-copy-sites-gate.ccpzZ9 | 17,580 | 18 | 576,864,424 | 632,504,320 |
| compiler-copy-sites-evidence.20261116-1 | 12 | 1 | 7,010 | 49,152 |
| gate | 1,356 | 11 | 156,658,487 | 161,140,736 |
| **Total** | **21,076** | **112** | **779,287,389** | **846,192,640** |

There are no symlinks or special entries. Every regular file has one link. The completed hash pass checked each file's initial metadata before and after reading it. Read atime is deliberately not part of the guard. The inventory SHA-256 is `e50bebef5501856268d0cb9459b9dd4465246cf8d49273803c4076c7dc934736`.

The standard `target/lkjscript`, every other target scope, cohort 2 run/evidence, extractor/census proposals, all worktrees, shared parent directories, global store key, operation credentials, services/data and `.generation-one` are outside this cleanup. The two inventoried 6cd compiler copies remain present until explicit release.

## Exact recovery and new retention

`recovery.json` names the specific Git object or compressed archive member that recovers each of 2,384 file instances, totaling 92,144,809 bytes. Verification compared the recovered bytes' size and SHA-256 to the live inventory. Equality of a hash to a recorded observation alone is not called recovery.

The existing source/proposal archives recover the original failed candidates and most combined projection files. Two qualified combined projections still had no exact committed counterpart:

| Live relative path | Bytes | SHA-256 |
|---|---:|---|
| compiler-copy-sites-integrated/src/native/cache.lkj | 16,862 | 2d5a6636f17c5d2e4f980537f10462f395f6135b5a3781b07ac8c7af0d7ec3cc |
| compiler-copy-sites-integrated/src/native/profiling.lkj | 41,975 | a57da9cbc0627bdc83e416e4c2675c10e46270082e2cdfa4d3f65344c80af44e |

Root selected **all 71 remaining non-product gaps** for retention: those two projections, 62 logs, and seven other observations/metadata files. `retained-inputs.json` enumerates their exact paths, sizes and hashes. Their total is **259,435 bytes**.

The deterministic `retained-unmatched.tar.gz` contains exactly those 71 regular members, with sorted paths, fixed zero mtime/uid/gid, ustar framing and gzip without a timestamp. It is **40,221 bytes**, SHA-256 **56825fc0ed67a0e772d46404598c0b0dd2d3ff099d05734e977cda0582317a22**. Every archive member was read to stdout, hashed and compared to the original inventory; each live input was checked before and after retention. No archive member was extracted into a source or inventoried path.

The retained gaps include the original profile console, the prior retention check, source locator, gate owner/release metadata, older gate logs, and the small witness-selection and authority projections. Private credentials are not included. The profile console records a temporary credential *path* as part of the existing shell trace; no credential contents were read.

The existing `bench/compiler-copy-sites-data/roles.samples.gz` independently decompresses to the exact 28,522-byte role sample, SHA-256 `1f0912222954ccfc998e2ee27f8164c0e631381701b6ae1b04d1ae43907a27a1`. The compressed object is 5,394 bytes with SHA-256 `9d0109a7e0db1d1c27f74f0f77c13fbe20b36ee0e3b7dc6ee1b2005347ca673b`. Its existing companion and observation records remain committed.

### Canonical recovery is exact and separate from disposal

Git object `7d0731e99aeaf241c4334fc5ff065a341fb388c8:seed/lkjscript.gz` is 6,547,345 bytes with SHA-256 `505be947525d53b4db1bff6c85cb77622cb2f5dc7e7d7fe92b772ee3b8ffc3e2`. It decompresses to exactly 32,584,542 bytes, SHA-256 `6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc`.

That exact byte sequence matches both `compiler-copy-sites-integration-evidence/compiler` and `compiler-copy-sites-gate.ccpzZ9/canonical`; the two gate seed gzip files also match the committed compressed bytes. No compiler image was executed to establish this recovery. Both live compiler copies remain untouched until root explicitly releases their removal. The ordinary canonical path `target/lkjscript` is outside the literal scopes.

## Explicit disposable products

The remaining **18,621 files / 686,883,145 bytes** are enumerated in [disposable.json](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-copy-sites-root-cleanup-data/disposable.json), not inferred again at cleanup time. Their hashes identify exactly what would be discarded; they do not make these bytes recoverable.

| Product class | Files | Bytes |
|---|---:|---:|
| Isolated completed-gate store | 16,057 | 321,266,634 |
| Generated executables | 32 | 255,925,355 |
| Generated checked-IR files | 394 | 84,222,911 |
| Generated language test sources | 2,040 | 2,594,546 |
| Generated authority projections | 14 | 5,078,655 |
| Generated Rust witness sources | 4 | 39,534 |
| HTTP gate sample/memory products | 80 | 17,755,510 |

The 32 generated executables include three focused integration test images: `ordinary-old` and `ordinary-new` (46,490 bytes each, both `abcba417…0cffd`), and `roles` (136,687 bytes, `b6d71323…15506`). They are explicitly proposed disposable test products, with current byte identities and retained parity/role observations; their executable bytes have no archive-recovery claim. Root's final exact plan review must retain that distinction.

### HTTP outcomes and limits

All **114 current top-level gate logs** have exact committed counterparts. `test-product-contracts.json` binds those files and the source contracts that generate `target/gate/generated` and `http-profiles/http.{memory,samples}.PID`.

The current HTTP pair ending `02777731` is named in the retained copy-sites gate `details-http.log`; that log records 29 functional cases, 12 persistent/pipelined requests, and native/interpreter HTTP-get outcomes. The prior origin/container gate similarly names the pair ending `02606246`. Thus four of the 80 accumulated HTTP files have direct filename bindings to these retained outcome logs.

The other 76 older accumulated products have the same explicit test-output contract, but this bundle does not claim an individually retained success transcript for every old PID. They are proposed discarded as transient gate products. Their raw bytes are not recoverable from hashes.

These HTTP files are separate from the copy-site hot regression and the compiler copy-site measurement. The existing report expressly says its transient hot sample, companion, cache-stat and error files were removed by that test's normal cleanup. This inventory adds no hot census or fresh independent recomputation claim.

## Guard and release boundary

`guard.pl` is an adaptation of the established legacy cleanup approach. It has not been invoked. Root review of this exact adaptation is pending; an earlier review of the legacy guard does not count as review of this file.

After explicit release, the only proposed form is:

```text
perl bench/compiler-copy-sites-root-cleanup-data/guard.pl --execute-after-root-release EXACT_COMMITTED_BUNDLE_REVISION
```

The guard requires that revision in main history, verifies every committed manifest entry and its own exact bytes, checks the pinned inventory and complete recovery/disposal partition, reconstructs every retained counterpart, and independently verifies the new archive. It then enumerates exactly the six literal scopes and rejects changed metadata/content, missing or extra entries, aliases, symlinks, special entries, or extra file links.

Before every unlink it rechecks the parent identities, no-follow file descriptor identity, whole file hash and metadata. Only enumerated regular files are unlinked; only enumerated directories are removed with `rmdir`. The final six absences must be `ENOENT`. There is no recursive deletion helper, fallback, process observer or credential input.

The guard writes a fresh result directory outside the six scopes and refuses to repeat if that directory already exists. A refusal, including a partial failure during unlinking, must be retained and returned to root; the plan cannot be broadened or retried through another route. The future wrapper must retain UTC endpoints, GNU elapsed time, before/after load, stdout, stderr and exit status outside the deleted scopes.

These are cooperative ownership checks, not an atomic defense against a hostile concurrent parent rename. Root must confirm known consumers are finished before release. No cleanup is authorized by merely preparing or committing this bundle.
