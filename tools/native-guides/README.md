# Native public-guide tool

This ordinary lkjscript program renders all eight product reference pages.
The executable's `capabilities --generate-docs` and `--verify-generated` routes
execute its embedded artifact. Native meaning owns page prose, layout, escaping,
required-reference selection and missing/ambiguous-reference rejection. Rust
observes authoritative registry/interface metadata and numeric adapter defaults,
admits typed arguments, executes pure commands and publishes derived files.
No parallel Rust page renderer remains. Its strict grant-free artifact embedding
uses the shared `platform::native_tool::PureTool` boundary also adopted by the
[repository-policy tool](../native-policy/README.md). Preparation is shared only
for immutable code; invocation state and cancellation remain per call.

This is native development-tool adoption, not compiler self-hosting. The program
needs no Python, Node, shell subprocess or compiler checkout at runtime. The
bootstrap compiler, semantic observers and platform adapters remain Rust. The
program uses the existing exact standard library without a special intrinsic.
It is not part of the frozen public v0.1.44 executable.

## Authority and regeneration

`project/` is the accepted editable meaning repository. `markup` owns reusable
HTML tables, escaped code blocks and balanced Text concatenation. `guides` owns
typed observations, native selection/validation and all eight command targets.
The literal proposals in `requests/` preserve initial authorship, not a second
synchronized authority. `create.lkjc` creates the initial tool; the additive
`reference-pages.lkjc` extends its observed `guides` module. Fresh reproduction
substitutes only `BASE` and, for the extension, `GUIDES` with observed identities.
Maintained edits are drafted from accepted meaning, not replayed over existing owners.

From the compiler repository root, using a compatible installed executable:

```sh
lkjscript --project tools/native-guides/project check
lkjscript --project tools/native-guides/project query find module guides
lkjscript --project tools/native-guides/project change draft --owner MODULE --output edit.lkjc
lkjscript --project tools/native-guides/project change plan --input-file edit.lkjc --output edit.logical-plan
lkjscript --project tools/native-guides/project change apply --input-file edit.lkjc --plan TOKEN
lkjscript --project tools/native-guides/project build --output rebuilt-guides.lkja
```

Replace `MODULE` and `TOKEN` with observed exact identities. An untouched draft
plans as unchanged and has no publication token; omit `--output` for that no-op.
Check runs 16 native tests plus 45 tests from the exact standard supplier,
differentially. An unchanged build must match `generated/guides.lkja` exactly.
After a reviewed edit, replace that derived artifact with the accepted build,
rebuild the host executable, then regenerate and verify `docs/generated/` through
the public capabilities route. Never edit graph storage or compiled bytes directly.
A failed build does not roll back an accepted source edit.

The existing public CLI test owns copied-product fresh authorship of both proposals,
canonical re-entry, independent expected text, old/new detached execution and
exact artifact/all-page regeneration. Native adapter tests independently exercise
all page families, strict types, escaping, reference order and each required
reference's missing/duplicate cases. No new receipt or parallel test owner is used.

## Ordinary command inputs

Every target takes a Build record first: `{product, version, capabilities}`, all
Text. Additional positional arguments are:

| Target | Additional typed inputs | Result |
| --- | --- | --- |
| `operations` | Operation list, Template list, runner-observation Text | Text |
| `diagnostics` | Diagnostic list, Exit list | Text |
| `change-grammar` | ordered List of registry-section Text | Text |
| `function-definition`, `deployment` | registry-section Text | Text |
| `builtin-standard` | Standard `{package, revision, records}`, all Text | Text |
| `stateful-http` | complete Owner list | Document |
| `relay-information` | Template, complete Owner list, Limit list | Document |

An Owner is `{kind, name, parent, reference}`, all Text. The observer uses an empty
parent for a top-level declaration and its full package/owner reference otherwise.
Native selection matches kind, name and exact parent, requires one nonempty
reference, and renders its own fixed order independently of inventory order.
A Limit is `{name: Text, maximum: I64}`; the host observes actual adapter defaults.
A Document is `{valid: Bool, content: Text}`. Invalid selection returns false and
native diagnostic text, not a partially valid guide. The host rejects that result
before the generated-document collection reaches file publication. This does not
make later independent filesystem writes atomic as a group.

For example, this needs only an ordinary compatible executable:

```sh
lkjscript --project tools/native-guides/project run function-definition --arguments '[{"product":"demo","version":"1","capabilities":"example"},"owner name=example\n"]' --result-file definition.json
```

The output is one JSON Text value. The host decodes it to Markdown bytes. Larger
inputs use `--arguments-file`. Build and copy the bundle with an ordinary Command
deployment descriptor to run after removing the authoring project; no grants or
secrets are needed. Record field names come from accepted native declarations.

## Output compatibility and costs

Dynamic code and tables use escaped HTML, not Markdown delimiter interpolation.
Pipes, backticks, quotes, angle brackets and Unicode remain content. Markdown
renderers must support HTML tables and `pre` blocks; raw-text consumers see HTML.
The reference tables now expose exact owner kinds and the relay limit table uses
numeric observations. Registry compact records, identities, diagnostic codes,
graph/artifact encodings and application data are unchanged.

The pure embedded program cannot opt into tasks or component grants. Normal typed
input/output bounds and cancellation remain in force. Balanced concatenation
avoids repeatedly copying the document prefix; this is not a claim of superior
speed to Rust. The campaign records matched generation costs and their limits.

For accepted source `ee2c6079`, seven alternating fresh-process pairs measured
complete eight-page generation at median 458.215 ms, versus 389.797 ms for the
accepted two-native-page predecessor. Outputs were byte-checked per revision;
filesystem caches were not flushed. The host executable grew by 254,272 bytes.
These same-machine command costs are not application-runtime or peak-memory
measurements. See the [completion campaign](../../docs/campaigns/202609240132.md)
for the full fresh 26-gate source result, reproducibility, costs and delivery boundary.
