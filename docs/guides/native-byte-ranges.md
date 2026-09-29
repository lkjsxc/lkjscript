# Native byte ranges

Tested runtime: development v0.1.61. See [status](../status.md) for the separately
published binary. The operations are ordinary immutable `Bytes`; they do not add
an owned buffer, mutable view, lifetime parameter or region type.

## Select bytes, not characters

`bytes-slice(bytes: Bytes, start: I64, end: I64) -> Bytes` selects the half-open range
`[start, end)`. Both bounds must be nonnegative, start must not exceed end, and end
must not exceed the byte length. Invalid bounds trap rather than clamp. Equal valid
bounds return empty Bytes, including a range at the end of an empty or nonempty input.
This differs from `list-window`, whose third argument is a clamped count.

With `(use std builtin)`, an ordinary function body can use:

```text
(call std::bytes-slice (local bytes) (local start) (local end))
```

A range can contain NUL, 255, invalid UTF-8 or part of a multibyte character. It is
still valid binary data. Decode separately with `bytes-to-text-result` when the
application needs a checked text boundary; slicing does not decode or normalize.
The production checked evaluator shares backing storage without copying the selected
payload. The source-reference evaluator independently copies equal values.

## Choose retention deliberately

A nonempty range keeps its original backing alive. For example, retaining a four-byte
field can keep a much larger input buffer alive even after the original local variable
has gone away. Repeated slicing stays flat: a view owns the original backing, not an
unbounded chain of older views. Empty ranges do not retain that backing.

`bytes-copy(bytes: Bytes) -> Bytes` creates an independent backing for the visible
nonempty bytes. The [tested consumer](../../tests/fixtures/byte-ranges-consumer.lkjc)
passes `std::bytes-copy` as an ordinary function value to a generic range transformer.
Use that boundary when a small result must outlive a large parent without retaining
it. Other aliases still own the parent, and copying has its own allocation and byte
copy cost. It is not secure erasure or a promise that process RSS immediately falls.

Slicing, copying and subsequent concatenation preserve every retained alias. Map
keys compare visible contents, so a shared view and an independently copied equal
value address the same key. Captures keep a valid immutable value after their creating
function returns. Typed-data and JSON encoding emit only the visible bytes.

## Compose across an exact package boundary

The maintained [library request](../../tests/fixtures/byte-ranges-library.lkjc) owns
three ordinary functions: `map-range<Output>` invokes a supplied `Bytes -> Output`
function on a range, `remember` returns a callable retaining its input, and `payload`
selects a packet body whose length is stored in its first octet. The packet function
is deliberately small: an absent length byte or truncated body traps, and trailing
bytes remain outside its selected result. It is not a complete network protocol.

The [consumer request](../../tests/fixtures/byte-ranges-consumer.lkjc) combines that
library with `Bytes` and `Text` callbacks, captures, copies, map keys, concatenation,
nested ranges and typed-data round trips. Its `LIBRARY_PACKAGE` and
`LIBRARY_PACKAGE_REVISION` fields are substitution slots for the exact identities
returned by library export, not literal package names to paste unchanged.
Follow the [native library workflow](native-library.md) for export, staging, dependency
acceptance and the separate consumer revision. No compiler-internal graph writer is
needed; the accepted meaning graph remains authoritative.

Discovery uses the installed executable:

```sh
lkjscript package builtin query owners --name bytes-slice
lkjscript package builtin query owners --name bytes-copy
```

The [public integration test](../../tests/public_cli/native_byte_ranges.rs) retains
the complete executable workflow. It copies the executable outside the compiler
checkout, authors and checks both packages, verifies canonical drafting, and builds
a command artifact. It then removes both source projects, requests, draft and exported
transport before exercising the detached artifact. Independent binary expectations
cover empty/full/nested ranges, all-octet input and packet bodies. Invalid ranges and
foreign inputs must publish no result and must not change the accepted revision.

## Availability and limits

The expanded standard adds two pure closed externals, `core.bytes.slice` and
`core.bytes.copy`, with exact signatures. A runtime missing either intrinsic rejects
the new closure during admission, even when that declaration is not called. Existing
applications retain their exact standard selection and are not upgraded by installing
a different runtime. The preceding v0.1.60 candidate does not contain this addition.

New view descriptors and copied payloads retain checked allocation admission and
cancellation. Input/value limits are not raised. These are cumulative work bounds,
not precise accounting of all retained physical memory or allocator overhead. This
slice removes a concrete payload copy; it does not establish general scoped borrowing,
region reclamation or whole-program zero-copy execution.

The [language contract](../spec/language.md#immutable-byte-ranges-and-explicit-backing-detachment)
owns semantics. The [campaign](../campaigns/20260930-byte-ranges.md) owns exact-source,
predecessor, allocation, retention and native-execution evidence, including failed
intermediate attempts rather than only the final passing result.
