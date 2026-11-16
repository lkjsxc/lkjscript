# Semantic container v3: implementation and focused validation

The semantic index now publishes one authenticated container per snapshot.
Queries authenticate a fixed front and only the rooted extents they request.
Ordinary Store reconciliation charges and checks the full physical container
through its authenticated front plus exact EOF, so the indexed-body bookkeeping
no longer needs a separate cache entry for every radix node.

This report covers implementation and focused validation before rung timing.
It does not claim a measured speed or memory improvement. The frozen experiment
is compiler-query-container-plan.md, with executable identities and controls in
compiler-query-container-identities.md. Its 10k/100k sequence remains a distinct
experiment; the tiny helper fixture below is not a rung.

## Representation and trust boundary

The new format is v3-only. A snapshot's canonical file starts with a 90-byte
authenticated envelope containing a fixed 400-byte manifest, followed by
individually authenticated node envelopes in deterministic postorder.
Each child descriptor contains its payload digest, byte offset and envelope
length. The root ends exactly at authenticated physical EOF.

Every requested extent must start after the 490-byte front, fit within signed
nonnegative i64 offsets, have a 32,768-byte-or-smaller payload and end before its
parent. All descriptors in a parsed branch are validated, including unselected
children. Complete descriptor validation precedes raw-node cache reuse.
The private Reader snapshot fixes the producer/root/selector lifetime.
The existing logical-page cache retains only fully validated success or
authenticated absence, never failures.

The Store layer owns transport manifests, extent verification and the private
stream writer. Semantic fact production, IR admission, the checker and native
emission are unchanged. One verified payload covers one manifest or node;
reading the front is never called whole-container authentication.
Existing source-claim and checked-IR labels and query stdout facts are unchanged.
Protocol version, rooted offsets, snapshot IDs and authenticated cursors change
because their framing changes.

Streaming owns a fresh private temporary Directory/File, reserves capacity
before the front and every append, backpatches only that newly created file,
and renames between held directory handles. Capacity includes the full proposed
stream, its accounting row and the larger old/prospective signed index.
The writer cannot interleave arbitrary Store puts while it owns the exclusive
Store borrow. Failed-eviction poison now prevents both ordinary publication and
streamed append from continuing after a failed accounting mutation.

Published container size is charged in full. Queries bound each allocated
extent independently and may read a container larger than their default
256 MiB allocation cap. A held readonly file is not kernel sealed: bytes can
be changed through another handle, and already owned authenticated bytes can
remain valid for the fixed pin. No filesystem-freshness claim is added.

Canonical old semantic page/root/latest/locator families are retired under the
existing writer lock. The old locator remains recognizable by Budget decoding
for accounting and retirement only, preserving unrelated entries in a valid
mixed ledger. No v2 reader fallback or trusted migration marker exists.
The initial legacy scan remains proportional to the old entry count; steady
state still scans all retained containers, ordinary entries and disposable
debris. Old writers sharing a v3 store are unsupported.

Final snapshot retention is checked again after the driver's later
Memo.remember writes, so a capacity eviction cannot be hidden by printing a
snapshot that was retained only before those writes.

## Source operations and independently checked provenance

All 802 retained production .lkj lines are operation-backed:

| File | Retained added + deleted lines |
| --- | ---: |
| src/store.lkj | 671 |
| src/semantic_store.lkj | 117 |
| src/semantic_query.lkj | 8 |
| src/driver.lkj | 6 |
| **Production total** | **802** |

The standalone src/query_store_cases.lkj migration contributes another
143 retained lines (109 added, 34 deleted), also through checked operations.
It required a separate archived root because the old standalone root could not
compile against the already changed Store API. The archived root's selected
six source files and all 32 library files were independently checked against
the production dependency closure. Only the exact checked case-file bytes
were transferred; the repeated Store dependency edits are not double-counted.

Main source used 62 successful requested operations. The separate case root
used five case operations, then one case-only correction. Its 47 repeated
Store operations validate that root and earn no additional production-line
credit. The final retained src .lkj total is 945/945 lines. Generated specimens
inside evidence, shell/Perl helpers and reports are outside this denominator.

