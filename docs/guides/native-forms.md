# Bounded native form encoding and strict input

This guide requires **v0.1.46 or a compatible successor**. The official immutable
v0.1.45 binary does not recognize `core.bytes.from-list` or
`core.bytes.to-text-result` and correctly rejects the new closure. Application
use requires a compatible executable, not Cargo or a compiler checkout at runtime.
The [original campaign](../campaigns/202609250926.md) owns the decoder observations.
The [encoding continuation](../campaigns/202609261045.md) proves the new library on
unchanged public v0.1.47, from fresh authorship through detached execution.
The [durable editor](native-editor.md) consumes it for authenticated, conditional
saves. Public v0.1.46 now supplies the required operations; the
[release record](../release.md) retains its distinct distribution evidence.

## A library, not a form-specific runtime primitive

The [literal library](examples/form-codec.lkjc) exports ordinary nominal data:

```text
field { name: Text, value: Text }
result = valid(List<field>) | invalid(Text)
encoding = valid(Bytes) | invalid(Text)
decode(body: Bytes, maximum-bytes: I64, maximum-fields: I64) -> result
encode(fields: List<field>, maximum-bytes: I64, maximum-fields: I64) -> encoding
```

Its state machine, percent decoding/encoding, limits and error policy are ordinary
pure lkjscript. The kernel contributes only general byte construction and checked
UTF-8 conversion. There is no form-specific opcode, HTTP adapter parser, JSON/base64
round trip, external executable or language-to-language source generator.

The [consumer](examples/form-consumer.lkjc) imports the exact library, projects its
nominal fields into an application-owned structural report and exposes individual
and batch command targets. It does not inherit filesystem/network authority.

## Construct a body or query without interpolating values

