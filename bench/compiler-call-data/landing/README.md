# Local landing packet: checked call replacement

Prepared for lead's separate integration decision. The implementation and all
proofs remain local. Publication is held while Programs diagnoses the public
parent's macro-origins CI watchdog failure. No type-defaults changes or watchdog
repair are included here.

## Exact inputs and deltas

- Published parent: `739c8cde24f8df4644274451d5f39c23302a8af6`.
- Tested compiler and gate inputs: `ecd0ff6d0e2210be5b7eaed9770b675b7c070554`.
- Initial completed report/evidence commit: `513e13a80f6f1a7aad0054aa4c05dacfb36c8283`.
- Native producer: 40,989,463 bytes, SHA-256 `103557eddf30d24e046cb09f6a881e549829283495c8648c0ac9dcd1d72a5d35`.
- Compiler, library, tools, tests and witness input files remain identical to the
  tested source. The original full-gate receipt files remain identical to the
  initial evidence commit.

| Artifact | Review scope |
| --- | --- |
| [Production patch](production-source.patch.gz) | Exact diff from the published parent to tested code in `src/driver/help.lkj`, `src/structure_edit.lkj` and `src/syntax.lkj`; no library change. |
| [Tests and gate patch](tests-and-gate.patch.gz) | One `tools/check.sh` registration and the focused `tools/structure-call-test.sh`; no conformance or witness-source change. |
| [Documentation patch](documentation.patch.gz) | Current `docs/status.md` and `docs/structure.md` delta from the published parent; Programs status text preserved. |
| [Source delta](source-delta.txt) | Five source/test files, 451 insertions and three deletions in total. |
| [Manifest](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-call-data/landing/manifest.json) | Exact hashes, scope, proof outcomes and unapplied seed proposal. |

The patches are gzip encodings of exact Git diffs; decompression preserves their
required blank context lines. Production and test patches target the named parent.
Lead must reconcile the later watchdog repair separately before landing. The
report, replay runner and raw evidence are the separate benchmark/documentation
follow-up; they do not change the native compiler or full-gate inputs.

## Completed verification

The preserved [full gate](../gate/full-gate.log) passed in **1,260.372 seconds**
with four workers and an initially absent store: 6,575 case requests rerun,
zero case-request reuse, and 128 IR mutants with zero findings. Both conformance
modes retain two pending entries. The native output and public seed hashes were
verified after the gate; its shared lock was released.

The subsequently requested [standalone real-program check](standalone.log) passed
in **4.02 seconds**, including exact source/provenance controls, eight tests on
each engine and 7,239 native corpus rows.

The complete documented [Rust witness](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-call-data/landing/witness.log) passed in **156.87 seconds**:
499 values/output cases and 652 verdicts, plus its directory, network, macro,
TemporaryFiles and original-program checks. Its Rust-built compiler emitted
exactly the native bytes above. The Rust-host compiler itself has SHA-256
`fbebd08a32c9823642625e240164c2590ce4fdb443b8f886230b963ab36f4ac6`.
[Output identities](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-call-data/landing/witness-output.sha256), [toolchain](toolchain.txt) and both
[standalone](standalone.time) and [witness](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-call-data/landing/witness.time) time records are retained.
Cargo used one build worker and one release code-generation unit; the native
witness coordinator used its one-worker default. These are script observations,
not comparative performance claims. There is no CI result for this unpublished
compiler slice.

## Proposed separate compiler-owned seed step

Only `seed/lkjscript.gz` would change. The proposal is not applied to that path
and is not a seed commit. `docs/direction.md` assigns seed refresh to the compiler
in its own commit.

| Property | Current tracked seed | Proposed seed |
| --- | --- | --- |
| Gzip bytes | 8,192,800 | 8,204,839 |
| Gzip SHA-256 | `9327f64a3c9a5c37146ca6c91a6926a8707d9fef3d9fd67398125909f4b5f612` | `65408daa78849909a0a3ddcf0a2cab29120b9ddfc095ffb2095c422261f6a6ee` |
| Decoded bytes | 40,929,908 | 40,989,463 |
| Decoded SHA-256 | `a2d0d8581c73ceb937addb5ea986ba708fa06c2505161d8745353cf123a9bc8f` | `103557eddf30d24e046cb09f6a881e549829283495c8648c0ac9dcd1d72a5d35` |

The prepared artifact is:

`/home/coder/workspace/lkjscript-semantic-operations-20261118/target/semantic-operations/landing/proposed-lkjscript.gz`

It was created with `gzip -n -9` from the reviewed native producer and decoded
byte-for-byte against that producer. Its compressed size increase is 12,039 bytes.
The deterministic bytes are reproducible from the tested compiler; the large
proposal artifact is kept outside the source/evidence commit.

After lead's integration decision, the separate seed step would copy that exact
artifact to `seed/lkjscript.gz` and commit only that file. This proposal assumes
compiler and library sources remain identical to the tested inputs. No proposed
seed bootstrap/full-gate run or publication has been performed; a new full-gate
slot and publication remain lead's decisions. The already completed gate receipt
continues to describe the original published seed, not this proposal.
