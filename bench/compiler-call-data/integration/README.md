# Replace-call integration on the corrected public parent

This packet records the new local integration separately from the preserved
[original landing packet](../landing/README.md) and its original gate receipts.

## Frozen identities

- Exact verified public parent: `e414aeddbb2ec6985fd564c96a44a4f583b6ba1b`.
- Original reviewed source: `ecd0ff6d0e2210be5b7eaed9770b675b7c070554`.
- Original code/evidence branch: `7f87d6d7b73365a6d9976c367823d48343a6b347`.
- History-preserving integration merge: `db299790d539f75f5bc3cde25399628fd4952737`.
- Separate seed-only commit: `20935b4c66f87e9e52a68ce79bf5e60603b75198`.
- Expected compiler and decoded seed SHA-256: `103557eddf30d24e046cb09f6a881e549829283495c8648c0ac9dcd1d72a5d35` (40,989,463 bytes).
- Expected seed gzip SHA-256: `65408daa78849909a0a3ddcf0a2cab29120b9ddfc095ffb2095c422261f6a6ee` (8,204,839 bytes).

The original proposed gzip and its decompressed image both matched these exact
approved identities before the seed was copied. Decompression also compared
byte-for-byte with the original reviewed producer.

## Integration checks

The merge completed automatically with no conflicts. The public parent and
original candidate are both ancestors, so public main can fast-forward to this
branch. Relative to the public parent, production changes remain exclusively
`src/driver/help.lkj`, `src/structure_edit.lkj` and `src/syntax.lkj`.
The complete production patch SHA-256 remains
`9c90f2ed55346a356a919a1cedcaa6195524ddf8ba5867647aeb4f13b9b43bac`,
identical to the reviewed original packet.

Compiler, library, witness, tests and examples are byte-identical to the reviewed
source. The public macro-origin diagnostic harness is preserved exactly, and
`tools/check.sh` adds only the reviewed call-suite registration relative to
public main; its 1,800-second macro-origin watchdog remains. The public Programs
status section is unchanged. Type defaults and namespace-collision repairs
remain separate.

## Verification status

The new integrated four-worker gate is pending. It will use the shared mkdir lock
and a fresh empty store, retaining the exact clean-tree input, start/end times,
exit status, bootstrap identity and standalone real-program receipts. The two
pending conformance entries remain explicit.

The prior Rust witness passed on the identical reviewed compiler source and
producer. Its reuse requires exact equality of every relevant source, library,
witness, test, example and invoked host-script input, plus the newly bootstrapped
compiler bytes; the final receipt will record that binding or a rerun.

No publication is authorized to this stream. The final exact candidate and
receipts go to lead for review and the external next action.
