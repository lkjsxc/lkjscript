# Cleanup of completed copy-site integration work

Root has reviewed the exact [six-scope preparation](compiler-copy-sites-root-cleanup-data/README.md)
and deletion guard `3001e0e524c6694e56c1ad27cdc162a381493782670ce3a804c92522dbded5e7`.
This records acceptance of the retained/disposable partition; execution is still
pending a separate root release naming the committed bundle.

The six literal scopes contain 21,076 regular files and 112 directories,
779,287,389 logical bytes. Exact existing Git/archive/seed recovery covers
2,384 files / 92,144,809 B. All 71 remaining non-product gaps, 259,435 B,
are retained in a verified 40,221 B archive. The other 18,621 files /
686,883,145 B are explicitly disposable generated executables, IR, test
sources, test products and completed-gate store contents.

The disposable executable count includes the two ordinary parity images and
the role image; their bytes are not archived. All 114 current gate logs are
retained. Four of the 80 HTTP products are explicitly named by retained outcome
logs; the other 76 have the producing test contract but no individually claimed
success transcript. Deleting these products does not create a recovery or
independent performance proof.

## Root review and consumer closure

Root read the complete 16,497 B guard and the exact plan. Its literal scope
checks, exact committed manifest, complete recovery/disposal partition,
no-follow regular-file identity/hash checks, parent identities, enumerated
unlink/rmdir operations and final ENOENT checks match the proposed action.
There is no recursive removal or fallback. A partial deletion failure is
retained and stops; this is not an atomic tree transaction or a guarantee
against a hostile concurrent parent rename.

All known consumers of these six scopes have finished. Cohort 2 completed
and its complete evidence and independent acceptance are committed. The later
census uses separate held inputs, outside these scopes. The capture wrapper
has been copied exactly into committed bench tooling. Old role validation
inputs, operation proposals, gate captures and original failed candidates have
specific retained counterparts or the new archive.

The standard `target/lkjscript` remains outside the scope and was independently
hashed as `6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc`
before release. The same canonical bytes inside the cleanup scopes recover
exactly from seed commit 7d0731e9. Global keys, operation credentials, every
other target scope, worktrees, services/data and generation one are excluded.

Root copied and compared all 40 bundle files, including its 39-record manifest,
byte-for-byte: 19,995,938 retained bytes. The manifest SHA-256 is
`ca8ee6113e3fb5a7e11bdeca3deef7f68b88e97023e1bffd4f53587100c7b9c1`.
The future one-shot invocation must preserve its exit, stdout/stderr, UTC,
GNU timing and load outside the six removed scopes.
