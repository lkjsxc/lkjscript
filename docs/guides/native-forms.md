# Strict native form input

This guide requires **v0.1.46 or a compatible successor**. The official immutable
v0.1.45 binary does not recognize `core.bytes.from-list` or
`core.bytes.to-text-result` and correctly rejects the new closure. Application
use requires a compatible executable, not Cargo or a compiler checkout at runtime.
The [campaign](../campaigns/202609250926.md) owns this increment's source observations.
The [durable editor](native-editor.md) consumes it for authenticated, conditional
saves. Public v0.1.46 now supplies the required operations; the
[release record](../release.md) retains its distinct distribution evidence.

## A library, not a form-specific runtime primitive

The [literal library](examples/form-codec.lkjc) exports ordinary nominal data:

```text
field { name: Text, value: Text }
result = valid(List<field>) | invalid(Text)
decode(body: Bytes, maximum-bytes: I64, maximum-fields: I64) -> result
```

Its state machine, percent decoding, limits and error policy are ordinary pure
lkjscript. The kernel contributes only general byte construction and checked UTF-8
conversion. There is no form-specific opcode, HTTP adapter parser, JSON/base64
round trip, external executable or language-to-language source generator.

The [consumer](examples/form-consumer.lkjc) imports the exact library, projects its
nominal fields into an application-owned structural report and exposes individual
and batch command targets. It does not inherit filesystem/network authority.

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

The accepted receiver closure passes 111 graph tests, including three new header
policy tests; shared standard/library tests are not counted as new tests. The
separate live observation covers 35 requests across two detached lifecycles,
including eight concurrent clients, an exact-bundle restart and joined SIGINT
shutdown. Its HTTP statuses are twenty 200, six 400, four 404, one 413 and four 415.
Both shutdowns leave no active/queued tasks or cleanup failures. The same retained
executable also passes a separate 688-case detached command oracle. These are
execution observations, not throughput, accessibility or live-browser claims.

This receiver merely reports supplied values and saves nothing. It has no account,
session, origin authorization or durable action. It does not make a POST endpoint
safe for a later effectful application. The separate [native editor](native-editor.md)
now composes explicit access/origin policy and conflict-aware durable saves;
those application guarantees remain at that workload's owner, not in this codec.
