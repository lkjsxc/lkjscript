# Compiler structure: supplemental exact-filename history roots

This supplement adds **one applied replay from four ordinary-program candidates**
in the same frozen three hundred commits. It does not change the original
179 compiler-root campaign or its eight successes. The other three candidates
stop at the current source-extension rule, before their source bodies are parsed.

The additional success is `70680680`, the lifecycle fixture. One checked
`replace-body` operation publishes the exact desired source bytes. Its desired
and applied-candidate checked IR is also byte-identical when emitted under the
same disposable path: 105,550 bytes, SHA-256
`c16cf6a97763063078c9d35bdeed09fc6b10ab32a633e931375f68869bcd3098`.

## Population and contract

The population is still frozen at
`1f48bd24c90f98e833c3647d902ea0b4515d4d90`. The original campaign selected
179 commits with changes in `src/` or `library/`; these four candidates come
from the other 121 commits. They were selected by inspecting changes to existing
ordinary source files under `tests/conformance/` and `examples/`, without
requiring new-file publication.

The source filenames and bytes are exact historical values. This campaign
creates no `.rs` to `.lkj` aliases, renames no historical files, and changes
no historical source to accommodate the current compiler. The harness asserts
that every candidate belongs to the frozen population, changes neither
`src/` nor `library/`, and has exactly the one recorded ordinary-source
change. Documentation and host-tool changes are listed in the raw commit-path
records and are outside this source-program proof.

The immutable producer is SHA-256
`4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09`,
20,627,356 bytes. It is copied and rehashed before the campaign. This is a
different producer and root selection from the original report, so these results
are a separate supplement.

Every pair first uses its exact historical library directory. When that route
refuses, the harness also records a separately labelled diagnostic trial with
the producer's embedded current library. Those diagnostic trials do not count
as historical-environment replays. All three such trials hit the same
`E_SOURCE_EXTENSION` before parsing; no library substitution admitted a
program or produced a replay.

## Four outcomes

| Revision | Exact selected root | Result |
|---|---|---|
| `43cc8ae9` | `tests/conformance/migrated/backend_vec_probe.rs`, entry `probe` | Both sides refuse `E_SOURCE_EXTENSION`; no planning or publication |
| `60b270eb` | `tests/conformance/output/process_capture.rs`, entry `main` | Both sides refuse `E_SOURCE_EXTENSION`; no planning or publication |
| `70680680` | `tests/conformance/output/lifecycle.lkj`, entry `main` | Both sides check; dry run and actual publication pass; complete changed-source coverage and exact IR equality |
| `075d9801` | `examples/http.rs`, entry `main`, loading changed `http_server.rs` | Both sides refuse `E_SOURCE_EXTENSION`; no planning or publication |

The three refused cases do not establish that their language constructs are
retired. The current driver refuses their exact filenames before examining
those constructs. Relaxing that rule or introducing filename projections would
be a different experiment. The writer independently restricts publication to
`.lkj`, but these trials never reach that later rule.

The other two excluded-population commits touching ordinary corpus sources
were not run: `70fe3b69` changes two fixtures and adds
`socket_process_witness.rs`; `7fe59bc8` adds 76 corpus sources including
intentional rejection cases. The former has new-root and multi-root coverage
requirements. The latter is one commit, not 76 replay opportunities.

## Lifecycle proof and cost

The parent is `4048e172ab9d2e3d517d6d79e52072f0fedf3d46`. The historical
change retains the 65,537-byte entropy fill and stops scanning after finding
the first changed byte. The transaction replaces `main`'s body in the one
changed source file. It checks 70 fresh instances, reuses no checked instance,
and reports no authority change. No lifecycle code or external process is run
during this campaign; admission and replay operate on the checked program.

| Step | Wall seconds | User / system seconds | Peak KiB | Load before → after, 1 / 5 / 15 minutes |
|---|---:|---:|---:|---|
| Parent admission | 0.18 | 0.18 / 0.00 | 42,112 | 2.07 / 2.36 / 2.74 → 2.07 / 2.36 / 2.74 |
| Desired admission | 0.18 | 0.18 / 0.00 | 40,496 | 2.07 / 2.36 / 2.74 → 2.07 / 2.36 / 2.74 |
| Dry run | 0.93 | 0.91 / 0.00 | 69,772 | 2.07 / 2.36 / 2.74 → 2.07 / 2.36 / 2.74 |
| Apply | 0.90 | 0.86 / 0.01 | 68,540 | 2.07 / 2.36 / 2.74 → 2.07 / 2.36 / 2.74 |
| Emit desired IR at common path | 0.20 | 0.19 / 0.00 | 40,264 | 2.07 / 2.36 / 2.74 → 2.07 / 2.36 / 2.74 |
| Emit applied candidate IR at common path | 0.19 | 0.19 / 0.00 | 40,496 | 2.07 / 2.36 / 2.74 → 2.14 / 2.37 / 2.74 |

