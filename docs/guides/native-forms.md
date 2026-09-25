# Strict native form input

This guide requires **unreleased development source after v0.1.45**. The official
immutable v0.1.45 binary does not recognize `core.bytes.from-list` or
`core.bytes.to-text-result` and correctly rejects the new closure. Application
use requires a compatible executable, not Cargo or a compiler checkout at runtime.
The [campaign](../campaigns/202609250926.md) owns actual acceptance and delivery.

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
coverage or a finished stateful framework.