| Request | Result | Wall s | Peak KiB | Load before → after |
| --- | --- | ---: | ---: | --- |
| Main request 1, static review | Unapplied | — | — | — |
| Main request 2, 61 operations | Refused; complete source inventory unchanged | 41.18 | 1,078,080 | 4.28 → 3.95 |
| Main request 3, 61 operations | Applied and fresh-checked | 51.27 | 1,129,104 | 9.08 → 11.69 |
| Main request 4, cursor decoder | Applied and fresh-checked | 50.67 | 1,151,944 | 13.02 → 12.50 |
| Standalone request 1 | Refused; inventory unchanged | 2.47 | 98,784 | 8.49 → 8.49 |
| Standalone request 2 | Applied, then exact case-file transfer | 2.13 | 106,848 | 8.08 → 7.83 |
| Standalone request 3, historical cursor fixture | Applied; only case file changed | 2.37 | 126,588 | 9.44 → 9.32 |

The main request-2 refusal was due to two ordinary borrow/type-elaboration
forms; request 3 used explicit scalar/boolean temporaries. One earlier setup
attempt stopped before invoking the compiler because a raw digest file was
mistaken for sha256sum input framing. Both refusals remain in the evidence.
The applied authority delta includes the existing file-seek effect; no runtime
primitive was added.

The independent main audit verifies all 1,206 original Git paths, the unchanged
refusal inventory, the complete after3→before4 chain, all four final production
hashes and all five resulting file proofs. The standalone audit reconstructs
the exact case text from the successful requests and checks the transfer and
later correction. The first two standalone argv records are explicitly
retrospective/transcript-derived; saved gateway result objects independently
recover their request IDs and cwd, not the submitted argv.

## Focused observations and retained failures

The initial default-inline candidate exposed a cursor codec integration error:
encoded traversal version 3 reached a decoder still requiring version 2.
The failure was retained and the decoder changed through request 4.
No file or producer claim from that failed run is presented as a later success.

| Facility / iteration | Outcome | Wall s | Load before → after |
| --- | --- | ---: | --- |
| Initial query suite | Cursor-version refusal, retained | 0.54 | 14.63 → 14.63 |
| Corrected query suite | Passed | 5.28 | 8.44 → 9.28 |
| Initial Store migration | Passed, with the v2-fixture qualification below | 3.02 | 10.02 → 10.10 |
| Store plus mixed legacy-ledger control | Passed, same qualification | 2.56 | 5.88 → 6.05 |
| Narrow standalone build scope | Passed, same qualification | 2.35 | 7.58 → 7.58 |
| Existing no-create matrix | 25 cases passed | 1.89 | 10.10 → 10.10 |
| Eager-v3 oracle, first fixture | Source-grant fixture refusal, retained | 1.68 | 10.10 → 10.10 |
| Eager-v3 oracle, second fixture | Mutable-reference fixture refusal, retained | 29.97 | 6.05 → 18.00 |
| Eager-v3 oracle, corrected fixture | Passed | 25.10 | 12.93 → 10.46 |
| Existing final-publication control | Passed at the late boundary | 0.55 | 10.10 → 10.10 |
| Ordinary I/O counter migration | Passed | 4.50 | 10.46 → 10.42 |
| Added indexed-counter observation | Fixture phase-attribution assertion failed, retained | 4.45 | 7.58 → 7.70 |
| Corrected indexed-counter observation | Passed | 5.00 | 7.24 → 7.62 |
| Final inline64 producer, corrected Store fixture | Passed | 2.76 | 7.85 → 8.19 |
| Final inline64 producer, query suite | Passed | 6.26 | 9.72 → 9.78 |

The no-create matrix retains absent/readonly/symlink/dangling/FIFO and
forged/substituted-envelope cases for ordinary Store, locator, historical root
and historical node surfaces. Hostile fixtures are owned and sequential.
The query suite retains eight locator attacks and seven container/node attacks,
including front/node corruption, truncation, trailing bytes and substitution.
Authentication refusals write no partial query stdout.

Direct QueryAccess tests exercise the exact pin, a wrong snapshot argument,
invalid extents and valid repeated-descriptor reuse. A valid container larger
than an 8 KiB reader cap succeeds while every requested allocation is bounded.
The generation-capacity controls preserve authenticated producer separation.

The eager-v3 oracle independently serializes boundary data and compares exact
root/snapshot bytes and continued query stdout with the production lazy stream.
Its four data boundaries cover empty data, an empty string, the maximum fact,
and multibyte 16-line pages. It compares nine question kinds over 70 continued
replies and 1,626 facts. Additional exact fixtures check malformed root bounds,
an invalid unselected branch child, and a cached digest presented later with an
invalid or shifted descriptor; valid repeated reuse remains accepted.

The mixed valid-budget fixture contains canonical legacy page/root/latest/
locator rows, v3 rows, an unrelated ordinary proof and a similarly prefixed
semantic-latest-extra namespace. Only the exact legacy families are retired.
This tests accounting recognition without adding a v2 query fallback.

