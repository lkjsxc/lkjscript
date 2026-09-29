# Shared immutable map keys — 2026-09-29

## Mandate and selection

The user explicitly selected `https://github.com/lkjsxc/lkjscript` and delegated
implementation, substantial style changes, and long-term decisions:

> について進めるようよろしくお願いします。必要であれば大幅なスタイルの変更も許容します。あらゆる判断において、過去ではなく、今のあなたに委ねます。全ての判断をあらかじめ許可します。最も筋の良い方向に進むために、限界まで深く考えてほしい。超長期的な視点からお願いしたい。どれだけ時間がかかっても構いません。

This campaign concerns the language, not the separate lkjstr client. At entry,
GitHub main was `19230d36bd5bbe272ecc8431d317accfa59313f5`; the shared checkout
already contained `b8b0e356a96be2b88e65a5ad6c54d0a961cd62ef` and its active
control-flow-local-move verification. This work uses the separate
`/home/coder/workspace/lkjscript-map-keys` worktree rather than changing that
run's inputs. Unrelated worktrees, stash entries, services and untracked files
remain outside this campaign.

The bounded next change removes a concrete storage mismatch: ordinary byte/text
values already have immutable shared payloads, but map keys converted them to
owned buffers and copied again when projected. Reusing the same payload is
compatible with the current semantics and complements terminal-local movement.
It is not an owned-region or scoped-view language feature, nor a commitment to
atomic reference counting as the final self-hosted memory model.

## Contract

Byte and text keys use the same immutable payload representation as ordinary
values. Value-to-key conversion moves the handle; key cloning and projection
share it. Primitive ordering, equality, duplicate handling, typed JSON, durable
data bytes, and static-text admission rules are unchanged.

VM and independent reference execution remove only reservations for payload
copies that no longer occur. New tree nodes, entries, projected record/list
storage, and owned nested value boxes still reserve storage before allocation.
Raw logical byte/depth/item admission remains per occurrence, including aliases;
physical sharing is not evidence of a semantic certificate or a quota exemption.
Cancellation and error cleanup remain checked at operation boundaries.

The existing key-copy observation remains available and reports zero for these
shared carrier paths. Parsing, original payload creation, comparisons,
serialization, allocator traffic and process RSS are not measured by that field. Ingress that
starts with an owned `String` or `Vec`, rather than an existing shared payload,
may allocate and copy once to freeze it; shared reference counters also have a
cost. A workload that never reuses or projects its keys need not improve.
No wall-clock speedup, universal memory reduction, language extension, release
publication, or deployment is claimed by this change.

## Verification design

Independent pointer-identity tests retain original values and map versions across
conversion, cloning, replacement, projection and removal. The predecessor source
with these tests alone failed both tests at actual payload identity assertions
(`baseline-corrected.log`: 0 passed, 2 failed, exit 101). An earlier test harness
attempt called a private test helper and did not compile; `baseline.log` is
retained separately and is not counted as the behavioral negative control.

Additional VM/reference tests check pointer identity, equal-content independently
allocated keys, exact-fit and one-byte-short budgets, and the byte-accounting
slope for keys of 1 and 8,193 bytes. A key occurring as map input and lookup
argument must still contribute its logical bytes twice; an entries projection
must not charge a second key payload.

The literal `tests/fixtures/shared-map-keys.lkjc` consumer exercises lookup,
replacement, removal, ordered entry projection and retained versions for both
Text and Bytes through the copied executable. Its detached route has no original
project path or proposal source. Existing randomized map, hostile admission,
cancellation, raw-host cleanup and codec witnesses remain enabled.

Local logs and the authoritative full-check receipt are retained in the worktree
under `.artifacts/20260929-map-keys/` and `.artifacts/lkjscript-dev/check/`.
The first candidate map run passed 78 tests and failed the new admission-slope
harness because its fixture did not expose the selected `core.map.contains`
external signature. The harness now uses the maintained `core.map.get-or` with a
fixed scalar fallback; its independent logical-byte expectation is unchanged.
`map-tests-corrected.log` passed all 79 tests with no ignored cases. This includes
both VM/reference pointer witnesses, alias admission, exact-fit/one-byte-short
boundaries, existing map codecs, cancellation and retained-version regressions.
The earlier failing logs remain intact. The first public-consumer attempt
successfully produced its expected project result, then its harness incorrectly
looked for the foreground-only JSON observation field on a project execution.
The harness now checks the distinct existing protocols: project differential and
both flat key-copy observations, or foreground production JSON and cleanup.
No product response grammar was changed to accommodate the test.

The corrected public consumer passed all four Text/Bytes project/detached routes
(`public-consumer-corrected.log`, one test, no ignored cases). Workspace/all-target
Clippy also passed with warnings denied (`clippy.log`). These focused results
precede final source freezing and do not substitute for fresh full acceptance.
Final accepted-source and integration evidence is recorded after execution.