The actual apply report contains `mode=apply`, one operation, and equal
desired/candidate checked-meaning identity
`d8fce79ca3ede040bbabdf4e26b16f8580ea17c9346f88535c0e40b30d6a0d46`.
Publication returns before the driver writes that report.

The source before hash is
`cec4d68163055b2bc72783244c43cc5d28d5c3e757ca24e4bcba9391f6271c7d`.
Both desired and published source hash to
`7adcb557666bb00a85d8e73a00040848e9cb51cebeaf07676c2c75b9419d8e80`.
For the additional exact-IR proof, the desired tree and then the applied
candidate are copied into the same disposable source path and independently
checked and serialized. Their unmodified IR byte streams compare equal; this
does not substitute checked-meaning equality for exact IR equality.

All 18 compiler commands use one sequential worker, a 4 GiB address-space
limit and a 900-second timeout with a five-second termination grace. The
authenticated store budget is 2 GiB. Free space before expansion was
115,841,068 KiB, above the 3 GiB preflight requirement. The recorded command
walls sum to 2.58 seconds at GNU time's hundredth-second resolution; the
extension refusals individually report 0.00 seconds at that resolution.
Temporary trees, the copied producer, signing key, IR artifacts and store
were removed. Raw output and measurement evidence remain; no subprocess remains.

An initial harness quoting error occurred before its first compiler command.
It is preserved as a harness setup failure and contributes no admission,
replay or performance row. The corrected harness then completed all four
candidates without a retry of any compiler command.

## What this does and does not resolve

The original eight successes are actual publications, not dry-run equalities:
the retained raw reports for all eight contain `mode=apply`. Their
documented `.lkj` filename projections remain part of that earlier campaign's
contract. This supplement neither removes that qualification nor changes their
original producer or counts.

The original first failures include 20 newly loaded-file cases and one separate
extension-transition ambiguity. Seventeen of those 20 have existing parent
directories; fourteen also change semantic-test sources outside the selected
CLI root. Only four of the six without that coverage gap are clean first
candidates for a small existing-parent file-creation operation:
`a49aceb3`, `4891bf56`, `4bea1891` and `22b97948`.
`675c088d` needs a new parent directory, and `2dff5612` also changes a
top-level import in an existing file, which retains the separate framing rule.

A future creation operation would need an explicit absent/present before-state,
an acquired existing parent directory, a pinned negative input snapshot, full
candidate checking of the newly loaded module, and signed recovery that accepts
only old absence or exact new bytes. Staging must precede atomic publication;
writing directly to the final new leaf would leave a partially written third
state after a kill. The current replacing rename is not an atomic absent-only
operation. A strict competing-writer absence guarantee needs no-replace
publication; retaining only the present advisory-writer assumption must be
stated explicitly. Missing parent directories can remain refused in the first
implementation. None of that writer work is implemented by this supplement.

The requested thirty replays remain unproved. Even treating every new-file
first refusal as a future success would give only 8 original + 1 supplemental
+ 20 new-file cases = 29, before accounting for the later scope and framing
barriers. Multi-root test coverage, the separate data/framing cases, or another
explicitly justified program-root campaign would still be needed. The 144
older compiler-root current-language refusals are unchanged.

## Reproduction and evidence

Run the supplied harness with an executable matching the producer hash:

```sh
bash bench/compiler-history-roots.sh \
  /absolute/lkjscript-repository /absolute/frozen-compiler \
  target/compiler-history-roots-exact
```

The output prefix must be new. The fixed four-candidate manifest and original
filenames are part of the harness, not dynamically selected by successful
outcomes.

- [Harness](compiler-history-roots.sh)
- [Candidate manifest](compiler-history-roots-candidates.tsv)
- [All command measurements](compiler-history-roots.tsv)
- [Candidate outcomes and identities](compiler-history-roots-results.tsv)
- [Producer and resource metadata](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-history-roots.meta)
- [Raw command output and proof observations](compiler-history-roots-observations.txt)
- [Original compiler-root campaign](compiler-history.md)