The encoder follows the UTF-8 [URL Standard form serializer](https://url.spec.whatwg.org/#urlencoded-serializing).
ASCII letters, digits and `* - . _` remain unchanged. A space becomes `+`; all other
UTF-8 bytes become uppercase `%HH` escapes. A literal plus is `%2B`, a tilde is
`%7E`, and an already percent-looking value is encoded as data, exactly once.
Every field contributes `encoded-name=encoded-value`, joined by `&`. Empty names
and values, duplicate names and pair order are retained. No BOM stripping, Unicode
normalization, line-ending conversion or special `_charset_` substitution occurs.

For example, two fields named `tag` with values `猫 + Rust` and `a&b` produce:

```text
tag=%E7%8C%AB+%2B+Rust&tag=a%26b
```

The byte limit applies to the **complete encoded output**, including expansion,
equals signs and ampersands. It is not an input-character limit. An empty field
list succeeds with zero bytes and zero fields; one empty name/value requires one
byte for `=`. Bounds are inclusive, so an exact-size output succeeds. Negative
bounds reject first, too many fields next, and output overflow returns
`invalid("body too large")`. No successful prefix is exposed on failure.

The ordinary implementation checks remaining capacity before appending each
encoded byte group or delimiter. It also checks a text's UTF-8 byte length before
copying that text to Bytes. It does not reserve storage proportional to a caller's
maximum, and subtracts from remaining capacity rather than adding an unchecked
output size. A large positive bound does not bypass runtime resource policies.

This is a form/query serializer, **not** a URL path escaper, an HTML escaper or an
origin/redirect validator. Use its Bytes as a form POST body with the matching
Content-Type, or its ASCII text as an entire query. Do not percent-encode the result
again. A URL placed in markup still needs the typed UI library's attribute escaping.
Authentication, permitted destinations and transport capabilities remain explicit
application responsibilities.

Encoding retains bounded persistent-list staging and is not a zero-copy or
throughput claim. Per-form bounds do not replace aggregate allocation, collection,
cancellation or task limits. A batch of individually valid forms can exhaust its
execution policy; that operational failure must not become an `invalid` data value.
Callers must choose an appropriate batch size and execution policy. The maintained
checks exercise individual 8192-byte outputs without raising the default command
quota, and reject an already oversized input before copying its content.

## Exact input policy

The parser consumes `application/x-www-form-urlencoded` bytes. It splits on raw
ampersands, skips empty segments, splits each nonempty segment on its first raw
equals sign and treats a missing equals sign as an empty value. A plus becomes a
space. Percent escapes decode exactly once; escaped separators never split fields.
Duplicate names remain distinct and ordered. Neither the first nor the last value
silently wins. Empty names and values, NUL, BOM and line endings remain data.

Malformed percent escapes and incomplete or invalid UTF-8 reject the complete
result. No accepted prefix or replacement text is exposed. This is deliberately
stricter than the permissive decoding algorithm in the
[URL Standard](https://url.spec.whatwg.org/#urlencoded-parsing); it is not a claim
of identical recovery behavior. Valid form encoding follows the same delimiter,
plus and ordered-pair rules. Non-UTF-8 legacy form encodings are unsupported.

Limits are inclusive. Negative limits reject. The original encoded body length
is checked before parsing, so percent expansion cannot bypass the byte bound.
Each nonempty segment, including `=`, counts as one field. Empty segments do not.
The output has at most the original byte volume plus bounded list/field metadata;
intermediate allocation, cancellation and execution policy still apply. The
implementation traverses persistent lists; this is not a zero-copy parser or an
unbounded-input throughput claim.

| Failure | Library result |
| --- | --- |
| Negative bound | `invalid("invalid limits")` |
| Original body exceeds its bound | `invalid("body too large")` |
| Too many nonempty field segments | `invalid("too many fields")` |
| Missing or nonhex percent digits | `invalid("invalid percent escape")` |
| Invalid decoded name or value | `invalid("invalid UTF-8")` |

Operational allocation/cancellation failures remain failures; they are not converted
into malformed-input results. The general `bytes-from-list` traps on any I64 outside
0 through 255. `bytes-to-text-result` distinguishes valid empty text from invalid
encoding; the old trapping `bytes-to-text` remains unchanged.

## Author and import through the public product

In a fresh owned directory, create a command library, obtain its revision and replace
only `LIBRARY_BASE` in the literal library proposal with that exact revision:

```sh
lkjscript new forms --template command --name forms
lkjscript --project forms status
lkjscript --project forms change plan --input-file forms.lkjc --output forms.plan
```

Review the original declarations and complete exported plan before applying its
returned token to the identical input. Then check and export the accepted package:

```sh
lkjscript --project forms change apply --input-file forms.lkjc --plan PLAN_TOKEN
lkjscript --project forms check
lkjscript --project forms package current export --kind transport --output forms.lkjp
lkjscript --project forms build --output forms.lkja
lkjscript new consumer --template command --name form-consumer
lkjscript --project consumer status
lkjscript --project consumer package dependency stage \
  --transport LIBRARY_TRANSPORT --input-file forms.lkjp
```

The export supplies exact package, semantic revision, package revision and transport
values. Substitute those metadata fields and the consumer base into the separate
literal consumer proposal. Review/plan/apply it, then check and build it in the same
way. The `report` port takes `Bytes, I64, I64`; `batch` takes `List<Bytes>` and uses
8192 bytes and 64 fields per body. Argument-file Bytes use the public tagged base64
boundary, not integer arrays; native algorithms operate directly on Bytes.

The same consumer exposes `encode(List<Field>, I64, I64)` and
`encode-batch(List<EncodingInput>)`. `EncodingInput` contains `fields`,
`maximum-bytes` and `maximum-fields`. The result has `valid`, `body: Bytes` and
`error`; invalid input has `valid=false`, empty Bytes and an explicit error.
An empty valid body is distinguished by `valid=true`.

After accepting the consumer, save this literal command argument array as
`encode-arguments.json`:

```json
[[{"name":"tag","value":"猫 + Rust"},{"name":"tag","value":"a&b"}],8192,64]
```

```sh
lkjscript --project consumer run encode \
  --arguments-file encode-arguments.json --result-file encoded.json
```

The native result body contains the wire example above. Only the external JSON
result represents Bytes as `{"$bytes":"..."}`; native callers receive Bytes directly.
For source-free execution, use a command deployment descriptor with target `encode`
and invoke `run --deployment` with the same argument/result options. No HTTP, file
or secret grants are needed by this pure serializer.

Existing accepted packages do not silently gain these exports. Explicitly author,
review, export and stage the updated ordinary library and consumer; no runtime
upgrade is implied merely by updating a documentation example.

Keep the executable, built consumer bundle and its command deployment descriptor.
The relative artifact path belongs to that descriptor. Detached execution does not
need the library producer, consumer authoring graph, transport staging or proposal
files. The maintained [public CLI test](../../tests/public_cli/native_forms.rs)
exercises this path with an isolated copied executable and an empty PATH.

## What an HTTP application must still own

Read a bounded request body only after admitting method, origin, host and content
type. Reject duplicate or unknown action fields explicitly. Apply domain validation
after decoding; preserving control characters in a codec does not authorize them
in a note, identifier, header or database key. Emit escaped output through the typed
UI/HTML library rather than interpolating decoded text as markup.

Origin checks are not authentication. Trusted target-origin policy must not be
inferred from untrusted forwarded headers. State-changing GET, permissive missing
Origin handling and silent duplicate selection are not supplied as defaults here;
see the [OWASP CSRF guidance](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html).
Persistent actions additionally need exact-base conditional updates, a completed
transaction outcome and explicit conflict/error responses. This codec does not
claim those application properties, accounts, multipart uploads, browser end-to-end
coverage or a finished stateful framework. The separate [durable editor](native-editor.md)
now supplies and tests a small explicit admission/transaction/UI composition rather
than putting those policies inside this codec.

## Compose a stateless POST receiver

The [HTTP receiver literal](examples/form-http.lkjc) imports the accepted consumer's
public `report` function. It is ordinary lkjscript and uses only the existing HTTP
runner and bounded byte-stream capability. Export the accepted consumer transport:

```sh
lkjscript --project consumer package current export \
  --kind transport --output consumer.lkjp
lkjscript new receiver --template http --name form-receiver
lkjscript --project receiver status
lkjscript --project receiver package dependency stage \
  --transport CONSUMER_TRANSPORT --input-file consumer.lkjp
```

Bind the literal's `SITE_BASE`, `CONSUMER_PACKAGE_REVISION`, `CONSUMER_PACKAGE` and
`CONSUMER_REVISION` to those observed values, replacing longer placeholders first.
Plan/review/apply the unchanged bound proposal, then check and build:

```sh
lkjscript --project receiver change plan \
  --input-file form-http-bound.lkjc --output form-http.plan
# Review the complete proposal and plan, then use the returned plan token.
lkjscript --project receiver change apply \
  --input-file form-http-bound.lkjc --plan PLAN_TOKEN
lkjscript --project receiver check
lkjscript --project receiver build --output application.lkja
```

Copy the generated service descriptor beside the executable and bundle. Set its
artifact to `application.lkja`, target to `form-http`, and
`http.maximum_request_body_bytes` to 8192. Keep the generated `127.0.0.1:0`
listener, stream grant and independent runtime policies. Invoke
`lkjscript serve --deployment service.deployment.json` and use the reported local
address. No public listener or external service configuration is required.

POST `/decode` returns a JSON report, HTTP 200 for valid input and 400 for malformed
form data. The transport rejects a body over 8192 bytes with 413. This diagnostic
receiver admits exactly one Content-Type header with the literal value
`application/x-www-form-urlencoded` or
`application/x-www-form-urlencoded; charset=UTF-8`; other spellings, missing values
and duplicate headers return 415. That narrow fixture policy is not a general MIME
parser. Other methods/paths return 404. Query fields do not replace body fields.

The original decoder-only receiver closure passed 111 graph tests, including three
new header policy tests; shared standard/library tests were not counted as new tests.
The separate live observation covers 35 requests across two detached lifecycles,
including eight concurrent clients, an exact-bundle restart and joined SIGINT
shutdown. Its HTTP statuses are twenty 200, six 400, four 404, one 413 and four 415.
Both shutdowns leave no active/queued tasks or cleanup failures. The same retained
executable also passes a separate 688-case detached command oracle. These historical
decoder observations remain at the
[original campaign](../campaigns/202609250926.md); they are not reattributed to a later
library revision, nor are they throughput, accessibility or live-browser claims.

The encoding extension adds 18 literal native tests. Its current library, consumer
and receiver closures pass 124, 125 and 129 graph tests respectively, with pure
reference/VM agreement. The [maintained encoding workflows](../../tests/public_cli/native_form_encoding.rs)
compare 674 independent expected cases through both project and source-free command
paths, then check 205 detached encode/decode roundtrips. Exact bytes are checked
before roundtrips so mutually incorrect codecs cannot validate one another.
They cover all ASCII characters, UTF-8 scalar boundaries, ordered duplicates,
negative and maximum I64 bounds, exact output expansion and separator accounting.
Invalid proposals and altered review tokens leave accepted meaning unchanged; a
well-typed space-encoding mutation is detected by the literal native tests.

A separate encoder/receiver composition sends 12 real loopback requests across two
joined source-free server lifecycles: eight successful POSTs (four concurrent clients
per lifecycle), two malformed bodies and two rejected content types. Neither the
original library producer nor an application authoring graph is present. The encoder
and receiver are ordinary native programs; the test's independent Rust client
transports their bytes. This
is not a claim of a native outbound HTTP client or browser end-to-end coverage.
The unchanged durable editor is also checked against the newly authored exact
supplier. [Current evidence](../campaigns/202609261045.md) retains the actual runtime,
source identities, operational batch-boundary failure and corrected test scope.

An additional observation uses the unchanged official v0.1.47 executable to create
and review both ordinary packages, check their 124/125 tests, export/import, build,
remove the producer and consumer sources, and execute 642 detached cases. Expected
bytes come from independent `URLSearchParams`, not a copy of the native algorithm.
Every product subprocess has an empty PATH; the external oracle is test-only, not
an application dependency. This is one published-runtime interoperability witness,
not an assumption that every older executable supports the same operations.

This receiver merely reports supplied values and saves nothing. It has no account,
session, origin authorization or durable action. It does not make a POST endpoint
safe for a later effectful application. The separate [native editor](native-editor.md)
now composes explicit access/origin policy and conflict-aware durable saves;
those application guarantees remain at that workload's owner, not in this codec.
