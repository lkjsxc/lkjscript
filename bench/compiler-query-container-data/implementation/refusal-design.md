# Deterministic failed-eviction refusal cases

Prepared for the existing native Store/query-store fixture. No observer, tracing,
signals, competing process or concurrent write is involved. These are proposed
cases; no compiler or test has run during the gate reservation.

Production request-2.json SHA-256:
d4d7f47ea8058f8c0b5b4aa8b81858687b27466d5e07b5da57efce7598fe155c

## Common owned fixture

Use a fresh owned directory, a private key outside compiler source/library grants,
and Store::open_bounded with capacity 24,000 bytes. Put two entries under one
producer, in distinct namespaces ordered lexicographically:

- a-poison, fixed digest key A, 14,000 payload bytes;
- b-poison, fixed digest key B, 64 payload bytes.

Call finish and retain the exact signed .budget-index bytes. These entries plus
their row/index costs fit below the cap. Compute the exact physical names from
the actual raw producer digest and existing producer-slot derivation. Keep the
same Store object open, so it still owns its writer lock and loaded accounting.

Through an ordinary Directory capability for this same owned fixture, remove
only the b-poison regular file and create a directory at that exact basename.
Do not modify any outside path. This deliberately makes the later erase fail,
without a scheduling race. The already-held Store does not reconcile again.

## Ordinary publication

Attempt a c-poison put of 16,000 payload bytes. This must require eviction.
The first a-poison removal succeeds and frees enough bytes that the proposed
new entry would fit, but b-poison removal fails because its leaf is a directory.

The repaired code must not treat the failed eviction as an empty-generation
case and resume publication after checkpoint. Assert:

- a-poison is absent; b-poison remains a directory;
- c-poison is absent and get(c-poison) is None;
- another small put is also refused by the failed writer state;
- finish leaves the retained signed budget bytes unchanged (failed cleanup did
  not publish misleading replacement accounting);
- no newly published proof/container or owned staging directory remains.

A narrow old-source ordinary control may use this same Store-only sequence
without any new container API and report whether c-poison was published.
Retain its exact source/producer and output; the expected prior behavior is
an observation to verify, not a fabricated baseline result.

## Staged append

Use an independent copy of the common fixture. Begin QueryContainerWriter while
both original entries are still regular. Its 490-byte reserved front and
prospective entry/index overhead fit. Then make the same owned b-poison
directory substitution through the independent Directory capability.

Append a bounded 16,000-byte test payload. The required prospective container
extent exceeds capacity while both old entries remain; removal of a-poison
makes the extent fit, then b-poison removal fails. Assert:

- append_node returns Err immediately;
- a second append on that same failed writer returns Err;
- dropping the writer removes its owned stage;
- no container, locator or latest has been published;
- a-poison is absent, b-poison remains the controlled directory;
- the signed budget bytes remain the pre-failure bytes;
- ordinary publication using the same Store remains refused after writer drop.

The test payload is an opaque bounded transport specimen. No claim is made that
it is a semantically parsed fact page; no successful finish is attempted in
this refusal case. Existing successful query/index cases cover valid nodes.

## Implementation form and evidence

Add the two modes to the existing checked query-store fixture, preserving its
producer/pin/cursor/budget observations. Because changing Store's API makes the
old standalone fixture root fail before editing, validate the Store and fixture
changes jointly in a fresh owned baseline-source copy through identity operations.
Retain request, source manifests, complete checked apply output and exact final
Store/fixture hashes. Transfer only the byte-identical checked fixture output to
the production worktree; account any unmatched manual remainder as zero.

Run only after the compiler worker is released and the core production request
has checked. Use one worker, external 4 GiB/900 s limits, before/after loads and
full stdout/stderr/exit evidence. A mismatched refusal cause or an ordinary
success caused by a setup mistake is a failure, not permission to retry silently.
