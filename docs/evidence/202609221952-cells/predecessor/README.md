# Official v0.1.40 participation baseline

These are fresh execution observations from 2026-09-22, separate from the architect's read-only
investigation and from the new guarded library's acceptance.

The anonymously acquired official [v0.1.40 release](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.40)
uses source `1cdaf72888a1f46359a6d38956050747335f8e32` and target
`x86_64-unknown-linux-musl`. The archive SHA-256
`87b340a1eca452309a930d332379ba69ce49ea5922264371044a2abb712dfa06` matches the public
`SHA256SUMS` and the separately observed release-service metadata. The extracted executable
SHA-256 `bfe2f8bf643930db1d3d37d73093391d332d2fdce0d8814d3fa0ca463e53bdd7` matches
`RELEASE-MANIFEST.json`; its capabilities report version 0.1.40 and digest
`38a65237a10587799c2e2b8006597829be9af6ef34503f0300ccecde63ef5b24`.

The executable was copied outside the compiler checkout to
`/home/coder/workspace/lkjscript-cell-baseline-20260922/lkjscript`. Every product invocation cleared
`GH_TOKEN`, `GITHUB_TOKEN`, `GH_ENTERPRISE_TOKEN` and `GITHUB_ENTERPRISE_TOKEN`. Public
capabilities discovery, literal requests, plans, apply/check/build/export outputs, deployments,
original rejected inputs, and per-invocation stdout/stderr remain under that owned directory.
The official archive and content manifest remain under
`.artifacts/composable-20260922/predecessor`. These are local retained resources with no configured
expiry; they do not imply indefinite remote artifact retention. No credential settings were changed.

The producer request preserves the maintained old `try-update` body unchanged from main
`868e55c73c7f14fbbb2387f39c67290ed0122cd1`. It was authored through public plan/apply,
exported and staged separately into a fresh consumer. The consumer adds an ordinary generic
`raw-adjust<R>` helper and a separately authored `read-credit` observer. Host commands only
launch the product, construct deployment configuration and observe bytes; the graph performs
every cell update. Authoring project directories were moved before detached artifact execution.

The literal inputs and compact authentic fixture live at
[`tests/fixtures/cell-participation-predecessor`](../../../../tests/fixtures/cell-participation-predecessor/README.md).
The artifact includes all three exact package closures and has identity
`artifact_bundle_4659d83df8f33b7eacd596328fccfdf6ab1893ad399c2e82582d1aca37ed2ea2`;
its SHA-256 is `ededb903b72c250405fccbc93db843d6a2217f11a2e35fc09b3b24c9a24b09e4`.
Source/transport identities and original descriptor are recorded once in the fixture's provenance.
The official runtime executable is not tracked in the fixture.

| Fixed expectation before execution | Fresh observation |
| --- | --- |
| An initialized empty store reads `[]`. | Separate public `read-credit` invocation returned `[]`. |
| Calling the old standalone `try-update` inside the same canonical owner rejects. | Exit 6, infrastructure `normalized_transaction_nested`: `one exact requirement cannot begin a nested transaction`. |
| Rejected nesting publishes no data. | Raw HEAD and object inventory remained byte-identical; a new process still read `[]`. |
| Two raw helper calls under one ancestor owner compose: fallback 100, -30, then -10. | Returned `Committed(60)`; five capability calls and maximum one live transaction. |
| The composed result persists once. | A new process read `[60]`; public `data verify` reported two revisions including initialization, two objects, one record, zero schemas and zero staging leftovers. |

This confirms that raw helper participation and multi-operation atomic transactions were already
possible. The selected work introduces the explicit guard and supported ordinary library boundary;
it does not claim to invent raw participation.

The baseline also exposed two authentic v0.1.40 native-authoring failures. A complete native
consumer using the advertised `(transaction-outcome ... (types I64) ...)` form rejects with
`change_unit_unresolved`, naming `types` as an unresolved declaration. Canonically drafting the
already accepted old producer fails with `change_block_expression` at `<native-draft>`. Both inputs
and diagnostics are preserved. The baseline continued through public `expression.block` owners
mixed with native ordinary helper declarations; no accepted storage mutation or semantic generator
was used. These product defects require repair for the campaign's native public workflow.

The maintained `requirements_participation_predecessor.rs` reader now stages the authentic old
transport and runs the exact old artifact with fresh owned stores. Its independent snapshot
observer compares full frozen I64 encoding bytes and HEAD; its receipt reader requires the exact
invocations, descriptors, classified rejection and physical revision count. Implementation of this
reader is complete, but execution against the new runtime and the opposite old-runtime/new-guard
preflight boundary are not yet observed at this writing. Their results must be appended below.

## Execution addition — old-runtime rejection of the new guard

The opposite compatibility boundary is now freshly observed. A copied intermediate campaign
executable, SHA-256 `4c822b8541d807e664f3734c73118371e783991c461fba4d1c7b59beb0630215`,
authored a tiny native caller against explicit standard package revision
`package_revision_3b195535785762a7dcdec5df40080cd51e1abbb592bd2edfd2c26a05bcdd0a92`.
Public plan/apply/check/build produced
`artifact_bundle_fa1f2ff01c98c9a72b423192010956d9b7770921eda4c7d73f7c605500e9fb11`.
The caller requires `get`, `put`, `require-transaction` and invokes the ordinary standard participant
with a callback that would divide by zero. The authoring executable is an intermediate source
build, not the final candidate, and this observation is not candidate acceptance.

With `LKJSCRIPT_PARTICIPATION_UNAVAILABLE_SECRET` unset and the configured data root absent,
the authenticated official v0.1.40 executable returned exit 3, class `capability`, code
`normalized_data_operation`, message `first-party data adapter does not implement exact operation
'require-transaction'`. The absent root remained absent. This is the unsupported operation's
preflight rejection, before unavailable secret acquisition or live store opening. It is not a
failure to decode a newer graph/compiler/artifact encoding.

A bounded intermediate-runtime invocation of the same artifact, with no secret and a separately
initialized disposable store, instead returned `normalized_data_transaction_required` (exit 3)
and retained byte-identical HEAD. The trapping callback did not execute. This was one source-only
boundary probe; the maintained composition owner supplies the complete candidate behavior proof.

The fixture now includes the exact new artifact, literal request, original deployment, authentic
old-runtime stdout/exit and separate provenance. The maintained reader independently admits the
artifact identity and requires the original classified unsupported-operation rejection. It reuses
this retained historical runtime observation rather than making acceptance depend on network
acquisition of a v0.1.40 executable. New-runtime execution of the old exact fixture remains
pending the focused owner run, as do the reader's newly added adversarial tests at this point.

## Execution addition — new-runtime compatibility admitted

The subsequent focused owner run 02 freshly executed this exact old artifact and transport with
Product 02, including nested-owner rejection, raw-helper `Committed(60)`, separate-process read,
unchanged rejection HEAD and exactly one successful physical completion. The retained official
old-runtime/new-guard rejection was independently admitted by the current reader. The complete
229-command run and its original reader passed; its identity and limits are recorded at the
[parent evidence owner](../README.md). The predecessor reader's normal preflight substitution
test and ignored original-receipt mutation test also passed. These additions close the pending
focused observations above; final-candidate and publication proof remain separate.