The failed-eviction fixture is deterministic and single-process: an owned
directory occupies the second record that eviction would remove. A prior
record is removed, the failed removal poisons the writer, the original signed
Budget remains, and neither an ordinary entry nor a staged append publishes.
No concurrent observer, tracing or scheduling race is involved.

The final-publication fixture derives an exact 63,360-byte capacity and verifies
that the late semantic-input memo exists before the refusal. It therefore
reaches the post-Memo retention boundary rather than passing on an earlier
capacity failure.

### Historical v2 cursor fixture qualification

The first migrated standalone fixture used the wrong domain preimage for its
v2-shaped envelope. Those earlier passes demonstrate rejection of that malformed
keyed envelope, not rejection of a valid historical v2 cursor.
The old v2 key derivation uses raw
lkjscript-semantic-cursor-question-2 bytes before framed producer/question fields.
A case-only checked operation corrected the fixture, and the final inline64
Store run passed with that exact historical preimage. The subsequent tiny
oracle also authenticates an actual cursor emitted by the unchanged v2 ADC
producer before comparing its decoded state with v3.

### Counter qualification

Optional Store counters remain disjoint timed intervals. The new stream-write
counter measures completed write buffers, including initial front reservation
and backpatch; it is not all attempted syscall traffic.
The header counter still means completed fixed 90-byte headers. Separate
counters report completed 400-byte manifests and physical EOF checks.

The expanded counter fixture initially assumed all Store time lay in the
driver's store_io interval. Indexed publication is nested within semantic_index,
so that assertion was too narrow. The ordinary assertion remains unchanged;
the indexed enclosing bound uses store_io + semantic_index. Exact counter-sum
equality with the summary is checked separately. The broader enclosing bound
alone is not a proof that its subintervals are mutually exclusive.

The passing tiny indexed-body observation sees one authenticated fixed
manifest and one EOF check, with 400 manifest bytes; the prior ordinary counter
checks remain intact. These are attribution checks, not throughput benchmarks.

## Factory and prelaunch comparator

The final measurement producer is the inline64, byte-identical two-generation
candidate eed6ca32edcc182a92c4e697f4702ec685dff8e7778b0374a27552aa9c938947.
Its complete factory commands, hashes, source manifest, timings and load are in
the identity addendum. Earlier inline0 candidates remain correctness-only.

The separately declared tiny preflight syntax-checks all helpers, then indexes
one two-function module with the actual fixed v2/v3 producers.
It authenticates and compares all 169 logical pages, 206 facts and 35,987 value
bytes; the v3 container has 17 nodes and 58,788 physical bytes. Native images are
byte-identical. One outline question forces a cursor with limit 1, and a second
uses limit 96. Both compare exact fact bytes and authenticated decoded traversal
state. Only the independently verified version/producer/snapshot fields and
the traversal state's version word are normalized.

The static review caught and repaired two pre-execution helper defects:
Perl lexical snapshot variables shadowed sort aliases, and the header validator
omitted the actual kind/depth fields. A later hardening confines normalization
to the verified final continuation record, with exactly one replacement.
No measured observation used those drafts.

All tiny stages had load 6.93 → 6.93; baseline/candidate index elapsed times were
0.35/0.41 s, and each comparator elapsed time was 0.01 s. These coarse values
serve validation only. A further read-only validation of captured summaries
and zero-hash pinned-query stats took 0.06 s, load 4.18 → 4.18.
The full run metadata, including the current helper hashes, is retained.

## Evidence and remaining gates

Implementation evidence is copied unchanged to
compiler-query-container-data/implementation; the separate tiny preflight is
under compiler-query-container-data/helper-preflight-1.
The manifest covers 789 intentional text records. Four 32-byte public root
digests captured by the query fixture are represented as explicit hex files;
binary-digest-encodings.tsv records their raw SHA256 and encoding paths. An
initial text-only packaging check stopped on those non-UTF-8 digests before
copying anything. All other records are copied byte for byte. No private key,
store, native image or compiler executable is committed.
Large owned fixtures and keys remain private until report acceptance and are
then removed; historical raw evidence is preserved.

This isolated work has not run or bypassed the full cold gate.
Root owns integration, complete gate, CI, seed and landing.
The new syscall/race observer was automatically stopped during preparation;
it was not recreated. New concurrent mutation and precise interruption
boundaries outside the existing facilities remain explicit validation gaps.
The representation remains text in Git, with authenticated semantic snapshots
as queryable claims; no repository-truth decision changed.
